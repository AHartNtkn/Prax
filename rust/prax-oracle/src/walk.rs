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

/// One MMIX linear-congruential step.
pub fn lcg(x: u64) -> u64 {
    6_364_136_223_846_793_005_u64
        .wrapping_mul(x)
        .wrapping_add(1_442_695_040_888_963_407)
}

/// A uniform index in `[0, n)` and the next seed — the frozen `pick`.
///
/// # Panics
/// If `n == 0` (the walk only picks from a non-empty candidate list).
pub fn pick(n: usize, s: u64) -> (usize, u64) {
    assert!(n > 0, "pick from an empty candidate list");
    let s2 = lcg(s);
    ((s2 % n as u64) as usize, s2)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    // The generator is a transcription, so it is pinned by its own arithmetic:
    // one step from 0 is the increment, and `pick` returns that step's residue.
    #[test]
    fn lcg_step_is_the_mmix_recurrence() {
        assert_eq!(lcg(0), 1_442_695_040_888_963_407);
        assert_eq!(
            lcg(1),
            6_364_136_223_846_793_005_u64.wrapping_add(1_442_695_040_888_963_407)
        );
    }

    #[test]
    fn pick_indexes_the_advanced_seed_not_the_current_one() {
        let (i, s) = pick(7, 0);
        assert_eq!(s, lcg(0));
        assert_eq!(i, (lcg(0) % 7) as usize);
    }
}
