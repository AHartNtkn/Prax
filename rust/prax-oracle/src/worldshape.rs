//! `worldshape` — the world-fidelity gate (S7 design §2).
//!
//! Worlds are authored DATA. A mis-transcribed label, a swapped role, a weight
//! typo or a dropped setup fact presents at trace time EXACTLY like an engine
//! divergence — and costs a slice's worth of localization to tell apart. This
//! module emits the same canonical JSON the frozen `worldshape` does, so every
//! port error becomes a one-line structural diff BEFORE a turn runs, and
//! ENUMERATION is only reportable behind a green one ([S-I2]).
//!
//! Two top-level keys, so a shape mismatch reports differently from a body one:
//!
//! - `shape` — the skeleton AND the FULL post-setup state. [S-C6] killed the
//!   "setup db as a set suffices" claim: two setup orders can produce an
//!   identical sentence set with different expiries, a different stream
//!   position, or a different schedule firing order. The state fields go in
//!   verbatim and schedule rules are listed in DECLARATION order [D-I5].
//! - `bodies` — every Condition and Outcome under a canonical encoder
//!   implemented on BOTH sides. Haskell `show` versus Rust `Debug` must never be
//!   the channel: it would report a formatting difference as a port error and
//!   hide a real one behind an accidental match.

use crate::record::state_fields;
use prax_core::engine::State;
use prax_core::query::{CalcOp, CmpOp, Condition};
use prax_core::types::{Outcome, Practice};
use serde_json::{Map, Value, json};

/// The canonical Condition encoding: a JSON array headed by the constructor
/// name. TOTAL — every constructor is listed, so a new one is a compile error
/// here and a mismatch there, never a silent omission.
pub fn cond_json(c: &Condition) -> Value {
    match c {
        Condition::Match(s) => json!(["Match", s]),
        Condition::Not(s) => json!(["Not", s]),
        Condition::Eq(a, b) => json!(["Eq", a, b]),
        Condition::Neq(a, b) => json!(["Neq", a, b]),
        Condition::Cmp(op, a, b) => json!(["Cmp", cmp_op(*op), a, b]),
        Condition::Calc(r, op, a, b) => json!(["Calc", r, calc_op(*op), a, b]),
        Condition::Count(r, s) => json!(["Count", r, s]),
        Condition::Subquery { set, find, where_ } => {
            json!(["Subquery", set, find, conds_json(where_)])
        }
        Condition::Or(cls) => json!([
            "Or",
            cls.iter().map(|c| conds_json(c)).collect::<Vec<_>>()
        ]),
        Condition::Absent(cs) => json!(["Absent", conds_json(cs)]),
        Condition::Exists(cs) => json!(["Exists", conds_json(cs)]),
    }
}

/// A conjunction, canonically encoded.
pub fn conds_json(cs: &[Condition]) -> Value {
    Value::Array(cs.iter().map(cond_json).collect())
}

/// The canonical Outcome encoding, same discipline as [`cond_json`].
pub fn outcome_json(o: &Outcome) -> Value {
    match o {
        Outcome::Insert(s) => json!(["Insert", s]),
        Outcome::Delete(s) => json!(["Delete", s]),
        Outcome::InsertFor(n, s) => json!(["InsertFor", n, s]),
        Outcome::Call(f, args) => json!(["Call", f, args]),
        Outcome::ForEach(cs, os) => json!(["ForEach", conds_json(cs), outs_json(os)]),
        Outcome::Roll(n, d, cs, os) => json!(["Roll", n, d, conds_json(cs), outs_json(os)]),
    }
}

/// An outcome list, canonically encoded.
pub fn outs_json(os: &[Outcome]) -> Value {
    Value::Array(os.iter().map(outcome_json).collect())
}

/// The frozen `show`ing of a comparison operator.
fn cmp_op(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Lt => "Lt",
        CmpOp::Lte => "Lte",
        CmpOp::Gt => "Gt",
        CmpOp::Gte => "Gte",
    }
}

/// The frozen `show`ing of a calculation operator.
fn calc_op(op: CalcOp) -> &'static str {
    match op {
        CalcOp::Add => "Add",
        CalcOp::Sub => "Sub",
        CalcOp::Mul => "Mul",
        CalcOp::Mod => "Mod",
    }
}

/// Every sentence string an outcome list mentions (`Prax.Types.outcomeSents`) —
/// the shape's summary of what a spawn writes.
fn outcome_sents(os: &[Outcome]) -> Vec<String> {
    let mut out = Vec::new();
    for o in os {
        match o {
            Outcome::Insert(s) | Outcome::Delete(s) | Outcome::InsertFor(_, s) => {
                out.push(s.clone());
            }
            Outcome::Call(_, _) => {}
            Outcome::ForEach(cs, inner) | Outcome::Roll(_, _, cs, inner) => {
                out.extend(cond_sents(cs));
                out.extend(outcome_sents(inner));
            }
        }
    }
    out
}

/// Every sentence string a conjunction mentions (`Prax.Query.condSents`).
fn cond_sents(cs: &[Condition]) -> Vec<String> {
    let mut out = Vec::new();
    for c in cs {
        match c {
            Condition::Match(s) | Condition::Not(s) => out.push(s.clone()),
            Condition::Subquery { where_, .. } => out.extend(cond_sents(where_)),
            Condition::Or(cls) => {
                for cl in cls {
                    out.extend(cond_sents(cl));
                }
            }
            Condition::Absent(inner) | Condition::Exists(inner) => out.extend(cond_sents(inner)),
            _ => {}
        }
    }
    out
}

/// Can any outcome in this subtree draw? The static half of the
/// zero-setup-rolls assertion.
fn outcome_draws(o: &Outcome) -> bool {
    match o {
        Outcome::Roll(_, _, _, _) => true,
        Outcome::ForEach(_, os) => os.iter().any(outcome_draws),
        _ => false,
    }
}

/// The whole shape+bodies document for a built world.
///
/// # Errors
/// If setup can consume the die — the setup-db SET comparison is only sound for
/// a world whose setup consumes no rolls [D-I5], so a violation stops the gate
/// rather than being footnoted in it.
pub fn worldshape(world: &str, st: &mut State) -> Result<Value, String> {
    check_setup_rolls_zero(world, st)?;
    Ok(json!({
        "format": 1,
        "engine": "rust",
        "world": world,
        "shape": shape(st),
        "bodies": bodies(st),
    }))
}

fn check_setup_rolls_zero(world: &str, st: &State) -> Result<(), String> {
    let offenders: Vec<&str> = st
        .practice_defs()
        .values()
        .filter(|p| p.init_outcomes.iter().any(outcome_draws))
        .map(|p| p.id.as_str())
        .collect();
    if offenders.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "worldshape {world}: setup draws from the die — practice init outcomes {offenders:?} \
             contain a Roll. The setup-db set comparison is only sound for a world whose setup \
             consumes no rolls (S7 design [D-I5])."
        ))
    }
}

fn practice_shape(p: &Practice) -> Value {
    json!({
        "id": p.id,
        "name": p.name,
        "roles": p.roles,
        // DECLARATION order — the goldens are label sequences and the planner's
        // tiebreak is by label, so this is the fidelity crux.
        "action_labels": p.actions.iter().map(|a| a.name.clone()).collect::<Vec<_>>(),
        "data_facts": p.data_facts,
        "init_sentences": outcome_sents(&p.init_outcomes),
    })
}

fn shape(st: &mut State) -> Value {
    let practices: Vec<Value> = st.practice_defs().values().map(practice_shape).collect();
    let characters: Vec<Value> = st
        .characters()
        .iter()
        .map(|c| {
            json!({
                "name": c.name,
                "bound_to": c.bound_to,
                "want_utilities": c.wants.iter().map(|w| w.utility).collect::<Vec<_>>(),
                "desires": c.desires,
            })
        })
        .collect();
    let desires: Vec<String> = st.desires_src().iter().map(|d| d.name.clone()).collect();
    let schedule: Vec<Value> = st
        .schedule_src()
        .iter()
        .map(|r| json!({"name": r.name, "period": r.period}))
        .collect();
    let functions: Vec<String> = st.functions_src().iter().map(|f| f.name.clone()).collect();
    let sorts: Vec<Value> = st
        .sorts()
        .iter()
        .map(|(s, ms)| json!([s, ms]))
        .collect();
    let engine_rules = st.engine_rule_names().to_vec();
    let axiom_heads = st.axiom_head_names();
    let scope_size = st.prediction_scope().len();
    let mut state = Map::new();
    for (k, v) in state_fields(st) {
        state.insert(k.into(), v);
    }
    json!({
        "practices": practices,
        "characters": characters,
        "desires": desires,
        "schedule": schedule,
        "engine_rules": engine_rules,
        "functions": functions,
        "sorts": sorts,
        "axiom_heads": axiom_heads,
        "prediction_scope_size": scope_size,
        "state": Value::Object(state),
        "setup_db": st.labeled_facts(),
        "setup_rolls_zero": true,
    })
}

fn bodies(st: &State) -> Value {
    let mut practices = Map::new();
    for (pid, p) in st.practice_defs() {
        practices.insert(
            pid.clone(),
            json!({
                "actions": p.actions.iter().map(|a| json!({
                    "label": a.name,
                    "when": conds_json(&a.when),
                    "then": outs_json(&a.then),
                })).collect::<Vec<_>>(),
                "inits": outs_json(&p.init_outcomes),
            }),
        );
    }
    let mut characters = Map::new();
    for c in st.characters() {
        characters.insert(
            c.name.clone(),
            Value::Array(
                c.wants
                    .iter()
                    .map(|w| json!({"utility": w.utility, "when": conds_json(&w.when)}))
                    .collect(),
            ),
        );
    }
    let mut desires = Map::new();
    for d in st.desires_src() {
        desires.insert(
            d.name.clone(),
            json!({"utility": d.want.utility, "when": conds_json(&d.want.when)}),
        );
    }
    let mut schedule = Map::new();
    for r in st.schedule_src() {
        schedule.insert(
            r.name.clone(),
            Value::Array(
                r.body
                    .iter()
                    .map(|(cs, os)| json!({"when": conds_json(cs), "then": outs_json(os)}))
                    .collect(),
            ),
        );
    }
    let mut functions = Map::new();
    for f in st.functions_src() {
        functions.insert(
            f.name.clone(),
            json!({
                "params": f.params,
                "cases": f.cases.iter().map(|c| json!({
                    "when": conds_json(&c.conditions),
                    "then": outs_json(&c.outcomes),
                })).collect::<Vec<_>>(),
            }),
        );
    }
    json!({
        "practices": Value::Object(practices),
        "characters": Value::Object(characters),
        "desires": Value::Object(desires),
        "schedule": Value::Object(schedule),
        "functions": Value::Object(functions),
        "axioms": st.axioms_src().iter().map(|a| json!({
            "when": conds_json(&a.when),
            "then": a.then,
        })).collect::<Vec<_>>(),
        "prediction_scope": conds_json(st.prediction_scope()),
    })
}
