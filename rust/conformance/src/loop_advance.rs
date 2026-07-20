//! The `Prax.Loop.advance` half re-expressed: WHEN the round boundary fires and
//! who is re-selected after it (the wrap/boundary/liveness family). No `npcAct` is
//! involved — these pins are about the loop's rotation wiring, not any decision.
//!
//! The rest of `LoopSpec` drives `npcAct`/`runNpcTicks` (the planner, S6) over the
//! shipped worlds (barWorld/villageWorld, S7); those labels are killed in
//! `conformance/KILLED.md` with their owing stage, since neither the planner nor
//! the worlds exist yet.

#[cfg(test)]
mod tests {
    use prax_core::engine::State;
    use prax_core::query::Condition;
    use prax_core::turn::advance;
    use prax_core::types::{Character, Practice, ScheduleRule, insert};

    /// A minimal cast over an empty practice set with an optional engine schedule —
    /// just enough for [`advance`] to exercise the round-boundary wiring (v44).
    fn boundary_world(names: &[&str], rules: Vec<ScheduleRule>) -> State {
        let mut st = State::new();
        st.define_practices(Vec::<Practice>::new()).unwrap();
        st.set_characters(names.iter().map(|n| Character::new(*n)).collect())
            .unwrap();
        st.set_schedule(rules).unwrap();
        st
    }

    /// A period-1 rule that stamps a fact every boundary (its firing is observable).
    fn beat_rule() -> ScheduleRule {
        ScheduleRule::new("beat", 1).clause(Vec::<Condition>::new(), [insert("tick.done")])
    }

    // H: LoopSpec.hs "Prax.Loop"
    // H: LoopSpec.hs "no boundary fires before round 1 (cursor -1 selects index 0, no wrap)"
    #[test]
    fn no_boundary_fires_before_round_1() {
        let mut st = boundary_world(&["a", "b", "c"], vec![beat_rule()]);
        let actor = advance(&mut st);
        assert_eq!(actor.name, "a");
        assert_eq!(st.cursor(), 0);
        assert!(st.db_has("turn!0"), "clock not advanced before round 1");
        assert!(
            !st.db_has("tick.done"),
            "no schedule rule fired before round 1"
        );
    }

    // H: LoopSpec.hs "a single-survivor cast wraps every turn (i == cursor), firing the boundary"
    #[test]
    fn a_single_survivor_cast_wraps_every_turn() {
        let mut st = boundary_world(&["solo"], vec![beat_rule()]);
        let _ = advance(&mut st); // selects solo, cursor 0, no wrap
        let actor = advance(&mut st); // i == cursor 0: WRAP -> boundary
        assert_eq!(actor.name, "solo");
        assert!(st.db_has("turn!1"), "the boundary advanced the clock");
        assert!(st.db_has("tick.done"), "the period-1 rule fired at the wrap");
    }

    // H: LoopSpec.hs "the wrap skips a dead character and still fires the boundary"
    #[test]
    fn the_wrap_skips_a_dead_character_and_still_fires_the_boundary() {
        let mut st = boundary_world(&["a", "b", "c"], vec![beat_rule()]);
        st.perform_outcome(&insert("dead.b")).unwrap();
        let a1 = advance(&mut st); // a, cursor 0
        let a2 = advance(&mut st); // c (dead b skipped), cursor 2
        let a3 = advance(&mut st); // next living wraps to 0 <= cursor 2: boundary -> a
        assert_eq!(
            [a1.name, a2.name, a3.name],
            ["a".to_owned(), "c".to_owned(), "a".to_owned()]
        );
        assert!(
            st.db_has("turn!1"),
            "the boundary fired at the wrap past the dead"
        );
        assert!(st.db_has("tick.done"), "beat fired at that boundary");
    }

    // H: LoopSpec.hs "a schedule rule killing a character mid-wrap: the dead take no turn"
    #[test]
    fn a_schedule_rule_killing_a_character_mid_wrap_the_dead_take_no_turn() {
        let reaper =
            ScheduleRule::new("reaper", 1).clause(Vec::<Condition>::new(), [insert("dead.a")]);
        let mut st = boundary_world(&["a", "b", "c"], vec![reaper]);
        let _ = advance(&mut st); // a, cursor 0
        let _ = advance(&mut st); // b, cursor 1
        let _ = advance(&mut st); // c, cursor 2
        let a4 = advance(&mut st); // wrap: boundary fires reaper (kills a); re-select skips a -> b
        assert!(
            st.db_has("dead.a"),
            "the reaper killed a at the boundary"
        );
        assert_eq!(a4.name, "b");
    }
}
