//! The three `LoopSpec` rows that could only be written once BOTH the loop (S6)
//! and `barWorld` (S7) existed: the 25-turn narration golden, the emergent +
//! director-driven outcomes it leaves behind, and the dead-character skip driven
//! through a shipped world rather than a fixture.
//!
//! These are the S7-owed discharges from `conformance/KILLED.md`, removed there
//! as they land here. They are NATIVE Rust re-expressions over
//! [`prax_core::turn::run_npc_ticks`], and they are ALSO cross-checked by
//! `prax-oracle compare bar --mode trace`, which drives the frozen and Rust
//! engines side by side over the same world — two independent nets, not one,
//! because the two have different SHAPES: `runNpcTicks` omits an idle turn
//! entirely, where the comparator's `driveLabels` emits `"<name>: -"` for it
//! (S7 design §5). A bug that moved an idle turn to a real one would be visible
//! in both; a bug in how an idle turn is RENDERED is visible only in the other.
//!
//! The expected narration is LOADED from `conformance/goldens/loop-bar-25.txt`
//! ([`crate::goldens::load`]) and never typed here [D-C3(b)]; the repo-wide
//! `no_inline_golden_literals` gate sweeps this file like every other.

#[cfg(test)]
mod tests {
    use crate::goldens::load;
    use prax_core::turn::run_npc_ticks;
    use prax_core::types::insert;
    use prax_worlds::bar::bar_world;

    // H: LoopSpec.hs "25-turn NPC replay matches the golden narration"
    #[test]
    fn twenty_five_turn_npc_replay_matches_the_golden_narration() {
        let mut st = bar_world();
        let got = run_npc_ticks(&mut st, 2, 25);
        for (i, line) in got.iter().enumerate() {
            println!("{i:2}: {line}");
        }
        assert_eq!(
            got,
            load("loop-bar-25"),
            "the 25-turn bar narration must match the frozen golden line for line"
        );
    }

    // H: LoopSpec.hs "the emergent + director-driven outcomes hold after the replay"
    #[test]
    fn the_emergent_and_director_driven_outcomes_hold_after_the_replay() {
        let mut st = bar_world();
        run_npc_ticks(&mut st, 2, 25);
        let fs = st.labeled_facts();
        let has = |f: &str| assert!(fs.contains(&f.to_owned()), "{f} missing from {fs:?}");

        // bex responded to ada's greeting via the reaction
        has("practice.greet.world.greeted.bex.ada");
        // Grudging courtesy prices ada's take-offense down, so the
        // ignored-greeting grievance never arises in this run.
        assert!(
            !fs.contains(&"practice.greet.world.grievance.ada.you".to_owned()),
            "ada never bears the ignored-greeting grievance"
        );
        // the director intervened once, injecting a rivalry between the friends
        has("dm.stirred");
        has("practice.greet.world.grievance.ada.bex");
        // With the ticker turns gone the 4-member round reaches further in 25
        // turns, so bex's arc completes — she settles in to belonging (its own
        // warmth held even as the director soured ada toward it), leaving
        // hopeful behind.
        has("bex.arc!belonging");
        assert!(
            !fs.contains(&"bex.arc!hopeful".to_owned()),
            "bex has moved on from hopeful once she belongs"
        );
        // no NPC ever chose the against-desires transformation
        assert!(
            !fs.contains(&"bex.arc!lonely".to_owned())
                && !fs.contains(&"you.arc!lonely".to_owned()),
            "no NPC resigned to solitude, got {fs:?}"
        );
    }

    // H: LoopSpec.hs "a dead character is skipped in turn-taking"
    #[test]
    fn a_dead_character_is_skipped_in_turn_taking() {
        // mark bex dead; over a full run bex must never act again
        let mut st = bar_world();
        st.perform_outcome(&insert("dead.bex")).expect("killing bex");
        let tr = run_npc_ticks(&mut st, 2, 16);
        assert!(
            !tr.iter().any(|l| l.contains("bex:")),
            "bex takes no turns once dead, got {tr:?}"
        );
    }
}
