# Vampire Time-Model Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Put the vampire game on the foundational ~5-min-per-turn scale ([`docs/PRINCIPLES.md`](../../PRINCIPLES.md)): real-scale timers and a real day/night span, a night/home location rhythm that makes turning unobserved and feeding nocturnal, and a single `TimeScale` value so the test suite runs a compressed (labelled) scale while play and the offline mass-run run the real one.

**Architecture:** All changes in `rust/prax-worlds/src/vampire.rs`. Durations move out of the `TURN_DELAY`/`FEED_COOLDOWN` constants into a `TimeScale` struct (`real()`/`test()`) threaded through `vampire_world`. The `phase_clock` is rebuilt to span a real day. The always-on home anchor becomes a night-home + day-square want pair. Existing time-dependent tests move to the compressed test scale.

**Tech Stack:** Rust, workspace at `rust/Cargo.toml`. Build/test: `cargo … --manifest-path rust/Cargo.toml -p prax-worlds --lib`.

## Global Constraints

- Only `rust/prax-worlds/src/vampire.rs` changes. `git checkout --` sibling `cargo fmt` drift.
- `type_check(&st)` stays empty at BOTH scales.
- **Time-compression vigilance (standing rule):** when a test behaves unexpectedly at the compressed `test()` scale, first determine whether compression is distorting the dynamics (a behavior that needs a long night/day span can't occur in a short one) versus a real defect — do NOT weaken an assertion to make a compressed run pass, and do NOT let a test pass only because the compressed timing accidentally aligns. If a dynamic genuinely cannot be exercised at test scale, assert the *local* part at test scale and validate the rest at real scale (Task 5); state the limitation in the test.
- `TimeScale::test()` MUST preserve `TimeScale::real()`'s ratios (night:day, incubation:day, cooldown:day) and document the real value beside each compressed one. Never present a compressed value as the real semantics.
- Real values (per PRINCIPLES): 1 turn = 5 min; 24h = 288 turns. Registration (`worlds::build`, CLI) and the mass-run use `real()`; the test suite uses `test()`.

## The `TimeScale` (introduced in Task 1, used throughout)

```rust
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
        Self { day_turns: 192, night_turns: 96, incubation: 288, cooldown: 288 }
    }
    /// A compressed scale for tests — same ratios (day:night 2:1, timer:day 1.5:1), tractable
    /// turn counts. NOT the real semantics; the real values are on `real()`.
    fn test() -> Self {
        Self { day_turns: 4, night_turns: 2, incubation: 6, cooldown: 6 }
    }
    fn cycle(&self) -> i64 { self.day_turns + self.night_turns }
}
```

---

### Task 1: Thread `TimeScale` through the world (timers), tests to the test scale

**Files:** `rust/prax-worlds/src/vampire.rs`. **Interfaces produced:** `vampire_world()` (real scale, for registration), a test-scale builder for the suite; `transformation`/feed cooldown read the scale.

- [ ] **Step 1: Add `TimeScale`** (the struct above) near the top of the module.

- [ ] **Step 2: Parameterize the builder.** Change `pub fn vampire_world() -> State` to build from a scale:
```rust
pub fn vampire_world() -> State { vampire_world_scaled(TimeScale::real()) }
fn vampire_world_scaled(scale: TimeScale) -> State { /* the current body, using `scale` */ }
```
Registration callers (`worlds::build`, CLI `world_named`) keep calling `vampire_world()` → real. Every `#[cfg(test)]` caller changes to `vampire_world_scaled(TimeScale::test())` — add a test helper `fn world() -> State { vampire_world_scaled(TimeScale::test()) }` and route the tests' `vampire_world()` calls (and `seeded_two_at`, which builds one) through it.

- [ ] **Step 3: Timers read the scale.** Remove `const TURN_DELAY` / `const FEED_COOLDOWN`. `transformation` and `prey_practice`'s `insert_for(...)` take the scale's `incubation`/`cooldown`. Since these are built inside `vampire_world_scaled`, thread `scale` into `transformation(scale)` and `prey_practice(scale)` (and any helper that used the constants). The transformation axiom's `cmp(Gte, "E", scale.incubation.to_string())`; the feed's `insert_for(scale.cooldown, "fed.Actor")`.

- [ ] **Step 4: Keep phase as-is for now.** Do NOT touch `phase_clock` yet (Task 2). At this step the world still flips phase per turn; `TimeScale`'s `day_turns`/`night_turns` are unused by the clock until Task 2. This keeps the diff reviewable and tests green.

- [ ] **Step 5: Update the tests' timing.** Tests that `advance_boundaries(TURN_DELAY, ...)` become `advance_boundaries(TimeScale::test().incubation, ...)` (= 6). Run `cargo test --manifest-path rust/Cargo.toml -p prax-worlds --lib`. Every test green. **Vigilance:** a test that turned a victim after 2 boundaries now needs 6 — confirm the failure/adjustment is purely the timer change, not a dynamics distortion.

- [ ] **Step 6: Commit** — `git commit -am "vampire(time): thread TimeScale through the world; tests on the compressed scale"`.

---

### Task 2: The phase clock spans a real day

**Files:** `rust/prax-worlds/src/vampire.rs`. **Consumes:** `TimeScale`. **Produces:** `phase!day` for the first `day_turns` of each cycle, `phase!night` for the rest — at both scales.

- [ ] **Step 1: Write the failing test.**
```rust
    // H: time-model spec "phase spans a real day, not a flip every turn"
    #[test]
    fn phase_spans_the_day_then_the_night() {
        let scale = TimeScale::test(); // day 4 / night 2, cycle 6
        let mut st = vampire_world_scaled(scale);
        // turns 0..day_turns are day; day_turns..cycle are night; then it repeats.
        for k in 0..(scale.cycle() * 2) {
            // read the phase AFTER the boundary that stamps turn k+1... drive one boundary at a time
        }
        // Assert: at a turn whose position in the cycle is < day_turns, phase!day holds;
        // otherwise phase!night. (Drive boundaries and check `fact(&mut st,"phase!day")` /
        // `phase!night` against `turn mod cycle`.)
    }
```
Write it concretely: drive `scale.cycle()*2` boundaries, and at each, assert `phase!day` iff `(current_turn mod cycle) < day_turns`. Use `st.current_turn()` (or read `turn!Now` from the view) for the position.

- [ ] **Step 2: Run it — FAIL** (the per-turn parity clock flips every turn).

- [ ] **Step 3: Rebuild `phase_clock`** as a two-arm rule off the cycle. Replace the `phaseOfParity` table and the `mod 2` clause:
```rust
fn phase_clock(scale: TimeScale) -> ScheduleRule {
    ScheduleRule::new("phase", 1)
        // day: position in the day/night cycle is before dusk
        .clause(
            [matches("turn!Now"), calc("InCycle", CalcOp::Mod, "Now", scale.cycle().to_string()),
             cmp(CmpOp::Lt, "InCycle", scale.day_turns.to_string())],
            [insert("phase!day")],
        )
        // night: at or past dusk
        .clause(
            [matches("turn!Now"), calc("InCycle", CalcOp::Mod, "Now", scale.cycle().to_string()),
             cmp(CmpOp::Gte, "InCycle", scale.day_turns.to_string())],
            [insert("phase!night")],
        )
}
```
`phase!X` is single-valued (`!`), so each boundary's insert replaces the previous phase. Remove the `phaseOfParity.0!day`/`.1!night` setup facts and the `phase!day` seed (or keep an initial `phase!day` for turn 0). Confirm `calc`/`cmp`/`CalcOp::Mod`/`CmpOp::Lt`/`Gte` are the same imports the file already uses.

- [ ] **Step 4: Run — PASS.** Then the full suite. **Vigilance:** `turn_patient_zero` fires on the first `phase!night`, which is now turn `day_turns` (= 4 at test scale), not turn 1. Tests like `advance_to_first_night` must reach turn `day_turns`; `endings_absent_before_the_outbreak` and any "before the outbreak" test must account for the outbreak now starting at the first night (turn `day_turns`). Update these, confirming each change is the phase-span shift, not a broken assertion.

- [ ] **Step 5: Commit** — `git commit -am "vampire(time): phase_clock spans a real day/night, not a per-turn flip"`.

---

### Task 3: The night/home + day/square rhythm

**Files:** `rust/prax-worlds/src/vampire.rs`. **Produces:** villagers home at night, in the square by day.

- [ ] **Step 1: Write the failing test.** A seeded run: at a night turn every villager is at their `HOMES` location; at a day turn they are (mostly) at the square. Drive the real `advance`+`npc_act` loop through at least one full cycle and assert positions by phase. (Assert the strong claim for night — everyone home; a looser claim for day — at least most at square — since day movement is milder.)

- [ ] **Step 2: Run it — FAIL** (the always-home anchor keeps them home day and night).

- [ ] **Step 3: Replace the home anchor with the rhythm wants.** In `vampire_cast`'s `HOMES` closure, drop the `+1` always-home `Want` and add:
```rust
    // night → home: be at one's own home while it is night (strong — this is what puts the
    // household together at night, makes the turning unobserved, and makes night the feeding
    // window). Weight tuned in Task 4; principled, not magic.
    .want(Want::new(vec![matches("phase!night"),
        matches(format!("practice.world.world.at.{who}!{home}"))], NIGHT_HOME_WEIGHT))
    // day → out: be in the square while it is day (mild — market/gossip), so the day/night
    // distinction actually moves people off their homes.
    .want(Want::new(vec![matches("phase!day"),
        matches(format!("practice.world.world.at.{who}!square"))], DAY_SQUARE_WEIGHT))
```
Add the weight consts with justification (start `NIGHT_HOME_WEIGHT = 3`, `DAY_SQUARE_WEIGHT = 1` — night pull stronger than day so home wins at night; tuned in Task 4). Note the sate-hunger and conceal wants are unchanged.

- [ ] **Step 4: Run — PASS** (tune weights until the rhythm holds). Full suite green. **Vigilance:** a 2-turn night (test scale) is short — confirm the villagers actually reach home within the night span (homes are one hop from the square, so one move suffices). If they can't reach home in a compressed night, that is a compression artifact — note it and, if needed, assert the rhythm only at real scale in Task 5 while keeping a weaker test-scale check.

- [ ] **Step 5: Commit** — `git commit -am "vampire(time): night/home + day/square location rhythm"`.

---

### Task 4: Turning is unobserved; feeding is nocturnal (acceptance at test scale)

**Files:** `rust/prax-worlds/src/vampire.rs`. **Consumes:** everything above.

- [ ] **Step 1: Write the acceptance tests.**
  (a) **Unobserved turning:** in a seeded real-loop run, at the turn patient zero turns (`vampire.mara` first appears), no non-household member is co-present, and no `.believes.vampire.mara` forms from the turning.
  (b) **Nocturnal, concealed feeding:** across the run, bites (`bittenOn.*` appearing) occur while `phase!night`, and the concealment invariant from detection (no character ever believes any villager a vampire by name — the `HOMES`-wide check) still holds at the test scale.

- [ ] **Step 2: Run — likely needs weight tuning** (the night-home/day-square/conceal weights must combine so the vampire is home at night with its housemate and feeds there, concealed). Tune `NIGHT_HOME_WEIGHT`/`DAY_SQUARE_WEIGHT` (not the conceal weight, fixed in detection). If NO weighting yields nocturnal concealed feeding at test scale, STOP and report — that is a real finding about the dynamics (possibly a compression artifact: a 2-turn night may be too short to both gather home and feed — check at real scale before concluding the design is wrong).

- [ ] **Step 3: Run the merged acceptances** — `the_infection_runs_to_an_ending` (adjust its cap to the compressed turn counts: incubation 6, cooldown 6, cycle 6 — the loop should still close to `ending.vampires`, likely in a few hundred turns) and the detection concealment invariant. Both green. **Vigilance:** if the infection no longer closes, determine whether the night/home rhythm genuinely stalls spread (real finding) or the compressed night is too short for cross-household hunting (compression artifact — validate at real scale in Task 5, and if so, document that the suite covers single-household spread while real scale covers the rest).

- [ ] **Step 4: Commit** — `git commit -am "vampire(time): acceptance — unobserved nocturnal turning and concealed feeding"`.

---

### Task 5: Real-scale validation (offline, out of the fast suite)

**Files:** `rust/prax-worlds/src/vampire.rs`. **Produces:** confidence the real scale runs and behaves, and coverage of the dynamics compression cannot exercise.

- [ ] **Step 1: Add a real-scale test, marked `#[ignore]` with a reason** (it is slow — thousands of turns — so it is not in the default fast suite, per the spec's "mass-run is offline"):
```rust
    /// Real-scale behavioural check — SLOW (thousands of turns). Run explicitly with
    /// `--ignored`. Validates what the compressed suite cannot: a full real day/night span,
    /// unobserved nocturnal turning, and the infection closing over the real 288-turn timers.
    #[test]
    #[ignore = "real-scale: thousands of turns; run with --ignored"]
    fn real_scale_infection_closes_and_stays_concealed() {
        let mut st = vampire_world(); // real scale
        // drive the real loop to an ending (cap generously, e.g. 30 real days ~= 8640 turns +
        // headroom); assert it closes to `ending.vampires`, the concealment invariant holds
        // throughout, and turning/feeding happened at night.
    }
```

- [ ] **Step 2: Run it explicitly** — `cargo test --manifest-path rust/Cargo.toml -p prax-worlds --lib real_scale_infection_closes -- --ignored --nocapture`. Confirm it closes and stays concealed at real scale. Record the turn count and wall-clock in the report (this is the perf datapoint the mass-run depends on). If it does NOT close or concealment breaks at real scale, that is a genuine design finding (not compression) — report it.

- [ ] **Step 3: Confirm the fast suite excludes it** — `cargo test --manifest-path rust/Cargo.toml -p prax-worlds --lib` does not run the ignored test and stays fast and green.

- [ ] **Step 4: Commit** — `git commit -am "vampire(time): real-scale offline acceptance (ignored, slow)"`.

---

## Self-Review

**Spec coverage:** real-scale `TimeScale` with compressed `test()` (T1) ✓; phase spans a real day (T2) ✓; night/home + day/square rhythm (T3) ✓; unobserved nocturnal turning + concealed feeding + infection closes at test scale (T4) ✓; real-scale offline validation (T5) ✓. Deferred (spec scope-out): richer day schedules, cast scale, the mark/scarf channel, perf work.

**Placeholder scan:** the phase-clock and rhythm code is concrete; the tuned weights (`NIGHT_HOME_WEIGHT`/`DAY_SQUARE_WEIGHT`) and the compressed `test()` values are flagged as tuned/labelled, with the real values documented beside them — the same empirical-tuning pattern as the skeleton anchor and the detection conceal weight. Test bodies for T2–T5 describe the exact assertions; the implementer fills the boundary-driving loop against the stated conditions.

**Type consistency:** `TimeScale` fields (`day_turns/night_turns/incubation/cooldown`) are used consistently; `phase!day`/`phase!night` and `practice.world.world.at.<who>!<place>` match the existing fact shapes; the conceal invariant reuses detection's `HOMES`-wide check.

**Risk carried forward:** the compression-distortion risk is the standing constraint above and is called out per-task; the crux (does the rhythm produce nocturnal concealed feeding, and does the infection still close) is T4, validated empirically at test scale and confirmed at real scale in T5. If a dynamic proves unreachable at test scale, the split is: assert the local part in the fast suite, the rest in the ignored real-scale test.
