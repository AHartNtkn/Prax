//! The Core Model: relationships (Versu paper §X).
//!
//! Versu calls the agent's emotional and relationship state "the channel through
//! which the different practices communicate". It is ordinary DB state — this
//! module adds no engine machinery, only a reusable standard library of
//! conventions:
//!
//! * relationship evaluations are numeric, multiple, asymmetric, and carry a
//!   reason (`A.relationship.B.Role.score!N`, `A.relationship.B.Role.reason!Why`);
//! * a public "bond" is symmetric (`bond.A.B!State` written both ways).
//!
//! The read-modify-write pieces (seed-then-accumulate a score) are provided once
//! as [`Function`]s ([`core_fns`]), reusing the `Call`/`Calc` machinery.
//! Register [`core_fns`] with `State::define_functions` to make the smart
//! constructors below usable.
//!
//! Frozen reference: `src/Prax/Core.hs` (the module is `Prax.Core`; the Rust
//! module is named for the frozen module's own title, "the Core Model", because
//! `core` is the Rust prelude crate). Infallible throughout — the frozen module
//! raises nowhere and calls no tokenizing helper, checked one call deeper per
//! slice 1's [C2].
//!
//! **[`core_fns`]' case ORDER is observable** (S7 design §3): matching is
//! FIRST-match and the registry ends in an unguarded fallback case, so swapping
//! the two `prax_adjustScore` cases would make every adjustment re-seed instead
//! of accumulate. [`case_order_is_load_bearing`](tests::case_order_is_load_bearing)
//! pins it directly.

use prax_core::query::{CalcOp, CmpOp, Condition, calc, cmp, matches};
use prax_core::types::{Function, Outcome, call, insert};

/// Example evaluation dimensions. A relationship is judged on any number of
/// these, independently in each direction.
pub const WARMTH: &str = "warmth";
/// See [`WARMTH`].
pub const RESPECT: &str = "respect";

/// Adjust `a`'s evaluation of `b` on `role` by `delta` (seeding it if this is the
/// first interaction), recording `reason`. Negative deltas cool the relation.
pub fn adjust_score(a: &str, b: &str, role: &str, delta: i32, reason: &str) -> Outcome {
    call(
        "prax_adjustScore",
        vec![
            a.to_owned(),
            b.to_owned(),
            role.to_owned(),
            delta.to_string(),
            reason.to_owned(),
        ],
    )
}

/// Set the symmetric public bond between `a` and `b` (e.g. `"friends"`).
pub fn set_bond(a: &str, b: &str, st: &str) -> Outcome {
    call(
        "prax_setBond",
        vec![a.to_owned(), b.to_owned(), st.to_owned()],
    )
}

/// Require `a`'s evaluation of `b` on `role` to be at least `n`. Binds a
/// role-specific score variable (safe to combine across distinct roles).
pub fn score_at_least(a: &str, b: &str, role: &str, n: i32) -> Vec<Condition> {
    let v = format!("Score{}", capitalize(role));
    vec![
        matches(format!("{a}.relationship.{b}.{role}.score!{v}")),
        cmp(CmpOp::Gte, v, n.to_string()),
    ]
}

/// `Data.Char.toUpper` on the first character (`Core.hs:69`).
///
/// `toUpper` is Unicode's SIMPLE uppercase mapping; Rust's `char::to_uppercase`
/// is the FULL mapping, which differs only by expanding to more than one char
/// (`ß` → `SS`, the ligatures). Where the full mapping is a single char it IS
/// the simple mapping, and where it expands the simple mapping is the character
/// unchanged — which is exactly what the fallback below keeps. So this is
/// `toUpper`, not an approximation of it.
fn capitalize(s: &str) -> String {
    let mut cs = s.chars();
    match cs.next() {
        None => String::new(),
        Some(c) => {
            let mut up = c.to_uppercase();
            let first = match (up.next(), up.next()) {
                (Some(u), None) => u,
                _ => c,
            };
            let mut out = String::from(first);
            out.push_str(cs.as_str());
            out
        }
    }
}

/// The reusable core-model functions. Register them with
/// `State::define_functions`; they are found by name whenever an action calls
/// them.
pub fn core_fns() -> Vec<Function> {
    vec![adjust_score_fn(), set_bond_fn()]
}

/// Add `Delta` to an existing score, or seed it with `Delta` on first
/// interaction. The guarded case is FIRST; the fallback case is last and
/// unguarded — first-match semantics make that order the whole behaviour.
fn adjust_score_fn() -> Function {
    Function::new(
        "prax_adjustScore",
        ["A", "B", "Role", "Delta", "Reason"],
    )
    .case(
        [
            matches("A.relationship.B.Role.score!N"),
            calc("M", CalcOp::Add, "N", "Delta"),
        ],
        [
            insert("A.relationship.B.Role.score!M"),
            insert("A.relationship.B.Role.reason!Reason"),
        ],
    )
    .case(
        [],
        [
            insert("A.relationship.B.Role.score!Delta"),
            insert("A.relationship.B.Role.reason!Reason"),
        ],
    )
}

/// Write the symmetric bond in both directions at once.
fn set_bond_fn() -> Function {
    Function::new("prax_setBond", ["A", "B", "State"]).case(
        [],
        [insert("bond.A.B!State"), insert("bond.B.A!State")],
    )
}

#[cfg(test)]
mod tests {
    // H: CoreSpec.hs "Prax.Core"
    //
    // The frozen `Prax.CoreSpec`, re-expressed against the Rust engine, plus the
    // three Call-routing pins the frozen spec never wrote and slice 2 is the
    // first slice to need: first-CASE, first-BINDING, and the BASE-db quirk.
    use super::*;
    use prax_core::engine::State;
    use prax_core::types::insert;

    /// A state with the core library registered.
    fn base() -> State {
        let mut st = State::new();
        st.define_functions(core_fns()).expect("the core library");
        st
    }

    fn perform(st: &mut State, o: &Outcome) {
        st.perform_outcome(o).expect("a core-model outcome");
    }

    // H: CoreSpec.hs "relationships (numeric, asymmetric, with reason)"
    // (the frozen group label; its three cases follow)

    // H: CoreSpec.hs "adjustScore seeds on first use, then accumulates"
    #[test]
    fn adjust_score_seeds_on_first_use_then_accumulates() {
        let mut st = base();
        perform(&mut st, &adjust_score("ada", "bex", WARMTH, 10, "greeting"));
        assert!(
            st.labeled_facts()
                .contains(&"ada.relationship.bex.warmth.score!10".to_owned()),
            "seeded to 10, got {:?}",
            st.labeled_facts()
        );
        perform(&mut st, &adjust_score("ada", "bex", WARMTH, 5, "served"));
        let fs = st.labeled_facts();
        assert!(
            fs.contains(&"ada.relationship.bex.warmth.score!15".to_owned()),
            "accumulated to 15, got {fs:?}"
        );
        assert!(
            !fs.contains(&"ada.relationship.bex.warmth.score!10".to_owned()),
            "single score value, got {fs:?}"
        );
        assert!(
            fs.contains(&"ada.relationship.bex.warmth.reason!served".to_owned()),
            "reason updated, got {fs:?}"
        );
    }

    // H: CoreSpec.hs "a negative delta cools the relationship"
    #[test]
    fn a_negative_delta_cools_the_relationship() {
        let mut st = base();
        perform(&mut st, &adjust_score("ada", "bex", WARMTH, 10, "greeting"));
        perform(&mut st, &adjust_score("ada", "bex", WARMTH, -30, "insulted"));
        assert!(
            st.labeled_facts()
                .contains(&"ada.relationship.bex.warmth.score!-20".to_owned()),
            "score went negative, got {:?}",
            st.labeled_facts()
        );
    }

    // H: CoreSpec.hs "evaluations are asymmetric"
    #[test]
    fn evaluations_are_asymmetric() {
        let mut st = base();
        perform(&mut st, &adjust_score("ada", "bex", WARMTH, 10, "greeting"));
        let fs = st.labeled_facts();
        assert!(
            fs.contains(&"ada.relationship.bex.warmth.score!10".to_owned()),
            "ada judges bex, got {fs:?}"
        );
        assert!(
            !fs.iter().any(|s| s.contains("bex.relationship.ada")),
            "bex does not (yet) judge ada, got {fs:?}"
        );
    }

    // H: CoreSpec.hs "public bond (symmetric)"
    // (the frozen group label; its one case follows)

    // H: CoreSpec.hs "setBond writes both directions"
    #[test]
    fn set_bond_writes_both_directions() {
        let mut st = base();
        perform(&mut st, &set_bond("ada", "bex", "friends"));
        let fs = st.labeled_facts();
        assert!(fs.contains(&"bond.ada.bex!friends".to_owned()), "a->b");
        assert!(fs.contains(&"bond.bex.ada!friends".to_owned()), "b->a");
    }

    /// `scoreAtLeast` has no frozen pin of its own; it is exported API and the
    /// variable name it mints (`Score` ++ capitalize role) is what makes two
    /// roles combinable in one action guard. A drift there silently joins two
    /// independent scores.
    #[test]
    fn score_at_least_binds_a_role_specific_variable() {
        assert_eq!(
            score_at_least("ada", "bex", WARMTH, 3),
            vec![
                matches("ada.relationship.bex.warmth.score!ScoreWarmth"),
                cmp(CmpOp::Gte, "ScoreWarmth", "3"),
            ]
        );
        assert_eq!(
            score_at_least("ada", "bex", RESPECT, -1)[1],
            cmp(CmpOp::Gte, "ScoreRespect", "-1"),
            "distinct roles mint distinct variables, so the two guards do not join"
        );
        assert_eq!(capitalize(""), "", "the empty role capitalizes to nothing");
    }

    /// FIRST-MATCH over cases is the whole of `prax_adjustScore`'s behaviour: the
    /// accumulating case is guarded and first, the seeding case is unguarded and
    /// last. If the fallback were consulted first, every adjustment would
    /// overwrite instead of accumulate — so this pins the order rather than the
    /// outcome list.
    #[test]
    fn case_order_is_load_bearing() {
        let f = adjust_score_fn();
        assert_eq!(f.cases.len(), 2);
        assert!(
            !f.cases[0].conditions.is_empty(),
            "the guarded (accumulating) case comes first"
        );
        assert!(
            f.cases[1].conditions.is_empty(),
            "the unguarded fallback comes last — anything after it is dead"
        );

        // and the engine honours it: with the order reversed, the second
        // adjustment re-seeds instead of accumulating.
        let reversed = Function::new("prax_adjustScore", ["A", "B", "Role", "Delta", "Reason"])
            .case(
                [],
                [
                    insert("A.relationship.B.Role.score!Delta"),
                    insert("A.relationship.B.Role.reason!Reason"),
                ],
            )
            .case(
                [
                    matches("A.relationship.B.Role.score!N"),
                    calc("M", CalcOp::Add, "N", "Delta"),
                ],
                [
                    insert("A.relationship.B.Role.score!M"),
                    insert("A.relationship.B.Role.reason!Reason"),
                ],
            );
        let mut st = State::new();
        st.define_functions(vec![reversed]).expect("the mutant");
        perform(&mut st, &adjust_score("ada", "bex", WARMTH, 10, "greeting"));
        perform(&mut st, &adjust_score("ada", "bex", WARMTH, 5, "served"));
        assert!(
            st.labeled_facts()
                .contains(&"ada.relationship.bex.warmth.score!5".to_owned()),
            "fallback-first re-seeds to 5 — which is what the shipped order avoids"
        );
    }

    /// A function case's guard is queried against the BASE db, never the closed
    /// VIEW (the frozen quirk, S7 design §1.3's STATE row). A derived score is
    /// invisible to `prax_adjustScore`, so the fallback case fires and the value
    /// SEEDS rather than accumulating.
    #[test]
    fn a_function_case_queries_the_base_db_not_the_view() {
        let mut st = base();
        // `ada.relationship.bex.warmth.score!7` exists ONLY as a derived fact.
        st.set_axioms(vec![prax_core::types::Axiom::new(
            vec![matches("seed.ada.bex")],
            ["ada.relationship.bex.warmth.score!7"],
        )])
        .expect("the deriving axiom");
        perform(&mut st, &insert("seed.ada.bex"));
        assert!(
            st.view_has("ada.relationship.bex.warmth.score!7"),
            "fixture: the score is derived into the view"
        );
        assert!(
            !st.db_has("ada.relationship.bex.warmth.score!7"),
            "fixture: and is absent from the base"
        );

        perform(&mut st, &adjust_score("ada", "bex", WARMTH, 5, "served"));
        assert!(
            st.db_has("ada.relationship.bex.warmth.score!5"),
            "the guarded case did NOT see the derived 7, so the fallback seeded 5 \
             — a view-reading Call would have written 12"
        );
    }

    /// FIRST-BINDING: a matching case runs its outcomes for ONE binding row, not
    /// for every row (that is `ForEach`'s job). Two scores under one role would
    /// otherwise both be adjusted.
    #[test]
    fn a_matching_case_runs_for_the_first_binding_only() {
        let mut st = State::new();
        // A function whose guard has TWO solutions; the shipped `prax_adjustScore`
        // cannot exhibit this (its score slot is single-valued by `!`), so the
        // semantics is pinned on a fixture that can.
        st.define_functions(vec![
            Function::new("mark_one", ["Who"]).case(
                [matches("candidate.Who.C")],
                [insert("marked.Who.C")],
            ),
        ])
        .expect("the fixture function");
        for f in ["candidate.ada.x", "candidate.ada.y"] {
            perform(&mut st, &insert(f));
        }
        perform(&mut st, &call("mark_one", vec!["ada".to_owned()]));
        let marked: Vec<String> = st
            .labeled_facts()
            .into_iter()
            .filter(|s| s.starts_with("marked."))
            .collect();
        assert_eq!(
            marked,
            vec!["marked.ada.x".to_owned()],
            "exactly one binding fired, and it is the first in the query's \
             name-ordered branch — not both candidates"
        );
    }
}
