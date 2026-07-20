//! The record: ONE builder, both walks.
//!
//! A differential record is a `serde_json::Value` on both sides, so field ORDER
//! can never matter and the Rust side never round-trips its own JSON (S7 design
//! §1.1). Every field the frozen `oracle/TraceMain.hs` emits is built here and
//! nowhere else — two builders (one per walk) would be a dual system whose drift
//! reads as an engine divergence.
//!
//! THE ORACLE CANON [D-C1], obeyed on both sides: facts, dues and expiries are
//! NAME-SORTED (genuinely unordered); candidate lists and score rows are
//! NATIVE-ORDER and their order is part of the comparison.

use prax_core::engine::{EffectStep, GroundedAction, State};
use prax_core::rng::roll_step;
use serde_json::{Map, Value, json};

/// A record stream: the header, the turn records, and the terminal record.
pub type Record = Value;

/// What the walk emits per turn. `decisions` omits facts entirely; `state` adds
/// the base db's labeled sentences; `view` additionally adds the closed view's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    /// Decisions only — the cheapest stream.
    Decisions,
    /// Base facts (the default comparison depth).
    State,
    /// Base facts AND the closed view — the DIV-1-shaped reclassification's
    /// channel ([S-C4]: `randtrace` gained `--mode` for exactly this).
    View,
}

impl Mode {
    /// The wire spelling (the frozen `modeStr`).
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Decisions => "decisions",
            Mode::State => "state",
            Mode::View => "view",
        }
    }
    /// Parse the wire spelling; `None` on anything else (never a silent default).
    pub fn parse(s: &str) -> Option<Mode> {
        match s {
            "decisions" => Some(Mode::Decisions),
            "state" => Some(Mode::State),
            "view" => Some(Mode::View),
            _ => None,
        }
    }
}

/// Which localization fields a walk emits (the frozen `Emit`). All off by
/// default; the comparator turns them on for the rerun at the divergent record.
/// `candidates` is MANDATORY in matrix mode [S-I4] — without it ENUMERATION can
/// never fire and every enumeration bug reports as DECISION.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Emit {
    /// `--candidates`: the actor's candidate labels in NATIVE order.
    pub candidates: bool,
    /// `--scores`: the scoreActions table at depths `0..D`, plus the intention.
    pub scores: bool,
    /// `--identity`: the acted action's identity, not merely its label [S-C3].
    pub identity: bool,
    /// `--logs`: the per-turn draw log and boundary log [S-C5].
    pub logs: bool,
}

impl Emit {
    /// Everything on — the localization rerun.
    pub fn all() -> Emit {
        Emit {
            candidates: true,
            scores: true,
            identity: true,
            logs: true,
        }
    }
    /// The matrix baseline: candidates only [S-I4].
    pub fn matrix() -> Emit {
        Emit {
            candidates: true,
            ..Emit::default()
        }
    }
    /// The flags as oracle command-line arguments.
    pub fn args(self) -> Vec<String> {
        let mut v = Vec::new();
        if self.candidates {
            v.push("--candidates".to_owned());
        }
        if self.scores {
            v.push("--scores".to_owned());
        }
        if self.identity {
            v.push("--identity".to_owned());
        }
        if self.logs {
            v.push("--logs".to_owned());
        }
        v
    }
    /// The four booleans as they appear in every header.
    pub fn header_fields(self) -> Vec<(&'static str, Value)> {
        vec![
            ("candidates", json!(self.candidates)),
            ("scores", json!(self.scores)),
            ("identity", json!(self.identity)),
            ("logs", json!(self.logs)),
        ]
    }
}

/// The turn fields the two walks disagree about; everything else is derived from
/// the state by [`turn_record`].
pub struct TurnFields {
    /// The walk's turn counter (`t`) — NOT the record ordinal ([M2]: randtrace's
    /// `t` does not advance on an idle pass).
    pub t: i64,
    /// Did `advance` fire a round boundary this turn?
    pub boundary: bool,
    /// The acting character's name.
    pub actor: String,
    /// The rendered action: `"-"` in the trace walk, `null` in the randtrace
    /// walk's idle records — mirrored exactly.
    pub action: Value,
    /// Did the turn produce no move?
    pub idle: bool,
    /// The randtrace pass counter before this step ([S-I3]).
    pub passes: Option<i64>,
    /// The randtrace walk seed after the pick (`null` on an idle pass).
    pub walk_seed: Option<Option<u64>>,
    /// The localization fields, already built (candidates, scores, identity,
    /// intention_*, draws, boundary_log), in the order the walk added them.
    pub extras: Vec<(&'static str, Value)>,
}

/// Build one turn record: the walk's own fields, then the state fields, then the
/// fact fields for the mode.
pub fn turn_record(st: &State, f: TurnFields, mode: Mode) -> Record {
    let mut m = Map::new();
    m.insert("t".into(), json!(f.t));
    m.insert("boundary".into(), json!(f.boundary));
    m.insert("actor".into(), json!(f.actor));
    m.insert("action".into(), f.action);
    m.insert("idle".into(), json!(f.idle));
    if let Some(p) = f.passes {
        m.insert("passes".into(), json!(p));
    }
    if let Some(ws) = f.walk_seed {
        m.insert("walkSeed".into(), json!(ws));
    }
    for (k, v) in f.extras {
        m.insert(k.into(), v);
    }
    for (k, v) in state_fields(st) {
        m.insert(k.into(), v);
    }
    for (k, v) in fact_fields(st, mode) {
        m.insert(k.into(), v);
    }
    Value::Object(m)
}

/// The per-turn state fields shared by both walks (the frozen `stateFields`).
pub fn state_fields(st: &State) -> Vec<(&'static str, Value)> {
    vec![
        ("cursor", json!(st.cursor())),
        ("rng", json!(st.rng_seed())),
        ("dues", json!(st.schedule_dues())),
        ("expiries", json!(st.expiries_rendered())),
    ]
}

/// The fact fields for a mode (the frozen `factFields`).
pub fn fact_fields(st: &State, mode: Mode) -> Vec<(&'static str, Value)> {
    match mode {
        Mode::Decisions => vec![],
        Mode::State => vec![("facts", json!(st.labeled_facts()))],
        Mode::View => vec![
            ("facts", json!(st.labeled_facts())),
            ("view", json!(st.labeled_view())),
        ],
    }
}

/// The terminal record every walk ends with ([S-I3]): why the stream stopped,
/// the pass counter, and how many turn records preceded it. Without it a shorter
/// stream on one side has no class and no evidence.
pub fn stop_record(reason: &str, ending: Option<&str>, passes: i64, records: i64) -> Record {
    json!({
        "end": true,
        "reason": reason,
        "ending": ending,
        "passes": passes,
        "records": records,
    })
}

/// An action's IDENTITY, not its rendered label ([S-C3]).
pub fn identity(st: &State, ga: &GroundedAction) -> Value {
    json!({
        "practice_id": ga.practice_id,
        "instance_id": ga.instance_id,
        "action_id": ga.action_id,
        "bindings": st.render_bindings(&ga.bindings),
    })
}

/// The score table at depths `0..=depth`, rows in NATIVE order [D-C1], each
/// score as its raw IEEE-754 bits (`castDoubleToWord64` on the frozen side) so
/// no decimal enters the comparison [D-I1].
pub fn scores(rows: Vec<(i32, Vec<(String, f64)>)>) -> Value {
    Value::Array(
        rows.into_iter()
            .map(|(d, rs)| {
                json!({
                    "depth": d,
                    "rows": rs.into_iter()
                        .map(|(label, s)| json!({"label": label, "bits": s.to_bits()}))
                        .collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

/// One [`EffectStep`] as a draw-log entry — the mirror of the frozen
/// `drawLogJSON`'s per-outcome object.
pub fn draw_entry(step: &EffectStep) -> Value {
    json!({
        "i": step.index,
        "odds": step.odds.iter().map(|(n, d)| vec![*n, *d]).collect::<Vec<_>>(),
        "before": step.rng_before,
        "after": step.rng_after,
        "values": stream_values(step.rng_before, step.rng_after),
        "changed": step.changed,
    })
}

/// The Lehmer values a step consumed, in order. Loud past the bound: an
/// unbounded search would hang, and a silent empty list would hide exactly the
/// RNG divergence this log exists for.
///
/// # Panics
/// If the stream moved more than 4096 steps in one outcome.
pub fn stream_values(before: Option<i64>, after: Option<i64>) -> Vec<i64> {
    let (Some(a), Some(b)) = (before, after) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut s = a;
    while s != b {
        assert!(
            out.len() < 4096,
            "the rng stream moved more than 4096 steps in one outcome (from {a} to {b})"
        );
        s = roll_step(s);
        out.push(s);
    }
    out
}
