# v27 — Incremental View Maintenance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop re-closing the world for deltas that provably cannot change what the axioms derive, per `docs/specs/2026-07-11-v27-incremental-view.md`, with bit-for-bit identical decisions AND bit-for-bit identical views.

**Architecture:** (1) An invariant net FIRST — per-turn `readView == recomputed closure` over real drives, plus a proof the checker catches a deliberately-doctored view — this is the round's core and lands before any second construction path exists. (2) `axiomFootprint` in `Prax.Derive` (every pattern the axioms read or write, any polarity, lifted forms included), carried as a `PraxState` field. (3) The `performOutcome` fast path: a footprint-irrelevant delta is applied to `db` and `readView` in lockstep, skipping `reclose`. (4) Measurement + the Phase-2 (truth maintenance) gate decision returns to the controller.

**Tech Stack:** Haskell (GHC 9.10, cabal), tasty/tasty-hunit (existing deps only).

## Global Constraints

- **Exactness**: goldens (GoldenDriveSpec) byte-identical after every task; the new invariant net green after every task. If either fails, the change is WRONG — BLOCK with the trace; never adjust an expected value.
- The commutation theorem's conservativity: `relevantDelta` may answer False (fast path) only when provable; anything uncertain recloses. An unsound fast-path classification is a Critical defect.
- `cabal build all` zero warnings; `hlint src app test` "No hints"; suite green (303 baseline, ~110s).
- The v26 grep-gates stay empty (db/axioms/desires record updates outside Engine/Types).
- No heuristics/magic numbers.
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: The invariant net (before any second construction path exists)

**Files:**
- Create: `test/Prax/ViewInvariantSpec.hs`
- Modify: `prax.cabal` (test `other-modules`, after `Prax.GoldenDriveSpec`), `test/Spec.hs` (import + register)

**Interfaces:**
- Consumes: `closure` (`Prax.Derive`), `advance`/`npcAct` (`Prax.Loop`), the three golden worlds.
- Produces: the checker every later task must keep green.

- [ ] **Step 1: Write the spec file:**

```haskell
module Prax.ViewInvariantSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (dbToLabeledSentences, insert)
import           Prax.Derive (closure)
import           Prax.Types
import           Prax.Engine (performOutcome)
import           Prax.Loop (advance, npcAct)
import           Prax.Worlds.Village (villageWorld, playerName)
import           Prax.Worlds.Bar (barWorld)
import           Prax.Worlds.Intrigue (intrigueWorld)

-- The round's core invariant: the cached view IS the closure of the base
-- under the axioms — label-faithfully, whatever construction path built it.
viewConsistent :: PraxState -> Bool
viewConsistent st =
  dbToLabeledSentences (readView st) == dbToLabeledSentences recomputed
  where
    recomputed = case closure (axioms st) (db st) of
      Right c -> c
      Left _  -> insert "contradiction" (db st)

-- Drive n turns through the REAL loop; report the first turn (1-based) after
-- which the invariant fails, if any. The Maybe Int in the assertion message
-- names the offending turn directly.
firstDrift :: Maybe String -> Int -> PraxState -> Maybe Int
firstDrift idle n st0 = go 1 st0
  where
    go k st
      | k > n = Nothing
      | otherwise =
          let (actor, st1) = advance st
              st2 | Just (charName actor) == idle = st1
                  | otherwise                     = snd (npcAct 2 actor st1)
          in if viewConsistent st2 then go (k + 1) st2 else Just k

tests :: TestTree
tests = testGroup "Prax.ViewInvariant (readView == recomputed closure)"
  [ testCase "the checker catches a deliberately doctored stale view" $ do
      -- a raw record update that bypasses the Engine helpers — exactly the
      -- construction the src/ grep-gate bans — leaves the cached view behind
      let st       = performOutcome (Insert "probe.fact") villageWorld
          doctored = st { readView = readView villageWorld }
      assertBool "a helper-built state passes" (viewConsistent st)
      assertBool "the stale view is caught"    (not (viewConsistent doctored))

  , testCase "village: 3 rounds of free play, invariant after every turn" $
      firstDrift (Just playerName) 21 villageWorld @?= Nothing

  , testCase "bar: 12 turns, invariant after every turn" $
      firstDrift Nothing 12 barWorld @?= Nothing

  , testCase "intrigue: 12 turns, invariant after every turn" $
      firstDrift Nothing 12 intrigueWorld @?= Nothing
  ]
```

- [ ] **Step 2: Register; run the group; observe GREEN** (today there is only one construction path, so the drive tests pass trivially — the doctored-view test is what proves the checker can fail):

Run: `cabal test --test-options='-p "ViewInvariant"' 2>&1 | tail -5`
Expected: 4 tests pass. The doctored-view test's RED-capability is internal to it (it asserts the checker returns False on constructed staleness — observe that assertion is genuinely exercised by temporarily flipping it if in any doubt, then restoring).

- [ ] **Step 3: Full suite once** (expect 307 green), zero warnings, hlint clean.

- [ ] **Step 4: Commit.**

```bash
git add test/Prax/ViewInvariantSpec.hs prax.cabal test/Spec.hs
git commit -m "ViewInvariantSpec: readView must equal a from-scratch closure, every turn"
```
(with the standard trailers)

---

### Task 2: `axiomFootprint` + the `footprint` field + delta classification

**Files:**
- Modify: `src/Prax/Derive.hs` (add `condPatterns`, `axiomFootprint`; export the latter)
- Modify: `src/Prax/Types.hs` (field `footprint :: [String]`; `emptyState` gets `footprint = []`)
- Modify: `src/Prax/Engine.hs` (`retable` also sets `footprint`; new exported `relevantDelta`)
- Modify: `src/Prax/Relevance.hs` (export `evictionShadows`)
- Test: `test/Prax/DeriveSpec.hs` (footprint unit test), `test/Prax/RelevanceSpec.hs` (village classification test)

**Interfaces:**
- Consumes: `mayUnify`/`evictionShadows` (`Prax.Relevance`), `liftObliged` (already in Derive).
- Produces (Task 3 relies on):
  - `axiomFootprint :: [Axiom] -> [String]`
  - `PraxState.footprint :: [String]` (maintained by `retable`, i.e. by `definePractices`/`setAxioms`/`setDesires`)
  - `relevantDelta :: String -> PraxState -> Bool`

- [ ] **Step 1: Write the failing tests.** In `test/Prax/DeriveSpec.hs` (follow its existing imports/style):

```haskell
  , testCase "axiomFootprint collects bodies (any polarity), heads, and lifted forms" $ do
      let ax = axiom [ Match "parent.X.Y", Absent [ Match "dead.X" ] ] [ "elder.X" ]
          fp = axiomFootprint [ax]
      assertBool "body atom"          ("parent.X.Y" `elem` fp)
      assertBool "negated body atom"  ("dead.X" `elem` fp)
      assertBool "head"               ("elder.X" `elem` fp)
      -- an Absent body blocks □-lifting; an all-Match rule contributes both
      -- lifted body and lifted head:
      let fp2 = axiomFootprint [ axiom [ Match "a.X" ] [ "b.X" ] ]
      assertBool "lifted body" ("obliged.Obligor.a.X" `elem` fp2)
      assertBool "lifted head" ("obliged.Obligor.b.X" `elem` fp2)
```

In `test/Prax/RelevanceSpec.hs` (it already imports `Prax.Engine` and `villageWorld`; add `relevantDelta` to the Engine import):

```haskell
  , testCase "delta relevance against the village's axioms" $ do
      assertBool "movement commutes with closure (fast path)"
        (not (relevantDelta "practice.world.world.at.bob!square" villageWorld))
      assertBool "a witness deposit is relevant (standingUnless reads believes)"
        (relevantDelta "you.believes.stole.bob.loaf.seen" villageWorld)
      assertBool "an atonement is relevant (it defeats standing)"
        (relevantDelta "atoned.bob" villageWorld)
      assertBool "the stall's stock is not"
        (not (relevantDelta "stall.loaf" villageWorld))
```

- [ ] **Step 2: Observe RED** (compile failures: `axiomFootprint`, `relevantDelta` undefined).

- [ ] **Step 3: Implement.**

**(a) `src/Prax/Derive.hs`** (export `axiomFootprint`):

```haskell
-- | Every path pattern the axioms can read or write: body atoms at any
-- polarity (including inside Absent\/Exists\/Or\/Subquery), head templates,
-- and the □-lifted forms of both. A ground delta that may-unify none of
-- these commutes with 'closure' (v27 spec theorem) — the basis of the
-- engine's delta-irrelevance fast path.
axiomFootprint :: [Axiom] -> [String]
axiomFootprint axs =
  concat [ concatMap condPatterns body ++ hs
         | Axiom body hs <- axs ++ mapMaybe liftObliged axs ]

-- All path patterns a condition mentions, any polarity.
condPatterns :: Condition -> [String]
condPatterns c = case c of
  Match s        -> [s]
  Not s          -> [s]
  Absent cs      -> concatMap condPatterns cs
  Exists cs      -> concatMap condPatterns cs
  Or clauses     -> concatMap (concatMap condPatterns) clauses
  Subquery _ _ w -> concatMap condPatterns w
  Eq {}          -> []
  Neq {}         -> []
  Cmp {}         -> []
  Calc {}        -> []
  Count {}       -> []
```

**(b) `src/Prax/Types.hs`**: add the field after `improvables`:

```haskell
  , footprint :: [String]
    -- ^ Every pattern the axioms read or write ('Prax.Derive.axiomFootprint')
    -- — rebuilt with the vocabulary; a ground delta unifying none of it
    -- commutes with closure and may skip 'reclose' (the engine's fast path).
```

`emptyState`: `, footprint = []`.

**(c) `src/Prax/Engine.hs`**: extend `retable`:

```haskell
retable :: PraxState -> PraxState
retable st = st
  { improvables = improvableDesires (practiceDefs st) (axioms st) (desires st)
  , footprint   = axiomFootprint (axioms st) }
```

and add + export:

```haskell
-- | Can this ground delta change what the axioms derive? Conservative:
-- False only when the sentence — and anything its exclusions evict —
-- may-unify nothing in the axioms' footprint (v27 spec theorem). False is
-- the licence for 'performOutcome' to skip 'reclose'.
relevantDelta :: String -> PraxState -> Bool
relevantDelta s st =
  any (\x -> any (mayUnify x) (footprint st)) (s : evictionShadows s)
```

**(d) `src/Prax/Relevance.hs`**: add `evictionShadows` to the export list (its haddock already states its meaning).

- [ ] **Step 4: Observe GREEN**: the two test patterns, then the full suite once (309 expected), zero warnings, hlint clean, goldens + ViewInvariant green.

- [ ] **Step 5: Commit.**

```bash
git add src/Prax/Derive.hs src/Prax/Types.hs src/Prax/Engine.hs src/Prax/Relevance.hs \
        test/Prax/DeriveSpec.hs test/Prax/RelevanceSpec.hs
git commit -m "axiomFootprint + relevantDelta: what the axioms can see, per world"
```
(with the standard trailers)

---

### Task 3: The fast path

**Files:**
- Modify: `src/Prax/Engine.hs` only.

**Interfaces:**
- Consumes: Task 2's `relevantDelta`; Task 1's net is the correctness proof.

- [ ] **Step 1: Implement** (no new observable behavior exists to RED-test — the invariant net and goldens, proven capable of failing in Task 1, ARE this task's tests; state that in the report):

```haskell
-- | Apply one delta to the base AND the cached view in lockstep — sound
-- exactly when 'relevantDelta' answered False (the delta commutes with
-- closure; v27 spec theorem). The only sanctioned 'readView' write outside
-- 'reclose'.
applyDirect :: (Db -> Db) -> PraxState -> PraxState
applyDirect f st = st { db = f (db st), readView = f (readView st) }
```

and reroute the two primitive cases of `performOutcome`:

```haskell
performOutcome (Delete s) st
  | relevantDelta s st = withDb (retract s) st
  | otherwise          = applyDirect (retract s) st
performOutcome (Insert s) st =
  let st' | relevantDelta s st = withDb (insert s) st
          | otherwise          = applyDirect (insert s) st
  in case spawnedInstance s st of
       Just (def, roleVals) ->
         let roleBindings = Map.fromList (zip (roles def) (map VStr roleVals))
         in foldl' (\s2 o -> performOutcome (groundOutcome o roleBindings) s2)
                   st' (initOutcomes def)
       Nothing -> st'
```

(`Call`/`ForEach` recurse into these primitives and inherit the split; nothing else changes.)

- [ ] **Step 2: The proof run.** `cabal test --test-options='-p "ViewInvariant"'` (the net now guards a REAL second path), then `-p "GoldenDrive"`, then the full suite ONCE — all green, and record the suite wall time (expect a substantial drop; the headline).

- [ ] **Step 3: Gates**: zero warnings; hlint; the grep-gates still empty.

- [ ] **Step 4: Commit.**

```bash
git add src/Prax/Engine.hs
git commit -m "performOutcome: deltas the axioms cannot see skip the reclose"
```
(with the standard trailers)

---

### Task 4: Measure; decide Phase 2 with evidence

**Files:** none in the repo (scratchpad + transient `cabal.project.local`, deleted before finishing).

- [ ] **Step 1:** Re-profile the one-round drive (the scratchpad's `V26Prof.hs`; profiling flow as v26 Task 4: `cabal.project.local` with `profiling: True`/`profiling-detail: all-functions`, build, `ghc -O1 -prof -fprof-auto -package prax`, run `+RTS -p`).
- [ ] **Step 2:** Record: total time vs 2.83s; `reclose` entries vs 11,840 (the fast-path hit rate is `1 − reclose/oldReclose` adjusted for turn counts — report the raw entry counts and let the controller compute); `applyDirect` entries; top cost centres.
- [ ] **Step 3:** `rm cabal.project.local`; rebuild vanilla; `git status` clean.
- [ ] **Step 4:** Report and STOP — no commit. The controller decides Phase 2 (DRed truth maintenance) per the spec's criterion: closure still the top cost centre, or not.

---

### Task 5: Docs

**Files:**
- Modify: `docs/LEDGER.md`, `README.md` (if warranted).

- [ ] **Step 1:** Measure once: full suite time + Village group time (post-Task-3).
- [ ] **Step 2:** LEDGER: v27 legend row (the invariant net as the round's core, the commutation theorem, the fast-path hit rate and times — measured figures only); resolve/update the **Incremental view maintenance (#17)** item per the Task 4 decision (Phase 2 shipped / warranted-and-planned / recorded-as-not-warranted).
- [ ] **Step 3:** Full gate (warnings, hlint, `prax check` ×7, grep-gates) recorded; commit:

```bash
git add docs/LEDGER.md README.md
git commit -m "Docs: v27 — the view keeps itself, provably"
```
(with the standard trailers)
