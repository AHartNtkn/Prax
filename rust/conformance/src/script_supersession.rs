//! **CG-1, closed.** The v44 SUPERSESSION law reached from a world's OWN
//! AUTHORED DATA, at world scale, through the JSON door.
//!
//! **The law**: a BARE insert onto a path that already carries a pending expiry
//! CANCELS that expiry, so the fact stands permanently
//! (`prax_core::engine::perform_effect`'s `rt.expiries.remove(path)` under
//! `Effect::Insert`). An `InsertFor` onto the same path routes through the same
//! branch but re-arms afterwards — a REFRESH, not a supersession.
//!
//! # What was carried, and why it is now discharged
//!
//! `PROGRAM.md`'s CG-1 carried this gap from S5 through S7: the law had an
//! ENGINE-scale net (S5's unit pins over a bare `State::new()`) and a
//! WORLD-scale net driven from OUTSIDE (`supersession_world`, which performs the
//! bare insert itself against `village_world()`), but no shipped world reached
//! the case through its own authored data — so a supersession bug that only
//! manifested through AUTHORING had no net anywhere.
//!
//! The S8 design asserted the gap could not close here either: *"the only timer
//! in the script machinery is the compiler-armed patience marker, and `compile`
//! REFUSES any authored `scenePatience` sentence in all five lists — so a bare
//! insert onto a live script timer is INEXPRESSIBLE in a script."*
//!
//! **That is FALSE, and this file is the counterexample.** The refusal covers
//! only the compiler-owned `scenePatience` family. `Outcome::InsertFor` is in
//! the AUTHORED outcome surface: a scene's `setup` accepts it, a beat's
//! `effects` accept it, and `prax_script::json` spells it directly
//! (`{"insertFor":{"rounds":n,"sentence":s}}`). So a script can arm a timer on
//! ANY path it likes and then bare-insert that same path from a beat — and
//! `compile` never looks.
//!
//! The world is `conformance/fixtures/cg1_supersession.json`, a real authored
//! play-script and not a synthetic state: a night watch whose scene entry arms
//! `lantern.lit` for three rounds, one beat that shields the lantern (a bare
//! `insert` of that exact path), and one junction that ends the story the moment
//! the lantern is out. Left alone the light fails at boundary 3; shielded, the
//! pending expiry is cancelled and the ending never comes.
//!
//! Measured on the FROZEN engine over the same file, before any of this was
//! written:
//!
//! ```text
//! control    (i, lantern.lit, ending) = [(0,True,Nothing),(1,True,Nothing),(2,True,Nothing),
//!                                        (3,False,Just "darkness"), … ]
//! supersede  expiries after the beat  = []
//! supersede  (i, lantern.lit, ending) = [(0,True,Nothing) … (8,True,Nothing)]
//! ```
//!
//! The same file is the differential's `cg1` world on BOTH sides — the frozen
//! oracle reads it at `oracle/TraceMain.hs`'s `cg1ScriptPath`, the Rust registry
//! embeds it at `prax_oracle::worlds::CG1_SCRIPT_JSON` — so the two engines
//! cannot be driven by different scripts.
//!
//! These pins carry no `// H:` label: no frozen test drives this world, because
//! the world did not exist until S8 wrote it. That is the point of the gap.

#[cfg(test)]
mod tests {
    use prax_core::engine::{GroundedAction, State};
    use prax_script::compile::{compile, current_scene_of};
    use prax_script::json::decode_script;
    use prax_script::script::Script;

    /// The path the script's scene entry arms and its beat bare-inserts.
    const LANTERN: &str = "lantern.lit";
    /// The lifetime the script's `insertFor` authors.
    const LIFETIME: i64 = 3;

    /// The ONE committed script both engines are driven by. Embedded rather than
    /// read at runtime so the test needs no working-directory assumption — the
    /// `cargo test` cwd is the crate directory, not the repo root [M-4].
    const CG1_JSON: &str = include_str!("../../../conformance/fixtures/cg1_supersession.json");

    fn cg1_script() -> Script {
        decode_script(CG1_JSON.as_bytes()).expect("the committed CG-1 script decodes")
    }

    fn cg1_world() -> State {
        compile(&cg1_script()).expect("the committed CG-1 script compiles")
    }

    /// The ending reached, if any — the base-db `ending.E` probe.
    fn ending_of(st: &mut State) -> Option<String> {
        st.db_child_keys("ending").into_iter().next()
    }

    fn shield(st: &mut State) -> GroundedAction {
        st.possible_actions("q")
            .into_iter()
            .find(|ga| ga.label.contains("shield"))
            .expect("q can shield the lantern")
    }

    /// The premise the design denied: `compile` ACCEPTS a script that arms its
    /// own timer in a scene setup and bare-inserts the same path from a beat
    /// effect. Neither outcome is headed by a reserved family, so the five-list
    /// sweep never sees them.
    ///
    /// REDDENS UNDER: extending the reserved-family guard to cover any authored
    /// `InsertFor` path (which would also reject the world outright).
    #[test]
    fn a_script_can_author_a_timer_and_a_bare_insert_onto_the_same_path() {
        let scr = cg1_script();
        // the AUTHORED timer, in the scene's setup — not a compiler-armed
        // patience marker
        assert_eq!(
            scr.scenes[0].setup,
            vec![prax_core::types::Outcome::InsertFor(
                LIFETIME,
                LANTERN.to_owned()
            )],
            "the scene setup arms its own timer"
        );
        // the AUTHORED bare insert of the same path, in a beat effect
        assert!(
            scr.scenes[0].beats[0]
                .effects
                .contains(&prax_core::types::insert(LANTERN)),
            "the beat bare-inserts the very path the setup armed, got {:?}",
            scr.scenes[0].beats[0].effects
        );

        let mut st = cg1_world();
        assert_eq!(current_scene_of(&mut st).as_deref(), Some("vigil"));
        assert_eq!(
            st.expiries_rendered().get(LANTERN).copied(),
            Some(LIFETIME),
            "compile ran the authored InsertFor and the timer is live at t=0"
        );
    }

    /// The CONTROL: nobody shields the lantern, so the armed timer fires on
    /// schedule, the fact goes, and the junction that reads its absence ends the
    /// story. Without this half, the pin below would pass just as happily
    /// against an engine that never expires anything at all.
    ///
    /// REDDENS UNDER: an engine that stops firing due expiries at the boundary.
    #[test]
    fn the_authored_timer_fires_on_schedule_when_nothing_touches_it() {
        let mut st = cg1_world();
        for b in 1..LIFETIME {
            st.round_boundary();
            assert!(
                st.db_has(LANTERN),
                "the lantern still burns at boundary {b}"
            );
            assert_eq!(ending_of(&mut st), None, "and the story runs on");
        }
        st.round_boundary();
        assert!(
            !st.db_has(LANTERN),
            "the authored timer expired at boundary {LIFETIME}"
        );
        assert_eq!(
            ending_of(&mut st).as_deref(),
            Some("darkness"),
            "and the junction reading its absence ended the story"
        );
    }

    /// **CG-1's supersession half, from authored data, at world scale.** The
    /// beat's bare insert onto the live timer's exact path CANCELS the pending
    /// expiry: the lantern never goes out, and the ending the control reaches at
    /// boundary 3 never comes — driven eight boundaries, well past twice the
    /// authored lifetime.
    ///
    /// REDDENS UNDER: deleting `rt.expiries.remove(path)` from
    /// `prax_core::engine`'s `Effect::Insert` arm — verified, and the ONLY REDs
    /// in the workspace under that deletion are the law's own pins.
    #[test]
    fn a_beats_bare_insert_supersedes_the_scripts_own_authored_timer() {
        let mut st = cg1_world();
        assert!(
            st.expiries_rendered().contains_key(LANTERN),
            "precondition: the authored timer is live before the beat"
        );

        let ga = shield(&mut st);
        st.perform_action(&ga);

        assert!(
            !st.expiries_rendered().contains_key(LANTERN),
            "the beat's BARE insert superseded the pending expiry: the due is gone"
        );
        for _ in 0..8 {
            st.round_boundary();
        }
        assert!(
            st.db_has(LANTERN),
            "the superseded fact still stands eight boundaries later -- more \
             than twice the lifetime the script armed it with"
        );
        assert_eq!(
            ending_of(&mut st),
            None,
            "and the ending the CONTROL reaches at boundary 3 never comes: a \
             supersession bug reachable only through AUTHORING now has a net"
        );
    }

    /// The trap, pinned beside the law so the two cannot be confused. A
    /// re-`InsertFor` of the same path goes through the SAME `Effect::Insert`
    /// branch — and so through the same `expiries.remove` — and then RE-ARMS. It
    /// is a refresh; the fact still dies. A naive counter that reports every
    /// `expiries.remove` as a supersession would report this one too.
    ///
    /// This is also the REFRESH half's authored, world-scale exercise: it is the
    /// same authored `InsertFor` the script's scene entry uses, replayed.
    ///
    /// REDDENS UNDER: making `InsertFor` skip the removal (the timer would then
    /// keep its ORIGINAL due instead of the refreshed one).
    #[test]
    fn re_arming_the_authored_timer_refreshes_rather_than_supersedes() {
        let mut st = cg1_world();
        st.round_boundary();
        st.round_boundary();
        let refreshed_due = st.current_turn() + LIFETIME;
        st.perform_outcome(&prax_core::types::Outcome::InsertFor(
            LIFETIME,
            LANTERN.to_owned(),
        ))
        .expect("re-arm the script's own authored timer");
        assert_eq!(
            st.expiries_rendered().get(LANTERN).copied(),
            Some(refreshed_due),
            "the timer is RE-ARMED at the new due, not cancelled"
        );
        for _ in 0..LIFETIME {
            st.round_boundary();
        }
        assert!(
            !st.db_has(LANTERN),
            "a refreshed lantern still goes out -- only a BARE insert supersedes"
        );
    }

    /// The REFRESH half's other authored route, which the S8 design correctly
    /// identified: scene RE-ENTRY re-arms every patience marker through the
    /// compiler's scene-entry fold, reached by an authored `goto`. It is the
    /// minor half of CG-1's story, and it is pinned by the frozen-labelled
    /// re-entry case in `conformance::script_spec`; this pin records the
    /// connection rather than duplicating the drive.
    #[test]
    fn the_scripts_own_timer_is_the_one_the_law_is_about() {
        let st = cg1_world();
        // there is exactly ONE timer in this world and it is AUTHORED: the
        // script declares no timed junction, so no compiler-armed patience
        // marker exists to confuse the measurement.
        assert_eq!(
            st.expiries_rendered().keys().collect::<Vec<_>>(),
            vec![LANTERN],
            "the only live expiry is the one the author wrote"
        );
        assert!(
            cg1_script().scenes[0].junctions.iter().all(|j| j.after.is_none()),
            "no timed junction, so no scenePatience marker anywhere"
        );
    }
}
