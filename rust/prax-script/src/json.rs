//! The serde format for scripts — the durable on-disk representation that
//! survives the cut-over.
//!
//! Frozen reference: `src/Prax/Script/Json.hs`. The schema is HAND-WRITTEN on
//! both sides (aeson's generic sum encoding is noisy): each condition and effect
//! is a single-key tagged object.
//!
//! # Byte-compatibility is REQUIRED, not optional
//!
//! `prax dump-play` prints `encodeScript playScript`, and PLAN.md's cut-over
//! criteria require the `dump-play` outputs to be EQUAL. Equal stdout is byte
//! equality, and it is achievable: aeson 2's `ordered-keymap` emits object keys
//! SORTED, and `examples/play.json` is measured byte-identical to the frozen
//! `dump-play` output.
//!
//! So the encoder builds a [`serde_json::Value`] by hand, one function per
//! frozen `object [...]`, including the conditional key omissions.
//! `serde_json::Map` is a `BTreeMap`, so sorted keys come free and compact
//! `to_string` is byte-comparable. `#[derive(Serialize)]` would emit
//! DECLARATION order and would NOT be byte-equal — which is why this crate
//! depends on `serde_json` and NOT on `serde`'s derive.
//!
//! **The `preserve_order` hazard** [D-I6]: `Map` being a `BTreeMap` holds only
//! while NO crate in the workspace enables `serde_json/preserve_order`. Cargo
//! unifies features across the graph, so one dependency turning it on flips
//! `Map` to an `IndexMap` for everybody and `dump-play` silently starts emitting
//! declaration order. The failure is silent and remote from its cause, so it
//! carries its own one-line pin below.
//!
//! **Escaping** [M-5]: aeson and `serde_json` both minimal-escape, and
//! `examples/play.json` is pure ASCII with no `"` or `\`, so byte-compatibility
//! is safe. The two do NOT agree on every codepoint, so byte-compatibility is a
//! property of THAT FILE, not of the two encoders in general.
//!
//! # Duplicate keys are decided before any probe runs
//!
//! aeson keeps the FIRST occurrence of a duplicated JSON key; `serde_json::Map`
//! insert-overwrites and keeps the LAST. Duplicate-key resolution is a property
//! of the OBJECT REPRESENTATION, settled before the ordered key probe ever runs,
//! so probing in the right order is necessary but not sufficient. The decoder
//! therefore parses into [`Jv`], whose `visit_map` inserts only into a vacant
//! slot — first-wins, matching aeson. Without it there exists a file that loads
//! as one script under the frozen engine and a different script under Rust,
//! silently: the exact "same bytes, different meaning" failure the `memories`
//! guard exists to prevent, one layer down.

use std::collections::BTreeMap;
use std::fmt;

use prax_core::query::{CalcOp, CmpOp, Condition};
use prax_core::types::{Outcome, Want};
use serde_json::{Map, Value};

use crate::script::{Beat, CastMember, Junction, Scene, Script};

// ---- the first-wins JSON value --------------------------------------------

/// A parsed JSON document with aeson's duplicate-key rule: within one object the
/// FIRST occurrence of a key wins.
///
/// This exists only because `serde_json::Value`'s own `Map` keeps the LAST, and
/// that difference is silent — see the module docs. Numbers are split into
/// integer and float because every number the script schema reads is an integer
/// (`utility`, `after`, `rounds`, `num`, `den`), and a float there is a decode
/// failure, not a truncation.
#[derive(Debug, Clone, PartialEq)]
enum Jv {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Arr(Vec<Jv>),
    Obj(BTreeMap<String, Jv>),
}

impl<'de> serde::Deserialize<'de> for Jv {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Jv, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = Jv;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any JSON value")
            }
            fn visit_unit<E>(self) -> Result<Jv, E> {
                Ok(Jv::Null)
            }
            fn visit_none<E>(self) -> Result<Jv, E> {
                Ok(Jv::Null)
            }
            fn visit_some<D: serde::Deserializer<'de>>(self, d: D) -> Result<Jv, D::Error> {
                serde::Deserialize::deserialize(d)
            }
            fn visit_bool<E>(self, b: bool) -> Result<Jv, E> {
                Ok(Jv::Bool(b))
            }
            fn visit_i64<E>(self, n: i64) -> Result<Jv, E> {
                Ok(Jv::Int(n))
            }
            fn visit_u64<E>(self, n: u64) -> Result<Jv, E> {
                Ok(i64::try_from(n).map_or_else(|_| Jv::Float(n as f64), Jv::Int))
            }
            fn visit_f64<E>(self, n: f64) -> Result<Jv, E> {
                Ok(Jv::Float(n))
            }
            fn visit_str<E>(self, s: &str) -> Result<Jv, E> {
                Ok(Jv::Str(s.to_owned()))
            }
            fn visit_string<E>(self, s: String) -> Result<Jv, E> {
                Ok(Jv::Str(s))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<Jv, A::Error> {
                let mut out = Vec::new();
                while let Some(v) = a.next_element()? {
                    out.push(v);
                }
                Ok(Jv::Arr(out))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<Jv, A::Error> {
                let mut out: BTreeMap<String, Jv> = BTreeMap::new();
                while let Some((k, v)) = a.next_entry::<String, Jv>()? {
                    // FIRST-WINS, matching aeson. `insert` here would be
                    // last-wins and would silently disagree with the frozen
                    // decoder on a duplicated key.
                    out.entry(k).or_insert(v);
                }
                Ok(Jv::Obj(out))
            }
        }
        d.deserialize_any(V)
    }
}

impl Jv {
    fn as_obj(&self) -> Option<&BTreeMap<String, Jv>> {
        match self {
            Jv::Obj(m) => Some(m),
            _ => None,
        }
    }
    fn as_str(&self) -> Option<&str> {
        match self {
            Jv::Str(s) => Some(s),
            _ => None,
        }
    }
    fn as_arr(&self) -> Option<&[Jv]> {
        match self {
            Jv::Arr(a) => Some(a),
            _ => None,
        }
    }
    fn as_int(&self) -> Option<i64> {
        match self {
            Jv::Int(n) => Some(*n),
            _ => None,
        }
    }
}

/// A decode failure, reported as a message the way `eitherDecode` does.
type Dec<T> = Result<T, String>;

/// `o .: "k"` — a REQUIRED key. Missing or `null` both fail (aeson's `.:` on an
/// explicit `null` is a type error for every type this schema uses).
fn req<'a>(o: &'a BTreeMap<String, Jv>, k: &str, what: &str) -> Dec<&'a Jv> {
    match o.get(k) {
        Some(Jv::Null) | None => Err(format!("{what}: missing required key {k:?}")),
        Some(v) => Ok(v),
    }
}

/// `o .:? "k"` — an OPTIONAL key. aeson maps an explicit `null` to `Nothing`
/// exactly as it maps a missing key (measured), so `null` reads as absent here
/// too. This is why the struct fields decode through `Option<T>` +
/// `unwrap_or_default` and NOT through `#[serde(default)]`, which cannot
/// deserialize `null` into a `Vec` [I-5].
fn opt<'a>(o: &'a BTreeMap<String, Jv>, k: &str) -> Option<&'a Jv> {
    match o.get(k) {
        Some(Jv::Null) | None => None,
        Some(v) => Some(v),
    }
}

fn req_str(o: &BTreeMap<String, Jv>, k: &str, what: &str) -> Dec<String> {
    req(o, k, what)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("{what}: key {k:?} is not a string"))
}

fn opt_str(o: &BTreeMap<String, Jv>, k: &str, what: &str) -> Dec<Option<String>> {
    opt(o, k)
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{what}: key {k:?} is not a string"))
        })
        .transpose()
}

fn opt_int(o: &BTreeMap<String, Jv>, k: &str, what: &str) -> Dec<Option<i64>> {
    opt(o, k)
        .map(|v| {
            v.as_int()
                .ok_or_else(|| format!("{what}: key {k:?} is not an integer"))
        })
        .transpose()
}

fn opt_list<T>(
    o: &BTreeMap<String, Jv>,
    k: &str,
    what: &str,
    f: impl Fn(&Jv) -> Dec<T>,
) -> Dec<Vec<T>> {
    let Some(v) = opt(o, k) else {
        return Ok(Vec::new());
    };
    v.as_arr()
        .ok_or_else(|| format!("{what}: key {k:?} is not an array"))?
        .iter()
        .map(f)
        .collect()
}

fn req_list<T>(
    o: &BTreeMap<String, Jv>,
    k: &str,
    what: &str,
    f: impl Fn(&Jv) -> Dec<T>,
) -> Dec<Vec<T>> {
    req(o, k, what)?
        .as_arr()
        .ok_or_else(|| format!("{what}: key {k:?} is not an array"))?
        .iter()
        .map(f)
        .collect()
}

// ---- enum tags -------------------------------------------------------------

fn cmp_tag(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Lt => "lt",
        CmpOp::Lte => "lte",
        CmpOp::Gt => "gt",
        CmpOp::Gte => "gte",
    }
}

fn parse_cmp(s: &str) -> Option<CmpOp> {
    match s {
        "lt" => Some(CmpOp::Lt),
        "lte" => Some(CmpOp::Lte),
        "gt" => Some(CmpOp::Gt),
        "gte" => Some(CmpOp::Gte),
        _ => None,
    }
}

fn calc_tag(op: CalcOp) -> &'static str {
    match op {
        CalcOp::Add => "add",
        CalcOp::Sub => "sub",
        CalcOp::Mul => "mul",
        CalcOp::Mod => "mod",
    }
}

fn parse_calc(s: &str) -> Option<CalcOp> {
    match s {
        "add" => Some(CalcOp::Add),
        "sub" => Some(CalcOp::Sub),
        "mul" => Some(CalcOp::Mul),
        "mod" => Some(CalcOp::Mod),
        _ => None,
    }
}

// ---- the encoder -----------------------------------------------------------

fn obj(pairs: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert(k.to_owned(), v);
    }
    Value::Object(m)
}

fn cond_value(c: &Condition) -> Value {
    match c {
        Condition::Match(s) => obj([("match", Value::from(s.as_str()))]),
        Condition::Not(s) => obj([("not", Value::from(s.as_str()))]),
        Condition::Eq(a, b) => obj([("eq", Value::from(vec![a.clone(), b.clone()]))]),
        Condition::Neq(a, b) => obj([("neq", Value::from(vec![a.clone(), b.clone()]))]),
        Condition::Cmp(op, a, b) => obj([(
            "cmp",
            Value::from(vec![cmp_tag(*op).to_owned(), a.clone(), b.clone()]),
        )]),
        Condition::Calc(r, op, a, b) => obj([(
            "calc",
            Value::from(vec![
                r.clone(),
                calc_tag(*op).to_owned(),
                a.clone(),
                b.clone(),
            ]),
        )]),
        Condition::Count(r, s) => obj([("count", Value::from(vec![r.clone(), s.clone()]))]),
        Condition::Subquery { set, find, where_ } => obj([(
            "subquery",
            obj([
                ("set", Value::from(set.as_str())),
                ("find", Value::from(find.clone())),
                ("where", conds_value(where_)),
            ]),
        )]),
        Condition::Or(clauses) => obj([(
            "or",
            Value::Array(clauses.iter().map(|c| conds_value(c)).collect()),
        )]),
        Condition::Absent(cs) => obj([("absent", conds_value(cs))]),
        Condition::Exists(cs) => obj([("exists", conds_value(cs))]),
    }
}

fn conds_value(cs: &[Condition]) -> Value {
    Value::Array(cs.iter().map(cond_value).collect())
}

fn outcome_value(o: &Outcome) -> Value {
    match o {
        Outcome::Insert(s) => obj([("insert", Value::from(s.as_str()))]),
        Outcome::Delete(s) => obj([("delete", Value::from(s.as_str()))]),
        Outcome::InsertFor(n, s) => obj([(
            "insertFor",
            obj([
                ("rounds", Value::from(*n)),
                ("sentence", Value::from(s.as_str())),
            ]),
        )]),
        Outcome::Call(f, args) => obj([(
            "call",
            obj([
                ("fn", Value::from(f.as_str())),
                ("args", Value::from(args.clone())),
            ]),
        )]),
        Outcome::ForEach(cs, os) => obj([(
            "forEach",
            obj([("when", conds_value(cs)), ("do", outs_value(os))]),
        )]),
        Outcome::Roll(num, den, cs, os) => obj([(
            "roll",
            obj([
                ("num", Value::from(*num)),
                ("den", Value::from(*den)),
                ("when", conds_value(cs)),
                ("do", outs_value(os)),
            ]),
        )]),
    }
}

fn outs_value(os: &[Outcome]) -> Value {
    Value::Array(os.iter().map(outcome_value).collect())
}

fn want_value(w: &Want) -> Value {
    obj([
        ("when", conds_value(&w.when)),
        ("utility", Value::from(w.utility)),
    ])
}

fn cast_value(c: &CastMember) -> Value {
    obj([
        ("name", Value::from(c.name.as_str())),
        ("playable", Value::from(c.playable)),
        (
            "desires",
            Value::Array(c.desires.iter().map(want_value).collect()),
        ),
        ("traits", Value::from(c.traits.clone())),
    ])
}

fn beat_value(b: &Beat) -> Value {
    let mut m = Map::new();
    m.insert("label".to_owned(), Value::from(b.label.as_str()));
    m.insert("when".to_owned(), conds_value(&b.when));
    m.insert("effects".to_owned(), outs_value(&b.effects));
    // conditional key: a speaker-less beat emits NO "speaker" key at all
    if let Some(s) = &b.speaker {
        m.insert("speaker".to_owned(), Value::from(s.as_str()));
    }
    Value::Object(m)
}

fn junction_value(j: &Junction) -> Value {
    let mut m = Map::new();
    m.insert("name".to_owned(), Value::from(j.name.as_str()));
    m.insert("when".to_owned(), conds_value(&j.when));
    if let Some(t) = &j.to {
        m.insert("to".to_owned(), Value::from(t.as_str()));
    }
    if let Some(n) = j.after {
        m.insert("after".to_owned(), Value::from(n));
    }
    Value::Object(m)
}

fn scene_value(s: &Scene) -> Value {
    obj([
        ("id", Value::from(s.id.as_str())),
        ("opening", Value::from(s.opening.as_str())),
        ("setup", outs_value(&s.setup)),
        (
            "beats",
            Value::Array(s.beats.iter().map(beat_value).collect()),
        ),
        (
            "junctions",
            Value::Array(s.junctions.iter().map(junction_value).collect()),
        ),
    ])
}

fn script_value(scr: &Script) -> Value {
    obj([
        ("start", Value::from(scr.start.as_str())),
        (
            "cast",
            Value::Array(scr.cast.iter().map(cast_value).collect()),
        ),
        (
            "scenes",
            Value::Array(scr.scenes.iter().map(scene_value).collect()),
        ),
    ])
}

/// Serialize a play-script to compact JSON with SORTED keys
/// (`Prax.Script.Json.encodeScript`).
///
/// NOTE the trailing newline is NOT here [I-4]: the frozen `dump-play` arm is
/// `BLC.putStrLn (encodeScript …)`, so the CLI adds it. `examples/play.json` is
/// 2122 bytes and `encodeScript` produces 2121.
pub fn encode_script(scr: &Script) -> String {
    serde_json::to_string(&script_value(scr)).expect("a Value built here always serializes")
}

// ---- the decoder -----------------------------------------------------------

/// The ordered key probe with FALL-THROUGH, mirroring aeson's `<|>` chain.
///
/// The chain is `match, not, eq, neq, cmp, calc, count, subquery, or, absent,
/// exists`, and a present-but-MALFORMED payload FAILS that alternative and falls
/// through to the next — aeson's `Parser` `<|>` recovers from `fail`, from a
/// `.:` type mismatch and from a `do`-block pattern-match failure alike. So this
/// is an ordered probe returning `Option`, NOT a match on "which key is
/// present": measured on the frozen decoder,
/// `{"match": 1, "not": "x"}` decodes to `Not "x"`, and
/// `{"not": "y", "match": "x"}` decodes to `Match "x"` because the CHAIN order
/// wins over the document order.
fn decode_condition(v: &Jv) -> Dec<Condition> {
    let o = v
        .as_obj()
        .ok_or_else(|| "Condition: expected an object".to_owned())?;

    let two = |k: &str| -> Option<(String, String)> {
        match o.get(k)?.as_arr()? {
            [a, b] => Some((a.as_str()?.to_owned(), b.as_str()?.to_owned())),
            _ => None,
        }
    };
    let conds = |k: &str| -> Option<Vec<Condition>> {
        o.get(k)?
            .as_arr()?
            .iter()
            .map(decode_condition)
            .collect::<Dec<_>>()
            .ok()
    };

    if let Some(s) = o.get("match").and_then(Jv::as_str) {
        return Ok(Condition::Match(s.to_owned()));
    }
    if let Some(s) = o.get("not").and_then(Jv::as_str) {
        return Ok(Condition::Not(s.to_owned()));
    }
    if let Some((a, b)) = two("eq") {
        return Ok(Condition::Eq(a, b));
    }
    if let Some((a, b)) = two("neq") {
        return Ok(Condition::Neq(a, b));
    }
    if let Some([op, a, b]) = o.get("cmp").and_then(Jv::as_arr)
        && let (Some(op), Some(a), Some(b)) = (op.as_str(), a.as_str(), b.as_str())
        && let Some(op) = parse_cmp(op)
    {
        return Ok(Condition::Cmp(op, a.to_owned(), b.to_owned()));
    }
    if let Some([r, op, a, b]) = o.get("calc").and_then(Jv::as_arr)
        && let (Some(r), Some(op), Some(a), Some(b)) =
            (r.as_str(), op.as_str(), a.as_str(), b.as_str())
        && let Some(op) = parse_calc(op)
    {
        return Ok(Condition::Calc(r.to_owned(), op, a.to_owned(), b.to_owned()));
    }
    if let Some((a, b)) = two("count") {
        return Ok(Condition::Count(a, b));
    }
    if let Some(sub) = o.get("subquery").and_then(Jv::as_obj)
        && let (Ok(set), Ok(find), Ok(where_)) = (
            req_str(sub, "set", "subquery"),
            req_list(sub, "find", "subquery", |v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| "subquery: a find entry is not a string".to_owned())
            }),
            req_list(sub, "where", "subquery", decode_condition),
        )
    {
        return Ok(Condition::Subquery { set, find, where_ });
    }
    if let Some(clauses) = o.get("or").and_then(Jv::as_arr)
        && let Some(cls) = clauses
            .iter()
            .map(|c| {
                c.as_arr()
                    .and_then(|xs| xs.iter().map(decode_condition).collect::<Dec<_>>().ok())
            })
            .collect::<Option<Vec<Vec<Condition>>>>()
    {
        return Ok(Condition::Or(cls));
    }
    if let Some(cs) = conds("absent") {
        return Ok(Condition::Absent(cs));
    }
    if let Some(cs) = conds("exists") {
        return Ok(Condition::Exists(cs));
    }
    Err(format!(
        "Condition: no alternative in the probe chain (match, not, eq, neq, cmp, \
         calc, count, subquery, or, absent, exists) accepted {v:?}"
    ))
}

/// The `Outcome` probe chain: `insert, delete, insertFor, call, forEach, roll`,
/// with the same fall-through discipline as [`decode_condition`].
fn decode_outcome(v: &Jv) -> Dec<Outcome> {
    let o = v
        .as_obj()
        .ok_or_else(|| "Outcome: expected an object".to_owned())?;

    if let Some(s) = o.get("insert").and_then(Jv::as_str) {
        return Ok(Outcome::Insert(s.to_owned()));
    }
    if let Some(s) = o.get("delete").and_then(Jv::as_str) {
        return Ok(Outcome::Delete(s.to_owned()));
    }
    if let Some(f) = o.get("insertFor").and_then(Jv::as_obj)
        && let (Some(n), Some(s)) = (
            f.get("rounds").and_then(Jv::as_int),
            f.get("sentence").and_then(Jv::as_str),
        )
    {
        return Ok(Outcome::InsertFor(n, s.to_owned()));
    }
    if let Some(c) = o.get("call").and_then(Jv::as_obj)
        && let Some(name) = c.get("fn").and_then(Jv::as_str)
        && let Some(args) = c.get("args").and_then(Jv::as_arr)
        && let Some(args) = args
            .iter()
            .map(|a| a.as_str().map(str::to_owned))
            .collect::<Option<Vec<String>>>()
    {
        return Ok(Outcome::Call(name.to_owned(), args));
    }
    if let Some(f) = o.get("forEach").and_then(Jv::as_obj)
        && let (Ok(when), Ok(then)) = (
            req_list(f, "when", "forEach", decode_condition),
            req_list(f, "do", "forEach", decode_outcome),
        )
    {
        return Ok(Outcome::ForEach(when, then));
    }
    if let Some(r) = o.get("roll").and_then(Jv::as_obj)
        && let (Some(num), Some(den)) = (
            r.get("num").and_then(Jv::as_int),
            r.get("den").and_then(Jv::as_int),
        )
        && let (Ok(when), Ok(then)) = (
            req_list(r, "when", "roll", decode_condition),
            req_list(r, "do", "roll", decode_outcome),
        )
    {
        return Ok(Outcome::Roll(num, den, when, then));
    }
    Err(format!(
        "Outcome: no alternative in the probe chain (insert, delete, insertFor, \
         call, forEach, roll) accepted {v:?}"
    ))
}

fn decode_want(v: &Jv) -> Dec<Want> {
    let o = v
        .as_obj()
        .ok_or_else(|| "Want: expected an object".to_owned())?;
    let when = req_list(o, "when", "Want", decode_condition)?;
    let utility = req(o, "utility", "Want")?
        .as_int()
        .ok_or_else(|| "Want: \"utility\" is not an integer".to_owned())?;
    // The stated i32 bound on authored weights (ARCHITECTURE, Scores). Loud at
    // the door rather than a silent narrowing.
    let utility = i32::try_from(utility)
        .map_err(|_| format!("Want: utility {utility} does not fit the engine's i32 weight"))?;
    Ok(Want::new(when, utility))
}

fn decode_cast_member(v: &Jv) -> Dec<CastMember> {
    let o = v
        .as_obj()
        .ok_or_else(|| "CastMember: expected an object".to_owned())?;
    Ok(CastMember {
        name: req_str(o, "name", "CastMember")?,
        playable: match opt(o, "playable") {
            Some(Jv::Bool(b)) => *b,
            Some(_) => return Err("CastMember: \"playable\" is not a boolean".to_owned()),
            None => false,
        },
        desires: opt_list(o, "desires", "CastMember", decode_want)?,
        traits: opt_list(o, "traits", "CastMember", |v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or_else(|| "CastMember: a trait is not a string".to_owned())
        })?,
    })
}

fn decode_beat(v: &Jv) -> Dec<Beat> {
    let o = v
        .as_obj()
        .ok_or_else(|| "Beat: expected an object".to_owned())?;
    Ok(Beat {
        label: req_str(o, "label", "Beat")?,
        speaker: opt_str(o, "speaker", "Beat")?,
        when: opt_list(o, "when", "Beat", decode_condition)?,
        effects: opt_list(o, "effects", "Beat", decode_outcome)?,
    })
}

fn decode_junction(v: &Jv) -> Dec<Junction> {
    let o = v
        .as_obj()
        .ok_or_else(|| "Junction: expected an object".to_owned())?;
    Ok(Junction {
        name: req_str(o, "name", "Junction")?,
        to: opt_str(o, "to", "Junction")?,
        when: opt_list(o, "when", "Junction", decode_condition)?,
        after: opt_int(o, "after", "Junction")?,
    })
}

/// The `"memories"` guard — LOUD, not silent.
///
/// A scene JSON carrying the pre-v46 memory field must be rejected: the feature
/// was deleted end-to-end, and the decoder's ignore-unknown-keys default would
/// otherwise drop the content quietly. `Prax.Persist` took the same "same bytes,
/// different meaning" stance with its v3 format bump.
///
/// **DIVERGENCE (DIV-5), deliberate and recorded.** The frozen guard is
/// `isJust <$> (o .:? "memories")`, and `.:?` maps an explicit `null` to
/// `Nothing` exactly as it maps a missing key — so the frozen guard does NOT
/// fire on `{"id":"a","memories":null}`. Measured on the frozen decoder: that
/// scene decodes clean, `Right (Scene {sceneId = "a", …})`. That is a hole in a
/// guard whose entire purpose is loudness, at the JSON spelling an author is
/// most likely to leave behind when half-deleting a field. Under the program's
/// ruling (Haskell bugs are never reproduced) the Rust guard fires on the KEY
/// being present, `null` included. See `docs/rewrite/DIVERGENCES.md` DIV-5.
fn check_no_memories(o: &BTreeMap<String, Jv>) -> Dec<()> {
    if o.contains_key("memories") {
        return Err("Prax.Script.Json: Scene's \"memories\" field is no longer \
                    supported -- the memory feature was removed (spec v46); \
                    author scene content via setup/beats/junctions instead"
            .to_owned());
    }
    Ok(())
}

fn decode_scene(v: &Jv) -> Dec<Scene> {
    let o = v
        .as_obj()
        .ok_or_else(|| "Scene: expected an object".to_owned())?;
    check_no_memories(o)?;
    Ok(Scene {
        id: req_str(o, "id", "Scene")?,
        opening: opt_str(o, "opening", "Scene")?.unwrap_or_default(),
        setup: opt_list(o, "setup", "Scene", decode_outcome)?,
        beats: opt_list(o, "beats", "Scene", decode_beat)?,
        junctions: opt_list(o, "junctions", "Scene", decode_junction)?,
    })
}

/// Parse a play-script from JSON, reporting the error on failure
/// (`Prax.Script.Json.decodeScript`).
///
/// UNKNOWN KEYS STAY IGNORED, except `memories`. A `deny_unknown_fields`-style
/// contract would be a STRICTER contract smuggled in as hygiene: it would reject
/// forward-compatible files the frozen accepts. If we ever want strictness it is
/// a fork question, not a decoder convenience.
///
/// # Errors
/// The decode message, as a string.
pub fn decode_script(bytes: &[u8]) -> Result<Script, String> {
    let v: Jv = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let o = v
        .as_obj()
        .ok_or_else(|| "Script: expected an object".to_owned())?;
    Ok(Script {
        cast: req_list(o, "cast", "Script", decode_cast_member)?,
        scenes: req_list(o, "scenes", "Script", decode_scene)?,
        start: req_str(o, "start", "Script")?,
    })
}

/// Load and parse a play-script from a `.json` file
/// (`Prax.Script.Json.loadScript`). Its consumer is the S9 CLI's
/// `play <file>.json` arm.
///
/// `Prax.Script.Json.saveScript` is NOT ported: it is exported and called by
/// nobody in the frozen tree, and the house rule is no dead code. A caller that
/// wants it writes `fs::write(path, encode_script(scr))`.
///
/// # Errors
/// The IO error or the decode message, as a string.
pub fn load_script(path: &std::path::Path) -> Result<Script, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    decode_script(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script;
    use prax_core::query::{absent, matches, neq, not_};
    use prax_core::types::insert;

    /// NATIVE PIN [D-I6] — the `serde_json` `preserve_order` feature-unification
    /// hazard. [`encode_script`]'s byte-compatibility rests entirely on
    /// `serde_json::Map` being a `BTreeMap`, which holds only while NO crate in
    /// the workspace enables `serde_json/preserve_order`. Cargo unifies features
    /// across the whole graph, so ONE dependency turning it on flips `Map` to an
    /// `IndexMap` for everybody and `dump-play` silently starts emitting
    /// declaration order. Silent, and remote from its cause.
    ///
    /// REDDENS UNDER: adding `features = ["preserve_order"]` to any `serde_json`
    /// dependency anywhere in the workspace.
    #[test]
    fn serde_json_objects_sort_their_keys() {
        assert_eq!(
            serde_json::to_string(&serde_json::json!({"b": 1, "a": 2})).unwrap(),
            r#"{"a":2,"b":1}"#,
            "if this fails, serde_json/preserve_order is enabled somewhere in \
             the workspace and every encoder byte-compatibility claim is void"
        );
    }

    /// Decode one condition through the whole document path, so the probe is
    /// exercised where a file actually reaches it.
    fn cond(json: &str) -> Condition {
        let raw = format!(
            r#"{{"start":"a","cast":[],"scenes":[{{"id":"a","junctions":[
               {{"name":"j","when":[{json}]}}]}}]}}"#
        );
        decode_script(raw.as_bytes()).expect("decodes").scenes[0].junctions[0].when[0].clone()
    }

    /// NATIVE PIN [S-C2] — duplicate JSON keys resolve FIRST-WINS, as aeson
    /// does. `serde_json::Value` keeps the LAST, so a decoder built on it would
    /// read a different script from the same bytes than the frozen engine, with
    /// no diagnostic anywhere. Measured on the frozen decoder:
    /// `{"match":"first","match":"second"}` → `Match "first"`;
    /// `{"not":"n1","not":"n2"}` → `Not "n1"`.
    ///
    /// REDDENS UNDER: replacing `Jv`'s `entry().or_insert()` with `insert()`, or
    /// swapping `Jv` for `serde_json::Value` in `decode_script`.
    #[test]
    fn a_duplicated_json_key_resolves_to_the_first_occurrence() {
        assert_eq!(
            cond(r#"{"match":"first","match":"second"}"#),
            matches("first"),
            "aeson keeps the FIRST duplicate key; serde_json's Map keeps the last"
        );
        assert_eq!(cond(r#"{"not":"n1","not":"n2"}"#), not_("n1"));
    }

    /// NATIVE PIN [R8] — the ordered key probe with FALL-THROUGH is SEMANTICS,
    /// not a lookup convenience. Nothing frozen pins it; all four cases below
    /// were measured on the frozen decoder.
    ///
    /// REDDENS UNDER: matching on "which key is present" instead of probing in
    /// chain order (case 2), or returning an error on a malformed payload
    /// instead of falling through (cases 1, 3 and 4).
    #[test]
    fn the_condition_probe_runs_in_chain_order_and_falls_through_on_malformed() {
        // 1. the first key is present but MALFORMED: fall through to `not`
        assert_eq!(cond(r#"{"match": 1, "not": "x"}"#), not_("x"));
        // 2. both valid, document order != chain order: the CHAIN wins
        assert_eq!(cond(r#"{"not": "y", "match": "x"}"#), matches("x"));
        // 3. a malformed `cmp` (two elements where three are needed) falls
        //    through to `count`
        assert_eq!(
            cond(r#"{"cmp": ["gt","a"], "count": ["R","s"]}"#),
            Condition::Count("R".to_owned(), "s".to_owned())
        );
        // 4. an UNKNOWN comparison operator also falls through rather than
        //    reporting a loud unknown-operator error [M-2]: the frozen
        //    `parseCmp` failure is swallowed by the `<|>` chain, so "improving"
        //    the message here would be a stricter contract
        assert_eq!(cond(r#"{"cmp": ["gtt","a","b"], "match": "m"}"#), matches("m"));
    }

    /// The `Outcome` chain's twin of the case above, measured on the frozen
    /// decoder: a malformed `insert` falls through to `call`. NATIVE PIN — same
    /// reasoning, same reddening mutation.
    #[test]
    fn the_outcome_probe_falls_through_a_malformed_payload() {
        let raw = br#"{"start":"a","cast":[],"scenes":[{"id":"a","setup":[
            {"insert": 5, "call": {"fn":"f","args":["x"]}}]}]}"#;
        assert_eq!(
            decode_script(raw).expect("decodes").scenes[0].setup,
            vec![Outcome::Call("f".to_owned(), vec!["x".to_owned()])]
        );
    }

    /// NATIVE PIN [I-5] — an explicit `null` reads as an ABSENT optional field,
    /// for every optional field in the schema. Measured on the frozen decoder.
    /// The natural-looking `#[serde(default)] beats: Vec<Beat>` would FAIL on
    /// `"beats": null`, which is the single most likely place a faithful-LOOKING
    /// port diverges — and it is invisible to the round-trip pin, because the
    /// encoder never emits `null`.
    ///
    /// REDDENS UNDER: making `opt` return `Some(&Jv::Null)` instead of `None`.
    #[test]
    fn an_explicit_null_reads_as_an_absent_optional_field() {
        let raw = br#"{"start":"a","cast":[{"name":"p","playable":null,"desires":null,"traits":null}],
            "scenes":[{"id":"a","beats":null,"junctions":null,"setup":null,"opening":null}]}"#;
        let scr = decode_script(raw).expect("explicit nulls decode as absent");
        assert!(!scr.cast[0].playable);
        assert!(scr.cast[0].desires.is_empty() && scr.cast[0].traits.is_empty());
        let s = &scr.scenes[0];
        assert_eq!(s.opening, "");
        assert!(s.setup.is_empty() && s.beats.is_empty() && s.junctions.is_empty());
    }

    /// NATIVE PIN — unknown keys stay IGNORED. The frozen `withObject` ignores
    /// them (measured), so a stricter Rust decoder would reject
    /// forward-compatible files the frozen accepts. `memories` is the one
    /// exception, and it has its own frozen-labelled pin in the conformance
    /// crate.
    ///
    /// REDDENS UNDER: adding a `deny_unknown_fields`-style sweep.
    #[test]
    fn unknown_keys_are_ignored_except_memories() {
        let raw = br#"{"start":"a","cast":[],"scenes":[{"id":"a","futureField":42}],"extra":1}"#;
        assert!(decode_script(raw).is_ok());
    }

    /// Every constructor of both languages survives the round trip. This is the
    /// encoder/decoder TOTALITY net: a constructor missing from either side
    /// fails here, rather than at whichever world happens to use it first.
    #[test]
    fn every_condition_and_outcome_constructor_round_trips() {
        let conds = vec![
            matches("a.b!c"),
            not_("d"),
            Condition::Eq("X".to_owned(), "y".to_owned()),
            neq("X", "z"),
            prax_core::query::cmp(CmpOp::Gte, "N", "3"),
            prax_core::query::calc("R", CalcOp::Mod, "17", "5"),
            prax_core::query::count("N", "S"),
            prax_core::query::subquery("S", vec!["C".to_owned()], vec![matches("at.C!P")]),
            prax_core::query::or_(vec![vec![matches("p")], vec![not_("q")]]),
            absent(vec![matches("e")]),
            prax_core::query::exists(vec![matches("f")]),
        ];
        let outs = vec![
            insert("i"),
            prax_core::types::delete("d"),
            Outcome::InsertFor(3, "mood!a".to_owned()),
            prax_core::types::call("f", vec!["x".to_owned()]),
            prax_core::types::for_each(vec![matches("at.W!P")], vec![insert("W.saw")]),
            Outcome::Roll(1, 4, vec![matches("t.T")], vec![insert("T.angry")]),
        ];
        let scr = Script::new("s")
            .cast([script::wanting(
                script::player("p"),
                [Want::new(conds.clone(), 100)],
            )])
            .scenes([script::scene("s")
                .opening("o")
                .setup(outs.clone())
                .beats([script::quip("p", "l", conds.clone(), outs.clone())])
                .junctions([script::goto("g", "s", conds), script::timeout("t", 5)])]);
        assert_eq!(decode_script(encode_script(&scr).as_bytes()), Ok(scr));
    }

    /// NATIVE PIN — the conditional key omissions the frozen encoder makes. A
    /// speaker-less beat emits NO `"speaker"` key and an untimed junction emits
    /// NO `"after"` key; emitting `null` instead would still round-trip (the
    /// decoder reads `null` as absent) but would NOT be byte-compatible with
    /// `examples/play.json`, and byte-compatibility is a cut-over criterion.
    ///
    /// REDDENS UNDER: always inserting the key with a `null` value.
    #[test]
    fn the_encoder_omits_absent_speaker_and_after_keys() {
        let scr = Script::new("s").scenes([script::scene("s")
            .beats([script::beat("l", vec![], vec![])])
            .junctions([script::ending("e", vec![])])]);
        assert_eq!(
            encode_script(&scr),
            r#"{"cast":[],"scenes":[{"beats":[{"effects":[],"label":"l","when":[]}],"id":"s","junctions":[{"name":"e","when":[]}],"opening":"","setup":[]}],"start":"s"}"#
        );
    }
}
