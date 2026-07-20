//! Byte-for-byte replay of `conformance/fixtures/engine.json` against the Rust
//! engine: each scenario's world + step sequence is reconstructed with the Rust
//! builder/install API, and after every step the full state dump (labeled base
//! facts, closed view, cursor, rng, dues, expiries) is asserted equal to the one
//! the FROZEN engine produced (D-panel I4). Perform semantics — the three-tier
//! router, spawn's BASE-vs-view opacity and re-spawn, ForEach's snapshot, Call's
//! BASE-db quirk and first-case-first-binding, expiry arm/refresh/cancel/purge,
//! Roll's advance-on-miss, the ⊥ collapse — is pinned by OBSERVATION here, not by
//! transcription.

#[cfg(test)]
mod replay {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    use prax_core::engine::State;
    use prax_core::query::Condition;
    use prax_core::types::{
        Axiom, Function, Outcome, Practice, call, delete, for_each, insert, insert_for,
    };
    use serde_json::Value;

    fn load() -> Value {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../../conformance/fixtures/engine.json");
        let text = fs::read_to_string(&p).unwrap_or_else(|e| panic!("reading engine.json: {e}"));
        serde_json::from_str(&text).expect("parsing engine.json")
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

    fn ins(s: &str) -> Outcome {
        insert(s)
    }

    /// A step: perform one outcome (all engine scenarios drive perform_outcome).
    type Step = Box<dyn Fn(&mut State)>;

    fn perform(o: Outcome) -> Step {
        Box::new(move |s: &mut State| s.perform_outcome(&o).unwrap())
    }

    /// Reconstruct a named scenario's built world and its ordered steps — the
    /// Rust twin of the oracle's `engineFixture` scenarios (world-construction is
    /// transcribed on both sides; only the OUTPUT dumps come from the frozen
    /// engine).
    fn scenario(name: &str) -> (State, Vec<Step>) {
        match name {
            "spawn: base-vs-view opacity, inits run despite a view-visible instance" => {
                let mut st = State::new();
                st.define_practices([Practice::new("pp")
                    .roles(["R"])
                    .init([ins("practice.pp.R.mark")])])
                    .unwrap();
                st.set_axioms(vec![Axiom::new(vec![m("seed.X")], ["practice.pp.X"])])
                    .unwrap();
                (
                    st,
                    vec![perform(ins("seed.a")), perform(ins("practice.pp.a"))],
                )
            }
            "spawn: re-spawn after delete re-runs init" => {
                let mut st = State::new();
                st.define_practices([Practice::new("rp")
                    .roles(["R"])
                    .init([ins("practice.rp.R.mark")])])
                    .unwrap();
                (
                    st,
                    vec![
                        perform(ins("practice.rp.a")),
                        perform(delete("practice.rp.a")),
                        perform(ins("practice.rp.a")),
                    ],
                )
            }
            "ForEach snapshots bindings: a member inserted mid-fold is not visited" => {
                let st = State::new();
                (
                    st,
                    vec![
                        perform(ins("member.a")),
                        perform(for_each(
                            vec![m("member.X")],
                            vec![ins("member.b"), ins("visited.X")],
                        )),
                    ],
                )
            }
            "Call: queries BASE (not the view), first case + first binding only" => {
                let mut st = State::new();
                st.define_functions([Function::new("pick", ["Who"])
                    .case([m("cand.Who.X")], [ins("chose.X")])
                    .case([], [ins("fallback")])])
                    .unwrap();
                st.set_axioms(vec![Axiom::new(vec![m("trig.W")], ["cand.W.zzz"])])
                    .unwrap();
                (
                    st,
                    vec![
                        perform(ins("cand.gil.beta")),
                        perform(ins("cand.gil.alpha")),
                        perform(ins("trig.gil")),
                        perform(call("pick", vec!["gil".to_owned()])),
                    ],
                )
            }
            "InsertFor: arm, refresh, bare-insert cancel, sibling arm, delete purge" => {
                let st = State::new();
                (
                    st,
                    vec![
                        perform(insert_for(3, "a.b.c")),
                        perform(insert_for(5, "a.b.c")),
                        perform(ins("a.b.c")),
                        perform(insert_for(4, "a.b.c")),
                        perform(insert_for(4, "a.b.d")),
                        perform(delete("a.b")),
                    ],
                )
            }
            "Roll: unconditional advance on a miss (seed 1: rollStep is odd -> miss)" => {
                let mut st = State::new();
                st.seed_die(1).unwrap();
                (
                    st,
                    vec![
                        perform(Outcome::Roll(1, 2, vec![], vec![ins("roll.a")])),
                        perform(Outcome::Roll(1, 2, vec![], vec![ins("roll.b")])),
                    ],
                )
            }
            "Roll: a hit applies the body (seed 2: rollStep is even -> hit)" => {
                let mut st = State::new();
                st.seed_die(2).unwrap();
                (
                    st,
                    vec![perform(Outcome::Roll(1, 2, vec![], vec![ins("roll.hit")]))],
                )
            }
            "bottom collapse: a contradicting insert surfaces `contradiction` in the view" => {
                let mut st = State::new();
                st.set_axioms(vec![
                    Axiom::new(vec![m("trig")], ["light!red"]),
                    Axiom::new(vec![m("trig")], ["light!green"]),
                ])
                .unwrap();
                (st, vec![perform(ins("trig"))])
            }
            other => panic!("engine.json scenario has no Rust twin: {other:?}"),
        }
    }

    fn check_dump(st: &State, dump: &Value, name: &str, i: usize) {
        assert_eq!(st.labeled_facts(), strs(&dump["facts"]), "facts @ {name} step {i}");
        assert_eq!(st.labeled_view(), strs(&dump["view"]), "view @ {name} step {i}");
        assert_eq!(
            i64::from(st.cursor()),
            dump["cursor"].as_i64().unwrap(),
            "cursor @ {name} step {i}"
        );
        assert_eq!(st.rng_seed(), dump["rng"].as_i64(), "rng @ {name} step {i}");
        assert_eq!(st.schedule_dues(), i64_map(&dump["dues"]), "dues @ {name} step {i}");
        assert_eq!(
            st.expiries_rendered(),
            i64_map(&dump["expiries"]),
            "expiries @ {name} step {i}"
        );
    }

    // FIXTURE REPLAY: engine.json — reconstruct each scenario and assert every
    // step's full state dump equals the frozen engine's, byte-for-byte.
    #[test]
    fn engine_scenarios_replay_byte_for_byte() {
        let data = load();
        let scenarios = data["scenarios"].as_array().expect("scenarios array");
        assert!(!scenarios.is_empty(), "engine.json has no scenarios");
        for sc in scenarios {
            let name = sc["name"].as_str().expect("scenario name");
            let steps = sc["steps"].as_array().expect("steps array");
            let (mut st, ops) = scenario(name);
            // step 0 is the <initial> dump (before any op).
            check_dump(&st, &steps[0]["dump"], name, 0);
            assert_eq!(
                ops.len() + 1,
                steps.len(),
                "scenario {name}: Rust has {} ops, fixture has {} steps",
                ops.len(),
                steps.len() - 1
            );
            for (idx, op) in ops.iter().enumerate() {
                op(&mut st);
                check_dump(&st, &steps[idx + 1]["dump"], name, idx + 1);
            }
        }
    }
}
