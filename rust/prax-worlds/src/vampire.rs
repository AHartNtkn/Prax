//! The vampire village: an infection social sim built as a fresh content
//! module, on the same movement/sight substrate as [`crate::village`].
//!
//! Task 1 is the empty stage: an 8-villager cast, a few connected places, and
//! the movement + sight machinery — no vampire facts, practices, or axioms
//! yet. Later tasks add feeding, transformation, and endings.

use prax_core::engine::State;
use prax_core::query::{Condition, matches};
use prax_core::schedule::sight_rule;
use prax_core::types::{Action, Character, Outcome, Practice, insert};
use prax_vocab::persona::cast;

/// The village's sighting template, over the movement vocabulary below:
/// whoever shares a place with you is someone you see. Copied from
/// `village::village_sighting` rather than shared — worlds stay independent.
fn sighting() -> Vec<Condition> {
    vec![
        matches("practice.world.world.at.Seer!Spot"),
        matches("practice.world.world.at.Seen!Spot"),
    ]
}

/// Places and movement. Copied from `village::world_practice` rather than
/// shared — worlds stay independent.
fn world_practice() -> Practice {
    Practice::new("world")
        .name("The village exists")
        .roles(["World"])
        .action(
            Action::new("[Actor]: Go to [Place]")
                .when([
                    matches("practice.world.World.at.Actor!OtherPlace"),
                    matches("practice.world.World.connected.OtherPlace.Place"),
                ])
                .then([insert("practice.world.World.at.Actor!Place")]),
        )
        .action(
            Action::new("[Actor]: Wait a moment")
                .when([matches("practice.world.World.at.Actor!Place")]),
        )
}

/// The die seed for this playthrough. No draws are made yet in this task, but
/// the engine requires a seed to be set before it will run.
const VAMPIRE_SEED: i64 = 1897;

/// The eight-villager roster. No wants, no traits: this task is the empty
/// stage only. [`cast`] still stamps `character.<who>` per member (the
/// setup fact [`transparent`](prax_vocab::persona::transparent) and this
/// task's own test read).
fn vampire_cast() -> (Vec<Character>, Vec<Outcome>) {
    cast(
        &[],
        vec![
            (Character::new("aldric"), Vec::new()),
            (Character::new("mara"), Vec::new()),
            (Character::new("bram"), Vec::new()),
            (Character::new("cole"), Vec::new()),
            (Character::new("rosa"), Vec::new()),
            (Character::new("gideon"), Vec::new()),
            (Character::new("nessa"), Vec::new()),
            (Character::new("tam"), Vec::new()),
        ],
    )
    .expect("the vampire village roster")
}

/// The vampire village's setup facts: four connected places (`square`,
/// `church`, `mill`, `home`, all reachable from the square) and the cast's
/// starting positions, two to a place.
fn vampire_setup() -> Vec<Outcome> {
    vec![
        insert("practice.world.world.connected.square.church"),
        insert("practice.world.world.connected.church.square"),
        insert("practice.world.world.connected.square.mill"),
        insert("practice.world.world.connected.mill.square"),
        insert("practice.world.world.connected.square.home"),
        insert("practice.world.world.connected.home.square"),
        insert("practice.world.world.at.aldric!square"),
        insert("practice.world.world.at.mara!square"),
        insert("practice.world.world.at.bram!church"),
        insert("practice.world.world.at.cole!church"),
        insert("practice.world.world.at.rosa!mill"),
        insert("practice.world.world.at.gideon!mill"),
        insert("practice.world.world.at.nessa!home"),
        insert("practice.world.world.at.tam!home"),
    ]
}

/// The fully initialized vampire village — the empty stage.
///
/// # Panics
/// If a construction guard rejects the authored content — a bug in this
/// file, not a condition a world can handle.
pub fn vampire_world() -> State {
    let (roster, persona_facts) = vampire_cast();
    let mut st = State::new();
    st.define_practices([world_practice()])
        .expect("vampire village practices");
    st.set_characters(roster).expect("vampire village cast");
    st.set_schedule(vec![sight_rule(sighting())])
        .expect("vampire village schedule");
    for o in vampire_setup().iter().chain(persona_facts.iter()) {
        st.perform_outcome(o).expect("vampire village setup");
    }
    st.set_axioms(Vec::new()).expect("vampire village axioms");
    st.set_desires(Vec::new()).expect("vampire village desires");
    st.seed_die(VAMPIRE_SEED)
        .expect("the vampire village's die seed");
    st.set_prediction_scope(sighting())
        .expect("vampire village prediction scope");
    st
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_world_builds_and_is_well_formed() {
        let st = vampire_world();
        assert_eq!(
            prax_core::typecheck::type_check(&st),
            vec![],
            "the vampire world must be well-formed"
        );
        // Eight named villagers exist as characters.
        for who in [
            "aldric", "mara", "bram", "cole", "rosa", "gideon", "nessa", "tam",
        ] {
            assert!(
                st.labeled_facts().contains(&format!("character.{who}")),
                "{who} should be a character in the world, got {:?}",
                st.labeled_facts()
            );
        }
    }
}
