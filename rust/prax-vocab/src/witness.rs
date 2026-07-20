//! Authored witnessing: information asymmetry from observation.
//!
//! An action's public appearance is a semantic property its author STATES with
//! [`observable`] — undeclared actions are not events (waiting is not news), and
//! the declared appearance may deliberately differ from what the action DOES
//! (poisoning the cup can look like pouring wine).
//!
//! A witnessed event is an ordinary belief ([`crate::beliefs`]):
//! `<witness>.believes.<event>.seen` — the `.seen` leaf records PROVENANCE
//! (direct observation, MULTI-VALUED). The rumor layer adds `.heard.<source>`
//! edges beside `.seen`, one per teller, so evidence accumulates instead of
//! overwriting; an exclusive slot would let hearsay destroy an eyewitness
//! record, and mixing `!` and `.` on the slot is a cardinality clash the checker
//! rejects.
//!
//! Co-presence is WORLD VOCABULARY (the engine has no notion of place): each
//! world supplies a [`CoPresence`] template once, relating the fixed variables
//! `Witness` and `Actor` in its own terms.
//!
//! Frozen reference: `src/Prax/Witness.hs`. [`CoPresence`] is a TYPE ALIAS, not
//! a newtype — worlds build them inline as plain condition lists, exactly as the
//! frozen `type CoPresence = [Condition]` lets them. See [`as_role`] for the one
//! implementation divergence in this module (DIV-4).

use prax_core::query::Condition;
use prax_core::types::{Action, Outcome, for_each, insert};

use crate::beliefs::belief_about;

/// Conditions relating the fixed variables `Witness` and `Actor` in the world's
/// own vocabulary (location facts, current scene, …). Everything that constrains
/// who can witness is the template's job; [`observable`] adds only the
/// actor-exclusion.
///
/// An ALIAS, matching the frozen `type CoPresence = [Condition]`: a world writes
/// `vec![matches("…Actor!P"), matches("…Witness!P")]` directly, with no wrapper
/// to construct or unwrap.
pub type CoPresence = Vec<Condition>;

/// The witness deposit as a first-class outcome: every co-present character
/// (except the actor) comes to believe `event` with provenance `seen`. This is
/// what [`observable`] appends; exported so generated actions (e.g.
/// `crate::project` stages) can carry observability in their own effects.
pub fn witnessed(copresence: &CoPresence, event: &str) -> Outcome {
    let mut when = copresence.clone();
    when.push(Condition::Neq("Witness".to_owned(), "Actor".to_owned()));
    for_each(
        when,
        vec![insert(format!("{}.seen", belief_about("Witness", event)))],
    )
}

/// Declare an action's public appearance: every co-present character (except the
/// actor, who already knows what they did) comes to believe `event` with
/// provenance `seen`. The event sentence may use the action's own variables.
pub fn observable(copresence: &CoPresence, event: &str, act: Action) -> Action {
    let mut out = act;
    out.then.push(witnessed(copresence, event));
    out
}

/// Condition: `who` directly witnessed `event`.
pub fn saw(who: &str, event: &str) -> Condition {
    Condition::Match(format!("{}.seen", belief_about(who, event)))
}

/// Retarget a co-presence template: substitute a different variable for
/// `Witness` (e.g. `Hearer`), so the template stays single-sourced in the world
/// while other layers quantify over their own role.
///
/// # Divergence (DIV-4) and the contract it rests on
/// Frozen `asRole` is `map (groundCondition (Map.singleton (intern "Witness")
/// (VSym (intern v))))`. Rust's [`prax_core::query::ground_condition`] needs a
/// `&mut Interner` and returns a `Result`, which would infect every downstream
/// signature (`Rumor`/`Deceit`/`Blackmail`/`Confession` all call this from pure
/// value-builders). Per S7 design [S-I6] this is implemented instead with S4's
/// [`prax_core::types::rename_vars`] — pure, infallible, operator-preserving.
///
/// The two are VALUE-IDENTICAL for every shipped case and the tests below pin
/// that equality directly against `ground_condition` over the actual shipped
/// templates. They differ IN GENERAL:
///
/// * `ground_condition` substitutes only segments the tokenizer classifies as
///   VARIABLES, while `rename_vars` substitutes any segment whose NAME matches.
///   These coincide here because the substituted key is the fixed, capitalized
///   `Witness`, which is always a variable — but they would not for a lowercase
///   key, which is why the key is not a parameter.
/// * `ground_condition` REJECTS a malformed template (a sentence ending in `.`
///   or `!`) with [`prax_core::error::WorldError::TrailingOperator`], where
///   `rename_vars` returns a malformed sentence. No shipped template is
///   malformed, and the tests below pin that the shipped ones round-trip.
///
/// **Stated contract**: `v` MUST be a variable name (capitalized). The frozen
/// function substitutes a `VSym`, so a non-variable replacement would produce a
/// template that no longer quantifies — an authoring error either way, but only
/// this implementation will carry it silently.
pub fn as_role(v: &str, copresence: &CoPresence) -> Vec<Condition> {
    let subst = std::collections::BTreeMap::from([("Witness".to_owned(), v.to_owned())]);
    prax_core::types::rename_vars(&subst, copresence)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_core::db::{Bindings, Val};
    use prax_core::engine::State;
    use prax_core::interner::{Interner, is_variable_name};
    use prax_core::query::{ground_condition, matches, subquery};
    use prax_core::types::{Character, Practice};

    // H: WitnessSpec.hs "Prax.Witness"
    //
    // The frozen `Prax.WitnessSpec`, re-expressed against the Rust engine, over
    // the spec's own three-character fixture — plus the `as_role` equality pin
    // [S-I6] requires, which the frozen spec has no counterpart for.

    /// The spec's co-presence: sharing a place.
    fn together() -> CoPresence {
        vec![matches("at.Actor!P"), matches("at.Witness!P")]
    }

    fn wave() -> Action {
        observable(
            &together(),
            "waved.Actor",
            Action::new("[Actor]: wave")
                .when([matches("at.Actor!Place")])
                .then([insert("waved")]),
        )
    }

    fn world() -> State {
        let mut st = State::new();
        st.define_practices([Practice::new("greet").roles(["R"]).action(wave())])
            .unwrap();
        st.set_characters(
            ["ann", "bea", "cal"]
                .into_iter()
                .map(Character::new)
                .collect(),
        )
        .unwrap();
        for o in [
            insert("practice.greet.stage"),
            insert("at.ann!square"),
            insert("at.bea!square"),
            insert("at.cal!mill"),
        ] {
            st.perform_outcome(&o).expect("witness setup");
        }
        st
    }

    /// ann performs her (only) available action.
    fn ann_acts(mut st: State) -> State {
        let ga = st
            .possible_actions("ann")
            .into_iter()
            .next()
            .expect("wave is offered to ann");
        st.perform_action(&ga);
        st
    }

    // H: WitnessSpec.hs "a co-present character comes to believe the event; an absent one doesn't"
    #[test]
    fn a_co_present_character_believes_the_event_an_absent_one_does_not() {
        let mut st = ann_acts(world());
        assert!(
            st.db_has("bea.believes.waved.ann.seen"),
            "bea is co-present and believes it, with provenance seen"
        );
        assert!(
            !st.db_has("cal.believes.waved.ann.seen"),
            "cal is elsewhere and holds no such belief"
        );
    }

    // H: WitnessSpec.hs "the actor is not their own witness"
    #[test]
    fn the_actor_is_not_their_own_witness() {
        let mut st = ann_acts(world());
        assert!(
            !st.db_has("ann.believes.waved.ann.seen"),
            "ann holds no belief about her own act"
        );
    }

    // H: WitnessSpec.hs "observable only appends; the action's own effects are untouched"
    #[test]
    fn observable_only_appends() {
        let w = wave();
        assert_eq!(w.name, "[Actor]: wave");
        assert_eq!(w.then[..1], [insert("waved")]);
        assert_eq!(w.then.len(), 2, "exactly one deposit appended");
    }

    // H: WitnessSpec.hs "saw is the seen-provenance belief condition"
    #[test]
    fn saw_is_the_seen_provenance_belief_condition() {
        assert_eq!(
            saw("W", "waved.ann"),
            Condition::Match("W.believes.waved.ann.seen".to_owned())
        );
    }

    // H: WitnessSpec.hs "the seen deposit is multi-valued: other evidence survives beside it"
    #[test]
    fn the_seen_deposit_is_multi_valued() {
        let mut st = world();
        st.perform_outcome(&insert("bea.believes.waved.ann.heard.cal"))
            .expect("planting hearsay");
        let mut st = ann_acts(st);
        assert!(
            st.db_has("bea.believes.waved.ann.seen"),
            "the witness deposit landed"
        );
        assert!(
            st.db_has("bea.believes.waved.ann.heard.cal"),
            "the pre-existing hearsay edge survived the deposit — an exclusive \
             slot would have destroyed it"
        );
    }

    /// The `as_role` equality pin [S-I6] requires: over the ACTUAL SHIPPED
    /// templates, the `rename_vars` implementation is value-identical to the
    /// frozen `groundCondition` one it replaces.
    ///
    /// The templates are the two shipped worlds' `together` (Bar and Village
    /// author the same two atoms) and this spec's own — reproduced here as data
    /// rather than imported, because `prax-vocab` must not depend on
    /// `prax-worlds`; `worldshape bar`'s bodies comparison is what holds the
    /// world's copy identical to the frozen one.
    #[test]
    fn as_role_agrees_with_ground_condition_on_every_shipped_template() {
        let shipped: Vec<(&str, CoPresence)> = vec![
            (
                "Bar/Village `together`",
                vec![
                    matches("practice.world.world.at.Actor!P"),
                    matches("practice.world.world.at.Witness!P"),
                ],
            ),
            ("WitnessSpec `together`", together()),
        ];
        // Every role the shipped call sites retarget to (Rumor/Deceit/
        // Confession/Blackmail use `Hearer`; Blackmail's trigger uses the
        // victim's own bound variable, always capitalized).
        for role in ["Hearer", "V", "Witness"] {
            for (name, template) in &shipped {
                let got = as_role(role, template);
                let want = ground_reference(role, template);
                assert_eq!(
                    got, want,
                    "as_role({role:?}) diverges from groundCondition on {name}"
                );
            }
        }
    }

    /// The frozen implementation, transcribed: `map (groundCondition
    /// (Map.singleton (intern "Witness") (VSym (intern v))))`. Lives in the test
    /// module because it is the ORACLE for the pin above, never the shipped path.
    fn ground_reference(v: &str, copresence: &CoPresence) -> Vec<Condition> {
        let mut i = Interner::new();
        let mut b = Bindings::new();
        let key = i.intern("Witness");
        let val = Val::Sym(i.intern(v));
        b.insert(key, val);
        copresence
            .iter()
            .map(|c| ground_condition(&mut i, &b, c).expect("a shipped template is well-formed"))
            .collect()
    }

    /// The contract the divergence rests on, asserted rather than merely
    /// documented: the substituted key is a variable name (so the classifier
    /// difference cannot bite), and the replacements the shipped call sites pass
    /// are variable names too.
    #[test]
    fn as_roles_contract_the_replacement_is_a_variable() {
        assert!(is_variable_name("Witness"), "the substituted key is a variable");
        for role in ["Hearer", "V", "Witness"] {
            assert!(
                is_variable_name(role),
                "as_role's stated contract: the replacement {role:?} must be a variable"
            );
        }
    }

    /// The general difference, DEMONSTRATED rather than asserted in prose, so
    /// DIV-4's claim is checkable: on a template carrying a Subquery binder and
    /// on one that is malformed, the two implementations are distinguishable —
    /// which is exactly why the contract above is a contract and not a comment.
    #[test]
    fn the_two_implementations_differ_off_the_shipped_shape() {
        // A Subquery binder named `Witness` moves under BOTH implementations
        // (rename_vars substitutes set/find/interior together; ground_condition
        // grounds set/find too), so this shape is NOT where they part — recorded
        // because [S-I6] names Subquery binders as the thing to check.
        let sub = vec![subquery(
            "Witness",
            vec!["C".to_owned()],
            vec![matches("at.C!P")],
        )];
        assert_eq!(as_role("Hearer", &sub), ground_reference("Hearer", &sub));

        // Where they DO part: a malformed template. `ground_condition` rejects a
        // trailing operator (`Prax.Db.tokens` raises); `rename_vars` renders a
        // malformed sentence and carries on.
        let bad: CoPresence = vec![matches("at.Witness!")];
        let mut i = Interner::new();
        let mut b = Bindings::new();
        let key = i.intern("Witness");
        let val = Val::Sym(i.intern("Hearer"));
        b.insert(key, val);
        assert!(
            ground_condition(&mut i, &b, &bad[0]).is_err(),
            "the frozen implementation rejects a trailing operator"
        );
        assert_eq!(
            as_role("Hearer", &bad),
            vec![Condition::Match("at.Hearer!".to_owned())],
            "this implementation carries it — the divergence DIV-4 records"
        );
    }
}
