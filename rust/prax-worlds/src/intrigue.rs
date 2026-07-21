//! A dramatic vertical slice — "mini Blood & Laurels" — verifying empirically
//! that Versu-style drama is expressible on our primitives: a murder, a
//! character who can die (and leave the cast), betrayal vs. loyalty vs.
//! complicity, a light romance, and multiple distinct ENDINGS.
//!
//! Rome. `cassia` means to poison the patron `artus`; she confides the plot to
//! `marcus` (the player). If no one warns Artus, Cassia poisons him — he dies
//! and is removed from play (ending: betrayal). Marcus, once he knows, may warn
//! Artus (ending: loyalty), do the deed himself (ending: complicity), or warm to
//! Cassia along the way. Left to the autonomous cast, the plot runs its course.
//!
//! Uses: [`prax_vocab::beliefs`] (Marcus learns the plot), the core model
//! ([`prax_vocab::core_model`] — gratitude/romance, and this is the first world
//! to register `Function`s and route `Call`s), the FOL `Absent` (freeze once an
//! ending is reached), a NAMED vocabulary desire driving Cassia's planning, and
//! the cast-removal (`dead.<name>`) mechanic.
//!
//! Frozen reference: `src/Prax/Worlds/Intrigue.hs`. Construction ORDER is part
//! of the port — practices, then functions, then cast, then desires, then the
//! setup outcomes, exactly as the frozen world nests them; `worldshape intrigue`
//! compares the whole post-setup state.

use prax_core::engine::State;
use prax_core::query::{absent, eq, matches, neq, not_};
use prax_core::types::{Action, Character, Desire, Practice, Want, dead_sentence, insert};
use prax_vocab::beliefs::{believe, believes_that};
use prax_vocab::core_model::{WARMTH, adjust_score, core_fns, set_bond};
use prax_vocab::emotion::{PLEASED, feel_toward};

/// The player is Marcus, the poet.
pub const PLAYER_NAME: &str = "marcus";

/// The schemer's named motive. Naming it is what lets a confidant's belief about
/// it — and so the planner's theory of mind — get any purchase on it at all.
const KILL_ARTUS: &str = "kill-artus";

/// Everyone present may simply do nothing — so an unmotivated character (and an
/// idle player) waits rather than being forced into a dramatic act.
fn presence_practice() -> Practice {
    Practice::new("presence")
        .name("the company at [Place]")
        .roles(["Place"])
        .action(Action::new("[Actor]: bide your time").when([matches("character.Actor")]))
}

/// The conspiracy. Instance: `practice.plot.<Schemer>.<Target>`.
fn plot_practice() -> Practice {
    Practice::new("plot")
        .name("[Schemer] conspires against [Target]")
        .roles(["Schemer", "Target"])
        // Recruit/inform an ally — which also lets that ally warn the victim.
        // Confiding shares not just the fact of danger but the schemer's own
        // believed mind: the ally comes to hold a motive-belief over the named
        // vocabulary desire, sourced from the schemer herself, so the ally's
        // planner can predict the schemer's next move.
        .action(
            Action::new("[Actor]: confide the plot against [Target] to [Ally]")
                .when([
                    eq("Actor", "Schemer"),
                    matches("character.Ally"),
                    neq("Ally", "Schemer"),
                    neq("Ally", "Target"),
                    not_("practice.plot.Schemer.Target.confided.Ally"),
                ])
                .then([
                    insert("practice.plot.Schemer.Target.confided.Ally"),
                    believe("Ally", "plotAgainst.Target", "yes"),
                    insert(format!("Ally.believes.desires.Schemer.{KILL_ARTUS}.heard.Schemer")),
                    adjust_score("Ally", "Schemer", WARMTH, 5, "sharedASecret"),
                ]),
        )
        // The murder: needs a confided accomplice, no warning, victim alive, no
        // ending yet. Kills the target and removes them from the cast.
        .action(
            Action::new("[Actor]: slip poison into [Target]'s cup")
                .when([
                    eq("Actor", "Schemer"),
                    matches("practice.plot.Schemer.Target.confided.Accomplice"),
                    not_("practice.plot.Schemer.Target.foiled"),
                    not_("dead.Target"),
                    absent(vec![matches("ending.E")]),
                ])
                .then([insert("dead.Target"), insert("ending!betrayal")]),
        )
        // Loyalty: anyone who knows can warn the victim, foiling the plot.
        .action(
            Action::new("[Actor]: warn [Target] that [Schemer] means to kill them")
                .when([
                    believes_that("Actor", "plotAgainst.Target", "yes"),
                    neq("Actor", "Schemer"),
                    neq("Actor", "Target"),
                    not_("practice.plot.Schemer.Target.foiled"),
                    not_("dead.Target"),
                    absent(vec![matches("ending.E")]),
                ])
                .then([
                    insert("practice.plot.Schemer.Target.foiled"),
                    adjust_score("Target", "Actor", WARMTH, 30, "savedMyLife"),
                    feel_toward("Target", PLEASED, "Actor"),
                    insert("ending!loyalty"),
                ]),
        )
        // Complicity: the ally does the deed themselves (a dark player choice).
        .action(
            Action::new("[Actor]: poison [Target] with your own hand")
                .when([
                    believes_that("Actor", "plotAgainst.Target", "yes"),
                    neq("Actor", "Schemer"),
                    neq("Actor", "Target"),
                    not_("practice.plot.Schemer.Target.foiled"),
                    not_("dead.Target"),
                    absent(vec![matches("ending.E")]),
                ])
                .then([insert("dead.Target"), insert("ending!complicity")]),
        )
        // Romance: warm to the conspirator you now share a secret with.
        .action(
            Action::new("[Actor]: warm to [Schemer]'s charms")
                .when([
                    believes_that("Actor", "plotAgainst.Target", "yes"),
                    neq("Actor", "Schemer"),
                    neq("Actor", "Target"),
                    not_("bond.Actor.Schemer!lovers"),
                ])
                .then([
                    set_bond("Actor", "Schemer", "lovers"),
                    adjust_score("Actor", "Schemer", WARMTH, 15, "sweptUp"),
                    feel_toward("Actor", PLEASED, "Schemer"),
                ]),
        )
}

/// The fully set-up episode.
///
/// # Panics
/// If a construction guard rejects the authored content — a bug in this file,
/// not a condition a world can handle.
pub fn intrigue_world() -> State {
    let mut st = State::new();
    st.define_practices([presence_practice(), plot_practice()])
        .expect("intrigue practices");
    st.define_functions(core_fns())
        .expect("the core-model library");
    // marcus (the player) and artus (the oblivious patron) have no wants; the
    // schemer's motive is the NAMED desire below, not a plain want.
    st.set_characters(vec![
        Character::new(PLAYER_NAME),
        Character::new("artus"),
        Character::new("cassia").holds(KILL_ARTUS),
    ])
    .expect("intrigue cast");
    // The schemer wants the patron dead; her lookahead makes her confide first
    // (which enables the poisoning) then strike.
    st.set_desires(vec![Desire::new(
        KILL_ARTUS,
        Want::new(vec![matches(dead_sentence("artus"))], 100),
    )])
    .expect("intrigue desires");
    for o in [
        insert("character.marcus"),
        insert("character.artus"),
        insert("character.cassia"),
        insert("practice.presence.rome"),
        insert("practice.plot.cassia.artus"),
    ] {
        st.perform_outcome(&o).expect("intrigue setup");
    }
    st
}

#[cfg(test)]
mod tests {
    // H: IntrigueSpec.hs "Prax.Worlds.Intrigue (a dramatic slice)"
    //
    // The frozen `Prax.IntrigueSpec`, re-expressed against the Rust engine: the
    // confided belief, the three endings, the ending freeze, and the
    // theory-of-mind prediction a leaked motive turns on.
    use super::*;
    use prax_core::engine::{GroundedAction, State};
    use prax_core::turn::run_npc_ticks;

    /// Perform the first action whose label contains `needle` for `actor`.
    fn act(st: &mut State, actor: &str, needle: &str) {
        let ga = find(st, actor, needle);
        st.perform_action(&ga);
    }

    fn find(st: &mut State, actor: &str, needle: &str) -> GroundedAction {
        let acts = st.possible_actions(actor);
        acts.iter()
            .find(|ga| ga.label.contains(needle))
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "no action matching {needle:?} for {actor}; had: {:?}",
                    acts.iter().map(|g| &g.label).collect::<Vec<_>>()
                )
            })
    }

    /// The state just after Cassia has confided the plot to Marcus (turn 3), so
    /// Marcus now knows and can act on it.
    fn after_confide() -> State {
        let mut st = intrigue_world();
        run_npc_ticks(&mut st, 2, 3);
        st
    }

    // H: IntrigueSpec.hs "the schemer confides, then Marcus learns the plot"
    #[test]
    fn the_schemer_confides_then_marcus_learns_the_plot() {
        let st = after_confide();
        assert!(
            st.labeled_facts()
                .contains(&"marcus.believes.plotAgainst.artus!yes".to_owned()),
            "Marcus believes Artus is in danger, got {:?}",
            st.labeled_facts()
        );
    }

    // H: IntrigueSpec.hs "an idle player lets the plot run to the betrayal ending; the victim dies"
    #[test]
    fn an_idle_player_lets_the_plot_run_to_the_betrayal_ending() {
        let mut st = intrigue_world();
        let tr = run_npc_ticks(&mut st, 2, 8);
        let fs = st.labeled_facts();
        assert!(
            tr.contains(&"cassia: slip poison into artus's cup".to_owned()),
            "Cassia poisons Artus, got {tr:?}"
        );
        assert!(fs.contains(&"dead.artus".to_owned()), "Artus is dead");
        assert!(
            fs.contains(&"ending!betrayal".to_owned()),
            "the betrayal ending is reached, got {fs:?}"
        );
        // and once dead, Artus takes no further turn
        assert!(
            !tr.iter().skip(6).any(|l| l.contains("artus: ")),
            "no action by the dead Artus after the poisoning, got {tr:?}"
        );
    }

    // H: IntrigueSpec.hs "the inspector explains why an action is (un)available"
    //
    // Owed row 11 discharged: `Prax.Inspect.explain` over the intrigue world.
    // Before Marcus knows the plot, warning is blocked by the belief precondition
    // (the reason names `believes`); once Cassia has confided, it is AVAILABLE.
    #[test]
    fn the_inspector_explains_why_an_action_is_unavailable() {
        let before = prax_core::inspect::explain(&intrigue_world(), "marcus", "warn artus").join("");
        assert!(
            before.contains("blocked by") && before.contains("believes"),
            "blocked, reason names the belief precondition: {before:?}"
        );
        let after = prax_core::inspect::explain(&after_confide(), "marcus", "warn artus").join("");
        assert!(after.contains("AVAILABLE"), "now available: {after:?}");
    }

    // H: IntrigueSpec.hs "the inspector handles an instantiated zero-role practice"
    //
    // Owed row 12 discharged: a zero-role practice's instance fact is exactly
    // `practice.<pid>`, and the inspector's instance query must NOT append a
    // dangling separator (the v43 trailing-operator class). Built from the cooked
    // `instance_names` segment list, it cannot.
    #[test]
    fn the_inspector_handles_an_instantiated_zero_role_practice() {
        let mut w = intrigue_world();
        let shrine = Practice::new("shrine")
            .action(Action::new("[Actor]: kneel").then([insert("knelt.Actor")]));
        w.define_practices([shrine]).unwrap();
        w.perform_outcome(&insert("practice.shrine")).unwrap();
        let out = prax_core::inspect::explain(&w, "marcus", "kneel").join("");
        assert!(out.contains("AVAILABLE"), "kneel explained: {out:?}");
    }

    // H: IntrigueSpec.hs "warning the patron reaches the loyalty ending, and he lives"
    #[test]
    fn warning_the_patron_reaches_the_loyalty_ending() {
        let mut st = after_confide();
        act(&mut st, "marcus", "warn artus");
        let fs = st.labeled_facts();
        assert!(
            fs.contains(&"ending!loyalty".to_owned()),
            "the loyalty ending is reached, got {fs:?}"
        );
        assert!(!fs.contains(&"dead.artus".to_owned()), "Artus lives");
        assert!(
            fs.iter()
                .any(|s| s.contains("artus.relationship.marcus.warmth")),
            "Artus is grateful (warmth toward Marcus), got {fs:?}"
        );
    }

    // H: IntrigueSpec.hs "the player can commit the murder themselves (complicity ending)"
    #[test]
    fn the_player_can_commit_the_murder_themselves() {
        let mut st = after_confide();
        act(&mut st, "marcus", "poison artus with your own hand");
        let fs = st.labeled_facts();
        assert!(
            fs.contains(&"dead.artus".to_owned()),
            "Artus dies by Marcus's hand"
        );
        assert!(
            fs.contains(&"ending!complicity".to_owned()),
            "the complicity ending is reached, got {fs:?}"
        );
    }

    // H: IntrigueSpec.hs "the player can romance the conspirator"
    #[test]
    fn the_player_can_romance_the_conspirator() {
        let mut st = after_confide();
        act(&mut st, "marcus", "warm to cassia");
        assert!(
            st.labeled_facts()
                .contains(&"bond.marcus.cassia!lovers".to_owned()),
            "Marcus and Cassia become lovers, got {:?}",
            st.labeled_facts()
        );
    }

    // H: IntrigueSpec.hs "once an ending is reached, the drama freezes (no further plot moves)"
    #[test]
    fn once_an_ending_is_reached_the_drama_freezes() {
        // reach an ending, then Cassia (who wanted Artus dead) has nothing left
        // to do: every plot move but the romance carries `Absent [Match
        // "ending.E"]`.
        let mut st = after_confide();
        act(&mut st, "marcus", "warn artus");
        assert!(
            !st.possible_actions("cassia")
                .iter()
                .any(|ga| ga.label.contains("poison")),
            "no poisoning remains available, had {:?}",
            st.possible_actions("cassia")
                .iter()
                .map(|g| &g.label)
                .collect::<Vec<_>>()
        );
    }

    // H: IntrigueSpec.hs "the confidant can foresee the poisoning; the victim cannot"
    #[test]
    fn the_confidant_can_foresee_the_poisoning_the_victim_cannot() {
        // cassia confides in marcus (the existing plot action); this both unlocks
        // her own poisoning move and, via the confide's motive-belief insert,
        // gives marcus a believed model of cassia's mind to predict from.
        let mut st = intrigue_world();
        act(
            &mut st,
            "cassia",
            "confide the plot against artus to marcus",
        );
        let marcus = Character::new("marcus");
        let artus = Character::new("artus");
        let cassia = Character::new("cassia");
        assert_eq!(
            st.predict_move(&marcus, &cassia).map(|g| g.label),
            Some("cassia: slip poison into artus's cup".to_owned())
        );
        // artus never received the belief, so cassia's mind is unreadable to him.
        assert!(
            st.predict_move(&artus, &cassia).is_none(),
            "the victim cannot read the schemer's mind"
        );
    }

    /// DESIRE-DRIVEN PLANNING, pinned directly. The frozen spec observes the
    /// schemer's plan only through its RESULT (the betrayal ending, eight ticks
    /// later); slice 2 is the first slice where a named vocabulary desire — not
    /// a plain want — is the whole motive, so the link deserves a pin of its own.
    ///
    /// Cassia's first move is the confide, which scores nothing by itself: it is
    /// chosen because depth-2 lookahead reaches the poisoning that satisfies
    /// `kill-artus`. Strip the desire from her and the same world, same
    /// affordances, same planner produces a bide — so this pins the DESIRE as the
    /// cause, not merely the choice as a fact.
    #[test]
    fn the_named_desire_is_what_drives_the_schemer() {
        let mut st = intrigue_world();
        let cassia = st
            .characters()
            .iter()
            .find(|c| c.name == "cassia")
            .cloned()
            .expect("cassia is in the cast");
        assert_eq!(cassia.desires, vec![KILL_ARTUS.to_owned()]);
        assert_eq!(
            st.pick_action(2, &cassia).map(|g| g.label),
            Some("cassia: confide the plot against artus to marcus".to_owned()),
            "the confide scores nothing on its own; depth-2 lookahead reaches the \
             poisoning that satisfies kill-artus"
        );

        // The same world with the desire not HELD by her: the registry still
        // carries it, so this isolates holding the desire from its existence.
        let mut motiveless = intrigue_world();
        let mut cast = motiveless.characters().to_vec();
        for c in &mut cast {
            if c.name == "cassia" {
                c.desires.clear();
            }
        }
        motiveless.set_characters(cast).expect("the motiveless cast");
        let bare = Character::new("cassia");
        assert_eq!(
            motiveless.pick_action(2, &bare).map(|g| g.label),
            Some("cassia: bide your time".to_owned()),
            "without the desire she has nothing to plan toward and waits"
        );
    }

    // H: IntrigueSpec.hs "a leaked motive changes who can see the plan"
    #[test]
    fn a_leaked_motive_changes_who_can_see_the_plan() {
        let mut st = intrigue_world();
        act(
            &mut st,
            "cassia",
            "confide the plot against artus to marcus",
        );
        let artus = Character::new("artus");
        let cassia = Character::new("cassia");
        assert!(st.predict_move(&artus, &cassia).is_none());
        // plant the motive-belief directly, as if the secret had leaked to artus
        st.perform_outcome(&insert(
            "artus.believes.desires.cassia.kill-artus.heard.marcus",
        ))
        .expect("the leak");
        assert_eq!(
            st.predict_move(&artus, &cassia).map(|g| g.label),
            Some("cassia: slip poison into artus's cup".to_owned())
        );
    }
}
