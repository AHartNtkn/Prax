//! The round-robin turn loop (`Prax.Loop.advance`): the next living character,
//! with the engine's round boundary fired once per rotation wrap (v44). This is
//! the loop's PURE stepping primitive so the same core drives the interactive CLI
//! and deterministic replay.
//!
//! `npcAct`/`runNpcTicks` (the planner's commitment/reconsideration half, v35
//! intentions) are S6 — they need the planner. This module exposes [`advance`]
//! and that S6 seam cleanly: [`advance`] selects and returns the actor; S6 layers
//! deliberation on top.

use crate::engine::State;
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
