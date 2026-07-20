//! `WorldError` — construction-guard failures as `Result<_, WorldError>`
//! (thiserror; loud). Engine-invariant breaches panic instead; a detected
//! contradiction stays a queryable fact, never an error here (PLAN.md, Errors).

use thiserror::Error;

/// A construction-time failure at an authoring boundary. Loud and
/// `#[must_use]`: worlds `.expect()` these at build, so a malformed path can
/// never silently reach the engine.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[must_use]
pub enum WorldError {
    /// A sentence ends in an operator (`.`/`!`) rather than a name — it would
    /// set a leaf's exclusion flag that nothing ever reads (write-only state
    /// breaking `Db` equality and the serialize round-trip). The frozen
    /// `Prax.Db.tokens` rejects this loudly; so do we. (H: DbSpec.hs "tokens:
    /// trailing-operator rejection".)
    #[error(
        "Prax path {sentence:?}: trailing operator {op:?} -- a sentence ends in a name, not an operator"
    )]
    TrailingOperator { sentence: String, op: char },

    /// A path has more segments than the exclusion bitmask (`u32`) can label.
    /// Real paths are a handful of segments deep; this guards the bitmask
    /// against silent shift overflow rather than corrupting exclusion flags.
    #[error(
        "Prax path {sentence:?}: {segments} segments exceeds the 32-segment exclusion-bitmask limit"
    )]
    PathTooLong { sentence: String, segments: usize },

    /// A `Subquery` appears inside another `Subquery`'s where-clause. The frozen
    /// `Prax.Query` rejects this with a runtime `error`, calling it a structural
    /// error that "cannot depend on runtime state". Because the Rust design
    /// compiles the whole condition family at install (the one compile choke
    /// point), that structural check moves to its natural home — a construction
    /// guard at [`crate::query::compile_condition`] — catching every nested
    /// subquery whether or not the containing query is ever evaluated.
    #[error(
        "Prax condition: a Subquery is nested inside another Subquery's where-clause -- subqueries may not nest"
    )]
    NestedSubquery,

    /// Two actions in one practice share a name. Action names are lookup keys
    /// (delta anchors, standing intentions), so a duplicate would make one
    /// unreachable (`Prax.Engine.definePractice`).
    #[error(
        "Prax.Engine.definePractice: practice {practice:?} declares two actions named {action:?} -- action names are lookup keys; rename one"
    )]
    DuplicateActionName { practice: String, action: String },

    /// A function name is already registered — within one `define_functions`
    /// batch, or against the already-registered set. `Call` resolution is by
    /// bare name (`Prax.Engine.defineFunctions`), so a duplicate collides.
    #[error(
        "Prax.Engine.define_functions: function {function:?} is already registered -- Call resolution is by bare name; rename one"
    )]
    DuplicateFunctionName { function: String },

    /// A schedule-rule name collides — within a batch or across BOTH doors
    /// (the authoring door and the compiler door share one globally-keyed rule
    /// table, and the dues map is keyed by name). (`Prax.Engine.addScheduleRules`.)
    #[error(
        "Prax.Engine: duplicate schedule-rule name {name:?} would share one due key -- rule names are globally keyed across both registration doors; rename one"
    )]
    DuplicateScheduleRuleName { name: String },

    /// A schedule-rule name is not a single path segment. A multi-segment name
    /// would corrupt the by-name due keying (`Prax.Engine.addScheduleRules`).
    #[error("Prax.Engine: schedule rule name must be a single segment: {name:?}")]
    MultiSegmentRuleName { name: String },

    /// A schedule rule's period is not positive (`Prax.Engine.addScheduleRules`).
    #[error("Prax.Engine: schedule rule {name:?} needs a positive period")]
    NonPositivePeriod { name: String },

    /// An authored fragment uses a reserved variable at a combinator/install
    /// boundary: the `Prax` namespace (all machinery variables) or a name the
    /// combinator itself binds in the same splice (the v40 hygiene boundary,
    /// `Prax.Types.authoredVarClash`). `context` names the boundary that caught it.
    #[error(
        "Prax {context}: a fragment authors {var:?} -- the Prax namespace is reserved for engine machinery{extra}"
    )]
    ReservedVarClash {
        context: String,
        var: String,
        /// A boundary-specific tail (e.g. `set_schedule`'s Actor note); empty
        /// otherwise.
        extra: String,
    },

    /// A die seed lies outside the stream's domain: `0` and multiples of the
    /// modulus are fixed points (`Prax.Engine.seedDie`, `Prax.Rng.seedBounds`).
    #[error(
        "Prax.Engine.seed_die: seed {seed} lies outside the die's domain [{lo}, {hi}] -- 0 and multiples of the modulus are fixed points (a die that always rolls the same face)"
    )]
    SeedOutOfDomain { seed: i64, lo: i64, hi: i64 },

    /// A `draw`'s odds are not a real chance (`0 < num < den`): certainty and
    /// impossibility are authored dishonesty (`Prax.Rng.draw`).
    #[error("Prax.Rng: draw odds {num}/{den} must satisfy 0 < num < den")]
    DrawOdds { num: i64, den: i64 },

    /// [`crate::schedule::lasts`] was handed something other than an `Insert`. A
    /// lifetime on a `Delete`/`Call`/`ForEach`/`Roll` has no meaning — the one
    /// expiry mechanism arms an asserted fact, nothing else (`Prax.Schedule.lasts`).
    #[error(
        "Prax.Schedule.lasts: only an Insert can carry a lifetime, got: {outcome} -- a lifetime on a Delete/Call/ForEach/Roll has no meaning"
    )]
    LifetimeOnNonInsert { outcome: String },

    /// A [`crate::schedule::gathering`]'s duration is not `0 < duration < period`:
    /// a gathering that never opens, or one whose opening never lapses before the
    /// next, is not a gathering (`Prax.Schedule.gathering`).
    #[error(
        "Prax.Schedule.gathering {name:?} needs 0 < duration < period, got duration {duration} / period {period}"
    )]
    GatheringDuration {
        name: String,
        period: i64,
        duration: i64,
    },
}
