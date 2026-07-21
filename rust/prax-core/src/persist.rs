//! Save/resume: a serde-JSON format for the runtime state.
//!
//! **No cross-engine save compatibility with the Haskell — a clean break
//! (PLAN.md, ARCHITECTURE).** The frozen `Prax.Persist` is line-oriented
//! (`prax-state v4`); reproducing its bytes would be pointless (nothing loads a
//! cross-engine save), so the Rust format is serde JSON with its OWN version tag
//! [`FORMAT_VERSION`] and there is no persist differential. The net is the
//! Rust-internal round-trip law plus the frozen `PersistSpec` re-expressed as
//! BEHAVIORAL pins over this format (`conformance::persist_spec`).
//!
//! The whole mutable state is the fact database, the turn cursor, the standing
//! intentions, the engine schedule's runtime half (per-rule next-dues + the
//! one-shot expiry queue), and the drama die's stream position (`rng_seed`).
//! Practices, characters, wants, and the schedule DECLARATIONS are code (the
//! world's rules), so a save captures only those six mutable fields and is
//! reloaded onto a freshly-constructed world of the same kind — which supplies
//! the rule bodies the dues re-associate to BY NAME.
//!
//! Every interned [`Sym`](crate::interner::Sym) is process-local, so it crosses
//! the file boundary by NAME and is re-interned on load. This is the [S-I1]
//! re-intern hazard the persist round trip discharges: a reloaded `Intention`'s
//! `GroundedAction` must still compare-equal against freshly computed candidates
//! after a fresh interner has minted new ids — sound because equality is
//! content-canonical under a monotonic interner.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::db::Val;
use crate::engine::{GroundedAction, State};
use crate::planner::{Intention, MotiveSignature};

/// The save-format tag, the `version` field of every serialized state. Bump it
/// when the JSON shape below changes OR when the same bytes would carry
/// different meaning under the current world model; [`deserialize_state`]
/// rejects any other tag loudly — no silent misparse of a save whose facts a
/// freshly-constructed world no longer interprets (the frozen rejection-ladder
/// STANCE, reproduced over this format's own tag).
pub const FORMAT_VERSION: &str = "prax-rs-state v1";

/// What went wrong loading a save. Every variant is a LOUD failure — a save is
/// never silently misparsed into a wrong-but-plausible state.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PersistError {
    /// A version tag other than [`FORMAT_VERSION`] — a save from another format
    /// era (or another engine). The message carries "unsupported save format".
    #[error("unsupported save format {found:?} (expected {expected:?})")]
    UnsupportedVersion { found: String, expected: String },
    /// The bytes are not a well-formed save (not JSON, missing the version tag,
    /// missing a required field, a malformed path). The message carries
    /// "malformed save".
    #[error("malformed save: {detail}")]
    Malformed { detail: String },
    /// A due names a schedule rule the reloaded world does not declare — a save
    /// from a world whose schedule has since changed. Carries "unknown schedule
    /// rule".
    #[error("unknown schedule rule {name:?} (not declared in the reloaded world)")]
    UnknownScheduleRule { name: String },
}

/// A [`Val`] crossing the file boundary as text — every symbol written by name,
/// re-interned on load.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ValRepr {
    Sym(String),
    Num(i64),
    Set(Vec<Vec<String>>),
}

/// A [`GroundedAction`] by name: the practice/instance/action ids and label are
/// already strings; the bindings serialize as `(var-name, value)` pairs.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GaRepr {
    practice_id: String,
    instance_id: String,
    action_id: String,
    bindings: Vec<(String, ValRepr)>,
    label: String,
}

/// An [`Intention`] by name: the optional grounded action plus the four
/// name-valued components of its [`MotiveSignature`].
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IntentRepr {
    act: Option<GaRepr>,
    bearing: Vec<String>,
    satisfaction: Vec<usize>,
    live_desires: Vec<String>,
    known_motives: Vec<(String, String)>,
}

/// The whole serialized state (the six mutable fields plus the version tag).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SaveDoc {
    version: String,
    cursor: i32,
    /// Present only for a seeded state; an unseeded save reloads as `None`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    rng_seed: Option<i64>,
    /// Standing intentions, keyed by character name.
    intentions: BTreeMap<String, IntentRepr>,
    /// Per-rule next-dues, keyed by schedule-rule name.
    dues: BTreeMap<String, i64>,
    /// The one-shot expiry queue: labeled sentence → due boundary. A map keeps
    /// the round trip set-equal (the internal `CompiledPath` order is
    /// incidental).
    expiries: BTreeMap<String, i64>,
    /// The fact database as labeled sentences (`!`/`.` preserved so the reload
    /// rebuilds the exclusion structure exactly).
    facts: Vec<String>,
}

fn repr_val(interner: &crate::interner::Interner, v: &Val) -> ValRepr {
    match v {
        Val::Sym(s) => ValRepr::Sym(interner.resolve(*s).to_owned()),
        Val::Num(n) => ValRepr::Num(*n),
        Val::Set(xs) => ValRepr::Set(
            xs.iter()
                .map(|row| row.iter().map(|s| interner.resolve(*s).to_owned()).collect())
                .collect(),
        ),
    }
}

fn unrepr_val(st: &mut State, r: &ValRepr) -> Val {
    match r {
        ValRepr::Sym(s) => Val::Sym(st.intern_name(s)),
        ValRepr::Num(n) => Val::Num(*n),
        ValRepr::Set(xs) => Val::Set(
            xs.iter()
                .map(|row| row.iter().map(|s| st.intern_name(s)).collect())
                .collect(),
        ),
    }
}

fn repr_intention(interner: &crate::interner::Interner, i: &Intention) -> IntentRepr {
    IntentRepr {
        act: i.act.as_ref().map(|ga| GaRepr {
            practice_id: ga.practice_id.clone(),
            instance_id: ga.instance_id.clone(),
            action_id: ga.action_id.clone(),
            bindings: ga
                .bindings
                .iter()
                .map(|(k, v)| (interner.resolve(k).to_owned(), repr_val(interner, v)))
                .collect(),
            label: ga.label.clone(),
        }),
        bearing: i.basis.bearing.clone(),
        satisfaction: i.basis.satisfaction.clone(),
        live_desires: i.basis.live_desires.clone(),
        known_motives: i.basis.known_motives.clone(),
    }
}

fn unrepr_intention(st: &mut State, r: &IntentRepr) -> Intention {
    let act = r.act.as_ref().map(|g| {
        let mut bindings = crate::db::Bindings::new();
        for (k, v) in &g.bindings {
            let key = st.intern_name(k);
            let val = unrepr_val(st, v);
            bindings.insert(key, val);
        }
        GroundedAction {
            practice_id: g.practice_id.clone(),
            instance_id: g.instance_id.clone(),
            action_id: g.action_id.clone(),
            bindings,
            label: g.label.clone(),
        }
    });
    Intention {
        act,
        basis: MotiveSignature {
            bearing: r.bearing.clone(),
            satisfaction: r.satisfaction.clone(),
            live_desires: r.live_desires.clone(),
            known_motives: r.known_motives.clone(),
        },
    }
}

/// Serialize the mutable state to a JSON string: the six mutable fields under
/// [`FORMAT_VERSION`]. Infallible — the format tolerates every legal state.
pub fn serialize_state(st: &State) -> String {
    let interner = st.interner();
    let doc = SaveDoc {
        version: FORMAT_VERSION.to_owned(),
        cursor: st.cursor(),
        rng_seed: st.rng_seed(),
        intentions: st
            .intentions_map()
            .iter()
            .map(|(name, i)| (name.clone(), repr_intention(interner, i)))
            .collect(),
        dues: st.schedule_dues(),
        expiries: st.expiries_labeled().into_iter().collect(),
        facts: st.labeled_facts(),
    };
    serde_json::to_string_pretty(&doc).expect("SaveDoc serializes")
}

/// Rebuild a saved state onto `world` (a fresh world of the same kind, which
/// supplies the practice definitions and cast). Loud on malformed input, on a
/// save from another format era (a version tag other than [`FORMAT_VERSION`]),
/// and on a due naming a rule the reloaded world does not declare.
///
/// The version tag is checked BEFORE any field parse — a foreign-era save must
/// reject as a version mismatch, not as an incidental field error (the frozen
/// order: version, then cursor).
pub fn deserialize_state(text: &str, mut world: State) -> Result<State, PersistError> {
    let value: Value = serde_json::from_str(text)
        .map_err(|e| PersistError::Malformed { detail: format!("not a JSON save: {e}") })?;
    match value.get("version").and_then(Value::as_str) {
        None => {
            return Err(PersistError::Malformed {
                detail: "no version tag (expected the format header)".to_owned(),
            });
        }
        Some(v) if v != FORMAT_VERSION => {
            return Err(PersistError::UnsupportedVersion {
                found: v.to_owned(),
                expected: FORMAT_VERSION.to_owned(),
            });
        }
        Some(_) => {}
    }
    let doc: SaveDoc = serde_json::from_value(value)
        .map_err(|e| PersistError::Malformed { detail: format!("malformed save: {e}") })?;

    // Dues re-associate to the world's declared rules BY NAME; an unknown name
    // is a loud error (a save from a world whose schedule has since changed).
    let declared: Vec<&str> = world.schedule_rules().iter().map(|r| r.name.as_str()).collect();
    for name in doc.dues.keys() {
        if !declared.contains(&name.as_str()) {
            return Err(PersistError::UnknownScheduleRule { name: name.clone() });
        }
    }

    // Facts: replace the db wholesale with the saved sentences (the frozen
    // `withDb (const (insertAll factLines emptyDb))`). `with_db` recloses.
    let facts = doc.facts.clone();
    let mut tokenize_err: Option<String> = None;
    world.with_db(|interner, _old| {
        let mut db = crate::db::Db::empty();
        for line in &facts {
            match crate::path::tokenize(interner, line) {
                Ok(path) => db = db.insert(&path),
                Err(e) => {
                    if tokenize_err.is_none() {
                        tokenize_err = Some(format!("malformed fact line {line:?}: {e:?}"));
                    }
                }
            }
        }
        db
    });
    if let Some(detail) = tokenize_err {
        return Err(PersistError::Malformed { detail });
    }

    world.set_cursor_loaded(doc.cursor);
    world.set_rng_seed_loaded(doc.rng_seed);
    world.replace_schedule_dues(doc.dues);
    let expiries: Vec<(String, i64)> = doc.expiries.into_iter().collect();
    world
        .replace_expiries_from_labeled(&expiries)
        .map_err(|e| PersistError::Malformed { detail: format!("malformed expiry path: {e:?}") })?;
    let intentions: BTreeMap<String, Intention> = doc
        .intentions
        .iter()
        .map(|(name, r)| (name.clone(), unrepr_intention(&mut world, r)))
        .collect();
    world.with_intentions(intentions);
    Ok(world)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The banked round-trip law (ARCHITECTURE): `deserialize(serialize(st)) ==
    /// st` for a state carrying facts, a cursor, and a seed. Exercised natively
    /// here over a seeded, mutated `empty` state; `conformance::persist_spec`
    /// carries the world-driven round trips and the proptest law.
    #[test]
    fn round_trips_a_seeded_mutated_state() {
        use crate::types::{Outcome, insert};
        let mut st = State::new();
        st.seed_die(1988).unwrap();
        st.perform_outcome(&insert("a.b.c")).unwrap();
        st.perform_outcome(&Outcome::Insert("m!x".to_owned())).unwrap();
        let reloaded = deserialize_state(&serialize_state(&st), State::new()).unwrap();
        assert_eq!(reloaded.labeled_facts(), st.labeled_facts());
        assert_eq!(reloaded.rng_seed(), st.rng_seed());
        assert_eq!(reloaded.cursor(), st.cursor());
    }

    #[test]
    fn a_foreign_version_tag_is_a_loud_unsupported_error() {
        let err = deserialize_state(r#"{"version":"prax-state v0","cursor":0}"#, State::new())
            .unwrap_err();
        assert!(
            matches!(err, PersistError::UnsupportedVersion { .. }),
            "got {err:?}"
        );
        assert!(err.to_string().contains("unsupported save format"));
    }

    #[test]
    fn empty_input_is_a_loud_malformed_error() {
        let err = deserialize_state("", State::new()).unwrap_err();
        assert!(matches!(err, PersistError::Malformed { .. }), "got {err:?}");
    }
}
