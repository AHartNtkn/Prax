//! The record diff, and the fact-level path diff (S7 design §1.5).
//!
//! The class a divergence gets is TRIAGE; the artifact of record is the record
//! PAIR plus the full field diff. This module builds that diff.
//!
//! Fact sets get three buckets, not two: `only_frozen` and `only_rust` (set
//! differences, grouped by longest common SEGMENT PREFIX and rendered as a
//! CAPPED tree so one closure bug cannot emit 4,000 lines), and **`relabeled`**
//! — same path, different operator (`x!y` vs `x.y`). Relabeling is a distinct
//! bug class (exclusion semantics, ground round-trips) and must never be buried
//! inside the set differences, where it would read as one deletion plus one
//! unrelated insertion.
//!
//! `expiries` gets the same tree treatment [M3]: it is keyed by exact labeled
//! path, so a lifetime bug produces exactly the wide, structured difference the
//! tree exists for.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// How many leaves a path-tree group prints before it summarizes.
pub const TREE_CAP: usize = 12;

/// One field's difference between the two records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDiff {
    /// The record field that differs.
    pub field: String,
    /// The frozen value.
    pub frozen: Value,
    /// The Rust value.
    pub rust: Value,
    /// For labeled-sentence fields, the three-bucket path diff.
    pub paths: Option<PathDiff>,
}

/// The three buckets of a labeled-sentence set difference.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PathDiff {
    /// Present on the frozen side only.
    pub only_frozen: Vec<String>,
    /// Present on the Rust side only.
    pub only_rust: Vec<String>,
    /// The same segment path under a DIFFERENT operator: `(frozen, rust)`.
    pub relabeled: Vec<(String, String)>,
}

impl PathDiff {
    /// Every path this diff mentions, in one flat list — the register's
    /// per-path coverage test and the non-growth invariant both read it.
    pub fn all_paths(&self) -> Vec<String> {
        let mut v: Vec<String> = self.only_frozen.clone();
        v.extend(self.only_rust.clone());
        for (a, b) in &self.relabeled {
            v.push(a.clone());
            v.push(b.clone());
        }
        v.sort();
        v.dedup();
        v
    }
    /// Is there any difference at all?
    pub fn is_empty(&self) -> bool {
        self.only_frozen.is_empty() && self.only_rust.is_empty() && self.relabeled.is_empty()
    }
}

/// The whole difference between two records.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordDiff {
    /// One entry per differing field, in field-name order.
    pub fields: Vec<FieldDiff>,
}

impl RecordDiff {
    /// Do the two records agree?
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
    /// The names of the differing fields.
    pub fn field_names(&self) -> Vec<String> {
        self.fields.iter().map(|f| f.field.clone()).collect()
    }
    /// Does a named field differ?
    pub fn has(&self, field: &str) -> bool {
        self.fields.iter().any(|f| f.field == field)
    }
    /// The named field's difference, if it differs.
    pub fn get(&self, field: &str) -> Option<&FieldDiff> {
        self.fields.iter().find(|f| f.field == field)
    }
}

/// The fields whose values are labeled-sentence LISTS (three-bucket treatment).
const SENTENCE_LIST_FIELDS: &[&str] = &["facts", "view", "setup_db"];
/// The fields whose values are labeled-path-keyed MAPS ([M3]).
const PATH_MAP_FIELDS: &[&str] = &["expiries"];

/// Diff two records field by field. Fields present on one side only are
/// reported with `null` on the other — a missing field is a difference, never a
/// skip.
pub fn diff_records(frozen: &Value, rust: &Value) -> RecordDiff {
    let empty = serde_json::Map::new();
    let fa = frozen.as_object().unwrap_or(&empty);
    let fb = rust.as_object().unwrap_or(&empty);
    let keys: BTreeSet<&String> = fa.keys().chain(fb.keys()).collect();
    let mut fields = Vec::new();
    for k in keys {
        // `engine` names the side, by construction, on every record.
        if k == "engine" {
            continue;
        }
        let a = fa.get(k).unwrap_or(&Value::Null);
        let b = fb.get(k).unwrap_or(&Value::Null);
        if a == b {
            continue;
        }
        let paths = if SENTENCE_LIST_FIELDS.contains(&k.as_str()) {
            Some(path_diff(&string_list(a), &string_list(b)))
        } else if PATH_MAP_FIELDS.contains(&k.as_str()) {
            Some(path_diff(&map_keys(a), &map_keys(b)))
        } else {
            None
        };
        fields.push(FieldDiff {
            field: k.clone(),
            frozen: a.clone(),
            rust: b.clone(),
            paths,
        });
    }
    RecordDiff { fields }
}

fn string_list(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|x| x.as_str().unwrap_or_default().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn map_keys(v: &Value) -> Vec<String> {
    v.as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default()
}

/// A labeled sentence's SEGMENT path, operators stripped: `a.b!c` → `a.b.c`.
/// Two sentences with the same segment path and different operators are
/// RELABELED, not one deletion plus one insertion.
pub fn segment_path(labeled: &str) -> String {
    labeled.replace('!', ".")
}

/// The three-bucket path diff.
pub fn path_diff(frozen: &[String], rust: &[String]) -> PathDiff {
    let fa: BTreeSet<&String> = frozen.iter().collect();
    let fb: BTreeSet<&String> = rust.iter().collect();
    let only_a: Vec<String> = fa.difference(&fb).map(|s| (*s).clone()).collect();
    let only_b: Vec<String> = fb.difference(&fa).map(|s| (*s).clone()).collect();
    // Pair up by segment path: a segment path present on BOTH sides but spelled
    // differently is a relabeling, and leaves both set differences.
    let mut by_seg_a: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for s in &only_a {
        by_seg_a.entry(segment_path(s)).or_default().push(s.clone());
    }
    let mut by_seg_b: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for s in &only_b {
        by_seg_b.entry(segment_path(s)).or_default().push(s.clone());
    }
    let mut relabeled = Vec::new();
    let mut consumed_a = BTreeSet::new();
    let mut consumed_b = BTreeSet::new();
    for (seg, aa) in &by_seg_a {
        if let Some(bb) = by_seg_b.get(seg) {
            for (x, y) in aa.iter().zip(bb.iter()) {
                relabeled.push((x.clone(), y.clone()));
                consumed_a.insert(x.clone());
                consumed_b.insert(y.clone());
            }
        }
    }
    PathDiff {
        only_frozen: only_a
            .into_iter()
            .filter(|s| !consumed_a.contains(s))
            .collect(),
        only_rust: only_b
            .into_iter()
            .filter(|s| !consumed_b.contains(s))
            .collect(),
        relabeled,
    }
}

/// Group paths by their longest common SEGMENT PREFIX and render as a capped
/// tree: the family summary first, the tree second, so one closure bug cannot
/// bury the report in 4,000 lines.
pub fn render_tree(paths: &[String], cap: usize) -> Vec<String> {
    if paths.is_empty() {
        return Vec::new();
    }
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for p in paths {
        let head = segment_path(p)
            .split('.')
            .next()
            .unwrap_or_default()
            .to_owned();
        groups.entry(head).or_default().push(p.clone());
    }
    let mut out = Vec::new();
    out.push(format!(
        "{} path(s) in {} famil{}:",
        paths.len(),
        groups.len(),
        if groups.len() == 1 { "y" } else { "ies" }
    ));
    for (fam, mut ps) in groups {
        ps.sort();
        out.push(format!("  {fam}.  ({} path(s))", ps.len()));
        for p in ps.iter().take(cap) {
            out.push(format!("    {p}"));
        }
        if ps.len() > cap {
            out.push(format!("    … and {} more under {fam}.", ps.len() - cap));
        }
    }
    out
}

/// Render one field difference for the report.
pub fn render_field(fd: &FieldDiff) -> Vec<String> {
    let mut out = vec![format!("field `{}`:", fd.field)];
    match &fd.paths {
        None => {
            out.push(format!("  frozen: {}", truncate(&fd.frozen)));
            out.push(format!("  rust  : {}", truncate(&fd.rust)));
        }
        Some(pd) => {
            if !pd.relabeled.is_empty() {
                out.push(format!(
                    "  RELABELED ({}) — same path, different operator:",
                    pd.relabeled.len()
                ));
                for (a, b) in pd.relabeled.iter().take(TREE_CAP) {
                    out.push(format!("    frozen {a}   rust {b}"));
                }
                if pd.relabeled.len() > TREE_CAP {
                    out.push(format!("    … and {} more", pd.relabeled.len() - TREE_CAP));
                }
            }
            if !pd.only_frozen.is_empty() {
                out.push("  only_frozen:".to_owned());
                out.extend(
                    render_tree(&pd.only_frozen, TREE_CAP)
                        .into_iter()
                        .map(|l| format!("  {l}")),
                );
            }
            if !pd.only_rust.is_empty() {
                out.push("  only_rust:".to_owned());
                out.extend(
                    render_tree(&pd.only_rust, TREE_CAP)
                        .into_iter()
                        .map(|l| format!("  {l}")),
                );
            }
        }
    }
    out
}

fn truncate(v: &Value) -> String {
    let s = v.to_string();
    if s.len() <= 400 {
        s
    } else {
        format!("{}… ({} bytes)", &s[..400], s.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_relabeling_is_its_own_bucket_not_two_set_differences() {
        let pd = path_diff(
            &["mood.beth!sad".into(), "char.a".into()],
            &["mood.beth.sad".into(), "char.a".into()],
        );
        assert_eq!(
            pd.relabeled,
            vec![("mood.beth!sad".to_owned(), "mood.beth.sad".to_owned())]
        );
        assert!(pd.only_frozen.is_empty(), "{:?}", pd.only_frozen);
        assert!(pd.only_rust.is_empty(), "{:?}", pd.only_rust);
    }

    #[test]
    fn genuine_set_differences_stay_in_their_buckets() {
        let pd = path_diff(&["a.b".into(), "c.d".into()], &["a.b".into(), "e.f".into()]);
        assert_eq!(pd.only_frozen, vec!["c.d".to_owned()]);
        assert_eq!(pd.only_rust, vec!["e.f".to_owned()]);
        assert!(pd.relabeled.is_empty());
    }

    #[test]
    fn expiries_get_the_path_tree_treatment() {
        let a = json!({"expiries": {"a.b!c": 3, "a.b!d": 3}});
        let b = json!({"expiries": {"a.b!c": 3}});
        let d = diff_records(&a, &b);
        let fd = d.get("expiries").expect("expiries differ");
        assert_eq!(
            fd.paths.as_ref().expect("path diff").only_frozen,
            vec!["a.b!d".to_owned()]
        );
    }

    #[test]
    fn the_engine_field_names_the_side_and_is_never_a_difference() {
        let a = json!({"engine": "haskell", "t": 1});
        let b = json!({"engine": "rust", "t": 1});
        assert!(diff_records(&a, &b).is_empty());
    }

    #[test]
    fn a_field_present_on_one_side_only_is_a_difference() {
        let a = json!({"t": 1, "walkSeed": 9});
        let b = json!({"t": 1});
        assert_eq!(diff_records(&a, &b).field_names(), vec!["walkSeed"]);
    }

    #[test]
    fn the_tree_caps_a_wide_family_and_says_how_many_it_hid() {
        let paths: Vec<String> = (0..40).map(|i| format!("bel.x{i}")).collect();
        let lines = render_tree(&paths, 5);
        assert!(lines[0].starts_with("40 path(s) in 1 family"));
        assert!(lines.iter().any(|l| l.contains("and 35 more under bel.")));
    }
}
