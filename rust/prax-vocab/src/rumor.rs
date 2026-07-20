//! Rumor propagation: telling what you have evidence for.
//!
//! With witnessing ([`crate::witness`]) this closes the information loop: what
//! happens in front of people travels beyond them. A tell plants the same
//! event-belief in the hearer with HEARSAY PROVENANCE —
//! `<hearer>.believes.<event>.heard.<teller>`, one edge per teller, beside any
//! `.seen` edge — so evidence accumulates (corroboration is countable) and an
//! eyewitness record is never overwritten.
//!
//! Like observability, WHAT IS TELLABLE IS AUTHORED: [`gossip`] declares one
//! tell-action per event pattern (a generic "share any belief" is impossible
//! anyway — a query variable binds a single path segment). Spreading is
//! want-driven: author a character who wants others to know, and the ordinary
//! planner carries the news.
//!
//! Frozen reference: `src/Prax/Rumor.hs`.

use prax_core::error::WorldError;
use prax_core::interner::is_variable_name;
use prax_core::path::segment_names_checked;
use prax_core::query::{Condition, absent, exists, matches, neq, not_};
use prax_core::types::{Action, authored_pat_clash, authored_var_clash, insert};

use crate::beliefs::belief_about;
use crate::witness::{CoPresence, as_role};

/// An action: tell a co-present hearer about an event you have evidence for.
///
/// The event pattern may use variables (e.g. `"stole.Culprit.Item"`); its FIRST
/// variable is the rumor's subject, who is never offered as a hearer (you don't
/// tell bob about bob's theft). Also never offered: the teller themself, an
/// eyewitness (no news value), and anyone this teller has already told
/// (`.heard.<teller>` doubles as the one-shot marker). The world adds its own
/// gate (e.g. "not someone you distrust") and its [`CoPresence`] template —
/// written over `Witness`, retargeted at the `Hearer` via
/// [`crate::witness::as_role`].
///
/// The evidence condition is a PREFIX match on `believes.<event>`: the node
/// exists iff some provenance edge sits beneath it, and matching the prefix binds
/// the pattern's variables exactly once per known event no matter how many
/// provenance edges there are — no duplicate tells for a teller who both saw and
/// heard. The event pattern's namespace must not overlap any valued-belief issue
/// path in the same world — [`crate::witness`] and gossip deposits must be the
/// only writers under the pattern's `believes.` prefix.
///
/// # Errors
/// [`WorldError::ReservedVarClash`] if the gate or the event pattern authors the
/// `Prax` namespace (the v43 guard); [`WorldError::TrailingOperator`] on a
/// malformed gate sentence or pattern; [`WorldError::PatternVariables`] if the
/// pattern names no one (a rumor is about someone).
pub fn gossip(
    copresence: &CoPresence,
    gate: Vec<Condition>,
    pat: &str,
    label: &str,
) -> Result<Action, WorldError> {
    // UNCHECKED-SPLIT is not taken: the frozen guard splits `pat` with
    // `pathNames`, which raises on a trailing operator (S7 design §12).
    let names = segment_names_checked(pat)?;
    let mut offenders = authored_var_clash(&[], &gate, &[])?;
    offenders.extend(authored_pat_clash(&[], &names));
    if let Some(v) = offenders.first() {
        return Err(WorldError::ReservedVarClash {
            context: "Rumor.gossip".to_owned(),
            var: v.clone(),
            extra: " in an authored gate or event pattern -- the Prax namespace is reserved for machinery"
                .to_owned(),
        });
    }
    let subject =
        names
            .iter()
            .find(|n| is_variable_name(n))
            .ok_or_else(|| WorldError::PatternVariables {
                context: "Rumor.gossip".to_owned(),
                pattern: pat.to_owned(),
                needs: "name someone (a rumor is about someone)".to_owned(),
            })?;
    let believes_hearer = belief_about("Hearer", pat);
    // any evidence; binds the pattern's variables
    let mut conds = vec![matches(belief_about("Actor", pat))];
    conds.extend(as_role("Hearer", copresence));
    conds.extend([
        neq("Hearer", "Actor"),
        neq("Hearer", subject.as_str()),
        absent(vec![matches(format!("{believes_hearer}.seen"))]),
        not_(format!("{believes_hearer}.heard.Actor")),
    ]);
    conds.extend(gate);
    Ok(Action::new(label)
        .when(conds)
        .then([insert(format!("{believes_hearer}.heard.Actor"))]))
}

/// Condition: `who` has hearsay evidence of `event` (from anyone). A boolean ∃,
/// so multiple sources yield one row — corroboration never duplicates an
/// affordance.
pub fn heard(who: &str, event: &str) -> Condition {
    exists(vec![matches(format!(
        "{}.heard.Src",
        belief_about(who, event)
    ))])
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_core::engine::State;
    use prax_core::types::{Character, Practice};

    // H: RumorSpec.hs "Prax.Rumor"
    //
    // The frozen `Prax.RumorSpec`, re-expressed against the Rust engine.
    //
    // The tale: tess tripped. sam and rita saw it; hana and pip know nothing.
    // The world's gate: you don't gossip with someone you hold a grudge against.

    /// One yard; everyone in it can be told.
    fn together() -> CoPresence {
        vec![matches("at.Actor!P"), matches("at.Witness!P")]
    }

    fn tell() -> Action {
        gossip(
            &together(),
            vec![not_("grudge.Actor.Hearer")],
            "tripped.Klutz",
            "[Actor]: tell [Hearer] that [Klutz] tripped",
        )
        .expect("the shipped tell fixture is legal")
    }

    fn world() -> State {
        let mut st = State::new();
        st.define_practices([Practice::new("yard").roles(["R"]).action(tell())])
            .unwrap();
        st.set_characters(
            ["sam", "rita", "hana", "tess", "pip"]
                .into_iter()
                .map(Character::new)
                .collect(),
        )
        .unwrap();
        for o in [
            insert("practice.yard.here"),
            insert("at.sam!yard"),
            insert("at.rita!yard"),
            insert("at.hana!yard"),
            insert("at.tess!yard"),
            insert("at.pip!yard"),
            insert("sam.believes.tripped.tess.seen"),
            insert("rita.believes.tripped.tess.seen"),
        ] {
            st.perform_outcome(&o).expect("rumor setup");
        }
        st
    }

    /// `teller` performs their tell aimed at `hearer`.
    fn tell_to(teller: &str, hearer: &str, st: &mut State) {
        let needle = format!("tell {hearer}");
        let found = st
            .possible_actions(teller)
            .into_iter()
            .find(|ga| ga.label.contains(&needle));
        match found {
            Some(ga) => st.perform_action(&ga),
            None => panic!(
                "no tell to {hearer} offered to {teller}; had: {:?}",
                st.possible_actions(teller)
                    .into_iter()
                    .map(|ga| ga.label)
                    .collect::<Vec<_>>()
            ),
        }
    }

    /// Is `teller` offered a tell to `hearer`?
    fn offers(teller: &str, hearer: &str, st: &mut State) -> bool {
        let needle = format!("tell {hearer}");
        st.possible_actions(teller)
            .iter()
            .any(|ga| ga.label.contains(&needle))
    }

    // H: RumorSpec.hs "telling plants hearsay with the teller as source"
    #[test]
    fn telling_plants_hearsay_with_the_teller_as_source() {
        let mut st = world();
        tell_to("sam", "hana", &mut st);
        assert!(
            st.db_has("hana.believes.tripped.tess.heard.sam"),
            "hana heard it from sam"
        );
    }

    // H: RumorSpec.hs "the subject of the rumor is never offered as hearer"
    #[test]
    fn the_subject_of_the_rumor_is_never_offered_as_hearer() {
        let mut st = world();
        assert!(
            !offers("sam", "tess", &mut st),
            "no telling tess about tess"
        );
    }

    // H: RumorSpec.hs "in a multi-variable pattern only the FIRST variable is the subject"
    #[test]
    fn only_the_first_variable_is_the_subject() {
        // `Borrower` is the subject; `Item` is just quantified.
        let lent = gossip(
            &together(),
            Vec::new(),
            "borrowed.Borrower.Item",
            "[Actor]: tell [Hearer] that [Borrower] borrowed the [Item]",
        )
        .unwrap();
        let mut st = State::new();
        st.define_practices([Practice::new("yard2").roles(["R"]).action(lent)])
            .unwrap();
        st.set_characters(
            ["sam", "hana", "tess", "pip"]
                .into_iter()
                .map(Character::new)
                .collect(),
        )
        .unwrap();
        for o in [
            insert("practice.yard2.here"),
            insert("at.sam!yard2"),
            insert("at.hana!yard2"),
            insert("at.tess!yard2"),
            insert("at.pip!yard2"),
            // tess borrowed pip(!)
            insert("sam.believes.borrowed.tess.pip.seen"),
        ] {
            st.perform_outcome(&o).expect("yard2 setup");
        }
        assert!(
            !st.possible_actions("sam")
                .iter()
                .any(|ga| ga.label.contains("tell tess")),
            "tess (first variable: the subject) is excluded as hearer"
        );
        assert!(
            st.possible_actions("sam")
                .iter()
                .any(|ga| ga.label.contains("tell pip")),
            "pip (second variable: not the subject) may be told"
        );
    }

    // H: RumorSpec.hs "a hearer who saw the event is not told (no news value)"
    #[test]
    fn a_hearer_who_saw_the_event_is_not_told() {
        let mut st = world();
        assert!(
            !offers("sam", "rita", &mut st),
            "sam not offered telling rita (an eyewitness)"
        );
    }

    // H: RumorSpec.hs "retelling to the same hearer is not offered (one-shot per teller)"
    #[test]
    fn retelling_to_the_same_hearer_is_not_offered() {
        let mut st = world();
        tell_to("sam", "hana", &mut st);
        assert!(!offers("sam", "hana", &mut st), "sam cannot retell hana");
    }

    // H: RumorSpec.hs "a second teller adds a second heard edge (corroboration)"
    #[test]
    fn a_second_teller_adds_a_second_heard_edge() {
        let mut st = world();
        tell_to("sam", "hana", &mut st);
        tell_to("rita", "hana", &mut st);
        assert!(
            st.db_has("hana.believes.tripped.tess.heard.sam"),
            "heard from sam"
        );
        assert!(
            st.db_has("hana.believes.tripped.tess.heard.rita"),
            "heard from rita"
        );
    }

    // H: RumorSpec.hs "hearsay can be retold (rumor chains)"
    #[test]
    fn hearsay_can_be_retold() {
        let mut st = world();
        tell_to("sam", "hana", &mut st);
        tell_to("hana", "pip", &mut st);
        assert!(
            st.db_has("pip.believes.tripped.tess.heard.hana"),
            "pip heard it from hana"
        );
    }

    // H: RumorSpec.hs "no evidence, nothing to tell"
    #[test]
    fn no_evidence_nothing_to_tell() {
        let mut st = world();
        assert!(
            !st.possible_actions("hana")
                .iter()
                .any(|ga| ga.label.contains("tell")),
            "hana (who knows nothing yet) offers no tells"
        );
    }

    // H: RumorSpec.hs "the world's gate closes the channel"
    #[test]
    fn the_worlds_gate_closes_the_channel() {
        let mut st = world();
        st.perform_outcome(&insert("grudge.sam.hana")).unwrap();
        assert!(
            !offers("sam", "hana", &mut st),
            "sam won't gossip with hana"
        );
        assert!(offers("rita", "hana", &mut st), "but rita still will");
    }

    // H: RumorSpec.hs "heard is a boolean exists (no per-source bindings leak)"
    #[test]
    fn heard_is_a_boolean_exists() {
        assert_eq!(
            heard("W", "tripped.tess"),
            exists(vec![matches("W.believes.tripped.tess.heard.Src")])
        );
    }

    // H: RumorSpec.hs "a pattern with no variable errors loudly"
    #[test]
    fn a_pattern_with_no_variable_errors_loudly() {
        assert!(
            matches!(
                gossip(
                    &together(),
                    Vec::new(),
                    "somethinghappened",
                    "[Actor]: mention it to [Hearer]"
                ),
                Err(WorldError::PatternVariables { .. })
            ),
            "gossip on a subject-less pattern is an error"
        );
    }

    // H: RumorSpec.hs "v43: the missing namespace guard (previously latent: an authored gate or event pattern had no guard at all)"
    // H: RumorSpec.hs "a gate authoring the Prax namespace is a loud construction-time error"
    #[test]
    fn a_gate_authoring_the_prax_namespace_is_rejected() {
        assert!(
            matches!(
                gossip(
                    &together(),
                    vec![not_("flag.PraxD")],
                    "tripped.Klutz",
                    "[Actor]: tell [Hearer] that [Klutz] tripped"
                ),
                Err(WorldError::ReservedVarClash { .. })
            ),
            "the Prax namespace in the gate is rejected"
        );
    }

    // H: RumorSpec.hs "an event pattern authoring the Prax namespace is a loud construction-time error"
    #[test]
    fn an_event_pattern_authoring_the_prax_namespace_is_rejected() {
        assert!(
            matches!(
                gossip(
                    &together(),
                    Vec::new(),
                    "tripped.PraxWho",
                    "[Actor]: tell [Hearer] that [Klutz] tripped"
                ),
                Err(WorldError::ReservedVarClash { .. })
            ),
            "the Prax namespace in the event pattern is rejected"
        );
    }

    // H: RumorSpec.hs "the shipped tell fixture (an ordinary gate and pattern) is not rejected"
    #[test]
    fn the_shipped_tell_fixture_is_not_rejected() {
        assert!(
            gossip(
                &together(),
                vec![not_("grudge.Actor.Hearer")],
                "tripped.Klutz",
                "[Actor]: tell [Hearer] that [Klutz] tripped"
            )
            .is_ok(),
            "the legal shape is not rejected"
        );
    }
}
