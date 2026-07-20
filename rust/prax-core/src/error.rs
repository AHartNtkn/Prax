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
}
