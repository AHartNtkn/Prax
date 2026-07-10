# v19 — Quantified Outcomes + Witnessing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the `ForEach` quantified outcome to the engine and build the authored-witnessing layer (`Prax.Witness`) plus the village seed world on top of it, per `docs/specs/2026-07-10-v19-witnessing-design.md`.

**Architecture:** One small foundational extension (a fourth `Outcome` constructor with snapshot semantics, evaluated against the defeasible view) and two compiled layers: a combinator that appends a quantified belief-deposit to observable actions, and a demo world whose information asymmetry is visible in play.

**Tech Stack:** Haskell (GHC 9.10, cabal), tasty/tasty-hunit, aeson (existing deps only).

## Global Constraints

- `cabal build` must stay `-Wall`-clean; `hlint src app test` must report "No hints".
- TDD: every step writes the failing test first, observes it fail, then implements, then observes it pass.
- No heuristics, no magic numbers, no placeholder/TODO code, no skipped tests, no mocks.
- All 178 existing tests must stay green; every world (including the new one) must pass `prax check`.
- Commit after each green task with the trailer:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: `groundCondition` in `Prax.Query`

Grounding a `ForEach` needs to substitute the enclosing action's bindings into a `[Condition]`. No such traversal exists (only `ground :: String -> Bindings -> String` over sentences, in `Prax.Db`).

**Files:**
- Modify: `src/Prax/Query.hs` (add `groundCondition`, export it)
- Test: `test/Prax/QuerySpec.hs`

**Interfaces:**
- Consumes: `ground :: String -> Bindings -> String` (`Prax.Db`), the `Condition` ADT.
- Produces: `groundCondition :: Bindings -> Condition -> Condition` — used by Task 2's `groundOutcome`.

- [ ] **Step 1: Write the failing test** (append to the test list in `test/Prax/QuerySpec.hs`; add `groundCondition` to its `Prax.Query` import and `Prax.Db (Val (..))`/`Data.Map.Strict` imports as needed — check what the file already imports first):

```haskell
  , testCase "groundCondition substitutes bindings through every constructor" $ do
      let b = Map.fromList [("A", VStr "bob")]
      groundCondition b (Match "at.A!P")        @?= Match "at.bob!P"
      groundCondition b (Not "seen.A")          @?= Not "seen.bob"
      groundCondition b (Eq "A" "X")            @?= Eq "bob" "X"
      groundCondition b (Neq "W" "A")           @?= Neq "W" "bob"
      groundCondition b (Cmp Gt "A" "N")        @?= Cmp Gt "bob" "N"
      groundCondition b (Calc "R" Add "A" "1")  @?= Calc "R" Add "bob" "1"
      groundCondition b (Count "R" "A")         @?= Count "R" "bob"
      groundCondition b (Subquery "S" ["A"] [Match "p.A"])
                                                @?= Subquery "S" ["bob"] [Match "p.bob"]
      groundCondition b (Or [[Match "p.A"], [Match "q.A"]])
                                                @?= Or [[Match "p.bob"], [Match "q.bob"]]
      groundCondition b (Absent [Match "p.A"])  @?= Absent [Match "p.bob"]
      groundCondition b (Exists [Match "p.A"])  @?= Exists [Match "p.bob"]
```

- [ ] **Step 2: Run it, verify it fails**

Run: `cabal test 2>&1 | grep -A2 groundCondition`
Expected: FAIL — `groundCondition` not in scope (compile error).

- [ ] **Step 3: Implement** (in `src/Prax/Query.hs`; add to the export list):

```haskell
-- | Substitute bindings into every sentence/operand of a condition. Variables
-- not present in the bindings are left for the query to quantify.
groundCondition :: Bindings -> Condition -> Condition
groundCondition b c = case c of
  Match s          -> Match (ground s b)
  Not s            -> Not (ground s b)
  Eq x y           -> Eq (ground x b) (ground y b)
  Neq x y          -> Neq (ground x b) (ground y b)
  Cmp op x y       -> Cmp op (ground x b) (ground y b)
  Calc r op x y    -> Calc (ground r b) op (ground x b) (ground y b)
  Count r s        -> Count (ground r b) (ground s b)
  Subquery s f w   -> Subquery (ground s b) (map (`ground` b) f) (map (groundCondition b) w)
  Or clauses       -> Or (map (map (groundCondition b)) clauses)
  Absent cs        -> Absent (map (groundCondition b) cs)
  Exists cs        -> Exists (map (groundCondition b) cs)
```

(`ground` is already imported into `Prax.Query` from `Prax.Db` — verify; if not, extend the import.)

- [ ] **Step 4: Run tests, verify pass**

Run: `cabal test 2>&1 | tail -3`
Expected: all tests pass (179).

- [ ] **Step 5: Commit**

```bash
git add src/Prax/Query.hs test/Prax/QuerySpec.hs
git commit -m "Add groundCondition: substitute bindings through a Condition"
```

---

### Task 2: The `ForEach` outcome (Types + Engine)

**Files:**
- Modify: `src/Prax/Types.hs` (the `Outcome` ADT, ~line 70)
- Modify: `src/Prax/Engine.hs` (`groundOutcome` ~line 98, `performOutcome` ~line 104)
- Test: `test/Prax/EngineSpec.hs`

**Interfaces:**
- Consumes: `groundCondition` (Task 1), `query`, `readView`.
- Produces: `ForEach [Condition] [Outcome]` — the constructor every later task builds on.
  Semantics: evaluate conditions against `readView st` with empty seed bindings, **snapshot** all
  bindings first, then fold the sub-outcomes over each binding in order. Zero bindings ⇒ no-op.

- [ ] **Step 1: Write the failing tests** (append to `test/Prax/EngineSpec.hs`; use its existing helpers/imports — it already imports `Prax.Types`, `Prax.Engine`, `Prax.Db`):

```haskell
  , testCase "ForEach applies its outcomes for every binding" $ do
      let st = foldl (flip performOutcome) emptyState
                 [ Insert "member.a", Insert "member.b", Insert "member.c" ]
          st' = performOutcome (ForEach [ Match "member.X" ] [ Insert "greeted.X" ]) st
      mapM_ (\n -> assertBool ("greeted." ++ n) (exists ("greeted." ++ n) (db st')))
            ["a", "b", "c"]

  , testCase "ForEach with zero bindings is a no-op" $ do
      let st  = performOutcome (Insert "unrelated") emptyState
          st' = performOutcome (ForEach [ Match "member.X" ] [ Insert "greeted.X" ]) st
      db st' @?= db st

  , testCase "ForEach snapshots its bindings: mutations cannot extend the quantification" $ do
      -- Inserting a new member from inside the fold must NOT add a binding:
      -- quantification is over the state at entry.
      let st  = performOutcome (Insert "member.a") emptyState
          st' = performOutcome
                  (ForEach [ Match "member.X" ]
                           [ Insert "member.b", Insert "visited.X" ]) st
      assertBool "visited the original member" (exists "visited.a" (db st'))
      assertBool "did NOT visit the member inserted mid-fold"
        (not (exists "visited.b" (db st')))

  , testCase "ForEach grounds the enclosing action's bindings first" $ do
      let p = practice
            { practiceId = "tell", roles = ["R"]
            , actions = [ action "[Actor]: tell friends about [Target]"
                            [ Match "target.Target" ]
                            [ ForEach [ Match "friend.Target.W" ]
                                      [ Insert "told.W.Target" ] ] ] }
          st = foldl (flip performOutcome)
                 ((definePractices [p] emptyState)
                    { characters = [ character "ann" ] })
                 [ Insert "practice.tell.stage"
                 , Insert "target.bob"
                 , Insert "friend.bob.carol", Insert "friend.bob.dave"
                 , Insert "friend.eve.mallory" ]   -- a different target's friend: must not fire
          st' = case possibleActions st "ann" of
                  (ga : _) -> performAction st ga
                  []       -> error "tell action not offered"
      assertBool "told carol about bob" (exists "told.carol.bob" (db st'))
      assertBool "told dave about bob"  (exists "told.dave.bob" (db st'))
      assertBool "eve's friend untouched" (not (exists "told.mallory.eve" (db st')))

  , testCase "ForEach nests: outer bindings ground the inner quantifier" $ do
      let st = foldl (flip performOutcome) emptyState
                 [ Insert "row.a", Insert "row.b", Insert "col.x", Insert "col.y" ]
          st' = performOutcome
                  (ForEach [ Match "row.R" ]
                           [ ForEach [ Match "col.C" ] [ Insert "cell.R.C" ] ]) st
      mapM_ (\s -> assertBool s (exists s (db st')))
            [ "cell.a.x", "cell.a.y", "cell.b.x", "cell.b.y" ]
```

If `EngineSpec` lacks any of `practice`/`action`/`character`/`definePractices`/`possibleActions`/`performAction`/`exists` in its imports, extend the imports; do not redefine helpers.

- [ ] **Step 2: Run, verify they fail**

Run: `cabal test 2>&1 | grep -iE "ForEach|error" | head`
Expected: compile error — `ForEach` is not a `Outcome` constructor.

- [ ] **Step 3: Implement.** In `src/Prax/Types.hs`, extend the ADT (keep the comment style):

```haskell
data Outcome
  = Insert String            -- ^ assert a sentence (may spawn a practice)
  | Delete String            -- ^ retract a subtree
  | Call String [String]     -- ^ invoke a practice 'Function' by name with args
  | ForEach [Condition] [Outcome]
    -- ^ Quantified effect: for /every/ binding of the conditions (evaluated
    -- against the closed view, snapshot at entry), apply the sub-outcomes.
  deriving (Eq, Show)
```

(`Prax.Types` must import `Condition` from `Prax.Query` — check for an import cycle: `Prax.Query` must not import `Prax.Types`. It doesn't today; verify with `grep "import Prax" src/Prax/Query.hs`. If `Types` already imports `Prax.Query`, reuse it.)

In `src/Prax/Engine.hs`:

```haskell
groundOutcome :: Outcome -> Bindings -> Outcome
groundOutcome (Insert s)          b = Insert (ground s b)
groundOutcome (Delete s)          b = Delete (ground s b)
groundOutcome (Call fn args)      b = Call fn (map (`ground` b) args)
groundOutcome (ForEach conds outs) b =
  ForEach (map (groundCondition b) conds) (map (`groundOutcome` b) outs)
```

and a new `performOutcome` case (before or after the `Call` case):

```haskell
performOutcome (ForEach conds outs) st =
  let bs = query (readView st) conds Map.empty   -- snapshot: all bindings up front
  in foldl' (\s b -> foldl' (\s2 o -> performOutcome (groundOutcome o b) s2) s outs) st bs
```

Extend `Prax.Engine`'s import of `Prax.Query` with `groundCondition`.

- [ ] **Step 4: Run tests, verify pass**

Run: `cabal test 2>&1 | tail -3` and `cabal build 2>&1 | grep -i warning || echo clean`
Expected: all pass; `-Wall` clean. **Note:** the build will now warn about non-exhaustive patterns anywhere `Outcome` is scrutinized — `Prax.TypeCheck` (`outcomeUses`, `outcomeSents`, sort pass) and `Prax.Script.Json`. If it does, those are Task 3/4's subjects; add the minimally-correct case *in this task* only where the warning forces it, and let Tasks 3–4 replace it with tested behavior. `-Wall`-clean is non-negotiable at every commit.

- [ ] **Step 5: Commit**

```bash
git add src/Prax/Types.hs src/Prax/Engine.hs test/Prax/EngineSpec.hs
git commit -m "Add ForEach: quantified outcomes with snapshot semantics"
```

---

### Task 3: `ForEach` in the type checker

**Files:**
- Modify: `src/Prax/TypeCheck.hs` (`outcomeUses` ~line 82, the local `inserts` ~line 171, `outcomeSents` ~line 263)
- Test: `test/Prax/TypeCheckSpec.hs`

**Interfaces:**
- Consumes: `ForEach` (Task 2), `condVars` (existing, `TypeCheck.hs:66`).
- Produces: nothing new — the existing `typeCheck :: PraxState -> [TypeError]` handles `ForEach`.

- [ ] **Step 1: Write the failing tests** (append to `test/Prax/TypeCheckSpec.hs`, mirroring its `world1`/fixture style — read the file's helpers first):

```haskell
  , testCase "a variable bound by ForEach conditions is not unbound" $ do
      let p = practice
            { practiceId = "w", roles = ["R"]
            , actions = [ action "[Actor]: broadcast" []
                            [ ForEach [ Match "member.X" ] [ Insert "told.X" ] ] ] }
      typeCheck (world1 p) @?= []

  , testCase "a genuinely unbound variable inside ForEach is flagged" $ do
      let p = practice
            { practiceId = "w", roles = ["R"]
            , actions = [ action "[Actor]: broadcast" []
                            [ ForEach [ Match "member.X" ] [ Insert "told.Ghost" ] ] ] }
      assertBool "UnboundVar Ghost"
        (any (\case UnboundVar _ "Ghost" _ -> True; _ -> False) (typeCheck (world1 p)))

  , testCase "ForEach sub-inserts join the cardinality corpus" $ do
      -- The same relation asserted '!' at top level and '.' inside a ForEach must clash.
      let p = practice
            { practiceId = "w", roles = ["R"]
            , actions = [ action "[Actor]: a" [] [ Insert "mark.R!x" ]
                        , action "[Actor]: b" []
                            [ ForEach [ Match "member.X" ] [ Insert "mark.X.y" ] ] ] }
      assertBool "CardinalityClash detected"
        (any (\case CardinalityClash {} -> True; _ -> False) (typeCheck (world1 p)))
```

(`CardinalityClash { teSlot :: String }` is the exact constructor, `TypeCheck.hs:40`.)

- [ ] **Step 2: Run, verify failures**

Run: `cabal test 2>&1 | grep -iE "ForEach|unbound|cardinality" | head`
Expected: first test may already pass or fail depending on Task 2's stopgap case; tests 2–3 FAIL (uses/corpus don't descend into `ForEach`). At least one must fail — if all three pass, the stopgap already implements this task; delete the stopgap comment and skip to Step 4 only after confirming each assertion actually exercises the new code (`git stash` the impl to watch them fail if in doubt).

- [ ] **Step 3: Implement** in `src/Prax/TypeCheck.hs`:

```haskell
outcomeUses :: Outcome -> [(String, String)]
outcomeUses (Insert s)           = [ (v, s) | v <- varsOf s ]
outcomeUses (Delete s)           = [ (v, s) | v <- varsOf s ]
outcomeUses (Call fn args)       = [ (v, fn) | a <- args, v <- varsOf a ]
outcomeUses (ForEach conds outs) =
  [ (v, s) | (v, s) <- concatMap outcomeUses outs
           , v `notElem` concatMap condVars conds ]
```

the local `inserts` (inside `assertedSentences`):

```haskell
    inserts os = [ s | Insert s <- os ] ++ concat [ inserts subs | ForEach _ subs <- os ]
```

and `outcomeSents` (sort pass — the conditions' sentences join too, since they relate positions):

```haskell
outcomeSents :: [Outcome] -> [String]
outcomeSents = concatMap go
  where
    go (Insert s)           = [s]
    go (Delete s)           = [s]
    go (Call _ _)           = []
    go (ForEach conds outs) = condSents conds ++ outcomeSents outs
```

- [ ] **Step 4: Run tests, verify pass**

Run: `cabal test 2>&1 | tail -3`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/Prax/TypeCheck.hs test/Prax/TypeCheckSpec.hs
git commit -m "Type-check ForEach: conditions bind, sub-inserts assert"
```

---

### Task 4: `ForEach` JSON round-trip

**Files:**
- Modify: `src/Prax/Script/Json.hs` (the `Outcome` instances, ~lines 116–125)
- Test: `test/Prax/Script/JsonSpec.hs`

**Interfaces:**
- Consumes: `ForEach` (Task 2); the existing `ToJSON`/`FromJSON Condition` instances.
- Produces: the wire shape `{"forEach": {"when": [...], "do": [...]}}`.

- [ ] **Step 1: Write the failing test** (append to `test/Prax/Script/JsonSpec.hs`, following its existing round-trip test style — read the file first for its helpers):

```haskell
  , testCase "a ForEach outcome round-trips through JSON" $ do
      let o = ForEach [ Match "at.Witness!P", Neq "Witness" "Actor" ]
                      [ Insert "Witness.believes.stole.Actor.loaf!seen" ]
      decode (encode o) @?= Just o
```

(Use the module's existing encode/decode helpers if it wraps aeson; otherwise `Data.Aeson.encode`/`decode`.)

- [ ] **Step 2: Run, verify it fails**

Expected: FAIL — either a non-exhaustive `toJSON` or the Task 2 stopgap's placeholder behavior.

- [ ] **Step 3: Implement** in `src/Prax/Script/Json.hs`:

```haskell
  toJSON (ForEach conds outs) =
    object [ "forEach" .= object [ "when" .= conds, "do" .= outs ] ]
```

and in the `FromJSON Outcome` alternatives:

```haskell
    <|> (o .: "forEach" >>= withObject "forEach"
           (\f -> ForEach <$> f .: "when" <*> f .: "do"))
```

- [ ] **Step 4: Run tests, verify pass**

Run: `cabal test 2>&1 | tail -3`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/Prax/Script/Json.hs test/Prax/Script/JsonSpec.hs
git commit -m "Round-trip ForEach outcomes through JSON"
```

---

### Task 5: `Prax.Witness`

**Files:**
- Create: `src/Prax/Witness.hs`
- Modify: `prax.cabal` (add `Prax.Witness` to `exposed-modules`, after `Prax.Beliefs`; add `Prax.WitnessSpec` to the test-suite `other-modules`)
- Create: `test/Prax/WitnessSpec.hs`
- Modify: `test/Spec.hs` (import + register `Prax.WitnessSpec.tests`)

**Interfaces:**
- Consumes: `ForEach` (Task 2), `beliefSentence`/`believesThat` (`Prax.Beliefs`), `Action { actionOutcomes }` (`Prax.Types`).
- Produces (for Task 6 and v20+):
  - `type CoPresence = [Condition]` — conditions over the fixed variables `"Witness"` and `"Actor"`.
  - `observable :: CoPresence -> String -> Action -> Action`
  - `saw :: String -> String -> Condition` — sugar: `saw w event = believesThat w event "seen"`.

- [ ] **Step 1: Write the failing tests.** The lowest-level API is the combinator plus one minimal inline world (per the minimal-bug-test rule; the village comes in Task 6). Create `test/Prax/WitnessSpec.hs`:

```haskell
module Prax.WitnessSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, possibleActions, performAction)
import           Prax.Witness

-- A minimal world: three located characters, one observable act.
-- Co-presence: sharing a place.
together :: CoPresence
together = [ Match "at.Actor!P", Match "at.Witness!P" ]

wave :: Action
wave = observable together "waved.Actor" $
  action "[Actor]: wave" [ Match "at.Actor!Place" ] [ Insert "waved" ]

world :: PraxState
world = foldl (flip performOutcome) base setup
  where
    base = (definePractices [p] emptyState)
             { characters = map character ["ann", "bea", "cal"] }
    p = practice { practiceId = "greet", roles = ["R"], actions = [wave] }
    setup =
      [ Insert "practice.greet.stage"
      , Insert "at.ann!square", Insert "at.bea!square", Insert "at.cal!mill" ]

-- ann performs her (only) available action.
annActs :: PraxState
annActs = case possibleActions world "ann" of
  (ga : _) -> performAction world ga
  []       -> error "wave not offered to ann"

tests :: TestTree
tests = testGroup "Prax.Witness"
  [ testCase "a co-present character comes to believe the event; an absent one doesn't" $ do
      assertBool "bea (co-present) believes it, provenance seen"
        (exists "bea.believes.waved.ann!seen" (db annActs))
      assertBool "cal (elsewhere) holds no such belief"
        (not (exists "cal.believes.waved.ann!seen" (db annActs)))

  , testCase "the actor is not their own witness" $
      assertBool "ann holds no belief about her own act"
        (not (exists "ann.believes.waved.ann!seen" (db annActs)))

  , testCase "observable only appends; the action's own effects are untouched" $ do
      actionName wave @?= "[Actor]: wave"
      take 1 (actionOutcomes wave) @?= [ Insert "waved" ]

  , testCase "saw is the seen-provenance belief condition" $
      saw "W" "waved.ann" @?= Match "W.believes.waved.ann!seen"
  ]
```

- [ ] **Step 2: Register and run, verify failure**

Add `Prax.WitnessSpec` to `prax.cabal` `other-modules` and to `test/Spec.hs` (import + list entry, next to `Prax.BeliefsSpec` — keep the file's grouping order).
Run: `cabal test 2>&1 | grep -iE "witness|error" | head`
Expected: compile error — `Prax.Witness` does not exist.

- [ ] **Step 3: Implement.** Create `src/Prax/Witness.hs` and add to `prax.cabal` `exposed-modules`:

```haskell
-- | Authored witnessing: information asymmetry from observation.
--
-- An action's public appearance is a semantic property its author states with
-- 'observable' — undeclared actions are not events (waiting is not news), and
-- the declared appearance may deliberately differ from what the action /does/
-- (poisoning the cup can look like pouring wine).
--
-- A witnessed event is an ordinary belief ("Prax.Beliefs"):
-- @\<witness\>.believes.\<event\>!seen@ — the @!seen@ value records /provenance/
-- (direct observation), so later layers can distinguish an eyewitness from
-- hearsay while all existing belief machinery works on both.
--
-- Co-presence is __world vocabulary__ (the engine has no notion of place): each
-- world supplies a 'CoPresence' template once, relating the fixed variables
-- @Witness@ and @Actor@ in its own terms.
module Prax.Witness
  ( CoPresence
  , observable
  , saw
  ) where

import           Prax.Query (Condition (..))
import           Prax.Types (Action (..), Outcome (..))
import           Prax.Beliefs (beliefSentence, believesThat)

-- | Conditions relating the fixed variables @Witness@ and @Actor@ in the
-- world's own vocabulary (location facts, current scene, …). Everything that
-- constrains who can witness is the template's job; 'observable' adds only the
-- actor-exclusion.
type CoPresence = [Condition]

-- | Declare an action's public appearance: every co-present character (except
-- the actor, who already knows what they did) comes to believe @event@ with
-- provenance @seen@. The event sentence may use the action's own variables.
observable :: CoPresence -> String -> Action -> Action
observable copresence event act =
  act { actionOutcomes =
          actionOutcomes act
            ++ [ ForEach (copresence ++ [ Neq "Witness" "Actor" ])
                         [ Insert (beliefSentence "Witness" event "seen") ] ] }

-- | Condition: @who@ directly witnessed @event@.
saw :: String -> String -> Condition
saw who event = believesThat who event "seen"
```

- [ ] **Step 4: Run tests, verify pass**

Run: `cabal test 2>&1 | tail -3` and `hlint src test 2>&1 | tail -1`
Expected: all pass; no hints.

- [ ] **Step 5: Commit**

```bash
git add src/Prax/Witness.hs test/Prax/WitnessSpec.hs test/Spec.hs prax.cabal
git commit -m "Add Prax.Witness: authored observability over ForEach"
```

---

### Task 6: `Prax.Worlds.Village` + CLI

**Files:**
- Create: `src/Prax/Worlds/Village.hs`
- Modify: `prax.cabal` (exposed-modules: `Prax.Worlds.Village` after `Prax.Worlds.Audience`; test other-modules: `Prax.VillageSpec`)
- Create: `test/Prax/VillageSpec.hs`
- Modify: `test/Spec.hs` (import + register), `test/Prax/TypeCheckSpec.hs` (add village to "every shipped world is well-formed")
- Modify: `app/Main.hs` (world wiring — two places, exactly like the `audience` wiring added in v18)

**Interfaces:**
- Consumes: `observable`, `saw`, `CoPresence` (Task 5); `adjustScore` (`Prax.Core`); the bar's `world`-practice idiom for places.
- Produces: `villageWorld :: PraxState`, `playerName :: String` (= `"you"`), `together :: CoPresence` (exported for v20/v21 to reuse).

- [ ] **Step 1: Write the failing tests.** Create `test/Prax/VillageSpec.hs`:

```haskell
module Prax.VillageSpec (tests) where

import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool)

import           Prax.Db (exists)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction)
import           Prax.Worlds.Village

-- Perform the named actor's action whose label mentions @needle@.
doAct :: String -> String -> PraxState -> PraxState
doAct who needle st =
  case [ ga | ga <- possibleActions st who, needle `isInfixOf` gaLabel ga ] of
    (ga : _) -> performAction st ga
    []       -> error ("no action for " ++ who ++ " matching " ++ show needle
                       ++ "; had: " ++ show (map gaLabel (possibleActions st who)))

tests :: TestTree
tests = testGroup "Prax.Worlds.Village"
  [ testCase "the theft is witnessed by the square, not the mill" $ do
      let st = doAct "bob" "steal the loaf" villageWorld
      assertBool "carol (in the square) saw it"
        (exists "carol.believes.stole.bob.loaf!seen" (db st))
      assertBool "you (in the square) saw it"
        (exists "you.believes.stole.bob.loaf!seen" (db st))
      assertBool "dana (at the mill) holds no such belief"
        (not (exists "dana.believes.stole.bob.loaf!seen" (db st)))
      assertBool "bob is not his own witness"
        (not (exists "bob.believes.stole.bob.loaf!seen" (db st)))

  , testCase "movement is not news (undeclared actions deposit nothing)" $ do
      let st = doAct "bob" "Go to mill" villageWorld
      assertBool "no one 'believes' bob walked"
        (not (any (\w -> exists (w ++ ".believes.went.bob!seen") (db st))
                  ["you", "carol", "dana"]))

  , testCase "only a witness can confront the thief" $ do
      let st = doAct "bob" "steal the loaf" villageWorld
      assertBool "carol can confront"
        (any (("confront bob" `isInfixOf`) . gaLabel) (possibleActions st "carol"))
      assertBool "dana cannot"
        (not (any (("confront bob" `isInfixOf`) . gaLabel) (possibleActions st "dana")))

  , testCase "confronting cools the witness toward the thief, once" $ do
      let st  = doAct "carol" "confront bob" (doAct "bob" "steal the loaf" villageWorld)
      assertBool "trust dropped"
        ("carol.relationship.bob.trust.score.-10" `elem` dbToSentences (db st))
      assertBool "confront is one-shot"
        (not (any (("confront bob" `isInfixOf`) . gaLabel) (possibleActions st "carol")))
  ]
```

(The trust assertion uses the exact storage shape `Prax.Core` documents —
`A.relationship.B.Role.score!N` — asserted the way `Prax.CoreSpec` asserts it: sentence membership
in `dbToSentences`, dot-form. Add `dbToSentences` to the `Prax.Db` import.)

- [ ] **Step 2: Register and run, verify failure**

Wire `Prax.VillageSpec` into `prax.cabal` + `test/Spec.hs`.
Run: `cabal test 2>&1 | grep -iE "village|error" | head`
Expected: compile error — `Prax.Worlds.Village` does not exist.

- [ ] **Step 3: Implement.** Create `src/Prax/Worlds/Village.hs`:

```haskell
-- | The village: the proving ground for the sandbox arc (spec
-- @docs/specs/2026-07-10-v19-witnessing-design.md@). v19 seeds it with the
-- witnessing keystone: bob steals a loaf in the square; whoever is /there/
-- comes to believe it and can act on the belief — whoever isn't, doesn't and
-- can't. Rumor (v20) and reputation (v21) grow from here.
module Prax.Worlds.Village
  ( villageWorld
  , playerName
  , together
  ) where

import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome)
import           Prax.Core (adjustScore)
import           Prax.Witness

-- | You are a villager — one agent among many.
playerName :: String
playerName = "you"

-- | Co-presence in the village: sharing a place.
together :: CoPresence
together = [ Match "practice.world.world.at.Actor!P"
           , Match "practice.world.world.at.Witness!P" ]

-- Places and movement, in the bar's idiom.
worldP :: Practice
worldP = practice
  { practiceId = "world"
  , practiceName = "The village exists"
  , roles = ["World"]
  , actions =
      [ action "[Actor]: Go to [Place]"
          [ Match "practice.world.World.at.Actor!OtherPlace"
          , Match "practice.world.World.connected.OtherPlace.Place" ]
          [ Insert "practice.world.World.at.Actor!Place" ]
      , action "[Actor]: Wait a moment"
          [ Match "practice.world.World.at.Actor!Place" ]
          []
      ]
  }

-- Village life: the theft (observable) and the belief-gated confrontation.
villageP :: Practice
villageP = practice
  { practiceId = "village"
  , practiceName = "Village life"
  , roles = ["V"]
  , actions =
      [ -- Anyone at the stall can steal — bob is merely the one who wants to.
        observable together "stole.Actor.loaf" $
        action "[Actor]: steal the loaf from the stall"
          [ Match "practice.world.world.at.Actor!square"
          , Match "stall.loaf" ]
          [ Delete "stall.loaf"
          , Insert "holding.Actor.loaf" ]

        -- Only someone who SAW the theft can call it out; it cools them toward
        -- the thief. dana, who was elsewhere, never gets this affordance.
      , action "[Actor]: confront [Thief] about the theft"
          [ saw "Actor" "stole.Thief.loaf"
          , Match "practice.world.world.at.Actor!P"
          , Match "practice.world.world.at.Thief!P"
          , Not "confronted.Actor.Thief" ]
          [ Insert "confronted.Actor.Thief"
          , adjustScore "Actor" "Thief" "trust" (-10) "sawTheft" ]
      ]
  }

villageWorld :: PraxState
villageWorld = foldl (flip performOutcome) base setup
  where
    base = (definePractices [worldP, villageP] emptyState)
             { characters =
                 [ character "you"
                 , (character "bob")
                     { charWants = [ Want [ Match "holding.bob.loaf" ] 10 ] }
                 , (character "carol")
                     { charWants = [ Want [ Match "confronted.carol.T" ] 5 ] }
                 , (character "dana")
                     { charWants = [ Want [ Match "confronted.dana.T" ] 5 ] }
                 ] }
    setup =
      [ Insert "practice.village.here"
      , Insert "practice.world.world.connected.square.mill"
      , Insert "practice.world.world.connected.mill.square"
      , Insert "practice.world.world.at.you!square"
      , Insert "practice.world.world.at.bob!square"
      , Insert "practice.world.world.at.carol!square"
      , Insert "practice.world.world.at.dana!mill"
      , Insert "stall.loaf"
      ]
```

(Spawn idioms verified against the codebase: the bar never inserts a bare `practice.world.world` —
the deeper `connected`/`at` inserts create the instance node — and `practice.village.here` mirrors
the feud's single-role `Insert "practice.society.here"`.)

Wire `app/Main.hs` exactly like the v18 `audience` wiring (same two places):

```haskell
import qualified Prax.Worlds.Village as Village
```

```haskell
worldNamed ("village" : _)  = ("the village", Village.villageWorld, Village.playerName)
```

```haskell
        ("village" : _) ->
          ( "prax — the village"
          , "You are a villager. What you see — and what you miss — decides what you can do."
          , Village.villageWorld, Village.playerName )
```

Add to `test/Prax/TypeCheckSpec.hs`'s "every shipped world is well-formed":

```haskell
      typeCheck Village.villageWorld  @?= []
```

(with `import qualified Prax.Worlds.Village as Village`).

- [ ] **Step 4: Run tests + observe the demo live**

Run: `cabal test 2>&1 | tail -3` — all pass.
Run: `cabal run -v0 prax -- check village` — expected: `well-formed: the village`.
Run: `printf 'm\nm\nm\nq\n' | cabal run -v0 prax -- village 2>&1 | head -30` — expected, visible in the narration: bob steals the loaf on his own (want-driven), carol confronts him on a later turn; the menus offered to *you* include "confront bob" only after the theft happened in front of you.
Run: `cabal run -v0 prax -- stress village 2>&1 | head -8` — no crashes; report prints (a sandbox: endings `[]` is correct).

- [ ] **Step 5: Commit**

```bash
git add src/Prax/Worlds/Village.hs test/Prax/VillageSpec.hs test/Spec.hs test/Prax/TypeCheckSpec.hs app/Main.hs prax.cabal
git commit -m "Add the village: witnessing seed world (prax village)"
```

---

### Task 7: Docs + final verification

**Files:**
- Modify: `docs/LEDGER.md` (the backlog's **K** entry), `README.md`, `docs/WALKTHROUGH.md`

**Interfaces:** none — documentation of Tasks 1–6.

- [ ] **Step 1: LEDGER.** In the "Sandbox extension backlog" section, rewrite K's status clause: replace `*(chosen first — in design)*` with `*(done — v19: `ForEach` quantified outcomes in the engine; `Prax.Witness` authored observability; `Prax.Worlds.Village` seed, CLI `prax village`)*`. Add a `- **v19** — …` line to the version legend at the top of the file, matching the existing lines' style.

- [ ] **Step 2: README.** Add a bullet after the `Prax.TypeCheck` bullet, in the established voice, covering: `ForEach` (outcomes gain their quantifier, dual of v8), `Prax.Witness` (authored observability, `!seen` provenance, co-presence as world vocabulary), and the village demo. Add `cabal run prax -- village` to the run list with a one-line description.

- [ ] **Step 3: WALKTHROUGH.** Add `### 22. Witnessing — who knows what (`prax village`) (v19)` at the end of Part II, in the established style: the steal-in-the-square scene, the carol/dana asymmetry as shown in actual play output, the authored-observability principle (movement is not news), and the → code/spec pointers. Add one coverage-map row (`ForEach` + witnessing | `Prax.Engine` / `Prax.Witness` | `prax village`) and `Prax.WitnessSpec`/`Prax.VillageSpec` to the test-pointer paragraph. Base every quoted line on output actually captured from `prax village`, not memory.

- [ ] **Step 4: Full verification**

```bash
cabal build 2>&1 | grep -i warning || echo "-Wall clean"
cabal test 2>&1 | tail -3
hlint src app test 2>&1 | tail -1
for w in bar intrigue play feud audience village dm; do cabal run -v0 prax -- check "$w"; done
```

Expected: clean build, all tests green (≥190), "No hints", every world well-formed.

- [ ] **Step 5: Commit and push**

```bash
git add docs/LEDGER.md README.md docs/WALKTHROUGH.md
git commit -m "Document v19: quantified outcomes + witnessing"
git push
```
