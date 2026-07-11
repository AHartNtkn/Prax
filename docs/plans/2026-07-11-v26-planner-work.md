# v26 — Planner Work Elimination Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the planner work measured to be decision-irrelevant — recomputed axiom closures and provably-fruitless predictions — with bit-for-bit identical decisions, per `docs/specs/2026-07-11-v26-planner-work.md`.

**Architecture:** (1) Golden decision-sequence tests captured from the CURRENT build before anything changes (the exactness proof). (2) The closed view becomes a lazily-computed `PraxState` field maintained by smart helpers in `Prax.Engine`; raw `db`/`axioms` record updates are eliminated everywhere. (3) A new `Prax.Relevance` module computes, once per world, which desires any authored action could possibly improve (outcome↔pattern unification with polarity, conservative-TRUE on anything uncertain); `predictMove` skips a pair outright when no believed desire is improvable. (4) A re-profile decides whether the Db tokenization rewrite is warranted — that decision returns to the controller either way.

**Tech Stack:** Haskell (GHC 9.10, cabal), tasty/tasty-hunit (existing deps only).

## Global Constraints

- **Exactness is the contract**: no change may alter any planner decision. The golden tests (Task 1) plus the full 292-test suite must stay green after every task, unmodified. If a golden test fails after a change, that change is WRONG — BLOCK with the trace; never re-capture a golden to match new behavior.
- `cabal build all` zero warnings (`cabal build all 2>&1 | grep -iE "warning" | grep -v "package list"` — empty); `hlint src app test` "No hints".
- TDD: failing/observed test first for every new unit of behavior; goldens are additionally verified non-vacuous by observing them fail under a deliberate perturbation, then restored.
- No heuristics/magic numbers; conservative fallbacks in Relevance must be conservative-TRUE (keep the pair) — an unsound skip is a Critical defect.
- Baseline: 292 tests green; full suite ~726s, `Prax.VillageSpec` group ~580–660s (do not run the full suite more often than each task's final gate demands; use targeted patterns while iterating).
- Never create `cabal.project.local` except transiently in Task 4, and delete it before that task's commit.
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: Golden decision sequences (captured before any change)

**Files:**
- Create: `test/Prax/GoldenDriveSpec.hs`
- Modify: `prax.cabal` (add `Prax.GoldenDriveSpec` to test `other-modules` after `Prax.FeudSpec`), `test/Spec.hs` (import + register alongside the others)

**Interfaces:**
- Consumes: `advance`/`npcAct` (`Prax.Loop`), `villageWorld`/`playerName` (`Prax.Worlds.Village`), `barWorld` (`Prax.Worlds.Bar`), `intrigueWorld` (`Prax.Worlds.Intrigue`).
- Produces: the exactness net every later task must keep green.

- [ ] **Step 1: Write the harness with EMPTY expected lists** (they will be filled from a live capture, never authored):

```haskell
module Prax.GoldenDriveSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, (@?=))

import           Prax.Types
import           Prax.Loop (advance, npcAct)
import           Prax.Worlds.Village (villageWorld, playerName)
import           Prax.Worlds.Bar (barWorld)
import           Prax.Worlds.Intrigue (intrigueWorld)

-- One planner-driven turn per cast member per round; the named character
-- idles (mirrors VillageSpec's driveIdle). Each turn contributes one line:
-- "<actor>: <label>" for a performed action, "<actor>: -" for idle/no move.
driveLabels :: Int -> Maybe String -> PraxState -> [String]
driveLabels n idle st0 = go n st0
  where
    go 0 _  = []
    go k st =
      let (actor, st1) = advance st
      in if Just (charName actor) == idle
           then (charName actor ++ ": -") : go (k - 1) st1
           else case npcAct 2 actor st1 of
                  (mga, st2) ->
                    (charName actor ++ ": " ++ maybe "-" gaLabel mga)
                      : go (k - 1) st2

-- Captured from a live run of the pre-v26 planner (see the capture program in
-- this plan). These sequences ARE the planner's contract: any change that
-- perturbs a single decision fails here. Never edit them to match new
-- behavior — a failure means the change is wrong.
villageGolden :: [String]
villageGolden = []   -- filled in Step 2

barGolden :: [String]
barGolden = []       -- filled in Step 2

intrigueGolden :: [String]
intrigueGolden = []  -- filled in Step 2

tests :: TestTree
tests = testGroup "Prax.GoldenDrive (decision-sequence exactness)"
  [ testCase "village: 3 rounds of free play, decision for decision" $
      driveLabels 21 (Just playerName) villageWorld @?= villageGolden
  , testCase "bar: 12 turns, decision for decision" $
      driveLabels 12 Nothing barWorld @?= barGolden
  , testCase "intrigue: 12 turns, decision for decision" $
      driveLabels 12 Nothing intrigueWorld @?= intrigueGolden
  ]
```

- [ ] **Step 2: Capture the sequences live.** Write this program to the session scratchpad (NOT the repo) as `GoldenCapture.hs`, with `driveLabels` pasted in verbatim from Step 1:

```haskell
module Main (main) where

import           Prax.Types
import           Prax.Loop (advance, npcAct)
import           Prax.Worlds.Village (villageWorld, playerName)
import           Prax.Worlds.Bar (barWorld)
import           Prax.Worlds.Intrigue (intrigueWorld)

-- <paste driveLabels here verbatim>

main :: IO ()
main = do
  let hs name xs = do
        putStrLn (name ++ " :: [String]")
        putStrLn (name ++ " =")
        mapM_ putStrLn
          [ (if i == 0 then "  [ " else "  , ") ++ show x
          | (i, x) <- zip [0 :: Int ..] xs ]
        putStrLn "  ]"
  hs "villageGolden"  (driveLabels 21 (Just playerName) villageWorld)
  hs "barGolden"      (driveLabels 12 Nothing barWorld)
  hs "intrigueGolden" (driveLabels 12 Nothing intrigueWorld)
```

Compile and run (from the repo root; ~30–60s, dominated by the village rounds):
`cabal exec -- ghc -O1 -package prax -outputdir <scratchpad>/gbuild -o <scratchpad>/gcap <scratchpad>/GoldenCapture.hs && <scratchpad>/gcap`
Paste its output over the three empty definitions in `GoldenDriveSpec.hs`.

- [ ] **Step 3: Register and observe GREEN.**

Run: `cabal test --test-options='-p "GoldenDrive"' 2>&1 | tail -5`
Expected: 3 tests pass (they pin the current build against itself).

- [ ] **Step 4: Observe the goldens are non-vacuous.** Temporarily change one string inside `villageGolden` (e.g. append `"X"` to any label), rerun the pattern, OBSERVE the failure and that its diff names the perturbed turn; restore the exact captured value; rerun; OBSERVE green again. State both observations in your report.

- [ ] **Step 5: Commit.**

```bash
git add test/Prax/GoldenDriveSpec.hs prax.cabal test/Spec.hs
git commit -m "Golden decision sequences: the v26 exactness net"
```
(with the standard trailers)

---

### Task 2: The closed view as a cached state field

**Files:**
- Modify: `src/Prax/Types.hs` (field + emptyState)
- Modify: `src/Prax/Engine.hs` (smart helpers; delete the `readView` function; route all db writes)
- Modify: `src/Prax/Persist.hs` (route its state rebuild)
- Modify: every site doing a post-construction record update of `axioms` (find them ALL: `grep -rn "axioms = " src app test | grep -v "axioms st\|axioms ::"` — at minimum `src/Prax/Worlds/Village.hs`, plus test fixtures in `PersonaSpec`, `MindsSpec`, and any other world/spec the grep reveals)
- Modify: import lists that had `readView` from `Prax.Engine` (it moves to the `PraxState` field, re-exported by `Prax.Types`; the grep in Step 4 finds them)
- Test: `test/Prax/EngineSpec.hs` (one new test)

**Interfaces:**
- Consumes: `closure` (`Prax.Derive`), `insert` (`Prax.Db`).
- Produces (Task 3 relies on):
  - `PraxState` gains `readView :: Db` — the closure of `db` under `axioms`, established at construction (lazy). All existing `readView st` call sites compile unchanged as field accesses.
  - `Prax.Engine` exports `withDb :: (Db -> Db) -> PraxState -> PraxState` and `setAxioms :: [Axiom] -> PraxState -> PraxState` — the ONLY sanctioned ways to change `db`/`axioms` on a built state.

- [ ] **Step 1: Write the failing test.** In `test/Prax/EngineSpec.hs`, add (adjusting imports to the file's existing style — it already imports `Prax.Engine` and `Prax.Types`):

```haskell
  , testCase "setAxioms re-derives the cached view on a built state" $ do
      let ax = axiom [ Match "parent.X.Y" ] [ "elder.X" ]
          st0 = performOutcome (Insert "parent.ada.bea") emptyState
      assertBool "no axioms: nothing derived"
        (not (exists "elder.ada" (readView st0)))
      let st1 = setAxioms [ax] st0
      assertBool "derived after setAxioms" (exists "elder.ada" (readView st1))
      -- and the view tracks subsequent writes through the helpers
      let st2 = performOutcome (Insert "parent.bea.cal") st1
      assertBool "new base fact derives too" (exists "elder.bea" (readView st2))
```

(`axiom` comes from `Prax.Derive`; add the import if the file lacks it.)

- [ ] **Step 2: Observe RED.**

Run: `cabal test --test-options='-p "Engine"' 2>&1 | tail -5`
Expected: compile failure — `setAxioms` is not defined. That is the RED.

- [ ] **Step 3: Implement.**

**(a) `src/Prax/Types.hs`** — add the field and initialize it in `emptyState`:

```haskell
data PraxState = PraxState
  { db           :: Db
  , practiceDefs :: Map String Practice
  , characters   :: [Character]
  , cursor       :: Int
  , axioms       :: [Axiom]
  , sorts        :: [(String, [String])]
  , desires      :: [Desire]
  , predictionScope :: [Condition]
  , readView     :: Db
    -- ^ The db closed under the axioms — established (lazily) whenever the
    -- state is built, so reads share one closure per state. Change 'db' or
    -- 'axioms' ONLY through 'Prax.Engine.withDb' / 'Prax.Engine.setAxioms',
    -- which rebuild it; a raw record update of either leaves this stale.
  }
```

and in `emptyState`: `, readView = emptyDb` (with `emptyDb` already imported).

**(b) `src/Prax/Engine.hs`** — delete the `readView` function (and its export; the field accessor now comes with `Prax.Types`), and add the helpers plus routing. The reclose logic is the old function body, moved:

```haskell
-- | Rebuild the cached closed view. Internal: every helper that changes
-- 'db' or 'axioms' must end here.
reclose :: PraxState -> PraxState
reclose st = st { readView = case closure (axioms st) (db st) of
                               Right closed -> closed
                               Left _       -> insert "contradiction" (db st) }

-- | The only sanctioned way to change the fact base of a built state.
withDb :: (Db -> Db) -> PraxState -> PraxState
withDb f st = reclose st { db = f (db st) }

-- | The only sanctioned way to change the axioms of a built state.
setAxioms :: [Axiom] -> PraxState -> PraxState
setAxioms axs st = reclose st { axioms = axs }
```

Export `withDb` and `setAxioms`. Route every db write in the module through `withDb`:
- `performOutcome (Delete s) st = withDb (retract s) st`
- the `Insert` case's `st { db = insert s (db st) }` becomes `withDb (insert s) st`
- the practice-spawn/dataFacts site (`db = insertAll …`) becomes `withDb (insertAll …)`
(keep each site's surrounding logic identical; only the write is rerouted).

**(c) `src/Prax/Persist.hs`** — `world { db = insertAll …, cursor = c }` becomes
`(withDb (const (insertAll (filter (not . null) rest) emptyDb)) world) { cursor = c }` (import `withDb`; the `cursor` record update is harmless).

**(d) axiom record updates** — every `st { axioms = … }`/construction-literal `{ …, axioms = … }` on an already-built state becomes a `setAxioms` application. For the village:

```haskell
villageWorld =
  (setAxioms villageAxioms (foldl (flip performOutcome) base (setup ++ personaFacts)))
    { desires = [ earnBreadPursuit, spitesCarol ] ++ personaVocabulary [honest]
    , predictionScope = [ Or [ together, sightedWithin 2 ] ]
    }
```

(`desires`/`predictionScope`/`characters`/`cursor` record updates remain legal — they do not affect the view. Apply the same shape to every site the grep finds, including test fixtures like PersonaSpec's `axioms = [transparent]`.)

- [ ] **Step 4: The stale-view grep-gate.** Run and record:

```bash
grep -rn "{ *db *=\|, *db *=\|{ *axioms *=\|, *axioms *=" src app test --include="*.hs" | grep -vE "src/Prax/(Engine|Types)\.hs"
```

Expected: NO matches (Engine holds the helpers; Types holds `emptyState`). Any hit is an unrouted site — fix it before proceeding.

- [ ] **Step 5: Fix `readView` imports.** `grep -rln "Engine.*readView\|readView.*Engine" src app test` — remove `readView` from every `import Prax.Engine (…)` list (the field accessor arrives via the modules' existing `Prax.Types` imports). Build until clean.

- [ ] **Step 6: Observe GREEN — including the goldens.**

Run: `cabal test 2>&1 | tail -5`
Expected: 295 tests green (292 + 3 goldens), sequences identical. Note the wall time in your report (this task alone should collapse the Village group substantially).

- [ ] **Step 7: Commit.**

```bash
git add -A src test
git commit -m "Cache the closed view per state: one closure per state, not ~15k per round"
```
(with the standard trailers)

---

### Task 3: `Prax.Relevance` — the improvability pre-filter

**Files:**
- Create: `src/Prax/Relevance.hs`
- Create: `test/Prax/RelevanceSpec.hs`
- Modify: `src/Prax/Types.hs` (field `improvables :: [String]`), `src/Prax/Engine.hs` (maintain it; `setDesires`), `src/Prax/Minds.hs` (`believedDesires`), `src/Prax/Planner.hs` (the skip), every post-construction `{ desires = … }` record update (grep as in Task 2 — worlds AND test fixtures switch to `setDesires`)
- Modify: `prax.cabal` (expose `Prax.Relevance` after `Prax.Minds`; test module after `Prax.PersonaSpec`), `test/Spec.hs`

**Interfaces:**
- Consumes: Task 2's helper discipline; `Axiom(..)` fields (`Prax.Derive`), `pathNames`/`isVariable` (`Prax.Db`).
- Produces:
  - `mayUnify :: String -> String -> Bool`
  - `improvableDesires :: Map String Practice -> [Axiom] -> [Desire] -> [String]`
  - `PraxState.improvables :: [String]`; `Prax.Engine.setDesires :: [Desire] -> PraxState -> PraxState`
  - `Prax.Minds.believedDesires :: PraxState -> Character -> Character -> [Desire]` (with `believedWants` kept, now defined through it)
  - `predictMove` returns `Nothing` — without grounding or evaluating anything — when no believed desire is improvable.

- [ ] **Step 1: Write the failing tests.** Create `test/Prax/RelevanceSpec.hs`:

```haskell
module Prax.RelevanceSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool)

import           Prax.Engine (setDesires)
import           Prax.Types
import           Prax.Worlds.Village (villageWorld)
import           Prax.Relevance

tests :: TestTree
tests = testGroup "Prax.Relevance"
  [ testCase "mayUnify: variables are wildcards, prefixes are compatible" $ do
      assertBool "var vs concrete" (mayUnify "lied.Actor.H.stole.C.loaf"
                                             "lied.eve.dana.stole.carol.loaf")
      assertBool "prefix compatibility (longer insert, shorter pattern)"
        (mayUnify "Hearer.believes.took.Culprit.gem.heard.Actor"
                  "oz.believes.took.kit.gem")
      assertBool "distinct constants do not unify"
        (not (mayUnify "regards.W.carol.thief" "practice.earnBread.Owner.done.S"))

  , testCase "the village table: conscience dead, spite and pursuit live" $ do
      let tbl = improvableDesires (practiceDefs villageWorld)
                                  (axioms villageWorld)
                                  (desires villageWorld)
      -- No authored village action Deletes a lied-mark, no axiom head touches
      -- one: a conscience-only believed model can never be improved.
      assertBool "clean-conscience is not improvable"
        ("clean-conscience" `notElem` tbl)
      -- spites-carol counts DERIVED regards facts (standingUnless's head):
      -- conservatively improvable, so eve's predicted whisper stays live.
      assertBool "spites-carol is improvable" ("spites-carol" `elem` tbl)
      -- pursuit counts base done-facts the stage actions Insert.
      assertBool "pursues-earnBread is improvable"
        ("pursues-earnBread" `elem` tbl)

  , testCase "the state carries the table and setDesires rebuilds it" $ do
      assertBool "villageWorld's field matches the module computation"
        (improvables villageWorld
           == improvableDesires (practiceDefs villageWorld)
                                (axioms villageWorld)
                                (desires villageWorld))
      let st = setDesires [ d | d <- desires villageWorld
                              , desireName d == "spites-carol" ] villageWorld
      assertBool "narrowed vocabulary narrows the table"
        ("pursues-earnBread" `notElem` improvables st)
  ]
```

Register the spec in `prax.cabal`/`test/Spec.hs`.

- [ ] **Step 2: Observe RED.** `cabal build all 2>&1 | tail -3` — fails: `Prax.Relevance` does not exist.

- [ ] **Step 3: Implement `src/Prax/Relevance.hs`:**

```haskell
-- | Which desires can authored action even in principle improve? Computed once
-- per world from the vocabulary (spec
-- @docs/specs/2026-07-11-v26-planner-work.md@ §2) and consulted by the planner
-- to skip predictions that are provably fruitless: a believed model none of
-- whose desires any available action can improve admits no motivated move.
--
-- The analysis is __conservative by construction__: it may only ever answer
-- "not improvable" when that is provable from the authored patterns. Anything
-- uncertain — outcomes behind unresolvable 'Call's, wants over facts an axiom
-- may derive, wants gated by 'Subquery'\/'Count'\/'Calc' — counts as
-- improvable. An unsound "not improvable" is a planner behavior change and a
-- defect; a spurious "improvable" merely costs the evaluation we would have
-- done anyway.
module Prax.Relevance
  ( mayUnify
  , improvableDesires
  ) where

import           Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map

import           Prax.Db (isVariable, pathNames)
import           Prax.Derive (Axiom (..))
import           Prax.Query (Condition (..))
import           Prax.Types

-- | Could a grounded instance of one path pattern be an instance (or a
-- prefix\/extension) of the other? Segments unify when either is a variable
-- or they are equal; length mismatch is prefix-compatible (a 'Match' sees
-- subtrees). Conservative: never a false negative.
mayUnify :: String -> String -> Bool
mayUnify a b = and (zipWith seg (pathNames a) (pathNames b))
  where seg x y = isVariable x || isVariable y || x == y

-- The insert- and delete-shaped atoms an outcome can produce, resolving
-- 'Call's through the worlds' declared functions (conservatively: all cases).
-- An @!@ path both asserts its value and evicts siblings, so it counts on
-- both sides. Returns Nothing for "unknown effects" (unresolvable Call):
-- the caller must treat that as improves-everything.
outcomeAtoms :: Map String [Outcome] -> [String] -> Outcome
             -> Maybe ([String], [String])
outcomeAtoms fns visited o = case o of
  Insert s | '!' `elem` s -> Just ([s], [s])
           | otherwise    -> Just ([s], [])
  Delete s                -> Just ([], [s])
  ForEach _ outs          -> mconcat' (map (outcomeAtoms fns visited) outs)
  Call fn _
    | fn `elem` visited   -> Just ([], [])           -- cycle: already counted
    | otherwise -> case Map.lookup fn fns of
        Nothing   -> Nothing                         -- unknown function: wild
        Just outs -> mconcat' (map (outcomeAtoms fns (fn : visited)) outs)
  where
    mconcat' ms = do
      pairs <- sequence ms
      pure (concatMap fst pairs, concatMap snd pairs)

-- Positive and negated path patterns of a want's conditions. The Bool is
-- "uncertain": the want's satisfaction depends on machinery (numeric binds,
-- counts, subqueries) beyond pattern presence.
wantPatterns :: [Condition] -> ([String], [String], Bool)
wantPatterns = foldr step ([], [], False)
  where
    step c (pos, neg, unc) = case c of
      Match p      -> (p : pos, neg, unc)
      Not p        -> (pos, p : neg, unc)
      Absent cs    -> let (p', n', u') = wantPatterns cs
                      in (pos ++ n', neg ++ p', unc || u')
      Exists cs    -> let (p', n', u') = wantPatterns cs
                      in (pos ++ p', neg ++ n', unc || u')
      Or clauses   -> let parts = map wantPatterns clauses
                      in ( pos ++ concatMap (\(p', _, _) -> p') parts
                         , neg ++ concatMap (\(_, n', _) -> n') parts
                         , unc || any (\(_, _, u') -> u') parts )
      Eq _ _       -> (pos, neg, unc)
      Neq _ _      -> (pos, neg, unc)
      Cmp {}       -> (pos, neg, unc)
      Calc {}      -> (pos, neg, True)
      Count {}     -> (pos, neg, True)
      Subquery {}  -> (pos, neg, True)

-- | The names of the desires some authored action might improve. See the
-- module header for the conservativity contract.
improvableDesires :: Map String Practice -> [Axiom] -> [Desire] -> [String]
improvableDesires defs axs ds =
  [ desireName d | d <- ds, improvable d ]
  where
    practices = Map.elems defs
    fns = Map.fromList [ (fnName f, concatMap caseOutcomes (fnCases f))
                       | p <- practices, f <- functions p ]
    -- every effect an authored action can cause: its declared outcomes, plus
    -- the initOutcomes of any practice (spawning runs them)
    atoms = [ outcomeAtoms fns [] o
            | p <- practices, a <- actions p, o <- actionOutcomes a ]
         ++ [ outcomeAtoms fns [] o | p <- practices, o <- initOutcomes p ]
    wild = Nothing `elem` atoms
    inserted = concatMap (maybe [] fst) atoms
    deleted  = concatMap (maybe [] snd) atoms
    -- axiom heads, including their auto-□-lifted forms, count as derivable:
    -- a want over a derivable pattern is conservatively improvable.
    heads = concatMap axiomThen axs
    liftedHeads = [ "obliged.W." ++ h | h <- heads ]
    derivable p = any (mayUnify p) (heads ++ liftedHeads)
    improvable (Desire _ (Want conds u))
      | u == 0    = False
      | wild      = True
      | unc       = True
      | any derivable (pos ++ neg) = True
      | u > 0     = any (\i -> any (mayUnify i) pos) inserted
                    || any (\dl -> any (mayUnify dl) neg) deleted
      | otherwise = any (\dl -> any (mayUnify dl) pos) deleted
                    || any (\i -> any (mayUnify i) neg) inserted
      where (pos, neg, unc) = wantPatterns conds
```

- [ ] **Step 4: Wire the state field.**

**(a) `src/Prax/Types.hs`**: add to `PraxState`:

```haskell
  , improvables :: [String]
    -- ^ Names of desires some authored action may improve
    -- ('Prax.Relevance.improvableDesires') — rebuilt with the vocabulary
    -- ('Prax.Engine.definePractices' / 'setAxioms' / 'setDesires'); the
    -- planner skips predictions over models with no improvable desire.
```

`emptyState`: `, improvables = []`.

**(b) `src/Prax/Engine.hs`** (imports `Prax.Relevance`):

```haskell
-- | Rebuild the derived vocabulary tables. Internal: every helper that
-- changes 'practiceDefs', 'axioms', or 'desires' must end here.
retable :: PraxState -> PraxState
retable st = st { improvables =
                    improvableDesires (practiceDefs st) (axioms st) (desires st) }

setAxioms :: [Axiom] -> PraxState -> PraxState
setAxioms axs st = retable (reclose st { axioms = axs })

-- | The only sanctioned way to change the desire vocabulary of a built state.
setDesires :: [Desire] -> PraxState -> PraxState
setDesires ds st = retable st { desires = ds }
```

`definePractices` ends in `retable` too (its `practiceDefs` update). Export `setDesires`.

**(c) Route the `{ desires = … }` record updates**: `grep -rn "desires = " src app test --include="*.hs" | grep -v "desires st\|charDesires\|traitDesires\|desires ::"` — every post-construction site (villageWorld's update, VillageSpec's `villageWorld { desires = vocab }`, MindsSpec/PersonaSpec/DeceitSpec fixtures, other worlds) becomes `setDesires …` composed with the remaining harmless updates. Extend the Task 2 grep-gate to `desires`:

```bash
grep -rn "{ *desires *=\|, *desires *=" src app test --include="*.hs" | grep -vE "src/Prax/(Engine|Types)\.hs"
```

Expected: no matches.

- [ ] **Step 5: The planner skip + `believedDesires`.**

**(a) `src/Prax/Minds.hs`**:

```haskell
-- | The vocabulary desires the predictor believes (any provenance) the mover
-- to have. The model can be wrong — it is the predictor's, not the mover's.
believedDesires :: PraxState -> Character -> Character -> [Desire]
believedDesires st p m =
  [ d | d <- desires st
      , exists (charName p ++ ".believes.desires." ++ charName m
                  ++ "." ++ desireName d) view ]
  where view = readView st

believedWants :: PraxState -> Character -> Character -> [Want]
believedWants st p m = map (wantFor (charName m)) (believedDesires st p m)
```

(export `believedDesires`; `believedWants` keeps its name, callers unchanged).

**(b) `src/Prax/Planner.hs`** — `predictMove` becomes:

```haskell
predictMove :: PraxState -> Character -> Character -> Maybe GroundedAction
predictMove st p m =
  case believedDesires st p m of
    [] -> Nothing
    ds
      -- no believed desire is improvable by any authored action: no candidate
      -- can strictly beat standing still, so don't ground or evaluate any
      -- (Prax.Relevance; exact — improvable desires keep the FULL model,
      -- unimprovable costs included, so deterrents still deter)
      | all ((`notElem` improvables st) . desireName) ds -> Nothing
      | otherwise ->
          let model  = map (wantFor (charName m)) ds
              still  = evaluate st model
              scored = sortOn (\(ga, s) -> (Down s, gaLabel ga))
                         [ (a, evaluate (performAction st a) model)
                         | a <- candidateActions st m ]
          in case scored of
               ((a, s) : _) | s > still -> Just a
               _                        -> Nothing
```

(imports: `believedDesires`, `wantFor` from `Prax.Minds`; drop the `believedWants` import if now unused).

- [ ] **Step 6: Observe GREEN — suite + goldens unmodified.**

Run: `cabal test 2>&1 | tail -5`
Expected: 298 tests green (295 + 3 RelevanceSpec), golden sequences identical, all v25 prediction tests (PersonaSpec believed-conscience, VillageSpec prediction-contrast — the soundness-critical mixed-model cases) untouched and green. Record the wall time.

- [ ] **Step 7: Commit.**

```bash
git add -A src test prax.cabal
git commit -m "Prax.Relevance: skip predictions no authored action could motivate"
```
(with the standard trailers)

---

### Task 4: Re-profile; decide the Db tokenization question with evidence

**Files:** none in the repo (scratchpad only; a transient `cabal.project.local` deleted before finishing).

- [ ] **Step 1:** Re-create the one-round profile from the spec's findings: write to the scratchpad the same 7-turn drive used pre-round (`V26Prof.hs` exists in the session scratchpad; reuse it), create `cabal.project.local` with `profiling: True` / `profiling-detail: all-functions`, `cabal build lib:prax`, compile the driver with `ghc -O1 -prof -fprof-auto -package prax`, run with `+RTS -p -RTS`.
- [ ] **Step 2:** Record: total time vs the pre-round 7.07s profile; the top-10 cost centres; the `readView` entry count (expect ~one per distinct state) and `predictMove` entry count (expect the skip visible).
- [ ] **Step 3:** `rm cabal.project.local` and `cabal build all 2>&1 | tail -2` (restore the vanilla plan). Confirm `git status` is clean.
- [ ] **Step 4:** Report the numbers and STOP — no commit (nothing in the repo changed). The controller decides from the profile whether a tokenization task is commissioned (spec §3's criterion: it remains the top cost centre) or the measurement is recorded and the rewrite skipped.

---

### Task 5: Docs, gates, and the honest performance report

**Files:**
- Modify: `docs/LEDGER.md`, `README.md` (only if its performance/feature text warrants it — check)

- [ ] **Step 1: Measure the headline numbers ONCE**: `cabal test 2>&1 | tail -3` (total wall time; 298 expected) and `cabal test --test-options='-p "Village"' 2>&1 | tail -3` (group time).
- [ ] **Step 2: LEDGER.** Add the v26 legend row (cached per-state view; Relevance pre-filter; golden decision-sequence net; measured before/after: suite ~726s → <measured>, Village group ~580–660s → <measured> — real numbers only, no rounding up the win). Update the v25 "planner runtime under cast growth" backlog item: resolved by v26 for the measured regression, with the residual (Db tokenization decision per Task 4's profile, incremental cross-state closure) recorded as the item's remaining scope or closed, whichever the Task 4 evidence says.
- [ ] **Step 3: Run the full gate** and record all four outputs:

```bash
cabal build all 2>&1 | grep -iE "warning" | grep -v "package list"   # empty
hlint src app test                                                   # No hints
for w in bar intrigue play feud audience village dm; do cabal run -v0 prax -- check $w; done
grep -rn "{ *db *=\|, *db *=\|{ *axioms *=\|, *axioms *=\|{ *desires *=\|, *desires *=" src app test --include="*.hs" | grep -vE "src/Prax/(Engine|Types)\.hs"   # empty
```

- [ ] **Step 4: Commit.**

```bash
git add docs/LEDGER.md README.md
git commit -m "Docs: v26 — the planner stops paying for work that decides nothing"
```
(with the standard trailers)

---

### Task 4b: Single tokenization on the measured hot paths (commissioned by the Task 4 profile)

The Task 4 re-profile (one village round, post Tasks 2–3): 7.07s → 2.83s; `readView` is now a
free field access; the remaining cost is `reclose` (11,840 entries — one closure per distinct
state — 71.8% inherited) and **string tokenization is still ~48% of total** (`tokens.go` 23.5+5.4,
`trim.f` 15.2, `parseNames` 2.0, `insertToks` 2.4). Spec §3's criterion is met. The waste has two
mechanical sources: (1) `query` re-parses each condition's pattern once **per binding-set**
(`concatMap (unify s db) matches`), and (2) `closure`'s loop grounds heads to strings that
`entailed`/`meetOne`/`insertAll` immediately re-tokenize, with the heads themselves re-tokenized
per binding. Both fixes are internal; the String authoring surface does not change.

**Files:**
- Modify: `src/Prax/Db.hs` (export `insertToks`; add `unifyNames`, `groundTokens`, `tokensToSentence`)
- Modify: `src/Prax/Query.hs` (hoist parsing in `queryWith`/`unifyAll` usage)
- Modify: `src/Prax/Derive.hs` (token-level closure loop; heads tokenized once per closure)
- Test: `test/Prax/DbSpec.hs` (unit tests for the new Db functions)

**Interfaces:**
- Consumes: everything as of Task 3.
- Produces: identical observable behavior (goldens + 299 tests prove it), faster.

- [ ] **Step 1: Write the failing tests.** In `test/Prax/DbSpec.hs`, add (following the file's existing import/style conventions):

```haskell
  , testCase "unifyNames is unify with the parse hoisted out" $ do
      let db = insertAll ["at.bob!square", "at.eve!mill"] emptyDb
      unifyNames (pathNames "at.Who!Where") db Map.empty
        @?= unify "at.Who!Where" db Map.empty

  , testCase "groundTokens substitutes bindings segment-wise, preserving operators" $ do
      let toks = tokens "at.Who!Where"
          b    = Map.fromList [("Who", VStr "bob"), ("Where", VStr "square")]
      tokensToSentence (groundTokens toks b) @?= ground "at.Who!Where" b
      tokensToSentence (groundTokens (tokens "plain.path") Map.empty)
        @?= "plain.path"
```

(If `pathNames`/`tokens`/`ground`/`VStr` are not yet imported there, extend the import list.)

- [ ] **Step 2: Observe RED.** `cabal test --test-options='-p "Prax.Db"' 2>&1 | tail -5` — compile failure: `unifyNames`/`groundTokens`/`tokensToSentence` undefined.

- [ ] **Step 3: Implement.**

**(a) `src/Prax/Db.hs`** — add to the export list `insertToks`, `unifyNames`, `groundTokens`, `tokensToSentence`, and define:

```haskell
-- | 'unify' with the sentence already split into names — for callers that
-- evaluate one pattern against many binding sets ('Prax.Query' hoists the
-- parse out of that loop).
unifyNames :: [String] -> Db -> Bindings -> [Bindings]
unifyNames names db0 bindings =
  map snd (foldl step [(db0, bindings)] names)
  where
    step worlds part = concatMap (descend part) worlds
    descend part (Db _ m, b)
      | isVariable part =
          case Map.lookup part b of
            Just v  -> case Map.lookup (valToString v) m of
                         Just sub -> [(sub, b)]
                         Nothing  -> []
            Nothing -> [ (sub, Map.insert part (VStr k) b)
                       | (k, sub) <- Map.toList m ]
      | otherwise =
          case Map.lookup part m of
            Just sub -> [(sub, b)]
            Nothing  -> []

unify :: String -> Db -> Bindings -> [Bindings]
unify sentence = unifyNames (parseNames sentence)
```

(the old `unify` body moves into `unifyNames`; `unify` becomes the parsing entry point.)

```haskell
-- | 'ground' at the token level: substitute bindings into already-split
-- tokens. 'Prax.Derive.closure' grounds each axiom head once per binding —
-- tokenizing the head template once per closure, not once per binding.
groundTokens :: [(String, Maybe Char)] -> Bindings -> [(String, Maybe Char)]
groundTokens toks b = [ (value n, op) | (n, op) <- toks ]
  where
    value name
      | isVariable name = maybe name valToString (Map.lookup name b)
      | otherwise       = name

-- | Re-emit tokens as a sentence (inverse of 'tokens' up to trimming).
tokensToSentence :: [(String, Maybe Char)] -> String
tokensToSentence = concatMap emit
  where emit (name, mop) = name ++ maybe "" pure mop
```

and redefine `ground` through them: `ground sentence b = tokensToSentence (groundTokens (tokens sentence) b)`.

**(b) `src/Prax/Query.hs`** — in `queryWith`, hoist the parse per condition instead of per binding:

```haskell
queryWith :: Bool -> Db -> [Condition] -> Bindings -> [Bindings]
queryWith inSub db conds b0 = foldl step [b0] conds
  where
    step matches cond = case cond of
      -- parse the pattern once per condition, not once per binding
      Match s -> let names = parseNames s
                 in concatMap (unifyNames names db) matches
      Not s   -> let names = parseNames s
                 in concatMap (\b -> [ b | null (unifyNames names db b) ]) matches
      _       -> concatMap (evalCond inSub db cond) matches
```

(`evalCond`'s own `Match`/`Not` cases stay for any direct callers; import `unifyNames`, `parseNames` from `Prax.Db`.)

**(c) `src/Prax/Derive.hs`** — the closure loop works in tokens end to end:

```haskell
closure :: [Axiom] -> Db -> Either Contradiction Db
closure []  db0 = Right db0
closure axs db0 = go db0 db0
  where
    rules = [ (body, map tokens hs)
            | Axiom body hs <- axs ++ mapMaybe liftObliged axs ]

    go model delta =
      let heads = [ groundTokens h b | (body, hs) <- rules
                                     , b <- deltaJoin model delta body, h <- hs ]
          fresh = nub (filter (not . entailed model) heads)
      in if null fresh
           then Right model
           else case foldM meetOne model fresh of
                  Left c  -> Left c
                  Right m -> go m (foldl (flip insertToks) emptyDb fresh)

    meetOne m h = maybe (Left (Contradiction (tokensToSentence h))) Right
                        (meet m (insertToks h emptyDb))
    entailed m h = leq m (insertToks h emptyDb)
```

(`deltaJoin` and everything else unchanged; imports gain `insertToks`, `tokens`, `groundTokens`, `tokensToSentence` and drop `insert`/`insertAll`/`ground` if now unused — chase the warnings to zero.)

- [ ] **Step 4: Observe GREEN — full suite once, goldens byte-identical.**

Run: `cabal test 2>&1 | tail -4`
Expected: 301 tests green (299 + 2). Record the wall time (the round's final headline).

- [ ] **Step 5: Gates.** Zero warnings; hlint "No hints".

- [ ] **Step 6: Commit.**

```bash
git add src/Prax/Db.hs src/Prax/Query.hs src/Prax/Derive.hs test/Prax/DbSpec.hs
git commit -m "Tokenize once: hoist pattern parsing out of binding loops"
```
(with the standard trailers)

---

### Task 5b: Shared test trajectories (user-directed follow-on)

The village drive tests re-simulate overlapping prefixes of two deterministic trajectories:
free-play from t=0 (snapshots wanted at turns 7, 28, 40, 49) and post-theft (42, 49, 70, 105).
That is ~490 driven turns where ~155 suffice. The planner is deterministic, so replacing each
test's private drive with a snapshot of a shared trace changes NOTHING about what is asserted —
same states, same assertions, same turn counts — only how often identical turns are simulated.

**Files:**
- Modify: `test/Prax/VillageSpec.hs` (only)

- [ ] **Step 1: Record the before number.** `cabal test --test-options='-p "Village"' 2>&1 | tail -3` — note the group wall time.

- [ ] **Step 2: Refactor the helper.** Replace the current `driveIdle` definition with:

```haskell
-- One planner-driven turn, with @idle@'s turn consumed but not acted.
idleStep :: String -> PraxState -> PraxState
idleStep idle st =
  let (actor, st1) = advance st
  in if charName actor == idle then st1 else snd (npcAct 2 actor st1)

-- Run @k@ turns with everyone planner-driven except @idle@, who waits.
driveIdle :: String -> Int -> PraxState -> PraxState
driveIdle idle n st = iterate (idleStep idle) st !! n

-- The suite's two long trajectories, shared across tests: the planner is
-- deterministic, so a test wanting the state after N turns reads a snapshot
-- of the one trace instead of re-simulating the same N turns privately.
-- Top-level sharing: each trace is computed once per test-suite run.
freePlayAt :: Int -> PraxState
freePlayAt = (trace !!)
  where trace = iterate (idleStep "you") villageWorld

postTheftAt :: Int -> PraxState
postTheftAt = (trace !!)
  where trace = iterate (idleStep "you") (doAct "bob" "steal the loaf" villageWorld)
```

- [ ] **Step 3: Swap the ten family call sites** (assertions and turn counts UNTOUCHED):

| line (approx) | old | new |
|---|---|---|
| 78 | `driveIdle "you" 42 (doAct "bob" "steal the loaf" villageWorld)` | `postTheftAt 42` |
| 84 | `driveIdle "you" 49 (doAct "bob" "steal the loaf" villageWorld)` | `postTheftAt 49` |
| 166 | `driveIdle "you" 70 (…)` | `postTheftAt 70` |
| 195 | `driveIdle "you" 105 (…)` | `postTheftAt 105` |
| 217 | `driveIdle "you" 7 villageWorld` | `freePlayAt 7` |
| 273 | `driveIdle "you" 28 villageWorld` | `freePlayAt 28` |
| 291 | `driveIdle "you" 40 villageWorld` | `freePlayAt 40` |
| 300 | `driveIdle "you" 40 villageWorld` | `freePlayAt 40` |
| 311 | `driveIdle "you" 49 villageWorld` | `freePlayAt 49` |
| 361 | `driveIdle "you" 49 villageWorld` | `freePlayAt 49` |

The one non-family drive (the "perfect crime", line ~282: 14 turns from a moved-away start)
keeps its private `driveIdle`.

- [ ] **Step 4: Observe GREEN + the after number.** `cabal test --test-options='-p "Village"' 2>&1 | tail -3` — all Village tests green (same count), group time recorded. Then the full suite ONCE: 301 green (goldens byte-identical — the shared traces must not change a single decision).

- [ ] **Step 5: Gates.** Zero warnings; hlint "No hints".

- [ ] **Step 6: Commit.**

```bash
git add test/Prax/VillageSpec.hs
git commit -m "VillageSpec: share the two drive trajectories across tests"
```
(with the standard trailers; include the before/after group times in the report, and hand them to the Task 5 LEDGER text if it has not yet committed — otherwise note them for the round report)
