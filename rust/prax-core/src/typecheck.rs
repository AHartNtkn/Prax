//! The static well-formedness checker: unbound variables, exclusion-cardinality
//! clashes, dangling references, dead conditions, reserved-family touches, an
//! unseeded die, unclosed obligations, unmotivated coercions. Sound and
//! declaration-free (no false positives), so the report is trustworthy.
//!
//! The checker itself is S9. What lives here at S8 is the one thing S8's script
//! layer forces a decision about: the RESERVED-FAMILY LIST.
//!
//! Frozen reference: `Prax.TypeCheck.reservedFamilies`, a top-level constant
//! `[turnPath, "contradiction", scenePatienceFamily, currentScenePath]` consulted
//! for EVERY state, script-compiled or not. Two of its four members are declared
//! in `Prax.Script` and imported here; the Rust crate graph forbids
//! `prax-core → prax-script` (the cycle argument that split `obliged_close` at
//! S4), so the two script constants live HERE, at the checker, and `prax-script`
//! reads them from here. One home each, and the list stays a WORLD-INDEPENDENT
//! constant exactly as the frozen one is.
//!
//! The alternative the S8 design first proposed — a per-`State` list grown
//! through an engine door when a script compiles — was rejected by the design
//! panel [D-C2] and is not implemented: it would make membership a property of
//! how a world was BUILT, so a NON-script world authoring `scenePatience` would
//! be flagged by the frozen checker and not by the Rust one. That is a semantic
//! divergence, and the already-booked owed:S9 row on `ScheduleRuleSpec`'s
//! provenance verdict pins exactly the case it would break (`emptyState`, no
//! prax-script anywhere). With the list static there is nothing left for a
//! `register_reserved_families` door to do, so no such door exists.

use std::collections::BTreeMap;

use smallvec::SmallVec;

use crate::compilepipe::Effect;
use crate::engine::State;
use crate::interner::{Sym, is_variable_name};
use crate::path::{segment_names_checked, segment_tokens_checked};
use crate::query::{Cond, Condition, cond_sents, condition_vars};
use crate::relevance::{cooked_fn_pool, cooked_outcome_atoms, may_unify_syms};
use crate::types::{Axiom, Outcome, Practice, outcome_sents};
use crate::vocab_consts::{OBLIGED_HEAD, OBLIGED_LIFT_PREFIX, PUNITIVE_PREFIX};

/// The engine clock's fact family (`Prax.Types.turnPath`).
pub const TURN_PATH: &str = "turn";

/// The detected-contradiction marker's family (the frozen `reservedFamilies`
/// spells this one as a literal too).
pub const CONTRADICTION_PATH: &str = "contradiction";

/// The timed-junction patience-marker family (`Prax.Script.scenePatienceFamily`,
/// spec v50): a timed junction `j` of scene `sid` carries the fact
/// `scenePatience.<sid>.<j>`, armed with lifetime `n` on scene entry and
/// retracted `n` boundaries later by the v44 expiry schedule. Compiler
/// machinery, not fiction — produced only by the script compiler's scene-entry
/// fold, read only by the compiled `story` rule, and closed to authors.
pub const SCENE_PATIENCE_FAMILY: &str = "scenePatience";

/// The current-scene fact family (`Prax.Script.currentScenePath`, spec v46): the
/// single-slot fact `currentScene!<id>` names the active scene. Compiler-emitted
/// and literal-tailed with exactly one legitimate writer, so it is reserved: no
/// authored surface may write it.
pub const CURRENT_SCENE_PATH: &str = "currentScene";

/// The families no AUTHORED rule, action or desire may write — the checker's
/// world-independent constant (`Prax.TypeCheck.reservedFamilies`). Rules
/// installed through the compiler-level door
/// ([`crate::engine::door::register_engine_rules`]) are exempt by PROVENANCE
/// (v53), which is what [`crate::engine::State::engine_rule_names`] records; the
/// family list itself never varies by world.
pub const RESERVED_FAMILIES: [&str; 4] = [
    TURN_PATH,
    CONTRADICTION_PATH,
    SCENE_PATIENCE_FAMILY,
    CURRENT_SCENE_PATH,
];

/// The patience marker for timed junction `jname` of scene `sid`
/// (`Prax.Script.scenePatiencePath`). Lives beside the family constant it is
/// built from, so the two cannot desync; the script compiler is its only writer.
pub fn scene_patience_path(sid: &str, jname: &str) -> String {
    format!("{SCENE_PATIENCE_FAMILY}.{sid}.{jname}")
}

/// A well-formedness problem found in a world (`Prax.TypeCheck.TypeError`). One
/// constructor per check, NINE in all (`Prax.TypeCheck`'s "nine in all"): the
/// module's whole charter is soundness — no false positives — so the report is
/// trustworthy. Empty ⇒ the world is well-formed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeError {
    /// `var` in `sentence` (at `where_`) is bound by nothing — the most common
    /// real bug: an ungroundable variable silently inserts a literal or a no-op.
    UnboundVar {
        where_: String,
        var: String,
        sentence: String,
    },
    /// The variable-normalized slot is used both single-valued (`!`) and
    /// multi-valued (`.`) — the paper's exclusion-information check, done static.
    CardinalityClash { slot: String },
    /// A `Call`/spawn (at `where_`) names a function or practice never defined.
    UndefinedRef { where_: String, name: String },
    /// A position/variable (`where_`) is inferred to have two sorts (`detail`).
    SortConflict { where_: String, detail: String },
    /// An authored definition touches the engine-owned family `family` (v45/v53):
    /// its facts are machinery, written only by compiled mechanism.
    ReservedFamily {
        family: String,
        where_: String,
        sentence: String,
    },
    /// A world's authored outcomes contain a reachable `Roll` yet `rng_seed` is
    /// `None`: executing the draw would be a loud unseeded-die error at runtime.
    SeedlessDraw,
    /// The positive pattern `sentence` (at `where_`) may-unifies nothing the world
    /// can ever contain: the site can never fire.
    DeadCondition { where_: String, sentence: String },
    /// The world can invoke an obligation yet omits the □-lifted twin of the
    /// liftable rule whose first head is `sentence` (DEON property 1, v51).
    DeonticUnclosed { sentence: String },
    /// An authored outcome deposits a coercion's motive belief for a punitive
    /// desire `name` absent from the registered vocabulary (v54 §2).
    CoercionUnmotivated { name: String },
}

/// A pre-split, pre-interned path (a lint pattern / producible atom).
type Names = SmallVec<[Sym; 6]>;

/// Every well-formedness problem in a world (empty ⇒ well-formed) —
/// `Prax.TypeCheck.typeCheck`. The eleven concatenated passes in the FROZEN order
/// (`TypeCheck.hs:96-108`): the three `UnboundVar` sub-passes
/// practice→function→axiom, then cardinality, refs, sorts, reserved-family,
/// seedless draw, dead conditions, deontic closure, coercion. That order is
/// observable only through a doubly-offending world's first error (the §9 pin).
pub fn type_check(st: &State) -> Vec<TypeError> {
    let mut errs = Vec::new();
    // 1-3: unbound variables (practice, then function, then axiom).
    for p in st.practice_defs().values() {
        unbound_in_practice(p, &mut errs);
    }
    for f in st.functions_src() {
        unbound_in_function(f, &mut errs);
    }
    for ax in st.axioms_src() {
        unbound_in_axiom(ax, &mut errs);
    }
    // 4: exclusion-cardinality consistency.
    cardinality_errors(&asserted_sentences(st), &mut errs);
    // 5: dangling references.
    ref_errors(st, &mut errs);
    // 6: ML-style sort inference (only when sorts are declared).
    sort_errors(st, &mut errs);
    // 7: reserved-family writes (v45/v53).
    reserved_family_errors(st, &mut errs);
    // 8: draws need a seeded die.
    seedless_draw_errors(st, &mut errs);
    // 9: dead conditions.
    dead_condition_errors(st, &mut errs);
    // 10: a world that can invoke obligation declares its closure (v51).
    deontic_unclosed_errors(st, &mut errs);
    // 11: a deposited punitive belief names a registered desire (v54).
    coercion_unmotivated_errors(st, &mut errs);
    errs
}

// ---- variables mentioned (`condVars`/`varsOf`, filtered to variables) -------

/// The variables a raw sentence mentions (`varsOf = filter isVariable . pathNames`).
fn vars_of(s: &str) -> Vec<String> {
    segment_names_checked(s)
        .expect("an installed sentence is tokenize-valid")
        .into_iter()
        .filter(|n| is_variable_name(n))
        .collect()
}

/// The variables a condition mentions (`Prax.TypeCheck.condVars`) — the frozen
/// `conditionVars` filtered to variables (an over-approximation of what it binds,
/// sound for the unbound check).
fn cond_vars(c: &Condition) -> Vec<String> {
    condition_vars(c)
        .expect("an installed condition is tokenize-valid")
        .into_iter()
        .filter(|n| is_variable_name(n))
        .collect()
}

fn flat_cond_vars(cs: &[Condition]) -> Vec<String> {
    cs.iter().flat_map(cond_vars).collect()
}

// ---- Check 1: unbound variables ---------------------------------------------

/// The variables an outcome USES, paired with the text they appear in
/// (`Prax.TypeCheck.outcomeUses`). `ForEach`/`Roll` drop variables the guard
/// binds.
fn outcome_uses(o: &Outcome) -> Vec<(String, String)> {
    match o {
        Outcome::Insert(s) | Outcome::Delete(s) | Outcome::InsertFor(_, s) => {
            vars_of(s).into_iter().map(|v| (v, s.clone())).collect()
        }
        Outcome::Call(fn_, args) => args
            .iter()
            .flat_map(|a| vars_of(a).into_iter().map(|v| (v, fn_.clone())))
            .collect(),
        Outcome::ForEach(conds, outs) | Outcome::Roll(_, _, conds, outs) => {
            let bound = flat_cond_vars(conds);
            outs.iter()
                .flat_map(outcome_uses)
                .filter(|(v, _)| !bound.contains(v))
                .collect()
        }
    }
}

fn unbound_in_outcomes(loc: &str, bound: &[String], outs: &[Outcome], errs: &mut Vec<TypeError>) {
    for o in outs {
        for (v, s) in outcome_uses(o) {
            if !bound.contains(&v) {
                errs.push(TypeError::UnboundVar {
                    where_: loc.to_owned(),
                    var: v,
                    sentence: s,
                });
            }
        }
    }
}

fn unbound_in_practice(p: &Practice, errs: &mut Vec<TypeError>) {
    unbound_in_outcomes(&format!("{} (init)", p.id), &p.roles, &p.init_outcomes, errs);
    for a in &p.actions {
        let mut bound = vec!["Actor".to_owned()];
        bound.extend(p.roles.iter().cloned());
        bound.extend(flat_cond_vars(&a.when));
        unbound_in_outcomes(&format!("{} / {}", p.id, a.name), &bound, &a.then, errs);
    }
}

fn unbound_in_function(f: &crate::types::Function, errs: &mut Vec<TypeError>) {
    for c in &f.cases {
        let mut bound = f.params.clone();
        bound.extend(flat_cond_vars(&c.conditions));
        unbound_in_outcomes(&format!("fn {}", f.name), &bound, &c.outcomes, errs);
    }
}

fn unbound_in_axiom(ax: &Axiom, errs: &mut Vec<TypeError>) {
    let bound = flat_cond_vars(&ax.when);
    for h in &ax.then {
        for v in vars_of(h) {
            if !bound.contains(&v) {
                errs.push(TypeError::UnboundVar {
                    where_: "axiom".to_owned(),
                    var: v,
                    sentence: h.clone(),
                });
            }
        }
    }
}

// ---- Check 2: exclusion-cardinality consistency -----------------------------

/// Each edge of a sentence, keyed by the variable-normalized path to its parent,
/// paired with whether that edge is exclusive (`!`) — `Prax.TypeCheck.edgesOf`.
fn edges_of(s: &str) -> Vec<(String, bool)> {
    let ts = segment_tokens_checked(s).expect("an installed sentence is tokenize-valid");
    let names: Vec<String> = ts
        .iter()
        .map(|(n, _)| if is_variable_name(n) { "_".to_owned() } else { n.clone() })
        .collect();
    let mut out = Vec::new();
    for (i, (_, op)) in ts.iter().enumerate() {
        if let Some(c) = op {
            out.push((names[..=i].join("."), *c == '!'));
        }
    }
    out
}

fn cardinality_errors(sentences: &[String], errs: &mut Vec<TypeError>) {
    let mut byslot: BTreeMap<String, Vec<bool>> = BTreeMap::new();
    for s in sentences {
        for (slot, excl) in edges_of(s) {
            byslot.entry(slot).or_default().push(excl);
        }
    }
    for (slot, labels) in byslot {
        let mut distinct = labels.clone();
        distinct.sort_unstable();
        distinct.dedup();
        if distinct.len() > 1 {
            errs.push(TypeError::CardinalityClash { slot });
        }
    }
}

/// The insert- and insert_for-shaped sentences of an outcome list, recursing
/// through `ForEach`/`Roll` (`Prax.TypeCheck.assertedSentences`'s `inserts`).
fn inserts_of(os: &[Outcome]) -> Vec<String> {
    let mut out = Vec::new();
    for o in os {
        match o {
            Outcome::Insert(s) | Outcome::InsertFor(_, s) => out.push(s.clone()),
            Outcome::ForEach(_, subs) | Outcome::Roll(_, _, _, subs) => {
                out.extend(inserts_of(subs));
            }
            _ => {}
        }
    }
    out
}

/// The world's ASSERTING sentences, for the cardinality pass
/// (`Prax.TypeCheck.assertedSentences`) — inserts, static data facts, axiom
/// heads, and live facts; query conditions are deliberately excluded.
fn asserted_sentences(st: &State) -> Vec<String> {
    let mut out = Vec::new();
    for p in st.practice_defs().values() {
        out.extend(p.data_facts.iter().cloned());
        out.extend(inserts_of(&p.init_outcomes));
        for a in &p.actions {
            out.extend(inserts_of(&a.then));
        }
    }
    for f in st.functions_src() {
        for c in &f.cases {
            out.extend(inserts_of(&c.outcomes));
        }
    }
    for ax in st.axioms_src() {
        out.extend(ax.then.iter().cloned());
    }
    out.extend(st.labeled_facts());
    out
}

// ---- Check 3: dangling references -------------------------------------------

fn ref_errors(st: &State, errs: &mut Vec<TypeError>) {
    let defined_fns: Vec<&str> = st.functions_src().iter().map(|f| f.name.as_str()).collect();
    let defined_prac: Vec<&str> = st.practice_defs().keys().map(String::as_str).collect();
    let outcome_ref = |loc: &str, o: &Outcome, errs: &mut Vec<TypeError>| {
        outcome_ref_scan(loc, o, &defined_fns, &defined_prac, errs);
    };
    for p in st.practice_defs().values() {
        for o in &p.init_outcomes {
            outcome_ref(&p.id, o, errs);
        }
        for a in &p.actions {
            for o in &a.then {
                outcome_ref(&p.id, o, errs);
            }
        }
    }
    for f in st.functions_src() {
        for c in &f.cases {
            for o in &c.outcomes {
                outcome_ref(&format!("fn {}", f.name), o, errs);
            }
        }
    }
}

fn outcome_ref_scan(
    loc: &str,
    o: &Outcome,
    defined_fns: &[&str],
    defined_prac: &[&str],
    errs: &mut Vec<TypeError>,
) {
    match o {
        Outcome::Call(fn_, _) if !defined_fns.contains(&fn_.as_str()) => {
            errs.push(TypeError::UndefinedRef {
                where_: loc.to_owned(),
                name: fn_.clone(),
            });
        }
        Outcome::Insert(s) | Outcome::InsertFor(_, s) => {
            let names = segment_names_checked(s).expect("an installed sentence is tokenize-valid");
            if let (Some(head), Some(pid)) = (names.first(), names.get(1))
                && head == "practice"
                && !defined_prac.contains(&pid.as_str())
            {
                errs.push(TypeError::UndefinedRef {
                    where_: loc.to_owned(),
                    name: format!("practice.{pid}"),
                });
            }
        }
        Outcome::ForEach(_, subs) | Outcome::Roll(_, _, _, subs) => {
            for sub in subs {
                outcome_ref_scan(loc, sub, defined_fns, defined_prac, errs);
            }
        }
        _ => {}
    }
}

// ---- Check 4: ML-style sort inference (only when sorts are declared) ---------

fn sort_errors(st: &State, errs: &mut Vec<TypeError>) {
    let sorts = st.sorts();
    if sorts.is_empty() {
        return;
    }
    let member_pairs: Vec<(String, String)> = sorts
        .iter()
        .flat_map(|(s, cs)| cs.iter().map(move |c| (c.clone(), s.clone())))
        .collect();
    let member_sort: BTreeMap<String, String> =
        member_pairs.iter().map(|(c, s)| (c.clone(), s.clone())).collect();
    let is_member = |c: &str| member_sort.contains_key(c);

    // A constant declared in two sorts is itself a conflict.
    let mut by_const: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (c, s) in &member_pairs {
        by_const.entry(c.clone()).or_default().push(s.clone());
    }
    for (c, ss) in &by_const {
        let uss = nub(ss);
        if uss.len() > 1 {
            errs.push(TypeError::SortConflict {
                where_: c.clone(),
                detail: format!("declared in {}", uss.join(", ")),
            });
        }
    }

    // object/value occurrences of a sentence: (segment, position key).
    let occs = |sentence: &str| -> Vec<(String, String)> {
        let segs = segment_names_checked(sentence).expect("install-valid");
        let mut out = Vec::new();
        for (i, seg) in segs.iter().enumerate() {
            if is_variable_name(seg) || is_member(seg) {
                let key = segs[..=i]
                    .iter()
                    .map(|s| {
                        if is_variable_name(s) || is_member(s) {
                            "_".to_owned()
                        } else {
                            s.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(".");
                out.push((seg.clone(), key));
            }
        }
        out
    };

    let scoped: Vec<(String, String)> = sentences_by_scope(st)
        .into_iter()
        .flat_map(|(sc, ss)| ss.into_iter().map(move |s| (sc.clone(), s)))
        .collect();

    // Per scope, the positions each variable occupies (drives the unions).
    let mut var_pos_map: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for (sc, s) in &scoped {
        for (seg, key) in occs(s) {
            if is_variable_name(&seg) {
                var_pos_map
                    .entry((sc.clone(), seg))
                    .or_default()
                    .push(key);
            }
        }
    }
    // Positions labelled by a member constant landing there (global).
    let mut pos_labels: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (_, s) in &scoped {
        for (seg, key) in occs(s) {
            if is_member(&seg) {
                pos_labels
                    .entry(key)
                    .or_default()
                    .push(member_sort[&seg].clone());
            }
        }
    }

    // Union positions that a variable connects, then group labels by class.
    let mut uf: BTreeMap<String, String> = BTreeMap::new();
    for positions in var_pos_map.values() {
        union_all(&mut uf, positions);
    }
    let mut by_rep: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (key, labs) in &pos_labels {
        by_rep
            .entry(find(&uf, key))
            .or_default()
            .extend(labs.iter().cloned());
    }
    for (rep, labs) in &by_rep {
        let ss = nub(labs);
        if ss.len() > 1 {
            errs.push(TypeError::SortConflict {
                where_: readable(rep),
                detail: ss.join(" vs "),
            });
        }
    }
}

fn readable(key: &str) -> String {
    let ns: Vec<String> = segment_names_checked(key)
        .expect("a position key is a valid dot-path")
        .into_iter()
        .filter(|n| n != "_")
        .collect();
    if ns.is_empty() {
        key.to_owned()
    } else {
        ns.join(".")
    }
}

/// Every sentence, grouped by the scope its variables belong to
/// (`Prax.TypeCheck.sentencesByScope`).
fn sentences_by_scope(st: &State) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for p in st.practice_defs().values() {
        let mut sents = Vec::new();
        for a in &p.actions {
            sents.extend(cond_sents(&a.when));
            sents.extend(outcome_sents(&a.then));
        }
        sents.extend(outcome_sents(&p.init_outcomes));
        out.push((p.id.clone(), sents));
    }
    for f in st.functions_src() {
        let mut sents = Vec::new();
        for c in &f.cases {
            sents.extend(cond_sents(&c.conditions));
            sents.extend(outcome_sents(&c.outcomes));
        }
        out.push((format!("fn {}", f.name), sents));
    }
    for (i, ax) in st.axioms_src().iter().enumerate() {
        let mut sents = cond_sents(&ax.when);
        sents.extend(ax.then.iter().cloned());
        out.push((format!("axiom{i}"), sents));
    }
    out.push(("<facts>".to_owned(), st.labeled_facts()));
    out
}

fn nub(xs: &[String]) -> Vec<String> {
    let mut seen = Vec::new();
    for x in xs {
        if !seen.contains(x) {
            seen.push(x.clone());
        }
    }
    seen
}

/// A tiny union-find over position-key strings (`Prax.TypeCheck.find`).
fn find(uf: &BTreeMap<String, String>, x: &str) -> String {
    match uf.get(x) {
        Some(p) if p != x => find(uf, p),
        _ => x.to_owned(),
    }
}

fn union_all(uf: &mut BTreeMap<String, String>, xs: &[String]) {
    if let Some((first, rest)) = xs.split_first() {
        for b in rest {
            let ra = find(uf, first);
            let rb = find(uf, b);
            if ra != rb {
                uf.insert(ra, rb);
            }
        }
    }
}

// ---- Check 5: reserved-family writes (v45/v53) ------------------------------

/// The sentences an outcome writes (`Prax.TypeCheck.writesOf`): insert/insert_for/
/// delete (a delete IS a write), recursing through `ForEach`/`Roll`; a `Call`
/// writes nothing.
fn writes_of(o: &Outcome) -> Vec<String> {
    match o {
        Outcome::Insert(s) | Outcome::InsertFor(_, s) | Outcome::Delete(s) => vec![s.clone()],
        Outcome::ForEach(_, os) | Outcome::Roll(_, _, _, os) => {
            os.iter().flat_map(writes_of).collect()
        }
        Outcome::Call(_, _) => Vec::new(),
    }
}

/// The AUTHORED write sites, with labels (`Prax.TypeCheck.writeSites`): practice
/// init/action outcomes, function-case outcomes, and every AUTHORED schedule
/// rule body — rules installed through the compiler door (`engine_rule_names`)
/// are dropped WHOLE (the v53 provenance exemption lives HERE, not on `schedule`).
fn write_sites(st: &State) -> Vec<(String, &[Outcome])> {
    let mut v = Vec::new();
    for p in st.practice_defs().values() {
        v.push((format!("{} (init)", p.id), p.init_outcomes.as_slice()));
    }
    for p in st.practice_defs().values() {
        for a in &p.actions {
            v.push((format!("{} / {}", p.id, a.name), a.then.as_slice()));
        }
    }
    for f in st.functions_src() {
        for c in &f.cases {
            v.push((format!("fn {}", f.name), c.outcomes.as_slice()));
        }
    }
    let eng = st.engine_rule_names();
    for r in st.schedule_src() {
        if !eng.iter().any(|n| n == &r.name) {
            for (_conds, outs) in &r.body {
                v.push((format!("schedule {}", r.name), outs.as_slice()));
            }
        }
    }
    v
}

fn family_of(s: &str) -> Option<String> {
    let names = segment_names_checked(s).expect("an installed sentence is tokenize-valid");
    match names.first() {
        Some(h) if RESERVED_FAMILIES.contains(&h.as_str()) => Some(h.clone()),
        _ => None,
    }
}

fn reserved_family_errors(st: &State, errs: &mut Vec<TypeError>) {
    for (loc, os) in write_sites(st) {
        for o in os {
            for s in writes_of(o) {
                if let Some(fam) = family_of(&s) {
                    errs.push(TypeError::ReservedFamily {
                        family: fam,
                        where_: loc.clone(),
                        sentence: s,
                    });
                }
            }
        }
    }
    for ax in st.axioms_src() {
        for h in &ax.then {
            if let Some(fam) = family_of(h) {
                errs.push(TypeError::ReservedFamily {
                    family: fam,
                    where_: "axiom".to_owned(),
                    sentence: h.clone(),
                });
            }
        }
    }
}

// ---- Check 6: draws need a seeded die ----------------------------------------

fn has_roll(o: &Outcome) -> bool {
    match o {
        Outcome::Roll(..) => true,
        Outcome::ForEach(_, outs) => outs.iter().any(has_roll),
        _ => false,
    }
}

fn seedless_draw_errors(st: &State, errs: &mut Vec<TypeError>) {
    if st.rng_seed().is_some() {
        return;
    }
    let mut any = false;
    for p in st.practice_defs().values() {
        any |= p.init_outcomes.iter().any(has_roll);
        for a in &p.actions {
            any |= a.then.iter().any(has_roll);
        }
    }
    for f in st.functions_src() {
        for c in &f.cases {
            any |= c.outcomes.iter().any(has_roll);
        }
    }
    // The schedule is read DIRECTLY here (no engine-rule exemption): a compiled
    // draw anywhere still needs a seed.
    for r in st.schedule_src() {
        for (_conds, outs) in &r.body {
            any |= outs.iter().any(has_roll);
        }
    }
    if any {
        errs.push(TypeError::SeedlessDraw);
    }
}

// ---- Check 7: dead conditions ------------------------------------------------

/// The positive patterns of a cooked condition (`Prax.TypeCheck.positives`):
/// top-level `CMatch`, or inside `CExists`; everything else contributes none.
fn positives(c: &Cond) -> Vec<&Names> {
    match c {
        Cond::Match(p) => vec![p],
        Cond::Exists(cs) => cs.iter().flat_map(positives).collect(),
        _ => Vec::new(),
    }
}

/// The `ForEach`/`Roll` guards of a cooked outcome list, recursively
/// (`Prax.TypeCheck.forEachGuards`).
fn for_each_guards(outs: &[Effect]) -> Vec<Vec<Cond>> {
    let mut out = Vec::new();
    for o in outs {
        if let Effect::ForEach(conds, os) | Effect::Roll(_, _, conds, os) = o {
            out.push(conds.clone());
            out.extend(for_each_guards(os));
        }
    }
    out
}

/// The affordance/motive sites the dead-condition lint scans, with labels
/// (`Prax.TypeCheck.lintSites`) — axiom bodies are deliberately OUT of scope.
fn lint_sites(st: &State) -> Vec<(String, Vec<Cond>)> {
    let comp = st.compiled_tables();
    let mut out = Vec::new();
    for (pid, cp) in &comp.practices {
        for a in &cp.actions {
            out.push((format!("{pid} / {}", a.name), a.conds.clone()));
        }
    }
    for (pid, cp) in &comp.practices {
        for a in &cp.actions {
            for gs in for_each_guards(&a.outs) {
                out.push((format!("{pid} / {} (effect guard)", a.name), gs));
            }
        }
    }
    for (pid, cp) in &comp.practices {
        for gs in for_each_guards(&cp.inits) {
            out.push((format!("{pid} (init guard)"), gs));
        }
    }
    for (fn_, (_params, cases)) in &comp.fns {
        for (cs, _os) in cases {
            out.push((format!("fn {fn_}"), cs.clone()));
        }
    }
    for (fn_, (_params, cases)) in &comp.fns {
        for (_cs, os) in cases {
            for gs in for_each_guards(os) {
                out.push((format!("fn {fn_} (effect guard)"), gs));
            }
        }
    }
    for r in &comp.schedule {
        for (cs, _os) in &r.body {
            out.push((format!("schedule {}", r.name), cs.clone()));
        }
    }
    for r in &comp.schedule {
        for (_cs, os) in &r.body {
            for gs in for_each_guards(os) {
                out.push((format!("schedule {} (effect guard)", r.name), gs));
            }
        }
    }
    for (n, cs) in &comp.desires {
        out.push((format!("desire {n}"), cs.clone()));
    }
    for (n, css) in &comp.wants {
        for cs in css {
            out.push((format!("want of {n}"), cs.clone()));
        }
    }
    out
}

fn dead_condition_errors(st: &State, errs: &mut Vec<TypeError>) {
    let pool = match st.producible_atoms() {
        None => return, // wild world: silence the lint entirely
        Some(pool) => pool,
    };
    for (loc, conds) in lint_sites(st) {
        for c in &conds {
            for p in positives(c) {
                if p.iter().all(|s| s.is_var()) {
                    continue; // all-variable: matches everything, never dead
                }
                if !pool.iter().any(|atom| may_unify_syms(p, atom)) {
                    errs.push(TypeError::DeadCondition {
                        where_: loc.clone(),
                        sentence: st.render_segs(p),
                    });
                }
            }
        }
    }
}

// ---- Check 8: obligation closure (v51) --------------------------------------

fn lifted(s: &str) -> bool {
    s.starts_with(OBLIGED_LIFT_PREFIX)
}

/// The □-lifted twin of a purely-conjunctive rule (`Prax.Deontic.obligedLift`,
/// replicated syntactically here from the shared prefix so the checker
/// (`prax-core`) needs no `prax-vocab` edge — the vocab_consts design [C1]).
fn obliged_lift(ax: &Axiom) -> Option<Axiom> {
    if ax.when.iter().all(|c| matches!(c, Condition::Match(_))) {
        let when = ax
            .when
            .iter()
            .map(|c| match c {
                Condition::Match(s) => Condition::Match(format!("{OBLIGED_LIFT_PREFIX}{s}")),
                other => other.clone(),
            })
            .collect();
        let then = ax.then.iter().map(|s| format!("{OBLIGED_LIFT_PREFIX}{s}")).collect();
        Some(Axiom { when, then })
    } else {
        None
    }
}

/// An axiom that IS already a □-form owes no twin (`Prax.TypeCheck.alreadyLifted`).
fn already_lifted(ax: &Axiom) -> bool {
    ax.when
        .iter()
        .all(|c| matches!(c, Condition::Match(s) if lifted(s)))
        && ax.then.iter().all(|h| lifted(h))
}

/// Can this world ever contain an `obliged.*` fact
/// (`Prax.TypeCheck.deonticInvokable`)? The producer census — practice and
/// schedule insert atoms, db facts as of now, and axiom heads; a variable-headed
/// producer or an unresolvable `Call` (wild) counts.
fn deontic_invokable(st: &State) -> bool {
    let mut interner = st.interner_snapshot();
    let comp = st.compiled_tables();
    let fn_pool = cooked_fn_pool(&comp.fns);
    let obliged = interner.intern(OBLIGED_HEAD);

    let mut outcome_atoms: Vec<Option<(Vec<Names>, Vec<Names>)>> = Vec::new();
    for cp in comp.practices.values() {
        for a in &cp.actions {
            for o in &a.outs {
                outcome_atoms.push(cooked_outcome_atoms(&mut interner, &fn_pool, &[], o));
            }
        }
    }
    for cp in comp.practices.values() {
        for o in &cp.inits {
            outcome_atoms.push(cooked_outcome_atoms(&mut interner, &fn_pool, &[], o));
        }
    }
    for csr in &comp.schedule {
        for (_cs, outs) in &csr.body {
            for o in outs {
                outcome_atoms.push(cooked_outcome_atoms(&mut interner, &fn_pool, &[], o));
            }
        }
    }
    if outcome_atoms.iter().any(Option::is_none) {
        return true; // wild: an unresolvable Call could produce anything
    }
    let head_produces = |h: Sym| h == obliged || h.is_var();
    // insert-atom heads
    for oa in outcome_atoms.iter().flatten() {
        for atom in &oa.0 {
            if let Some(&h) = atom.first()
                && head_produces(h)
            {
                return true;
            }
        }
    }
    // db-fact heads
    for s in st.labeled_facts() {
        let names = segment_names_checked(&s).expect("a db sentence is tokenize-valid");
        if let Some(first) = names.first()
            && head_produces(interner.intern(first))
        {
            return true;
        }
    }
    // axiom heads
    for ax in st.axioms_src() {
        for s in &ax.then {
            let names = segment_names_checked(s).expect("an axiom head is tokenize-valid");
            if let Some(first) = names.first()
                && head_produces(interner.intern(first))
            {
                return true;
            }
        }
    }
    false
}

fn deontic_unclosed_errors(st: &State, errs: &mut Vec<TypeError>) {
    if !deontic_invokable(st) {
        return;
    }
    let axioms = st.axioms_src();
    for ax in axioms {
        if let Some(twin) = obliged_lift(ax)
            && !already_lifted(ax)
            && !axioms.contains(&twin)
        {
            errs.push(TypeError::DeonticUnclosed {
                sentence: ax.then.first().cloned().unwrap_or_default(),
            });
        }
    }
}

// ---- Check 9: unmotivated coercion (v54) ------------------------------------

fn coercion_unmotivated_errors(st: &State, errs: &mut Vec<TypeError>) {
    let registered: Vec<&str> = st.desires_src().iter().map(|d| d.name.as_str()).collect();
    let mut seen: Vec<String> = Vec::new();
    for (_loc, os) in write_sites(st) {
        for o in os {
            for s in writes_of(o) {
                let names = segment_names_checked(&s).expect("an installed sentence is tokenize-valid");
                for seg in names {
                    if seg.starts_with(PUNITIVE_PREFIX)
                        && !registered.contains(&seg.as_str())
                        && !seen.contains(&seg)
                    {
                        seen.push(seg.clone());
                        errs.push(TypeError::CoercionUnmotivated { name: seg });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// NATIVE PIN — no frozen label. The frozen suite never asserts the CONTENT
    /// of `reservedFamilies`; it asserts consequences of it through `typeCheck`,
    /// which has no Rust twin until S9. Until then this list is the whole
    /// contract, and the S9 checker will be written against it, so it is pinned
    /// here by content and by ORDER (the order a diagnostic would enumerate).
    ///
    /// REDDENS UNDER (both verified): replacing `SCENE_PATIENCE_FAMILY` with a
    /// look-alike — the [D-C2] failure mode, a checker that no longer refuses an
    /// authored patience write — and reordering the four. DROPPING a member does
    /// not reach this assertion at all: the fixed-length `[&str; 4]` makes it a
    /// compile error, which is the stronger outcome and is why the type is an
    /// array rather than a slice.
    #[test]
    fn the_reserved_family_list_is_the_frozen_four_in_order() {
        assert_eq!(
            RESERVED_FAMILIES,
            ["turn", "contradiction", "scenePatience", "currentScene"],
            "Prax.TypeCheck.reservedFamilies is a fixed list of four, consulted \
             for every state -- script-compiled or not"
        );
    }

    /// NATIVE PIN — no frozen label (`scenePatiencePath` is private to the
    /// frozen `Prax.Script`, so nothing frozen names it). The compiled `story`
    /// rule's `Not` guard and the scene-entry `InsertFor` must agree on this
    /// spelling or a timed junction never fires; both build it from here.
    ///
    /// REDDENS UNDER: swapping the two interpolations (`sid`/`jname`).
    #[test]
    fn the_patience_marker_path_keys_scene_then_junction() {
        assert_eq!(
            scene_patience_path("audience", "dismissed"),
            "scenePatience.audience.dismissed"
        );
    }

    // NATIVE PINS — no frozen label. The frozen `TypeCheckSpec` asserts each
    // constructor through malformed synthetic fixtures; those fixtures are not
    // shipped worlds and never reach the oracle `check` comparator, so they are
    // re-expressed here as SHOULD-flag native pins (one per constructor). Each
    // reddens if its pass is dropped. The shipped-worlds `type_check == []`
    // standing net lives in `conformance/typecheck_spec.rs`.
    mod checker {
        use crate::engine::State;
        use crate::query::{Condition, matches, not_};
        use crate::typecheck::{TypeError, type_check};
        use crate::types::{Action, Axiom, Desire, Function, Outcome, Practice, Want, call, insert};

        fn m(s: &str) -> Condition {
            matches(s)
        }

        fn prac(id: &str) -> Practice {
            Practice::new(id).roles(["R"])
        }

        // 1. UnboundVar: a variable in an outcome no role/Actor/precondition binds.
        #[test]
        fn flags_unbound_var() {
            let mut st = State::new();
            st.define_practices([prac("give").action(
                Action::new("[Actor]: give").then([insert("gift.Stranger")]),
            )])
            .unwrap();
            assert!(type_check(&st).iter().any(|e| matches!(
                e, TypeError::UnboundVar { var, .. } if var == "Stranger"
            )));
        }

        // 2. CardinalityClash: a slot used both `!` and `.`.
        #[test]
        fn flags_cardinality_clash() {
            let mut st = State::new();
            st.define_practices([prac("mark").action(
                Action::new("[Actor]: mark").then([insert("slot!v"), insert("slot.w")]),
            )])
            .unwrap();
            assert!(type_check(&st).iter().any(|e| matches!(
                e, TypeError::CardinalityClash { slot } if slot == "slot"
            )));
        }

        // 3. UndefinedRef: a `Call` to a function never defined.
        #[test]
        fn flags_undefined_ref() {
            let mut st = State::new();
            st.define_practices([prac("tidy").action(
                Action::new("[Actor]: tidy").then([call("missing", vec!["Actor".into()])]),
            )])
            .unwrap();
            assert!(type_check(&st).iter().any(|e| matches!(
                e, TypeError::UndefinedRef { name, .. } if name == "missing"
            )));
        }

        // 4. SortConflict: a constant declared in two sorts.
        #[test]
        fn flags_sort_conflict() {
            let mut st = State::new();
            st.set_sorts(vec![
                ("agent".into(), vec!["bob".into()]),
                ("place".into(), vec!["bob".into()]),
            ])
            .unwrap();
            assert!(type_check(&st).iter().any(|e| matches!(
                e, TypeError::SortConflict { where_, .. } if where_ == "bob"
            )));
        }

        // 5. ReservedFamily: an authored action writes an engine-owned family.
        #[test]
        fn flags_reserved_family_write() {
            let mut st = State::new();
            st.define_practices([prac("meddle").action(
                Action::new("[Actor]: meddle").then([insert("scenePatience.s.j")]),
            )])
            .unwrap();
            assert!(type_check(&st).iter().any(|e| matches!(
                e, TypeError::ReservedFamily { family, .. } if family == "scenePatience"
            )));
        }

        // 5b. The v53 flag/exempt PAIR: the SAME body registered through the
        // engine door (`engine_rule_names`) is exempt. Discharges the
        // ScheduleRuleSpec provenance verdict (owed row 9).
        // H: ScheduleRuleSpec.hs "an authored rule writing a reserved family is flagged; the SAME body through the engine door is exempt"
        #[test]
        fn reserved_family_exempts_the_engine_door() {
            use crate::types::ScheduleRule;
            // Authored: flagged.
            let mut a = State::new();
            a.set_schedule(vec![
                ScheduleRule::new("story", 1).clause(vec![], vec![insert("scenePatience.s.j")]),
            ])
            .unwrap();
            assert!(
                type_check(&a).iter().any(|e| matches!(e, TypeError::ReservedFamily { .. })),
                "an AUTHORED schedule rule writing a reserved family is flagged"
            );
            // Same body via the door: exempt.
            let mut b = State::new();
            crate::engine::door::register_engine_rules(
                &mut b,
                vec![ScheduleRule::new("story", 1)
                    .clause(vec![], vec![insert("scenePatience.s.j")])],
            )
            .unwrap();
            assert!(
                !type_check(&b).iter().any(|e| matches!(e, TypeError::ReservedFamily { .. })),
                "the SAME body registered through the door is exempt by v53 provenance"
            );
        }

        // 6. SeedlessDraw: a reachable `Roll` with no seed.
        #[test]
        fn flags_seedless_draw() {
            let mut st = State::new();
            st.define_practices([prac("gamble").action(
                Action::new("[Actor]: gamble")
                    .then([Outcome::Roll(1, 2, vec![], vec![insert("hit.Actor")])]),
            )])
            .unwrap();
            assert!(type_check(&st).iter().any(|e| matches!(e, TypeError::SeedlessDraw)));
        }

        // 7. DeadCondition: a positive guard matching a fact nothing can produce.
        #[test]
        fn flags_dead_condition() {
            let mut st = State::new();
            st.define_practices([prac("act").action(
                Action::new("[Actor]: act")
                    .when([m("neverProduced.here")])
                    .then([insert("did.Actor")]),
            )])
            .unwrap();
            assert!(type_check(&st).iter().any(|e| matches!(
                e, TypeError::DeadCondition { sentence, .. } if sentence == "neverProduced.here"
            )));
        }

        // 8. DeonticUnclosed: an obligation-invokable world with a liftable rule
        // whose □-lifted twin is absent.
        #[test]
        fn flags_deontic_unclosed() {
            let mut st = State::new();
            st.define_practices([prac("duty").action(
                Action::new("[Actor]: oblige").then([insert("obliged.bob.pay")]),
            )])
            .unwrap();
            st.set_axioms(vec![Axiom::new(vec![m("a.X")], ["b.X"])]).unwrap();
            assert!(type_check(&st).iter().any(|e| matches!(
                e, TypeError::DeonticUnclosed { sentence } if sentence == "b.X"
            )));
        }

        // 9. CoercionUnmotivated: a deposited punitive belief naming no registered
        // desire.
        #[test]
        fn flags_coercion_unmotivated() {
            let mut st = State::new();
            st.define_practices([prac("threaten").action(
                Action::new("[Actor]: threaten")
                    .then([insert("carol.believes.desires.eve.punishes-noPay")]),
            )])
            .unwrap();
            assert!(type_check(&st).iter().any(|e| matches!(
                e, TypeError::CoercionUnmotivated { name } if name == "punishes-noPay"
            )));
        }

        // [P6] — the ELEVEN-pass order: a doubly-offending world (an unbound var
        // in a PRACTICE and another in an AXIOM) surfaces the practice one FIRST,
        // because `unboundInPractice` (pass 1) precedes `unboundInAxiom` (pass 3).
        // REDDENS UNDER: reordering the UnboundVar sub-passes (axiom before
        // practice) — a merged-pass implementation that reorders the group.
        #[test]
        fn eleven_pass_order_surfaces_practice_unbound_before_axiom() {
            let mut st = State::new();
            st.define_practices([prac("p").action(
                Action::new("[Actor]: p").then([insert("state.PraxlessVarPX")]),
            )])
            .unwrap();
            // `QY` in the head is bound by nothing in the (empty) body.
            st.set_axioms(vec![Axiom::new(vec![], ["head.QY"])]).unwrap();
            let errs = type_check(&st);
            let practice_idx = errs.iter().position(|e| matches!(
                e, TypeError::UnboundVar { where_, .. } if where_ != "axiom"
            ));
            let axiom_idx = errs.iter().position(|e| matches!(
                e, TypeError::UnboundVar { where_, .. } if where_ == "axiom"
            ));
            assert!(
                practice_idx.is_some() && axiom_idx.is_some(),
                "both offenders must be flagged: {errs:?}"
            );
            assert!(
                practice_idx < axiom_idx,
                "the PRACTICE unbound var must surface before the AXIOM one \
                 (pass 1 before pass 3): {errs:?}"
            );
        }

        // Keep an unused-import guard honest: `not_`/`Function`/`Want`/`Desire`
        // are the vocabulary the SHOULD-flag family draws on across revisions.
        #[allow(dead_code)]
        fn _vocab_touch() -> (Function, Want, Desire) {
            (
                Function::new("f", ["P"]).case(vec![not_("g.P")], vec![insert("h.P")]),
                Want::new(vec![m("w.Owner")], 1),
                Desire::new("d", Want::new(vec![m("w.Owner")], 1)),
            )
        }
    }
}
