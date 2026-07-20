//! Byte-for-byte replay of `conformance/fixtures/npc.json` against the Rust
//! loop: `run_npc_ticks` end to end over synthetic casts, before any shipped
//! world exists.
//!
//! Each scenario reconstructs the world with the Rust builder API, runs the same
//! 24 turns at the same depth, and asserts the FROZEN engine's narration line
//! for line, the full final state dump (labeled base facts, closed view, cursor,
//! rng, dues, expiries), the standing-intention map (act label plus every motive
//! signature field), and who is still alive.
//!
//! The three scenarios cover the loop's S6 surface: round boundaries firing on
//! the rotation wrap, a death mid-run (the corpse never acts again), and the v35
//! commitment semantics in both directions — intentions HOLDING through quiet
//! rounds and WAKING when a schedule rule flips a gated desire's liveness under
//! them (the v37 wake).

#[cfg(test)]
mod replay {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    use prax_core::engine::State;
    use prax_core::query::{Condition, eq, not_};
    use prax_core::turn::run_npc_ticks;
    use prax_core::types::{Action, Character, Desire, Practice, ScheduleRule, Want, insert};
    use serde_json::Value;

    fn load() -> Value {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../../conformance/fixtures/npc.json");
        let text = fs::read_to_string(&p).unwrap_or_else(|e| panic!("reading npc.json: {e}"));
        serde_json::from_str(&text).expect("parsing npc.json")
    }

    fn strs(v: &Value) -> Vec<String> {
        v.as_array()
            .expect("array")
            .iter()
            .map(|s| s.as_str().expect("string").to_owned())
            .collect()
    }

    fn i64_map(v: &Value) -> BTreeMap<String, i64> {
        v.as_object()
            .expect("object")
            .iter()
            .map(|(k, n)| (k.clone(), n.as_i64().expect("i64")))
            .collect()
    }

    fn m(s: &str) -> Condition {
        Condition::Match(s.to_owned())
    }

    /// The Rust twin of the oracle's `npcScenarios`, by name.
    fn scenario(name: &str) -> State {
        match name {
            "npc: boundaries and quiet holds" => {
                let yard = Practice::new("yard")
                    .roles(["R"])
                    .action(
                        Action::new("[Actor]: sweep the step")
                            .when([not_("swept.Actor")])
                            .then([insert("swept.Actor")]),
                    )
                    .action(Action::new("[Actor]: idle about"));
                let mut st = State::new();
                st.define_practices([yard]).unwrap();
                st.set_characters(vec![
                    Character::new("alice").want(Want::new(vec![m("swept.alice")], 2)),
                    Character::new("bob").want(Want::new(vec![m("swept.bob")], 2)),
                    Character::new("cara"),
                ])
                .unwrap();
                st.perform_outcome(&insert("practice.yard.here")).unwrap();
                st
            }
            "npc: a death mid-run" => {
                let duel = Practice::new("duel")
                    .roles(["R"])
                    .action(
                        Action::new("[Actor]: strike bob")
                            .when([eq("Actor", "cara"), not_("dead.bob")])
                            .then([insert("dead.bob")]),
                    )
                    .action(
                        Action::new("[Actor]: sweep the step")
                            .when([not_("swept.Actor")])
                            .then([insert("swept.Actor")]),
                    )
                    .action(Action::new("[Actor]: idle about"));
                let mut st = State::new();
                st.define_practices([duel]).unwrap();
                st.set_characters(vec![
                    Character::new("alice").want(Want::new(vec![m("swept.alice")], 2)),
                    Character::new("bob"),
                    Character::new("cara").want(Want::new(vec![m("dead.bob")], 9)),
                ])
                .unwrap();
                st.perform_outcome(&insert("practice.duel.here")).unwrap();
                st
            }
            "npc: the schedule-gated wake" => {
                let square = Practice::new("square")
                    .roles(["R"])
                    .action(
                        Action::new("[Actor]: sell at the market")
                            .when([m("marketDay"), not_("sold.Actor")])
                            .then([insert("sold.Actor")]),
                    )
                    .action(
                        Action::new("[Actor]: sweep the step")
                            .when([not_("swept.Actor")])
                            .then([insert("swept.Actor")]),
                    )
                    .action(Action::new("[Actor]: idle about"));
                let mut st = State::new();
                st.define_practices([square]).unwrap();
                st.set_characters(vec![
                    Character::new("alice").holds("wants-market"),
                    Character::new("bob").want(Want::new(vec![m("swept.bob")], 2)),
                    Character::new("cara"),
                ])
                .unwrap();
                st.set_desires(vec![Desire::new(
                    "wants-market",
                    Want::new(vec![m("marketDay"), m("sold.Owner")], 5),
                )])
                .unwrap();
                st.set_schedule(vec![
                    ScheduleRule::new("market", 3).clause([not_("marketDay")], [insert("marketDay")]),
                ])
                .unwrap();
                st.perform_outcome(&insert("practice.square.here")).unwrap();
                st
            }
            other => panic!("npc.json scenario has no Rust twin: {other:?}"),
        }
    }

    fn check_signature(got: &prax_core::planner::MotiveSignature, want: &Value, ctx: &str) {
        assert_eq!(got.bearing, strs(&want["bearing"]), "bearing @ {ctx}");
        let sat: Vec<usize> = want["satisfaction"]
            .as_array()
            .expect("satisfaction array")
            .iter()
            .map(|n| usize::try_from(n.as_u64().expect("count")).expect("count fits"))
            .collect();
        assert_eq!(got.satisfaction, sat, "satisfaction @ {ctx}");
        assert_eq!(
            got.live_desires,
            strs(&want["liveDesires"]),
            "liveDesires @ {ctx}"
        );
        let known: Vec<(String, String)> = want["knownMotives"]
            .as_array()
            .expect("knownMotives array")
            .iter()
            .map(|pair| {
                let p = strs(pair);
                (p[0].clone(), p[1].clone())
            })
            .collect();
        assert_eq!(got.known_motives, known, "knownMotives @ {ctx}");
    }

    // FIXTURE REPLAY: npc.json — the narration, the final dump, the standing
    // intentions, and the survivors, all asserted against the frozen loop.
    #[test]
    fn npc_runs_replay_line_for_line() {
        let data = load();
        let scenarios = data["scenarios"].as_array().expect("scenarios array");
        assert!(!scenarios.is_empty(), "npc.json has no scenarios");
        for sc in scenarios {
            let name = sc["name"].as_str().expect("scenario name");
            let depth =
                i32::try_from(sc["depth"].as_i64().expect("depth")).expect("depth fits");
            let steps =
                i32::try_from(sc["steps"].as_i64().expect("steps")).expect("steps fits");
            let mut st = scenario(name);

            let narration = run_npc_ticks(&mut st, depth, steps);
            assert_eq!(narration, strs(&sc["narration"]), "narration @ {name}");

            let dump = &sc["final"];
            assert_eq!(st.labeled_facts(), strs(&dump["facts"]), "facts @ {name}");
            assert_eq!(st.labeled_view(), strs(&dump["view"]), "view @ {name}");
            assert_eq!(
                i64::from(st.cursor()),
                dump["cursor"].as_i64().expect("cursor"),
                "cursor @ {name}"
            );
            assert_eq!(st.rng_seed(), dump["rng"].as_i64(), "rng @ {name}");
            assert_eq!(st.schedule_dues(), i64_map(&dump["dues"]), "dues @ {name}");
            assert_eq!(
                st.expiries_rendered(),
                i64_map(&dump["expiries"]),
                "expiries @ {name}"
            );

            // The survivors, in cast order.
            let names: Vec<String> = st.characters().iter().map(|c| c.name.clone()).collect();
            let mut alive = Vec::new();
            for n in names {
                if !st.db_has(&format!("dead.{n}")) {
                    alive.push(n);
                }
            }
            assert_eq!(alive, strs(&sc["alive"]), "survivors @ {name}");

            // The standing intentions: the same keys, the same acts, the same bases.
            let want_int = sc["intentions"].as_object().expect("intentions object");
            let got_int = st.intentions_map().clone();
            let got_keys: Vec<&String> = got_int.keys().collect();
            let want_keys: Vec<&String> = want_int.keys().collect();
            assert_eq!(got_keys, want_keys, "intention keys @ {name}");
            for (who, want) in want_int {
                let intent = &got_int[who];
                assert_eq!(
                    intent.act.as_ref().map(|g| g.label.clone()),
                    want["act"].as_str().map(str::to_owned),
                    "intention act for {who} @ {name}"
                );
                check_signature(&intent.basis, &want["basis"], &format!("{who} @ {name}"));
            }
        }
    }

    // The wake is the point of the third scenario: alice idles under a shut gate
    // and SELLS on the turn after the schedule opens the market. If the narration
    // ever loses that line, the v37 wake has stopped working.
    #[test]
    fn the_schedule_gated_wake_actually_wakes() {
        let data = load();
        let sc = data["scenarios"]
            .as_array()
            .expect("scenarios")
            .iter()
            .find(|s| s["name"] == "npc: the schedule-gated wake")
            .expect("the wake scenario must be in the corpus");
        let narration = strs(&sc["narration"]);
        assert!(
            narration.contains(&"alice: sell at the market".to_owned()),
            "the gated desire never woke: {narration:?}"
        );
        assert_eq!(
            narration[0], "alice: idle about",
            "alice must be committed to idling while the gate is shut"
        );
    }
}
