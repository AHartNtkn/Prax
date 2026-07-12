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

---

### Task 4b: Phase 2 as measured — the monotone-insert continuation + tokenized classification

The Task 4 re-profile (spec §Phase 2, amended in-round): fast path halved closures
(11,840 → ~5,740/round) but the relevant residue — witness/belief deposits — still costs
~40%, and `relevantDelta`'s own string re-parsing ~8–10%. Two exact tiers land here.

**Files:**
- Modify: `src/Prax/Derive.hs` (share `closure`'s loop as `closureFrom`; `axiomNegPatterns`; `monotoneAxioms`)
- Modify: `src/Prax/Types.hs` (footprint fields become pre-tokenized; new `negFootprint`, `contMonotone`)
- Modify: `src/Prax/Engine.hs` (`retable`; `relevantDelta` on tokens; `monotoneInsert`; `applyGrow`; the three-way `performOutcome` branch)
- Modify: `src/Prax/Relevance.hs` (export `mayUnifyNames`; `mayUnify` defined through it)
- Test: `test/Prax/DeriveSpec.hs`, `test/Prax/RelevanceSpec.hs`

**Interfaces produced:**
- `Prax.Derive.closureFrom :: [Axiom] -> Db -> [String] -> Either Contradiction Db` — continue an already-closed model with new base facts (caller guarantees monotonicity).
- `Prax.Derive.axiomNegPatterns :: [Axiom] -> [String]` — every pattern under an `Absent`/`Not` in any body.
- `Prax.Derive.monotoneAxioms :: [Axiom] -> Bool` — whether the world's axiom set is continuation-safe.
- `Prax.Relevance.mayUnifyNames :: [String] -> [String] -> Bool`.
- `PraxState.footprint :: [[String]]`, `negFootprint :: [[String]]`, `contMonotone :: Bool`.

- [ ] **Step 1: Write the failing tests.**

`test/Prax/DeriveSpec.hs` additions:

```haskell
  , testCase "closureFrom continues a closed model exactly as a from-scratch closure" $ do
      let axs = [ axiom [ Match "parent.X.Y" ] [ "elder.X" ]
                , axiom [ Match "elder.X", Match "wise.X" ] [ "sage.X" ] ]
          base = insertAll [ "parent.ada.bea", "wise.ada" ] emptyDb
          Right closed = closure axs base
          -- a monotone new fact cascading through BOTH rules:
          Right cont   = closureFrom axs closed [ "parent.cal.dun" ]
          Right scratch = closure axs (insert "parent.cal.dun" base)
      dbToLabeledSentences cont @?= dbToLabeledSentences scratch

  , testCase "axiomNegPatterns collects exactly the negated interiors" $ do
      let axs = [ axiom [ Match "a.X", Absent [ Match "b.X", Not "c.X" ] ] [ "d.X" ] ]
      assertBool "Absent interior" ("b.X" `elem` axiomNegPatterns axs)
      assertBool "Not inside Absent" ("c.X" `elem` axiomNegPatterns axs)
      assertBool "positive atom is NOT negated" ("a.X" `notElem` axiomNegPatterns axs)

  , testCase "monotoneAxioms accepts the count-threshold shape and rejects anti-monotone" $ do
      assertBool "Match-only is safe" (monotoneAxioms [ axiom [ Match "a.X" ] [ "b.X" ] ])
      assertBool "the notoriety shape (Subquery+Count+Cmp Gte literal) is safe"
        (monotoneAxioms [ axiom [ Subquery "Rs" ["W"] [ Match "r.W.T" ]
                                , Count "N" "Rs", Cmp Gte "N" "3" ] [ "n.T" ] ])
      assertBool "Cmp Lt with the literal on the right is anti-monotone"
        (not (monotoneAxioms [ axiom [ Count "N" "Rs", Cmp Lt "N" "3" ] [ "q.T" ] ]))
      assertBool "Calc disables the tier"
        (not (monotoneAxioms [ axiom [ Calc "M" Add "N" "1" ] [ "q.M" ] ]))
```

(imports: `Subquery`/`Count`/`Cmp`/`Calc`/`CmpOp (..)`/`CalcOp (..)` from `Prax.Query` as needed;
`dbToLabeledSentences`/`insert`/`insertAll`/`emptyDb` from `Prax.Db`. If the incomplete-pattern
`Right` binds trip `-Wall`, use explicit `case`/`either (error . show) id` instead.)

`test/Prax/RelevanceSpec.hs` addition (import `monotoneInsert` from `Prax.Engine`):

```haskell
  , testCase "monotone-insert classification against the village" $ do
      assertBool "the village's axioms admit the continuation tier"
        (contMonotone villageWorld)
      assertBool "a witness deposit grows monotonically"
        (monotoneInsert "you.believes.stole.bob.loaf.seen" villageWorld)
      assertBool "atonement defeats standing: full reclose"
        (not (monotoneInsert "atoned.bob" villageWorld))
      assertBool "an exclusion insert never takes the continuation"
        (not (monotoneInsert "practice.world.world.at.bob!square" villageWorld))
```

- [ ] **Step 2: Observe RED** (compile failures for the new names).

- [ ] **Step 3: Implement.**

**(a) `src/Prax/Derive.hs`** — extract `closure`'s loop so both entry points share it verbatim,
and add the analyses (export `closureFrom`, `axiomNegPatterns`, `monotoneAxioms`):

```haskell
closure :: [Axiom] -> Db -> Either Contradiction Db
closure []  db0 = Right db0
closure axs db0 = run axs db0 db0

-- | Continue an ALREADY-CLOSED model with new base facts. Exactly
-- 'closure'’s semi-naive loop, entered at (model ∪ facts, delta = facts).
-- Sound only when the facts are monotone for these axioms — '!'-free and
-- unifying no negated body pattern, with 'monotoneAxioms' true — which is
-- the CALLER's obligation ('Prax.Engine.monotoneInsert'); a violation is
-- caught by the ViewInvariant net, not silently absorbed.
closureFrom :: [Axiom] -> Db -> [String] -> Either Contradiction Db
closureFrom axs closed facts =
  run axs (insertAll facts closed) (insertAll facts emptyDb)

-- The shared semi-naive engine (the former closure-local 'go', verbatim,
-- with 'rules' computed from the axiom list).
run :: [Axiom] -> Db -> Db -> Either Contradiction Db
run axs = go
  where
    rules = [ (body, map tokens hs)
            | Axiom body hs <- axs ++ mapMaybe liftObliged axs ]
    go model delta = <the existing loop body, unchanged>
```

(move the existing `go`/`meetOne`/`entailed`/`deltaJoin` bodies under `run` untouched.)

```haskell
-- | Every pattern under a negation in any body: inserting a fact these
-- patterns match can UN-fire a rule (retraction), so such facts never take
-- the continuation tier.
axiomNegPatterns :: [Axiom] -> [String]
axiomNegPatterns axs = concat
  [ concatMap negOf body | Axiom body _ <- axs ++ mapMaybe liftObliged axs ]
  where
    negOf c = case c of
      Not s          -> [s]
      Absent cs      -> concatMap condPatterns cs   -- everything inside a ¬∃
      Exists cs      -> concatMap negOf cs
      Or clauses     -> concatMap (concatMap negOf) clauses
      Subquery _ _ w -> concatMap negOf w
      _              -> []

-- | Is the axiom set continuation-safe: does adding base facts only ever ADD
-- derived facts (given the caller also avoids negated patterns)? Conditions
-- must be monotone-up: Match/Eq/Neq/Not/Absent (negations are handled via
-- 'axiomNegPatterns'), recursion through Exists/Or/Subquery, Count freely,
-- and Cmp only in the grows-only direction — the count side growing past a
-- numeric literal (Gt/Gte with the literal right, Lt/Lte with it left).
-- Calc (and any other Cmp shape) disables the tier for the world; the
-- fallback is today's full reclose, correct just slower.
monotoneAxioms :: [Axiom] -> Bool
monotoneAxioms axs =
  all bodyOk [ body | Axiom body _ <- axs ++ mapMaybe liftObliged axs ]
  where
    bodyOk = all condOk
    condOk c = case c of
      Match _        -> True
      Not _          -> True
      Absent _       -> True
      Eq _ _         -> True
      Neq _ _        -> True
      Count _ _      -> True
      Exists cs      -> bodyOk cs
      Or clauses     -> all bodyOk clauses
      Subquery _ _ w -> bodyOk w
      Cmp op l r     -> case op of
        Gt  -> numeric r
        Gte -> numeric r
        Lt  -> numeric l
        Lte -> numeric l
        _   -> False
      Calc {}        -> False
    numeric x = not (null x) && all (`elem` ("0123456789" :: String)) x
```

(check `CmpOp`'s constructors in `Prax.Query` — cover any beyond Gt/Gte/Lt/Lte, e.g. an
equality op, as `False`.)

**(b) `src/Prax/Relevance.hs`** — split `mayUnify`:

```haskell
mayUnify :: String -> String -> Bool
mayUnify a b = mayUnifyNames (pathNames a) (pathNames b)

-- | 'mayUnify' on pre-split paths — the planner-hot form ('relevantDelta'
-- classifies every primitive delta against every footprint pattern).
mayUnifyNames :: [String] -> [String] -> Bool
mayUnifyNames as bs = anchored && and (zipWith seg as bs)
  where
    seg x y = isVariable x || isVariable y || x == y
    anchored = or (zipWith literalMatch as bs)
    literalMatch x y = not (isVariable x) && not (isVariable y) && x == y
```

(export `mayUnifyNames`.)

**(c) `src/Prax/Types.hs`** — the classification fields become:

```haskell
  , footprint :: [[String]]
    -- ^ Pre-tokenized ('pathNames') patterns the axioms read or write; a
    -- ground delta unifying none of them commutes with closure (fast path).
  , negFootprint :: [[String]]
    -- ^ Pre-tokenized negated body interiors: a '!'-free insert unifying
    -- none of these (in a 'contMonotone' world) only ADDS derived facts and
    -- takes the continuation tier.
  , contMonotone :: Bool
    -- ^ 'Prax.Derive.monotoneAxioms' of this world's axioms.
```

(`emptyState`: `[]`/`[]`/`True`.)

**(d) `src/Prax/Engine.hs`**:

```haskell
retable :: PraxState -> PraxState
retable st = st
  { improvables  = improvableDesires (practiceDefs st) (axioms st) (desires st)
  , footprint    = map pathNames (axiomFootprint (axioms st))
  , negFootprint = map pathNames (axiomNegPatterns (axioms st))
  , contMonotone = monotoneAxioms (axioms st) }

relevantDelta :: String -> PraxState -> Bool
relevantDelta s st =
  any (\x -> any (mayUnifyNames x) (footprint st))
      (map pathNames (s : evictionShadows s))

-- | May this insert take the continuation tier: the world is
-- continuation-safe, the insert evicts nothing, and it can defeat nothing.
monotoneInsert :: String -> PraxState -> Bool
monotoneInsert s st =
  contMonotone st
    && '!' `notElem` s
    && not (any (mayUnifyNames (pathNames s)) (negFootprint st))

-- | The continuation tier: grow the base and continue the ALREADY-CLOSED
-- view with the one new fact. A contradiction (⊥) falls back to the full
-- reclose path, which reaches the same "contradiction" marker from scratch.
applyGrow :: String -> PraxState -> PraxState
applyGrow s st = case closureFrom (axioms st) (readView st) [s] of
  Right v -> st { db = insert s (db st), readView = v }
  Left _  -> withDb (insert s) st
```

and the `Insert` case becomes a three-way branch (Delete unchanged from Task 3):

```haskell
performOutcome (Insert s) st =
  let st' | not (relevantDelta s st) = applyDirect (insert s) st
          | monotoneInsert s st      = applyGrow s st
          | otherwise                = withDb (insert s) st
  in case spawnedInstance s st of
       ...unchanged...
```

Export `monotoneInsert` (tests) alongside the existing exports.

- [ ] **Step 4: The proof run.** `-p "ViewInvariant"` (now guarding a THIRD construction path), `-p "GoldenDrive"`, `-p "Derive"`, `-p "Relevance"`, then the full suite once (expect 316 green: 309 + 4 DeriveSpec + 1 RelevanceSpec + 2 adjust to the actual count — report the real number) with wall time recorded.

- [ ] **Step 5: Gates**: zero warnings; hlint; grep-gates.

- [ ] **Step 6: Commit.**

```bash
git add src/Prax/Derive.hs src/Prax/Types.hs src/Prax/Engine.hs src/Prax/Relevance.hs \
        test/Prax/DeriveSpec.hs test/Prax/RelevanceSpec.hs
git commit -m "The continuation tier: monotone inserts grow the closed view in place"
```
(with the standard trailers)
