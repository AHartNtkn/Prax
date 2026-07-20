//! The adjudicated-divergence register (`conformance/ADJUDICATED.json`).
//!
//! The program's ruling is that the Rust must be RIGHT, not Haskell-equal: a
//! frozen bug is never reproduced, so some shipped traces will legitimately
//! differ forever. The register is how the matrix reads "clean modulo
//! adjudicated fixes" WITHOUT ever drowning fresh signal.
//!
//! Three load-bearing laws, the second replaced by the soundness panel:
//!
//! 1. **Per-field-difference suppression, never per-record.** A record is
//!    `clean-mod-adjudicated` only if EVERY differing field is covered, and for
//!    a path-bearing field every differing PATH matches a pattern. One fresh
//!    path alongside an adjudicated one ⇒ DIVERGENT.
//! 2. **[S-I1] Marked continuation under a NON-GROWTH invariant** (replacing
//!    "an adjudicated difference truncates the comparison"). Truncation was too
//!    strong: a stable adjudicated derived-fact difference would truncate every
//!    walk at its first record while reporting non-DIVERGENT — i.e. suppress the
//!    entire run. Instead the comparison CONTINUES, the record is marked, and
//!    the suppressed difference set must not GROW. If it grows, the record is
//!    DIVERGENT — which is law 1's escalation, the thing truncation was
//!    presuming.
//! 3. **The anti-drift gate.** A conformance test asserts a bijection between
//!    ADJUDICATED ids and `DIVERGENCES.md`'s `## DIV-n` headings that declare a
//!    suppression. Neither register grows without the other.
//!
//! **The register ships EMPTY.** DIV-1 and DIV-2 need no suppression (neither
//! changes a shipped trace), and no-op entries would lie about the mechanism.
//! What ships instead is MUTATION EVIDENCE, in the GateSpec tradition: fixture
//! registers and synthetic divergent record pairs proving the mechanism
//! DISCRIMINATES in four directions. A suppression mechanism nobody has seen
//! discriminate is not a mechanism.

use crate::classify::Class;
use crate::diff::RecordDiff;
use serde_json::Value;
use std::collections::BTreeSet;

/// One adjudicated divergence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The `DIV-n` id this suppression belongs to (the bijection's key).
    pub id: String,
    /// The world it applies to.
    pub world: String,
    /// `trace` or `randtrace`.
    pub mode: String,
    /// The class the divergence must classify as; a class mismatch defeats it.
    pub class: String,
    /// The record fields it covers.
    pub fields: Vec<String>,
    /// Path patterns for path-bearing fields (`*` = one segment, `**` = any
    /// suffix).
    pub paths: Vec<String>,
    /// The seeds it applies to; empty = every seed.
    pub seeds: Vec<i64>,
    /// The first record ordinal it may apply from.
    pub from_turn: i64,
    /// Why this is adjudicated (prose, for the report).
    pub note: String,
}

/// The parsed register.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Register {
    /// The entries, in file order.
    pub entries: Vec<Entry>,
}

/// WHERE a divergent record sits: the coordinates an entry is matched against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Situation {
    /// The world.
    pub world: String,
    /// `trace` or `randtrace`.
    pub mode: String,
    /// The walk seed, when there is one.
    pub seed: Option<i64>,
    /// The record ORDINAL [M2].
    pub ordinal: i64,
}

/// What the register says about one divergent record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ruling {
    /// Not covered — a real divergence.
    Divergent(String),
    /// Every differing field and path is covered by these entries, and the
    /// suppressed difference set has not grown.
    Suppressed {
        /// The entry ids that covered it.
        ids: Vec<String>,
        /// The suppressed difference set at this record (fields ⊎ paths), which
        /// later records may not exceed.
        marks: BTreeSet<String>,
    },
}

impl Register {
    /// Parse `conformance/ADJUDICATED.json`: `{"format": 1, "entries": [...]}`.
    ///
    /// # Errors
    /// On malformed JSON or a malformed entry — an unreadable register is never
    /// treated as an empty one, because that would silently un-suppress (or, in
    /// the other direction, silently suppress nothing while claiming to).
    pub fn parse(v: &Value) -> Result<Register, String> {
        let obj = v.as_object().ok_or("ADJUDICATED.json is not an object")?;
        match obj.get("format").and_then(Value::as_i64) {
            Some(1) => {}
            other => return Err(format!("ADJUDICATED.json format {other:?}, expected 1")),
        }
        let arr = obj
            .get("entries")
            .and_then(Value::as_array)
            .ok_or("ADJUDICATED.json has no `entries` array")?;
        let mut entries = Vec::new();
        for (i, e) in arr.iter().enumerate() {
            entries.push(Entry {
                id: str_field(e, "id", i)?,
                world: str_field(e, "world", i)?,
                mode: str_field(e, "mode", i)?,
                class: str_field(e, "class", i)?,
                fields: str_list(e, "fields", i)?,
                paths: str_list(e, "paths", i)?,
                seeds: e
                    .get("seeds")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(Value::as_i64).collect())
                    .unwrap_or_default(),
                from_turn: e.get("from_turn").and_then(Value::as_i64).unwrap_or(0),
                note: str_field(e, "note", i)?,
            });
        }
        Ok(Register { entries })
    }

    /// Load the register from a file.
    ///
    /// # Errors
    /// If the file cannot be read or parsed.
    pub fn load(path: &std::path::Path) -> Result<Register, String> {
        let body = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let v: Value = serde_json::from_str(&body)
            .map_err(|e| format!("{} is not JSON: {e}", path.display()))?;
        Register::parse(&v)
    }

    /// Adjudicate one divergent record.
    ///
    /// `prior_marks` is the suppressed difference set from the FIRST suppressed
    /// record of this run; a record whose own marks exceed it has GROWN and is
    /// DIVERGENT ([S-I1]).
    pub fn adjudicate(
        &self,
        at: &Situation,
        class: Class,
        d: &RecordDiff,
        prior_marks: Option<&BTreeSet<String>>,
    ) -> Ruling {
        let Situation {
            world,
            mode,
            seed,
            ordinal,
        } = at;
        let (world, mode, seed, ordinal) = (world.as_str(), mode.as_str(), *seed, *ordinal);
        let applicable: Vec<&Entry> = self
            .entries
            .iter()
            .filter(|e| {
                e.world == world
                    && e.mode == mode
                    && ordinal >= e.from_turn
                    && (e.seeds.is_empty() || seed.is_none_or(|s| e.seeds.contains(&s)))
            })
            .collect();
        if applicable.is_empty() {
            return Ruling::Divergent(format!(
                "no register entry applies to ({world}, {mode}, seed {seed:?}, record {ordinal})"
            ));
        }
        // A class mismatch defeats suppression: an entry adjudicates a specific
        // KIND of difference, and the same fields differing for a different
        // reason is fresh signal.
        let class_ok: Vec<&&Entry> = applicable
            .iter()
            .filter(|e| e.class == class.as_str())
            .collect();
        if class_ok.is_empty() {
            return Ruling::Divergent(format!(
                "the divergence classifies as {} but the applicable register entr{} adjudicate{} \
                 {:?} — a class mismatch defeats suppression",
                class.as_str(),
                if applicable.len() == 1 { "y" } else { "ies" },
                if applicable.len() == 1 { "s" } else { "" },
                applicable.iter().map(|e| &e.class).collect::<Vec<_>>()
            ));
        }
        // LAW 1: per-field, and for path-bearing fields per-PATH.
        let mut ids = BTreeSet::new();
        let mut marks = BTreeSet::new();
        for fd in &d.fields {
            let covering: Vec<&&&Entry> = class_ok
                .iter()
                .filter(|e| e.fields.iter().any(|f| f == &fd.field))
                .collect();
            if covering.is_empty() {
                return Ruling::Divergent(format!(
                    "field `{}` is not covered by any applicable register entry — one fresh field \
                     alongside an adjudicated one is DIVERGENT (law 1)",
                    fd.field
                ));
            }
            marks.insert(format!("field:{}", fd.field));
            for e in &covering {
                ids.insert(e.id.clone());
            }
            if let Some(pd) = &fd.paths {
                for p in pd.all_paths() {
                    let matched = covering
                        .iter()
                        .any(|e| e.paths.iter().any(|pat| path_matches(pat, &p)));
                    if !matched {
                        return Ruling::Divergent(format!(
                            "path `{p}` under field `{}` matches no adjudicated pattern — one \
                             fresh path alongside an adjudicated one is DIVERGENT (law 1)",
                            fd.field
                        ));
                    }
                    marks.insert(format!("path:{p}"));
                }
            }
        }
        // LAW 2 [S-I1]: marked continuation under a non-growth invariant.
        if let Some(prior) = prior_marks {
            let grew: Vec<&String> = marks.difference(prior).collect();
            if !grew.is_empty() {
                return Ruling::Divergent(format!(
                    "the suppressed difference set GREW at record {ordinal} (new: {grew:?}). An \
                     adjudicated difference may persist, but a difference that spreads is a fresh \
                     divergence, not the adjudicated one ([S-I1] non-growth invariant)"
                ));
            }
        }
        Ruling::Suppressed {
            ids: ids.into_iter().collect(),
            marks,
        }
    }
}

fn str_field(e: &Value, k: &str, i: usize) -> Result<String, String> {
    e.get(k)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("ADJUDICATED.json entry {i} has no string `{k}`"))
}

fn str_list(e: &Value, k: &str, i: usize) -> Result<Vec<String>, String> {
    match e.get(k) {
        None => Ok(Vec::new()),
        Some(v) => v
            .as_array()
            .ok_or_else(|| format!("ADJUDICATED.json entry {i}: `{k}` is not an array"))?
            .iter()
            .map(|x| {
                x.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| format!("ADJUDICATED.json entry {i}: `{k}` holds a non-string"))
            })
            .collect(),
    }
}

/// Segment-wise path matching: `*` matches exactly ONE segment, a trailing `**`
/// matches any (possibly empty) suffix, everything else is literal. Operators
/// are normalized away first — a pattern adjudicates a PATH, and a relabeling of
/// that path is a different bug that must not slip through the same entry.
pub fn path_matches(pattern: &str, path: &str) -> bool {
    let pat: Vec<&str> = pattern.split('.').collect();
    let norm = crate::diff::segment_path(path);
    let segs: Vec<&str> = norm.split('.').collect();
    let mut i = 0;
    while i < pat.len() {
        if pat[i] == "**" {
            return i + 1 == pat.len();
        }
        if i >= segs.len() {
            return false;
        }
        if pat[i] != "*" && pat[i] != segs[i] {
            return false;
        }
        i += 1;
    }
    i == segs.len()
}

#[cfg(test)]
mod mutation_evidence {
    //! THE REGISTER'S MUTATION EVIDENCE (S7 design §1.7 as amended by [S-I1]).
    //!
    //! The register ships EMPTY, so nothing in the shipped corpus exercises it.
    //! These four fixtures are what stand in the way of shipping a suppression
    //! mechanism nobody has seen discriminate: a test-only fixture register plus
    //! synthetic divergent record pairs proving that
    //!
    //!   (a) a covered field IS suppressed,
    //!   (b) a co-occurring UNCOVERED PATH defeats suppression,
    //!   (c) a CLASS MISMATCH defeats suppression,
    //!   (d) [S-I1] a covered difference that later GROWS is DIVERGENT.

    use super::*;
    use crate::diff::diff_records;
    use serde_json::json;

    /// The fixture register: one entry adjudicating a `facts` difference under
    /// the `regards.` family, classified STATE, in the `probe` world's trace.
    fn fixture_register() -> Register {
        Register::parse(&json!({
            "format": 1,
            "entries": [{
                "id": "DIV-FIXTURE",
                "world": "probe",
                "mode": "trace",
                "class": "STATE",
                "fields": ["facts"],
                "paths": ["regards.**"],
                "seeds": [],
                "from_turn": 0,
                "note": "a test-only fixture: the register itself ships EMPTY."
            }]
        }))
        .expect("the fixture register parses")
    }

    fn at(ordinal: i64) -> Situation {
        Situation {
            world: "probe".to_owned(),
            mode: "trace".to_owned(),
            seed: None,
            ordinal,
        }
    }

    fn pair(frozen_facts: &[&str], rust_facts: &[&str]) -> (Value, Value) {
        (
            json!({"t": 1, "actor": "vera", "facts": frozen_facts}),
            json!({"t": 1, "actor": "vera", "facts": rust_facts}),
        )
    }

    #[test]
    fn a_covered_field_is_suppressed() {
        let (a, b) = pair(&["char.a", "regards.w.t.thief"], &["char.a"]);
        let d = diff_records(&a, &b);
        let r = fixture_register().adjudicate(&at(1), Class::State, &d, None);
        match r {
            Ruling::Suppressed { ids, .. } => assert_eq!(ids, vec!["DIV-FIXTURE".to_owned()]),
            other => panic!("expected suppression, got {other:?}"),
        }
    }

    #[test]
    fn a_co_occurring_uncovered_path_defeats_suppression() {
        // The adjudicated `regards.` difference is still there — and one FRESH
        // path rides alongside it. Law 1: fresh signal is never drowned.
        let (a, b) = pair(
            &["char.a", "regards.w.t.thief", "mood.beth!sad"],
            &["char.a"],
        );
        let d = diff_records(&a, &b);
        let r = fixture_register().adjudicate(&at(1), Class::State, &d, None);
        match r {
            Ruling::Divergent(why) => assert!(
                why.contains("mood.beth!sad"),
                "the reason must name the fresh path: {why}"
            ),
            other => panic!("expected DIVERGENT, got {other:?}"),
        }
    }

    #[test]
    fn a_class_mismatch_defeats_suppression() {
        // The same field, the same path — but the divergence classifies as
        // SCHEDULE, and the entry adjudicates STATE. An entry adjudicates a KIND
        // of difference, not a coordinate.
        let (a, b) = pair(&["char.a", "regards.w.t.thief"], &["char.a"]);
        let d = diff_records(&a, &b);
        let r = fixture_register().adjudicate(&at(1), Class::Schedule, &d, None);
        match r {
            Ruling::Divergent(why) => assert!(
                why.contains("class mismatch"),
                "the reason must name the class mismatch: {why}"
            ),
            other => panic!("expected DIVERGENT, got {other:?}"),
        }
    }

    #[test]
    fn a_covered_difference_that_later_grows_is_divergent() {
        // [S-I1]: marked continuation under a NON-GROWTH invariant. Record 1's
        // suppressed set becomes the ceiling; record 5 spreads to a second
        // covered path, so the adjudicated difference is PROPAGATING — a fork
        // question, not a suppression.
        let reg = fixture_register();
        let (a1, b1) = pair(&["char.a", "regards.w.t.thief"], &["char.a"]);
        let d1 = diff_records(&a1, &b1);
        let marks = match reg.adjudicate(&at(1), Class::State, &d1, None) {
            Ruling::Suppressed { marks, .. } => marks,
            other => panic!("record 1 should suppress, got {other:?}"),
        };
        let (a5, b5) = pair(
            &["char.a", "regards.w.t.thief", "regards.x.t.thief"],
            &["char.a"],
        );
        let d5 = diff_records(&a5, &b5);
        let r = reg.adjudicate(&at(5), Class::State, &d5, Some(&marks));
        match r {
            Ruling::Divergent(why) => assert!(
                why.contains("GREW"),
                "the reason must name the growth: {why}"
            ),
            other => panic!("expected DIVERGENT on growth, got {other:?}"),
        }
    }

    #[test]
    fn the_same_difference_persisting_unchanged_stays_suppressed() {
        // The other half of [S-I1]: continuation is the POINT. A stable
        // adjudicated difference must not truncate the run.
        let reg = fixture_register();
        let (a, b) = pair(&["char.a", "regards.w.t.thief"], &["char.a"]);
        let d = diff_records(&a, &b);
        let marks = match reg.adjudicate(&at(1), Class::State, &d, None) {
            Ruling::Suppressed { marks, .. } => marks,
            other => panic!("record 1 should suppress, got {other:?}"),
        };
        let r = reg.adjudicate(&at(20), Class::State, &d, Some(&marks));
        assert!(
            matches!(r, Ruling::Suppressed { .. }),
            "a stable adjudicated difference must keep the comparison running: {r:?}"
        );
    }

    #[test]
    fn path_patterns_match_segment_wise_and_ignore_operators() {
        assert!(path_matches("regards.**", "regards.w.t.thief"));
        assert!(path_matches("mood.*.sad", "mood.beth!sad"));
        assert!(!path_matches("mood.*.sad", "mood.beth.glad"));
        assert!(!path_matches("regards.*", "regards.w.t"));
        assert!(path_matches("regards.*", "regards.w"));
    }

    #[test]
    fn an_empty_register_suppresses_nothing() {
        let reg = Register::parse(&json!({"format": 1, "entries": []})).expect("parses");
        let (a, b) = pair(&["char.a"], &[]);
        let d = diff_records(&a, &b);
        assert!(matches!(
            reg.adjudicate(&at(1), Class::State, &d, None),
            Ruling::Divergent(_)
        ));
    }
}
