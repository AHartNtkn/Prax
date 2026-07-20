//! Deontic Exclusion Logic — a first-class "should"/obligation layer (Evans,
//! "Introducing Exclusion Logic as a Deontic Logic", DEON 2010).
//!
//! The paper's result is that a deontic logic needs NO new semantics over
//! Exclusion Logic: `□P` ("P should be the case") is syntactic sugar for the
//! ordinary sentence `Ob:P`. Our world DB is already an Exclusion-Logic model
//! (our `.` is the paper's multi-valued operator; our `!` its exclusion), so this
//! module adds NO machinery — like [`crate::core_model`] and [`crate::reactions`]
//! it is a library of smart constructors over the existing engine.
//!
//! An obligation "`who` should bring about `content`" is the fact
//! `obliged.<who>.<content>` (mirroring [`crate::reactions`]'s
//! `violated.<who>.<norm>`). Obligations are multi-valued, so one agent may hold
//! many at once. Iterated `□□` — the reparative / contrary-to-duty obligation —
//! is the nested fact `obliged.<who>.obliged.<who>.<content>`
//! ([`oblige_reparative`]).
//!
//! **Stratification is the whole point.** It is what avoids the
//! Standard-Deontic-Logic paradoxes (Ross's, "tautologies are obligatory",
//! Chisholm): `□` applies only to a SIMPLE TERM, never to `∧`/`∨`/`→`. So
//! [`oblige`]/[`is_obliged`] take a plain sentence `&str`, never a
//! `&[Condition]` — do not relax this.
//!
//! The vocabulary CONSTANTS (`obliged.*` head, the lifted prefix, and the
//! `obligationPath` builder) live in [`prax_core::vocab_consts`] —
//! checker-visible without a crate cycle (design C1). They are USED from there
//! rather than redefined or re-exported here: one home, no dual system. The
//! OPERATORS are this module.
//!
//! **The computing half touches the engine at runtime.** [`conflicts`] and
//! [`incompatible_pairs`] run a scratch [`Db`] over a PRIVATE THROWAWAY
//! [`Interner`] (no id escapes it), and [`obligations_of`] reads a `Db` through
//! the CALLER's interner. This is the one place `prax-vocab` is not purely a
//! value-builder, and it is deliberate: the frozen module computes over
//! `Prax.Db` too. Do not "clean it up" into a string predicate — the `!`
//! exclusion semantics that make [`conflicts`] meaningful live in the trie.
//!
//! Frozen reference: `src/Prax/Deontic.hs`. Signature for signature, with one
//! recorded consequence: `Prax.Db.insertAll`/`exists` raise on a malformed
//! sentence, so [`conflicts`] and [`incompatible_pairs`] are `Result`-returning
//! here — the frozen partiality, made visible instead of `unwrap`ped ([S-I5]).

use prax_core::db::Db;
use prax_core::error::WorldError;
use prax_core::interner::Interner;
use prax_core::query::{Condition, matches};
use prax_core::types::{Axiom, Outcome, Want, delete, insert};
use prax_core::vocab_consts::{OBLIGED_HEAD, OBLIGED_LIFT_PREFIX, obligation_path};

use crate::reactions::{mark_violation, violation_of};

/// Prefix every `□`-lifted sentence with `obliged.Obligor.`.
fn lift_sent(s: &str) -> String {
    format!("{OBLIGED_LIFT_PREFIX}{s}")
}

/// Lift a purely-conjunctive domain rule under the obligation operator
/// (`Prax.Deontic.obligedLift`): prefix `obliged.Obligor.` to every body `Match`
/// and every head, so `□A ⊢ □B` whenever `A ⊢ B`. A rule whose body uses any
/// non-`Match` condition is not lifted (nothing sensible to place under `□`) —
/// `None`.
pub fn obliged_lift(ax: &Axiom) -> Option<Axiom> {
    if ax.when.iter().all(|c| matches!(c, Condition::Match(_))) {
        let when = ax
            .when
            .iter()
            .map(|c| match c {
                Condition::Match(s) => Condition::Match(lift_sent(s)),
                other => other.clone(),
            })
            .collect();
        let then = ax.then.iter().map(|s| lift_sent(s)).collect();
        Some(Axiom { when, then })
    } else {
        None
    }
}

/// Close a world's axioms under the obligation operator (DEON property 1): the
/// authored rules plus the `□`-lifted twin of every all-`Match` rule
/// (`Prax.Deontic.obligedClose`). A deontic world declares its closure with
/// `set_axioms(obliged_close(&rules))`; the general engine closes over exactly
/// this list, lift included.
pub fn obliged_close(axs: &[Axiom]) -> Vec<Axiom> {
    let mut out: Vec<Axiom> = axs.to_vec();
    out.extend(axs.iter().filter_map(obliged_lift));
    out
}

// Asserting / discharging ----------------------------------------------------

/// `oblige who content` — assert that `who` should bring about `content`
/// (`□content`).
pub fn oblige(who: &str, content: &str) -> Outcome {
    insert(obligation_path(who, content))
}

/// Retract an obligation (it has been met, or is cancelled).
pub fn discharge(who: &str, content: &str) -> Outcome {
    delete(obligation_path(who, content))
}

/// Record that `who` left a duty unmet. A breach IS a norm violation, so this
/// reuses [`crate::reactions`] verbatim (an agent given a negative
/// [`avoid_breach`] want steers clear of it, exactly as with any violation).
pub fn breach(who: &str, norm: &str) -> Outcome {
    mark_violation(who, norm)
}

/// The reparative / contrary-to-duty obligation `□□content` — "ideally you would
/// have brought about `content`; failing that, you now ought to". It is an
/// obligation ABOUT an obligation: the nested fact
/// `obliged.<who>.obliged.<who>.<content>`.
pub fn oblige_reparative(who: &str, content: &str) -> Outcome {
    oblige(who, &obligation_path(who, content))
}

// Querying -------------------------------------------------------------------

/// Condition: `who` is currently obliged to bring about `content`.
pub fn is_obliged(who: &str, content: &str) -> Condition {
    matches(obligation_path(who, content))
}

/// Conditions for a MET obligation: `who` is obliged to `content` AND it
/// actually holds. Ought does not collapse to is (paper §2.3, property 3), so
/// both conjuncts are required.
pub fn fulfilled(who: &str, content: &str) -> Vec<Condition> {
    vec![is_obliged(who, content), matches(content)]
}

/// Condition: `who` is in breach of the duty `content` (a recorded violation).
pub fn in_breach(who: &str, norm: &str) -> Condition {
    violation_of(who, norm)
}

// Norm-conflict detection ----------------------------------------------------

/// The paper's incompatibility test (property 2): are `c1` and `c2` jointly
/// unsatisfiable? Assert both into a scratch model; they conflict iff one
/// `!`-clears the other so they cannot both survive (e.g.
/// `conflicts("go!true", "go!false") == Ok(true)`). Symmetric; `false` for
/// identical or compatible contents. Pure and cheap (the DB is persistent).
///
/// There is deliberately no world-seeded variant: our `!` exclusion is applied
/// per-insert rather than as a persistent schema, so a live world can never
/// change whether two contents mutually conflict.
///
/// The [`Interner`] is minted here and dropped here — no `Sym` crosses the
/// boundary, so there is one lineage and no cross-lineage comparison [S-I5]. The
/// scratch DB never leaves either.
///
/// # Errors
/// [`WorldError`] if either content is not a well-formed sentence — the frozen
/// `Prax.Db.insertAll`/`exists` raise there, and that partiality is surfaced
/// rather than `unwrap`ped.
pub fn conflicts(c1: &str, c2: &str) -> Result<bool, WorldError> {
    let mut scratch = Interner::new();
    let d = Db::empty()
        .insert_str(&mut scratch, c1)?
        .insert_str(&mut scratch, c2)?;
    Ok(!(d.exists_str(&mut scratch, c1)? && d.exists_str(&mut scratch, c2)?))
}

/// Every incompatible pair among a list of candidate contents — for vetting a
/// PROPOSED set of duties for internal consistency before assigning them (two
/// genuinely incompatible obligations cannot be co-held, so this checks
/// candidates, not the live DB).
///
/// Pairs are produced in the frozen order: for each suffix `(a : rest)` of the
/// input, every `b` in `rest` with `conflicts a b`.
///
/// # Errors
/// Propagates [`conflicts`]' rejection of a malformed content.
pub fn incompatible_pairs(cs: &[&str]) -> Result<Vec<(String, String)>, WorldError> {
    let mut out = Vec::new();
    for (idx, a) in cs.iter().enumerate() {
        for b in &cs[idx + 1..] {
            if conflicts(a, b)? {
                out.push(((*a).to_owned(), (*b).to_owned()));
            }
        }
    }
    Ok(out)
}

/// The contents `who` is currently obliged to (for introspection / a drama
/// manager). Note: reading back from the trie flattens the `!`/`.` distinction,
/// so results are for DISPLAY, not to be fed back into [`conflicts`].
///
/// Frozen parameter order (`who`, then the db) is preserved; the caller's
/// [`Interner`] is appended because rendering a `Db` needs one and `prax-vocab`
/// mints none of its own except [`conflicts`]' throwaway.
pub fn obligations_of(who: &str, db: &Db, interner: &Interner) -> Vec<String> {
    let prefix = format!("{OBLIGED_HEAD}.{who}.");
    db.to_sentences(interner)
        .into_iter()
        .filter_map(|s| s.strip_prefix(&prefix).map(str::to_owned))
        .collect()
}

// Behavioural coupling -------------------------------------------------------

/// A want to FULFIL an obligation: utility `k` per met duty. Feeds the unchanged
/// planner, which then pursues the duty.
pub fn want_fulfilled(who: &str, content: &str, k: i32) -> Want {
    Want::new(fulfilled(who, content), k)
}

/// A want to AVOID BREACHING a duty: applied as strong-negative `-|k|` so the
/// planner steers away from the breach (the mechanism by which norms already
/// shape behaviour in `prax_worlds::bar`).
pub fn avoid_breach(who: &str, content: &str, k: i32) -> Want {
    Want::new(vec![in_breach(who, content)], -k.abs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_core::db::Db;
    use prax_core::derive::{CompiledRule, axiom_footprint, close};
    use prax_core::engine::State;
    use prax_core::interner::{Interner, Sym};
    use prax_core::path::tokenize;
    use prax_core::types::{Action, Character, Practice};
    use smallvec::SmallVec;

    fn m(s: &str) -> Condition {
        Condition::Match(s.to_owned())
    }

    fn compile_rules(i: &mut Interner, axs: &[Axiom]) -> Vec<CompiledRule> {
        axs.iter()
            .map(|ax| {
                let heads: Vec<&str> = ax.then.iter().map(String::as_str).collect();
                CompiledRule::compile(i, &ax.when, &heads).unwrap()
            })
            .collect()
    }

    fn has_path(i: &mut Interner, ps: &[SmallVec<[Sym; 6]>], s: &str) -> bool {
        let segs = tokenize(i, s).unwrap().segs;
        ps.contains(&segs)
    }

    fn build(i: &mut Interner, facts: &[&str]) -> Db {
        let mut db = Db::empty();
        for f in facts {
            db = db.insert_str(i, f).unwrap();
        }
        db
    }

    // H: DeonticSpec.hs "Prax.Deontic"
    //
    // The frozen `Prax.DeonticSpec`, re-expressed against the Rust engine. The
    // four entailment-closure labels were carried by an unpinned sanity check
    // while only the S4 operator was in scope; DeonticSpec.hs joins the
    // allowlist at S7 slice 3, so they are pinned to their own tests below.

    // H: DeonticSpec.hs "entailment closure (□ property 1)"
    // H: DeonticSpec.hs "obligedHead is the shared vocabulary constant"
    #[test]
    fn obliged_head_is_the_shared_vocabulary_constant() {
        assert_eq!(OBLIGED_HEAD, "obliged");
        assert_eq!(
            obligation_path("bex", "settle.up"),
            format!("{OBLIGED_HEAD}.bex.settle.up")
        );
    }

    // H: DeonticSpec.hs "obligedLift lifts an all-Match rule to the obliged.Obligor shape"
    #[test]
    fn obliged_lift_lifts_an_all_match_rule() {
        assert_eq!(
            obliged_lift(&Axiom::new(vec![m("at.W.bar")], ["in.W.building"])),
            Some(Axiom::new(
                vec![m("obliged.Obligor.at.W.bar")],
                ["obliged.Obligor.in.W.building"]
            ))
        );
    }

    // H: DeonticSpec.hs "obligedLift refuses a rule whose body uses a non-Match condition"
    #[test]
    fn obliged_lift_refuses_a_non_match_body() {
        assert_eq!(
            obliged_lift(&Axiom::new(
                vec![m("at.W.bar"), Condition::Not("dead.W".into())],
                ["in.W.building"]
            )),
            None
        );
    }

    // H: DeonticSpec.hs "obligedClose = the base rules plus the lifted twin of each all-Match rule"
    #[test]
    fn obliged_close_is_the_base_rules_plus_the_lifted_twins() {
        let liftable = Axiom::new(vec![m("a.X")], ["b.X"]);
        let unliftable = Axiom::new(vec![m("c.Y"), Condition::Not("d.Y".into())], ["e.Y"]);
        assert_eq!(
            obliged_close(&[liftable.clone(), unliftable.clone()]),
            vec![
                liftable,
                unliftable,
                Axiom::new(vec![m("obliged.Obligor.a.X")], ["obliged.Obligor.b.X"]),
            ]
        );
    }

    // ---- fact conventions --------------------------------------------------

    // H: DeonticSpec.hs "fact conventions"
    // H: DeonticSpec.hs "obligationPath is obliged.<who>.<content>"
    #[test]
    fn obligation_path_is_obliged_who_content() {
        assert_eq!(obligation_path("bex", "settle.up"), "obliged.bex.settle.up");
    }

    // H: DeonticSpec.hs "isObliged matches the obligation fact"
    #[test]
    fn is_obliged_matches_the_obligation_fact() {
        assert_eq!(is_obliged("bex", "settle.up"), m("obliged.bex.settle.up"));
    }

    // H: DeonticSpec.hs "inBreach is a violation (breach reuses the norm machinery)"
    #[test]
    fn in_breach_is_a_violation() {
        assert_eq!(
            in_breach("bex", "settle.up"),
            violation_of("bex", "settle.up")
        );
    }

    // ---- obligation lifecycle ----------------------------------------------

    /// Apply outcomes to a fresh state, the frozen `applyAll` over `emptyState`.
    fn apply_all(outs: &[Outcome]) -> State {
        let mut st = State::new();
        for o in outs {
            st.perform_outcome(o).expect("a deontic outcome");
        }
        st
    }

    /// The frozen `sat st conds = satisfies (db st) conds Map.empty`, for the
    /// ground `Match` conditions this spec builds. Any other shape is a
    /// re-expression error, so it panics loudly rather than silently passing.
    fn sat(st: &mut State, conds: &[Condition]) -> bool {
        conds.iter().all(|c| match c {
            Condition::Match(s) => st.db_has(s),
            other => panic!("this spec only asserts ground Match conditions, got {other:?}"),
        })
    }

    // H: DeonticSpec.hs "obligation lifecycle"
    // H: DeonticSpec.hs "oblige asserts, discharge retracts"
    #[test]
    fn oblige_asserts_discharge_retracts() {
        let mut st = apply_all(&[oblige("bex", "settle.up")]);
        assert!(
            st.labeled_facts()
                .contains(&"obliged.bex.settle.up".to_owned()),
            "the obligation fact is asserted"
        );
        assert!(sat(&mut st, &[is_obliged("bex", "settle.up")]));
        st.perform_outcome(&discharge("bex", "settle.up"))
            .expect("discharging");
        assert!(
            !st.labeled_facts()
                .contains(&"obliged.bex.settle.up".to_owned()),
            "the obligation is retracted"
        );
    }

    // H: DeonticSpec.hs "breach records a violation matched by inBreach"
    #[test]
    fn breach_records_a_violation_matched_by_in_breach() {
        let mut st = apply_all(&[breach("bex", "stiffed")]);
        assert!(
            st.labeled_facts()
                .contains(&"violated.bex.stiffed".to_owned()),
            "the violation fact"
        );
        assert!(sat(&mut st, &[in_breach("bex", "stiffed")]));
    }

    // H: DeonticSpec.hs "fulfilled needs the obligation AND its content (ought is not is)"
    #[test]
    fn fulfilled_needs_the_obligation_and_its_content() {
        let mut obliged_only = apply_all(&[oblige("bex", "tipped.ada")]);
        let mut met = apply_all(&[oblige("bex", "tipped.ada"), insert("tipped.ada")]);
        assert!(
            !sat(&mut obliged_only, &fulfilled("bex", "tipped.ada")),
            "unmet while the content is absent — ought does not collapse to is"
        );
        assert!(sat(&mut met, &fulfilled("bex", "tipped.ada")));
    }

    // ---- contrary-to-duty --------------------------------------------------

    // H: DeonticSpec.hs "contrary-to-duty (iterated box)"
    // H: DeonticSpec.hs "obligeReparative nests the obligation (square-square)"
    #[test]
    fn oblige_reparative_nests_the_obligation() {
        let mut st = apply_all(&[oblige_reparative("bex", "make.amends")]);
        assert!(
            st.labeled_facts()
                .contains(&"obliged.bex.obliged.bex.make.amends".to_owned()),
            "the nested reparative duty, got {:?}",
            st.labeled_facts()
        );
        assert!(
            sat(
                &mut st,
                &[is_obliged("bex", &obligation_path("bex", "make.amends"))]
            ),
            "matchable as an obligation-about-an-obligation"
        );
    }

    // ---- conflict detection ------------------------------------------------

    // H: DeonticSpec.hs "conflict detection (paper property 2)"
    // H: DeonticSpec.hs "incompatible: exclusive values of one slot collapse"
    #[test]
    fn incompatible_exclusive_values_of_one_slot_collapse() {
        assert_eq!(conflicts("go!true", "go!false"), Ok(true));
    }

    // H: DeonticSpec.hs "compatible: multi-valued siblings coexist"
    #[test]
    fn compatible_multi_valued_siblings_coexist() {
        assert_eq!(conflicts("likes.a", "likes.b"), Ok(false));
    }

    // H: DeonticSpec.hs "unrelated slots never conflict"
    #[test]
    fn unrelated_slots_never_conflict() {
        assert_eq!(conflicts("tipped.ada", "boarded.train"), Ok(false));
    }

    // H: DeonticSpec.hs "a content does not conflict with itself"
    #[test]
    fn a_content_does_not_conflict_with_itself() {
        assert_eq!(conflicts("go!true", "go!true"), Ok(false));
    }

    // H: DeonticSpec.hs "two exclusive locations for one agent conflict"
    #[test]
    fn two_exclusive_locations_for_one_agent_conflict() {
        assert_eq!(
            conflicts(
                "practice.world.world.at.bex!bar",
                "practice.world.world.at.bex!entrance"
            ),
            Ok(true)
        );
    }

    // H: DeonticSpec.hs "incompatiblePairs finds exactly the incompatible pair"
    #[test]
    fn incompatible_pairs_finds_exactly_the_incompatible_pair() {
        assert_eq!(
            incompatible_pairs(&["at!bar", "at!entrance", "tipped.ada"]),
            Ok(vec![("at!bar".to_owned(), "at!entrance".to_owned())])
        );
    }

    /// The frozen `conflicts` is PARTIAL — `Prax.Db.insertAll` raises on a
    /// malformed sentence — and this port surfaces that as `Err` rather than
    /// `unwrap`ping it ([S-I5]). Pinned so the fallibility is a tested property
    /// of the API and not a comment; `incompatible_pairs` propagates it.
    #[test]
    fn conflicts_rejects_a_malformed_content_rather_than_panicking() {
        assert!(
            conflicts("go.", "go!false").is_err(),
            "a trailing operator is rejected, as Prax.Db.tokens rejects it"
        );
        assert!(incompatible_pairs(&["ok.fine", "go."]).is_err());
    }

    // ---- introspection -----------------------------------------------------

    // H: DeonticSpec.hs "introspection"
    // H: DeonticSpec.hs "obligationsOf lists an agent's current duties"
    #[test]
    fn obligations_of_lists_an_agents_current_duties() {
        // Built through `oblige` itself, so the pin covers the path convention
        // and the reader together rather than a hand-written prefix.
        let mut i = Interner::new();
        let mut db = Db::empty();
        for o in [
            oblige("bex", "settle.up"),
            oblige("bex", "greet.ada"),
            oblige("cai", "leave.now"),
        ] {
            let Outcome::Insert(s) = &o else {
                panic!("oblige builds an Insert, got {o:?}")
            };
            db = db.insert_str(&mut i, s).expect("an obligation sentence");
        }
        let bex = obligations_of("bex", &db, &i);
        assert!(bex.contains(&"settle.up".to_owned()), "got {bex:?}");
        assert!(bex.contains(&"greet.ada".to_owned()), "got {bex:?}");
        assert!(
            !bex.iter().any(|s| s.contains("leave")),
            "other agents excluded, got {bex:?}"
        );
    }

    // ---- behavioural coupling ----------------------------------------------

    /// The frozen `conductWorld`: one agent `gil` whose available actions bring
    /// about different contents, so the planner's choice is driven purely by its
    /// wants.
    fn conduct_world(acts: Vec<(&str, Vec<Condition>, Vec<Outcome>)>, wants: Vec<Want>) -> State {
        let mut p = Practice::new("conduct").name("conduct").roles(["X"]);
        for (label, cs, os) in acts {
            p = p.action(Action::new(label).when(cs).then(os));
        }
        let mut agent = Character::new("gil");
        agent.wants = wants;
        let mut st = State::new();
        st.define_practices([p]).expect("the conduct practice");
        st.set_characters(vec![agent]).expect("the conduct cast");
        st.perform_outcome(&insert("practice.conduct.gil"))
            .expect("the conduct instance");
        st
    }

    fn gil(st: &State) -> Character {
        st.characters()
            .iter()
            .find(|c| c.name == "gil")
            .cloned()
            .expect("gil is in the cast")
    }

    // H: DeonticSpec.hs "behavioural coupling (planner unchanged)"
    // H: DeonticSpec.hs "an agent pursues the action that fulfils its obligation"
    #[test]
    fn an_agent_pursues_the_action_that_fulfils_its_obligation() {
        let mut st = conduct_world(
            vec![
                ("[Actor]: do the duty", vec![], vec![insert("did.duty")]),
                ("[Actor]: slack off", vec![], vec![insert("did.nothing")]),
            ],
            vec![want_fulfilled("gil", "did.duty", 20)],
        );
        st.perform_outcome(&oblige("gil", "did.duty"))
            .expect("the duty");
        let g = gil(&st);
        assert_eq!(
            st.pick_action(1, &g).map(|a| a.label),
            Some("gil: do the duty".to_owned())
        );
    }

    // H: DeonticSpec.hs "an agent avoids an action that would breach a duty"
    #[test]
    fn an_agent_avoids_an_action_that_would_breach_a_duty() {
        let mut st = conduct_world(
            vec![
                ("[Actor]: behave", vec![], vec![]),
                (
                    "[Actor]: transgress",
                    vec![],
                    vec![breach("gil", "tipping")],
                ),
            ],
            vec![avoid_breach("gil", "tipping", 50)],
        );
        let g = gil(&st);
        assert_eq!(
            st.pick_action(1, &g).map(|a| a.label),
            Some("gil: behave".to_owned())
        );
    }

    // H: DeonticSpec.hs "conflicting duties: planner fulfils the higher-valued; the other is foreclosed"
    #[test]
    fn conflicting_duties_the_planner_fulfils_the_higher_valued() {
        let mut st = conduct_world(
            vec![
                (
                    "[Actor]: help alice",
                    vec![Condition::Not("committed".into())],
                    vec![insert("helped.alice"), insert("committed!alice")],
                ),
                (
                    "[Actor]: help bob",
                    vec![Condition::Not("committed".into())],
                    vec![insert("helped.bob"), insert("committed!bob")],
                ),
            ],
            vec![
                want_fulfilled("gil", "helped.alice", 30),
                want_fulfilled("gil", "helped.bob", 10),
            ],
        );
        for o in [
            oblige("gil", "helped.alice"),
            oblige("gil", "helped.bob"),
        ] {
            st.perform_outcome(&o).expect("a duty");
        }
        assert!(
            sat(&mut st, &[is_obliged("gil", "helped.alice")])
                && sat(&mut st, &[is_obliged("gil", "helped.bob")]),
            "both duties are genuinely held at once (distinct contents coexist)"
        );
        let g = gil(&st);
        let chosen = st.pick_action(1, &g);
        assert_eq!(
            chosen.as_ref().map(|a| a.label.clone()),
            Some("gil: help alice".to_owned()),
            "resolution is emergent: the higher-utility duty wins"
        );
        if let Some(ga) = &chosen {
            st.perform_action(ga);
        }
        assert!(
            !st.possible_actions("gil")
                .iter()
                .any(|a| a.label.contains("help bob")),
            "helping bob is now foreclosed"
        );
        assert!(
            sat(&mut st, &[is_obliged("gil", "helped.bob")])
                && !sat(&mut st, &fulfilled("gil", "helped.bob")),
            "bob's duty is still owed, and unfulfillable"
        );
    }

    // The two owed:S4 DeriveSpec discharges that need obliged_close in scope
    // (prax-vocab depends on prax-core, so both axiom_footprint and obliged_close
    // are available here).

    // H: DeriveSpec.hs "axiomFootprint collects bodies (any polarity) and heads; obligedClose adds the lifted forms"
    #[test]
    fn axiom_footprint_collects_bodies_heads_and_obliged_close_lifts() {
        let mut i = Interner::new();
        let ax = Axiom::new(
            vec![m("parent.X.Y"), Condition::Absent(vec![m("dead.X")])],
            ["elder.X"],
        );
        let fp = axiom_footprint(&compile_rules(&mut i, &[ax]));
        assert!(has_path(&mut i, &fp, "parent.X.Y"), "body atom");
        assert!(has_path(&mut i, &fp, "dead.X"), "negated body atom");
        assert!(has_path(&mut i, &fp, "elder.X"), "head");

        // cookAxioms is deontics-free: a bare all-Match rule contributes no
        // lifted twin. Declaring the closure adds the lifted rule, whose body and
        // head then appear in the footprint.
        let base_ax = [Axiom::new(vec![m("a.X")], ["b.X"])];
        let bare = axiom_footprint(&compile_rules(&mut i, &base_ax));
        let closed = axiom_footprint(&compile_rules(&mut i, &obliged_close(&base_ax)));
        assert!(has_path(&mut i, &bare, "a.X"), "bare base body kept");
        assert!(has_path(&mut i, &bare, "b.X"), "bare base head kept");
        assert!(!has_path(&mut i, &bare, "obliged.Obligor.a.X"), "bare has no lifted body");
        assert!(!has_path(&mut i, &bare, "obliged.Obligor.b.X"), "bare has no lifted head");
        assert!(has_path(&mut i, &closed, "obliged.Obligor.a.X"), "obligedClose lifts body");
        assert!(has_path(&mut i, &closed, "obliged.Obligor.b.X"), "obligedClose lifts head");
    }

    // H: DeriveSpec.hs "obligedClose: a domain rule (written once) also closes under obligation"
    #[test]
    fn obliged_close_closes_an_obliged_context() {
        let mut i = Interner::new();
        let axs = [Axiom::new(vec![m("at.W.bar")], ["in.W.building"])];
        // bex ought to be at the bar.
        let base = build(&mut i, &["obliged.bex.at.bex.bar"]);

        // Under the declared closure, an obliged context derives the sub-obligation.
        let closed_rules = compile_rules(&mut i, &obliged_close(&axs));
        let closed = close(&mut i, &closed_rules, &base).unwrap();
        assert!(
            closed
                .to_sentences(&i)
                .contains(&"obliged.bex.in.bex.building".to_owned()),
            "sub-obligation derived under declared closure (□a ⊢ □b)"
        );

        // Bare closure does NOT lift: no auto-obligation.
        let bare_rules = compile_rules(&mut i, &axs);
        let bare = close(&mut i, &bare_rules, &base).unwrap();
        assert!(
            !bare
                .to_sentences(&i)
                .contains(&"obliged.bex.in.bex.building".to_owned()),
            "bare closure does NOT lift"
        );
    }
}
