//! The round-robin turn loop (`Prax.Loop.advance`): the next living character,
//! with the engine's round boundary fired once per rotation wrap (v44). This is
//! the loop's PURE stepping primitive so the same core drives the interactive CLI
//! and deterministic replay.
//!
//! `npcAct`/`runNpcTicks` (the planner's commitment/reconsideration half, v35
//! intentions) are S6 — they need the planner. This module exposes [`advance`]
//! and that S6 seam cleanly: [`advance`] selects and returns the actor; S6 layers
//! deliberation on top.

use crate::engine::{GroundedAction, State};
use crate::planner::Intention;
use crate::types::Character;

/// Advance to the next living character (fact `dead.<name>` skipped) and return
/// it, running the engine's round boundary at each rotation wrap
/// (`Prax.Loop.advance`, spec v44).
///
/// The next living index `i` is a WRAP when `i <= cursor` — equality INCLUDED, so
/// a single-survivor cast wraps every turn (strict `<` would freeze engine time).
/// The initial `cursor = -1` means no boundary fires before round 1. At a wrap:
/// run [`State::round_boundary`] ONCE (it advances the clock and fires due
/// expiries then due rules — a rule may kill a character), then RE-select the
/// actor from the post-boundary aliveness. The wrap leaves `cursor` untouched and
/// a boundary can only kill (never revive), so re-scanning from `cursor` finds the
/// same lowest-index survivor — the round starts fresh.
///
/// # Panics
/// If the whole cast is dead (no living character to hand a turn).
pub fn advance(st: &mut State) -> Character {
    // Snapshot the roster BEFORE any boundary fires (S5 review M2): the wrap
    // re-selects against this pre-boundary cast — schedule rules cannot add or
    // remove characters (setters are the only cast writers, and no rule body
    // reaches them), so the snapshot equals the post-boundary roster by the
    // engine's own invariant; the capture order just makes that explicit.
    let names: Vec<String> = st.characters().iter().map(|c| c.name.clone()).collect();
    assert!(!names.is_empty(), "Prax.Loop.advance: the cast is empty");
    let cursor = st.cursor();
    match next_living(st, &names, cursor) {
        None => panic!("Prax.Loop.advance: no living characters"),
        // Wrap (equality: single survivor). Fire the boundary, then re-select — a
        // rule may have killed the character this wrap would otherwise pick.
        Some(i) if (i as i32) <= cursor => {
            st.round_boundary();
            let cursor = st.cursor(); // untouched by the boundary; read fresh
            match next_living(st, &names, cursor) {
                Some(j) => {
                    st.set_cursor(j as i32);
                    st.characters()[j].clone()
                }
                None => panic!("Prax.Loop.advance: no living characters"),
            }
        }
        Some(i) => {
            st.set_cursor(i as i32);
            st.characters()[i].clone()
        }
    }
}

/// Have an NPC act (`Prax.Loop.npcAct`, v35): if their motive signature equals the
/// one their standing intention was based on AND that intention is still offered,
/// act it WITHOUT deliberating (commitment is the default); otherwise deliberate
/// in full ([`State::pick_action`]), store the new intention (a `None` pick is
/// stored too — doing nothing is a commitment), and act. Mutates `st` (stores the
/// intention and performs the action); returns the performed action, if any.
///
/// `still_offered` checks the standing action against the CURRENT candidates by
/// full grounded equality — a movement pick is rarely want-bearing yet must expire
/// once acted, and a stale grounding is never performed.
pub fn npc_act(st: &mut State, depth: i32, actor: &Character) -> Option<GroundedAction> {
    let name = actor.name.clone();
    let sig = st.motive_signature(actor);
    if let Some(intent) = st.intention_of(&name)
        && intent.basis == sig
        && still_offered(st, actor, &intent.act)
    {
        return act(st, intent.act);
    }
    let chosen = st.pick_action(depth, actor);
    st.set_intention(
        name,
        Intention {
            act: chosen.clone(),
            basis: sig,
        },
    );
    act(st, chosen)
}

/// The standing action must still be offered, by full grounded equality
/// (`Prax.Loop.npcAct`'s `stillOffered`); `None` (doing nothing) is always offered.
fn still_offered(st: &mut State, actor: &Character, action: &Option<GroundedAction>) -> bool {
    match action {
        None => true,
        Some(ga) => st.candidate_actions(actor).contains(ga),
    }
}

fn act(st: &mut State, chosen: Option<GroundedAction>) -> Option<GroundedAction> {
    if let Some(ga) = &chosen {
        st.perform_action(ga);
    }
    chosen
}

/// Run `steps` NPC turns from the given state, collecting the narration of each
/// performed action (idle turns — a `None` pick — produce no line)
/// (`Prax.Loop.runNpcTicks`). The engine's round boundary rides inside
/// [`advance`], so a run of `steps` turns crosses a boundary at every rotation
/// wrap.
pub fn run_npc_ticks(st: &mut State, depth: i32, steps: i32) -> Vec<String> {
    let mut labels = Vec::new();
    for _ in 0..steps {
        let actor = advance(st);
        if let Some(ga) = npc_act(st, depth, &actor) {
            labels.push(ga.label.clone());
        }
    }
    labels
}

/// The first living character index strictly after `cursor` (wrapping), or `None`
/// if the whole cast is dead. Aliveness is `dead.<name>` absent from the BASE db
/// (`Prax.Loop.advance`'s `alive`/`nextLiving`).
fn next_living(st: &mut State, names: &[String], cursor: i32) -> Option<usize> {
    let n = names.len() as i32;
    for k in 1..=names.len() as i32 {
        let i = (cursor + k).rem_euclid(n) as usize;
        if !st.db_has(&format!("dead.{}", names[i])) {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod intention_spec {
    // The four owed:S6 LoopSpec intention pins, re-expressed natively [D-completeness]:
    // H: LoopSpec.hs "a quiet character acts their standing intention — even when fresh deliberation would differ"
    // H: LoopSpec.hs "each trigger reconsiders: options, satisfaction, live drive, learned motive"
    // H: LoopSpec.hs "a NON-bearing template appearing does not reconsider (irrelevant comings and goings)"
    // H: LoopSpec.hs "a standing action that is no longer offered forces re-deliberation"
    //
    // The frozen tests isolate the "external event" via `st { intentions =
    // intentions stA }` — a graft that relies on Haskell's ONE global interner.
    // Rust's per-state interner makes such a graft a cross-lineage `Sym` compare
    // ([S-I1]); the faithful re-expression instead ESTABLISHES the standing
    // intention in-place (deliberate, store, but do NOT perform) so the external
    // event is the only world change and the interner stays monotonic — exactly
    // the invariant `npc_act` runs under in the real loop.
    use super::npc_act;
    use crate::engine::{GroundedAction, State};
    use crate::planner::Intention;
    use crate::query::{Condition, neq};
    use crate::types::{Action, Character, Desire, Practice, Want, delete, insert};

    fn m(s: &str) -> Condition {
        Condition::Match(s.into())
    }
    fn label(a: Option<GroundedAction>) -> Option<String> {
        a.map(|g| g.label)
    }
    // Deliberate and store the standing intention WITHOUT performing it — the
    // isolation the frozen graft achieves, kept within one interner lineage.
    fn establish(st: &mut State, depth: i32, actor: &Character) -> Option<String> {
        let sig = st.motive_signature(actor);
        let chosen = st.pick_action(depth, actor);
        let lbl = chosen.as_ref().map(|g| g.label.clone());
        st.set_intention(
            actor.name.clone(),
            Intention {
                act: chosen,
                basis: sig,
            },
        );
        lbl
    }

    #[test]
    fn quiet_character_holds_intention_despite_diverging_deliberation() {
        let p = Practice::new("spat")
            .roles(["R"])
            .action(
                Action::new("[Actor]: goad beth")
                    .when([neq("Actor", "beth")])
                    .then([insert("goaded.beth")]),
            )
            .action(
                Action::new("[Actor]: slap priya")
                    .when([m("grudge.Actor"), m("goaded.Actor")])
                    .then([insert("slapped.priya")]),
            )
            .action(Action::new("[Actor]: wait about"));
        let priya = Character::new("priya")
            .want(Want::new(vec![m("goaded.beth")], 5))
            .want(Want::new(vec![m("slapped.priya")], -20));
        let mut st = State::new();
        st.define_practices([p]).unwrap();
        st.set_characters(vec![priya.clone(), Character::new("beth")]).unwrap();
        st.set_desires(vec![Desire::new(
            "vengeful",
            Want::new(vec![m("grudge.Owner"), m("slapped.priya")], 8),
        )])
        .unwrap();
        st.perform_outcome(&insert("practice.spat.here")).unwrap();
        st.perform_outcome(&insert(
            "priya.believes.desires.beth.vengeful.heard.gossip",
        ))
        .unwrap();

        // Establish the standing intention (goad) while beth is harmless.
        assert_eq!(establish(&mut st, 2, &priya), Some("priya: goad beth".into()));
        // The grudge lands through an external event priya has not processed.
        st.perform_outcome(&insert("grudge.beth")).unwrap();
        // Fresh deliberation would now WAIT (goad invites the -20 slap)…
        assert_eq!(label(st.pick_action(2, &priya)), Some("priya: wait about".into()));
        // …but no signature component moved — she goads anyway.
        assert_eq!(label(npc_act(&mut st, 2, &priya)), Some("priya: goad beth".into()));
    }

    #[test]
    fn options_trigger_reconsiders() {
        let p = Practice::new("mess")
            .roles(["R"])
            .action(
                Action::new("[Actor]: eat lunch")
                    .when([m("hungry.Actor")])
                    .then([insert("meal.Actor")]),
            )
            .action(Action::new("[Actor]: idle about"));
        let beth = Character::new("beth").want(Want::new(vec![m("meal.beth")], 10));
        let mut st = State::new();
        st.define_practices([p]).unwrap();
        st.set_characters(vec![beth.clone()]).unwrap();
        st.perform_outcome(&insert("practice.mess.here")).unwrap();

        let a1 = establish(&mut st, 2, &beth);
        assert_eq!(a1, Some("beth: idle about".into()));
        st.perform_outcome(&insert("hungry.beth")).unwrap();
        let a2 = label(npc_act(&mut st, 2, &beth));
        assert_eq!(a2, Some("beth: eat lunch".into()));
        assert_ne!(a1, a2, "a new option must force reconsideration");
    }

    #[test]
    fn non_bearing_template_does_not_reconsider() {
        let p = Practice::new("lull")
            .roles(["R"])
            .action(Action::new("[Actor]: amble over").when([m("gate.open")]))
            .action(Action::new("[Actor]: idle about"));
        let beth = Character::new("beth").want(Want::new(vec![m("meal.beth")], 10));
        let mut st = State::new();
        st.define_practices([p]).unwrap();
        st.set_characters(vec![beth.clone()]).unwrap();
        st.perform_outcome(&insert("practice.lull.here")).unwrap();

        assert_eq!(establish(&mut st, 2, &beth), Some("beth: idle about".into()));
        st.perform_outcome(&insert("gate.open")).unwrap();
        // Fresh deliberation WOULD switch (amble < idle at the 0-0 tie)…
        assert_eq!(label(st.pick_action(2, &beth)), Some("beth: amble over".into()));
        // …but amble bears on nothing beth wants, so the standing intention holds.
        assert_eq!(label(npc_act(&mut st, 2, &beth)), Some("beth: idle about".into()));
    }

    #[test]
    fn vanished_grounding_forces_redeliberation() {
        let p = Practice::new("lull2")
            .roles(["R"])
            .action(Action::new("[Actor]: amble over").when([m("roomy.here")]))
            .action(Action::new("[Actor]: idle about"));
        let beth = Character::new("beth").want(Want::new(vec![m("meal.beth")], 10));
        let mut st = State::new();
        st.define_practices([p]).unwrap();
        st.set_characters(vec![beth.clone()]).unwrap();
        st.perform_outcome(&insert("practice.lull2.here")).unwrap();
        st.perform_outcome(&insert("roomy.here")).unwrap();

        // Label tie-break: amble < idle, so the standing pick is amble.
        assert_eq!(establish(&mut st, 2, &beth), Some("beth: amble over".into()));
        st.perform_outcome(&delete("roomy.here")).unwrap();
        // The standing action left the candidate set → re-deliberate to idle.
        assert_eq!(label(npc_act(&mut st, 2, &beth)), Some("beth: idle about".into()));
    }
}
