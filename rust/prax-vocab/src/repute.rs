//! Reputation: standing derived from evidence.
//!
//! Nobody STORES reputation. `regards.<observer>.<subject>.<label>` is DERIVED
//! ([`prax_core::derive`]) from the observer's event-beliefs
//! ([`crate::witness`] / [`crate::rumor`]), so it lives only in the defeasible
//! view: it inherits information asymmetry (only those the news reached hold the
//! regard) and dissolves the moment its support does.
//!
//! Standing is defeated by ATONEMENT, NOT AMNESIA: [`standing_unless`] guards the
//! derivation with a BASE-FACT defeater (e.g. `atoned.<culprit>`) — one insertion
//! dissolves every derived regard at once while every belief (the memory of the
//! deed) persists. The defeater must name only base facts, never derived heads,
//! keeping the closure stratified.
//!
//! [`notoriety`] turns corroboration into a GLOBAL derived fact:
//! `notorious.<subject>.<label>` holds while at least `k` distinct observers hold
//! the regard — counting derived facts across fixpoint rounds. The threshold is
//! an authored world parameter with stated meaning ("the whole village knows").
//!
//! Conventions (as in [`crate::rumor`]): the deed pattern's FIRST variable is the
//! subject (who the standing attaches to), and the variable `Regarder` is an
//! INTERFACE variable in the v40 two-tier sense: worlds read it first-class (the
//! village's fear wants), and it is forbidden only inside spliced deed patterns,
//! where it would capture the axiom's own join.
//!
//! Frozen reference: `src/Prax/Repute.hs`. These are AXIOM builders (S7 design
//! §3.4): they change what the planner can READ, and `standing_unless` derives
//! its defeater by string surgery over the deed pattern — a one-character drift
//! renders plausibly and misbehaves silently, so every path here goes through
//! [`subject_of`] and [`checked_defeater`], never an inlined `format!`.

use prax_core::error::WorldError;
use prax_core::interner::is_variable_name;
use prax_core::path::segment_names_checked;
use prax_core::query::{CmpOp, Condition, cmp, count, matches, not_, subquery};
use prax_core::types::{Axiom, authored_pat_clash};

/// The deed pattern's first variable: who the standing is about (the frozen
/// `subjectOf`).
///
/// # Errors
/// [`WorldError::TrailingOperator`] on a malformed pattern (the frozen
/// `pathNames` raises), or [`WorldError::PatternVariables`] when the pattern
/// names nobody — a standing is about someone.
fn subject_of(pat: &str) -> Result<String, WorldError> {
    // UNCHECKED-SPLIT is not taken here: the frozen `subjectOf` splits with
    // `pathNames`, which raises on a trailing operator (S7 design §12).
    let names = segment_names_checked(pat)?;
    names
        .into_iter()
        .find(|n| is_variable_name(n))
        .ok_or_else(|| WorldError::PatternVariables {
            context: "Repute.standing".to_owned(),
            pattern: pat.to_owned(),
            needs: "name someone (a standing is about someone)".to_owned(),
        })
}

/// A defeater may use only the deed pattern's variables: negation-as-failure
/// would silently turn any OTHER variable into a global existential guard (one
/// unrelated fact dissolving everyone's standing).
///
/// # Errors
/// [`WorldError::TrailingOperator`] on a malformed pattern or defeater;
/// [`WorldError::PatternVariables`] when the defeater names a variable the deed
/// pattern does not.
fn checked_defeater(pat: &str, defeater: &str) -> Result<String, WorldError> {
    let pat_vars: Vec<String> = segment_names_checked(pat)?
        .into_iter()
        .filter(|n| is_variable_name(n))
        .collect();
    let def_vars: Vec<String> = segment_names_checked(defeater)?
        .into_iter()
        .filter(|n| is_variable_name(n))
        .collect();
    if def_vars.iter().all(|v| pat_vars.contains(v)) {
        Ok(defeater.to_owned())
    } else {
        Err(WorldError::PatternVariables {
            context: "Repute.standing_unless".to_owned(),
            pattern: defeater.to_owned(),
            needs: format!(
                "use only the deed pattern {pat:?}'s variables (it would defeat globally, not per-subject)"
            ),
        })
    }
}

/// The deed pattern's own hygiene guard, shared by both entry points: `Regarder`
/// (the axiom's own observer join variable) and the `Prax` namespace are
/// reserved.
///
/// # Errors
/// [`WorldError::TrailingOperator`] on a malformed pattern;
/// [`WorldError::ReservedVarClash`] on a reserved variable.
fn check_deed_pattern(pat: &str) -> Result<(), WorldError> {
    let names = segment_names_checked(pat)?;
    match authored_pat_clash(&["Regarder".to_owned()], &names).first() {
        Some(v) => Err(WorldError::ReservedVarClash {
            context: "Repute.standing".to_owned(),
            var: v.clone(),
            extra: format!(
                " -- deed pattern {pat:?} reserves it; the Prax namespace and Regarder \
                 (the axiom's own observer join variable) are both reserved"
            ),
        }),
        None => Ok(()),
    }
}

fn standing_with(pat: &str, extra: Vec<Condition>, label: &str) -> Result<Axiom, WorldError> {
    check_deed_pattern(pat)?;
    let subject = subject_of(pat)?;
    let mut when = vec![matches(format!("Regarder.believes.{pat}"))];
    when.extend(extra);
    Ok(Axiom::new(
        when,
        [format!("regards.Regarder.{subject}.{label}")],
    ))
}

/// Every observer with evidence of the deed regards its subject under `label`.
///
/// # Errors
/// [`check_deed_pattern`]'s and [`subject_of`]'s rejections.
pub fn standing(pat: &str, label: &str) -> Result<Axiom, WorldError> {
    standing_with(pat, Vec::new(), label)
}

/// [`standing`], defeated by a base-fact pattern (which may use only the deed
/// pattern's variables — checked, loud error otherwise): derives only while the
/// defeater is absent.
///
/// # Errors
/// The deed pattern's own rejections FIRST (the frozen `standingWith` guard is
/// forced before the lazily-built defeater list reaches `checkedDefeater`), then
/// [`checked_defeater`]'s.
pub fn standing_unless(pat: &str, defeater: &str, label: &str) -> Result<Axiom, WorldError> {
    // The frozen guard ORDER, reproduced: `standingWith`'s pattern-clash guard
    // is a `|` guard on the scrutinee, so it fires before anything forces the
    // `extra` list `standingUnless` builds. Checking the pattern here (and again
    // inside `standing_with`) is a recomputation, not a second guard — it is the
    // only way to keep the frozen precedence with strict arguments.
    check_deed_pattern(pat)?;
    standing_with(pat, vec![not_(checked_defeater(pat, defeater)?)], label)
}

/// Condition: `observer` regards `subject` under `label` (a derived fact — usable
/// in preconditions and wants, which read the closed view).
pub fn regarded_as(observer: &str, subject: &str, label: &str) -> Condition {
    matches(format!("regards.{observer}.{subject}.{label}"))
}

/// `notorious.<subject>.<label>` while at least `k` distinct observers hold the
/// regard.
pub fn notoriety(label: &str, k: i32) -> Axiom {
    Axiom::new(
        vec![
            matches(format!("regards.W0.T.{label}")),
            subquery(
                "Rs",
                vec!["W".to_owned()],
                vec![matches(format!("regards.W.T.{label}"))],
            ),
            count("N", "Rs"),
            cmp(CmpOp::Gte, "N", k.to_string()),
        ],
        [format!("notorious.T.{label}")],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_core::engine::State;
    use prax_core::types::{Action, Character, Practice, delete, insert};

    // H: ReputeSpec.hs "Prax.Repute"
    //
    // The frozen `Prax.ReputeSpec`, re-expressed against the Rust engine.
    //
    // The tale: kai is believed (variously) to have kicked the dog. Standing is
    // derived from the evidence, defeated by forgiveness, and notorious at two.

    fn world() -> State {
        let mut st = State::new();
        // an affordance gated on the DERIVED standing: preconditions read the view
        st.define_practices([Practice::new("yard").roles(["R"]).action(
            Action::new("[Actor]: scowl at [B]")
                .when([regarded_as("Actor", "B", "brute"), not_("scowled.Actor.B")])
                .then([insert("scowled.Actor.B")]),
        )])
        .unwrap();
        st.set_characters(
            ["ana", "ben", "kai"]
                .into_iter()
                .map(Character::new)
                .collect(),
        )
        .unwrap();
        for o in [
            insert("practice.yard.here"),
            insert("ana.believes.kicked.kai.dog.seen"),
            insert("ben.believes.kicked.kai.dog.heard.ana"),
        ] {
            st.perform_outcome(&o).expect("repute setup");
        }
        st.set_axioms(vec![
            standing_unless("kicked.Brute.dog", "forgiven.Brute", "brute").unwrap(),
            notoriety("brute", 2),
        ])
        .unwrap();
        st
    }

    // H: ReputeSpec.hs "evidence derives per-observer standing (seen and heard alike)"
    #[test]
    fn evidence_derives_per_observer_standing() {
        let mut st = world();
        assert!(
            st.view_has("regards.ana.kai.brute"),
            "ana (eyewitness) regards kai a brute"
        );
        assert!(
            st.view_has("regards.ben.kai.brute"),
            "ben (hearsay) regards kai a brute"
        );
        assert!(
            !st.view_has("regards.kai.kai.brute"),
            "kai holds no self-regard"
        );
    }

    // H: ReputeSpec.hs "notoriety holds at the threshold, not below"
    #[test]
    fn notoriety_holds_at_the_threshold_not_below() {
        let mut st = world();
        assert!(
            st.view_has("notorious.kai.brute"),
            "two regarders: notorious"
        );
        st.perform_outcome(&delete("ben.believes.kicked.kai.dog"))
            .unwrap();
        assert!(
            !st.view_has("notorious.kai.brute"),
            "one regarder: not notorious"
        );
        assert!(
            !st.view_has("regards.ben.kai.brute"),
            "and ben's regard dissolved with his evidence"
        );
    }

    // H: ReputeSpec.hs "the defeater dissolves standing while memory persists"
    #[test]
    fn the_defeater_dissolves_standing_while_memory_persists() {
        let mut st = world();
        st.perform_outcome(&insert("forgiven.kai")).unwrap();
        assert!(
            !st.view_has("regards.ana.kai.brute"),
            "no regard survives forgiveness"
        );
        assert!(!st.view_has("notorious.kai.brute"), "no notoriety survives");
        assert!(
            st.db_has("ana.believes.kicked.kai.dog.seen"),
            "ana still remembers what she saw"
        );
    }

    // H: ReputeSpec.hs "a derived-standing-gated affordance appears and disappears"
    #[test]
    fn a_derived_standing_gated_affordance_appears_and_disappears() {
        let mut st = world();
        assert!(
            st.possible_actions("ana")
                .iter()
                .any(|ga| ga.label.contains("scowl at kai")),
            "ana may scowl at kai"
        );
        st.perform_outcome(&insert("forgiven.kai")).unwrap();
        assert!(
            !st.possible_actions("ana")
                .iter()
                .any(|ga| ga.label.contains("scowl at kai")),
            "forgiven: the scowl is gone"
        );
    }

    // H: ReputeSpec.hs "regardedAs is the standing condition"
    #[test]
    fn regarded_as_is_the_standing_condition() {
        assert_eq!(
            regarded_as("W", "kai", "brute"),
            Condition::Match("regards.W.kai.brute".to_owned())
        );
    }

    // H: ReputeSpec.hs "the deed pattern's FIRST variable is the subject"
    #[test]
    fn the_deed_patterns_first_variable_is_the_subject() {
        assert_eq!(
            standing("sold.Seller.Buyer.secret", "snitch").unwrap().then,
            vec!["regards.Regarder.Seller.snitch".to_owned()]
        );
    }

    // H: ReputeSpec.hs "a deed pattern with no variable errors loudly"
    #[test]
    fn a_deed_pattern_with_no_variable_errors_loudly() {
        assert!(
            matches!(
                standing("somethinghappened", "x"),
                Err(WorldError::PatternVariables { .. })
            ),
            "standing on a subject-less pattern is an error"
        );
    }

    // H: ReputeSpec.hs "a defeater variable outside the deed pattern errors loudly"
    #[test]
    fn a_defeater_variable_outside_the_deed_pattern_errors_loudly() {
        assert!(
            matches!(
                standing_unless("stole.Culprit.loaf", "atoned.Someone", "thief"),
                Err(WorldError::PatternVariables { .. })
            ),
            "mis-scoped defeater is an error, not a silent global amnesty"
        );
    }

    // H: ReputeSpec.hs "standing: a deed pattern authoring Regarder (the axiom's own observer variable) errors loudly"
    #[test]
    fn a_deed_pattern_authoring_regarder_errors_loudly() {
        assert!(
            matches!(
                standing("kicked.Regarder.dog", "brute"),
                Err(WorldError::ReservedVarClash { .. })
            ),
            "Regarder is reserved -- the axiom's own observer join variable"
        );
    }

    // H: ReputeSpec.hs "standing: a deed pattern authoring the Prax namespace errors loudly"
    #[test]
    fn a_deed_pattern_authoring_the_prax_namespace_errors_loudly() {
        assert!(
            matches!(
                standing("kicked.PraxX.dog", "brute"),
                Err(WorldError::ReservedVarClash { .. })
            ),
            "the Prax namespace is reserved"
        );
    }

    // H: ReputeSpec.hs "standingUnless: Regarder is reserved through the same guard (both entry points funnel through it)"
    #[test]
    fn standing_unless_reserves_regarder_through_the_same_guard() {
        assert!(
            matches!(
                standing_unless("stole.Regarder.loaf", "atoned.Regarder", "thief"),
                Err(WorldError::ReservedVarClash { .. })
            ),
            "Regarder is reserved for standingUnless too"
        );
    }
}
