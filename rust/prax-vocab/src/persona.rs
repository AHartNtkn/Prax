//! Personality as conduct, not goals (spec
//! `docs/specs/2026-07-11-v25-persona-design.md`).
//!
//! A trait is a named bundle of (mostly negative) desires over the bearer's OWN
//! conduct-marks — `honest` values each of your own `lied` marks
//! ([`crate::deceit`]) at a cost. A bearer can do anything: trait-contrary
//! conduct carries negative utility, never prohibition, and the arithmetic is the
//! meaning ("her honesty outweighs her spite, by exactly the margin written").
//! Because the mark lands in the very state the planner evaluates, deterrence
//! needs no lookahead; and because trait desires are named vocabulary
//! ([`crate::minds`]), a BELIEVED temperament nets against believed motives
//! inside prediction — knowing someone honest changes what you expect of them,
//! not just what they do.
//!
//! Goal-bundles are deliberately NOT traits: a goal is a plain desire needing no
//! bundle. A trait says how you're willing to act, not what you're after.
//!
//! Frozen reference: `src/Prax/Persona.hs`. [`transparent`] is an AXIOM builder
//! (S7 design §3.4).

use prax_core::error::WorldError;
use prax_core::query::matches;
use prax_core::types::{Axiom, Character, Desire, Outcome, insert};

/// A named bundle of conduct-valuations. The name must be a single path segment
/// (it becomes one in `trait.<who>.<name>` facts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trait {
    pub name: String,
    /// Valuations over the bearer's own conduct-marks.
    pub desires: Vec<Desire>,
}

impl Trait {
    pub fn new(name: impl Into<String>, desires: Vec<Desire>) -> Trait {
        Trait {
            name: name.into(),
            desires,
        }
    }
}

/// The desires a trait list contributes to a world's vocabulary.
///
/// # Errors
/// [`WorldError::NotASinglePathSegment`] — reused here for its "a name that must
/// be unique/well-formed was not" role — on a duplicate desire name (bundles must
/// not collide).
pub fn persona_vocabulary(traits: &[Trait]) -> Result<Vec<Desire>, WorldError> {
    let ds: Vec<Desire> = traits.iter().flat_map(|t| t.desires.clone()).collect();
    let mut seen: Vec<&str> = Vec::new();
    for d in &ds {
        if seen.contains(&d.name.as_str()) {
            return Err(WorldError::NotASinglePathSegment {
                context: "Persona.persona_vocabulary: duplicate desire name".to_owned(),
                name: d.name.clone(),
            });
        }
        seen.push(&d.name);
    }
    Ok(ds)
}

/// Endow a character with a trait: they hold each of its desires by name.
#[must_use]
pub fn bearing(t: &Trait, c: Character) -> Character {
    let mut out = c;
    out.desires.extend(t.desires.iter().map(|d| d.name.clone()));
    out
}

/// Temperament is worn on the sleeve: every character presumes a bearer's
/// conduct-valuations. Defeasible (`.presumed`) like all derived belief.
pub fn transparent() -> Axiom {
    Axiom::new(
        vec![
            matches("trait.M.T"),
            matches("traitDesire.T.D"),
            matches("character.P"),
        ],
        ["P.believes.desires.M.D.presumed"],
    )
}

/// Deterministic roster assembly over hand-authored base characters: each member
/// [`bearing`] their traits, plus the setup facts [`transparent`] reads —
/// `trait.<who>.<name>` per bearing, `traitDesire.<trait>.<desire>` once per
/// trait, `character.<who>` per member.
///
/// # Errors
/// [`WorldError::NotASinglePathSegment`] for a trait name that is not a nonempty
/// single path segment, for a duplicate trait, and for a borne trait missing from
/// the vocabulary list (its valuations would be silently illegible) — in the
/// frozen guard order.
pub fn cast(
    traits: &[Trait],
    roster: Vec<(Character, Vec<Trait>)>,
) -> Result<(Vec<Character>, Vec<Outcome>), WorldError> {
    let tnames: Vec<&str> = traits.iter().map(|t| t.name.as_str()).collect();
    if let Some(bad) = tnames
        .iter()
        .find(|n| n.is_empty() || n.contains(['.', '!']))
    {
        return Err(WorldError::NotASinglePathSegment {
            context: "Persona.cast: trait name".to_owned(),
            name: (*bad).to_owned(),
        });
    }
    let mut seen: Vec<&str> = Vec::new();
    for n in &tnames {
        if seen.contains(n) {
            return Err(WorldError::NotASinglePathSegment {
                context: "Persona.cast: duplicate trait".to_owned(),
                name: (*n).to_owned(),
            });
        }
        seen.push(n);
    }
    if let Some(stray) = roster
        .iter()
        .flat_map(|(_, ts)| ts.iter())
        .find(|t| !tnames.contains(&t.name.as_str()))
    {
        return Err(WorldError::NotASinglePathSegment {
            context: "Persona.cast: trait is borne but not in the trait list \
                      (its valuations would be silently illegible)"
                .to_owned(),
            name: stray.name.clone(),
        });
    }
    let members: Vec<Character> = roster
        .iter()
        .map(|(c, ts)| ts.iter().fold(c.clone(), |acc, t| bearing(t, acc)))
        .collect();
    let mut outs: Vec<Outcome> = traits
        .iter()
        .flat_map(|t| {
            t.desires
                .iter()
                .map(move |d| insert(format!("traitDesire.{}.{}", t.name, d.name)))
        })
        .collect();
    outs.extend(
        roster
            .iter()
            .map(|(c, _)| insert(format!("character.{}", c.name))),
    );
    outs.extend(roster.iter().flat_map(|(c, ts)| {
        ts.iter()
            .map(move |t| insert(format!("trait.{}.{}", c.name, t.name)))
    }));
    Ok((members, outs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_core::engine::State;
    use prax_core::types::{Action, Practice, Want, delete};

    use crate::deceit::lie;
    use crate::witness::CoPresence;

    // H: PersonaSpec.hs "Prax.Persona"
    //
    // The frozen `Prax.PersonaSpec`, re-expressed against the Rust engine.

    fn together() -> CoPresence {
        vec![matches("at.Actor!P"), matches("at.Witness!P")]
    }

    /// Temperament: each of your own lie-marks costs you 6 — more than the 4 a
    /// deceived head is worth below, less than everything. Costs, not
    /// prohibitions.
    fn plainspoken() -> Trait {
        Trait::new(
            "plainspoken",
            vec![Desire::new(
                "clean-conscience",
                Want::new(vec![matches("Owner.lied.H.took.C.gem")], -6),
            )],
        )
    }

    /// The tale: ada and bea both covet oz's credulity (+4 if oz believes kit
    /// took the gem — a want only a lie can serve here, the temptation); ada is
    /// plainspoken, bea is her unprincipled twin. Same motive, one temperament.
    fn world() -> State {
        let (roster, persona_facts) = cast(
            &[plainspoken()],
            vec![
                (
                    Character::new("ada").holds("covets-credulity"),
                    vec![plainspoken()],
                ),
                (Character::new("bea").holds("covets-credulity"), Vec::new()),
                (Character::new("oz"), Vec::new()),
                (Character::new("kit"), Vec::new()),
            ],
        )
        .expect("the persona roster");

        let mut st = State::new();
        st.define_practices([Practice::new("yard")
            .roles(["R"])
            .action(
                lie(
                    &together(),
                    Vec::new(),
                    vec![matches("at.Culprit!Anywhere")],
                    "took.Culprit.gem",
                    "[Actor]: whisper to [Hearer] that [Culprit] took the gem",
                )
                .expect("the persona fixture's lie"),
            )
            .action(Action::new("[Actor]: hold your peace").when([matches("at.Actor!P")]))])
            .unwrap();
        st.set_axioms(vec![transparent()]).unwrap();
        st.set_characters(roster).unwrap();
        let mut desires = vec![Desire::new(
            "covets-credulity",
            Want::new(vec![matches("oz.believes.took.kit.gem")], 4),
        )];
        desires.extend(persona_vocabulary(&[plainspoken()]).unwrap());
        st.set_desires(desires).unwrap();
        for o in persona_facts.iter().chain(
            [
                insert("practice.yard.here"),
                insert("at.ada!yard"),
                insert("at.bea!yard"),
                insert("at.oz!yard"),
                insert("at.kit!yard"),
            ]
            .iter(),
        ) {
            st.perform_outcome(o).expect("persona setup");
        }
        st
    }

    fn member(st: &State, n: &str) -> Character {
        st.characters()
            .iter()
            .find(|c| c.name == n)
            .unwrap_or_else(|| panic!("no such member: {n}"))
            .clone()
    }

    // H: PersonaSpec.hs "bearing endows a character with the trait's desires by name"
    #[test]
    fn bearing_endows_a_character_with_the_traits_desires_by_name() {
        assert_eq!(
            bearing(&plainspoken(), Character::new("zed")).desires,
            vec!["clean-conscience".to_owned()]
        );
    }

    // H: PersonaSpec.hs "cast assembles the roster and the facts transparent reads"
    #[test]
    fn cast_assembles_the_roster_and_the_facts_transparent_reads() {
        let mut st = world();
        assert!(st.db_has("trait.ada.plainspoken"), "ada bears the trait");
        assert!(
            st.db_has("traitDesire.plainspoken.clean-conscience"),
            "the trait's desires are data facts"
        );
        for n in ["ada", "bea", "oz", "kit"] {
            assert!(
                st.db_has(&format!("character.{n}")),
                "every member is a character fact"
            );
        }
        assert!(!st.db_has("trait.bea"), "bea bears nothing");
        assert_eq!(
            member(&st, "ada").desires,
            vec!["covets-credulity".to_owned(), "clean-conscience".to_owned()]
        );
    }

    // H: PersonaSpec.hs "personaVocabulary rejects duplicate desire names loudly"
    #[test]
    fn persona_vocabulary_rejects_duplicate_desire_names_loudly() {
        let echo = Trait::new(
            "echo",
            vec![Desire::new(
                "clean-conscience",
                Want::new(vec![matches("x.y")], 1),
            )],
        );
        assert!(
            persona_vocabulary(&[plainspoken(), echo]).is_err(),
            "bundles must not collide"
        );
    }

    // H: PersonaSpec.hs "cast rejects a dotted trait name loudly"
    #[test]
    fn cast_rejects_a_dotted_trait_name_loudly() {
        assert!(
            cast(&[Trait::new("two.part", Vec::new())], Vec::new()).is_err(),
            "a trait name is a single path segment"
        );
    }

    // H: PersonaSpec.hs "cast rejects a borne trait missing from the vocabulary"
    #[test]
    fn cast_rejects_a_borne_trait_missing_from_the_vocabulary() {
        assert!(
            cast(&[], vec![(Character::new("zed"), vec![plainspoken()])]).is_err(),
            "a stray bearing would be silently illegible"
        );
    }

    // H: PersonaSpec.hs "temperament is legible: everyone presumes a bearer's valuations"
    #[test]
    fn temperament_is_legible() {
        let mut st = world();
        assert!(
            st.view_has("oz.believes.desires.ada.clean-conscience.presumed"),
            "oz presumes ada's conscience"
        );
        assert!(
            st.view_has("bea.believes.desires.ada.clean-conscience.presumed"),
            "bea presumes it too"
        );
        assert!(
            !st.view_has("oz.believes.desires.bea.clean-conscience"),
            "no conscience is presumed of bea (she bears no trait)"
        );
        assert!(
            !st.view_has("oz.believes.desires.ada.covets-credulity"),
            "the covets want, unheralded, is presumed of no one"
        );
    }

    // H: PersonaSpec.hs "the conduct-valuation core: the temptation splits the twins"
    #[test]
    fn the_temptation_splits_the_twins() {
        // identical motives, identical affordances; only the trait differs.
        let mut st = world();
        let bea = member(&st, "bea");
        let ada = member(&st, "ada");
        assert_eq!(
            st.pick_action(2, &bea).map(|ga| ga.label),
            Some("bea: whisper to oz that kit took the gem".to_owned())
        );
        assert_eq!(
            st.pick_action(2, &ada).map(|ga| ga.label),
            Some("ada: hold your peace".to_owned())
        );
    }

    // H: PersonaSpec.hs "a conscience with a memory: each mark costs again, and forgetting relieves"
    #[test]
    fn a_conscience_with_a_memory() {
        let mut st = world();
        let ada = member(&st, "ada");
        // plant one prior lie on ada's psyche: the NEXT lie still nets -2 (no
        // fall-from-grace discount) …
        st.perform_outcome(&insert("ada.lied.bea.took.oz.gem"))
            .unwrap();
        assert_eq!(
            st.pick_action(2, &ada).map(|ga| ga.label),
            Some("ada: hold your peace".to_owned())
        );
        // … and the arithmetic is per-mark, exactly:
        assert_eq!(st.evaluate_self_wants(&ada), -6);
        st.perform_outcome(&insert("ada.lied.oz.took.bea.gem"))
            .unwrap();
        assert_eq!(st.evaluate_self_wants(&ada), -12);
        st.perform_outcome(&delete("ada.lied")).unwrap();
        assert_eq!(st.evaluate_self_wants(&ada), 0);
    }

    // H: PersonaSpec.hs "a believed conscience nets against a believed motive in prediction"
    #[test]
    fn a_believed_conscience_nets_against_a_believed_motive_in_prediction() {
        // kit is told both women covet oz's credulity; ada's conscience he has
        // presumed all along (transparent). Motive alone predicts the whisper;
        // motive netted against conscience predicts nothing.
        let mut st = world();
        for o in [
            insert("kit.believes.desires.ada.covets-credulity.seen"),
            insert("kit.believes.desires.bea.covets-credulity.seen"),
        ] {
            st.perform_outcome(&o).unwrap();
        }
        let kit = member(&st, "kit");
        let bea = member(&st, "bea");
        let ada = member(&st, "ada");
        assert_eq!(
            st.predict_move(&kit, &bea).map(|ga| ga.label),
            Some("bea: whisper to oz that kit took the gem".to_owned())
        );
        assert_eq!(st.predict_move(&kit, &ada), None);
    }
}
