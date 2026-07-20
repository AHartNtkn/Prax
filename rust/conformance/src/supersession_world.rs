//! The v44 SUPERSESSION law at WORLD scale — the one net the four S7 slices do
//! not provide, written here because nothing else in the program has it.
//!
//! **The law**: a BARE insert onto a path that already carries a pending expiry
//! CANCELS that expiry, so the fact stands permanently
//! (`prax_core::engine::perform_effect`'s `rt.expiries.remove(path)` under
//! `Effect::Insert`). An `InsertFor` onto the same path routes through the same
//! branch but re-arms afterwards, which is a REFRESH, not a supersession — and
//! that distinction is the whole trap: a naive counter reports refreshes as
//! supersessions.
//!
//! **Why this file exists.** Neither bar (slice 3) nor village (slice 4) reaches
//! the case with its own authored data: both write `*.feels.*` through exactly
//! two routes, `feel_toward_for` (an `InsertFor`) and `unfeel_toward` (a
//! `Delete`), and neither world writes that family — or any other timed family —
//! with a bare `Insert`. Measured, not argued: with `rt.expiries.remove(path)`
//! deleted outright, `worldshape village --check`, the 42-turn village trace,
//! sixty randtrace seeds and the 300-seed matrix all stay CLEAN, and the only
//! REDs in the whole workspace are the two S4/S5 UNIT pins
//! (`schedule_spec::law_3_bare_re_insert_cancels_the_timer` and
//! `engine_replay::engine_scenarios_replay_byte_for_byte`). Those pins drive a
//! bare `State::new()`: no practices, no rules, no derived view, no eviction
//! shadows. So the law was netted only against an empty engine.
//!
//! This pin closes that half of the gap: the same law, driven over the real
//! compiled `village_world()` — its rules, tiers, footprint, derived view and
//! full character roster live — through the world's OWN emotion vocabulary. The
//! half it does NOT close is recorded as a CARRIED GAP in
//! `docs/rewrite/PROGRAM.md`: no shipped world's authored data reaches the case,
//! so a bug that only manifests through authoring still has no world-scale net.

#[cfg(test)]
mod tests {
    use prax_core::types::insert;
    use prax_vocab::emotion::{ANGRY, feel_toward_for};
    use prax_worlds::village::village_world;

    /// The exact path the village's two shun draws write, with the draw's
    /// variables ground to a real pair from the village roster. Spelled out
    /// rather than built from `feels_path`, which is private in both the frozen
    /// `Prax.Emotion` and its Rust port.
    fn carols_anger_at_dana() -> String {
        "carol.feels.angry.toward.dana".to_owned()
    }

    /// The CONTROL: without a bare insert, the armed timer fires and the fact
    /// goes. Without this half, the pin below could pass against an engine that
    /// never expires anything.
    #[test]
    fn a_village_timer_left_alone_fires_and_the_feeling_goes() {
        let mut st = village_world();
        let path = carols_anger_at_dana();
        st.perform_outcome(&feel_toward_for(4, "carol", ANGRY, "dana"))
            .expect("arm the village's own flare timer");
        assert!(st.db_has(&path), "the feeling is present at onset");
        let due = st.current_turn() + 4;
        assert_eq!(
            st.expiries_rendered().get(&path).copied(),
            Some(due),
            "the flare is queued 4 boundaries out"
        );
        for _ in 0..4 {
            st.round_boundary();
        }
        assert!(
            !st.db_has(&path),
            "the flare expired at its due -- the control holds"
        );
    }

    /// The law itself, at world scale: a bare insert onto the live timer's exact
    /// path cancels the due, and the feeling then outlives it indefinitely.
    #[test]
    fn a_bare_insert_onto_a_live_village_timer_supersedes_it() {
        let mut st = village_world();
        let path = carols_anger_at_dana();
        st.perform_outcome(&feel_toward_for(4, "carol", ANGRY, "dana"))
            .expect("arm the village's own flare timer");
        assert!(
            st.expiries_rendered().contains_key(&path),
            "precondition: the timer is live before the bare insert"
        );

        st.perform_outcome(&insert(path.clone()))
            .expect("the bare insert onto the live timer's path");

        assert!(
            !st.expiries_rendered().contains_key(&path),
            "the bare insert SUPERSEDES the pending expiry: the due is gone"
        );
        for _ in 0..8 {
            st.round_boundary();
        }
        assert!(
            st.db_has(&path),
            "the superseded feeling still stands eight boundaries later -- \
             twice the lifetime it was armed with"
        );
    }

    /// The trap, pinned so it cannot be mistaken for the law: a re-`InsertFor`
    /// goes through the SAME `Effect::Insert` branch (and so through the same
    /// `expiries.remove`) and then re-arms. It is a refresh; the fact still dies.
    #[test]
    fn a_re_insert_for_refreshes_rather_than_supersedes() {
        let mut st = village_world();
        let path = carols_anger_at_dana();
        st.perform_outcome(&feel_toward_for(4, "carol", ANGRY, "dana"))
            .expect("arm");
        st.round_boundary();
        st.round_boundary();
        let refreshed_due = st.current_turn() + 4;
        st.perform_outcome(&feel_toward_for(4, "carol", ANGRY, "dana"))
            .expect("refresh");
        assert_eq!(
            st.expiries_rendered().get(&path).copied(),
            Some(refreshed_due),
            "the timer is RE-ARMED at the new due, not cancelled"
        );
        for _ in 0..4 {
            st.round_boundary();
        }
        assert!(
            !st.db_has(&path),
            "a refreshed flare still dies -- only a BARE insert supersedes"
        );
    }
}
