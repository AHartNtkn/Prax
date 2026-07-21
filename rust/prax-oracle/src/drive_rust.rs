//! Driving the RUST engine in process.
//!
//! The two walks the frozen oracle emits, transcribed against `prax_core`:
//! [`trace_walk`] mirrors `oracle/TraceMain.hs`'s `traceWalk` (which mirrors
//! `Prax.GoldenDriveSpec.driveLabels`), and [`rand_walk`] mirrors `randWalk`
//! (which mirrors `Prax.Stress.runRandom`). Both build their records through the
//! ONE builder in [`crate::record`].
//!
//! In process, never a subprocess: the Rust side has no reason to serialize and
//! re-parse its own state, and a JSON round-trip would put a second
//! representation between the engine and the comparison.

use crate::record::{
    Emit, Mode, Record, TurnFields, draw_entry, identity, scores, stop_record, turn_record,
};
use crate::walk::{Stop, pick};
use prax_core::engine::State;
use prax_core::turn::{advance, npc_act_logged};
use serde_json::{Value, json};

/// The trace walk's header — the frozen `runTrace`'s, field for field except
/// `engine`.
pub fn trace_header(world: &str, turns: i64, idle: Option<&str>, depth: i32, mode: Mode, em: Emit) -> Record {
    let mut m = serde_json::Map::new();
    m.insert("format".into(), json!(1));
    m.insert("engine".into(), json!("rust"));
    m.insert("world".into(), json!(world));
    m.insert("turns".into(), json!(turns));
    m.insert("idle".into(), json!(idle));
    m.insert("depth".into(), json!(depth));
    m.insert("mode".into(), json!(mode.as_str()));
    m.insert("seed".into(), Value::Null);
    for (k, v) in em.header_fields() {
        m.insert(k.into(), v);
    }
    Value::Object(m)
}

/// The randtrace walk's header.
pub fn rand_header(world: &str, seed: i64, cap: i64, mode: Mode, die_seed: Option<i64>, em: Emit) -> Record {
    let mut m = serde_json::Map::new();
    m.insert("format".into(), json!(1));
    m.insert("engine".into(), json!("rust"));
    m.insert("world".into(), json!(world));
    m.insert("seed".into(), json!(seed));
    m.insert("cap".into(), json!(cap));
    m.insert("mode".into(), json!(mode.as_str()));
    m.insert("dieSeed".into(), json!(die_seed));
    for (k, v) in em.header_fields() {
        m.insert(k.into(), v);
    }
    Value::Object(m)
}

/// `traceWalk`: one record per turn — advance, and unless the actor is the
/// idler, have them act. The state fields report the carry-forward (post-action)
/// state; `boundary` is whether `advance` fired a round boundary. Ends in the
/// terminal record.
pub fn trace_walk(
    st: &mut State,
    em: Emit,
    depth: i32,
    total: i64,
    idle: Option<&str>,
    mode: Mode,
) -> Vec<Record> {
    let mut out = Vec::new();
    for t in 1..=total {
        let pre = if em.logs { Some(st.clone()) } else { None };
        let before = st.current_turn();
        let actor = advance(st);
        let boundary = st.current_turn() != before;
        let name = actor.name.clone();
        let mut extras: Vec<(&'static str, Value)> = Vec::new();
        if let (true, Some(p)) = (boundary && em.logs, pre.as_ref()) {
            extras.push(("boundary_log", boundary_log(&mut p.clone(), st)));
        }
        if Some(name.as_str()) == idle {
            out.push(turn_record(
                st,
                TurnFields {
                    t,
                    boundary,
                    actor: name,
                    action: json!("-"),
                    idle: true,
                    passes: None,
                    walk_seed: None,
                    extras,
                },
                mode,
            ));
            continue;
        }
        if em.candidates {
            let cands: Vec<String> = st
                .candidate_actions(&actor)
                .into_iter()
                .map(|g| g.label)
                .collect();
            extras.push(("candidates", json!(cands)));
        }
        if em.scores {
            let rows = (0..=depth)
                .map(|d| {
                    (
                        d,
                        st.score_actions(d, &actor)
                            .into_iter()
                            .map(|(g, s)| (g.label, s))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect();
            extras.push(("scores", scores(rows)));
            extras.push(("intention_before", intention_json(st.intention_of(&name))));
        }
        let mut steps: Vec<Value> = Vec::new();
        let acted = npc_act_logged(st, depth, &actor, &mut |step| steps.push(draw_entry(&step)));
        if em.logs {
            extras.push(("draws", Value::Array(steps)));
        }
        if em.identity {
            extras.push((
                "identity",
                acted.as_ref().map_or(Value::Null, |g| identity(st, g)),
            ));
        }
        if em.scores {
            extras.push(("intention_after", intention_json(st.intention_of(&name))));
        }
        let (action, idled) = match &acted {
            Some(g) => (json!(g.label), false),
            None => (json!("-"), true),
        };
        out.push(turn_record(
            st,
            TurnFields {
                t,
                boundary,
                actor: name,
                action,
                idle: idled,
                passes: None,
                walk_seed: None,
                extras,
            },
            mode,
        ));
    }
    out.push(stop_of(&Stop::Turns, 0, total));
    out
}

/// `randWalk`: replay `Prax.Stress.runRandom` step for step, emitting one record
/// per advance (idle passes included) and a terminal record naming the stop rule.
///
/// This walk never touches the planner — it selects with `possible_actions` and
/// [`pick`]. That is why the classifier is MODE-PARAMETERISED [D-C2].
pub fn rand_walk(st: &mut State, em: Emit, mode: Mode, cap: i64, seed0: u64) -> Vec<Record> {
    let mut out = Vec::new();
    let mut passes: i64 = 0;
    let mut t: i64 = 1;
    let mut k = cap;
    let mut s = seed0;
    let mut recs: i64 = 0;
    loop {
        // The go-loop's four pre-advance exits, in order, from the single shared
        // decision ([P8]) — so this walk and Stress's `run_random` can never
        // drift. Behaviour is byte-preserved: `pre_advance_stop` returns `Cap`
        // when `k == 0` before consulting `ending`/`living` (both side-effect
        // free reads), exactly as the former inline checks did.
        let ending = ending_reached(st);
        let living = st.living_characters().len();
        if let Some(rs) = prax_core::stress::pre_advance_stop(k, ending, living, passes) {
            // The go-loop decision is single-sourced in prax_core::stress ([P8]);
            // map it to the comparator's wire-reason `Stop` for the record.
            let stop = match rs {
                prax_core::stress::RunStop::Cap => Stop::Cap,
                prax_core::stress::RunStop::Ending(e) => Stop::Ending(e),
                prax_core::stress::RunStop::Extinct => Stop::Extinct,
                prax_core::stress::RunStop::DeadEnd => Stop::DeadEnd,
            };
            out.push(stop_of(&stop, passes, recs));
            return out;
        }
        let pre = if em.logs { Some(st.clone()) } else { None };
        let before = st.current_turn();
        let actor = advance(st);
        let boundary = st.current_turn() != before;
        let name = actor.name.clone();
        let mut extras: Vec<(&'static str, Value)> = Vec::new();
        if let (true, Some(p)) = (boundary && em.logs, pre.as_ref()) {
            extras.push(("boundary_log", boundary_log(&mut p.clone(), st)));
        }
        let acts = st.possible_actions(&name);
        if acts.is_empty() {
            if em.candidates {
                extras.push(("candidates", json!(Vec::<String>::new())));
            }
            if em.identity {
                extras.push(("identity", Value::Null));
            }
            out.push(turn_record(
                st,
                TurnFields {
                    t,
                    boundary,
                    actor: name,
                    action: Value::Null,
                    idle: true,
                    passes: Some(passes),
                    walk_seed: Some(None),
                    extras,
                },
                mode,
            ));
            passes += 1;
            recs += 1;
            continue;
        }
        let (i, s2) = pick(acts.len(), s);
        let ga = acts[i].clone();
        let mut steps: Vec<Value> = Vec::new();
        st.perform_action_logged(&ga, &mut |step| steps.push(draw_entry(&step)));
        if em.logs {
            extras.push(("draws", Value::Array(steps)));
        }
        // NATIVE order [D-C1]/[S-C2]: the walk indexes this list, so its order is
        // part of the comparison.
        if em.candidates {
            extras.push((
                "candidates",
                json!(acts.iter().map(|g| g.label.clone()).collect::<Vec<_>>()),
            ));
        }
        if em.identity {
            extras.push(("identity", identity(st, &ga)));
        }
        out.push(turn_record(
            st,
            TurnFields {
                t,
                boundary,
                actor: name,
                action: json!(ga.label),
                idle: false,
                passes: Some(passes),
                walk_seed: Some(Some(s2)),
                extras,
            },
            mode,
        ));
        passes = 0;
        t += 1;
        k -= 1;
        s = s2;
        recs += 1;
    }
}

/// The ending reached, if any — an `ending.<key>` fact in the BASE db (the
/// frozen `endingReached`, which takes the first binding).
///
/// [M3, carried by §10 and unguarded] The orders differ in principle: the frozen
/// `listToMaybe [ e | b <- unify "ending.E" … ]` takes the first UNIFY BINDING,
/// this takes the first CHILD KEY. They cannot differ in any world shipped so
/// far, because `ending!X` is an exclusion slot — `ending` has exactly one child
/// at all times, so both orders name it. The first world that can hold TWO
/// simultaneous endings makes this a real divergence, and there is no pin that
/// would catch it: the differential compares the walks, and both would stop.
fn ending_reached(st: &mut State) -> Option<String> {
    st.db_child_keys("ending").into_iter().next()
}

/// The terminal record for a stop rule.
fn stop_of(stop: &Stop, passes: i64, records: i64) -> Record {
    stop_record(stop.reason(), stop.ending(), passes, records)
}

/// A standing intention as JSON, or null ([M4]) — the tell that separates
/// "the score tables agree but the action differs" into intention-vs-planner.
fn intention_json(i: Option<prax_core::planner::Intention>) -> Value {
    match i {
        None => Value::Null,
        Some(i) => json!({
            "act": i.act.map(|g| g.label),
            "basis": {
                "bearing": i.basis.bearing,
                "satisfaction": i.basis.satisfaction,
                "liveDesires": i.basis.live_desires,
                "knownMotives": i.basis.known_motives.iter()
                    .map(|(m, d)| vec![m.clone(), d.clone()]).collect::<Vec<_>>(),
            },
        }),
    }
}

/// THE BOUNDARY LOG ([S-C5]): what actually fired at this boundary. An expiry
/// firing on the wrong subtree, or dropping silently, leaves the `expiries` MAP
/// equal — so the pointer has to come from what fired, not from what remains.
/// Read off the pre-boundary state exactly as `round_boundary` reads it (due
/// rules in DECLARATION order; due expiries by their queue entry, name-sorted
/// because the queue is genuinely unordered), with each due expiry's existence
/// guard and post-boundary presence OBSERVED on the two states.
fn boundary_log(pre: &mut State, post: &State) -> Value {
    let now = pre.current_turn() + 1;
    let dues = pre.schedule_dues();
    let due_rules: Vec<String> = pre
        .schedule_src()
        .iter()
        .filter(|r| *dues.get(&r.name).unwrap_or(&i64::MAX) <= now)
        .map(|r| r.name.clone())
        .collect();
    let mut post_probe = post.clone();
    let entries: Vec<Value> = pre
        .expiries_rendered()
        .into_iter()
        .filter(|(_, v)| *v <= now)
        .map(|(path, due)| {
            let existed = pre.db_has(&path);
            let present = post_probe.db_has(&path);
            json!({
                "path": path,
                "due": due,
                "existed_before": existed,
                "present_after": present,
            })
        })
        .collect();
    json!({ "now": now, "due_rules": due_rules, "due_expiries": entries })
}

