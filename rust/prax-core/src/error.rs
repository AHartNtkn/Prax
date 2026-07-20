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
}
