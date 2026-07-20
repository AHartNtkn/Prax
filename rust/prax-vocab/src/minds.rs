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
    // H: MindsSpec.hs "Prax.Minds"
    //
    // The frozen `Prax.MindsSpec`, re-expressed against the Rust engine. Two of
    // its cases read `prax_core`'s internals (`believedWants`' provenance law and
    // the compiled want/desire tables) and are pinned in `prax_core::minds`.
    use super::*;
    use prax_core::engine::State;
    use prax_core::query::{Condition, absent, matches, neq, not_};
    use prax_core::types::{Action, Practice, delete, insert};

    fn m(s: &str) -> Condition {
        Condition::Match(s.into())
    }

    /// The tale: a vocabulary of two desires; ida professes her sweet tooth,
    /// norm-respect is conventional, and rex's grudge is neither.
    fn vocab() -> Vec<Desire> {
        vec![
            Desire::new("sweet-tooth", Want::new(vec![m("holding.Owner.cake")], 5)),
            Desire::new("grudge-rex", Want::new(vec![m("shamed.rex")], 7)),
        ]
    }

    fn world() -> State {
        let mut st = State::new();
        st.set_characters(vec![
            Character::new("ida"),
            Character::new("rex").holds("grudge-rex"),
        ])
        .unwrap();
        for o in [
            insert("character.ida"),
            insert("character.rex"),
            insert("professes.ida.sweet-tooth"),
        ] {
            st.perform_outcome(&o).unwrap();
        }
        st.set_axioms(vec![professed(), conventional()]).unwrap();
        st.set_desires(vocab()).unwrap();
        st
    }

    // H: MindsSpec.hs "a profession derives presumed motive-beliefs across the cast"
    #[test]
    fn a_profession_derives_presumed_motive_beliefs_across_the_cast() {
        let mut st = world();
        assert!(
            st.view_has("rex.believes.desires.ida.sweet-tooth.presumed"),
            "rex presumes ida's sweet tooth"
        );
        assert!(
            !st.view_has("ida.believes.desires.rex.grudge-rex.presumed"),
            "nothing derives rex's unprofessed grudge"
        );
    }

    // H: MindsSpec.hs "the profession is defeasible"
    #[test]
    fn the_profession_is_defeasible() {
        let mut st = world();
        st.perform_outcome(&delete("professes.ida.sweet-tooth"))
            .unwrap();
        assert!(
            !st.view_has("rex.believes.desires.ida.sweet-tooth.presumed"),
            "presumption dissolved"
        );
    }

    // H: MindsSpec.hs "a conventional desire is presumed of everyone — even non-holders"
    #[test]
    fn a_conventional_desire_is_presumed_of_everyone_even_non_holders() {
        let mut st = world();
        st.perform_outcome(&insert("conventional.sweet-tooth"))
            .unwrap();
        assert!(
            st.view_has("ida.believes.desires.rex.sweet-tooth.presumed"),
            "ida presumes rex's sweet tooth (he does not have one)"
        );
    }

    // H: MindsSpec.hs "gossip about a motive flips a third party's prediction"
    #[test]
    fn gossip_about_a_motive_flips_a_third_partys_prediction() {
        // rex's grudge (vocab) drives him to seek petty revenge once someone
        // believes he holds it. ida already knows (an eyewitness); she tells nia,
        // and nia's arrival at a believed model of rex flips her prediction of him
        // from unreadable to a move.
        //
        // The telling action is authored here in the shape `Prax.Rumor.gossip`
        // splices: evidence (a prefix match on the teller's belief, binding the
        // pattern's variables), the co-presence template retargeted at `Hearer`,
        // the three never-offered guards (self, the subject, an eyewitness), the
        // one-shot marker, and a hearsay deposit stamped with the teller. The
        // COMBINATOR's own laws are `RumorSpec`'s, not this label's.
        let pat = "desires.Culprit.grudge-rex";
        let tell_grudge = Action::new("[Actor]: mention [Culprit]'s grudge to [Hearer]")
            .when([
                matches(format!("Actor.believes.{pat}")),
                matches("at.Actor!P"),
                matches("at.Hearer!P"),
                neq("Hearer", "Actor"),
                neq("Hearer", "Culprit"),
                absent(vec![matches(format!("Hearer.believes.{pat}.seen"))]),
                not_(format!("Hearer.believes.{pat}.heard.Actor")),
            ])
            .then([insert(format!("Hearer.believes.{pat}.heard.Actor"))]);
        let revenge = Action::new("[Actor]: seek petty revenge").then([insert("shamed.Actor")]);
        let town = Practice::new("town")
            .roles(["Place"])
            .action(tell_grudge)
            .action(revenge);

        let mut st = State::new();
        st.define_practices(vec![town]).unwrap();
        st.set_characters(vec![
            Character::new("ida"),
            Character::new("rex").holds("grudge-rex"),
            Character::new("nia"),
        ])
        .unwrap();
        st.set_desires(vocab()).unwrap();
        for o in [
            insert("practice.town.village"),
            insert("at.ida!village"),
            insert("at.nia!village"),
            insert("ida.believes.desires.rex.grudge-rex.seen"),
        ] {
            st.perform_outcome(&o).unwrap();
        }

        let nia = Character::new("nia");
        let rex = Character::new("rex").holds("grudge-rex");
        assert_eq!(
            st.predict_move(&nia, &rex),
            None,
            "nia has no model of rex, so he is unreadable"
        );

        let mut told = st.clone();
        let ga = told
            .possible_actions("ida")
            .into_iter()
            .find(|g| g.label.contains("mention rex's grudge to nia"))
            .expect("no gossip action offered to ida");
        told.perform_action(&ga);
        assert_eq!(
            told.predict_move(&nia, &rex).map(|g| g.label),
            Some("rex: seek petty revenge".to_owned()),
            "the hearsay motive-belief makes rex predictable to nia"
        );
    }

    // H: MindsSpec.hs "wantFor grounds the Owner variable"
    #[test]
    fn want_for_grounds_owner() {
        let d = Desire::new("sweet-tooth", Want::new(vec![m("holding.Owner.cake")], 5));
        assert_eq!(want_for("ida", &d), Want::new(vec![m("holding.ida.cake")], 5));
    }

    // H: MindsSpec.hs "selfWants = unnamed wants + own named desires, instantiated"
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
