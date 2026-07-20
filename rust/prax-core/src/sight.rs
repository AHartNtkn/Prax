//! The perceptual layer's scope fragment. `sightRule` (the period-1 engine rule
//! that deposits `<seer>.believes.at`/`atSince` for every co-present pair) landed
//! at S5; S6 owns `Prax.Sight`'s remainder: [`sighted_within`], the `atSince`
//! window a world `Or`s with co-presence-now in its prediction scope.
//!
//! Frozen reference: `src/Prax/Sight.hs` (`sightedWithin`).

use crate::query::{CalcOp, CmpOp, Condition, calc, cmp, matches};

/// Scope fragment over `Actor`/`Witness` (`Prax.Sight.sightedWithin`): the
/// Witness was sighted within the last `h` ticks. The stamp
/// (`Actor.believes.atSince.Witness!Since`) plus the clock (`turn!Now`) feed a
/// `Since + h ≥ Now` window. Worlds `Or` this with co-presence-now.
pub fn sighted_within(h: i64) -> Vec<Condition> {
    vec![
        matches("Actor.believes.atSince.Witness!Since"),
        matches("turn!Now"),
        calc("Expiry", CalcOp::Add, "Since", h.to_string()),
        cmp(CmpOp::Gte, "Expiry", "Now"),
    ]
}

#[cfg(test)]
mod tests {
    // H: SightSpec.hs "sightedWithin is a window over the stamp"
    use super::*;

    #[test]
    fn sighted_within_is_the_four_condition_window() {
        assert_eq!(
            sighted_within(2),
            vec![
                Condition::Match("Actor.believes.atSince.Witness!Since".into()),
                Condition::Match("turn!Now".into()),
                Condition::Calc("Expiry".into(), CalcOp::Add, "Since".into(), "2".into()),
                Condition::Cmp(CmpOp::Gte, "Expiry".into(), "Now".into()),
            ]
        );
    }
}
