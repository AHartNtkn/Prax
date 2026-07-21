//! `Prax.ScriptSpec`, re-expressed against the Rust engine: the scene layer's
//! behaviour end to end — the compiled start state, beat scoping, the silent
//! transition at a round boundary, the four story-order laws, the eight compile
//! guards, the v50 patience-marker timing, the character sketch, and both
//! script worlds' stories.
//!
//! One label of the frozen file is NOT here: the mid-scene save/resume
//! persistence-symmetry case, whose subject is `Prax.Persist` (S9). It carries
//! an `owed: S9` row in `conformance/KILLED.md`; the TIMING behaviour it rides
//! on is pinned here without persistence by the boundary-5 fidelity pin below.

#[cfg(test)]
mod tests {
    // H: ScriptSpec.hs "Prax.Script"
    //
    // The frozen `Prax.ScriptSpec` group.
    use prax_core::engine::{GroundedAction, State};
    use prax_core::query::{CmpOp, Condition, cmp, matches, not_};
    use prax_core::turn::{advance, npc_act};
    use prax_core::types::{Outcome, Want, delete, insert};
    use prax_script::compile::{compile, current_scene_of, flow_chart};
    use prax_script::script::{
        Script, after, beat, concerned_with, ending, goto, member, player, quip, scene, timeout,
        wanting, with_traits,
    };
    use prax_vocab::core_model::adjust_score;
    use prax_worlds::audience::audience_world;
    use prax_worlds::play::{play_script, play_world};

    /// The ending reached, if any (the frozen `endingOf`).
    fn ending_of(st: &mut State) -> Option<String> {
        st.db_child_keys("ending").into_iter().next()
    }

    /// One of a character's currently-available actions whose label mentions
    /// `needle` (the frozen `actionMatching`).
    fn action_matching(st: &mut State, who: &str, needle: &str) -> GroundedAction {
        let acts = st.possible_actions(who);
        acts.iter()
            .find(|ga| ga.label.contains(needle))
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "no action for {who} matching {needle:?}; had: {:?}",
                    acts.iter().map(|g| &g.label).collect::<Vec<_>>()
                )
            })
    }

    fn act(st: &mut State, who: &str, needle: &str) {
        let ga = action_matching(st, who, needle);
        st.perform_action(&ga);
    }

    /// Run the simulation with `idle` (the player) never acting and everyone
    /// else driven by the planner, until an ending, until `target` becomes the
    /// current scene, or for `k` advances — whichever comes first (the frozen
    /// `driveIdle`).
    fn drive_idle(st: &mut State, target: Option<&str>, idle: &str, k: i32) {
        for _ in 0..k {
            if ending_of(st).is_some() {
                return;
            }
            if current_scene_of(st).as_deref() == target {
                return;
            }
            let actor = advance(st);
            if actor.name != idle {
                npc_act(st, 2, &actor);
            }
        }
    }

    /// The state driven to the banquet with Marcus idle.
    fn at_banquet() -> State {
        let mut st = play_world();
        drive_idle(&mut st, Some("banquet"), "marcus", 20);
        st
    }

    // H: ScriptSpec.hs "compile: the start scene is active and the cast is present"
    #[test]
    fn compile_the_start_scene_is_active_and_the_cast_is_present() {
        let mut st = play_world();
        assert_eq!(current_scene_of(&mut st).as_deref(), Some("confidence"));
        assert!(st.db_has("character.marcus"), "marcus present");
        assert!(st.db_has("character.cassia"), "cassia present");
        // v46: there is no story manager; the cast is only real characters
        assert!(
            !st.characters().iter().any(|c| c.name == "_narrator"),
            "no _narrator in the cast"
        );
    }

    // H: ScriptSpec.hs "a beat fires only in its scene and applies its effects"
    #[test]
    fn a_beat_fires_only_in_its_scene_and_applies_its_effects() {
        let mut st = play_world();
        act(&mut st, "cassia", "confide");
        assert!(st.db_has("confided"), "confided asserted");
        assert!(st.db_has("marcusKnows"), "marcus now knows");
        // the banquet's poison beat is NOT available while `confidence` is current
        assert!(
            !st.possible_actions("cassia")
                .iter()
                .any(|ga| ga.label.contains("poison")),
            "no banquet beats yet"
        );
    }

    // H: ScriptSpec.hs "a transition junction fires silently at the round boundary (no story manager)"
    #[test]
    fn a_transition_junction_fires_silently_at_the_round_boundary() {
        let mut st = play_world();
        act(&mut st, "cassia", "confide"); // confided holds
        st.round_boundary(); // the engine fires the story rule
        assert_eq!(
            current_scene_of(&mut st).as_deref(),
            Some("banquet"),
            "the scene advanced and no actor took a turn"
        );
    }

    // H: ScriptSpec.hs "idle player: the plot runs to betrayal across two scenes"
    #[test]
    fn idle_player_the_plot_runs_to_betrayal_across_two_scenes() {
        let mut st = play_world();
        drive_idle(&mut st, None, "marcus", 30);
        assert_eq!(ending_of(&mut st).as_deref(), Some("betrayal"));
        assert_eq!(
            current_scene_of(&mut st).as_deref(),
            Some("banquet"),
            "the transition happened"
        );
    }

    // H: ScriptSpec.hs "the player can warn: ending loyalty"
    #[test]
    fn the_player_can_warn_ending_loyalty() {
        let mut st = at_banquet();
        assert_eq!(current_scene_of(&mut st).as_deref(), Some("banquet"));
        act(&mut st, "marcus", "warn");
        drive_idle(&mut st, None, "marcus", 20);
        assert_eq!(ending_of(&mut st).as_deref(), Some("loyalty"));
    }

    // H: ScriptSpec.hs "the player can strike first: ending complicity"
    #[test]
    fn the_player_can_strike_first_ending_complicity() {
        let mut st = at_banquet();
        act(&mut st, "marcus", "own hand");
        drive_idle(&mut st, None, "marcus", 20);
        assert_eq!(ending_of(&mut st).as_deref(), Some("complicity"));
    }

    // H: ScriptSpec.hs "the player can romance the conspirator (as in Intrigue)"
    #[test]
    fn the_player_can_romance_the_conspirator() {
        let mut st = at_banquet();
        act(&mut st, "marcus", "charms");
        assert!(
            st.db_has("bond.marcus.cassia.lovers"),
            "Marcus and Cassia become lovers, got {:?}",
            st.labeled_facts()
        );
    }

    // H: ScriptSpec.hs "flowChart names every scene and junction"
    #[test]
    fn flow_chart_names_every_scene_and_junction() {
        let chart = flow_chart(&play_script());
        for needle in [
            "confidence",
            "banquet",
            "toBanquet",
            "betrayal",
            "loyalty",
            "complicity",
            "graph TD",
        ] {
            assert!(chart.contains(needle), "{needle} in chart, got:\n{chart}");
        }
    }

    // ---- Prompter compilation features -------------------------------------

    // H: ScriptSpec.hs "a timed junction times out after N turns of inaction"
    #[test]
    fn a_timed_junction_times_out_after_n_turns_of_inaction() {
        let scr = Script::new("wait")
            .cast([player("p")])
            .scenes([scene("wait").junctions([timeout("gaveUp", 3)])]);
        let mut st = compile(&scr).expect("compiles");
        drive_idle(&mut st, None, "p", 30); // nobody acts; the clock runs out
        assert_eq!(ending_of(&mut st).as_deref(), Some("gaveUp"));
    }

    // ---- the story law: one boundary, the existing schedule machinery (v46) --
    //
    // Every clause carries its own gates; the executor threads state between
    // clauses, so eviction and Absent-ending decide firing, not any mode. These
    // four are the ONLY frozen pins that see the compiled clause ORDER (O2).

    // H: ScriptSpec.hs "same-scene co-enabled junctions: first in authored order fires, the eviction masks the second"
    #[test]
    fn same_scene_co_enabled_junctions_first_in_authored_order_fires() {
        // Both routes out of `s` are enabled at once; `toX` is authored first,
        // so it fires and its currentScene eviction masks `toY` in the same
        // boundary.
        let scr = Script::new("s").cast([player("p")]).scenes([
            scene("s").junctions([
                goto("toX", "x", vec![matches("go")]),
                goto("toY", "y", vec![matches("go")]),
            ]),
            scene("x"),
            scene("y"),
        ]);
        let mut st = compile(&scr).expect("compiles");
        st.perform_outcome(&insert("go")).expect("go");
        st.round_boundary();
        assert_eq!(
            current_scene_of(&mut st).as_deref(),
            Some("x"),
            "not \"y\": authored order + eviction"
        );
    }

    // H: ScriptSpec.hs "authored order, not alphabetical label order, resolves a simultaneous enable"
    #[test]
    fn authored_order_not_alphabetical_resolves_a_simultaneous_enable() {
        // `zzz` fires before `aaa` though it sorts later — the old tiebreak was
        // an accident of the planner's alphabetical sort; authored order is a
        // statement. Alphabetical order would land on "y".
        let scr = Script::new("s").cast([player("p")]).scenes([
            scene("s").junctions([
                goto("zzz", "x", vec![matches("go")]),
                goto("aaa", "y", vec![matches("go")]),
            ]),
            scene("x"),
            scene("y"),
        ]);
        let mut st = compile(&scr).expect("compiles");
        st.perform_outcome(&insert("go")).expect("go");
        st.round_boundary();
        assert_eq!(current_scene_of(&mut st).as_deref(), Some("x"));
    }

    // H: ScriptSpec.hs "an ending masks every later clause in the same boundary"
    #[test]
    fn an_ending_masks_every_later_clause_in_the_same_boundary() {
        // `endHere` fires (an ending) before `toY`; the `ending.E` fact masks
        // the transition's Absent-ending gate, so no scene change slips through.
        let scr = Script::new("s").cast([player("p")]).scenes([
            scene("s").junctions([
                ending("endHere", vec![matches("go")]),
                goto("toY", "y", vec![matches("go")]),
            ]),
            scene("y"),
        ]);
        let mut st = compile(&scr).expect("compiles");
        st.perform_outcome(&insert("go")).expect("go");
        st.round_boundary();
        assert_eq!(ending_of(&mut st).as_deref(), Some("endHere"));
        assert_eq!(
            current_scene_of(&mut st).as_deref(),
            Some("s"),
            "the transition was masked"
        );
    }

    // H: ScriptSpec.hs "cross-scene cascade: a pass-through scene traverses in one boundary (documented eager semantics)"
    #[test]
    fn cross_scene_cascade_traverses_a_pass_through_scene_in_one_boundary() {
        // Scene `b`'s exit condition already holds on entry (both gates read
        // `go`), so a->b->c happens within one boundary: `toB` fires, then `toC`
        // sees the fresh `currentScene!b` and fires too. The gates decide, not a
        // mode.
        let scr = Script::new("a").cast([player("p")]).scenes([
            scene("a").junctions([goto("toB", "b", vec![matches("go")])]),
            scene("b").junctions([goto("toC", "c", vec![matches("go")])]),
            scene("c"),
        ]);
        let mut st = compile(&scr).expect("compiles");
        st.perform_outcome(&insert("go")).expect("go");
        st.round_boundary();
        assert_eq!(
            current_scene_of(&mut st).as_deref(),
            Some("c"),
            "traversed a->b->c in one boundary"
        );
    }

    // ---- timed-junction / patience-marker compile guards (spec v50 T2) -------
    //
    // The frozen suite asserts only `isLeft` on each of these eight. The Rust
    // errors are typed, so each case asserts the VARIANT — strictly more than
    // the frozen label does, and the guard ORDER between them carries its own
    // native pin in `prax_script::compile`.

    // H: ScriptSpec.hs "compile rejects two junctions with the same name in one scene"
    #[test]
    fn compile_rejects_two_junctions_with_the_same_name_in_one_scene() {
        let scr = Script::new("a").cast([player("p")]).scenes([scene("a").junctions([
            ending("dup", vec![matches("x")]),
            ending("dup", vec![matches("y")]),
        ])]);
        assert!(matches!(
            compile(&scr).err(),
            Some(prax_core::error::WorldError::DuplicateJunctionName { .. })
        ));
    }

    // H: ScriptSpec.hs "compile rejects a timeout with a zero delay (n=0 is a plain junction)"
    #[test]
    fn compile_rejects_a_timeout_with_a_zero_delay() {
        let scr = Script::new("a")
            .cast([player("p")])
            .scenes([scene("a").junctions([timeout("now", 0)])]);
        assert!(matches!(
            compile(&scr).err(),
            Some(prax_core::error::WorldError::ZeroDelayJunction { .. })
        ));
    }

    // H: ScriptSpec.hs "compile rejects an 'after' goto with a zero delay"
    #[test]
    fn compile_rejects_an_after_goto_with_a_zero_delay() {
        let scr = Script::new("a").cast([player("p")]).scenes([
            scene("a").junctions([after("toB", 0, "b")]),
            scene("b"),
        ]);
        assert!(matches!(
            compile(&scr).err(),
            Some(prax_core::error::WorldError::ZeroDelayJunction { .. })
        ));
    }

    // H: ScriptSpec.hs "compile rejects an authored scenePatience read in a junction condition"
    #[test]
    fn compile_rejects_an_authored_scene_patience_read_in_a_junction_condition() {
        let scr = Script::new("a")
            .cast([player("p")])
            .scenes([scene("a")
                .junctions([ending("e", vec![matches("scenePatience.a.foo")])])]);
        assert!(matches!(
            compile(&scr).err(),
            Some(prax_core::error::WorldError::ReservedFamilyAuthored { .. })
        ));
    }

    // H: ScriptSpec.hs "compile rejects an authored scenePatience write in a scene setup"
    #[test]
    fn compile_rejects_an_authored_scene_patience_write_in_a_scene_setup() {
        let scr = Script::new("a")
            .cast([player("p")])
            .scenes([scene("a").setup([insert("scenePatience.a.foo")])]);
        assert!(matches!(
            compile(&scr).err(),
            Some(prax_core::error::WorldError::ReservedFamilyAuthored { .. })
        ));
    }

    // H: ScriptSpec.hs "compile rejects an authored scenePatience touch in a beat effect (a newly-swept list)"
    #[test]
    fn compile_rejects_an_authored_scene_patience_touch_in_a_beat_effect() {
        let scr = Script::new("a").cast([player("p")]).scenes([
            scene("a").beats([beat("meddle", vec![], vec![delete("scenePatience.a.foo")])]),
        ]);
        assert!(matches!(
            compile(&scr).err(),
            Some(prax_core::error::WorldError::ReservedFamilyAuthored { .. })
        ));
    }

    // H: ScriptSpec.hs "compile rejects an authored scenePatience touch in a beat condition (a newly-swept list)"
    #[test]
    fn compile_rejects_an_authored_scene_patience_touch_in_a_beat_condition() {
        let scr = Script::new("a").cast([player("p")]).scenes([
            scene("a").beats([beat("peek", vec![matches("scenePatience.a.foo")], vec![])]),
        ]);
        assert!(matches!(
            compile(&scr).err(),
            Some(prax_core::error::WorldError::ReservedFamilyAuthored { .. })
        ));
    }

    // H: ScriptSpec.hs "compile rejects an authored scenePatience touch in a cast-desire condition (a newly-swept list, absence polarity)"
    #[test]
    fn compile_rejects_an_authored_scene_patience_touch_in_a_cast_desire() {
        let scr = Script::new("a")
            .cast([
                player("p"),
                wanting(
                    member("d"),
                    [Want::new(vec![not_("scenePatience.a.foo")], 5)],
                ),
            ])
            .scenes([scene("a")]);
        assert!(matches!(
            compile(&scr).err(),
            Some(prax_core::error::WorldError::ReservedFamilyAuthored { .. })
        ));
    }

    // ---- v50: timed junctions ride patience markers -------------------------
    //
    // Each behaviour is driven boundary-by-boundary (the pure timing harness:
    // nobody acts, the markers just expire on the clock).

    // H: ScriptSpec.hs "the audience: 'dismissed' fires at boundary 5, not before (the fidelity pin)"
    #[test]
    fn the_audience_dismissed_fires_at_boundary_5_not_before() {
        // Entry at boundary 0, timeout 5, so the patience runs out exactly at
        // boundary 5. Exercises the START-scene entry path: marker emission must
        // ride scene entry.
        let mut st = audience_world();
        for _ in 0..4 {
            st.round_boundary();
        }
        assert_eq!(ending_of(&mut st), None, "patience still holds at boundary 4");
        st.round_boundary();
        assert_eq!(
            ending_of(&mut st).as_deref(),
            Some("dismissed"),
            "boundary 5: it ran out"
        );
    }

    // The owed:S9 ScriptSpec persistence-symmetry pin (KILLED row 15). The
    // patience marker is an ordinary fact; its PENDING expiry rides v44's due
    // serialization ([`prax_core::persist`], S9), so a save partway through a
    // timed scene resumes to the SAME dismissal boundary with no Persist code of
    // its own. This is the claim that proves `expiries` round-trip is complete
    // for a script world — it needs the persist machinery, so it lands with S9.
    //
    // H: ScriptSpec.hs "mid-scene save/resume reaches the same timeout boundary (persistence symmetry)"
    #[test]
    fn mid_scene_save_resume_reaches_the_same_timeout_boundary() {
        use prax_core::persist::{deserialize_state, serialize_state};
        // Boundary 2: mid-scene, the timed junction's patience marker armed and
        // its expiry PENDING.
        let mut mid = audience_world();
        mid.round_boundary();
        mid.round_boundary();
        assert_eq!(ending_of(&mut mid), None, "no ending at boundary 2");

        // Save and reload onto a fresh audience world.
        let mut resumed =
            deserialize_state(&serialize_state(&mid), audience_world()).expect("mid-scene round trip");
        resumed.round_boundary();
        resumed.round_boundary();
        assert_eq!(
            ending_of(&mut resumed),
            None,
            "still holds at absolute boundary 4 after reload"
        );
        resumed.round_boundary();
        assert_eq!(
            ending_of(&mut resumed).as_deref(),
            Some("dismissed"),
            "reaches dismissed at the SAME absolute boundary 5 (the pending \
             patience expiry rode the save)"
        );
    }

    // H: ScriptSpec.hs "a timed 'after' goto fires at its delay boundary"
    #[test]
    fn a_timed_after_goto_fires_at_its_delay_boundary() {
        let scr = Script::new("a").cast([player("p")]).scenes([
            scene("a").junctions([after("toB", 3, "b")]),
            scene("b"),
        ]);
        let mut st = compile(&scr).expect("compiles");
        st.round_boundary();
        st.round_boundary();
        assert_eq!(current_scene_of(&mut st).as_deref(), Some("a"), "not yet");
        st.round_boundary();
        assert_eq!(
            current_scene_of(&mut st).as_deref(),
            Some("b"),
            "boundary 3: hands off"
        );
    }

    // H: ScriptSpec.hs "re-entry resets a timed junction: it times out n from the LAST entry"
    #[test]
    fn re_entry_resets_a_timed_junction() {
        let scr = Script::new("a").cast([player("p")]).scenes([
            scene("a").junctions([
                goto("leave", "b", vec![matches("leaveNow")]),
                timeout("out", 3),
            ]),
            scene("b").junctions([goto("back", "a", vec![matches("backNow")])]),
        ]);
        let mut st = compile(&scr).expect("compiles");
        st.round_boundary(); // dawdle at a
        st.perform_outcome(&insert("leaveNow")).expect("leaveNow");
        st.round_boundary(); // leave before the timeout
        assert_eq!(current_scene_of(&mut st).as_deref(), Some("b"));
        st.perform_outcome(&delete("leaveNow")).expect("clear");
        st.perform_outcome(&insert("backNow")).expect("backNow");
        st.round_boundary(); // re-enter a
        assert_eq!(current_scene_of(&mut st).as_deref(), Some("a"));
        // from the LAST entry the clock runs a fresh 3 boundaries, not from the
        // first
        st.round_boundary();
        st.round_boundary();
        assert_eq!(ending_of(&mut st), None);
        st.round_boundary();
        assert_eq!(ending_of(&mut st).as_deref(), Some("out"));
    }

    // H: ScriptSpec.hs "early exit is harmless: a pending patience marker fires no stray junction"
    #[test]
    fn early_exit_is_harmless_a_pending_patience_marker_fires_no_stray_junction() {
        let scr = Script::new("a").cast([player("p")]).scenes([
            scene("a").junctions([
                goto("leave", "b", vec![matches("leaveNow")]),
                timeout("out", 3),
            ]),
            scene("b"),
        ]);
        let mut st = compile(&scr).expect("compiles");
        st.round_boundary(); // dawdle at a (marker mid-life)
        st.perform_outcome(&insert("leaveNow")).expect("leaveNow");
        st.round_boundary(); // leave before the timeout
        assert_eq!(current_scene_of(&mut st).as_deref(), Some("b"));
        // drive past the original due (boundary 3): the stale marker expires in
        // `b` with the currentScene gate false, so `out` never fires
        for _ in 0..5 {
            st.round_boundary();
        }
        assert_eq!(ending_of(&mut st), None);
    }

    // H: ScriptSpec.hs "two timed junctions on one scene fire independently at their own delays"
    #[test]
    fn two_timed_junctions_on_one_scene_fire_independently() {
        // distinct names (same-name is rejected) and distinct delays; the
        // shorter fires at ITS boundary, proving the markers are keyed
        // separately — a collapsed single marker would delay `quick` to `slow`'s
        // boundary.
        let scr = Script::new("a").cast([player("p")]).scenes([
            scene("a").junctions([after("quick", 2, "b"), after("slow", 4, "c")]),
            scene("b"),
            scene("c"),
        ]);
        let mut st = compile(&scr).expect("compiles");
        st.round_boundary();
        assert_eq!(
            current_scene_of(&mut st).as_deref(),
            Some("a"),
            "boundary 1: neither yet"
        );
        st.round_boundary();
        assert_eq!(
            current_scene_of(&mut st).as_deref(),
            Some("b"),
            "boundary 2: `quick` (delay 2) fired"
        );
    }

    // H: ScriptSpec.hs "a concern actually drives behaviour"
    #[test]
    fn a_concern_actually_drives_behaviour() {
        let scr = Script::new("s")
            .cast([
                player("p"),
                concerned_with(member("vain"), [("beauty", 50)]),
            ])
            .scenes([scene("s").beats([quip(
                "vain",
                "[Actor]: preen for p",
                vec![not_("preened")],
                vec![
                    insert("preened"),
                    adjust_score("p", "vain", "beauty", 5, "dazzled"),
                ],
            )])]);
        let mut st = compile(&scr).expect("compiles");
        drive_idle(&mut st, None, "p", 12);
        assert!(
            st.db_has("preened"),
            "trait/desire wiring lets vain be moved by regard: it preens"
        );
    }

    // H: ScriptSpec.hs "same-labelled quips by different speakers dispatch to their own effects"
    #[test]
    fn same_labelled_quips_by_different_speakers_dispatch_to_their_own_effects() {
        // Two quips share the display text "[Actor]: act"; performing b's must
        // run b's outcome, not a's. (A quip is a specific speaker's action, so
        // the compiled action id must distinguish the speakers.)
        let scr = Script::new("s")
            .cast([member("a"), member("b")])
            .scenes([scene("s").beats([
                quip("a", "[Actor]: act", vec![], vec![insert("aDid")]),
                quip("b", "[Actor]: act", vec![], vec![insert("bDid")]),
            ])]);
        let mut st = compile(&scr).expect("compiles");
        act(&mut st, "b", "act");
        assert!(st.db_has("bDid"), "b's beat set b's fact");
        assert!(!st.db_has("aDid"), "b's beat did NOT run a's beat");
    }

    // ---- the audience: one story that uses all three features together -------

    // H: ScriptSpec.hs "the audience: dawdling lets the clock run out to 'dismissed'"
    #[test]
    fn the_audience_dawdling_lets_the_clock_run_out_to_dismissed() {
        let mut st = audience_world();
        drive_idle(&mut st, None, "envoy", 40);
        assert_eq!(ending_of(&mut st).as_deref(), Some("dismissed"));
    }

    // H: ScriptSpec.hs "the audience: flatter then petition reaches 'granted'"
    #[test]
    fn the_audience_flatter_then_petition_reaches_granted() {
        let mut st = audience_world();
        act(&mut st, "envoy", "flatter");
        act(&mut st, "envoy", "petition");
        // enough round boundaries for the story rule to fire the ending;
        // `granted` (petitioned holds) lands well before the timeout clock could
        // reach `dismissed`.
        drive_idle(&mut st, None, "envoy", 15);
        assert_eq!(ending_of(&mut st).as_deref(), Some("granted"));
    }

    // H: ScriptSpec.hs "the audience: the Duke's concern moves him (once)"
    #[test]
    fn the_audience_the_dukes_concern_moves_him_once() {
        let mut st = audience_world();
        drive_idle(&mut st, None, "envoy", 6);
        assert!(
            st.db_has("dukeSpoke"),
            "the Duke, concerned for favour, flattered unbidden"
        );
        assert!(
            !st.possible_actions("duke")
                .iter()
                .any(|ga| ga.label.contains("flatter")),
            "and the one-shot held: no flatter left on the Duke's menu"
        );
    }

    /// The character-sketch label's data half lives at its home,
    /// `prax_script::script`'s own tests, where the combinators are — this
    /// comment records where the reader should look, since the label is
    /// accounted for there and not here.
    ///
    /// (`// H: ScriptSpec.hs "a character sketch compiles concerns to wants and
    /// traits to facts"` — see `prax_script::script::tests`.)
    #[test]
    fn the_sketch_reaches_the_compiled_world_as_facts_and_wants() {
        // NATIVE PIN, no frozen label: the frozen case asserts the sketch's
        // AST-level output only. This asserts the sketch actually LANDS in a
        // compiled world — the trait as a queryable fact, the concern as one of
        // the character's wants — which is the half the frozen case leaves to
        // the "a concern actually drives behaviour" case's end-to-end drive.
        //
        // REDDENS UNDER: dropping the trait inserts from `compile`'s setup fold,
        // or failing to copy `CastMember::desires` onto the `Character`.
        let scr = Script::new("s")
            .cast([with_traits(
                concerned_with(member("vain"), [("beauty", 50)]),
                ["proud"],
            )])
            .scenes([scene("s")]);
        let mut st = compile(&scr).expect("compiles");
        assert!(st.db_has("trait.vain.proud"), "the trait is a queryable fact");
        let vain = st
            .characters()
            .iter()
            .find(|c| c.name == "vain")
            .expect("vain is in the cast");
        assert_eq!(
            vain.wants,
            vec![Want::new(
                vec![
                    Condition::Match("Other.relationship.vain.beauty.score!N".to_owned()),
                    Condition::Neq("Other".to_owned(), "vain".to_owned()),
                    cmp(CmpOp::Gt, "N", "0"),
                ],
                50
            )]
        );
    }

    /// NATIVE PIN — the audience's non-empty INITIAL expiry map. `audience` is
    /// the first shipped world whose state carries a pending due at t=0: the
    /// compile-time setup fold arms `timeout "dismissed" 5`'s patience marker
    /// before any turn runs.
    ///
    /// This is the `worldshape` field the timing contract actually rides on.
    /// `setup_db` sees the marker FACT but not its DUE — the due lives in the
    /// runtime expiry map, which `shape.state` emits verbatim — so an engine
    /// that armed the marker with the wrong lifetime would present an identical
    /// `setup_db` and a different `state.expiries`.
    ///
    /// REDDENS UNDER: arming the marker with any lifetime other than the
    /// authored 5, or arming it at first boundary instead of at scene entry.
    #[test]
    fn the_audience_carries_a_pending_expiry_before_any_turn_runs() {
        let st = audience_world();
        assert_eq!(
            st.expiries_rendered()
                .into_iter()
                .collect::<Vec<(String, i64)>>(),
            vec![("scenePatience.audience.dismissed".to_owned(), 5)],
            "the start scene's timed junction armed its marker during compile"
        );
    }

    /// NATIVE PIN — the compiled action-label vector of the two SHIPPED script
    /// worlds. `prax_script::compile`'s O1 pin uses a synthetic probe; this one
    /// holds the real content, because the label vector is what the planner's
    /// full-tie fallback and randtrace's `pick` index run over, and its only
    /// other net (`worldshape.shape.action_labels`) dies at cut-over.
    ///
    /// REDDENS UNDER: any re-ordering or re-wording of a beat in either world,
    /// or a `bake_actor` change that stops baking the speaker into the id.
    #[test]
    fn the_shipped_script_worlds_compile_to_these_exact_action_labels() {
        let labels = |st: &State| -> Vec<String> {
            st.practice_defs()["beats"]
                .actions
                .iter()
                .map(|a| a.name.clone())
                .collect()
        };
        assert_eq!(
            labels(&play_world()),
            [
                "cassia: confide the plot against artus to marcus",
                "cassia: slip poison into artus's cup",
                "marcus: warn artus that cassia means to kill him",
                "marcus: poison artus with your own hand",
                "marcus: warm to cassia's charms",
            ]
        );
        assert_eq!(
            labels(&audience_world()),
            [
                "envoy: flatter the king",
                "envoy: present your petition",
                "duke: flatter the king",
            ]
        );
    }

    /// NATIVE PIN — the two-door COLLISION against a compiled script world, the
    /// discharge of `KILLED.md`'s owed:S8 row. Its home is
    /// `conformance::schedule_rule_spec`, where the frozen label lives; this
    /// comment records that the row is discharged and where.
    ///
    /// (`// H: ScheduleRuleSpec.hs "adding an authored 'story' rule to a
    /// compiled script world is a loud collision"` — see
    /// `conformance::schedule_rule_spec`.)
    #[test]
    fn the_compiled_world_holds_exactly_one_engine_rule_named_story() {
        let st = play_world();
        assert_eq!(st.engine_rule_names(), ["story"]);
        assert_eq!(
            st.schedule_src().iter().map(|r| &r.name).collect::<Vec<_>>(),
            vec!["story"]
        );
    }

    /// NATIVE PIN — a compiled script world's ONE outcome shape that the frozen
    /// timing pins never look at directly: the story rule's clause bodies for
    /// the shipped `audience`, including the patience `Not` guard the timeout
    /// junction carries and the `InsertFor` no transition re-arms (the audience
    /// has one scene, so nothing re-enters).
    ///
    /// REDDENS UNDER: emitting the patience guard for an UNTIMED junction, or
    /// omitting it for a timed one.
    #[test]
    fn the_audiences_story_clauses_carry_the_patience_guard_only_where_timed() {
        let st = audience_world();
        let body = &st.schedule_src()[0].body;
        assert_eq!(body.len(), 2, "one clause per junction");
        // `granted` is untimed: two standing gates plus the author's own `when`
        assert_eq!(
            body[0].0,
            vec![
                matches("currentScene!audience"),
                Condition::Absent(vec![matches("ending.E")]),
                matches("petitioned"),
            ]
        );
        assert_eq!(body[0].1, vec![insert("ending!granted")]);
        // `dismissed` is timed: no authored `when` at all, and the patience
        // `Not` the compiler expanded from `after: 5`
        assert_eq!(
            body[1].0,
            vec![
                matches("currentScene!audience"),
                Condition::Absent(vec![matches("ending.E")]),
                not_("scenePatience.audience.dismissed"),
            ]
        );
        assert_eq!(body[1].1, vec![insert("ending!dismissed")]);
    }

    /// NATIVE PIN — the scene-entry `InsertFor` a TRANSITION carries, which the
    /// audience (one scene, no transitions) cannot show. `play`'s transition has
    /// no timed destination, so this uses a two-scene probe: the destination's
    /// marker is re-armed by the transition clause itself, which is what makes
    /// re-entry reset the clock.
    ///
    /// REDDENS UNDER: emitting `setup_of` only at compile time and not in the
    /// transition body (re-entry would then never re-arm).
    #[test]
    fn a_transition_clause_re_arms_the_destinations_patience_marker() {
        let scr = Script::new("a").cast([player("p")]).scenes([
            scene("a").junctions([goto("toB", "b", vec![matches("go")])]),
            scene("b").junctions([timeout("out", 4)]),
        ]);
        let st = compile(&scr).expect("compiles");
        assert_eq!(
            st.schedule_src()[0].body[0].1,
            vec![
                insert("currentScene!b"),
                Outcome::InsertFor(4, "scenePatience.b.out".to_owned()),
            ],
            "the transition arms the destination's marker, so every entry path \
             (compile-time start, transition, re-entry) threads through the same \
             scene-entry outcomes"
        );
    }
}
