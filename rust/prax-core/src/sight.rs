//! The perceptual layer's scope fragment. `sightRule` (the period-1 engine rule
//! that deposits `<seer>.believes.at`/`atSince` for every co-present pair) landed
//! at S5; S6 owns `Prax.Sight`'s remainder: [`sighted_within`], the `atSince`
//! window a world `Or`s with co-presence-now in its prediction scope.
//!
//! Frozen reference: `src/Prax/Sight.hs` (`sightedWithin`).

use crate::query::{CalcOp, CmpOp, Condition, calc, cmp, matches};

/// Scope fragment over `Actor`/`Witness` (`Prax.Sight.sightedWithin`): the
/// Witness was sighted within the last `h` ticks. The stamp
/// (`Actor.believes.atSince.Witness!Since`) plus the clock (`turn!Now`) feed a
/// `Since + h ≥ Now` window. Worlds `Or` this with co-presence-now.
pub fn sighted_within(h: i64) -> Vec<Condition> {
    vec![
        matches("Actor.believes.atSince.Witness!Since"),
        matches("turn!Now"),
        calc("Expiry", CalcOp::Add, "Since", h.to_string()),
        cmp(CmpOp::Gte, "Expiry", "Now"),
    ]
}

#[cfg(test)]
mod tests {
    // H: SightSpec.hs "Prax.Sight"
    //
    // The frozen `Prax.SightSpec`, re-expressed against the Rust engine. Its
    // three construction-time guard cases (the `setSchedule` group) live beside
    // `sight_rule` in `crate::schedule`; its `typeCheck`-driven
    // well-formedness case is owed to S9 (KILLED.md) — `Prax.TypeCheck` has no
    // Rust twin yet.
    use super::*;
    use crate::engine::State;
    use crate::schedule::sight_rule;
    use crate::types::{Character, insert};

    /// The frozen fixture: two rooms; ute and vic share the hall, wes is alone in
    /// the attic. The engine's period-1 sighting rule fires at every round
    /// boundary; `State::new` seeds `turn!0`.
    fn world() -> State {
        let mut st = State::new();
        st.set_characters(vec![
            Character::new("ute"),
            Character::new("vic"),
            Character::new("wes"),
        ])
        .unwrap();
        st.set_schedule(vec![sight_rule(vec![
            matches("at.Seer!Spot"),
            matches("at.Seen!Spot"),
        ])])
        .unwrap();
        for o in [
            insert("at.ute!hall"),
            insert("at.vic!hall"),
            insert("at.wes!attic"),
        ] {
            st.perform_outcome(&o).unwrap();
        }
        st
    }

    /// One round boundary = one perception tick: the boundary advances the clock
    /// and fires the due period-1 sighting rule.
    fn tick(st: &State) -> State {
        let mut next = st.clone();
        next.round_boundary();
        next
    }

    // The sighting fixture is well-formed (owed:S9 discharged): the v42
    // dead-condition and reserved-family lints see a schedule rule whose guard is
    // fed only by authored location facts (`at.*`, in the db), so `type_check`
    // finds nothing.
    // H: SightSpec.hs "the fixture world is well-formed"
    #[test]
    fn the_fixture_world_is_well_formed() {
        let st = world();
        assert!(
            crate::typecheck::type_check(&st).is_empty(),
            "the sighting fixture is well-formed, got {:?}",
            crate::typecheck::type_check(&st)
        );
    }

    // H: SightSpec.hs "the round boundary advances the world turn"
    #[test]
    fn the_round_boundary_advances_the_world_turn() {
        let mut st = world();
        assert!(st.db_has("turn!0"), "turn 0 at setup");
        let mut one = tick(&st);
        assert!(one.db_has("turn!1"), "turn 1 after a boundary");
        let mut two = tick(&one);
        assert!(two.db_has("turn!2"), "turn 2 after two");
    }

    // H: SightSpec.hs "co-presence deposits sightings, both ways; the absent see nothing"
    #[test]
    fn co_presence_deposits_sightings_both_ways() {
        let mut st = tick(&world());
        assert!(
            st.db_has("ute.believes.at.vic!hall"),
            "ute sighted vic in the hall"
        );
        assert!(st.db_has("vic.believes.at.ute!hall"), "vic sighted ute");
        assert!(
            st.db_has("ute.believes.atSince.vic!1"),
            "stamped with the turn"
        );
        assert!(!st.db_has("ute.believes.at.wes"), "nobody sighted wes");
        assert!(!st.db_has("wes.believes.at.ute"), "wes sighted nobody");
    }

    // H: SightSpec.hs "a sighting persists after separation, and a new one overwrites it"
    #[test]
    fn a_sighting_persists_after_separation_and_a_new_one_overwrites_it() {
        let st1 = tick(&world()); // ute sees vic in the hall
        let mut moved = st1.clone();
        moved.perform_outcome(&insert("at.vic!attic")).unwrap(); // vic left
        let mut st2 = tick(&moved);
        assert!(
            st2.db_has("ute.believes.at.vic!hall"),
            "ute still believes vic is in the hall (stale)"
        );
        assert!(
            st2.db_has("ute.believes.atSince.vic!1"),
            "and the stamp did not refresh"
        );
        // ute follows and re-sights: overwrite.
        let mut followed = st2.clone();
        followed.perform_outcome(&insert("at.ute!attic")).unwrap();
        let mut st3 = tick(&followed);
        assert!(st3.db_has("ute.believes.at.vic!attic"), "belief updated");
        assert!(!st3.db_has("ute.believes.at.vic!hall"), "old belief gone");
        assert!(st3.db_has("ute.believes.atSince.vic!3"), "stamp refreshed");
    }

    // H: SightSpec.hs "sightedWithin is a window over the stamp"
    #[test]
    fn sighted_within_is_a_window_over_the_stamp() {
        // The fragment itself, four conditions long…
        assert_eq!(
            sighted_within(2),
            vec![
                Condition::Match("Actor.believes.atSince.Witness!Since".into()),
                Condition::Match("turn!Now".into()),
                Condition::Calc("Expiry".into(), CalcOp::Add, "Since".into(), "2".into()),
                Condition::Cmp(CmpOp::Gte, "Expiry".into(), "Now".into()),
            ]
        );

        // …and, driven against a live world, a WINDOW: the frozen case's own
        // walk. ute sights vic at turn 1, then they separate (so no later
        // boundary refreshes the stamp — co-presence would otherwise re-sight
        // every tick and the window would never lapse).
        let pair = [("Actor", "ute"), ("Witness", "vic")];
        let mut st1 = tick(&world());
        assert!(
            st1.conditions_hold(&sighted_within(2), &pair),
            "holds right after the sighting"
        );
        st1.perform_outcome(&insert("at.vic!attic")).unwrap();
        let mut st2 = tick(&tick(&st1)); // turn 3 = the expiry (1 + 2)
        assert!(
            st2.conditions_hold(&sighted_within(2), &pair),
            "still holds AT the expiry turn (the comparison is >=)"
        );
        let mut st3 = tick(&st2); // turn 4: lapsed
        assert!(
            !st3.conditions_hold(&sighted_within(2), &pair),
            "fails once the window has lapsed"
        );
    }
}
