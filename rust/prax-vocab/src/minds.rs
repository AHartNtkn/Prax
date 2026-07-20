//! Minds as objects of belief — the frozen library surface (`Prax.Minds`'s
//! diagnostics and the common-knowledge axiom builders), split from the planner's
//! core trio [D-I3] which lives in `prax_core::minds`. `MindsSpec` pins land here.
//!
//! To believe something about a mind, minds must be nameable: a world declares a
//! vocabulary of named, `Owner`-parameterized desires, and a motive-belief is an
//! ordinary belief over `desires.<owner>.<name>` in the provenance shape — so the
//! whole information stack works on minds unchanged. Common knowledge is derived,
//! defeasibly: [`professed`] spreads an openly-held desire; [`conventional`]
//! presumes a desire of everyone.
//!
//! Frozen reference: `src/Prax/Minds.hs`
//! (`wantFor`/`selfWants`/`professed`/`conventional`).

use std::collections::BTreeMap;

use prax_core::query::matches;
use prax_core::types::{Axiom, Character, Desire, Want, rename_vars};

/// Instantiate a desire template for its owner — grounds `Owner`
/// (`Prax.Minds.wantFor`). A string-surfaced diagnostic: the planner scores the
/// cooked mirror, never this.
pub fn want_for(owner: &str, d: &Desire) -> Want {
    let subst: BTreeMap<String, String> = BTreeMap::from([("Owner".to_owned(), owner.to_owned())]);
    Want::new(rename_vars(&subst, &d.want.when), d.want.utility)
}

/// What a character plans with: their whole mind — unnamed wants plus their own
/// named desires, instantiated (`Prax.Minds.selfWants`).
pub fn self_wants(desires: &[Desire], c: &Character) -> Vec<Want> {
    let mut out = c.wants.clone();
    for d in desires {
        if c.desires.contains(&d.name) {
            out.push(want_for(&c.name, d));
        }
    }
    out
}

/// An openly-held desire is presumed known by everyone (`Prax.Minds.professed`):
/// `professes.<owner>.<name>` ⇒ every character presumes it.
pub fn professed() -> Axiom {
    Axiom::new(
        vec![matches("professes.Owner.D"), matches("character.P")],
        ["P.believes.desires.Owner.D.presumed"],
    )
}

/// A conventional desire is presumed of everyone by everyone — even of those who
/// do not actually have it (`Prax.Minds.conventional`).
pub fn conventional() -> Axiom {
    Axiom::new(
        vec![
            matches("conventional.D"),
            matches("character.P"),
            matches("character.M"),
        ],
        ["P.believes.desires.M.D.presumed"],
    )
}

#[cfg(test)]
mod tests {
    // H: MindsSpec.hs "wantFor grounds the Owner variable"
    // H: MindsSpec.hs "selfWants = unnamed wants + own named desires, instantiated"
    use super::*;
    use prax_core::query::Condition;

    fn m(s: &str) -> Condition {
        Condition::Match(s.into())
    }

    #[test]
    fn want_for_grounds_owner() {
        let d = Desire::new("sweet-tooth", Want::new(vec![m("holding.Owner.cake")], 5));
        assert_eq!(want_for("ida", &d), Want::new(vec![m("holding.ida.cake")], 5));
    }

    #[test]
    fn self_wants_is_unnamed_plus_instantiated_desires() {
        let vocab = vec![
            Desire::new("sweet-tooth", Want::new(vec![m("holding.Owner.cake")], 5)),
            Desire::new("grudge-rex", Want::new(vec![m("shamed.rex")], 7)),
        ];
        let rex = Character::new("rex").want(Want::new(vec![m("x")], 1)).holds("grudge-rex");
        assert_eq!(
            self_wants(&vocab, &rex),
            vec![Want::new(vec![m("x")], 1), Want::new(vec![m("shamed.rex")], 7)]
        );
    }
}
