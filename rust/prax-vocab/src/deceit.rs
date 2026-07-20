//! Secrets and deception: managing what is known.
//!
//! **Concealment is authoring, not machinery.** [`conceal`] is a want that nobody
//! believe an event; the planner's lookahead already simulates the witness
//! deposits ([`crate::witness`]), so an agent who values a secret avoids being
//! seen BY PLANNING — waiting for the room to empty falls out of utility.
//!
//! **A lie is an assertion without evidence.** [`lie`] mirrors
//! [`crate::rumor::gossip`] with two inversions: the speaker must hold NO evidence
//! (if they ever hear their own lie back, the lie action vanishes and plain gossip
//! appears), and the fabricated subject binds from world-supplied conditions (whom
//! you COULD frame) rather than from a belief. The effect is identical to
//! gossip's — the deceived hold real hearsay, indistinguishable from truth, and the
//! whole rumor/reputation stack cascades on the falsehood unmodified.
//!
//! **A lie marks the liar.** The only residue is the liar's own memory —
//! `<liar>.lied.<hearer>.<event>`, rooted under their name like all mental state:
//! owned, forgettable (one `Delete` on its root), perishing with its bearer. There
//! is deliberately no objective record: history persists only through the marks it
//! makes, and the truth can become unrecoverable. What the mark buys now is
//! conscience: a trait that values your own `lied` marks negatively
//! ([`crate::persona`]) is a reason not to.
//!
//! Frozen reference: `src/Prax/Deceit.hs`.

use prax_core::error::WorldError;
use prax_core::interner::is_variable_name;
use prax_core::path::segment_names_checked;
use prax_core::query::{Condition, absent, matches, neq, not_};
use prax_core::types::{Action, Want, authored_pat_clash, authored_var_clash, insert};

use crate::beliefs::belief_about;
use crate::witness::{CoPresence, as_role};

/// A desire that nobody believe `event` — how much the secret is worth is
/// authored character. The event must be variable-free (a concealment want
/// quantifies over OBSERVERS, not deeds); the variable `Anyone` is reserved.
///
/// # Errors
/// [`WorldError::TrailingOperator`] on a malformed event;
/// [`WorldError::PatternVariables`] if the event names a variable.
pub fn conceal(event: &str, k: i32) -> Result<Want, WorldError> {
    // UNCHECKED-SPLIT is not taken: the frozen guard splits with `pathNames`,
    // which raises on a trailing operator (S7 design §12).
    let names = segment_names_checked(event)?;
    if names.iter().any(|n| is_variable_name(n)) {
        return Err(WorldError::PatternVariables {
            context: "Deceit.conceal".to_owned(),
            pattern: event.to_owned(),
            needs: "be variable-free (a concealment want quantifies over observers, not deeds)"
                .to_owned(),
        });
    }
    Ok(Want::new(
        vec![absent(vec![matches(belief_about("Anyone", event))])],
        k,
    ))
}

/// An action: assert an event you have no evidence for, to a co-present hearer.
///
/// The pattern's FIRST variable is the fabricated subject, bound by the
/// world-supplied fabrication conditions (whom you could frame); framing yourself
/// is excluded (that would be a confession, not a lie). Hearer gates are gossip's:
/// never the subject, never an eyewitness, one-shot per hearer, plus the world's
/// own gate.
///
/// # Errors
/// [`WorldError::ReservedVarClash`] if the gate, the fabrication, or the event
/// pattern authors the `Prax` namespace (the v43 guard);
/// [`WorldError::TrailingOperator`] on a malformed sentence;
/// [`WorldError::PatternVariables`] if the pattern names no one.
pub fn lie(
    copresence: &CoPresence,
    gate: Vec<Condition>,
    fabrication: Vec<Condition>,
    pat: &str,
    label: &str,
) -> Result<Action, WorldError> {
    // UNCHECKED-SPLIT is not taken: the frozen guard splits `pat` with
    // `pathNames` (S7 design §12).
    let names = segment_names_checked(pat)?;
    let mut gate_and_fabrication = gate.clone();
    gate_and_fabrication.extend(fabrication.clone());
    let mut offenders = authored_var_clash(&[], &gate_and_fabrication, &[])?;
    offenders.extend(authored_pat_clash(&[], &names));
    if let Some(v) = offenders.first() {
        return Err(WorldError::ReservedVarClash {
            context: "Deceit.lie".to_owned(),
            var: v.clone(),
            extra: " in an authored gate, fabrication, or event pattern -- the Prax namespace is reserved for machinery"
                .to_owned(),
        });
    }
    let subject =
        names
            .iter()
            .find(|n| is_variable_name(n))
            .ok_or_else(|| WorldError::PatternVariables {
                context: "Deceit.lie".to_owned(),
                pattern: pat.to_owned(),
                needs: "name someone (a lie is about someone)".to_owned(),
            })?;
    let believes_hearer = belief_about("Hearer", pat);
    let mut conds = fabrication;
    conds.extend([
        neq(subject.as_str(), "Actor"),
        // no evidence: what makes it a lie
        absent(vec![matches(belief_about("Actor", pat))]),
    ]);
    conds.extend(as_role("Hearer", copresence));
    conds.extend([
        neq("Hearer", "Actor"),
        neq("Hearer", subject.as_str()),
        absent(vec![matches(format!("{believes_hearer}.seen"))]),
        not_(format!("{believes_hearer}.heard.Actor")),
    ]);
    conds.extend(gate);
    Ok(Action::new(label).when(conds).then([
        insert(format!("{believes_hearer}.heard.Actor")),
        // the liar's own memory of the deed — a mark on their psyche, rooted
        // under their name like all mental state; there is no world ledger
        insert(format!("Actor.lied.Hearer.{pat}")),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_core::engine::State;
    use prax_core::types::{Character, Desire, Practice, delete};

    use crate::rumor::gossip;
    use crate::witness::observable;

    // H: DeceitSpec.hs "Prax.Deceit"
    //
    // The frozen `Prax.DeceitSpec`, re-expressed against the Rust engine.
    //
    // The tale: sid covets the gem but not being seen taking it matters more;
    // nia whispers lies about kit. oz and kit share the yard; mia is at the shed.

    fn together() -> CoPresence {
        vec![matches("at.Actor!P"), matches("at.Witness!P")]
    }

    fn raw_lie() -> Action {
        lie(
            &together(),
            Vec::new(),
            vec![matches("at.Culprit!Anywhere")],
            "took.Culprit.gem",
            "[Actor]: whisper to [Hearer] that [Culprit] took the gem",
        )
        .expect("the shipped lie fixture is legal")
    }

    fn world() -> State {
        let mut st = State::new();
        st.define_practices([Practice::new("vault")
            .roles(["R"])
            .action(observable(
                &together(),
                "took.Actor.gem",
                Action::new("[Actor]: take the gem")
                    .when([matches("at.Actor!yard"), matches("gem.here")])
                    .then([delete("gem.here"), insert("holding.Actor.gem")]),
            ))
            .action(
                gossip(
                    &together(),
                    Vec::new(),
                    "took.Culprit.gem",
                    "[Actor]: tell [Hearer] that [Culprit] took the gem",
                )
                .unwrap(),
            )
            .action(raw_lie())])
            .unwrap();
        st.set_characters(vec![
            Character::new("sid")
                .want(Want::new(vec![matches("holding.sid.gem")], 5))
                .want(conceal("took.sid.gem", 8).unwrap()),
            Character::new("nia").want(Want::new(vec![matches("W.believes.took.kit.gem")], 4)),
            Character::new("oz"),
            Character::new("kit"),
            Character::new("mia"),
        ])
        .unwrap();
        for o in [
            insert("practice.vault.here"),
            insert("at.sid!yard"),
            insert("at.nia!yard"),
            insert("at.oz!yard"),
            insert("at.kit!yard"),
            insert("at.mia!shed"),
            insert("gem.here"),
        ] {
            st.perform_outcome(&o).expect("deceit setup");
        }
        st
    }

    fn char_named(st: &State, n: &str) -> Character {
        st.characters()
            .iter()
            .find(|c| c.name == n)
            .unwrap_or_else(|| panic!("no such character: {n}"))
            .clone()
    }

    /// Perform the named actor's action whose label mentions `needle`.
    fn do_act(who: &str, needle: &str, st: &mut State) {
        let found = st
            .possible_actions(who)
            .into_iter()
            .find(|ga| ga.label.contains(needle));
        match found {
            Some(ga) => st.perform_action(&ga),
            None => panic!(
                "no action for {who} matching {needle:?}; had: {:?}",
                st.possible_actions(who)
                    .into_iter()
                    .map(|ga| ga.label)
                    .collect::<Vec<_>>()
            ),
        }
    }

    fn offered(who: &str, needle: &str, st: &mut State) -> bool {
        st.possible_actions(who)
            .iter()
            .any(|ga| ga.label.contains(needle))
    }

    /// Dedicated fixture for testing [`lie`] over motive patterns. Same structure
    /// as [`world`], but with a "revenge" desire and a motive-pattern lie action
    /// in the practice.
    fn motive_lie_world() -> State {
        let mut st = State::new();
        st.define_practices([Practice::new("rumor").roles(["R"]).action(
            lie(
                &together(),
                Vec::new(),
                vec![matches("at.Culprit!Anywhere")],
                "desires.Culprit.revenge",
                "[Actor]: whisper to [Hearer] that [Culprit] wants revenge",
            )
            .unwrap(),
        )])
        .unwrap();
        st.set_characters(
            ["nia", "oz", "kit"]
                .into_iter()
                .map(Character::new)
                .collect(),
        )
        .unwrap();
        st.set_desires(vec![Desire::new(
            "revenge",
            Want::new(vec![matches("harms.Owner")], 10),
        )])
        .unwrap();
        for o in [
            insert("practice.rumor.here"),
            insert("at.nia!yard"),
            insert("at.oz!yard"),
            insert("at.kit!yard"),
        ] {
            st.perform_outcome(&o).expect("motive-lie setup");
        }
        st
    }

    // H: DeceitSpec.hs "conceal is the nobody-believes want"
    #[test]
    fn conceal_is_the_nobody_believes_want() {
        assert_eq!(
            conceal("took.sid.gem", 8).unwrap(),
            Want::new(
                vec![absent(vec![matches("Anyone.believes.took.sid.gem")])],
                8
            )
        );
    }

    // H: DeceitSpec.hs "conceal rejects a variable-bearing event loudly"
    #[test]
    fn conceal_rejects_a_variable_bearing_event_loudly() {
        assert!(
            conceal("took.Who.gem", 8).is_err(),
            "variables in a secret are an error"
        );
    }

    // H: DeceitSpec.hs "a concealer waits for privacy, then acts, and no one knows"
    #[test]
    fn a_concealer_waits_for_privacy_then_acts() {
        let mut st = world();
        let sid = char_named(&st, "sid");
        assert_ne!(
            st.pick_action(2, &sid).map(|ga| ga.label),
            Some("sid: take the gem".to_owned()),
            "watched: sid does not take the gem"
        );
        for o in [
            insert("at.nia!shed"),
            insert("at.oz!shed"),
            insert("at.kit!shed"),
        ] {
            st.perform_outcome(&o).unwrap();
        }
        assert_eq!(
            st.pick_action(2, &sid).map(|ga| ga.label),
            Some("sid: take the gem".to_owned())
        );
        do_act("sid", "take the gem", &mut st);
        assert!(st.db_has("holding.sid.gem"), "took it");
        for w in ["nia", "oz", "kit", "mia"] {
            assert!(
                !st.db_has(&format!("{w}.believes.took.sid.gem")),
                "and nobody believes it"
            );
        }
    }

    // H: DeceitSpec.hs "a lie plants sourced hearsay the liar never had evidence for"
    #[test]
    fn a_lie_plants_sourced_hearsay_the_liar_never_had_evidence_for() {
        let mut st = world();
        do_act("nia", "whisper to oz that kit took the gem", &mut st);
        assert!(
            st.db_has("oz.believes.took.kit.gem.heard.nia"),
            "oz heard it from nia"
        );
        assert!(
            !st.db_has("nia.believes.took.kit.gem"),
            "nia still has no evidence of her own"
        );
    }

    // H: DeceitSpec.hs "hearing your own lie back turns it into gossip"
    #[test]
    fn hearing_your_own_lie_back_turns_it_into_gossip() {
        let mut st = world();
        do_act("nia", "whisper to oz that kit took the gem", &mut st);
        do_act("oz", "tell nia that kit took the gem", &mut st);
        assert!(
            st.db_has("nia.believes.took.kit.gem.heard.oz"),
            "nia now holds (fabricated) evidence"
        );
        assert!(
            !offered("nia", "whisper to oz that kit", &mut st)
                && !st
                    .possible_actions("nia")
                    .iter()
                    .any(|ga| { ga.label.contains("whisper") && ga.label.contains("kit took") }),
            "the lie action is gone (its no-evidence gate closed)"
        );
        assert!(
            offered("nia", "tell oz that kit took", &mut st)
                || offered("nia", "tell mia that kit took", &mut st)
                || offered("nia", "tell sid that kit took", &mut st),
            "plain gossip appears in its place"
        );
    }

    // H: DeceitSpec.hs "you cannot frame yourself (a lie about yourself is a confession)"
    #[test]
    fn you_cannot_frame_yourself() {
        let mut st = world();
        assert!(
            !st.possible_actions("nia")
                .iter()
                .any(|ga| ga.label.contains("whisper") && ga.label.contains("nia took")),
            "no whisper names the whisperer"
        );
    }

    // H: DeceitSpec.hs "the subject of the lie is never the hearer"
    #[test]
    fn the_subject_of_the_lie_is_never_the_hearer() {
        let mut st = world();
        assert!(
            !st.possible_actions("nia")
                .iter()
                .any(|ga| ga.label.contains("whisper to kit that kit")),
            "no whispering to kit about kit"
        );
    }

    // H: DeceitSpec.hs "lying to the same hearer twice is not offered"
    #[test]
    fn lying_to_the_same_hearer_twice_is_not_offered() {
        let mut st = world();
        do_act("nia", "whisper to oz that kit took the gem", &mut st);
        assert!(
            !offered("nia", "whisper to oz that kit", &mut st),
            "one-shot per hearer"
        );
    }

    // H: DeceitSpec.hs "a subject-less pattern errors loudly"
    #[test]
    fn a_subject_less_pattern_errors_loudly() {
        assert!(
            matches!(
                lie(
                    &together(),
                    Vec::new(),
                    Vec::new(),
                    "somethinghappened",
                    "[Actor]: mention it to [Hearer]"
                ),
                Err(WorldError::PatternVariables { .. })
            ),
            "a lie must be about someone"
        );
    }

    // H: DeceitSpec.hs "a lie can fabricate a MOTIVE: desires.* patterns work like deed patterns"
    #[test]
    fn a_lie_can_fabricate_a_motive() {
        // nia whispers that kit nurses a revenge desire — evidence-free motive
        // framing.
        let mut st = motive_lie_world();
        assert!(
            !offered("nia", "whisper to kit that kit wants", &mut st),
            "kit is never offered as hearer (subject cannot be hearer)"
        );
        do_act("nia", "whisper to oz that kit wants revenge", &mut st);
        assert!(
            st.db_has("oz.believes.desires.kit.revenge.heard.nia"),
            "oz believes kit desires revenge (heard from nia)"
        );
        assert!(
            !st.db_has("nia.believes.desires.kit.revenge"),
            "nia keeps no evidence of her own motive claim"
        );
    }

    // H: DeceitSpec.hs "a lie marks the liar's own memory (marks, not records)"
    #[test]
    fn a_lie_marks_the_liars_own_memory() {
        let mut st = world();
        do_act("nia", "whisper to oz that kit took the gem", &mut st);
        assert!(
            st.db_has("nia.lied.oz.took.kit.gem"),
            "nia carries the mark of her own lie"
        );
        assert!(!st.db_has("oz.lied"), "the deceived carry no such mark");
        // the mark is the liar's psyche, not a world ledger: one Delete on its
        // root retracts the memory (PersonaSpec shows the conscience-cost
        // clearing with it)
        st.perform_outcome(&delete("nia.lied")).unwrap();
        assert!(!st.db_has("nia.lied"), "forgetting clears it");
    }

    // H: DeceitSpec.hs "v43: the missing namespace guard (previously latent: an authored gate, fabrication, or event pattern had no guard at all)"
    // H: DeceitSpec.hs "a gate authoring the Prax namespace is a loud construction-time error"
    #[test]
    fn a_gate_authoring_the_prax_namespace_is_rejected() {
        assert!(
            lie(
                &together(),
                vec![not_("flag.PraxD")],
                vec![matches("at.Culprit!Anywhere")],
                "took.Culprit.gem",
                "[Actor]: whisper to [Hearer] that [Culprit] took the gem"
            )
            .is_err(),
            "the Prax namespace in the gate is rejected"
        );
    }

    // H: DeceitSpec.hs "the fabrication conditions authoring the Prax namespace is a loud construction-time error"
    #[test]
    fn a_fabrication_authoring_the_prax_namespace_is_rejected() {
        assert!(
            lie(
                &together(),
                Vec::new(),
                vec![matches("at.Culprit!PraxAnywhere")],
                "took.Culprit.gem",
                "[Actor]: whisper to [Hearer] that [Culprit] took the gem"
            )
            .is_err(),
            "the Prax namespace in fabrication is rejected"
        );
    }

    // H: DeceitSpec.hs "an event pattern authoring the Prax namespace is a loud construction-time error"
    #[test]
    fn an_event_pattern_authoring_the_prax_namespace_is_rejected() {
        assert!(
            lie(
                &together(),
                Vec::new(),
                vec![matches("at.Culprit!Anywhere")],
                "took.PraxCulprit.gem",
                "[Actor]: whisper to [Hearer] that [Culprit] took the gem"
            )
            .is_err(),
            "the Prax namespace in the event pattern is rejected"
        );
    }

    // H: DeceitSpec.hs "the shipped lie fixture (an ordinary gate, fabrication, and pattern) is not rejected"
    #[test]
    fn the_shipped_lie_fixture_is_not_rejected() {
        assert!(
            lie(
                &together(),
                Vec::new(),
                vec![matches("at.Culprit!Anywhere")],
                "took.Culprit.gem",
                "[Actor]: whisper to [Hearer] that [Culprit] took the gem"
            )
            .is_ok(),
            "the legal shape is not rejected"
        );
    }

    // H: DeceitSpec.hs "truthful gossip leaves no lie-mark"
    #[test]
    fn truthful_gossip_leaves_no_lie_mark() {
        // sid takes the gem before witnesses; oz (an eyewitness) honestly tells
        // mia, who is walked over from the shed to be a valid hearer.
        let mut st = world();
        do_act("sid", "take the gem", &mut st);
        st.perform_outcome(&insert("at.mia!yard")).unwrap();
        do_act("oz", "tell mia that sid took the gem", &mut st);
        assert!(
            st.db_has("mia.believes.took.sid.gem.heard.oz"),
            "mia holds the hearsay"
        );
        assert!(!st.db_has("oz.lied"), "an honest telling marks nothing");
    }
}
