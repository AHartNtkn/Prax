# Vampire Village — Walking Skeleton Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A minimal but *running* vampire-infection sim in Prax — a small cast on the reused village movement/sight substrate, where a patient-zero vampire feeds (spreading the curse), victims transform after a delay, and the game reaches one of two endings — with a behavioral test for every mechanic.

**Architecture:** A new `prax-worlds` world module (`vampire.rs`) built exactly like `village.rs`: `State::new()` → `define_practices` → `define_functions` → `set_characters` → `set_schedule` → `perform_outcome`(setup) → `set_axioms` → `set_desires` → `seed_die` → `set_prediction_scope`. It reuses the village's `world` (movement) practice and `sight_rule` scope; the vampire mechanics are new practices/actions/axioms/wants. Timers use `insert_for(n, …)` (expiring cooldowns) and turn-arithmetic axioms (transformation threshold). Endings are `ending.<name>` facts detected by the existing walk/stress harness.

**Tech Stack:** Rust (edition 2024), the Prax `prax-core` engine + `prax-worlds` content crate + `prax-oracle`/`prax-cli` registration. Tests are `#[test]`s in `prax-worlds/src/vampire.rs`'s `mod tests`, driving the built `State` through `advance`/`pick_action`/`perform_outcome` and asserting on the view via `to_sentences`/`exists`.

## Global Constraints

- **No fiat.** Every behaviour emerges from wants + the planner or from axioms/schedule; never hard-wire a decision. (Cooldowns/timers/endings are world *rules*, which is legitimate — they are physics, not decisions.)
- **Test every mechanic as it is built** (project rule): each task ships with a behavioral test that fails before and passes after. No batching tests to the end.
- **Reuse the village substrate** where it fits (movement `world` practice, `sight_rule` scope, relationship/feud facts) rather than re-authoring it.
- **This is new content — no frozen oracle.** Validation is behavioral (the tests here) + `type_check == []` + the sim running under `stress`/`randtrace`. Do NOT wire it into the frozen differential.
- Facts, practices, and axiom forms named below are the committed design for the skeleton. If a test reveals a chosen form is wrong, fix the *world*, keep the test's asserted behaviour.
- The engine is under a live perf-optimization line of work but the authoring API (`State`, `Practice`, `Action`, `Outcome`, `Axiom`, `Want`, `Desire`, `ScheduleRule`, `Character`) is stable; build against it as `village.rs` does.

---

### Task 1: World scaffold that builds and type-checks

A `vampire_world()` with a cast of 8 on the village movement substrate and **no vampire mechanics yet** — the empty stage.

**Files:**
- Create: `rust/prax-worlds/src/vampire.rs`
- Modify: `rust/prax-worlds/src/lib.rs` (add `pub mod vampire;`)
- Test: in `vampire.rs` `#[cfg(test)] mod tests`

**Interfaces:**
- Produces: `pub fn vampire_world() -> prax_core::engine::State` — the built, type-checkable world. Later tasks add practices/axioms/wants to it.
- Consumes: `prax_worlds`-internal reuse of the village's `world` movement practice and `sight_rule` — copy the two small helpers (`world_practice`, a `village_sighting`-shaped scope) into `vampire.rs` rather than `pub`-exporting village internals (keeps worlds independent).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn the_world_builds_and_is_well_formed() {
    let st = vampire_world();
    assert_eq!(
        prax_core::typecheck::type_check(&st),
        vec![],
        "the vampire world must be well-formed"
    );
    // Eight named villagers exist as characters.
    for who in ["aldric", "mara", "bram", "cole", "rosa", "gideon", "nessa", "tam"] {
        assert!(
            st.db().exists_str(&mut st.interner_clone(), &format!("character.{who}")),
            "{who} should be a character in the world"
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p prax-worlds --lib vampire::tests::the_world_builds_and_is_well_formed`
Expected: FAIL — `vampire_world` not defined (compile error).

- [ ] **Step 3: Write minimal implementation**

Author `vampire_world()` mirroring `village_world()`'s assembly, but with only: a `world` movement practice (copied from village), a `Places` setup (a few connected locations: `square`, `church`, `mill`, `home`), a roster of 8 `Character::new(...)` with a couple of connected locations placed via setup `insert("practice.world.world.at.<who>!<place>")`, the `sight_rule(sighting())` schedule, `seed_die(<pick>)`, and `set_prediction_scope(sighting())`. No vampire facts yet.

Note the exact helper `exists_str`/`interner_clone` used in the test may not exist verbatim on `State` — if not, use the existing accessor the village tests use to check a fact (grep `village.rs` tests for the pattern, e.g. building a probe query) and mirror it. The asserted *behaviour* (character facts present, type_check clean) is fixed.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p prax-worlds --lib vampire::tests::the_world_builds_and_is_well_formed`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/prax-worlds/src/vampire.rs rust/prax-worlds/src/lib.rs
git commit -m "vampire: world scaffold — 8-villager cast on the movement substrate, type-clean"
```

---

### Task 2: Patient zero turns on the first night

One villager becomes a vampire after day 1, via a one-shot schedule rule keyed on the day/night clock. Establishes the `vampire.X` fact and the day/night phase.

**Files:**
- Modify: `rust/prax-worlds/src/vampire.rs`
- Test: same `mod tests`

**Interfaces:**
- Produces: the fact convention `vampire.<who>` (a base fact) and a `phase!day`/`phase!night` clock advanced by a period-1 schedule rule; a one-shot `turnPatientZero` schedule rule that inserts `vampire.mara` at the first `phase!night`.
- Consumes: `vampire_world()` from Task 1.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn patient_zero_turns_after_the_first_day() {
    let mut st = vampire_world();
    // No vampire on day one.
    assert!(!fact(&mut st, "vampire.mara"), "no vampire before the first night");
    // Advance the clock through day 1 into the first night.
    advance_to_first_night(&mut st);
    assert!(fact(&mut st, "vampire.mara"), "mara is patient zero after night 1");
    assert_eq!(count_vampires(&mut st), 1, "exactly one vampire at the start");
}
```

(`fact`, `count_vampires`, `advance_to_first_night` are small test helpers defined in `mod tests` — `fact` runs a one-path existence query on the view; `advance_to_first_night` calls the engine's boundary/advance the needed number of times; `count_vampires` counts `vampire.*` via `child_keys`.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p prax-worlds --lib vampire::tests::patient_zero_turns_after_the_first_day`
Expected: FAIL — `vampire.mara` never appears.

- [ ] **Step 3: Write minimal implementation**

Add a period-1 `phase` schedule rule that flips `phase!day`↔`phase!night` (two clauses, or a single rule reading a `dayCount`), and a one-shot rule: `[matches("phase!night"), not_("everBitten"), not_("vampire.mara")] → [insert("vampire.mara"), insert("mark.mara.neck"), insert("everBitten")]`. The `everBitten` guard makes it fire once. Patient zero gets a `mark` too (per design: every vampire is marked).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p prax-worlds --lib vampire::tests::patient_zero_turns_after_the_first_day`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/prax-worlds/src/vampire.rs
git commit -m "vampire: day/night clock + patient zero turns (marked) on the first night"
```

---

### Task 3: The feed action leaves a mark, arms the transformation timer, and arms a feed cooldown

The vampire's core act. Feeding requires a co-located target, hunger, and no active cooldown; it marks the target, records the bite time, sates hunger, and blocks re-feeding for the cooldown window.

**Files:**
- Modify: `rust/prax-worlds/src/vampire.rs`
- Test: same `mod tests`

**Interfaces:**
- Produces: a `prey` practice with the `feed` action; the facts `mark.<victim>.neck`, `bittenOn.<victim>!<turn>` (bite timestamp), `fed.<vampire>` (a cooldown fact armed via `insert_for(COOLDOWN, "fed.<vampire>")`), and hunger sated (`delete("bloodHunger.<vampire>")`).
- Consumes: `vampire.X` (Task 2), the movement `at` facts (Task 1), the `turn!N` clock fact.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn feeding_marks_the_victim_and_locks_the_cooldown() {
    let mut st = seeded_two_at(/*vampire=*/"mara", /*victim=*/"bram", /*place=*/"mill");
    make_hungry(&mut st, "mara");
    let bite = ground_feed(&mut st, "mara", "bram"); // build the grounded feed action
    st.act(&bite); // apply it
    assert!(fact(&mut st, "mark.bram.neck"), "the bite leaves a neck mark");
    assert!(fact_prefix(&mut st, "bittenOn.bram"), "the bite is timestamped");
    assert!(fact(&mut st, "fed.mara"), "feeding arms the cooldown");
    assert!(!fact(&mut st, "bloodHunger.mara"), "feeding sates hunger");
    // A second feed is blocked while the cooldown holds.
    make_hungry(&mut st, "mara");
    assert!(!feed_is_available(&mut st, "mara", "bram"), "cooldown blocks re-feeding");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p prax-worlds --lib vampire::tests::feeding_marks_the_victim_and_locks_the_cooldown`
Expected: FAIL — no `prey` practice / `feed` action.

- [ ] **Step 3: Write minimal implementation**

Add the `prey` practice with:
```
Action::new("[Actor]: feed on [Prey]")
  .when([
    matches("vampire.Actor"),
    matches("bloodHunger.Actor"),
    not_("fed.Actor"),
    matches("practice.world.world.at.Actor!Spot"),
    matches("practice.world.world.at.Prey!Spot"),
    neq("Actor", "Prey"),
    not_("vampire.Prey"),
  ])
  .then([
    insert("mark.Prey.neck"),
    insert_for(TURN_DELAY, "bittenOn.Prey.pending"),  // see Task 4 note on timing
    insert_for(FEED_COOLDOWN, "fed.Actor"),
    delete("bloodHunger.Actor"),
  ])
```
`TURN_DELAY`/`FEED_COOLDOWN` are the 24h windows in the world's turn units (a named const; day/night phase count). The instance/roles wiring follows the village's practice pattern (a practice instance per pair, or a single world-scoped `prey` practice with `Actor`/`Prey` roles — mirror how `whisper`/`world` are instanced in `village.rs`).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p prax-worlds --lib vampire::tests::feeding_marks_the_victim_and_locks_the_cooldown`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/prax-worlds/src/vampire.rs
git commit -m "vampire: feed action — marks the victim, timestamps the bite, arms the cooldown, sates hunger"
```

---

### Task 4: Transformation — a bitten victim turns after the delay

The bite's delayed consequence: `TURN_DELAY` boundaries after a bite, the victim becomes a vampire (and stays marked).

**Files:**
- Modify: `rust/prax-worlds/src/vampire.rs`
- Test: same `mod tests`

**Interfaces:**
- Produces: the transformation rule. Design choice: use a turn-arithmetic axiom — `bittenOn.Prey!T` recorded at feed time (persistent, NOT `insert_for`; revise Task 3's `then` to `insert("bittenOn.Prey!<turn-render>")` via a `call` fn that stamps the current turn, mirroring the village's timestamped-belief pattern) — and the axiom `[matches("bittenOn.V!T"), matches("turn!Now"), calc("E","Now","T","-"), cmp(Gte,"E",TURN_DELAY), not_("vampire.V")] → ["vampire.V"]`.
- Consumes: `bittenOn.V!T`, `turn!Now`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_bitten_victim_turns_after_the_delay() {
    let mut st = seeded_two_at("mara", "bram", "mill");
    make_hungry(&mut st, "mara");
    st.act(&ground_feed(&mut st, "mara", "bram"));
    assert!(!fact(&mut st, "vampire.bram"), "not yet turned right after the bite");
    advance_boundaries(&mut st, TURN_DELAY); // let the clock reach the threshold
    assert!(fact(&mut st, "vampire.bram"), "bram turns after the delay");
    assert!(fact(&mut st, "mark.bram.neck"), "the turned still bears the mark");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p prax-worlds --lib vampire::tests::a_bitten_victim_turns_after_the_delay`
Expected: FAIL — `vampire.bram` never appears.

- [ ] **Step 3: Write minimal implementation**

Change Task 3's feed to stamp `bittenOn.Prey!<currentTurn>` (add a small `Function`/`FnCase` or a `call` that renders the turn into the path, following how the village stamps `atSince.X!<turn>`), then add the transformation axiom above to `set_axioms`. The `not_("vampire.V")` guard makes it fire once and stay (monotone).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p prax-worlds --lib vampire::tests::a_bitten_victim_turns_after_the_delay`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/prax-worlds/src/vampire.rs
git commit -m "vampire: transformation — a bitten victim turns after the delay (turn-arithmetic axiom)"
```

---

### Task 5: Blood-hunger drive — a hungry vampire chooses to feed

Feeding must be *wanted*, not forced. A recurring hunger pulse + a `sate-hunger` want makes a hungry vampire with prey at hand pick `feed`.

**Files:**
- Modify: `rust/prax-worlds/src/vampire.rs`
- Test: same `mod tests`

**Interfaces:**
- Produces: a period-N `hunger` schedule rule (`[matches("vampire.X"), not_("fed.X"), not_("bloodHunger.X")] → [insert("bloodHunger.X")]`) and a `sate-hunger` `Desire` (`Want::new([matches("bloodHunger.Owner")], NEG)` — hunger is a negative state the vampire is driven to end, mirroring `suffers-hunger`), held by every vampire (attach via a want that keys on `vampire.Owner`, so a freshly-turned vampire inherits the drive without re-authoring the roster).
- Consumes: the `feed` action (Task 3), the planner.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_hungry_vampire_with_prey_chooses_to_feed() {
    let mut st = seeded_two_at("mara", "bram", "mill");
    make_hungry(&mut st, "mara");
    let choice = st.pick_action(2, &character("mara"));
    let label = choice.map(|g| g.label).unwrap_or_default();
    assert!(label.contains("feed on bram"), "a hungry vampire feeds; got {label:?}");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p prax-worlds --lib vampire::tests::a_hungry_vampire_with_prey_chooses_to_feed`
Expected: FAIL — no want makes `feed` outrank waiting.

- [ ] **Step 3: Write minimal implementation**

Add the `hunger` schedule rule and the `sate-hunger` desire; attach the desire to vampires via a want-template keyed on `vampire.Owner` (so it applies to any vampire). Tune the utility so feeding (which deletes `bloodHunger`) beats standing still, following the `suffers-hunger` magnitude pattern in `village.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p prax-worlds --lib vampire::tests::a_hungry_vampire_with_prey_chooses_to_feed`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/prax-worlds/src/vampire.rs
git commit -m "vampire: blood-hunger drive — a hungry vampire with prey chooses to feed"
```

---

### Task 6: Endings — all-vampires and no-vampires

The two terminal conditions, as `ending.<name>` facts derived by axioms, so the walk/stress harness detects them.

**Files:**
- Modify: `rust/prax-worlds/src/vampire.rs`
- Test: same `mod tests`

**Interfaces:**
- Produces: `ending.vampires` (`[matches("everBitten"), absent([matches("character.H"), not_("vampire.H"), not_("dead.H")])] → ["ending.vampires"]` — no living human remains) and `ending.village` (`[matches("everBitten"), absent([matches("vampire.V"), not_("dead.V")])] → ["ending.village"]` — no living vampire remains). Both guarded by `everBitten` so neither fires before the outbreak.
- Consumes: `vampire.X`, `dead.X`, `character.X`, `everBitten`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn endings_fire_on_their_conditions() {
    // no-vampires -> village wins
    let mut st = vampire_world();
    force_fact(&mut st, "everBitten");
    // (no vampire.* present)
    reclose(&mut st);
    assert!(fact(&mut st, "ending.village"), "no vampires => village ending");
    assert!(!fact(&mut st, "ending.vampires"), "not simultaneously the vampire ending");

    // all-humans-gone -> vampires win
    let mut st2 = vampire_world();
    force_fact(&mut st2, "everBitten");
    for who in ["aldric","mara","bram","cole","rosa","gideon","nessa","tam"] {
        force_fact(&mut st2, &format!("vampire.{who}"));
    }
    reclose(&mut st2);
    assert!(fact(&mut st2, "ending.vampires"), "all turned => vampire ending");
}
```

(`force_fact`/`reclose` are test helpers that insert a base fact and re-close the view, mirroring how village tests seed and re-derive.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p prax-worlds --lib vampire::tests::endings_fire_on_their_conditions`
Expected: FAIL — no ending facts derived.

- [ ] **Step 3: Write minimal implementation**

Add the two ending axioms to `set_axioms`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p prax-worlds --lib vampire::tests::endings_fire_on_their_conditions`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/prax-worlds/src/vampire.rs
git commit -m "vampire: endings — all-vampires and no-vampires terminal axioms"
```

---

### Task 7: Register the world so it can be run and stressed

Wire `vampire` into the oracle/CLI world registry and the stress harness so it runs under `randtrace`/`stress`.

**Files:**
- Modify: `rust/prax-oracle/src/worlds.rs` (add `"vampire" => Ok(prax_worlds::vampire::vampire_world())` and to `ported()`/`S7_WORLDS` if appropriate), `rust/prax-cli` world dispatch, and any `idler`/player mapping.
- Test: `rust/prax-oracle` (a lib or harness test) — or run manually per the commands below.

**Interfaces:**
- Consumes: `vampire_world()`.
- Produces: `emit vampire --mode randtrace` / `stress vampire` runnability.

- [ ] **Step 1: Write the failing test**

```rust
// in prax-oracle worlds tests
#[test]
fn vampire_world_is_registered_and_runs_a_walk() {
    let mut st = crate::worlds::build("vampire").expect("vampire world builds");
    // a short random walk produces records without panicking
    let recs = crate::drive_rust::rand_walk(&mut st, /*emit*/Default::default(), /*mode*/Default::default(), 30, 0);
    assert!(!recs.is_empty(), "the vampire world produces a walk");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p prax-oracle vampire_world_is_registered`
Expected: FAIL — `build("vampire")` errors (unknown world).

- [ ] **Step 3: Write minimal implementation**

Add the `vampire` arm to `worlds::build` and the CLI dispatch. Confirm the exact `rand_walk` signature/`Emit`/`Mode` defaults from `drive_rust.rs` and match them (adjust the test call to the real signature).

- [ ] **Step 4: Run test to verify it passes + smoke-run**

Run: `cargo test -p prax-oracle vampire_world_is_registered`
Then: `cargo run --release -p prax-oracle -- emit vampire --mode randtrace --seed 0 --cap 60 | tail -3`
Expected: PASS; the emit prints records and a terminal `{"end":true,...}`.

- [ ] **Step 5: Commit**

```bash
git add rust/prax-oracle/src/worlds.rs rust/prax-cli/src/main.rs
git commit -m "vampire: register the world in the oracle/CLI + stress harness"
```

---

### Task 8: End-to-end — the infection spreads to an ending

The skeleton's acceptance test: over a stress sweep the infection actually runs to completion, and at least the vampire ending is reachable (with only feeding+turning, humans can't fight back yet, so vampires should generally win — the point is the *loop closes*).

**Files:**
- Modify: `rust/prax-worlds/src/vampire.rs` (an end-to-end `#[test]`) and/or a stress assertion.
- Test: same `mod tests`

**Interfaces:**
- Consumes: everything above; the `stress_test`/walk harness.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn the_infection_runs_to_an_ending() {
    // A deterministic-enough seeded walk reaches a terminal ending fact.
    let mut st = vampire_world();
    let reached = run_until_ending_or_cap(&mut st, /*cap*/400);
    assert!(
        reached == Some("ending.vampires") || reached == Some("ending.village"),
        "the skeleton loop must reach an ending; got {reached:?}"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p prax-worlds --lib vampire::tests::the_infection_runs_to_an_ending`
Expected: FAIL — likely no ending within cap (mechanics not yet balanced: hunger cadence vs cooldown vs cast size).

- [ ] **Step 3: Balance to close the loop**

Tune the constants — `TURN_DELAY`, `FEED_COOLDOWN`, the hunger period, the cast's starting co-location — so the infection can actually propagate through the 8-person cast within the cap. Author `run_until_ending_or_cap` to advance the engine and check for any `ending.*` after each boundary.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p prax-worlds --lib vampire::tests::the_infection_runs_to_an_ending`
Then a spread check: `cargo run --release -p prax-oracle -- stress vampire | tail -5`
Expected: PASS; stress reports endings across seeds (at this stage, dominated by `ending.vampires` — humans get their tools in the next plan).

- [ ] **Step 5: Commit**

```bash
git add rust/prax-worlds/src/vampire.rs
git commit -m "vampire: skeleton acceptance — the infection runs to an ending; constants balanced"
```

---

## Self-Review

**Spec coverage (skeleton subset):** Cast/movement substrate (T1) ✓; patient-zero turn incl. the mark (T2) ✓; feed→mark→timestamp→cooldown→sate (T3) ✓; transformation after delay (T4) ✓; blood-hunger drive making feeding *emergent* not forced (T5) ✓; both endings (T6) ✓; registration for run/stress (T7) ✓; the loop closing to an ending (T8) ✓. **Deferred to later plans (out of this skeleton, per the spec's phasing):** scarf/mark-exposure, witnessing→gossip→suspicion, disguise + the disreputable indulgence, accuse/kill/priest-cure, the survival-driven *hiding*, scaling to ~30, mass-run mining. These are the *next* plan (detection) and the one after (elimination + disguise).

**Placeholder scan:** The task steps commit to concrete facts/axioms/actions. Two honest in-build resolutions are flagged, not hidden: (a) the exact `State` test-accessor for a fact existence (`fact`/`exists_str`) must be matched to whatever `village.rs`'s tests use — the asserted *behaviour* is fixed; (b) the exact `rand_walk`/`Emit`/`Mode`/`act`/`pick_action` signatures must be read from the engine and matched. These are signature-matching, not design placeholders.

**Type consistency:** Fact conventions are consistent across tasks — `vampire.X`, `mark.X.neck`, `bittenOn.X!<turn>`, `fed.X`, `bloodHunger.X`, `everBitten`, `ending.vampires`, `ending.village`, `phase!day`/`phase!night`. `TURN_DELAY`/`FEED_COOLDOWN` are named consts used identically in T3/T4/T8.

**Risk carried forward:** the skeleton deliberately has no human counterplay, so it tests the *infection* half of the loop only; the detection/elimination plan is what makes both endings genuinely contested. The perf number from T7/T8's runs is the input to the Phase-2 scale decision.
