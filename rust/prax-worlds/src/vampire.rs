//! The vampire village: an infection social sim built as a fresh content
//! module, on the same movement/sight substrate as [`crate::village`].
//!
//! Task 1 is the empty stage: an 8-villager cast, a few connected places, and
//! the movement + sight machinery. Task 2 adds the engine's day/night clock
//! (`phase!day`/`phase!night`) and turns patient zero — mara, marked — at
//! the first night. Later tasks add feeding, further transformation, and
//! endings.

use prax_core::engine::State;
use prax_core::query::{CalcOp, CmpOp, Condition, calc, cmp, matches, neq, not_};
use prax_core::schedule::sight_rule;
use prax_core::types::{
    Action, Axiom, Character, Outcome, Practice, ScheduleRule, delete, insert, insert_for,
};
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

/// Both of the feed action's timers, in the clock's own unit: one turn is one
/// phase (see [`phase_clock`]), so two turns is a full day-night cycle — the
/// design's "24h" window, for both the transformation delay and the feed
/// cooldown.
const TURN_DELAY: i64 = 2;
const FEED_COOLDOWN: i64 = 2;

/// Feeding: a vampire bites a co-located, not-yet-vampire victim. A
/// world-scoped singleton practice — the `Scene` role plays no part in the
/// action's own conditions (which read the movement substrate directly);
/// it exists only to give the practice an instance to spawn from, the same
/// idiom as `village::village_practice`'s `Scene` role. The bite leaves a
/// neck mark, timestamps itself with the current turn (`bittenOn.Prey!Now`
/// — a PERSISTENT insert, not `insert_for`: [`transformation`] still needs to
/// read it after the delay elapses) for [`transformation`]'s turn-arithmetic
/// axiom to consume, arms the actor's own feed cooldown for
/// [`FEED_COOLDOWN`], and sates the actor's hunger. The turn stamp follows
/// the same variable-binding idiom `sight_rule` uses for
/// `believes.atSince.Seen!Now` — no separate `call`/`Function` needed: an
/// action's `when` may freely READ the engine-owned `turn!Now` clock (only
/// WRITING it is reserved, per [`prax_core::typecheck`]), and the bound
/// `Now` is then substituted straight into the `then` path string.
fn prey_practice() -> Practice {
    Practice::new("prey")
        .name("Feeding")
        .roles(["Scene"])
        .action(
            Action::new("[Actor]: feed on [Prey]")
                .when([
                    matches("vampire.Actor"),
                    matches("bloodHunger.Actor"),
                    not_("fed.Actor"),
                    matches("practice.world.world.at.Actor!Spot"),
                    matches("practice.world.world.at.Prey!Spot"),
                    neq("Actor", "Prey"),
                    not_("vampire.Prey"),
                    matches("turn!Now"),
                ])
                .then([
                    insert("mark.Prey.neck"),
                    insert("bittenOn.Prey!Now"),
                    insert_for(FEED_COOLDOWN, "fed.Actor"),
                    delete("bloodHunger.Actor"),
                ]),
        )
}

/// Transformation: `TURN_DELAY` boundaries after a bite, the victim becomes
/// a vampire — a turn-arithmetic axiom over the bite's timestamp
/// ([`prey_practice`]'s `bittenOn.V!T`) and the live clock (`turn!Now`),
/// re-evaluated on every view read rather than fired once by a schedule
/// rule. Derived (a VIEW fact via [`crate::engine::State::set_axioms`]),
/// not a base-fact schedule-rule insert — deliberately, DESPITE patient
/// zero's `vampire.mara` being a base fact ([`turn_patient_zero`]): both
/// paths converge on the same base-vs-derived question the type checker
/// settles for us. A derived `vampire.V` still satisfies every existing
/// `vampire.*` read — [`prey_practice`]'s `not_("vampire.Prey")` and
/// [`turn_patient_zero`]'s own `not_("vampire.mara")` both query the VIEW,
/// which folds base and derived facts together — so newly-turned vampires
/// are indistinguishable, to every consumer, from patient zero. The
/// `not_("vampire.V")` guard is the monotone idiom already used by
/// `turn_patient_zero`: once derived it stays derivable every subsequent
/// read (`T`, and hence `E`, never shrink), so the guard never flips it back
/// off — it exists only to keep the axiom from being its own precondition.
fn transformation() -> Axiom {
    Axiom::new(
        vec![
            matches("bittenOn.V!T"),
            matches("turn!Now"),
            calc("E", CalcOp::Sub, "Now", "T"),
            cmp(CmpOp::Gte, "E", TURN_DELAY.to_string()),
            not_("vampire.V"),
        ],
        ["vampire.V"],
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
/// vampire (including patient zero) is marked — at the first night. She also
/// wakes hungry: turning IS becoming hungry, the same fact [`prey_practice`]'s
/// feed action sates and the only currently-reachable producer of
/// `bloodHunger.X` (Task 5's hunger DRIVE will elaborate how it re-arms
/// afterward; this is what keeps `bloodHunger.Actor` a live — not
/// dead-condition — read in the meantime). Guarded by `everBitten` so it
/// fires exactly once and never re-fires. Declared after [`phase_clock`] in
/// the schedule so the SAME round boundary that first flips the phase to
/// night also turns her: both are period-1 rules firing in declaration order
/// within one `round_boundary` call.
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
            insert("bloodHunger.mara"),
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
        // Spawns the `prey` practice's singleton instance (see
        // [`prey_practice`]) so its `feed` action can ever be offered.
        insert("practice.prey.here"),
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
    st.define_practices([world_practice(), prey_practice()])
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
    st.set_axioms(vec![transformation()])
        .expect("vampire village axioms");
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
    use prax_core::engine::GroundedAction;

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

    /// How many `vampire.*` facts hold, via the VIEW (`labeled_view`) rather
    /// than the base db: since [`transformation`] a turned victim's
    /// `vampire.V` is a derived fact, not a base one, so a base-db read
    /// (`db_child_keys`) silently undercounts. The same `labeled_view` idiom
    /// [`fact_prefix`] already uses, filtered to the `vampire.` prefix.
    fn count_vampires(st: &mut State) -> usize {
        st.labeled_view()
            .iter()
            .filter(|f| f.starts_with("vampire."))
            .count()
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

    /// A prefix existence query on the view: true if some fact in the closed
    /// view is `path` itself or begins `path` followed by a `.` or `!`
    /// separator. Used for `bittenOn.<victim>`, whose exact suffix Task 4
    /// refines from `.pending` to a turn stamp (`!<turn>`) — the test only
    /// pins the timestamping, not the interim suffix.
    fn fact_prefix(st: &mut State, path: &str) -> bool {
        st.labeled_view().iter().any(|f| {
            f == path || f.starts_with(&format!("{path}.")) || f.starts_with(&format!("{path}!"))
        })
    }

    /// A fresh vampire world with `vampire` already turned and co-located
    /// with `victim` at `place` — bypassing the day/night clock so feed
    /// tests don't have to advance to a real first night. Direct
    /// `perform_outcome` writes, exactly as [`vampire_setup`] itself seeds
    /// starting positions.
    fn seeded_two_at(vampire: &str, victim: &str, place: &str) -> State {
        let mut st = vampire_world();
        st.perform_outcome(&insert(format!("vampire.{vampire}")))
            .expect("mark the vampire");
        st.perform_outcome(&insert(format!(
            "practice.world.world.at.{vampire}!{place}"
        )))
        .expect("place the vampire");
        st.perform_outcome(&insert(format!("practice.world.world.at.{victim}!{place}")))
            .expect("place the victim");
        st
    }

    /// Arms `bloodHunger.<who>` directly — Task 5 wires this from the
    /// hunger DRIVE; this task only needs the fact `feed`'s `when` reads.
    fn make_hungry(st: &mut State, who: &str) {
        st.perform_outcome(&insert(format!("bloodHunger.{who}")))
            .expect("arm hunger");
    }

    /// The offered `prey.feed` action grounded on this exact victim, off the
    /// real `possible_actions` enumeration — the `bar::tests::find` idiom.
    /// Panics (with the actor's full offer list) if the pairing isn't
    /// offered, since every caller of this helper expects it to be.
    fn ground_feed(st: &mut State, vampire: &str, victim: &str) -> GroundedAction {
        let needle = format!("feed on {victim}");
        let had = labels(st, vampire);
        st.possible_actions(vampire)
            .into_iter()
            .find(|ga| ga.practice_id == "prey" && ga.label.contains(&needle))
            .unwrap_or_else(|| {
                panic!("no feed-on-{victim} action offered to {vampire}; available: {had:?}")
            })
    }

    /// Whether `vampire` is currently offered a feed on `victim` — the
    /// cooldown-blocks-refeeding check, which must find nothing rather than
    /// panic (unlike [`ground_feed`]).
    fn feed_is_available(st: &mut State, vampire: &str, victim: &str) -> bool {
        let needle = format!("feed on {victim}");
        st.possible_actions(vampire)
            .into_iter()
            .any(|ga| ga.practice_id == "prey" && ga.label.contains(&needle))
    }

    fn labels(st: &mut State, actor: &str) -> Vec<String> {
        st.possible_actions(actor)
            .into_iter()
            .map(|ga| ga.label)
            .collect()
    }

    /// Advance the engine's boundary clock `n` times — the plain stepping
    /// [`advance_to_first_night`] already does, generalized to an arbitrary
    /// count for [`transformation`]'s delay, which counts boundaries rather
    /// than phase flips.
    fn advance_boundaries(st: &mut State, n: i64) {
        for _ in 0..n {
            st.round_boundary();
        }
    }

    // H: task-3-brief.md "The feed action leaves a mark, arms the
    // transformation timer, and arms a feed cooldown"
    #[test]
    fn feeding_marks_the_victim_and_locks_the_cooldown() {
        let mut st = seeded_two_at(
            /* vampire= */ "mara", /* victim= */ "bram", /* place= */ "mill",
        );
        make_hungry(&mut st, "mara");
        let bite = ground_feed(&mut st, "mara", "bram");
        st.perform_action(&bite);
        assert!(
            fact(&mut st, "mark.bram.neck"),
            "the bite leaves a neck mark"
        );
        assert!(
            fact_prefix(&mut st, "bittenOn.bram"),
            "the bite is timestamped"
        );
        assert!(fact(&mut st, "fed.mara"), "feeding arms the cooldown");
        assert!(!fact(&mut st, "bloodHunger.mara"), "feeding sates hunger");
        // A second feed is blocked while the cooldown holds.
        make_hungry(&mut st, "mara");
        assert!(
            !feed_is_available(&mut st, "mara", "bram"),
            "cooldown blocks re-feeding"
        );
    }

    // H: task-4-brief.md "A bitten victim turns after the delay"
    #[test]
    fn a_bitten_victim_turns_after_the_delay() {
        let mut st = seeded_two_at(
            /* vampire= */ "mara", /* victim= */ "bram", /* place= */ "mill",
        );
        make_hungry(&mut st, "mara");
        let bite = ground_feed(&mut st, "mara", "bram");
        st.perform_action(&bite);
        assert!(
            !fact(&mut st, "vampire.bram"),
            "not yet turned right after the bite"
        );
        advance_boundaries(&mut st, TURN_DELAY);
        assert!(fact(&mut st, "vampire.bram"), "bram turns after the delay");
        assert!(
            fact(&mut st, "mark.bram.neck"),
            "the turned still bears the mark"
        );
    }

    /// Regression: [`count_vampires`] must count a DERIVED `vampire.V`
    /// (produced by [`transformation`]) alongside a base-fact one, not just
    /// the base db's own children. A base-db-only `count_vampires` reports 1
    /// here (only the seeded base-fact `vampire.mara`) even though
    /// `fact(&mut st, "vampire.bram")` — reading the view, like every other
    /// consumer of `vampire.*` — already asserts true.
    #[test]
    fn count_vampires_includes_derived_vampires() {
        let mut st = seeded_two_at(
            /* vampire= */ "mara", /* victim= */ "bram", /* place= */ "mill",
        );
        make_hungry(&mut st, "mara");
        let bite = ground_feed(&mut st, "mara", "bram");
        st.perform_action(&bite);
        advance_boundaries(&mut st, TURN_DELAY);
        assert!(fact(&mut st, "vampire.bram"), "bram turns after the delay");
        assert_eq!(
            count_vampires(&mut st),
            2,
            "count_vampires must count bram's derived vampire.bram alongside mara's base-fact vampire.mara"
        );
    }
}
