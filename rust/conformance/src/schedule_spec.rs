//! `Prax.ScheduleSpec` re-expressed: the exact-path expiry queue's
//! supersession/purge/eviction laws and the round-boundary function's ordering
//! and re-arm laws, driven directly on a tiny clocked fixture through the PUBLIC
//! State API (`perform_outcome`/`round_boundary`/`set_schedule`) — no white-box
//! `dueAt`. Where the frozen forces a due to a specific value with `dueAt`, the
//! Rust seeds dues only through `set_schedule` (a period out) and reaches the same
//! LAW either by the natural period seeding or by the clock-jump idiom (a plain
//! `Insert "turn!<n>"`), noted per test.

#[cfg(test)]
mod tests {
    use prax_core::engine::State;
    use prax_core::query::Condition;
    use prax_core::types::{ScheduleRule, delete, insert, insert_for};
    use proptest::prelude::*;

    fn m(s: &str) -> Condition {
        Condition::Match(s.to_owned())
    }

    // ===== the expiry queue =====

    // H: ScheduleSpec.hs "Prax.Schedule"
    // H: ScheduleSpec.hs "the expiry queue"
    // H: ScheduleSpec.hs "law 1: an InsertFor fact holds for n boundaries and is gone at the nth"
    #[test]
    fn law_1_insertfor_holds_for_n_boundaries() {
        let mut st = State::new();
        st.perform_outcome(&insert_for(2, "mood!a")).unwrap(); // due at 0+2 = 2
        assert_eq!(st.current_turn(), 0);
        assert!(st.db_has("mood.a"), "present at insert");
        st.round_boundary(); // now 1: not yet due
        assert!(st.db_has("mood.a"), "present after 1 boundary");
        st.round_boundary(); // now 2: due, retracted
        assert!(!st.db_has("mood.a"), "gone at the 2nd boundary");
        assert!(st.expiries_rendered().is_empty());
    }

    // H: ScheduleSpec.hs "law 2: re-InsertFor before due refreshes — survives the old due, dies at the new"
    #[test]
    fn law_2_re_insertfor_refreshes() {
        let mut st = State::new();
        st.perform_outcome(&insert_for(2, "mood!a")).unwrap(); // due at 2
        st.round_boundary(); // now 1: holds
        st.perform_outcome(&insert_for(2, "mood!a")).unwrap(); // refresh: due at 1+2 = 3
        st.round_boundary(); // now 2: past the OLD due, still holds
        assert!(st.db_has("mood.a"), "survives past the old due (turn 2)");
        st.round_boundary(); // now 3: the new due, retracted
        assert!(!st.db_has("mood.a"), "dies at the refreshed due (turn 3)");
    }

    // H: ScheduleSpec.hs "law 3: a bare re-insert cancels the timer — the fact never expires"
    #[test]
    fn law_3_bare_re_insert_cancels_the_timer() {
        let mut st = State::new();
        st.perform_outcome(&insert_for(2, "mood!a")).unwrap();
        st.perform_outcome(&insert("mood!a")).unwrap(); // bare: cancels the expiry
        assert!(st.expiries_rendered().is_empty());
        for _ in 0..5 {
            st.round_boundary();
        }
        assert!(st.db_has("mood.a"), "still standing after five boundaries");
    }

    // H: ScheduleSpec.hs "law 4: an authored delete purges the subtree's timers — no later ghost retract"
    #[test]
    fn law_4_authored_delete_purges_the_subtrees_timers() {
        let mut st = State::new();
        st.perform_outcome(&insert_for(2, "feels.anger.toward.bob")).unwrap();
        st.perform_outcome(&delete("feels.anger")).unwrap(); // purges the pending timer
        assert!(st.expiries_rendered().is_empty());
        st.perform_outcome(&insert("feels.anger.toward.bob")).unwrap(); // re-assert PERMANENTLY
        for _ in 0..5 {
            st.round_boundary();
        }
        assert!(
            st.db_has("feels.anger.toward.bob"),
            "the re-asserted fact is untouched by any stale timer"
        );
    }

    // H: ScheduleSpec.hs "law 5: a !-eviction leaves the displaced timer queued, and its firing is a harmless no-op"
    #[test]
    fn law_5_eviction_leaves_the_displaced_timer_queued() {
        let mut st = State::new();
        st.perform_outcome(&insert_for(2, "mood!a")).unwrap(); // due at 2
        st.perform_outcome(&insert("mood!b")).unwrap(); // excludes: mood.a evicted, timer stays queued
        assert!(!st.db_has("mood.a"), "the displaced value is already gone");
        st.round_boundary(); // now 1
        st.round_boundary(); // now 2: mood!a due, but its fact is gone
        assert!(st.db_has("mood.b"), "the surviving value still stands");
        assert!(st.expiries_rendered().is_empty());
    }

    // H: ScheduleSpec.hs "law 6: a lifetime on an interior path takes its descendants at expiry"
    #[test]
    fn law_6_interior_lifetime_takes_its_descendants() {
        let mut st = State::new();
        st.perform_outcome(&insert_for(2, "feels.anger")).unwrap();
        st.perform_outcome(&insert("feels.anger.toward.bob")).unwrap();
        st.round_boundary();
        st.round_boundary(); // now 2: interior retract
        assert!(!st.db_has("feels.anger"), "the interior fact is gone");
        assert!(
            !st.db_has("feels.anger.toward.bob"),
            "its descendant went with it"
        );
    }

    // ===== the round boundary =====

    // H: ScheduleSpec.hs "the round boundary"
    // H: ScheduleSpec.hs "law 7 (ghost observation): expiries fire BEFORE rules — a period-1 rule does not see an expiring fact"
    #[test]
    fn law_7_ghost_observation_expiries_fire_before_rules() {
        let sighting =
            ScheduleRule::new("sighting", 1).clause([m("mood!Now")], [insert("sighted.Now")]);
        // Control: lifetime 2, so at boundary 1 the fact still stands and the rule stamps it.
        let mut ctl = State::new();
        ctl.set_schedule(vec![sighting.clone()]).unwrap(); // period 1 -> due at 1
        ctl.perform_outcome(&insert_for(2, "mood!a")).unwrap();
        ctl.round_boundary();
        // Ghost: lifetime 1, so the fact expires AT boundary 1 — before the rule runs.
        let mut ghost = State::new();
        ghost.set_schedule(vec![sighting]).unwrap();
        ghost.perform_outcome(&insert_for(1, "mood!a")).unwrap();
        ghost.round_boundary();
        assert!(
            ctl.db_has("sighted.a"),
            "control: the rule stamps a still-present fact"
        );
        assert!(
            !ghost.db_has("mood.a"),
            "ghost: the fact expired this boundary"
        );
        assert!(
            !ghost.db_has("sighted.a"),
            "ghost: the rule never saw it (no stamp)"
        );
    }

    // H: ScheduleSpec.hs "law 8a: a due rule re-arms period boundaries FROM NOW (fires at 1 and 3, not 2)"
    #[test]
    fn law_8a_re_arms_from_now() {
        // The frozen forces the due to 1 (via `dueAt`) so a period-2 rule fires at
        // boundaries 1 and 3, skipping 2. The Rust seeds the due a period out (2)
        // through the public `set_schedule`, so the SAME every-other-boundary re-arm
        // law is pinned one boundary later: fires at 2 and 4, skips 3.
        let beat = ScheduleRule::new("beat", 2).clause([m("turn!Now")], [insert("beat.Now")]);
        let mut st = State::new();
        st.set_schedule(vec![beat]).unwrap(); // due at 0+2 = 2
        for _ in 0..4 {
            st.round_boundary(); // boundaries 1, 2, 3, 4
        }
        assert!(st.db_has("beat.2"), "fired at boundary 2");
        assert!(!st.db_has("beat.3"), "skipped boundary 3");
        assert!(
            st.db_has("beat.4"),
            "fired again at boundary 4 (re-armed FROM 2, not from 3)"
        );
    }

    // H: ScheduleSpec.hs "law 8a (late fire): re-arm is FROM the boundary it fires at, not the stale due"
    #[test]
    fn law_8a_late_fire_re_arms_from_the_boundary() {
        // Clock-jump so the due is already in the PAST — a late fire — so "from now"
        // (11+2=13) and "from the stale due" (2+2=4) diverge.
        let beat = ScheduleRule::new("beat", 2).clause([m("turn!Now")], [insert("beat.Now")]);
        let mut st = State::new();
        st.set_schedule(vec![beat]).unwrap(); // due at 2
        st.perform_outcome(&insert("turn!10")).unwrap(); // clock-jump: now overdue
        st.round_boundary(); // now 11: fires late
        assert!(st.db_has("beat.11"), "fired late, at boundary 11");
        assert_eq!(
            st.schedule_dues().get("beat"),
            Some(&13),
            "11 + period, not 2 + period"
        );
    }

    // H: ScheduleSpec.hs "law 8b: due rules fire in declaration order within a boundary"
    #[test]
    fn law_8b_due_rules_fire_in_declaration_order() {
        let open = ScheduleRule::new("open", 1).clause(Vec::<Condition>::new(), [insert("gate.open")]);
        let pass = ScheduleRule::new("pass", 1).clause([m("gate.open")], [insert("passed.here")]);
        // [open, pass]: open runs first, so pass sees the gate the same boundary.
        let mut st = State::new();
        st.set_schedule(vec![open, pass]).unwrap(); // both period 1 -> due at 1
        st.round_boundary();
        assert!(
            st.db_has("passed.here"),
            "the second rule saw the first's effect"
        );
    }

    // H: ScheduleSpec.hs "the clock advances one per boundary"
    #[test]
    fn the_clock_advances_one_per_boundary() {
        let mut st = State::new();
        for _ in 0..3 {
            st.round_boundary();
        }
        assert_eq!(st.current_turn(), 3);
    }

    // ===== boundary purity (proptest): the boundary is a pure function of state =====

    proptest! {
        // Same LOGICAL state -> same boundary result: two states built with the
        // SAME expiries inserted in different orders (so their FxHashMap layouts may
        // differ) must reach byte-identical facts/dues/expiries after a boundary.
        // This is the guard that the expiry HashMap's incidental iteration order
        // never leaks into observable state (S-panel I1 / the determinism contract).
        #[test]
        fn round_boundary_is_insertion_order_insensitive(
            idxs in prop::sample::subsequence(vec![0usize, 1, 2, 3, 4, 5], 1..=6)
        ) {
            const POOL: [&str; 6] = [
                "feels.anger", "feels.anger.toward.bob", "feels.joy",
                "mood!a", "mood!b", "at.dawn",
            ];
            let build = |order: &[usize]| {
                let mut st = State::new();
                st.set_schedule(vec![
                    ScheduleRule::new("beat", 1)
                        .clause(Vec::<Condition>::new(), [insert("tick.done")]),
                ])
                .unwrap();
                for &i in order {
                    st.perform_outcome(&insert_for(1, POOL[i])).unwrap();
                }
                st
            };
            let mut rev = idxs.clone();
            rev.reverse();
            let mut a = build(&idxs);
            a.round_boundary();
            let mut b = build(&rev);
            b.round_boundary();
            prop_assert_eq!(a.labeled_facts(), b.labeled_facts());
            prop_assert_eq!(a.schedule_dues(), b.schedule_dues());
            prop_assert_eq!(a.expiries_rendered(), b.expiries_rendered());
        }
    }
}
