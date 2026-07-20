//! Coexisting episodic feelings (spec v38), replacing the Versu-inherited
//! single-slot mood.
//!
//! `<who>.feels.<emotion>` and `<who>.feels.<emotion>.toward.<target>` are plain
//! multi-valued facts — angry at two people while afraid of a third all coexist,
//! each independent.
//!
//! THE INVARIANT (user, load-bearing): **feelings change decision-making, never
//! what decisions can be made.** Nothing here touches action availability;
//! pricing is ordinary desires reading these facts, authored per world.
//!
//! Onset is authored at the provoking action; wear-off is an authored LIFETIME
//! on the onset — [`feel_for`]/[`feel_toward_for`] assert through the engine's
//! expiry queue (`Prax.Types.InsertFor`), so each feeling lives its own `n`
//! rounds from its own onset (the v36 episodic principle, no synchronized
//! sweep). Feelings are EPISODIC: they fade; dispositions (traits, marks) never
//! do — a trait makes a feeling LIKELIER, not longer.
//!
//! Frozen reference: `src/Prax/Emotion.hs`. Infallible throughout: the frozen
//! module raises nowhere, and its one helper (`feelsPath`) is a bare `++`, never
//! `pathNames`/`tokens` — checked one call deeper, per slice 1's [C2].

use prax_core::query::{Condition, matches};
use prax_core::types::{Outcome, delete, insert, insert_for};

/// An Ekman-based vocabulary (moved from `Prax.Core`; plain names).
pub const HAPPY: &str = "happy";
/// See [`HAPPY`].
pub const SAD: &str = "sad";
/// See [`HAPPY`].
pub const ANGRY: &str = "angry";
/// See [`HAPPY`].
pub const AFRAID: &str = "afraid";
/// See [`HAPPY`].
pub const DISGUSTED: &str = "disgusted";
/// See [`HAPPY`].
pub const SURPRISED: &str = "surprised";
/// See [`HAPPY`].
pub const ANNOYED: &str = "annoyed";
/// See [`HAPPY`].
pub const PLEASED: &str = "pleased";

/// The module's OWN path helper (`Emotion.hs:38`): `<who>.feels.<emotion>`.
/// Every builder below composes on it rather than rebuilding the prefix.
fn feels_path(who: &str, emotion: &str) -> String {
    format!("{who}.feels.{emotion}")
}

/// `who` comes to feel `emotion` (untargeted).
pub fn feel(who: &str, emotion: &str) -> Outcome {
    insert(feels_path(who, emotion))
}

/// `who` comes to feel `emotion` toward `target`. Arguments may be action
/// variables, grounded when the outcome runs.
pub fn feel_toward(who: &str, emotion: &str, target: &str) -> Outcome {
    insert(feels_path(who, emotion) + ".toward." + target)
}

/// Like [`feel`], but the feeling EXPIRES `n` round boundaries after onset (the
/// one lifetime mechanism, `Prax.Types.InsertFor`). Each onset lives its own `n`
/// rounds from when it fired; re-feeling refreshes the timer, discharging
/// ([`unfeel`]) purges it.
pub fn feel_for(n: i64, who: &str, emotion: &str) -> Outcome {
    insert_for(n, feels_path(who, emotion))
}

/// Like [`feel_toward`], with an `n`-boundary lifetime (see [`feel_for`]).
pub fn feel_toward_for(n: i64, who: &str, emotion: &str, target: &str) -> Outcome {
    insert_for(n, feels_path(who, emotion) + ".toward." + target)
}

/// Discharge: the whole feeling goes, targets included (venting, confronting,
/// being won over — authored at the discharging action).
pub fn unfeel(who: &str, emotion: &str) -> Outcome {
    delete(feels_path(who, emotion))
}

/// Discharge one target of a feeling, leaving the others standing.
pub fn unfeel_toward(who: &str, emotion: &str, target: &str) -> Outcome {
    delete(feels_path(who, emotion) + ".toward." + target)
}

/// `who` currently feels `emotion` — true the instant onset writes ANY instance,
/// targeted or not (`Match` sees subtrees). Since v39 (`Prax.Db.retract` prunes
/// unasserted childless nodes) this correctly falls back to false the moment the
/// last instance is discharged: the now-childless, never-asserted `.toward`
/// scaffold is pruned rather than left standing, so there is no residue for a
/// subtree `Match` to read.
pub fn feeling(who: &str, emotion: &str) -> Condition {
    matches(feels_path(who, emotion))
}

/// `who` feels `emotion` toward `target` — pass an already-known target for a
/// specific check, or a fresh variable (the caller's own choice of name) to bind
/// it to an ACTUAL remaining target, so a want priced over it counts once per
/// standing grudge. The choice between this and [`feeling`] is about SEMANTICS
/// (per-target pricing versus a single presence test), not about avoiding a
/// residue trap: the recommended shape for any PRICING want over "still feels
/// this, toward whoever".
pub fn feeling_toward(who: &str, emotion: &str, target: &str) -> Condition {
    matches(feels_path(who, emotion) + ".toward." + target)
}

#[cfg(test)]
mod tests {
    // H: EmotionSpec.hs "Prax.Emotion"
    //
    // The frozen `Prax.EmotionSpec`, re-expressed against the Rust engine.
    use super::*;
    use prax_core::db::{Bindings, Db};
    use prax_core::engine::State;
    use prax_core::interner::Interner;
    use prax_core::path::tokenize;
    use prax_core::query::{compile_condition, neq, query};
    use prax_core::types::{Action, Character, Practice, insert};

    fn perform(st: &mut State, o: &Outcome) {
        st.perform_outcome(o).expect("a feeling outcome");
    }

    /// The frozen pin's `query (db st) [cond] Map.empty`, over a db built from
    /// the given sentences: does the condition have any solution?
    fn holds(sentences: &[&str], cond: &Condition) -> bool {
        let mut interner = Interner::new();
        let mut db = Db::empty();
        for s in sentences {
            let p = tokenize(&mut interner, s).expect("a feeling sentence");
            db = db.insert(&p);
        }
        let compiled = compile_condition(&mut interner, cond).expect("a feeling condition");
        !query(&mut interner, &db, &[compiled], &Bindings::new()).is_empty()
    }

    // H: EmotionSpec.hs "coexisting feelings"
    // (the frozen group label; its four cases follow)

    // H: EmotionSpec.hs "angry at carol and afraid of bob coexist independently"
    #[test]
    fn angry_at_carol_and_afraid_of_bob_coexist_independently() {
        let mut st = State::new();
        perform(&mut st, &feel_toward("ada", ANGRY, "carol"));
        perform(&mut st, &feel_toward("ada", AFRAID, "bob"));
        let fs = st.labeled_facts();
        assert!(
            fs.contains(&"ada.feels.angry.toward.carol".to_owned()),
            "angry at carol, got {fs:?}"
        );
        assert!(
            fs.contains(&"ada.feels.afraid.toward.bob".to_owned()),
            "afraid of bob, got {fs:?}"
        );
    }

    // H: EmotionSpec.hs "unfeeling one leaves the other standing"
    #[test]
    fn unfeeling_one_leaves_the_other_standing() {
        let mut st = State::new();
        perform(&mut st, &feel_toward("ada", ANGRY, "carol"));
        perform(&mut st, &feel_toward("ada", AFRAID, "bob"));
        perform(&mut st, &unfeel_toward("ada", ANGRY, "carol"));
        let fs = st.labeled_facts();
        assert!(
            !fs.contains(&"ada.feels.angry.toward.carol".to_owned()),
            "angry at carol discharged, got {fs:?}"
        );
        assert!(
            fs.contains(&"ada.feels.afraid.toward.bob".to_owned()),
            "afraid of bob survives, got {fs:?}"
        );
    }

    // H: EmotionSpec.hs "untargeted and targeted instances of the same emotion coexist"
    #[test]
    fn untargeted_and_targeted_instances_of_the_same_emotion_coexist() {
        // Since v39's asserted-endpoint marking, the untargeted instance ALSO
        // survives as its own asserted fact even after gaining a targeted sibling
        // underneath — the old leaf-vs-ancestor ambiguity is gone.
        let mut st = State::new();
        perform(&mut st, &feel("ada", HAPPY));
        perform(&mut st, &feel_toward("ada", HAPPY, "carol"));
        assert!(st.db_has("ada.feels.happy"), "untargeted happy");
        assert!(st.db_has("ada.feels.happy.toward.carol"), "targeted happy");
    }

    // H: EmotionSpec.hs "unfeel discharges the whole feeling, targets included"
    #[test]
    fn unfeel_discharges_the_whole_feeling_targets_included() {
        let mut st = State::new();
        perform(&mut st, &feel_toward("ada", ANGRY, "carol"));
        perform(&mut st, &feel_toward("ada", ANGRY, "bob"));
        perform(&mut st, &unfeel("ada", ANGRY));
        let fs = st.labeled_facts();
        assert!(
            !fs.contains(&"ada.feels.angry.toward.carol".to_owned()),
            "angry at carol gone, got {fs:?}"
        );
        assert!(
            !fs.contains(&"ada.feels.angry.toward.bob".to_owned()),
            "angry at bob gone, got {fs:?}"
        );
    }

    // H: EmotionSpec.hs "feeling/feelingToward"
    // (the frozen group label; its two cases follow)

    // H: EmotionSpec.hs "feeling matches a targeted instance (Match sees subtrees)"
    #[test]
    fn feeling_matches_a_targeted_instance() {
        assert!(
            holds(
                &["ada.feels.annoyed.toward.carol"],
                &feeling("ada", ANNOYED)
            ),
            "the untargeted Match sees the targeted instance's subtree"
        );
    }

    // H: EmotionSpec.hs "feelingToward requires the specific target"
    #[test]
    fn feeling_toward_requires_the_specific_target() {
        assert!(
            holds(
                &["ada.feels.annoyed.toward.carol"],
                &feeling_toward("ada", ANNOYED, "carol")
            ),
            "feelingToward carol holds"
        );
        assert!(
            !holds(
                &["ada.feels.annoyed.toward.carol"],
                &feeling_toward("ada", ANNOYED, "bob")
            ),
            "feelingToward bob does not"
        );
    }

    // H: EmotionSpec.hs "feelings fade"
    // (the frozen group label; its one case follows)

    // H: EmotionSpec.hs "both feelings gone after the lifetime lapses; feeling again is possible"
    #[test]
    fn both_feelings_gone_after_the_lifetime_lapses() {
        // v44: each onset carries its own expiry (`feel_toward_for` ->
        // InsertFor), and the engine retracts it at the round boundary its
        // lifetime lapses — per-onset spans, no synchronized sweep.
        let mut st = State::new();
        perform(&mut st, &feel_toward_for(2, "ada", ANGRY, "carol"));
        perform(&mut st, &feel_toward_for(2, "ada", AFRAID, "bob"));
        assert!(
            st.db_has("ada.feels.angry.toward.carol"),
            "angry present at onset"
        );
        assert!(
            st.db_has("ada.feels.afraid.toward.bob"),
            "afraid present at onset"
        );

        st.round_boundary(); // turn 1: not yet due (due at 0+2)
        assert!(
            st.db_has("ada.feels.angry.toward.carol"),
            "not yet lapsed after one boundary"
        );

        st.round_boundary(); // turn 2: the lifetime lapses
        assert!(
            !st.db_has("ada.feels.angry.toward.carol"),
            "angry gone at the due boundary"
        );
        assert!(
            !st.db_has("ada.feels.afraid.toward.bob"),
            "afraid gone at the due boundary"
        );

        // reappear-able: feeling again after the fade works exactly as before.
        perform(&mut st, &feel_toward("ada", ANGRY, "carol"));
        assert!(
            st.db_has("ada.feels.angry.toward.carol"),
            "angry can be felt again after fading"
        );
    }

    // H: EmotionSpec.hs "the invariant: feelings never gate action availability"
    // (the frozen group label; its one case follows)

    // H: EmotionSpec.hs "candidateActions is identical with and without every feeling"
    #[test]
    fn candidate_actions_is_identical_with_and_without_every_feeling() {
        // THE INVARIANT (load-bearing): emotions change decision-making, never
        // what decisions can be made. A fixture character's full candidate list
        // must be identical with and without every vocabulary feeling present.
        let vocabulary = [
            HAPPY, SAD, ANGRY, AFRAID, DISGUSTED, SURPRISED, ANNOYED, PLEASED,
        ];
        let fixture_p = Practice::new("fixture")
            .name("a fixture affordance")
            .roles(["W"])
            .action(Action::new("[Actor]: wave").when([matches("practice.fixture.W")]))
            .action(
                Action::new("[Actor]: greet [Other]")
                    .when([matches("character.Other"), neq("Actor", "Other")]),
            );
        let fix = Character::new("fix");
        let friend = Character::new("pal");
        let mut world = State::new();
        world.define_practices([fixture_p]).expect("the fixture");
        world
            .set_characters(vec![fix.clone(), friend])
            .expect("the fixture cast");
        for o in [
            insert("character.fix"),
            insert("character.pal"),
            insert("practice.fixture.fix"),
        ] {
            perform(&mut world, &o);
        }
        let baseline: Vec<String> = world
            .candidate_actions(&fix)
            .into_iter()
            .map(|g| g.label)
            .collect();

        let mut with_feelings = world.clone();
        for e in vocabulary {
            perform(&mut with_feelings, &feel("fix", e));
            perform(&mut with_feelings, &feel_toward("fix", e, "pal"));
        }
        let after: Vec<String> = with_feelings
            .candidate_actions(&fix)
            .into_iter()
            .map(|g| g.label)
            .collect();
        assert_eq!(
            baseline, after,
            "the candidate list must not move when every vocabulary feeling is present"
        );
    }
}
