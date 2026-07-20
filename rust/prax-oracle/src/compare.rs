//! Running the two engines side by side and localizing the first divergence.
//!
//! Records are compared IN ORDER and the run stops at the FIRST divergent record
//! (S7 design §1.2) — the localization anchor. Two amendments shape the walk:
//!
//! - **[M1] headers are compared FIRST**, and a mismatch is SHAPE-DIVERGENT, not
//!   a record divergence. A `--depth` or `--idle` drift would otherwise present
//!   as a DECISION storm from turn 1.
//! - **[M2] the anchor is the record ORDINAL, not `t`.** randtrace's `t` does
//!   not advance on an idle pass, so two records at the same `t` are not the
//!   same step. Both are printed.

use crate::classify::{Ctx, Shape, Verdict, classify};
use crate::diff::{RecordDiff, diff_records};
use crate::register::{Register, Ruling, Situation};
use serde_json::Value;
use std::collections::BTreeSet;

/// What a run of the comparator concluded.
#[derive(Clone, Debug)]
pub enum Outcome {
    /// Every record agreed.
    Clean,
    /// Every divergent record was covered by the register.
    CleanModAdjudicated {
        /// The entry ids that did the covering.
        ids: Vec<String>,
    },
    /// The headers disagree — the two sides were not asked the same question,
    /// or one drifted in a parameter [M1].
    ShapeDivergent {
        /// The header fields that differ.
        fields: Vec<String>,
        /// The rendered detail.
        detail: Vec<String>,
    },
    /// A real divergence, localized.
    Divergent(Box<Divergence>),
}

/// The localized first divergence — the artifact of record.
#[derive(Clone, Debug)]
pub struct Divergence {
    /// The record ORDINAL [M2] (0 = header).
    pub ordinal: usize,
    /// The walk's own `t` on each side, printed alongside [M2].
    pub t: (Option<i64>, Option<i64>),
    /// The triage.
    pub verdict: Verdict,
    /// The full field diff.
    pub diff: RecordDiff,
    /// The frozen record.
    pub frozen: Value,
    /// The Rust record.
    pub rust: Value,
    /// Why the register did not cover it, when it applied at all.
    pub register_note: Option<String>,
}

impl Outcome {
    /// The one-word matrix cell.
    pub fn cell(&self) -> &'static str {
        match self {
            Outcome::Clean => "clean",
            Outcome::CleanModAdjudicated { .. } => "clean-mod-adjudicated",
            Outcome::ShapeDivergent { .. } => "SHAPE-DIVERGENT",
            Outcome::Divergent(_) => "DIVERGENT",
        }
    }
    /// Does this outcome fail the run?
    pub fn is_failure(&self) -> bool {
        matches!(self, Outcome::ShapeDivergent { .. } | Outcome::Divergent(_))
    }
}

/// Compare two record streams (header first, then records in order).
///
/// # Errors
/// If the classifier refuses — notably when `candidates` differ without a green
/// `worldshape` at this freeze rev ([S-I2]).
#[allow(clippy::too_many_arguments)]
pub fn compare_streams(
    frozen: &[Value],
    rust: &[Value],
    ctx: &Ctx,
    reg: &Register,
    world: &str,
    mode_name: &str,
    seed: Option<i64>,
) -> Result<Outcome, String> {
    // [M1] the headers, first.
    let (fh, rh) = (
        frozen.first().ok_or("the frozen stream is empty")?,
        rust.first().ok_or("the rust stream is empty")?,
    );
    let hd = diff_records(fh, rh);
    if !hd.is_empty() {
        return Ok(Outcome::ShapeDivergent {
            fields: hd.field_names(),
            detail: hd.fields.iter().flat_map(crate::diff::render_field).collect(),
        });
    }

    let mut marks: Option<BTreeSet<String>> = None;
    let mut ids: BTreeSet<String> = BTreeSet::new();
    let n = frozen.len().max(rust.len());
    for i in 1..n {
        let a = frozen.get(i).unwrap_or(&Value::Null);
        let b = rust.get(i).unwrap_or(&Value::Null);
        let d = diff_records(a, b);
        if d.is_empty() {
            continue;
        }
        let verdict = classify(ctx, &d, a, b)?;
        let at = Situation {
            world: world.to_owned(),
            mode: mode_name.to_owned(),
            seed,
            ordinal: i as i64,
        };
        match reg.adjudicate(&at, verdict.class, &d, marks.as_ref()) {
            Ruling::Suppressed {
                ids: covering,
                marks: m,
            } => {
                ids.extend(covering);
                if marks.is_none() {
                    marks = Some(m);
                }
                // [S-I1] marked continuation: keep comparing.
                continue;
            }
            Ruling::Divergent(note) => {
                return Ok(Outcome::Divergent(Box::new(Divergence {
                    ordinal: i,
                    t: (rec_t(a), rec_t(b)),
                    verdict,
                    diff: d,
                    frozen: a.clone(),
                    rust: b.clone(),
                    register_note: (!reg.entries.is_empty()).then_some(note),
                })));
            }
        }
    }
    if ids.is_empty() {
        Ok(Outcome::Clean)
    } else {
        Ok(Outcome::CleanModAdjudicated {
            ids: ids.into_iter().collect(),
        })
    }
}

fn rec_t(v: &Value) -> Option<i64> {
    v.get("t").and_then(Value::as_i64)
}

/// Render a localized divergence — records side by side, the differing fields,
/// and the triage. The class is never the whole report.
pub fn render(d: &Divergence, shape: &Shape) -> Vec<String> {
    let mut out = Vec::new();
    out.push(format!(
        "DIVERGENCE at record ordinal {} (frozen t={:?}, rust t={:?}) [M2: the ordinal is the \
         anchor; `t` does not advance on an idle pass]",
        d.ordinal, d.t.0, d.t.1
    ));
    if let Shape::Green(rev) = shape {
        out.push(format!("worldshape: GREEN at freeze rev {rev}"));
    }
    out.extend(crate::classify::render(&d.verdict));
    if let Some(note) = &d.register_note {
        out.push(format!("register: {note}"));
    }
    out.push("--- field diff ---".to_owned());
    for fd in &d.diff.fields {
        out.extend(crate::diff::render_field(fd));
    }
    out.push("--- records ---".to_owned());
    out.push(format!("frozen: {}", brief(&d.frozen)));
    out.push(format!("rust  : {}", brief(&d.rust)));
    out
}

fn brief(v: &Value) -> String {
    let s = v.to_string();
    if s.len() <= 800 {
        s
    } else {
        format!("{}… ({} bytes)", &s[..800], s.len())
    }
}
