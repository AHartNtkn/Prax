//! The vampire village: an infection social sim built as a fresh content
//! module, on the same movement/sight substrate as [`crate::village`].
//!
//! Task 1 is the empty stage: an 8-villager cast, a few connected places, and
//! the movement + sight machinery. Task 2 adds the engine's day/night clock
//! (`phase!day`/`phase!night`) and turns patient zero — mara, marked — at
//! the first night. Later tasks add feeding, further transformation, and
//! endings.

use prax_core::engine::State;
use prax_core::query::{CalcOp, CmpOp, Condition, absent, calc, cmp, matches, neq, not_};
use prax_core::schedule::sight_rule;
use prax_core::types::{
    Action, Axiom, Character, Desire, Outcome, Practice, ScheduleRule, Want, delete, insert,
    insert_for,
};
use prax_vocab::deceit::conceal;
use prax_vocab::deontic::obliged_close;
use prax_vocab::persona::cast;
use prax_vocab::rumor::gossip;
use prax_vocab::witness::observable;

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

/// The world's time scale in turns. A turn is ~5 minutes (docs/PRINCIPLES.md), so the real
/// scale measures a day and the 24h timers in hundreds of turns. `test()` is a COMPRESSED
/// scale for the suite — it preserves the real ratios so the dynamics are the same shape, but
/// it is NOT the real semantics (a real night is 96 turns, not 2).
#[derive(Clone, Copy)]
struct TimeScale {
    /// Daylight turns per cycle.
    day_turns: i64,
    /// Night turns per cycle.
    night_turns: i64,
    /// Incubation: turns from bite to turning (24h real).
    incubation: i64,
    /// Feed cooldown: turns a vampire must wait to feed again (24h real).
    cooldown: i64,
}

impl TimeScale {
    /// The real ~5-minute scale: 16h day / 8h night, 24h timers. 24h = 288 turns.
    fn real() -> Self {
        Self {
            day_turns: 192,
            night_turns: 96,
            incubation: 288,
            cooldown: 288,
        }
    }
    /// A compressed scale for tests — same ratios (day:night 2:1, timer:day 1.5:1), tractable
    /// turn counts. NOT the real semantics; the real values are on `real()`.
    fn test() -> Self {
        Self {
            day_turns: 4,
            night_turns: 2,
            incubation: 6,
            cooldown: 6,
        }
    }
    fn cycle(&self) -> i64 {
        self.day_turns + self.night_turns
    }
}

/// The bite's co-presence template: whoever shares the biter's spot with them
/// — the same movement substrate [`sighting`] reads, over the `witness`
/// vocabulary's own fixed variables (`Actor`, `Witness`) rather than
/// `sighting`'s (`Seer`, `Seen`). Copied rather than reused for the same
/// reason `sighting` itself is copied from `village` — worlds stay
/// independent — and because the two serve different call sites:
/// `sighting` feeds [`sight_rule`]'s ambient perception, `bite_witnessing`
/// feeds [`prey_practice`]'s per-action `observable` deposit.
fn bite_witnessing() -> Vec<Condition> {
    vec![
        matches("practice.world.world.at.Actor!Spot"),
        matches("practice.world.world.at.Witness!Spot"),
    ]
}

/// Feeding: a vampire bites a co-located, not-yet-vampire, not-yet-bitten
/// victim. A world-scoped singleton practice — the `Scene` role plays no part
/// in the action's own conditions (which read the movement substrate
/// directly); it exists only to give the practice an instance to spawn from,
/// the same idiom as `village::village_practice`'s `Scene` role. The bite
/// leaves a neck mark, timestamps itself with the current turn
/// (`bittenOn.Prey!Now` — a PERSISTENT insert, not `insert_for`:
/// [`transformation`] still needs to read it after the delay elapses) for
/// [`transformation`]'s turn-arithmetic axiom to consume, arms the actor's
/// own feed cooldown for `scale.cooldown`, and sates the actor's hunger. The
/// turn stamp follows the same variable-binding idiom `sight_rule` uses for
/// `believes.atSince.Seen!Now` — no separate `call`/`Function` needed: an
/// action's `when` may freely READ the engine-owned `turn!Now` clock (only
/// WRITING it is reserved, per [`prax_core::typecheck`]), and the bound
/// `Now` is then substituted straight into the `then` path string.
///
/// `not_("bittenOn.Prey")` excludes an already-bitten, still-incubating
/// victim from being offered as prey at all — found while validating Task 4
/// (the concealment want): once every vampire prefers to disguise before
/// feeding, several can go hungry in the same window and converge on the
/// same nearest victim; without this guard a second bite silently overwrites
/// the first's `bittenOn.Prey!Now` timestamp (a single-valued slot), resetting
/// [`transformation`]'s elapsed-time clock, and a victim ganged up on by a
/// pack of vampires never accumulates enough uninterrupted time to turn — a
/// livelock, not a slowdown (`the_infection_runs_to_an_ending` never reaches
/// its ending). The guard matches on the bare `bittenOn.Prey` path with no
/// trailing `!T`: the engine's unify descent treats reaching the end of a
/// pattern as a match once the node is structurally present, without
/// requiring that exact node be independently asserted, so this matches the
/// moment ANY `bittenOn.Prey!<turn>` value exists, regardless of the bound
/// turn.
fn prey_practice(scale: TimeScale) -> Practice {
    Practice::new("prey")
        .name("Feeding")
        .roles(["Scene"])
        .action(observable(
            &bite_witnessing(),
            "bit.Appears.Prey",
            Action::new("[Actor]: feed on [Prey]")
                .when([
                    matches("vampire.Actor"),
                    matches("bloodHunger.Actor"),
                    not_("fed.Actor"),
                    matches("practice.world.world.at.Actor!Spot"),
                    matches("practice.world.world.at.Prey!Spot"),
                    neq("Actor", "Prey"),
                    not_("vampire.Prey"),
                    not_("bittenOn.Prey"),
                    matches("appears.Actor!Appears"),
                    matches("turn!Now"),
                ])
                .then([
                    insert("mark.Prey.neck"),
                    insert("bittenOn.Prey!Now"),
                    insert_for(scale.cooldown, "fed.Actor"),
                    delete("bloodHunger.Actor"),
                ]),
        ))
}

/// The general disguise affordance — available to ANYONE, not a vampire tell
/// (Part 2 gives it an innocent use). Disguising flips the actor's single-valued
/// `appears.Actor!` slot to `someone`; dropping it restores their name. A
/// witnessed act by a disguised actor is attributed to `someone`.
fn disguise_practice() -> Practice {
    Practice::new("disguise")
        .name("Disguise")
        .roles(["Scene"])
        .action(
            Action::new("[Actor]: put on a disguise")
                .when([matches("appears.Actor!Actor")]) // not already disguised
                .then([insert("appears.Actor!someone")]),
        )
        .action(
            Action::new("[Actor]: drop the disguise")
                .when([matches("appears.Actor!someone")])
                .then([insert("appears.Actor!Actor")]),
        )
}

/// Word travels: a villager who believes they saw a bite tells a co-present
/// other (never the biter, never another eyewitness, once per hearer). The
/// hearsay then breeds suspicion through [`bite_breeds_suspicion`] just as an
/// eyewitness account does.
fn talk_practice() -> Practice {
    Practice::new("talk").name("Talk").roles(["Scene"]).action(
        gossip(
            &bite_witnessing(),
            Vec::new(),
            "bit.Subject.Victim",
            "[Actor]: spread word about [Subject]",
        )
        .expect("the bite-gossip action"),
    )
}

/// Transformation: `scale.incubation` boundaries after a bite, the victim becomes
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
fn transformation(scale: TimeScale) -> Axiom {
    Axiom::new(
        vec![
            matches("bittenOn.V!T"),
            matches("turn!Now"),
            calc("E", CalcOp::Sub, "Now", "T"),
            cmp(CmpOp::Gte, "E", scale.incubation.to_string()),
            not_("vampire.V"),
        ],
        ["vampire.V"],
    )
}

/// Suspicion, the act channel: whoever believes `Biter` bit someone — by any
/// evidence, eyewitness OR hearsay ([`talk_practice`]'s gossip deposit) —
/// believes `Biter` is a vampire — biting is how vampirism manifests, so a
/// suspected biter is a suspected vampire. The body's pattern stops at
/// `Victim`, one segment short of the evidence leaf (`.seen` or
/// `.heard.<teller>`): per the engine's unify descent, reaching the end of a
/// pattern is a match once that node is structurally present, regardless of
/// which leaf sits beneath it — the same bare-pattern idiom [`prey_practice`]'s
/// `not_("bittenOn.Prey")` guard already relies on, and the one
/// [`prax_vocab::rumor::gossip`] itself uses for its own "does the teller have
/// ANY evidence" check. Derived (a belief head, no `.seen`), so it dissolves if
/// its supporting memory ever does and it is exactly the `<W>.believes.vampire.<X>`
/// shape [`conceal`] quantifies over. A disguised bite (Task 3) binds `Biter =
/// someone`, deriving the inert `…vampire.someone`.
fn bite_breeds_suspicion() -> Axiom {
    Axiom::new(
        vec![matches("Believer.believes.bit.Biter.Victim")],
        ["Believer.believes.vampire.Biter"],
    )
}

/// Vampires win: the outbreak has begun (`everBitten`) and no living human
/// remains. Stamped as a BASE fact by a period-1 referee schedule rule, NOT an
/// axiom: the stress harness's ending detector reads the base db
/// (`prax_core::stress::ending_reached` → `db_child_keys`, mirroring frozen
/// `Prax.Stress.endingReached`'s `unify "ending.E" (db st)`), so an axiom-derived
/// `ending.*` (view-only) is invisible to every mass run — the base-vs-view split
/// that also hid a derived `vampire.V` from a base-db `count_vampires` (Task 4).
/// The rule's `when` reads the closed VIEW, so it sees `transformation`'s derived
/// `vampire.V` just as [`hunger_pulse`] does. Guarded by `everBitten` (without it
/// the `absent` alone holds on day one, before mara turns) and by
/// `not_("ending.vampires")` (fire once). Declared AFTER [`hunger_pulse`] so the
/// boundary's view is fully updated when it evaluates. `not_("dead.H")` is a
/// negative read of a family no task yet produces — always-true, not a dead
/// condition (a POSITIVE read would be).
fn vampires_win_rule() -> ScheduleRule {
    ScheduleRule::new("vampiresWin", 1).clause(
        [
            matches("everBitten"),
            not_("ending.vampires"),
            absent(vec![
                matches("character.H"),
                not_("vampire.H"),
                not_("dead.H"),
            ]),
        ],
        [insert("ending.vampires")],
    )
}

/// Village wins: the outbreak has begun (`everBitten`) and no living vampire
/// remains — no `vampire.V` (base `vampire.mara` OR `transformation`'s derived
/// `vampire.V`; the closed view folds both) for which `V` isn't dead. A base-fact
/// referee rule for the same reason as [`vampires_win_rule`]; same `everBitten`
/// and `not_("ending.village")` guards, same unproducible-`dead.V` reasoning.
fn village_wins_rule() -> ScheduleRule {
    ScheduleRule::new("villageWins", 1).clause(
        [
            matches("everBitten"),
            not_("ending.village"),
            absent(vec![matches("vampire.V"), not_("dead.V")]),
        ],
        [insert("ending.village")],
    )
}

/// The day/night clock (`phase!day`/`phase!night`): a period-1, two-arm
/// schedule rule that derives the phase from the round-boundary clock's
/// position within a real day/night cycle — `turn!Now mod scale.cycle()` —
/// rather than flipping every turn. The day arm fires while that position is
/// before dusk (`< scale.day_turns`); the night arm fires from dusk onward
/// (`>= scale.day_turns`). Deriving the phase fresh from the absolute turn
/// count each boundary, rather than flipping the previous value, keeps it
/// self-correcting: nothing can desync it from the clock, and — unlike a
/// mutual pair of "if day then night / if night then day" rules — a clause
/// reading only the immutable `turn!Now` can never see its own write and
/// re-fire within the same boundary. `phase!X` is single-valued (`!`), so
/// whichever arm matches replaces the previous phase outright.
fn phase_clock(scale: TimeScale) -> ScheduleRule {
    ScheduleRule::new("phase", 1)
        // day: position in the day/night cycle is before dusk
        .clause(
            [
                matches("turn!Now"),
                calc("InCycle", CalcOp::Mod, "Now", scale.cycle().to_string()),
                cmp(CmpOp::Lt, "InCycle", scale.day_turns.to_string()),
            ],
            [insert("phase!day")],
        )
        // night: at or past dusk
        .clause(
            [
                matches("turn!Now"),
                calc("InCycle", CalcOp::Mod, "Now", scale.cycle().to_string()),
                cmp(CmpOp::Gte, "InCycle", scale.day_turns.to_string()),
            ],
            [insert("phase!night")],
        )
}

/// Patient zero: mara turns — and is marked, per the design that every
/// vampire (including patient zero) is marked — at the first night. Guarded
/// by `everBitten` so it fires exactly once and never re-fires. Declared
/// after [`phase_clock`] in the schedule so the SAME round boundary that
/// first flips the phase to night also turns her: both are period-1 rules
/// firing in declaration order within one `round_boundary` call. Turning no
/// longer hard-codes `bloodHunger.mara` here (a Task 3 stopgap, since feeding
/// was the only reachable producer of `bloodHunger.X` before Task 5's DRIVE
/// existed): [`hunger_pulse`], declared immediately after this rule, now
/// subsumes it — she satisfies `vampire.X` the instant THIS clause inserts
/// it, in the very same boundary, so she still wakes hungry the night she
/// turns, but via the one general mechanism instead of a duplicate special
/// case.
fn turn_patient_zero() -> ScheduleRule {
    ScheduleRule::new("turnPatientZero", 1).clause(
        [
            matches("phase!night"),
            not_("everBitten"),
            not_("vampire.mara"),
            // ...but only once the village has gone home for the night: no one
            // whose home is elsewhere is still co-present with mara. At the
            // night's first instant the day's square-crowd is still gathered,
            // so this holds the turning off until they disperse — patient zero
            // turns at home, under cover of night, with only her own household
            // present (the design's unobserved turning, [`home`] the household key).
            matches("practice.world.world.at.mara!Spot"),
            absent(vec![
                matches("practice.world.world.at.Other!Spot"),
                matches("home.Other!OtherHome"),
                neq("OtherHome", "Spot"),
            ]),
        ],
        [
            insert("vampire.mara"),
            insert("mark.mara.neck"),
            insert("everBitten"),
        ],
    )
}

/// Blood-hunger, the recurring DRIVE: a period-1 schedule rule, guarded
/// exactly like [`turn_patient_zero`]'s own idiom, that arms `bloodHunger.X`
/// for any vampire who is neither already hungry nor still within the feed
/// cooldown (`fed.X`, armed by [`prey_practice`]'s `feed` action for
/// `scale.cooldown` turns). Declared right after `turn_patient_zero` so a
/// freshly turned vampire (mara, or anyone [`transformation`] later turns)
/// picks up hunger the SAME boundary they turn — no separate insert needed
/// at the turning site; this one clause is now the only producer of
/// `bloodHunger.X` in the world. `not_("bloodHunger.X")` keeps re-firing
/// idempotent rather than a heuristic guard against "too much" hunger: there
/// is no such thing as more-hungry here, only hungry-or-not.
fn hunger_pulse() -> ScheduleRule {
    ScheduleRule::new("hunger", 1).clause(
        [matches("vampire.X"), not_("fed.X"), not_("bloodHunger.X")],
        [insert("bloodHunger.X")],
    )
}

/// Blood-hunger as a WANT: hunger is a negative state a vampire is driven to
/// end, the same idiom as `village::suffers_hunger` — and mirrored at that
/// desire's exact magnitude (-22). The vampire roster carries no other wants
/// at all (see [`vampire_cast`]), so unlike bob (who must outweigh a +19
/// bread/pride stack) any negative utility here would already make `feed`
/// (which deletes `bloodHunger.Actor`) outrank the neutral "Wait a moment"
/// (utility 0); -22 is reused rather than re-derived because it is the scale
/// this codebase has already proven survives combination with competing
/// wants, not because a smaller number would fail this particular roster.
fn sate_hunger() -> Desire {
    Desire::new(
        "sate-hunger",
        Want::new(vec![matches("bloodHunger.Owner")], -22),
    )
}

/// How much a villager values NOT being believed a vampire. Reuses the village
/// `conceal` scale (bob's `stole.bob.loaf` concealment is 12): strong enough
/// that forfeiting concealment outweighs the one-turn cost of disguising
/// before a bite. Confirmed empirically — not tuned — by the depth-2
/// pick-preference test
/// (`a_hungry_vampire_prefers_to_disguise_before_feeding_in_the_open`) and the
/// full-run acceptance test
/// (`the_vampire_conceals_itself_by_disguising_before_feeding`).
const CONCEAL_WEIGHT: i32 = 12;

/// How strongly a villager is pulled home once `phase!night` holds — the
/// night arm of the day/night location rhythm ([`vampire_cast`]). Must beat
/// [`DAY_SQUARE_WEIGHT`] so night reliably overrides a still-lingering day
/// want at the phase boundary, and both must stay far below [`sate_hunger`]'s
/// -22 magnitude so a hungry vampire still abandons home (or the square) to
/// hunt, and below [`CONCEAL_WEIGHT`] so a vampire still disguises before
/// feeding in the open rather than heading straight home. Confirmed
/// empirically by the real `advance`+`npc_act` loop
/// (`villagers_are_home_at_night_and_mostly_at_the_square_by_day`).
const NIGHT_HOME_WEIGHT: i32 = 3;

/// How strongly a villager is pulled to the square once `phase!day` holds —
/// the day arm of the rhythm (market/gossip; mild, since day is not the
/// governing constraint the way night is). Weaker than [`NIGHT_HOME_WEIGHT`]
/// so the asymmetry is principled rather than accidental: the design only
/// requires night to reliably pull people home (the feeding/concealment
/// window), not day to reliably empty every home, so day's pull is the
/// smaller of the two.
const DAY_SQUARE_WEIGHT: i32 = 1;

/// The die seed for this playthrough. No draws are made yet in this task, but
/// the engine requires a seed to be set before it will run.
const VAMPIRE_SEED: i64 = 1897;

/// Every villager and the place they start and are rooted to. The SINGLE source
/// of each home: [`vampire_cast`] reads it for each villager's anchor want and
/// [`vampire_setup`] reads it for their starting position, so the two can never
/// disagree. Roster order is name order at t=0 (the engine's cursor order).
const HOMES: &[(&str, &str)] = &[
    ("aldric", "square"),
    ("mara", "square"),
    ("bram", "church"),
    ("cole", "church"),
    ("rosa", "mill"),
    ("gideon", "mill"),
    ("nessa", "home"),
    ("tam", "home"),
];

/// The villager the CLI seats a human player in (`prax play vampire`). A plain
/// human villager — deliberately NOT mara (patient zero) — so a playthrough
/// begins as an ordinary inhabitant. The skeleton has no human counterplay yet
/// (detection/elimination are later plans); this is the provisional default
/// seat, read only by the CLI `world_named` play path (`stress`/`check` discard it).
pub const PLAYER_NAME: &str = "bram";

/// The eight-villager roster, built from [`HOMES`]. Each villager holds three
/// wants:
/// - `sate-hunger` ([`sate_hunger`]) — gated by `bloodHunger.Owner`, so it is
///   live only once they are a vampire (only a vampire is ever hungry, via
///   [`hunger_pulse`]); a human's copy contributes zero until the day they turn.
/// - a **day/night location rhythm**, two phase-gated wants replacing a
///   single always-on home anchor: `NIGHT_HOME_WEIGHT` to be at their
///   [`HOMES`] location while `phase!night` holds, and `DAY_SQUARE_WEIGHT` to
///   be at the square while `phase!day` holds — the spec's "ordinary
///   life-wants... the social substrate, inherited from the village world,"
///   now shaped by the clock instead of constant. Without SOME location want
///   the whole cast has identical empty valuations and drifts in lockstep, so
///   a hungry vampire is never co-located with prey at its own turn and the
///   infection cannot propagate; gating by phase is what makes night the
///   feeding/unobserved-turning window and day the social one, rather than
///   villagers sitting home around the clock. Both weights stay far below
///   hunger's `-22`, so a turned vampire abandons the rhythm to hunt.
/// - a **concealment** want, `conceal("vampire.<self>", CONCEAL_WEIGHT)`: nobody
///   should believe them a vampire. Dormant for a human — nobody yet believes
///   `vampire.<them>`, so the want is already satisfied and contributes nothing
///   — and load-bearing once they turn: it is what makes a hungry vampire
///   disguise before biting in the open, rather than biting undisguised and
///   forfeiting concealment (Task 6).
///
/// DYNAMIC vampirism (patient zero at first night, anyone [`transformation`]
/// later turns) means no fixed subset can be "the vampires" at authoring time,
/// so every member holds `sate-hunger` and the concealment want, not just a
/// chosen few. [`cast`] stamps `character.<who>` per member (the setup fact
/// this world's tests read).
fn vampire_cast() -> (Vec<Character>, Vec<Outcome>) {
    let roster = HOMES
        .iter()
        .map(|(who, home)| {
            (
                Character::new(*who)
                    .holds("sate-hunger")
                    // night → home: be at one's own home while it is night (strong — this is
                    // what puts the household together at night, makes the turning unobserved,
                    // and makes night the feeding window).
                    .want(Want::new(
                        vec![
                            matches("phase!night"),
                            matches(format!("practice.world.world.at.{who}!{home}")),
                        ],
                        NIGHT_HOME_WEIGHT,
                    ))
                    // day → square: be in the square while it is day (mild — market/gossip), so
                    // the day/night distinction actually moves people off their homes.
                    .want(Want::new(
                        vec![
                            matches("phase!day"),
                            matches(format!("practice.world.world.at.{who}!square")),
                        ],
                        DAY_SQUARE_WEIGHT,
                    ))
                    .want(
                        conceal(&format!("vampire.{who}"), CONCEAL_WEIGHT)
                            .expect("a villager's concealment want"),
                    ),
                Vec::new(),
            )
        })
        .collect();
    cast(&[], roster).expect("the vampire village roster")
}

/// The vampire village's setup facts: four connected places (`square`,
/// `church`, `mill`, `home`, all reachable from the square) and the cast's
/// starting positions, two to a place — the latter built from [`HOMES`], the
/// same single source [`vampire_cast`] reads for each villager's anchor want.
fn vampire_setup() -> Vec<Outcome> {
    vec![
        insert("practice.world.world.connected.square.church"),
        insert("practice.world.world.connected.church.square"),
        insert("practice.world.world.connected.square.mill"),
        insert("practice.world.world.connected.mill.square"),
        insert("practice.world.world.connected.square.home"),
        insert("practice.world.world.connected.home.square"),
        // Day one starts in daylight; [`phase_clock`] reads `turn!Now mod
        // scale.cycle()` every boundary to re-derive `phase!X` from scratch,
        // so this seed only covers the window before the first boundary.
        insert("phase!day"),
        // Spawns the `prey` practice's singleton instance (see
        // [`prey_practice`]) so its `feed` action can ever be offered.
        insert("practice.prey.here"),
        // Spawns the `disguise` practice's singleton instance (see
        // [`disguise_practice`]) so its two actions can ever be offered.
        insert("practice.disguise.here"),
        // Spawns the `talk` practice's singleton instance (see
        // [`talk_practice`]) so its gossip action can ever be offered.
        insert("practice.talk.here"),
    ]
    .into_iter()
    .chain(
        HOMES
            .iter()
            .map(|(who, home)| insert(format!("practice.world.world.at.{who}!{home}"))),
    )
    .chain(
        // Each villager's home — the household key `turn_patient_zero` reads to
        // fire the outbreak only once everyone has gone home for the night
        // (someone at mara's place whose `home` is elsewhere is a lingering
        // day-crowd member, and holds the turning off).
        HOMES
            .iter()
            .map(|(who, home)| insert(format!("home.{who}!{home}"))),
    )
    .chain(
        // Everyone appears as themselves until they disguise (a single-valued
        // slot the disguise practice flips to `someone`).
        HOMES
            .iter()
            .map(|(who, _)| insert(format!("appears.{who}!{who}"))),
    )
    .collect()
}

/// The fully initialized vampire village, on the real ~5-minute time scale
/// ([`TimeScale::real`]) — the registration entry point (`worlds::build`,
/// the CLI `world_named`) and every mass run.
///
/// # Panics
/// If a construction guard rejects the authored content — a bug in this
/// file, not a condition a world can handle.
pub fn vampire_world() -> State {
    vampire_world_scaled(TimeScale::real())
}

/// The fully initialized vampire village — the empty stage — built at the
/// given [`TimeScale`]. [`vampire_world`] is this at [`TimeScale::real`];
/// the test suite instead builds at [`TimeScale::test`] (the `#[cfg(test)]`
/// `world` helper), so the compressed and real worlds share every line of
/// construction, differing only in the timer magnitudes `scale` carries.
///
/// # Panics
/// If a construction guard rejects the authored content — a bug in this
/// file, not a condition a world can handle.
fn vampire_world_scaled(scale: TimeScale) -> State {
    let (roster, persona_facts) = vampire_cast();
    let mut st = State::new();
    st.define_practices([
        world_practice(),
        prey_practice(scale),
        disguise_practice(),
        talk_practice(),
    ])
    .expect("vampire village practices");
    st.set_characters(roster).expect("vampire village cast");
    st.set_schedule(vec![
        sight_rule(sighting()),
        phase_clock(scale),
        turn_patient_zero(),
        hunger_pulse(),
        vampires_win_rule(),
        village_wins_rule(),
    ])
    .expect("vampire village schedule");
    for o in vampire_setup().iter().chain(persona_facts.iter()) {
        st.perform_outcome(o).expect("vampire village setup");
    }
    // Wrapped in `obliged_close`, not a bare `vec![...]`: `prey_practice`'s
    // witness deposit (`witnessed`, via [`observable`]) writes to a
    // VARIABLE-headed path (`Witness.believes....seen`), so the checker's
    // conservative `deontic_invokable` census counts this world as able to
    // invoke an obligation — even though nothing here ever actually grounds
    // `Witness` to `obliged`. That makes `bite_breeds_suspicion` (a purely
    // `Match`-bodied rule) subject to v51 deontic closure: unclosed, it fails
    // `type_check` with `DeonticUnclosed`. `obliged_close` adds its inert
    // `□`-lifted twin (`transformation`, carrying a `calc`/`cmp` body, is
    // left alone — `obliged_lift` only lifts all-`Match` rules), the same
    // idiom [`village::village_world`] already uses.
    st.set_axioms(obliged_close(&[
        transformation(scale),
        bite_breeds_suspicion(),
    ]))
    .expect("vampire village axioms");
    st.set_desires(vec![sate_hunger()])
        .expect("vampire village desires");
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

    /// The suite's world builder: [`vampire_world_scaled`] at
    /// [`TimeScale::test`], the compressed scale ([`TimeScale::test`]'s own
    /// doc explains why) — every test in this module builds through here (or
    /// [`seeded_two_at`], which itself calls this), never through
    /// [`vampire_world`], which is reserved for the real-scale registration
    /// path.
    fn world() -> State {
        vampire_world_scaled(TimeScale::test())
    }

    #[test]
    fn the_world_builds_and_is_well_formed() {
        let st = world();
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

    // H: task-2-brief.md "phase spans a real day, not a flip every turn"
    #[test]
    fn phase_spans_the_day_then_the_night() {
        let scale = TimeScale::test(); // day 4 / night 2, cycle 6
        let mut st = vampire_world_scaled(scale);
        // turns 0..day_turns are day; day_turns..cycle are night; then it repeats.
        for _ in 0..(scale.cycle() * 2) {
            st.round_boundary();
            let turn = st.current_turn();
            let in_cycle = turn.rem_euclid(scale.cycle());
            if in_cycle < scale.day_turns {
                assert!(
                    fact(&mut st, "phase!day"),
                    "turn {turn} (in-cycle {in_cycle}) should be day, phase!day absent"
                );
                assert!(
                    !fact(&mut st, "phase!night"),
                    "turn {turn} (in-cycle {in_cycle}) should be day, but phase!night holds"
                );
            } else {
                assert!(
                    fact(&mut st, "phase!night"),
                    "turn {turn} (in-cycle {in_cycle}) should be night, phase!night absent"
                );
                assert!(
                    !fact(&mut st, "phase!day"),
                    "turn {turn} (in-cycle {in_cycle}) should be night, but phase!day holds"
                );
            }
        }
    }

    /// How many of [`HOMES`]'s villagers are currently at `place`.
    fn count_at(st: &mut State, place: &str) -> usize {
        HOMES
            .iter()
            .filter(|(who, _)| fact(st, &format!("practice.world.world.at.{who}!{place}")))
            .count()
    }

    /// The place-suffix of `who`'s current `practice.world.world.at.<who>!<place>`
    /// fact, if any — for failure-message diagnostics only (a plain named
    /// helper rather than an inline closure, so the `rustfmt`-mandated line
    /// breaks stay readable at the call sites below).
    fn position_of(st: &mut State, who: &str) -> Option<String> {
        let prefix = format!("practice.world.world.at.{who}!");
        st.labeled_view()
            .iter()
            .find(|f| f.starts_with(&prefix))
            .cloned()
    }

    /// Every villager's current position, for a failing assertion's message.
    fn all_positions(st: &mut State) -> Vec<(&'static str, Option<String>)> {
        HOMES
            .iter()
            .map(|(who, _)| (*who, position_of(st, who)))
            .collect()
    }

    /// The household living at `who`'s own [`HOMES`] address — `who` included
    /// — read straight off the single source [`HOMES`] rather than
    /// duplicated. For mara this resolves to `["aldric", "mara"]` (both home
    /// to the square); for everyone else it's the pair sharing their own
    /// home. Used by [`mara_turns_unobserved_by_non_household`] to tell her
    /// actual housemate apart from every other villager.
    fn household_of(who: &str) -> Vec<&'static str> {
        let home = HOMES
            .iter()
            .find(|entry| entry.0 == who)
            .map(|entry| entry.1)
            .unwrap_or_else(|| panic!("no such villager in HOMES: {who}"));
        HOMES
            .iter()
            .filter(|entry| entry.1 == home)
            .map(|entry| entry.0)
            .collect()
    }

    /// The bare place-suffix of `who`'s current position (`square`, `church`,
    /// `mill`, or `home` — no `who!` prefix), off [`position_of`]'s full fact
    /// path. Needed for direct place-to-place COMPARISON between two
    /// villagers: [`position_of`]'s own string differs by the `who` segment
    /// even when the two are co-located, so comparing its raw output can
    /// never detect co-presence.
    fn place_of(st: &mut State, who: &str) -> Option<String> {
        position_of(st, who).and_then(|p| p.rsplit('!').next().map(str::to_owned))
    }

    // H: task-3-brief.md "villagers home at night, in the square by day" — the
    // day/night location rhythm, driven by the REAL game loop (`advance` +
    // `npc_act`, the same primitives `the_infection_runs_to_an_ending` uses),
    // not a hand-advanced clock: the rhythm is a WANT, so it only shows up
    // through actual deliberation and action, never through `round_boundary`
    // alone.
    //
    // A "round" here is one full rotation through the 8-strong cast — exactly
    // what `advance` wraps a `round_boundary` on ([`prax_core::turn::advance`]),
    // so the phase is fixed for the whole round and every villager gets
    // exactly one action under it. Night is 2 rounds wide at `TimeScale::test`
    // ([`TimeScale::test`]), so — since every villager starts EITHER already
    // home ([`vampire_setup`] seeds them at their [`HOMES`] place) or at most
    // one hop from home (every non-square home connects directly to the
    // square, per [`vampire_setup`]'s `connected.*` facts, and the square
    // itself is home to aldric and mara) — one round's worth of individual
    // actions is enough for the WHOLE cast to reach home: no one needs the
    // second night round to arrive. There is no compression artifact from the
    // short night to work around here.
    //
    // The strong night claim is scoped to HUMANS, not literally "everyone" —
    // observed by running this exact loop with per-round position logging
    // (task-3-report.md carries the transcript): a human's only active life-
    // wants are the two location wants themselves (`conceal` contributes zero
    // utility while nobody suspects them, which holds throughout — this cast
    // never gets far enough for suspicion before the ending), so for a human
    // the night-home want is UNCONTESTED and the strong claim is exact, not
    // approximate. A vampire additionally carries `sate-hunger` at -22
    // ([`sate_hunger`]), which by design (see [`NIGHT_HOME_WEIGHT`]'s doc)
    // outweighs the +3 home want whenever a feed is actually reachable — and
    // reachable prey is a direct CONSEQUENCE of the rhythm itself: pulling
    // everyone home at night is what makes a hungry vampire's own household,
    // or a villager still finishing their walk home, the available target.
    // Observed directly in this test: mara — already home (home == square) —
    // bites and marks aldric during the first night (rounds 4-5); by round 11
    // aldric has turned (incubation, 6 test-scale turns) and mara's own feed
    // cooldown (also 6 turns) has just cleared, so she leaves her home for
    // church to chase newly-available prey (bram, cole) instead of idling
    // home hungry — night is the feeding window exactly because it is also
    // the home-gathering window. Excluding vampires from the strong claim
    // is not a weakening for pass-at-any-cost: it is the same -22-dominance
    // fact [`NIGHT_HOME_WEIGHT`]'s own doc states, checked here directly
    // against the real loop rather than asserted and left unverified.
    #[test]
    fn villagers_are_home_at_night_and_mostly_at_the_square_by_day() {
        use prax_core::turn::{advance, npc_act};
        let scale = TimeScale::test(); // day 4 / night 2, cycle 6
        let depth = 2;
        let mut st = world();
        let cast_size = HOMES.len();
        // Two full cycles: enough to see day, the night-round transition, and
        // the following day again, confirming the rhythm repeats rather than
        // being a one-shot artifact of the starting positions — and, as it
        // happens, enough to observe a vampire's hunger break the rhythm too
        // (see the doc comment above), which one cycle alone would not reach.
        let rounds = (scale.cycle() * 2) as usize;
        for round in 0..rounds {
            for _ in 0..cast_size {
                let actor = advance(&mut st);
                npc_act(&mut st, depth, &actor);
            }
            if fact(&mut st, "phase!night") {
                for (who, home) in HOMES {
                    if fact(&mut st, &format!("vampire.{who}")) {
                        continue; // hunger may legitimately pull a vampire off the rhythm
                    }
                    let home_fact = format!("practice.world.world.at.{who}!{home}");
                    let ok = fact(&mut st, &home_fact);
                    if !ok {
                        let positions = all_positions(&mut st);
                        panic!(
                            "round {round}: human villager {who} should be home ({home}) at \
                             night; positions: {positions:?}"
                        );
                    }
                }
            } else {
                assert!(
                    fact(&mut st, "phase!day"),
                    "round {round}: phase must be day or night"
                );
                let at_square = count_at(&mut st, "square");
                assert!(
                    at_square * 2 >= cast_size,
                    "round {round}: expected most of the cast at the square by day, got \
                     {at_square}/{cast_size}"
                );
            }
        }
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
        let mut st = world();
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
        let mut st = world();
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

    /// The offered action whose label contains `needle`, grounded off the real
    /// `possible_actions` enumeration. Panics (with the offer list) if absent.
    fn find_action(st: &mut State, actor: &str, needle: &str) -> GroundedAction {
        let had = labels(st, actor);
        st.possible_actions(actor)
            .into_iter()
            .find(|g| g.label.contains(needle))
            .unwrap_or_else(|| {
                panic!("no action containing {needle:?} offered to {actor}; available: {had:?}")
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

    // H: task-3 feed guards — a vampire cannot feed on itself or on another vampire
    #[test]
    fn feed_will_not_target_self_or_another_vampire() {
        let mut st = seeded_two_at("mara", "bram", "mill");
        make_hungry(&mut st, "mara");
        // neq(Actor, Prey): never offered to feed on itself.
        assert!(
            !feed_is_available(&mut st, "mara", "mara"),
            "a vampire cannot feed on itself"
        );
        // not_(vampire.Prey): another vampire is not prey.
        st.perform_outcome(&insert("vampire.bram"))
            .expect("make bram a vampire");
        assert!(
            !feed_is_available(&mut st, "mara", "bram"),
            "a vampire cannot feed on another vampire"
        );
    }

    // H: bite-defect "an already-bitten, still-incubating victim cannot be
    // re-bitten — a second bite must not be offered, since it would overwrite
    // bittenOn.Prey!Now and reset the transformation clock, letting a pack of
    // vampires keep a victim incubating forever"
    #[test]
    fn a_bitten_incubating_victim_cannot_be_rebitten() {
        let mut st = seeded_two_at("mara", "bram", "mill");
        st.perform_outcome(&insert("practice.world.world.at.cole!mill"))
            .expect("place cole at the mill");
        st.perform_outcome(&insert("vampire.cole"))
            .expect("make cole a second vampire");
        make_hungry(&mut st, "mara");
        let bite = ground_feed(&mut st, "mara", "bram");
        st.perform_action(&bite);
        // bram is now incubating (bitten, not yet turned). A second, distinct
        // vampire (on no cooldown of their own) must not be offered to bite
        // bram again while the first bite's transformation clock is running.
        make_hungry(&mut st, "cole");
        assert!(
            !feed_is_available(&mut st, "cole", "bram"),
            "an already-bitten, incubating victim cannot be re-bitten"
        );
    }

    // H: task-3 feed cooldown — clears after TimeScale::test().cooldown so the vampire can feed again
    #[test]
    fn the_feed_cooldown_clears_after_its_delay() {
        let mut st = seeded_two_at("mara", "bram", "mill");
        // a third villager co-located and never bitten, so they stay human prey
        st.perform_outcome(&insert("practice.world.world.at.cole!mill"))
            .expect("place cole at the mill");
        make_hungry(&mut st, "mara");
        let bite = ground_feed(&mut st, "mara", "bram");
        st.perform_action(&bite);
        assert!(fact(&mut st, "fed.mara"), "feeding arms the cooldown");
        // While the cooldown holds, a freshly-hungry mara still cannot feed.
        make_hungry(&mut st, "mara");
        assert!(
            !feed_is_available(&mut st, "mara", "cole"),
            "the cooldown blocks feeding again immediately"
        );
        advance_boundaries(&mut st, TimeScale::test().cooldown);
        assert!(
            !fact(&mut st, "fed.mara"),
            "the cooldown clears after TimeScale::test().cooldown boundaries"
        );
        // hunger_pulse re-arms mara each boundary; feed is offered again on a still-human prey.
        assert!(
            feed_is_available(&mut st, "mara", "cole"),
            "once the cooldown clears the vampire can feed again"
        );
    }

    // H: detection spec "a witnessed bite makes co-present characters believe a bite occurred"
    #[test]
    fn a_bite_is_witnessed_by_the_victim_and_bystanders() {
        // mara + bram (victim) + cole (bystander) all at the mill
        let mut st = seeded_two_at("mara", "bram", "mill");
        st.perform_outcome(&insert("practice.world.world.at.cole!mill"))
            .expect("place cole at the mill");
        make_hungry(&mut st, "mara");
        let bite = ground_feed(&mut st, "mara", "bram");
        st.perform_action(&bite);
        // the victim believes mara bit them
        assert!(
            fact(&mut st, "bram.believes.bit.mara.bram.seen"),
            "the victim witnesses who bit them"
        );
        // a co-present bystander believes it too
        assert!(
            fact(&mut st, "cole.believes.bit.mara.bram.seen"),
            "a bystander witnesses the bite"
        );
        // the biter is not their own witness
        assert!(
            !fact(&mut st, "mara.believes.bit.mara.bram.seen"),
            "the actor is not their own witness"
        );
    }

    // H: detection spec "believing X bit Y ⟹ believing X is a vampire"
    #[test]
    fn a_witnessed_bite_makes_the_witness_suspect_the_biter() {
        let mut st = seeded_two_at("mara", "bram", "mill");
        make_hungry(&mut st, "mara");
        let bite = ground_feed(&mut st, "mara", "bram");
        st.perform_action(&bite);
        assert!(
            fact(&mut st, "bram.believes.vampire.mara"),
            "the bitten victim suspects mara is a vampire"
        );
    }

    // H: detection spec "disguise is a toggle — dropping it restores the true identity"
    #[test]
    fn dropping_a_disguise_restores_the_true_identity() {
        let mut st = seeded_two_at("mara", "bram", "mill");
        let disg = find_action(&mut st, "mara", "put on a disguise");
        st.perform_action(&disg);
        assert!(fact(&mut st, "appears.mara!someone"), "mara is disguised");
        let drop = find_action(&mut st, "mara", "drop the disguise");
        st.perform_action(&drop);
        assert!(
            fact(&mut st, "appears.mara!mara"),
            "dropping the disguise restores her name"
        );
        assert!(
            !fact(&mut st, "appears.mara!someone"),
            "she is no longer disguised"
        );
    }

    // H: detection spec "a disguised bite records 'someone', not the biter's name"
    #[test]
    fn a_disguised_bite_masks_the_biter() {
        let mut st = seeded_two_at("mara", "bram", "mill");
        make_hungry(&mut st, "mara");
        // mara disguises: her apparent identity becomes "someone"
        let disg = find_action(&mut st, "mara", "put on a disguise");
        st.perform_action(&disg);
        assert!(
            fact(&mut st, "appears.mara!someone"),
            "disguise masks the apparent identity"
        );
        // now she feeds while disguised
        let bite = ground_feed(&mut st, "mara", "bram");
        st.perform_action(&bite);
        // the victim believes SOMEONE bit them, not mara
        assert!(
            fact(&mut st, "bram.believes.bit.someone.bram.seen"),
            "the bite is attributed to 'someone'"
        );
        assert!(
            !fact(&mut st, "bram.believes.bit.mara.bram.seen"),
            "the biter's name is not recorded"
        );
        // and so no vampire-suspicion attaches to mara
        assert!(
            !fact(&mut st, "bram.believes.vampire.mara"),
            "a masked bite breeds no suspicion of mara"
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
        advance_boundaries(&mut st, TimeScale::test().incubation);
        assert!(fact(&mut st, "vampire.bram"), "bram turns after the delay");
        assert!(
            fact(&mut st, "mark.bram.neck"),
            "the turned still bears the mark"
        );
    }

    /// The named character's own definition, straight off a fresh world's
    /// roster — the `villager` idiom `conformance::village`'s planner tests
    /// use: [`State::pick_action`] takes `&Character`, and
    /// `minds::cooked_self_wants` reads the PASSED Character's `.desires`
    /// field (not anything looked up on `st`) to decide which held desires
    /// apply, so the test must hand back exactly the roster's own Character —
    /// including its `.holds("sate-hunger")` marker — rather than a bare
    /// `Character::new(name)`.
    fn character(name: &str) -> Character {
        world()
            .characters()
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("no such character: {name}"))
            .clone()
    }

    // H: task-5-brief.md "A hungry vampire with prey at hand chooses to feed"
    #[test]
    fn a_hungry_vampire_with_prey_chooses_to_feed() {
        let mut st = seeded_two_at("mara", "bram", "mill");
        make_hungry(&mut st, "mara");
        // Task 4's concealment want makes a hungry vampire disguise before
        // biting in the open — see
        // `a_hungry_vampire_prefers_to_disguise_before_feeding_in_the_open`,
        // which pins that first-step choice directly. Drive that one step
        // here too, then assert the feed follows once disguised: disguise-
        // then-feed is the correct post-Task-4 behaviour, not a regression
        // of this test's original "a hungry vampire feeds" claim.
        let first = st.pick_action(2, &character("mara"));
        let first_label = first.map(|g| g.label).unwrap_or_default();
        assert!(
            first_label.contains("put on a disguise"),
            "a hungry vampire disguises before feeding in the open; got {first_label:?}"
        );
        let disg = find_action(&mut st, "mara", "put on a disguise");
        st.perform_action(&disg);
        let choice = st.pick_action(2, &character("mara"));
        let label = choice.map(|g| g.label).unwrap_or_default();
        assert!(
            label.contains("feed on bram"),
            "once disguised, a hungry vampire feeds; got {label:?}"
        );
    }

    // H: detection spec "the vampire conceals being believed a vampire, so it disguises first"
    #[test]
    fn a_hungry_vampire_prefers_to_disguise_before_feeding_in_the_open() {
        let mut st = seeded_two_at("mara", "bram", "mill");
        make_hungry(&mut st, "mara");
        let choice = st.pick_action(2, &character("mara"));
        let label = choice.map(|g| g.label).unwrap_or_default();
        assert!(
            label.contains("put on a disguise"),
            "a hungry vampire with an unconcealed identity disguises before biting in the open; got {label:?}"
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
        advance_boundaries(&mut st, TimeScale::test().incubation);
        assert!(fact(&mut st, "vampire.bram"), "bram turns after the delay");
        assert_eq!(
            count_vampires(&mut st),
            2,
            "count_vampires must count bram's derived vampire.bram alongside mara's base-fact vampire.mara"
        );
    }

    /// Insert a base fact directly and reclose — the same `perform_outcome`
    /// seeding idiom [`seeded_two_at`] already uses. `perform_outcome` inserts
    /// the base fact through the engine; it does not itself guarantee a full
    /// view reclose (`perform_effect`'s Insert branch has a skip path and an
    /// incremental `apply_grow` path alongside the full-reclose one), so
    /// callers that need the view to reflect the inserted fact call
    /// [`reclose`] explicitly afterward, as every ending test below does.
    fn force_fact(st: &mut State, path: &str) {
        st.perform_outcome(&insert(path))
            .expect("force a base fact");
    }

    /// Force a full view reclose off the current base, via
    /// [`State::with_db`]'s identity transform — [`crate::engine::reclose`]'s
    /// own from-scratch fixpoint (not `perform_outcome`'s incremental
    /// monotone-fast-path branch), so an ending test's assertions rest on the
    /// same closure the harness itself would reach, not an artifact of
    /// whichever internal update path a single `force_fact` happened to take.
    fn reclose(st: &mut State) {
        st.with_db(|_, db| db.clone());
    }

    // H: detection spec "suspicion spreads by gossip, not omniscience"
    #[test]
    fn a_witnessed_bite_spreads_by_gossip_to_an_absent_villager() {
        // cole witnesses the bite at the mill; later, cole and rosa are together
        // and cole gossips it to rosa, who was absent. rosa's HOME is the mill
        // itself ([`HOMES`]), so she must be sent elsewhere before the bite —
        // otherwise she is co-present with mara and bram from world creation
        // onward and becomes a direct eyewitness (`bite_witnessing`), never an
        // absent villager for [`talk_practice`]'s gossip to reach.
        let mut st = seeded_two_at("mara", "bram", "mill");
        st.perform_outcome(&insert("practice.world.world.at.cole!mill"))
            .expect("cole at mill");
        st.perform_outcome(&insert("practice.world.world.at.rosa!square"))
            .expect("rosa away from the mill");
        make_hungry(&mut st, "mara");
        let bite = ground_feed(&mut st, "mara", "bram");
        st.perform_action(&bite);
        assert!(
            fact(&mut st, "cole.believes.bit.mara.bram.seen"),
            "cole witnessed it"
        );
        assert!(
            !fact(&mut st, "rosa.believes.bit.mara.bram.seen"),
            "rosa did not witness it"
        );
        // bring rosa to the mill with cole; cole tells her
        st.perform_outcome(&insert("practice.world.world.at.rosa!mill"))
            .expect("rosa at mill");
        let tell = find_action(&mut st, "cole", "spread word");
        st.perform_action(&tell);
        assert!(
            fact_prefix(&mut st, "rosa.believes.bit.mara.bram.heard"),
            "rosa hears of the bite from cole"
        );
        // and reclosing derives rosa's suspicion via the Task-2 axiom
        reclose(&mut st);
        assert!(
            fact(&mut st, "rosa.believes.vampire.mara"),
            "hearsay breeds suspicion too"
        );
    }

    // H: task-7a-brief.md "the endings fire as base-fact referee schedule
    // rules, harness-visible via db_child_keys"
    #[test]
    fn endings_fire_on_their_conditions() {
        // no living vampires -> village wins. Forcing everBitten first guards off
        // turn_patient_zero, so this boundary does not turn mara: the premise holds.
        let mut st = world();
        force_fact(&mut st, "everBitten");
        st.round_boundary();
        assert!(
            fact(&mut st, "ending.village"),
            "no vampires => village ending"
        );
        assert!(
            !fact(&mut st, "ending.vampires"),
            "not simultaneously the vampire ending"
        );
        // The ending is a BASE fact — what the stress harness's base-db `ending_reached`
        // reads. A view-only derivation would be invisible to every mass run.
        assert!(
            st.db_child_keys("ending").contains(&"village".to_owned()),
            "the village ending must be a base fact visible to the stress harness"
        );

        // no living humans -> vampires win.
        let mut st2 = world();
        force_fact(&mut st2, "everBitten");
        for who in [
            "aldric", "mara", "bram", "cole", "rosa", "gideon", "nessa", "tam",
        ] {
            force_fact(&mut st2, &format!("vampire.{who}"));
        }
        st2.round_boundary();
        assert!(
            fact(&mut st2, "ending.vampires"),
            "all turned => vampire ending"
        );
        assert!(
            st2.db_child_keys("ending").contains(&"vampires".to_owned()),
            "the vampire ending must be a base fact visible to the stress harness"
        );
    }

    /// Regression: on a fresh world with NO boundary yet run, neither referee
    /// rule ([`vampires_win_rule`], [`village_wins_rule`]) has fired, so no
    /// `ending.*` fact exists — not in the view, and not (the harness-visible
    /// check) in the base db.
    #[test]
    fn endings_absent_before_the_outbreak() {
        let mut st = world();
        reclose(&mut st);
        assert!(
            !fact(&mut st, "ending.village"),
            "no village ending before any boundary"
        );
        assert!(
            !fact(&mut st, "ending.vampires"),
            "no vampire ending before any boundary"
        );
        assert!(
            st.db_child_keys("ending").is_empty(),
            "no base ending fact before any boundary"
        );
    }

    /// Regression: [`village_wins_rule`]'s `absent(vampire.V ...)` clause holds
    /// VACUOUSLY when no vampire exists yet — the same way it holds once the
    /// village has actually won. The `matches("everBitten")` guard is what
    /// keeps the rule from firing on that vacuous truth before the outbreak
    /// has even begun. The full [`vampire_world`] can't isolate this: its
    /// first night boundary fires `turn_patient_zero`, which sets `everBitten`
    /// itself. So this builds a MINIMAL `State` carrying only the `villageWins`
    /// referee rule and no `turnPatientZero`, keeping `everBitten` unset across
    /// a boundary. Falsifiable by construction: remove `matches("everBitten")`
    /// from [`village_wins_rule`] and this test fails (RED/GREEN transcript in
    /// the task report).
    #[test]
    fn everbitten_guard_blocks_endings_before_the_outbreak() {
        let mut st = State::new();
        st.set_characters(vec![Character::new("a"), Character::new("b")])
            .expect("minimal roster");
        st.set_schedule(vec![village_wins_rule()])
            .expect("minimal schedule");
        st.seed_die(VAMPIRE_SEED).expect("seed");
        st.round_boundary();
        assert!(
            !fact(&mut st, "ending.village"),
            "the everBitten guard must block the referee rule before the outbreak"
        );
        assert!(
            st.db_child_keys("ending").is_empty(),
            "no base ending fact either"
        );
    }

    /// The skeleton's end-to-end acceptance: driven by the REAL game loop
    /// (`advance` to select the next actor, then `npc_act` to deliberate and
    /// act — exactly what `prax play` runs, at the same depth), the infection
    /// spreads through the whole cast and reaches a terminal ending. With only
    /// feeding and turning — no human counterplay yet (detection/elimination
    /// are later plans) — the vampires take the village; the point is that the
    /// loop CLOSES, deterministically. It closes at ~208 character-turns
    /// (≈26 rounds × 8 actors); the 400 ceiling is generous headroom, present
    /// only so a regression that stalls the loop fails loudly instead of
    /// looping forever.
    #[test]
    fn the_infection_runs_to_an_ending() {
        use prax_core::turn::{advance, npc_act};
        // The lookahead depth the CLI plays the cast at (prax-cli LOOKAHEAD_DEPTH):
        // a hungry vampire must foresee move-then-feed, which is exactly depth 2.
        let depth = 2;
        let mut st = world();
        let mut reached = None;
        for _ in 0..400 {
            let actor = advance(&mut st);
            npc_act(&mut st, depth, &actor);
            if let Some(e) = st.db_child_keys("ending").into_iter().next() {
                reached = Some(e);
                break;
            }
        }
        assert_eq!(
            reached.as_deref(),
            Some("vampires"),
            "the skeleton loop must close to the vampire ending; got {reached:?}"
        );
    }

    // H: detection spec "the crux — the vampire emergently disguises before feeding"
    #[test]
    fn the_vampire_conceals_itself_by_disguising_before_feeding() {
        use prax_core::turn::{advance, npc_act};
        let mut st = world();
        let mut fed_at_least_once = false;
        for _ in 0..400 {
            let actor = advance(&mut st);
            npc_act(&mut st, 2, &actor);
            // `fact_prefix` appends its own `.`/`!` separator (see its
            // definition above); the family root is "bittenOn" with NO
            // trailing dot, matching every other call site in this file
            // (e.g. `fact_prefix(&mut st, "bittenOn.bram")`). A trailing dot
            // here would query for "bittenOn.." / "bittenOn.!", which no
            // real fact (`bittenOn.<victim>!<turn>`) ever matches.
            if fact_prefix(&mut st, "bittenOn") {
                fed_at_least_once = true;
            }
            // No character is ever pinned as a vampire by name — every vampire
            // disguises before biting (the inert `.believes.vampire.someone` a
            // masked bite derives is excluded: `someone` is not a villager).
            assert!(
                !st.labeled_view().iter().any(|f| {
                    HOMES
                        .iter()
                        .any(|(who, _)| f.ends_with(&format!(".believes.vampire.{who}")))
                }),
                "every vampire must stay concealed — each disguises before biting"
            );
            if !st.db_child_keys("ending").is_empty() {
                break;
            }
        }
        assert!(
            fed_at_least_once,
            "the vampire did feed (concealed), not merely avoid feeding"
        );
    }

    // H: task-4-brief.md "(a) unobserved turning — no non-household member is
    // co-present with patient zero at the turn, and no belief forms from it"
    //
    // Driven by the REAL game loop (`advance` + `npc_act`), stepped ONE actor at
    // a time so the check lands on the exact step patient zero turns — not a
    // round-granularity read after the cast has moved. `turn_patient_zero` is
    // gated to fire only once the village has gone home for the night (no member
    // whose `home` is elsewhere is co-present with mara), so she genuinely turns
    // with only her own household present. Without that gate she would turn at
    // the night's first instant, in the day's square-crowd — this test is that
    // gate's regression net.
    #[test]
    fn mara_turns_unobserved_by_non_household() {
        use prax_core::turn::{advance, npc_act};
        let mut st = world();
        let household = household_of("mara");
        let steps = TimeScale::test().cycle() * HOMES.len() as i64 * 3;
        let mut turned = false;
        for _ in 0..steps {
            let was = fact(&mut st, "vampire.mara");
            let actor = advance(&mut st);
            npc_act(&mut st, 2, &actor);
            if fact(&mut st, "vampire.mara") && !was {
                turned = true;
                break;
            }
        }
        assert!(turned, "patient zero must turn within the run");
        // Read at the exact step she turns:
        assert!(fact(&mut st, "phase!night"), "patient zero turns at night");
        let mara_place = place_of(&mut st, "mara");
        assert_eq!(
            place_of(&mut st, "mara").as_deref(),
            Some("square"),
            "patient zero turns at her own home"
        );
        for (who, _) in HOMES {
            if household.contains(who) {
                continue;
            }
            assert_ne!(
                place_of(&mut st, who),
                mara_place,
                "{who} (not mara's household) is co-present with her the moment she \
                 turns; positions: {:?}",
                all_positions(&mut st)
            );
        }
        assert!(
            !st.labeled_view()
                .iter()
                .any(|f| f.ends_with(".believes.vampire.mara")),
            "no one believes mara a vampire from the (unwitnessed) turning"
        );
    }

    // H: task-4-brief.md "(b) nocturnal, concealed feeding"
    //
    // At the compressed test scale this asserts the LOCAL claims: the first bite
    // lands on mara's reachable housemate (aldric, home together at the square),
    // and the concealment invariant holds across the whole run. It does NOT
    // assert nocturnality: `turn_patient_zero` is now gated to fire only in DEEP
    // night, once the village has dispersed home (so the turning is unobserved,
    // per test `mara_turns_unobserved_by_non_household`) — which at test scale is
    // the last of the 2-turn night, leaving no room to disguise-then-bite before
    // dawn, so the first bite spills into the next day. That is a compression
    // artifact: at `TimeScale::real()` the 96-turn night has ample room, and the
    // first bite IS nocturnal (spot-checked directly: victim aldric at turn 194,
    // `phase!night`). Nocturnal feeding is asserted at real scale in the ignored
    // `real_scale_infection_closes_and_stays_concealed` test below (Task 5), which
    // now genuinely exists and carries that assertion as a committed check.
    #[test]
    fn the_reachable_first_bite_lands_on_the_housemate_and_stays_concealed() {
        use prax_core::turn::{advance, npc_act};
        let mut st = world();
        let mut first_victim: Option<String> = None;
        for _ in 0..400 {
            let actor = advance(&mut st);
            npc_act(&mut st, 2, &actor);
            if first_victim.is_none() {
                for (who, _) in HOMES {
                    if fact_prefix(&mut st, &format!("bittenOn.{who}")) {
                        first_victim = Some((*who).to_owned());
                        break;
                    }
                }
            }
            assert!(
                !st.labeled_view().iter().any(|f| {
                    HOMES
                        .iter()
                        .any(|(who, _)| f.ends_with(&format!(".believes.vampire.{who}")))
                }),
                "every vampire must stay concealed — no one is ever believed a vampire by name"
            );
            if !st.db_child_keys("ending").is_empty() {
                break;
            }
        }
        let victim = first_victim.expect("at least one bite must occur in the run");
        assert!(
            household_of("mara").contains(&victim.as_str()),
            "the first (reachable, same-household) bite should land on mara's own \
             housemate; got {victim}"
        );
    }

    // H: task-5-brief.md "real-scale offline validation" — the ONLY committed
    // assertion of nocturnal cross-household feeding, since
    // `the_reachable_first_bite_lands_on_the_housemate_and_stays_concealed`'s own doc
    // comment explains that the compressed test scale cannot exercise it (the 2-turn
    // night leaves no room to disguise-then-bite before dawn). Driven by the REAL
    // game loop (`advance` + `npc_act`, exactly what `the_infection_runs_to_an_ending`
    // uses at test scale) but against [`vampire_world`] — the REAL `TimeScale::real()`
    // build, day 192 / night 96 / incubation 288 / cooldown 288 — not the `world()`
    // test helper. SLOW (tens of thousands of character-turns), so `#[ignore]`d out
    // of the fast suite per the spec's "mass-run is offline"; run explicitly with
    // `--ignored`.
    #[test]
    #[ignore = "real-scale: thousands of turns; run with --ignored"]
    fn real_scale_infection_closes_and_stays_concealed() {
        use prax_core::turn::{advance, npc_act};
        use std::time::Instant;
        let mut st = vampire_world(); // real scale: TimeScale::real()
        let started = Instant::now();
        // Cap: 30 real days (30 * scale.cycle() = 30 * 288 = 8640 turns) times the
        // 8-strong cast (one character-turn per `advance`+`npc_act` step, per
        // `prax_core::turn::advance`'s round-boundary-on-wrap semantics — the same
        // reasoning `the_infection_runs_to_an_ending`'s doc comment applies at test
        // scale) is ~69120 steps; generous headroom on top so a regression that
        // stalls the loop fails loudly instead of looping forever, rather than the
        // cap itself being a tight tuned bound.
        let cap_steps = 8640_i64 * HOMES.len() as i64 + 30_000;
        // (victim, turn, was_night) for the FIRST bite in the run — criterion #5
        // (nocturnality), tracked at the exact step it appears rather than sampled
        // after the fact, the same idiom
        // `the_reachable_first_bite_lands_on_the_housemate_and_stays_concealed` uses.
        let mut first_bite: Option<(String, i64, bool)> = None;
        let mut reached = None;
        let mut steps_taken = 0_i64;
        for _ in 0..cap_steps {
            steps_taken += 1;
            let actor = advance(&mut st);
            npc_act(&mut st, 2, &actor);
            if first_bite.is_none() {
                for (who, _) in HOMES {
                    if fact_prefix(&mut st, &format!("bittenOn.{who}")) {
                        first_bite = Some((
                            (*who).to_owned(),
                            st.current_turn(),
                            fact(&mut st, "phase!night"),
                        ));
                        break;
                    }
                }
            }
            // The concealment invariant, HOMES-wide, held throughout the entire
            // real-scale run — the same check
            // `the_vampire_conceals_itself_by_disguising_before_feeding` and
            // `the_reachable_first_bite_lands_on_the_housemate_and_stays_concealed` make at
            // test scale, now checked against the real 288-turn incubation/cooldown
            // timers rather than the compressed 6-turn ones.
            assert!(
                !st.labeled_view().iter().any(|f| {
                    HOMES
                        .iter()
                        .any(|(who, _)| f.ends_with(&format!(".believes.vampire.{who}")))
                }),
                "every vampire must stay concealed at real scale — no one is ever believed \
                 a vampire by name"
            );
            if let Some(e) = st.db_child_keys("ending").into_iter().next() {
                reached = Some(e);
                break;
            }
        }
        let elapsed = started.elapsed();
        let (victim, bite_turn, was_night) =
            first_bite.expect("at least one bite must occur within the real-scale run's turn cap");
        println!(
            "real-scale run: closed at turn {} ({} character-turns, cap {cap_steps}) in {:?}; \
             first bite victim={victim} turn={bite_turn} phase!night={was_night}; ending={:?}",
            st.current_turn(),
            steps_taken,
            elapsed,
            reached
        );
        // (a) the first bite is nocturnal.
        assert!(
            was_night,
            "the first bite must land at night at real scale; victim={victim} \
             turn={bite_turn} phase!night={was_night}"
        );
        // (c) the run closes to the vampire ending.
        assert_eq!(
            reached.as_deref(),
            Some("vampires"),
            "the real-scale run must close to the vampire ending within {cap_steps} steps; \
             got {reached:?}"
        );
    }
}
