//! The randtrace walk's arithmetic and stop rules — comparator-owned.
//!
//! `Prax.Stress.runRandom` is not part of the engine port: it is a HARNESS, and
//! its generator, index arithmetic and three stop rules are transcribed here so
//! the Rust walk steps the same stream as the frozen replay (S7 design §1.1).
//! S9's Stress port reuses this module rather than re-deriving it.
//!
//! Knuth's MMIX constants, wrapping `u64` arithmetic — NOT the engine's MINSTD
//! die (that lives in `prax_core::rng` and rides `CRoll`). Two different streams
//! with two different jobs: this one picks a candidate, that one decides whether
//! a temper flares. Confusing them is exactly the bug the classifier's walkSeed
//! rule exists to name.

// The MMIX generator and its `pick` are single-sourced in `prax_core::stress`
// (the frozen `Prax.Stress` is an engine-library module), so this transcription
// re-exports them — the randtrace driver and the Stress aggregator step the same
// stream by construction, not by two copies staying in sync.
pub use prax_core::stress::pick;

/// Why a walk ended ([S-I3]). Each of the frozen walk's exits gets a name, so a
/// stream-length divergence has a class (TERMINATION) and evidence.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Stop {
    /// The step cap was reached (`k == 0`).
    Cap,
    /// An `ending.<key>` fact appeared.
    Ending(String),
    /// The whole cast is dead.
    Extinct,
    /// Every living character passed in a row — a true dead end.
    DeadEnd,
    /// The trace walk simply ran its turn count out.
    Turns,
}

impl Stop {
    /// The wire spelling of the reason.
    pub fn reason(&self) -> &'static str {
        match self {
            Stop::Cap => "cap",
            Stop::Ending(_) => "ending",
            Stop::Extinct => "extinct",
            Stop::DeadEnd => "deadend",
            Stop::Turns => "turns",
        }
    }
    /// The ending key, when the walk stopped on one.
    pub fn ending(&self) -> Option<&str> {
        match self {
            Stop::Ending(e) => Some(e),
            _ => None,
        }
    }
}

