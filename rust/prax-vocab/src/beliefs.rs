//! Beliefs: per-issue divergence between a character's model and the truth
//! (Versu paper §X).
//!
//! Versu keeps a single shared world state and stores INDIVIDUAL beliefs only
//! where an agent's view may diverge from it — "false beliefs, or factual
//! disagreements". A belief is just a fact under the believing agent:
//!
//! ```text
//! X.believes.<issue>!Value
//! ```
//!
//! Single-slot per issue (the `!`): a new value overrides the old. Because the
//! belief lives UNDER the agent, two agents can hold different values for one
//! issue, and either can differ from the shared-world fact.
//!
//! Frozen reference: `src/Prax/Beliefs.hs`. Nothing here is fallible — the
//! frozen module raises nowhere, and neither does anything it calls (its two
//! path builders are bare `++`, never `pathNames`/`tokens`), so no builder here
//! returns `Result`. That was established by grepping the module AND its
//! callees, which is slice 1's [C2] lesson.

use prax_core::query::{Condition, matches};
use prax_core::types::{Outcome, delete, insert};

/// The sentence `who.believes.<issue>!value`. `issue` may be a dotted sub-path
/// (e.g. `"resentedBy.ada"`). Any part may be an action variable, grounded when
/// used in an outcome/condition.
///
/// The module's OWN path helper (`Beliefs.hs:34`); every builder below goes
/// through it or [`belief_about`], never through a reconstructed string.
pub fn belief_sentence(who: &str, issue: &str, value: &str) -> String {
    format!("{who}.believes.{issue}!{value}")
}

/// The path `who.believes.<issue>`, for binding the believed value:
/// `matches(belief_about(who, issue) + "!V")`.
pub fn belief_about(who: &str, issue: &str) -> String {
    format!("{who}.believes.{issue}")
}

/// `who` comes to believe `issue` has `value` (overriding any prior value).
pub fn believe(who: &str, issue: &str, value: &str) -> Outcome {
    insert(belief_sentence(who, issue, value))
}

/// Condition: `who` believes `issue` is `value`.
pub fn believes_that(who: &str, issue: &str, value: &str) -> Condition {
    matches(belief_sentence(who, issue, value))
}

/// `who` drops any belief about `issue`.
pub fn forget(who: &str, issue: &str) -> Outcome {
    delete(belief_about(who, issue))
}

#[cfg(test)]
mod tests {
    // H: BeliefsSpec.hs "Prax.Beliefs"
    //
    // The frozen `Prax.BeliefsSpec`, re-expressed against the Rust engine.
    use super::*;
    use prax_core::engine::State;
    use prax_core::types::insert;

    /// Apply outcomes to an empty state, return the resulting labeled facts.
    fn run(outs: &[Outcome]) -> Vec<String> {
        let mut st = State::new();
        for o in outs {
            st.perform_outcome(o).expect("a belief outcome");
        }
        st.labeled_facts()
    }

    // H: BeliefsSpec.hs "beliefSentence / beliefAbout build the expected paths"
    #[test]
    fn belief_sentence_and_belief_about_build_the_expected_paths() {
        assert_eq!(
            belief_sentence("bex", "resentedBy.ada", "yes"),
            "bex.believes.resentedBy.ada!yes"
        );
        assert_eq!(belief_about("bex", "sky"), "bex.believes.sky");
    }

    // H: BeliefsSpec.hs "believe records a belief; believesThat matches it"
    #[test]
    fn believe_records_a_belief_and_believes_that_matches_it() {
        assert!(
            run(&[believe("bex", "sky", "green")]).contains(&"bex.believes.sky!green".to_owned()),
            "belief recorded"
        );
        assert_eq!(
            believes_that("bex", "sky", "green"),
            matches("bex.believes.sky!green"),
            "believesThat is the Match on the belief sentence, `!` and all"
        );
    }

    // H: BeliefsSpec.hs "a new value for an issue overrides the old (single-slot)"
    #[test]
    fn a_new_value_for_an_issue_overrides_the_old() {
        let fs = run(&[believe("bex", "sky", "green"), believe("bex", "sky", "blue")]);
        assert!(
            fs.contains(&"bex.believes.sky!blue".to_owned()),
            "new value present, got {fs:?}"
        );
        assert!(
            !fs.contains(&"bex.believes.sky!green".to_owned()),
            "old value gone, got {fs:?}"
        );
    }

    // H: BeliefsSpec.hs "forget drops the belief"
    #[test]
    fn forget_drops_the_belief() {
        let fs = run(&[believe("bex", "sky", "blue"), forget("bex", "sky")]);
        assert!(
            !fs.iter().any(|s| s.starts_with("bex.believes.sky")),
            "no sky belief remains, got {fs:?}"
        );
    }

    // H: BeliefsSpec.hs "beliefs are per-agent: two agents can disagree"
    #[test]
    fn beliefs_are_per_agent_two_agents_can_disagree() {
        let fs = run(&[
            believe("bex", "murderer", "ada"),
            believe("cid", "murderer", "you"),
        ]);
        assert!(
            fs.contains(&"bex.believes.murderer!ada".to_owned()),
            "bex's view, got {fs:?}"
        );
        assert!(
            fs.contains(&"cid.believes.murderer!you".to_owned()),
            "cid's view, got {fs:?}"
        );
    }

    // H: BeliefsSpec.hs "a belief can be false — diverging from the shared world"
    #[test]
    fn a_belief_can_be_false_diverging_from_the_shared_world() {
        // Shared world: ada is actually pleased. bex nonetheless believes she is
        // cross.
        let mut st = State::new();
        st.perform_outcome(&insert("ada.mood!pleased"))
            .expect("the world's truth");
        st.perform_outcome(&believe("bex", "adaMood", "cross"))
            .expect("bex's belief");
        let fs = st.labeled_facts();
        assert!(
            fs.contains(&"ada.mood!pleased".to_owned()),
            "world truth stands, got {fs:?}"
        );
        assert!(
            fs.contains(&"bex.believes.adaMood!cross".to_owned()),
            "bex's belief diverges, got {fs:?}"
        );
        // Reading bex's belief yields `cross`, not the world's `pleased`: the
        // frozen pin binds `V` in `Match (beliefAbout bex adaMood ++ "!V")` and
        // asserts one row valued `cross`. The child keys of the belief path are
        // that binding's value set — one slot, because `!` excludes.
        assert_eq!(
            st.db_child_keys(&belief_about("bex", "adaMood")),
            vec!["cross".to_owned()],
            "the belief path binds exactly one value, and it is bex's, not the world's"
        );
    }
}
