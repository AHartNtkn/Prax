# Vampire Village — time-model redesign

The skeleton and detection were built on a fabricated time scale: a comment asserting "2 turns
= a 24h day-night cycle" (a turn = 12 hours), invented to fit the design's "24h" timers because
the foundational ~5-minutes-per-turn assumption was unrecorded (now [`docs/PRINCIPLES.md`](../../PRINCIPLES.md)).
This redesign puts the vampire game on the real scale and gives it the day/night location
rhythm the fiction requires — the thing that makes the turning unobserved and makes night the
feeding window.

## The real scale (per PRINCIPLES.md)

- **1 turn = 5 minutes.** 1 hour = 12 turns; **24 hours = 288 turns.**
- **A day is 288 turns**, split into a daylight span and a night span (proposed default:
  **192 daylight / 96 night** — ~16h waking, ~8h night; tunable).
- **The 24h timers are 288 turns:** the incubation delay (bite → turn) and the feed cooldown.
- A ~20–30 day game is therefore **~5,760–8,640 turns**. That is the intended scale; the
  designer approved the design under it.

## The day/night location rhythm (the missing behavior)

The skeleton has no day/night behavior: villagers drift and cluster regardless of phase, so
patient zero can turn in a crowd. The fiction requires **villagers home at night**. This is a
want, not a fiat schedule (nothing by fiat):

- **Night → home:** a strong want to be at one's own `HOMES` location while `phase!night`.
- **Day → out:** a milder want to be at the `square` (market/gossip) while `phase!day`, so the
  day/night distinction actually moves people (otherwise a standing home-anchor keeps them home
  around the clock). This **replaces** the skeleton's always-on `+1` home anchor.

**What this produces (to be validated empirically, like the Part-1 crux):** at night each
household is home together (two to a house in the current roster), so **night is the feeding
window** — a vampire is alone with its housemate, and the turning itself happens at night, at
home, unobserved. By day everyone is out in the square, where an open bite is witnessed — so
feeding is pushed into the night, exactly as the fiction intends. Once a housemate turns, the
household is out of prey and the vampire must hunt other households at night. The concealment,
disguise, and witnessed-bite machinery from detection Part 1 all still apply.

## Test compression (per PRINCIPLES.md — labelled, never asserted as real)

Real-scale games are thousands of turns; the **test suite cannot run that**. So the durations
live in a single `TimeScale` value, and the world builder takes one:

- `TimeScale::real()` — the 5-minute scale above (`day: 192, night: 96, incubation: 288,
  cooldown: 288`). Used by `prax play` and the offline mass-run.
- `TimeScale::test()` — a compressed scale that **preserves the ratios** (proposed: `day: 4,
  night: 2, incubation: 6, cooldown: 6`), so a mechanic/acceptance test closes in hundreds of
  turns, not thousands, while exercising the same qualitative dynamics. Used by the test suite.

The compression preserves the ratios that drive behaviour (night:day, incubation:day,
cooldown:day). The one ratio compression cannot fully preserve is **movement-per-night** (how
many locations a vampire can traverse in one night): at real scale a 96-turn night allows wide
roaming, at test scale a 2-turn night does not. Tests therefore assert the *local* dynamics
(feed on a co-present housemate; turning unobserved at night; concealment holds); the
wide-roaming, cross-household hunting is validated at real scale in the offline run, not the
suite. This limitation is stated, not hidden.

**The mass-run** (`prax stress`, the statistics/mining goal) runs at real scale and may take
hours; it is a rare offline activity, explicitly NOT part of the test suite.

## Mechanics to change

1. **`TimeScale`** — a small struct (`day_turns, night_turns, incubation, cooldown`), with
   `real()` and `test()`. `vampire_world(scale: TimeScale)` threads it. `TURN_DELAY` /
   `FEED_COOLDOWN` constants are removed in favour of `scale.incubation` / `scale.cooldown`.
   The transformation axiom and the feed cooldown read the scale's values.
2. **`phase_clock`** — rebuilt to span a real day. Instead of `phase = turn mod 2`, compute
   `in_cycle = turn mod (day_turns + night_turns)` and set `phase!day` when `in_cycle <
   day_turns`, else `phase!night` (a two-arm rule with `calc` + `cmp`; the static
   `phaseOfParity` lookup table is removed).
3. **The night/day wants** — replace the always-on home anchor in `vampire_cast` with the
   night-home and day-square wants above (built from `HOMES`).
4. **`turn_patient_zero`** — still fires on the first `phase!night`; with a real night span that
   is turn `day_turns` (e.g. 192), when the household is home. Its guard logic is unchanged.
5. **hunger_pulse** stays period-1 (idempotent, gated) — it arms hunger promptly once the (now
   288-turn) cooldown clears; the cooldown sets the feed cadence, not the pulse period.

## Impact on the merged skeleton + detection

Every test that hard-codes `TURN_DELAY = 2`, the per-turn phase flip, or advances a fixed small
number of boundaries must move to `TimeScale::test()` and the new phase spans. The behavioural
acceptance tests (`the_infection_runs_to_an_ending`, the concealment invariant) must still hold
at the test scale — re-validated, with caps adjusted to the compressed turn counts. This is a
substantial, deliberate re-capture of the time-dependent tests; no assertion is weakened.

## Success criteria (behavioural — no frozen oracle)

1. `type_check` clean; the world builds at both scales; the real-scale build runs without error.
2. **Phase spans a real day:** at `TimeScale::real()`, `phase!day` holds for the first
   `day_turns` of each cycle and `phase!night` for the rest — not a flip every turn.
3. **Night/home rhythm:** in a seeded run, villagers are home at night and out (square) by day.
4. **Turning is unobserved:** patient zero turns at night, at home, with no non-household member
   co-present — so no `.believes.vampire.<her>` forms from the turning itself.
5. **Feeding is nocturnal and concealed:** bites occur at night; the detection invariant (no one
   believes any vampire by name) still holds at the test scale.
6. **The infection still closes** to `ending.vampires` at the test scale, within an adjusted cap.
7. The compressed test scale's ratios match the real scale's, documented beside the values.

## Open decisions (flagged for review; I have proposed defaults above)

- The **day/night split** (192/96 proposed).
- The **test-scale values** (day 4 / night 2 / timers 6 proposed) — must keep the ratios.
- The **night-home vs day-square want weights** (tuned empirically, like the Part-1 conceal
  weight; principled magnitudes, not magic numbers).

## Scope out

Richer day schedules (professions, market hours, Sabbath), scaling the cast, the mark/scarf
channel and snatch (detection Part 2/3), and any perf work the real-scale mass-run may motivate.
