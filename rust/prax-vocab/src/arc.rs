//! Character arcs (Versu paper §X).
//!
//! Where a social practice offers EXTERNAL, low-level actions, a character arc
//! represents a character's INTERNAL, high-level state — the through-line of
//! their evening. It is a single fact `X.arc!<stage>`: single-slot via `!`, so
//! entering a new stage overrides the old. A character's wants can be gated on
//! their arc stage, so advancing the arc reshapes what they pursue; and the arc
//! advances in response to what happens to them.
//!
//! A tiny reusable library over the existing engine, like [`crate::beliefs`].
//!
//! Frozen reference: `src/Prax/Arc.hs`. Signature for signature; the module's
//! own path helpers ([`arc_sentence`], [`arc_of`]) are ported rather than
//! replaced (§3). Nothing here raises: the frozen module is four `++`
//! expressions with no `error`, no `pathNames`, and no clash guard, at this
//! level or one call deeper.

use prax_core::query::{Condition, matches};
use prax_core::types::{Outcome, insert};

/// The sentence `who.arc!stage`.
pub fn arc_sentence(who: &str, stage: &str) -> String {
    format!("{who}.arc!{stage}")
}

/// The path `who.arc` — to bind the current stage (`matches(arc_of(who) + "!S")`).
pub fn arc_of(who: &str) -> String {
    format!("{who}.arc")
}

/// Condition: `who` is currently in arc stage `stage`.
pub fn arc_is(who: &str, stage: &str) -> Condition {
    matches(arc_sentence(who, stage))
}

/// `who` enters arc stage `stage` (overriding any previous stage).
pub fn enter_arc(who: &str, stage: &str) -> Outcome {
    insert(arc_sentence(who, stage))
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_core::engine::State;

    // H: ArcSpec.hs "Prax.Arc"
    //
    // The frozen `Prax.ArcSpec`, re-expressed: the sentence shape, the deposit,
    // and the `!`-slot override that makes an arc a stage rather than a set.

    fn run(outs: &[Outcome]) -> Vec<String> {
        let mut st = State::new();
        for o in outs {
            st.perform_outcome(o).expect("arc outcome");
        }
        st.labeled_facts()
    }

    // H: ArcSpec.hs "arcSentence / arcIs build the expected fact"
    #[test]
    fn arc_sentence_and_arc_is_build_the_expected_fact() {
        assert_eq!(arc_sentence("bex", "hopeful"), "bex.arc!hopeful");
        assert_eq!(
            arc_is("bex", "hopeful"),
            Condition::Match("bex.arc!hopeful".to_owned())
        );
    }

    // H: ArcSpec.hs "enterArc records the stage"
    #[test]
    fn enter_arc_records_the_stage() {
        assert!(
            run(&[enter_arc("bex", "hopeful")]).contains(&"bex.arc!hopeful".to_owned()),
            "the stage is recorded"
        );
    }

    // H: ArcSpec.hs "entering a new stage overrides the old (single-slot)"
    #[test]
    fn entering_a_new_stage_overrides_the_old() {
        let fs = run(&[enter_arc("bex", "hopeful"), enter_arc("bex", "belonging")]);
        assert!(
            fs.contains(&"bex.arc!belonging".to_owned()),
            "the new stage, got {fs:?}"
        );
        assert!(
            !fs.iter().any(|s| s.contains("arc!hopeful")),
            "the old stage is gone, got {fs:?}"
        );
    }

    /// [`arc_of`] carries no frozen spec label — it exists so a world can BIND
    /// the current stage instead of testing a known one. Pinned as the prefix of
    /// [`arc_sentence`], and exercised through the engine as a binder, because a
    /// drift between the two would leave every `arc_of`-built pattern reading a
    /// family nothing writes.
    #[test]
    fn arc_of_is_the_bindable_prefix_of_the_arc_sentence() {
        assert_eq!(arc_of("bex"), "bex.arc");
        assert_eq!(
            arc_sentence("bex", "hopeful"),
            format!("{}!hopeful", arc_of("bex"))
        );
        let mut st = State::new();
        st.perform_outcome(&enter_arc("bex", "hopeful")).unwrap();
        assert!(
            st.db_has(&format!("{}!hopeful", arc_of("bex"))),
            "the bindable path is the one the deposit writes"
        );
    }
}
