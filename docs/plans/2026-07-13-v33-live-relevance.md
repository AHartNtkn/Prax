# v33 — State-Conditioned Relevance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The pair-skip learns "could it matter NOW?", per `docs/specs/2026-07-13-v33-live-relevance.md` — bit-for-bit identical decisions, the v32 A/B recovered.

**Architecture:** Tier 1 (the floor check) needs almost no new state — `cookedDesires` (v28) already carries each desire's Owner-template cooked conditions; polarity comes from `desireWant`. Tier 2 adds one static analysis (environment-gated conjuncts) to `Prax.Relevance`, carried in a retable-maintained field. `predictMove` gains `deadNow`. The nets prove exactness; the 31-test A/B proves the recovery.

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- Exact only: goldens byte-identical, ViewInvariant green, suite green (431 baseline @ ~235s — should DROP substantially by Task 2's end) after every task. Net failure = BLOCK with the trace.
- The skip is pair-level only: if ANY believed desire is live, the FULL model is evaluated. Never drop individual desires from an evaluated model.
- All conservativity one-directional: uncertainty ⇒ live. An unsound skip is Critical.
- Zero warnings; hlint "No hints"; `prax check` ×7; grep-gates.
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: The static side — polarity + environment gates

**Files:** `src/Prax/Relevance.hs`, `src/Prax/Types.hs`, `src/Prax/Engine.hs`, `test/Prax/RelevanceSpec.hs`.

**Design:**
- `Prax.Relevance` gains and exports:

```haskell
-- | Per-desire state-check recipe for the planner's dead-now test (spec
-- @docs/specs/2026-07-13-v33-live-relevance.md@). 'FloorCheck': a negative
-- want-kind is at its floor (unimprovable by anything) when its own
-- Owner-grounded conditions have zero bindings — sound unconditionally,
-- count zero is the minimum. 'GateCheck': a positive want-kind cannot gain
-- a binding while any environment-gated conjunct (a base-fact family no
-- authored outcome inserts and no axiom derives) is empty. 'AlwaysLive':
-- no cheap state test applies — the static verdict stands alone.
data Liveness
  = FloorCheck                  -- negative: check the desire's own conditions
  | GateCheck [[CookedCondition]] -- positive: each inner list is ONE gate
                                  -- conjunct (cooked, Owner-templated)
  | AlwaysLive
  deriving (Eq, Show)

livenessOf :: Map String Practice -> [Axiom] -> [Desire] -> Map String Liveness
```

  Rules for `livenessOf` (transcribe `improvableDesires`' existing internals — the atom
  pools, `derivable`, `wantPatterns` — do not re-derive them):
  - weight < 0 ⇒ `FloorCheck` (unconditional soundness; no further analysis needed).
  - weight > 0 ⇒ collect its top-level positive `Match` conjuncts whose pattern
    (a) may-unify NO Insert atom in the world's pool (including `!`-insert assert sides),
    (b) may-unify NO axiom head (the existing `derivable` check, lifted forms included),
    and (c) is not `unc`-tainted (the want has no Subquery/Count/Calc — reuse the existing
    taint from `wantPatterns`; a tainted want is `AlwaysLive`). Each qualifying conjunct,
    cooked (`cookCondition`), becomes one gate. No qualifying conjuncts ⇒ `AlwaysLive`.
  - weight == 0 is unreachable here (statically never-improvable already) — map it
    `AlwaysLive` defensively; the static filter screens it first.
  - Conservativity note in the haddock: gates only ever REMOVE work when provably safe;
    `Or`/`Absent`/`Exists` conjuncts are never gates (only top-level positive `Match`es).
- `PraxState` gains `liveness :: Map String Liveness` (haddock: the dead-now recipes,
  maintained with the vocabulary by `retable` like `improvables`); `emptyState` gets
  `Map.empty`; `retable` in Engine adds
  `liveness = livenessOf (practiceDefs st) (axioms st) (desires st)`.

**RelevanceSpec additions (RED-first):**
- `livenessOf` classification: a negative desire ⇒ FloorCheck; a positive desire with a
  ticker-only conjunct (fixture: `Want [Match "hungry.Owner", Match "meal.M"] 5` in a world
  whose only actions insert `meal.*`, never `hungry.*`) ⇒ GateCheck on the hungry conjunct
  only; a positive desire whose candidate gate is axiom-derivable (fixture axiom head
  unifying it) ⇒ AlwaysLive (the conservative case); a Subquery-bearing want ⇒ AlwaysLive.
- The field: `liveness villageWorld` maps `clean-conscience`/`conscience-remembers` to
  FloorCheck and `pursues-earnBread`/`spites-carol`/`punishes-whisper` to their expected
  classes (derive the expectations by reading the village vocabulary, assert exactly).

- [ ] RED (names missing) → implement → GREEN (`-p "Relevance"`, then suite once, count
  reported) → gates → commit `"Relevance: every want-kind gets a dead-now recipe"`.

---

### Task 2: The planner asks "now?" — and the A/B answers

**Files:** `src/Prax/Planner.hs`, `test/Prax/PlannerSpec.hs` (or RelevanceSpec — follow
where predictMove behavior is pinned today), plus the measurement (scratchpad only).

**Design:** `predictMove`'s skip line (Planner.hs:89) becomes: a believed desire is DEAD
when statically dead (`notElem improvables` — unchanged) OR dead-now:

```haskell
deadNow :: PraxState -> Character -> Desire -> Bool
deadNow st m d = case Map.lookup (desireName d) (liveness st) of
  Just FloorCheck    -> null (queryCooked v conds owner)
    where conds = Map.findWithDefault [] (desireName d) (cookedDesires st)
  Just (GateCheck gs) -> any (\g -> null (queryCooked v g owner)) gs
  _                   -> False
  where
    v     = readView st
    owner = Map.singleton (intern "Owner") (VSym (intern (charName m)))
```

(Adjust binding-construction to the actual Sym API — check how `cookedSelfWants`/
`evaluateCooked` ground Owner and mirror it exactly; a desire with FloorCheck but no
`cookedDesires` entry must count LIVE, not dead — empty conds would query-succeed with one
binding, verify `queryCooked v [] owner` returns non-null and add a guard comment either
way.) The pair skips when ALL believed desires are (statically dead ∨ dead-now). Mixed
models still evaluate in full.

**Tests (RED-first — both directions, the spec's demand):**
- The floor case: in a fixture (or the village), a predictor with a believed conscience-only
  model of a MARKLESS mover ⇒ `predictMove` returns Nothing without enumerating (assert via
  the skip's observable: it must equal the pre-check result — behaviorally assert Nothing;
  the exactness nets carry the rest); give the mover ONE lied-mark ⇒ the pair goes LIVE
  (assert the full-model path runs — e.g. the predicted move matches the unfiltered
  expectation; construct a case where a confess-shaped move IS the prediction so liveness
  has an observable).
- The gate case: hunger fixture — believed `wants-food` model with `hungry.<mover>` absent
  ⇒ Nothing (skipped); insert the hunger fact ⇒ the eat move is predicted.
- The conservative case: the axiom-derivable gate desire still predicts as before.

**The measurement (the round's acceptance):** uncontended, best-of-3, the SAME 31
pre-existing village tests filter from the v32 A/B (task-3 report has the exact filter):
report HEAD-after vs the recorded 171.64s (v32) and 31.11s (pre-v32) numbers. Then the full
suite time. Report all numbers as measured; the target is recovery TOWARD 31s, stated
honestly wherever it lands.

- [ ] RED → implement → GREEN → nets (`-p "GoldenDrive"`, `-p "ViewInvariant"`) → the A/B →
  suite once → gates → commit `"predictMove asks could-it-matter-now; the village answers in seconds"`.

---

### Task 3: Docs

**Files:** `docs/LEDGER.md`, `README.md` (if warranted).

- [ ] LEDGER: v33 legend row (the two checks, the soundness arguments, the measured
  recovery — all three A/B epochs: 31.11s / 171.64s / now); the v32 perf note gains its
  resolution; the v26 relevance row gains the state dimension note. WALKTHROUGH only if a
  claim there cited the slow numbers (sweep the v32 section's perf sentence).
- [ ] Full gate recorded; commit `"Docs: v33 — relevance, live"`.
