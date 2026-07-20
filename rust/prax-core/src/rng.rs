//! The drama die: a bit-exact MINSTD (Lehmer) stream. Advances unconditionally
//! on every draw (the frozen-die law: every draw spends one step, hit or miss),
//! then rolls on the advanced value. Seed domain guarded (0 and modulus
//! multiples are fixed points).
//!
//! Frozen reference: `src/Prax/Rng.hs`. This module keeps ALL the die math — the
//! Park & Miller MINSTD constants, the one-step advance ([`roll_step`]), and the
//! seed-domain bounds ([`SEED_BOUNDS`], consumed by `engine::State::seed_die`);
//! [`draw`] compiles authored odds to a first-class [`Outcome::Roll`] that the
//! engine executes against the stream. `Integer` becomes `i64` (a recorded
//! deviation, ARCHITECTURE.md) — the stream value stays well within `i64`.

use crate::error::WorldError;
use crate::query::Condition;
use crate::types::{Outcome, authored_var_clash};

/// Park & Miller (1988) MINSTD minimal-standard multiplier. Mechanism with
/// published provenance, fixed here, never tuned: the AUTHORED numbers are
/// [`draw`]'s odds.
const LEHMER_A: i64 = 16807;
/// The MINSTD modulus, `2^31 - 1`.
const LEHMER_M: i64 = 2_147_483_647;

/// The seed's valid domain, inclusive: strictly between 0 and the modulus. `0`
/// and multiples of the modulus are fixed points of the stream (a die that
/// always rolls the same face), so they are excluded (`Prax.Rng.seedBounds`).
pub const SEED_BOUNDS: (i64, i64) = (1, LEHMER_M - 1);

/// One Lehmer step: the advanced stream value (`Prax.Rng.rollStep`). The roll
/// basis IS this advanced value (a state advance evicts the old seed, so the roll
/// reads the fresh one — v50). `s * 16807` peaks near `2^45`, well inside `i64`.
pub fn roll_step(s: i64) -> i64 {
    (s * LEHMER_A) % LEHMER_M
}

/// "With probability num/den, where `conds` also hold, apply `outs`." The
/// fragment authors append to an action's outcomes (`Prax.Rng.draw`). Compiles
/// to a single [`Outcome::Roll`] — the engine advances the stream once and rolls
/// on the advanced value, so every draw consumes EXACTLY one step, hit or miss.
/// Guards are loud: odds must be a real chance (`0 < num < den` — certainty and
/// impossibility are authored dishonesty), and the caller's conditions and
/// outcomes may not use the reserved `Prax` namespace (v40 hygiene stands even
/// though the die no longer splices anything). Fire site with `forbidden = []`.
pub fn draw(
    num: i64,
    den: i64,
    conds: Vec<Condition>,
    outs: Vec<Outcome>,
) -> Result<Vec<Outcome>, WorldError> {
    if num <= 0 || num >= den {
        return Err(WorldError::DrawOdds { num, den });
    }
    if let Some(v) = authored_var_clash(&[], &conds, &outs).into_iter().next() {
        return Err(WorldError::ReservedVarClash {
            context: "Rng.draw".to_owned(),
            var: v,
            extra: " (the Prax namespace is reserved for the die's own machinery)".to_owned(),
        });
    }
    Ok(vec![Outcome::Roll(num, den, conds, outs)])
}

#[cfg(test)]
mod tests {
    // H: RngSpec.hs "Prax.Rng"
    //
    // The RngSpec draw-authoring guards and the pure die math live here (the
    // engine-integration half — seed_die, the frozen-die law, hit/miss, the
    // stream — is pinned in the engine tests where the stream rides State).
    use super::*;
    use crate::query::Condition;
    use crate::types::{for_each, insert};

    /// Park & Miller MINSTD, pinned independently of [`roll_step`] so the test is
    /// self-checking arithmetic, not a restatement of the implementation.
    fn lehmer_next(s: i64) -> i64 {
        (s * 16807) % 2_147_483_647
    }

    // H: RngSpec.hs "the domain bounds are the open interval (0, modulus)"
    #[test]
    fn seed_bounds_are_the_open_interval() {
        assert_eq!(SEED_BOUNDS, (1, 2_147_483_646));
    }

    #[test]
    fn roll_step_is_one_park_miller_step() {
        assert_eq!(roll_step(12345), lehmer_next(12345));
        assert_eq!(roll_step(1), 16807);
        assert_eq!(roll_step(2), 33614);
    }

    // ===== draw's authoring guards =====
    // H: RngSpec.hs "draw's authoring guards (surviving pins, re-pointed to the Roll form)"

    // H: RngSpec.hs "num == 0 is rejected"
    #[test]
    fn draw_rejects_num_zero() {
        assert_eq!(draw(0, 2, vec![], vec![]), Err(WorldError::DrawOdds { num: 0, den: 2 }));
    }

    // H: RngSpec.hs "num == den is rejected (certainty is not a chance)"
    #[test]
    fn draw_rejects_num_equals_den() {
        assert_eq!(draw(2, 2, vec![], vec![]), Err(WorldError::DrawOdds { num: 2, den: 2 }));
    }

    // H: RngSpec.hs "num > den is rejected"
    #[test]
    fn draw_rejects_num_over_den() {
        assert_eq!(draw(3, 2, vec![], vec![]), Err(WorldError::DrawOdds { num: 3, den: 2 }));
    }

    // H: RngSpec.hs "the Prax namespace in the caller's conditions is rejected"
    #[test]
    fn draw_rejects_prax_namespace_in_conditions() {
        assert!(matches!(
            draw(1, 2, vec![Condition::Match("flag.PraxS".into())], vec![]),
            Err(WorldError::ReservedVarClash { .. })
        ));
    }

    // H: RngSpec.hs "the Prax namespace in the caller's outcomes is rejected"
    #[test]
    fn draw_rejects_prax_namespace_in_outcomes() {
        assert!(matches!(
            draw(1, 2, vec![], vec![insert("marked.PraxR")]),
            Err(WorldError::ReservedVarClash { .. })
        ));
    }

    // H: RngSpec.hs "ordinary variables S/S2/S3/R are unremarkable authoring names"
    #[test]
    fn draw_accepts_ordinary_variables() {
        let r = draw(1, 2, vec![Condition::Match("flag.S".into())], vec![insert("marked.R")]);
        assert!(matches!(r.as_deref(), Ok([Outcome::Roll(1, 2, _, _)])));
    }

    // ===== GateSpec's shared-guard half, pinned THROUGH a real combinator =====
    // H: GateSpec.hs "Prax.Gate"
    // H: GateSpec.hs "the shared guard (Prax.Types.authoredVarClash), pinned through a real combinator"
    //
    // The v40 world-source gate's shared-guard half (`authoredVarClash` through
    // the real `draw` combinator). The scanner half — the world-source
    // string-literal grep — is retargeted at the `.rs` worlds at S9 (KILLED.md,
    // owed:S9).
    fn is_ok(r: Result<Vec<Outcome>, WorldError>) -> bool {
        r.is_ok()
    }
    fn subq_praxd() -> Condition {
        Condition::Subquery {
            set: "S".into(),
            find: vec!["PraxD".into()],
            where_: vec![Condition::Match("seen.ok".into())],
        }
    }

    // H: GateSpec.hs "sanity: an ordinary fragment (no Prax namespace) is accepted"
    #[test]
    fn gate_ordinary_fragment_is_accepted() {
        assert!(is_ok(draw(
            1,
            2,
            vec![Condition::Match("flag.X".into())],
            vec![insert("marked.X")]
        )));
    }

    // H: GateSpec.hs "a Prax-namespaced variable in the top-level conditions is caught"
    #[test]
    fn gate_prax_in_top_level_conditions_is_caught() {
        assert!(!is_ok(draw(1, 2, vec![Condition::Match("flag.PraxD".into())], vec![])));
    }

    // H: GateSpec.hs "a Prax-namespaced variable in the top-level outcomes is caught"
    #[test]
    fn gate_prax_in_top_level_outcomes_is_caught() {
        assert!(!is_ok(draw(1, 2, vec![], vec![insert("marked.PraxW")])));
    }

    // H: GateSpec.hs "a Prax-namespaced variable nested inside a ForEach outcome's own conditions is caught"
    #[test]
    fn gate_prax_nested_in_a_foreach_guard_is_caught() {
        assert!(!is_ok(draw(
            1,
            2,
            vec![],
            vec![for_each(vec![Condition::Match("y.PraxD".into())], vec![insert("done")])]
        )));
    }

    // H: GateSpec.hs "a Prax-namespaced variable in a Subquery's free-variable list is caught"
    #[test]
    fn gate_prax_in_a_subquery_free_var_list_is_caught() {
        assert!(!is_ok(draw(1, 2, vec![subq_praxd()], vec![])));
    }
}
