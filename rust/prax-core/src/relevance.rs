//! The relevance analysis. Two primitives serve the engine router:
//! [`may_unify_syms`] (the hot delta-vs-footprint classification) and
//! [`eviction_shadow_names`] (the sibling shadows of an exclusion insert). The
//! rest — [`improvable_desires`], [`liveness_of`], [`bearing_templates`],
//! [`mover_read_anchors`] and the atom pools they share — is the planner's
//! static screening apparatus.
//!
//! Frozen reference: `src/Prax/Relevance.hs`.
//!
//! One stated invariant carries [`may_unify_syms`]'s conservativity (an
//! assumption about authored worlds, not a construction guarantee): entity names
//! never collide with predicate-name literals — no character, place, or value is
//! named `lied`, `believes`, `regards`, and so on. The `anchored` clause spends
//! it: a pattern overlap covered entirely by variables carries no evidence the
//! two patterns denote the same predicate, so it is discarded.

use std::collections::BTreeMap;

use smallvec::{SmallVec, smallvec};

use crate::compilepipe::{CompiledFn, CompiledPractice, Effect};
use crate::db::{Bindings, Val};
use crate::derive::{CompiledRule, axiom_head_patterns};
use crate::interner::{Interner, Sym};
use crate::path::{CompiledPath, tokenize};
use crate::query::{Cond, ground_cond, ground_names, read_anchors};
use crate::types::{Character, Desire};

/// A pattern anchor: a pre-split, pre-interned path (the shape [`may_unify_syms`]
/// classifies). One name for the family that recurs through every S6 table.
type Names = SmallVec<[Sym; 6]>;

/// Could a grounded instance of one path pattern be an instance (or a
/// prefix/extension) of the other, on pre-split, pre-interned paths
/// (`Prax.Relevance.mayUnifySyms`) — the planner-hot classification the router
/// runs for every primitive delta against every footprint pattern. Segments unify
/// when either is a variable or they are equal; a length mismatch is
/// prefix-compatible (a `Match` sees subtrees), so the walk zips to the shorter
/// path. A pair unifies only if some overlapping segment is a shared LITERAL
/// (both sides constant and equal) — an overlap covered entirely by variables
/// carries no evidence the two patterns denote the same predicate. Variable-ness
/// is the parity bit and a shared literal is `Sym`-id equality — the hottest
/// classification in the engine, an `Int` equality rather than a `String` one.
pub fn may_unify_syms(a: &[Sym], b: &[Sym]) -> bool {
    let anchored = a
        .iter()
        .zip(b)
        .any(|(x, y)| !x.is_var() && !y.is_var() && x == y);
    anchored
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.is_var() || y.is_var() || x == y)
}

/// The eviction shadows of an exclusion insert, computed on the labeled path
/// (`Prax.Relevance.evictionShadowNames`). One shadow per `!` operator: the
/// segment names up to and including that point, followed by a fresh
/// `PraxEvicted` segment (interns as a variable — uppercase initial,
/// Prax-namespaced machinery — so [`may_unify_syms`] treats it as the wildcard it
/// denotes and no authored name can collide with it). Each exclusion clears the
/// displaced sibling's entire subtree (arbitrary depth), and [`may_unify_syms`]
/// compares only up to the shorter path, so the truncated shadow covers every
/// want under it.
pub fn eviction_shadow_names(
    interner: &mut Interner,
    path: &CompiledPath,
) -> Vec<SmallVec<[Sym; 6]>> {
    let evicted = interner.intern("PraxEvicted");
    let mut out = Vec::new();
    for j in 0..path.segs.len() {
        if path.is_excl_after(j) {
            let mut shadow: SmallVec<[Sym; 6]> = path.segs[..=j].iter().copied().collect();
            shadow.push(evicted);
            out.push(shadow);
        }
    }
    out
}

/// Each named desire's dead-now recipe (`Prax.Types.Liveness`): a negative
/// want-kind's floor check (its own conditions), a positive want-kind's
/// environment gates (each inner list is ONE gate conjunct, cooked and
/// Owner-templated), or [`Liveness::AlwaysLive`] when no cheap state test
/// applies. Consumed by the planner's `dead_now`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Liveness {
    FloorCheck,
    GateCheck(Vec<Vec<Cond>>),
    AlwaysLive,
}

/// The Call-resolution pool: every registered function's cooked case outcomes,
/// guards ignored (conservatively: all cases), keyed by fn name
/// (`Prax.Relevance.cookedFnPool`). Reads the one registry directly — since v47
/// functions have a single home, there is no practice fold and no
/// resolution-order subtlety.
pub(crate) fn cooked_fn_pool(fns: &BTreeMap<String, CompiledFn>) -> BTreeMap<String, Vec<Effect>> {
    fns.iter()
        .map(|(name, (_params, cases))| {
            (
                name.clone(),
                cases.iter().flat_map(|(_, os)| os.iter().cloned()).collect(),
            )
        })
        .collect()
}

/// The insert- and delete-shaped atoms an outcome can produce, resolving `Call`s
/// through the pool (conservatively: all cases) — `Prax.Relevance.cookedOutcomeAtoms`.
/// An exclusion insert both asserts its path and evicts that value's SIBLINGS
/// (whose names appear nowhere in the outcome), so the delete side carries the
/// path itself plus its eviction shadows. `None` = "unknown effects" (an
/// unresolvable `Call`): the caller must treat that as improves-everything.
pub(crate) fn cooked_outcome_atoms(
    interner: &mut Interner,
    fns: &BTreeMap<String, Vec<Effect>>,
    visited: &[&str],
    o: &Effect,
) -> Option<(Vec<Names>, Vec<Names>)> {
    match o {
        Effect::Insert(p) => {
            let names: Names = p.segs.clone();
            if (0..p.segs.len()).any(|j| p.is_excl_after(j)) {
                let mut dels = vec![names.clone()];
                dels.extend(eviction_shadow_names(interner, p));
                Some((vec![names], dels))
            } else {
                Some((vec![names], Vec::new()))
            }
        }
        // The deferred retract is environment: same atoms as the bare insert.
        Effect::InsertFor(_, p) => {
            cooked_outcome_atoms(interner, fns, visited, &Effect::Insert(p.clone()))
        }
        Effect::Delete(p) => Some((Vec::new(), vec![p.segs.clone()])),
        Effect::ForEach(_, outs) => mconcat_atoms(interner, fns, visited, outs),
        // The body may fire (the roll may hit): its anchors count exactly as a ForEach's.
        Effect::Roll(_, _, _, outs) => mconcat_atoms(interner, fns, visited, outs),
        Effect::Call(fn_, _) => {
            if visited.contains(&fn_.as_str()) {
                Some((Vec::new(), Vec::new())) // cycle: already counted
            } else {
                match fns.get(fn_) {
                    None => None, // unknown function: wild
                    Some(outs) => {
                        let mut v: Vec<&str> = visited.to_vec();
                        v.push(fn_.as_str());
                        mconcat_atoms(interner, fns, &v, outs)
                    }
                }
            }
        }
    }
}

/// The `mconcat'` of `Prax.Relevance.cookedOutcomeAtoms`: sequence the per-outcome
/// results (any `None` ⇒ `None`), then concatenate the insert and delete sides.
fn mconcat_atoms(
    interner: &mut Interner,
    fns: &BTreeMap<String, Vec<Effect>>,
    visited: &[&str],
    outs: &[Effect],
) -> Option<(Vec<Names>, Vec<Names>)> {
    let mut ins = Vec::new();
    let mut del = Vec::new();
    for o in outs {
        let (i, d) = cooked_outcome_atoms(interner, fns, visited, o)?;
        ins.extend(i);
        del.extend(d);
    }
    Some((ins, del))
}

/// Positive and negated path patterns of a want's cooked conditions, plus an
/// "uncertain" flag (satisfaction depends on machinery beyond pattern presence)
/// — `Prax.Relevance.cookedWantPatterns`. `Absent` swaps polarity; `Exists`
/// keeps it; `Or` unions; `Calc`/`Count`/`Subquery` taint. Only the SET of
/// patterns and the flag are consumed (`any`/membership), so accumulation order
/// is unobservable.
fn cooked_want_patterns(conds: &[Cond]) -> (Vec<Names>, Vec<Names>, bool) {
    let mut pos: Vec<Names> = Vec::new();
    let mut neg: Vec<Names> = Vec::new();
    let mut unc = false;
    for c in conds {
        match c {
            Cond::Match(p) => pos.push(p.clone()),
            Cond::Not(p) => neg.push(p.clone()),
            Cond::Absent(cs) => {
                let (p2, n2, u2) = cooked_want_patterns(cs);
                pos.extend(n2);
                neg.extend(p2);
                unc |= u2;
            }
            Cond::Exists(cs) => {
                let (p2, n2, u2) = cooked_want_patterns(cs);
                pos.extend(p2);
                neg.extend(n2);
                unc |= u2;
            }
            Cond::Or(clauses) => {
                for cl in clauses {
                    let (p2, n2, u2) = cooked_want_patterns(cl);
                    pos.extend(p2);
                    neg.extend(n2);
                    unc |= u2;
                }
            }
            Cond::Eq(..) | Cond::Neq(..) | Cond::Cmp(..) => {}
            Cond::Calc(..) | Cond::Count(..) | Cond::Subquery { .. } => unc = true,
        }
    }
    (pos, neg, unc)
}

/// Every effect an authored MOVER action can cause, resolved once per world: the
/// insert- and delete-shaped atom pools over every action's outcomes plus every
/// practice's inits (`Prax.Relevance.worldAtomPools`), and whether any is "wild".
/// Ranges over the practices movers can take — NOT the schedule surface (a desire
/// only the schedule can improve has no improving mover action, so the static
/// screen stays exact and its liveness becomes a `GateCheck` the pulse flips).
struct AtomPools {
    inserted: Vec<Names>,
    deleted: Vec<Names>,
    wild: bool,
}

fn world_atom_pools(
    interner: &mut Interner,
    defs: &BTreeMap<String, CompiledPractice>,
    fns: &BTreeMap<String, Vec<Effect>>,
) -> AtomPools {
    let mut atoms: Vec<Option<(Vec<Names>, Vec<Names>)>> = Vec::new();
    for cp in defs.values() {
        for a in &cp.actions {
            for o in &a.outs {
                atoms.push(cooked_outcome_atoms(interner, fns, &[], o));
            }
        }
    }
    for cp in defs.values() {
        for o in &cp.inits {
            atoms.push(cooked_outcome_atoms(interner, fns, &[], o));
        }
    }
    let wild = atoms.iter().any(Option::is_none);
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();
    for a in atoms.into_iter().flatten() {
        inserted.extend(a.0);
        deleted.extend(a.1);
    }
    AtomPools {
        inserted,
        deleted,
        wild,
    }
}

/// Axiom heads count as derivable (`Prax.Relevance.axiomDerivable`): a want (or
/// gate candidate) over a derivable pattern is conservatively improvable / never
/// a gate. `heads` is [`axiom_head_patterns`] of the world's rules (□-lifted
/// forms included; the `contradiction` witness excluded, as frozen).
fn axiom_derivable(heads: &[Names], p: &[Sym]) -> bool {
    heads.iter().any(|h| may_unify_syms(p, h))
}

/// The names of the desires some authored action might improve
/// (`Prax.Relevance.improvableDesires`). The analysis is conservative: it answers
/// "not improvable" only when that is provable from the authored patterns.
pub(crate) fn improvable_desires(
    interner: &mut Interner,
    defs: &BTreeMap<String, CompiledPractice>,
    fn_pool: &BTreeMap<String, Vec<Effect>>,
    rules: &[CompiledRule],
    desires_cooked: &BTreeMap<String, Vec<Cond>>,
    desires: &[Desire],
) -> Vec<String> {
    let pools = world_atom_pools(interner, defs, fn_pool);
    let heads = axiom_head_patterns(rules);
    let mut out = Vec::new();
    for d in desires {
        let conds = &desires_cooked[&d.name];
        let (pos, neg, unc) = cooked_want_patterns(conds);
        let u = d.want.utility;
        let improvable = if u == 0 {
            false
        } else if pools.wild
            || unc
            || pos.iter().chain(neg.iter()).any(|p| axiom_derivable(&heads, p))
        {
            true
        } else if u > 0 {
            pools
                .inserted
                .iter()
                .any(|i| pos.iter().any(|p| may_unify_syms(i, p)))
                || pools
                    .deleted
                    .iter()
                    .any(|dl| neg.iter().any(|p| may_unify_syms(dl, p)))
        } else {
            pools
                .deleted
                .iter()
                .any(|dl| pos.iter().any(|p| may_unify_syms(dl, p)))
                || pools
                    .inserted
                    .iter()
                    .any(|i| neg.iter().any(|p| may_unify_syms(i, p)))
        };
        if improvable {
            out.push(d.name.clone());
        }
    }
    out
}

/// Classify every named desire's dead-now recipe (`Prax.Relevance.livenessOf`): a
/// negative want-kind is unconditionally [`Liveness::FloorCheck`]; a positive
/// want-kind gates on its top-level positive `Match` conjuncts that are neither
/// action-insertable nor axiom-derivable; weight 0 (screened statically first) is
/// [`Liveness::AlwaysLive`] defensively.
pub(crate) fn liveness_of(
    interner: &mut Interner,
    defs: &BTreeMap<String, CompiledPractice>,
    fn_pool: &BTreeMap<String, Vec<Effect>>,
    rules: &[CompiledRule],
    desires_cooked: &BTreeMap<String, Vec<Cond>>,
    desires: &[Desire],
) -> BTreeMap<String, Liveness> {
    let pools = world_atom_pools(interner, defs, fn_pool);
    let heads = axiom_head_patterns(rules);
    let mut out = BTreeMap::new();
    for d in desires {
        let u = d.want.utility;
        let lv = if u < 0 {
            Liveness::FloorCheck
        } else if u > 0 {
            positive_liveness(&desires_cooked[&d.name], &pools, &heads)
        } else {
            Liveness::AlwaysLive
        };
        out.insert(d.name.clone(), lv);
    }
    out
}

fn positive_liveness(conds: &[Cond], pools: &AtomPools, heads: &[Names]) -> Liveness {
    let (_, _, unc) = cooked_want_patterns(conds);
    let gates: Vec<Vec<Cond>> = conds
        .iter()
        .filter_map(|c| match c {
            Cond::Match(p) => Some(p),
            _ => None,
        })
        .filter(|p| {
            !pools.inserted.iter().any(|i| may_unify_syms(p, i)) && !axiom_derivable(heads, p)
        })
        .map(|p| vec![Cond::Match(p.clone())])
        .collect();
    if unc || pools.wild || gates.is_empty() {
        Liveness::AlwaysLive
    } else {
        Liveness::GateCheck(gates)
    }
}

/// Per character, the affordance templates whose authored outcomes could touch
/// their own wants or held desires (`Prax.Relevance.bearingTemplates`) — the
/// opportunity-relevance half of the v35 motive signature. Conservative: an
/// unresolvable `Call` bears on everyone.
pub(crate) fn bearing_templates(
    interner: &mut Interner,
    defs: &BTreeMap<String, CompiledPractice>,
    fn_pool: &BTreeMap<String, Vec<Effect>>,
    desires_cooked: &BTreeMap<String, Vec<Cond>>,
    desires: &[Desire],
    wants: &BTreeMap<String, Vec<Vec<Cond>>>,
    characters: &[Character],
) -> BTreeMap<String, Vec<String>> {
    let mut action_atoms: Vec<(String, Option<Vec<Names>>)> = Vec::new();
    for cp in defs.values() {
        for a in &cp.actions {
            action_atoms.push((a.name.clone(), action_atoms_of(interner, fn_pool, &a.outs)));
        }
    }
    let mut out = BTreeMap::new();
    for c in characters {
        let pats = char_read_patterns(c, wants, desires_cooked, desires);
        let bearing: Vec<String> = action_atoms
            .iter()
            .filter(|(_, m)| match m {
                None => true,
                Some(atoms) => atoms
                    .iter()
                    .any(|atom| pats.iter().any(|p| may_unify_syms(atom, p))),
            })
            .map(|(n, _)| n.clone())
            .collect();
        out.insert(c.name.clone(), bearing);
    }
    out
}

/// One action's atom pool (both sides concatenated) for [`bearing_templates`], or
/// `None` if any outcome is wild (`bearingTemplates`'s `atoms`).
fn action_atoms_of(
    interner: &mut Interner,
    fn_pool: &BTreeMap<String, Vec<Effect>>,
    outs: &[Effect],
) -> Option<Vec<Names>> {
    let mut all = Vec::new();
    for o in outs {
        let (i, d) = cooked_outcome_atoms(interner, fn_pool, &[], o)?;
        all.extend(i);
        all.extend(d);
    }
    Some(all)
}

/// A character's read anchors: the anchors of their `charWants` conditions plus
/// their held desires' cooked conditions (`bearingTemplates`'s `charPats`).
fn char_read_patterns(
    c: &Character,
    wants: &BTreeMap<String, Vec<Vec<Cond>>>,
    desires_cooked: &BTreeMap<String, Vec<Cond>>,
    desires: &[Desire],
) -> Vec<Names> {
    let mut conds: Vec<Cond> = Vec::new();
    if let Some(ws) = wants.get(&c.name) {
        for w in ws {
            conds.extend(w.iter().cloned());
        }
    }
    for d in desires {
        if c.desires.contains(&d.name) {
            conds.extend(desires_cooked[&d.name].iter().cloned());
        }
    }
    read_anchors(&conds)
}

/// Everything `Prax.Planner.predictMove` (scope gate included) can read when the
/// pick's actor predicts mover `m`, as pattern anchors grounded to the pair
/// (`Prax.Relevance.moverReadAnchors`): the prediction-scope template
/// (Actor:=actor, Witness:=m); the believed-model source family; the mover's
/// death mark; every practice's instance pattern, action conditions, and
/// outcome-embedded conditions (Actor:=m); every function case (fully wild); and
/// every vocabulary desire's conditions (Owner:=m).
///
/// [S-C1] The scope template is sourced from the already-compiled `scope`
/// (`Compiled.scope`), never re-cooked from raw `prediction_scope`: every
/// CONSTANT read anchor is thus interner-resident from recompile, so the delta-vs-
/// read may-unify the planner runs is a same-interner id compare (the single
/// planner interner makes the frozen "cross-lineage" hazard structurally absent).
/// The sole post-recompile read name is the `PraxD` wildcard — a variable, never
/// id-compared.
pub(crate) fn mover_read_anchors(
    interner: &mut Interner,
    scope: &[Cond],
    practices: &BTreeMap<String, CompiledPractice>,
    fns: &BTreeMap<String, CompiledFn>,
    desires_cooked: &BTreeMap<String, Vec<Cond>>,
    actor: &str,
    m: &str,
) -> Vec<Names> {
    let m_sym = interner.intern(m);
    let actor_sym = interner.intern(actor);
    let actor_key = interner.intern("Actor");
    let owner_key = interner.intern("Owner");
    let witness_key = interner.intern("Witness");
    let mut actor_b = Bindings::new();
    actor_b.insert(actor_key, Val::Sym(m_sym));
    let mut owner_b = Bindings::new();
    owner_b.insert(owner_key, Val::Sym(m_sym));
    let mut scope_b = Bindings::new();
    scope_b.insert(actor_key, Val::Sym(actor_sym));
    scope_b.insert(witness_key, Val::Sym(m_sym));

    let mut out: Vec<Names> = Vec::new();
    out.extend(reads_of(interner, &scope_b, scope));
    let believes = interner.intern("believes");
    let desires_seg = interner.intern("desires");
    let prax_d = interner.intern("PraxD");
    out.push(smallvec![actor_sym, believes, desires_seg, m_sym, prax_d]);
    // The death mark goes through the TOKENIZER, exactly as `candidate_actions`
    // and `living_characters` build it (`Prax.Relevance.deadRead` is
    // `pathNames (deadSentence …)`, which splits). A single-segment character
    // name (DIV-2's guard) makes the two spellings coincide; the port is
    // written so it would not matter if they did not.
    out.push(
        tokenize(interner, &format!("dead.{m}"))
            .expect("dead path")
            .segs,
    );
    for cp in practices.values() {
        out.push(ground_names(interner, &actor_b, &cp.instance_names).into());
        for ca in &cp.actions {
            out.extend(reads_of(interner, &actor_b, &ca.conds));
            out.extend(outcome_cond_reads(interner, &actor_b, &ca.outs));
        }
        out.extend(outcome_cond_reads(interner, &Bindings::new(), &cp.inits));
    }
    for (_params, cases) in fns.values() {
        for (cs, os) in cases {
            out.extend(reads_of(interner, &Bindings::new(), cs));
            out.extend(outcome_cond_reads(interner, &Bindings::new(), os));
        }
    }
    for conds in desires_cooked.values() {
        out.extend(reads_of(interner, &owner_b, conds));
    }
    out
}

/// `cookedReadAnchors (map (groundCookedCondition b) conds)` — ground then anchor.
fn reads_of(interner: &mut Interner, b: &Bindings, conds: &[Cond]) -> Vec<Names> {
    let grounded: Vec<Cond> = conds.iter().map(|c| ground_cond(interner, b, c)).collect();
    read_anchors(&grounded)
}

/// Conditions embedded in outcomes (`ForEach`/`Roll` guards, recursively) — the
/// imagined apply queries these (`Prax.Relevance.outcomeCondReads`).
fn outcome_cond_reads(interner: &mut Interner, b: &Bindings, outs: &[Effect]) -> Vec<Names> {
    let mut out = Vec::new();
    for o in outs {
        match o {
            Effect::ForEach(cs, os) | Effect::Roll(_, _, cs, os) => {
                out.extend(reads_of(interner, b, cs));
                out.extend(outcome_cond_reads(interner, b, os));
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    // H: RelevanceSpec.hs "Prax.Relevance"
    //
    // The frozen `Prax.RelevanceSpec`, re-expressed against the Rust analysis.
    // Its five villageWorld-driven cases (the improvable table, the state field,
    // delta relevance, monotone-insert classification and the liveness field)
    // wait on the shipped world at S7 and are recorded in KILLED.md; every
    // synthetic case is below, alongside the two primitives' own unit tests.
    use super::*;
    use crate::path::tokenize;

    fn segs(i: &mut Interner, s: &str) -> Vec<Sym> {
        tokenize(i, s).unwrap().segs.to_vec()
    }

    // The death mark in `mover_read_anchors` is built by the SAME tokenizer that
    // `candidate_actions` and `living_characters` use, so the three planner sites
    // agree on how a name segments. The engine door forbids a separator-bearing
    // character name (DIV-2), so this is unobservable through `State` — it is
    // pinned here because the port must not be internally inconsistent where a
    // guard happens to hide the difference.
    #[test]
    fn the_death_read_anchor_is_tokenized_like_every_other_death_sentence() {
        let mut i = Interner::new();
        let anchors = mover_read_anchors(
            &mut i,
            &[],
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
            "olaf",
            "hall.keeper",
        );
        let want = segs(&mut i, "dead.hall.keeper");
        assert!(
            anchors.iter().any(|a| a.as_slice() == want.as_slice()),
            "the death anchor must SPLIT the mover name (3 segments: dead.hall.keeper), \
             exactly as candidate_actions/living_characters tokenize it; got {:?}",
            anchors
                .iter()
                .map(|a| a.len())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn may_unify_needs_a_shared_literal_anchor() {
        let mut i = Interner::new();
        // Same predicate literal, variable tail: unifies.
        let a = segs(&mut i, "lied.X");
        let b = segs(&mut i, "lied.bob");
        assert!(may_unify_syms(&a, &b));
        // Different predicate literals: no unify.
        let c = segs(&mut i, "regards.X");
        assert!(!may_unify_syms(&a, &c));
        // All-variable overlap: no literal anchor, evidence-free -> no unify.
        let v1 = segs(&mut i, "X.Y");
        let v2 = segs(&mut i, "P.bob");
        assert!(!may_unify_syms(&v1, &v2));
        // A conflicting literal at an aligned position blocks the unify.
        let d = segs(&mut i, "at.bar");
        let e = segs(&mut i, "at.mill");
        assert!(!may_unify_syms(&d, &e), "aligned differing literals cannot unify");
    }

    #[test]
    fn may_unify_is_prefix_compatible() {
        let mut i = Interner::new();
        // A shorter Match pattern sees a longer fact's subtree.
        let short = segs(&mut i, "practice.tendBar");
        let long = segs(&mut i, "practice.tendBar.ada.customer.beth");
        assert!(may_unify_syms(&short, &long));
    }

    // ---- RelevanceSpec (synthetic fixtures) --------------------------------

    // The frozen case's own three fixtures, on deep paths: a variable-strewn
    // pattern against its fully concrete instance, a shorter pattern against a
    // longer insert (a `Match` sees subtrees), and two patterns whose leading
    // literals differ. The two unit tests above use shallow paths and probe the
    // clauses one at a time; this one pins the frozen assertions verbatim.
    // H: RelevanceSpec.hs "mayUnifySyms: variables are wildcards, prefixes are compatible"
    #[test]
    fn may_unify_syms_wildcards_and_prefixes() {
        let mut i = Interner::new();
        let mut u = |a: &str, b: &str| {
            let (x, y) = (segs(&mut i, a), segs(&mut i, b));
            may_unify_syms(&x, &y)
        };
        assert!(
            u("lied.Actor.H.stole.C.loaf", "lied.eve.dana.stole.carol.loaf"),
            "var vs concrete"
        );
        assert!(
            u("Hearer.believes.took.Culprit.gem.heard.Actor", "oz.believes.took.kit.gem"),
            "prefix compatibility (longer insert, shorter pattern)"
        );
        assert!(
            !u("regards.W.carol.thief", "practice.earnBread.Owner.did.P"),
            "distinct constants do not unify"
        );
    }

    mod spec {
        use crate::engine::State;
        use crate::query::{Condition, not_, subquery};
        use crate::relevance::Liveness;
        use crate::types::{Action, Axiom, Character, Desire, Function, Practice, ScheduleRule, Want, insert};

        fn m(s: &str) -> Condition {
            Condition::Match(s.into())
        }

        // An exclusion insert counts as evicting ANY sibling on the delete side:
        // a negative want on the displaced value is improvable only through that
        // eviction, and the victim's name appears in no outcome.
        // H: RelevanceSpec.hs "an exclusion insert counts as evicting ANY sibling on the delete side"
        #[test]
        fn improvable_via_eviction_shadow() {
            let shrine = Practice::new("shrine").roles(["R"]).action(
                Action::new("[Actor]: enshrine the gem")
                    .when([m("slot.stone")])
                    .then([insert("slot!gem")]),
            );
            let mut st = State::new();
            st.define_practices([shrine]).unwrap();
            st.set_desires(vec![Desire::new(
                "hates-the-stone",
                Want::new(vec![m("slot.stone")], -2),
            )])
            .unwrap();
            assert_eq!(st.improvables(), ["hates-the-stone"]);
        }

        // Two exclusion points: the first eviction clears everything under
        // altar (arbitrary depth and shape), including branches that diverge
        // from the insert's own path right after the '!'.
        // H: RelevanceSpec.hs "eviction covers the WHOLE displaced subtree, not just the shadow's shape"
        #[test]
        fn improvable_eviction_covers_whole_subtree() {
            let temple = Practice::new("temple").roles(["R"]).action(
                Action::new("[Actor]: rededicate the altar")
                    .when([m("shrine.here")])
                    .then([insert("altar!new.rite!dawn")]),
            );
            let mut st = State::new();
            st.define_practices([temple]).unwrap();
            st.set_desires(vec![Desire::new(
                "mourns-the-relic",
                Want::new(vec![m("altar.old.relic.jade")], -2),
            )])
            .unwrap();
            assert_eq!(st.improvables(), ["mourns-the-relic"]);
        }

        /// The one gate table entry, rendered by name: `(tag, gates)` where each
        /// gate is its conjunct patterns dot-joined — the frozen
        /// `GateCheck [[cookCondition (Match "…")]]` equality, conjunct text
        /// included rather than merely counted.
        fn rendered(st: &State, name: &str) -> (String, Vec<Vec<String>>) {
            st.liveness_rendered()
                .remove(name)
                .unwrap_or_else(|| panic!("no liveness entry for {name}"))
        }

        // A negative want-kind is a floor check whatever the world affords: the
        // recipe never consults the atom pools.
        // H: RelevanceSpec.hs "livenessOf: a negative desire is FloorCheck unconditionally"
        #[test]
        fn liveness_negative_desire_is_floor_check() {
            let mut st = State::new();
            st.set_desires(vec![Desire::new("hates-mud", Want::new(vec![m("muddy.Owner")], -3))])
                .unwrap();
            assert_eq!(st.liveness_of("hates-mud"), Some(&Liveness::FloorCheck));
        }

        // Weight 0 is screened out statically long before liveness runs; the
        // defensive branch still has to answer, and answers AlwaysLive rather
        // than gating on a conjunct nobody will consult.
        // H: RelevanceSpec.hs "livenessOf: a weight-0 desire is AlwaysLive (defensive; screened statically first)"
        #[test]
        fn liveness_weight_zero_is_always_live() {
            let mut st = State::new();
            st.set_desires(vec![Desire::new("indifferent", Want::new(vec![m("whatever.Owner")], 0))])
                .unwrap();
            assert_eq!(st.liveness_of("indifferent"), Some(&Liveness::AlwaysLive));
        }

        // The only action in this world inserts meal.*, never hungry.* — so
        // "hungry.Owner" is environment-gated (no authored outcome can raise it)
        // while "meal.M" is action-insertable and so is NOT a gate. The gate list
        // is exactly the one conjunct, by name.
        // H: RelevanceSpec.hs "livenessOf: a positive desire with a ticker-only conjunct gates on it alone"
        #[test]
        fn liveness_positive_gates_on_the_ticker_only_conjunct() {
            let bakery = Practice::new("bakery").roles(["R"]).action(
                Action::new("[Actor]: bake")
                    .when([m("practice.bakery.here")])
                    .then([insert("meal.bread")]),
            );
            let mut st = State::new();
            st.define_practices([bakery]).unwrap();
            st.set_desires(vec![Desire::new(
                "pursues-lunch",
                Want::new(vec![m("hungry.Owner"), m("meal.M")], 5),
            )])
            .unwrap();
            assert_eq!(
                rendered(&st, "pursues-lunch"),
                ("GateCheck".to_owned(), vec![vec!["hungry.Owner".to_owned()]])
            );
        }

        // "hungry.Owner" is never Inserted, but an axiom's head unifies it, so it
        // is conservatively excluded from gating — and no other conjunct
        // qualifies, so the whole want stays AlwaysLive.
        // H: RelevanceSpec.hs "livenessOf: an axiom-derivable candidate gate never qualifies (conservative)"
        #[test]
        fn liveness_axiom_derivable_candidate_never_gates() {
            let mut st = State::new();
            st.set_axioms(vec![Axiom::new(vec![m("starving.Owner")], ["hungry.Owner"])])
                .unwrap();
            st.set_desires(vec![Desire::new("pursues-food", Want::new(vec![m("hungry.Owner")], 5))])
                .unwrap();
            assert_eq!(st.liveness_of("pursues-food"), Some(&Liveness::AlwaysLive));
        }

        // Satisfaction depends on machinery beyond pattern presence, so no cheap
        // state test can rule the want dead: uncertainty always wins.
        // H: RelevanceSpec.hs "livenessOf: a Subquery-bearing want is AlwaysLive (uncertainty always wins)"
        #[test]
        fn liveness_subquery_bearing_want_is_always_live() {
            let mut st = State::new();
            st.set_desires(vec![Desire::new(
                "counts-friends",
                Want::new(vec![subquery("Fs", vec!["F".into()], vec![m("friend.Owner.F")])], 5),
            )])
            .unwrap();
            assert_eq!(st.liveness_of("counts-friends"), Some(&Liveness::AlwaysLive));
        }

        /// The plaza world both schedule cases share: a stroll affordance, a
        /// festival schedule rule that inserts festive.now, and a want over both
        /// conjuncts. `also_person` adds a PERSON action inserting festive.now.
        fn plaza(also_person: bool) -> State {
            let mut p = Practice::new("plaza").roles(["R"]).action(
                Action::new("[Actor]: stroll the plaza")
                    .when([m("practice.plaza.here")])
                    .then([insert("strolled.Actor")]),
            );
            if also_person {
                p = p.action(
                    Action::new("[Actor]: light the lanterns")
                        .when([m("practice.plaza.here")])
                        .then([insert("festive.now")]),
                );
            }
            let mut st = State::new();
            st.define_practices([p]).unwrap();
            st.set_characters(vec![Character::new("ana")]).unwrap();
            st.set_desires(vec![Desire::new(
                "loves-a-crowd",
                Want::new(vec![m("festive.now"), m("strolled.Owner")], 3),
            )])
            .unwrap();
            st.set_schedule(vec![
                ScheduleRule::new("festival", 4).clause(vec![], vec![insert("festive.now")]),
            ])
            .unwrap();
            st
        }

        // "festive.now" is inserted ONLY by a schedule rule; the desire needs it
        // plus an action-reachable conjunct. The schedule lives off the mover
        // surface, so it never pollutes the insert pool — festive.now is the sole
        // GateCheck conjunct, and strolled.Owner (action-insertable) is not.
        // H: RelevanceSpec.hs "schedule-moved facts are environment gates (the schedule is not a mover)"
        #[test]
        fn schedule_moved_facts_are_environment_gates() {
            let st = plaza(false);
            assert_eq!(
                rendered(&st, "loves-a-crowd"),
                ("GateCheck".to_owned(), vec![vec!["festive.now".to_owned()]])
            );
        }

        // Same shape, but a PERSON action also inserts festive.now (lighting the
        // lanterns) — the mover pool sees it via that authored outcome, so no
        // conjunct qualifies: AlwaysLive, conservative as ever. The schedule
        // cannot launder an action-insertable fact into a gate.
        // H: RelevanceSpec.hs "action-insertable facts still never gate; the schedule cannot launder them"
        #[test]
        fn action_insertable_facts_still_never_gate() {
            let st = plaza(true);
            assert_eq!(st.liveness_of("loves-a-crowd"), Some(&Liveness::AlwaysLive));
        }

        // The eatery shape: eating only inserts meal.Actor, never hungry.* —
        // ONLY the schedule's guarded hunger rule inserts hungry.*. Because the
        // schedule is off the mover surface, the clock-moved hungry.Owner keeps
        // its GateCheck; meal.M, which the eat action does insert, does not
        // qualify.
        // H: RelevanceSpec.hs "the village hunger want-shape regains its gate under the reclassification"
        #[test]
        fn hunger_want_shape_regains_its_gate() {
            let eatery = Practice::new("eatery").roles(["R"]).action(
                Action::new("[Actor]: eat")
                    .when([m("hungry.Actor")])
                    .then([insert("meal.Actor")]),
            );
            let mut st = State::new();
            st.define_practices([eatery]).unwrap();
            st.set_characters(vec![Character::new("bob")]).unwrap();
            st.set_desires(vec![Desire::new(
                "wants-food",
                Want::new(vec![m("hungry.Owner"), m("meal.M")], 5),
            )])
            .unwrap();
            st.set_schedule(vec![ScheduleRule::new("hunger", 3).clause(
                vec![m("appetite.X"), not_("hungry.X")],
                vec![insert("hungry.X")],
            )])
            .unwrap();
            assert_eq!(
                rendered(&st, "wants-food"),
                ("GateCheck".to_owned(), vec![vec!["hungry.Owner".to_owned()]])
            );
        }

        // moverReadAnchors: scope, believes, death, affordances (incl. ForEach and
        // function-body guards), desires — all grounded to the pair, never to the
        // predictor.
        // H: RelevanceSpec.hs "moverReadAnchors: scope, believes, death, affordances, desires — grounded to the pair"
        #[test]
        fn mover_read_anchors_grounds_to_the_pair() {
            let eatery = Practice::new("eatery")
                .roles(["R"])
                .action(
                    Action::new("[Actor]: eat")
                        .when([m("hungry.Actor")])
                        .then([
                            crate::types::for_each(vec![m("crumb.C")], vec![crate::types::delete("crumb.C")]),
                            insert("meal.Actor"),
                        ]),
                )
                .action(Action::new("[Actor]: clean up").then([crate::types::call("tidy", vec!["Actor".into()])]));
            let tidy = Function::new("tidy", ["Who"]).case(
                vec![],
                vec![crate::types::for_each(vec![m("dish.D")], vec![crate::types::delete("dish.D")])],
            );
            let mut st = State::new();
            st.define_practices([eatery]).unwrap();
            st.define_functions([tidy]).unwrap();
            st.set_characters(vec![Character::new("priya"), Character::new("beth")]).unwrap();
            st.set_desires(vec![Desire::new("wants-food", Want::new(vec![m("hungry.Owner")], 5))])
                .unwrap();
            let anchors = st.mover_read_anchors_of("priya", "beth");
            let has = |st: &mut State, s: &str| anchors.contains(&st.intern_segs(s));
            assert!(has(&mut st, "priya.believes.desires.beth.PraxD"), "believes family");
            assert!(has(&mut st, "dead.beth"), "death mark");
            assert!(has(&mut st, "hungry.beth"), "affordance/desire cond, Actor/Owner:=beth");
            assert!(has(&mut st, "crumb.C"), "ForEach guard read");
            assert!(has(&mut st, "dish.D"), "function-body ForEach guard read");
            assert!(!has(&mut st, "hungry.priya"), "NOT grounded to the predictor");
        }
    }

    #[test]
    fn eviction_shadows_one_per_bang_truncated_with_wildcard() {
        let mut i = Interner::new();
        let evicted = i.intern("PraxEvicted");
        // `a.b!c!d`: `!` after b (index 1) and after c (index 2).
        let path = tokenize(&mut i, "a.b!c!d").unwrap();
        let shadows = eviction_shadow_names(&mut i, &path);
        let a = i.intern("a");
        let b = i.intern("b");
        let c = i.intern("c");
        let want1: SmallVec<[Sym; 6]> = smallvec::smallvec![a, b, evicted];
        let want2: SmallVec<[Sym; 6]> = smallvec::smallvec![a, b, c, evicted];
        assert_eq!(shadows, vec![want1, want2]);
        // A `!`-free path has no shadows.
        let plain = tokenize(&mut i, "a.b.c").unwrap();
        assert!(eviction_shadow_names(&mut i, &plain).is_empty());
    }
}
