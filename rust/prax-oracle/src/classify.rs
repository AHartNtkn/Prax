//! THE CLASSIFIER — the comparator's one job, stated as a total function.
//!
//! Its question is not "do the engines differ" (the diff answers that) but
//! *which went wrong — the port of the world, or the engine underneath it?* The
//! answer is TRIAGE, never a verdict: the artifact of record is always the
//! record PAIR plus the full field diff, and every rendering says so.
//!
//! The ladder (S7 design §1.3 as amended by [S-C1] and [S-I3]), each rung
//! presupposing agreement above it:
//!
//! | rung | fires when | points at |
//! |---|---|---|
//! | **TERMINATION** | the streams stop differently, one is short, or `passes` differs | the three stop rules |
//! | **TURN** | `actor`/`cursor`/`idle`/`t` differ | `advance`: cursor arithmetic, the `i <= cursor` wrap, aliveness, post-boundary re-selection |
//! | **ENUMERATION** | `candidates` differ | `possible_actions` ordering/filters — or a world-port error (§2) |
//! | **DECISION** | candidates equal, `action`/`identity`/`scores`/`intention_*` differs | MODE-DEPENDENT [D-C2] |
//! | **RNG** | action equal, `rng`/`walkSeed`/`draws` differ | `CRoll` execution, or the walk's own `pick`/LCG |
//! | **SCHEDULE** | action+rng equal, `boundary`/`dues`/`expiries` differ | boundary firing, re-arming, expiry arm/cancel/purge, v44 supersession |
//! | **STATE** | all above equal, `facts` differ | perform semantics, spawn, ForEach snapshot, Call's base-db quirk, closure |
//! | **STATE(view)** | `view` differs and `facts` does NOT (here, or at t−1) | derivation: axiom heads, defeater names, closure completeness |
//! | **UNCLASSIFIED** | differs but matches NO rung | THE COMPARATOR ITSELF — fails loud |
//!
//! Three amendments carry most of the classifier's value:
//!
//! - **[S-C1] totality.** `actor`/`cursor`/`idle`/`t` were emitted and
//!   unclassified while `advance` is a distinct bug site; and a pair that
//!   matched no rung was silently mislabeled as the last one. TURN and a
//!   terminal UNCLASSIFIED close both holes — UNCLASSIFIED is a comparator bug
//!   and reports as such rather than pointing an implementer at innocent code.
//! - **[D-C2] mode parameterisation.** `randWalk` never touches the planner: it
//!   selects with `possible_actions` + `pick`. So in randtrace mode "candidates
//!   equal, action differs" is DEFINITIONALLY an ordering or pick/LCG bug, and
//!   pointing at fold association or the reuse gate would send the reader to
//!   machinery that never ran.
//! - **[S-I2] the shape precedence is a RULE.** ENUMERATION is only reportable
//!   behind a green `worldshape` at the SAME freeze rev, and the report carries
//!   the rev. Without it, every world-port error in the corpus reads as an
//!   engine bug.

use crate::diff::RecordDiff;
use serde_json::Value;

/// Which walk produced the records — the classifier is parameterised by it
/// [D-C2].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Walk {
    /// `trace`: `advance` then `npcAct`. The planner runs.
    Trace,
    /// `randtrace`: `advance` then `possibleActions` + `pick`. It does NOT.
    Randtrace,
}

/// The `worldshape` gate's state for this world at this freeze rev [S-I2].
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Shape {
    /// Shape and bodies compared equal at this freeze rev.
    Green(String),
    /// Not compared — ENUMERATION is not reportable.
    NotChecked,
}

/// The classes, in ladder order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    /// The streams stop differently, or one is shorter.
    Termination,
    /// `advance` disagreed about whose turn it is.
    Turn,
    /// The candidate sets or their ORDER disagree.
    Enumeration,
    /// Same candidates, different choice.
    Decision,
    /// Same choice, different stream.
    Rng,
    /// Same everything above, different boundary bookkeeping.
    Schedule,
    /// Same everything above, different facts.
    State,
    /// A STATE divergence localized to the VIEW at t−1 — the DIV-1 shape.
    StateView,
    /// Differs, but matches no rung. A COMPARATOR BUG.
    Unclassified,
}

impl Class {
    /// The report spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Class::Termination => "TERMINATION",
            Class::Turn => "TURN",
            Class::Enumeration => "ENUMERATION",
            Class::Decision => "DECISION",
            Class::Rng => "RNG",
            Class::Schedule => "SCHEDULE",
            Class::State => "STATE",
            Class::StateView => "STATE(view)",
            Class::Unclassified => "UNCLASSIFIED",
        }
    }
}

/// A classification: the class, the fields that carried it, where to look, and
/// the standing reminder that the class is triage.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Verdict {
    /// The rung that fired.
    pub class: Class,
    /// The differing fields that put it on that rung.
    pub evidence: Vec<String>,
    /// Where to look — mode-dependent for DECISION [D-C2].
    pub pointer: String,
    /// Everything else that differs (the class is triage, not a verdict).
    pub other_fields: Vec<String>,
}

/// The classifier's inputs beyond the record pair.
pub struct Ctx {
    /// Which walk produced the stream.
    pub walk: Walk,
    /// The `worldshape` gate's state [S-I2].
    pub shape: Shape,
    /// Did the VIEWs differ at the PREVIOUS record while the base dbs agreed?
    /// The localizer sets this from the `--mode view` rerun; it reclassifies a
    /// STATE-looking divergence as `STATE(view)` — the DIV-1 shape and the
    /// single most valuable rule in the classifier.
    pub view_differs_at_previous: bool,
}

/// Fields grouped by rung, in ladder order.
///
/// THE UNION OF THESE SETS MUST COVER EVERY KEY THE ORACLE CAN EMIT. A field
/// that is emitted and claimed by no rung reports as UNCLASSIFIED — which sends
/// an implementer looking at `classify.rs` for what is a genuine engine
/// divergence. `rung_field_sets_cover_every_emitted_key` in
/// [`crate::harness_tests`] drives the frozen oracle with every emission flag on
/// and asserts the coverage, so a field added to the emission cannot land
/// without a rung.
const TURN_FIELDS: &[&str] = &["actor", "cursor", "idle", "t"];
const ENUMERATION_FIELDS: &[&str] = &["candidates"];
const DECISION_FIELDS: &[&str] = &[
    "action",
    "identity",
    "scores",
    "intention_before",
    "intention_after",
];
const RNG_FIELDS: &[&str] = &["rng", "walkSeed", "draws"];
const SCHEDULE_FIELDS: &[&str] = &["boundary", "dues", "expiries", "boundary_log"];
const STATE_FIELDS: &[&str] = &["facts", "view"];
const TERMINATION_FIELDS: &[&str] = &["end", "reason", "ending", "records", "passes"];

/// Every rung's fields, in ladder order — the totality test's subject.
#[cfg(test)]
pub const RUNGS: &[(&str, &[&str])] = &[
    ("TERMINATION", TERMINATION_FIELDS),
    ("TURN", TURN_FIELDS),
    ("ENUMERATION", ENUMERATION_FIELDS),
    ("DECISION", DECISION_FIELDS),
    ("RNG", RNG_FIELDS),
    ("SCHEDULE", SCHEDULE_FIELDS),
    ("STATE", STATE_FIELDS),
];

/// Classify one divergent record pair.
///
/// # Errors
/// - If the pair does not actually differ (classification of an agreement is a
///   caller bug).
/// - If `candidates` differ without a green `worldshape` at this freeze rev
///   ([S-I2]: the precedence is a rule, so the classifier refuses rather than
///   reporting an engine class for what may be a world-port error).
pub fn classify(ctx: &Ctx, d: &RecordDiff, frozen: &Value, rust: &Value) -> Result<Verdict, String> {
    if d.is_empty() {
        return Err("classify called on a pair that does not differ".to_owned());
    }
    let names = d.field_names();
    let hit = |group: &[&str]| -> Vec<String> {
        names
            .iter()
            .filter(|n| group.contains(&n.as_str()))
            .cloned()
            .collect()
    };
    let rest = |used: &[String]| -> Vec<String> {
        names.iter().filter(|n| !used.contains(n)).cloned().collect()
    };
    let verdict = |class: Class, evidence: Vec<String>, pointer: String| -> Verdict {
        let other_fields = rest(&evidence);
        Verdict {
            class,
            evidence,
            pointer,
            other_fields,
        }
    };

    // TERMINATION first: a terminal record against a turn record, or two
    // terminals that disagree, is a stop-rule divergence — and every field
    // difference below it would be an artifact of comparing unlike records.
    let is_end = |v: &Value| v.get("end").is_some();
    if is_end(frozen) != is_end(rust) {
        return Ok(verdict(
            Class::Termination,
            vec!["end".to_owned()],
            format!(
                "one stream ended and the other did not (frozen ended: {}, rust ended: {}) — \
                 the three randtrace stop rules: the cap, the `ending.E` fact, and the \
                 `passes > living` dead end",
                is_end(frozen),
                is_end(rust)
            ),
        ));
    }
    // Every field on this rung is stop-rule bookkeeping. `end`/`reason`/
    // `ending`/`records` exist only on a terminal record; `passes` rides every
    // randtrace turn record and is the counter the `passes > living` dead-end
    // rule reads. So the rung is NOT gated on the record being terminal —
    // otherwise a turn pair differing only in `passes` matches no rung at all.
    // It outranks TURN safely: the run stops at the FIRST divergent record, so a
    // `passes` difference with every earlier record equal (including `idle`)
    // cannot be downstream of a turn divergence — it is the counter's own
    // arithmetic.
    let term = hit(TERMINATION_FIELDS);
    if !term.is_empty() {
        return Ok(verdict(
            Class::Termination,
            term,
            "the stop rule itself: which of cap / ending / extinct / deadend fired, after how \
             many records, and the `passes` counter the dead-end rule reads (it advances only on \
             an idle pass, and the cap decrements only on an action)"
                .to_owned(),
        ));
    }

    let turn = hit(TURN_FIELDS);
    if !turn.is_empty() {
        return Ok(verdict(
            Class::Turn,
            turn,
            "`Prax.Loop.advance`: the cursor arithmetic, the `i <= cursor` wrap (equality \
             INCLUDED), aliveness, and the post-boundary re-selection"
                .to_owned(),
        ));
    }

    let enumeration = hit(ENUMERATION_FIELDS);
    if !enumeration.is_empty() {
        let rev = match &ctx.shape {
            Shape::Green(rev) => rev.clone(),
            Shape::NotChecked => {
                return Err(
                    "candidates differ, but `worldshape` has not been compared GREEN for this \
                     world at this freeze rev. ENUMERATION is not reportable until it has been \
                     ([S-I2]) — a mis-transcribed label, swapped role or dropped setup fact \
                     presents exactly like an enumeration bug. Run `prax-oracle worldshape \
                     <world>` first."
                        .to_owned(),
                );
            }
        };
        // The walkSeed rule [§1.3(b)]: the pick index depends on len(acts), so a
        // differing walkSeed with a differing candidate-list LENGTH is a SYMPTOM
        // of the enumeration difference, never an RNG divergence.
        let lens = candidate_lengths(frozen, rust);
        let note = match lens {
            Some((a, b)) if a != b => format!(
                " — candidate-list LENGTHS differ ({a} vs {b}), so any walkSeed difference here \
                 is a symptom of this, not an RNG divergence"
            ),
            _ => String::new(),
        };
        return Ok(verdict(
            Class::Enumeration,
            enumeration,
            format!(
                "`possibleActions` ordering/filters — OR A WORLD-PORT ERROR. worldshape is green \
                 at freeze rev {rev}, so the world's authored data agrees{note}"
            ),
        ));
    }

    let decision = hit(DECISION_FIELDS);
    if !decision.is_empty() {
        let pointer = match ctx.walk {
            // The planner ran: fold association, discounts, tiebreak, the reuse
            // gate, the intention hold.
            Walk::Trace =>
                "the planner: fold association, the 0.9/0.5 discounts, the label tiebreak, the \
                 v34 reuse gate, the v35 intention hold. The score table and the intention are on \
                 THIS rung because they are the planner's own evidence: if the score tables are \
                 identical and the action still differs, it is the INTENTION, not the planner \
                 [M4]; and a `scores` difference that does not move the argmax is a scoring bug \
                 whose action agrees."
                    .to_owned(),
            // The planner did NOT run [D-C2].
            Walk::Randtrace =>
                "enumeration ORDER or `pick` — `randWalk` never touches Prax.Planner, so with \
                 equal candidates a different action is the index arithmetic or the MMIX stream, \
                 never a planner bug [D-C2]."
                    .to_owned(),
        };
        return Ok(verdict(Class::Decision, decision, pointer));
    }

    let rng = hit(RNG_FIELDS);
    if !rng.is_empty() {
        let pointer = if rng.iter().any(|f| f == "draws") {
            "the draw log names it: `CRoll` execution — taken/not, the unconditional \
             advance-on-miss, draw order inside a ForEach. (`rng` alone cannot see taken-vs-not, \
             which is why the log exists [S-C5].)"
                .to_owned()
        } else if rng.iter().any(|f| f == "walkSeed") {
            "the WALK's own generator (MMIX `lcg`/`pick`), not the engine die — candidates are \
             equal here, so the list length cannot be the cause"
                .to_owned()
        } else {
            "the engine Lehmer stream: `CRoll` execution — taken/not, advance-on-miss, draw order \
             in a ForEach"
                .to_owned()
        };
        return Ok(verdict(Class::Rng, rng, pointer));
    }

    let sched = hit(SCHEDULE_FIELDS);
    if !sched.is_empty() {
        return Ok(verdict(
            Class::Schedule,
            sched,
            "boundary firing and re-arming, expiry arm/cancel/purge, the v44 supersession law. \
             The boundary log names WHICH dues and expiries fired — the maps alone cannot see an \
             expiry that fired on the wrong subtree [S-C5]."
                .to_owned(),
        ));
    }

    let state = hit(STATE_FIELDS);
    if !state.is_empty() {
        // [I1] The DIV-1 shape AT ITS OWN RECORD. `view` differing while `facts`
        // agrees is a DERIVATION divergence here and now — it needs no t−1 flag
        // to say so, and handling it only one record late would point the reader
        // at perform semantics for a closure bug.
        if state.iter().any(|f| f == "view") && !state.iter().any(|f| f == "facts") {
            return Ok(verdict(
                Class::StateView,
                state,
                "the base dbs AGREE and only the closed VIEW differs — a DERIVATION divergence \
                 (axiom heads, defeater names, closure completeness), at its own record. This is \
                 the DIV-1 shape; nothing in perform semantics can produce it."
                    .to_owned(),
            ));
        }
        if ctx.view_differs_at_previous {
            return Ok(verdict(
                Class::StateView,
                state,
                "the VIEW at t−1 already differed while the base dbs agreed — a DERIVATION \
                 divergence (axiom heads, defeater names, closure completeness), surfacing here a \
                 turn later. This is the DIV-1 shape."
                    .to_owned(),
            ));
        }
        return Ok(verdict(
            Class::State,
            state,
            "perform semantics: spawn (the base-vs-view opacity), the ForEach snapshot, Call's \
             BASE-db quirk and first-case/first-binding rule, and the closure tiers"
                .to_owned(),
        ));
    }

    // The pair differs and matched nothing. That is a COMPARATOR bug — a new
    // emitted field with no rung — and it reports as one rather than being
    // silently folded into STATE.
    Ok(Verdict {
        class: Class::Unclassified,
        evidence: names.clone(),
        pointer: format!(
            "THE COMPARATOR ITSELF: the record pair differs in {names:?}, none of which any rung \
             claims. A field was added to the emission without being added to the ladder. Fix \
             classify.rs — do NOT read this as an engine divergence."
        ),
        other_fields: Vec::new(),
    })
}

/// The two candidate-list lengths, when both records carry the field.
fn candidate_lengths(frozen: &Value, rust: &Value) -> Option<(usize, usize)> {
    Some((
        frozen.get("candidates")?.as_array()?.len(),
        rust.get("candidates")?.as_array()?.len(),
    ))
}

/// Render a verdict for the report.
pub fn render(v: &Verdict) -> Vec<String> {
    let mut out = vec![
        format!("class: {}", v.class.as_str()),
        format!("  evidence: {}", v.evidence.join(", ")),
        format!("  points at: {}", v.pointer),
    ];
    if !v.other_fields.is_empty() {
        out.push(format!(
            "  ALSO differing (the class is TRIAGE, not a verdict — the artifact of record is \
             the record pair plus the full field diff): {}",
            v.other_fields.join(", ")
        ));
    }
    out
}
