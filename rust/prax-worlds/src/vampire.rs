//! The vampire village: an infection social sim built as a fresh content
//! module, on the same movement/sight substrate as [`crate::village`].
//!
//! Task 1 is the empty stage: an 8-villager cast, a few connected places, and
//! the movement + sight machinery. Task 2 adds the engine's day/night clock
//! (`phase!day`/`phase!night`) and turns patient zero — mara, marked — at
//! the first night. Later tasks add feeding, further transformation, and
//! endings.

use prax_core::engine::State;
use prax_core::query::{CalcOp, Condition, calc, matches, not_};
use prax_core::schedule::sight_rule;
use prax_core::types::{Action, Character, Outcome, Practice, ScheduleRule, insert};
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

/// The day/night clock (`phase!day`/`phase!night`): a period-1 schedule rule
/// that derives the phase directly from the round-boundary clock's parity —
/// `turn!Now` mod 2 — via a static lookup table (`phaseOfParity.0!day`,
/// `phaseOfParity.1!night`; set up in [`vampire_setup`]), the same
/// static-relation idiom as `connected.X.Y`. Deriving the phase fresh from
/// the absolute turn count each boundary, rather than flipping the previous
/// value, keeps it self-correcting: nothing can desync it from the clock,
/// and — unlike a mutual pair of "if day then night / if night then day"
/// rules — a single clause reading only the immutable `turn!Now` can never
/// see its own write and re-fire within the same boundary.
fn phase_clock() -> ScheduleRule {
    ScheduleRule::new("phase", 1).clause(
        [
            matches("turn!Now"),
            calc("Parity", CalcOp::Mod, "Now", "2"),
            matches("phaseOfParity.Parity!Phase"),
        ],
        [insert("phase!Phase")],
    )
}

/// Patient zero: mara turns — and is marked, per the design that every
/// vampire (including patient zero) is marked — at the first night. Guarded
/// by `everBitten` so it fires exactly once and never re-fires. Declared
/// after [`phase_clock`] in the schedule so the SAME round boundary that
/// first flips the phase to night also turns her: both are period-1 rules
/// firing in declaration order within one `round_boundary` call.
fn turn_patient_zero() -> ScheduleRule {
    ScheduleRule::new("turnPatientZero", 1).clause(
        [
            matches("phase!night"),
            not_("everBitten"),
            not_("vampire.mara"),
        ],
        [
            insert("vampire.mara"),
            insert("mark.mara.neck"),
            insert("everBitten"),
        ],
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
        // Day one starts in daylight; the parity table [`phase_clock`] reads
        // every boundary to re-derive `phase!X` off `turn!Now`.
        insert("phase!day"),
        insert("phaseOfParity.0!day"),
        insert("phaseOfParity.1!night"),
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
    st.set_schedule(vec![
        sight_rule(sighting()),
        phase_clock(),
        turn_patient_zero(),
    ])
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

    /// A one-path existence query on the view — the sibling-world `fact`
    /// helper, over `State::view_has` rather than the whole-view
    /// `labeled_facts` snapshot Task 1's test reads.
    fn fact(st: &mut State, path: &str) -> bool {
        st.view_has(path)
    }

    /// How many `vampire.*` facts exist, via `child_keys` on the base db.
    fn count_vampires(st: &mut State) -> usize {
        st.db_child_keys("vampire").len()
    }

    /// Advance the engine's boundary clock until `phase!night` holds. The
    /// phase clock is self-correcting off `turn!Now` (see [`phase_clock`]),
    /// so this always converges in one boundary; the bound below only turns
    /// a clock regression into a loud failure instead of an infinite loop.
    fn advance_to_first_night(st: &mut State) {
        for _ in 0..8 {
            st.round_boundary();
            if fact(st, "phase!night") {
                return;
            }
        }
        panic!("the clock never reached phase!night after 8 round boundaries");
    }

    // H: task-2-brief.md "Patient zero turns on the first night"
    #[test]
    fn patient_zero_turns_after_the_first_day() {
        let mut st = vampire_world();
        // No vampire on day one.
        assert!(
            !fact(&mut st, "vampire.mara"),
            "no vampire before the first night"
        );
        // Advance the clock through day 1 into the first night.
        advance_to_first_night(&mut st);
        assert!(
            fact(&mut st, "vampire.mara"),
            "mara is patient zero after night 1"
        );
        assert_eq!(
            count_vampires(&mut st),
            1,
            "exactly one vampire at the start"
        );
    }
}
