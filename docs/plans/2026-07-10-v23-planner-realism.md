# v23 — Realistic Lookahead Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the optimistic-omniscient lookahead with the round-walk over believed minds, per `docs/specs/2026-07-10-v23-planner-realism-design.md`: a named desire vocabulary (`Prax.Minds`), motive-beliefs riding the existing information stack, derivation-based common knowledge, sightings (`Prax.Sight`), and an epistemic prediction scope.

**Architecture:** Two new compiled modules (`Prax.Minds`, `Prax.Sight` — the latter on the v18 clock idiom, zero engine surface), three new `Prax.Types` fields (`Desire`, `desires`, `charDesires`, `predictionScope`), and a rewrite of `Prax.Planner`'s lookahead (`worldValue` deleted; `predictMove` added). Only Intrigue migrates content; village/bar gain sight tickers and scopes.

**Tech Stack:** Haskell (GHC 9.10, cabal), tasty/tasty-hunit (existing deps only).

## Pre-flight (controller does this before Task 1 — recorded here for the ledger)

The working tree holds parked v22 Task-2 content. Park it on a branch, then clean master:
`git checkout -b v22-wip && git add -A && git commit -m "WIP: v22 Task 2 parked pending v23"` then
`git checkout master` (tree clean, HEAD unchanged). v22 resumes on top of v23 later.

## Global Constraints

- `cabal build` must stay `-Wall`-clean; `hlint src app test` must report "No hints".
- TDD: failing test first, observed failing, then implementation, observed passing.
- No heuristics, no magic numbers (horizons/weights are authored world parameters with stated
  meaning — never tuned; BLOCKED with a trace on any behavioral surprise), no placeholder code,
  no skipped tests, no mocks.
- Reserved variables (document in haddocks): `Owner` (desire templates), `Seer`/`Seen`/`Spot`
  (sighting templates), `Actor`/`Witness` (scope templates, as in v19).
- Behavioral regression: the full suite green. Where a test *encodes the old planner arithmetic*
  (the two `worldValue` lines) it is REWRITTEN to assert the new specified semantics with the
  arithmetic derived in comments; where a test encodes a *story*, the story must still hold —
  never tune it into place; BLOCKED with a trace if it breaks.
- Commit after each green task with the trailer:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: `Desire` + `Prax.Minds`

**Files:**
- Modify: `src/Prax/Types.hs` (add `Desire`, `desires`, `charDesires`)
- Create: `src/Prax/Minds.hs`
- Create: `test/Prax/MindsSpec.hs`
- Modify: `prax.cabal` (`Prax.Minds` in exposed-modules after `Prax.Deceit`; `Prax.MindsSpec` in test other-modules), `test/Spec.hs` (import + register)

**Interfaces:**
- Consumes: `groundCondition` (`Prax.Query`), `Val (..)`/`exists` (`Prax.Db`), `readView` (`Prax.Engine`), `Axiom`/`axiom` (`Prax.Derive`).
- Produces (Tasks 3–5): `Desire (..)` (in `Prax.Types`), and from `Prax.Minds`:
  - `wantFor :: String -> Desire -> Want`
  - `selfWants :: PraxState -> Character -> [Want]`
  - `believedWants :: PraxState -> Character{-predictor-} -> Character{-mover-} -> [Want]`
  - `professed :: Axiom`, `conventional :: Axiom`

- [ ] **Step 1: Types.** In `src/Prax/Types.hs`:

```haskell
-- | A nameable desire: a 'Want' whose conditions may use the reserved variable
-- @Owner@, instantiated per character ('Prax.Minds.wantFor'). Naming a desire is
-- what makes it a possible object of belief.
data Desire = Desire
  { desireName :: String
  , desireWant :: Want
  }
  deriving (Eq, Show)
```

`Character` gains `charDesires :: [String]` (after `charWants`); the `character` constructor
sets it `[]`. `PraxState` gains `desires :: [Desire]` and `predictionScope :: [Condition]`,
both defaulted in `emptyState` (`[]` / `[]`). Export `Desire (..)`.
(`predictionScope` lands now so Types is touched once; the Planner consumes it in Task 3.)

- [ ] **Step 2: Write the failing tests.** Create `test/Prax/MindsSpec.hs`:

```haskell
module Prax.MindsSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, readView)
import           Prax.Minds

-- The tale: a vocabulary of two desires; ida professes her sweet tooth,
-- norm-respect is conventional, and rex's grudge is neither.
vocab :: [Desire]
vocab =
  [ Desire "sweet-tooth" (Want [ Match "holding.Owner.cake" ] 5)
  , Desire "grudge-rex"  (Want [ Match "shamed.rex" ] 7)
  ]

world :: PraxState
world = (foldl (flip performOutcome) base setup)
          { axioms  = [ professed, conventional ]
          , desires = vocab }
  where
    base = (definePractices [] emptyState)
             { characters = [ character "ida"
                            , (character "rex") { charDesires = ["grudge-rex"] } ] }
    setup =
      [ Insert "character.ida", Insert "character.rex"
      , Insert "professes.ida.sweet-tooth" ]

tests :: TestTree
tests = testGroup "Prax.Minds"
  [ testCase "wantFor grounds the Owner variable" $
      wantFor "ida" (Desire "sweet-tooth" (Want [ Match "holding.Owner.cake" ] 5))
        @?= Want [ Match "holding.ida.cake" ] 5

  , testCase "selfWants = unnamed wants + own named desires, instantiated" $ do
      let rex = (character "rex") { charWants   = [ Want [ Match "x" ] 1 ]
                                  , charDesires = ["grudge-rex"] }
      selfWants world rex
        @?= [ Want [ Match "x" ] 1, Want [ Match "shamed.rex" ] 7 ]

  , testCase "a profession derives presumed motive-beliefs across the cast" $ do
      let v = readView world
      assertBool "rex presumes ida's sweet tooth"
        (exists "rex.believes.desires.ida.sweet-tooth.presumed" v)
      assertBool "nothing derives rex's unprofessed grudge"
        (not (exists "ida.believes.desires.rex.grudge-rex.presumed" v))

  , testCase "the profession is defeasible" $ do
      let w' = performOutcome (Delete "professes.ida.sweet-tooth") world
      assertBool "presumption dissolved"
        (not (exists "rex.believes.desires.ida.sweet-tooth.presumed" (readView w')))

  , testCase "a conventional desire is presumed of everyone — even non-holders" $ do
      let w' = performOutcome (Insert "conventional.sweet-tooth") world
          v  = readView w'
      assertBool "ida presumes rex's sweet tooth (he does not have one)"
        (exists "ida.believes.desires.rex.sweet-tooth.presumed" v)

  , testCase "believedWants reads any provenance, and only believed desires" $ do
      believedWants world (character "ida") (character "rex") @?= []
      let w' = performOutcome
                 (Insert "ida.believes.desires.rex.grudge-rex.heard.sam") world
      believedWants w' (character "ida") (character "rex")
        @?= [ Want [ Match "shamed.rex" ] 7 ]
      -- and presumption counts too:
      believedWants world (character "rex") (character "ida")
        @?= [ Want [ Match "holding.ida.cake" ] 5 ]
  ]
```

- [ ] **Step 3: Register and run, verify RED** (compile error — no `Prax.Minds`, no `Desire`).

- [ ] **Step 4: Implement.** Create `src/Prax/Minds.hs`:

```haskell
-- | Minds as objects of belief.
--
-- To believe something about a mind, minds must be nameable: a world declares
-- a vocabulary of named, @Owner@-parameterized 'Desire's, and a motive-belief
-- is an ordinary belief over the issue @desires.\<owner\>.\<name\>@ in the v20
-- provenance shape (@.seen@ \/ @.heard.\<src\>@ \/ @.presumed@) — so the whole
-- information stack (gossip, lies, confides, forgetting, derivation) works on
-- minds unchanged. An unnamed 'charWants' want has no name to believe and is
-- therefore inherently unreadable (right for the story manager's metalevel
-- desires). Common knowledge is /derived/, defeasibly: 'professed' spreads a
-- character's openly-held desire; 'conventional' presumes a desire of everyone
-- — even of those who do not actually have it (you expect strangers to be
-- conventional, and can be wrong).
module Prax.Minds
  ( wantFor
  , selfWants
  , believedWants
  , professed
  , conventional
  ) where

import qualified Data.Map.Strict as Map

import           Prax.Db (Val (..), exists)
import           Prax.Query (Condition (..), groundCondition)
import           Prax.Types
import           Prax.Engine (readView)
import           Prax.Derive (Axiom, axiom)

-- | Instantiate a desire template for its owner (grounds @Owner@).
wantFor :: String -> Desire -> Want
wantFor owner (Desire _ (Want cs u)) =
  Want (map (groundCondition (Map.singleton "Owner" (VStr owner))) cs) u

-- | What a character plans with: their whole mind — unnamed wants plus their
-- own named desires, instantiated.
selfWants :: PraxState -> Character -> [Want]
selfWants st c =
  charWants c
    ++ [ wantFor (charName c) d
       | d <- desires st, desireName d `elem` charDesires c ]

-- | The predictor's believed model of the mover: every vocabulary desire the
-- predictor believes (any provenance) the mover to have. The model can be
-- wrong — it is the predictor's, not the mover's.
believedWants :: PraxState -> Character -> Character -> [Want]
believedWants st p m =
  [ wantFor (charName m) d
  | d <- desires st
  , exists (charName p ++ ".believes.desires." ++ charName m ++ "." ++ desireName d) view ]
  where view = readView st

-- | An openly-held desire is presumed known by everyone:
-- @professes.\<owner\>.\<name\>@ ⇒ every character presumes it.
professed :: Axiom
professed = axiom
  [ Match "professes.Owner.D", Match "character.P" ]
  [ "P.believes.desires.Owner.D.presumed" ]

-- | A conventional desire is presumed of everyone by everyone — even of those
-- who do not actually have it.
conventional :: Axiom
conventional = axiom
  [ Match "conventional.D", Match "character.P", Match "character.M" ]
  [ "P.believes.desires.M.D.presumed" ]
```

- [ ] **Step 5: GREEN + gates.** Full suite (243 + 6 new = expect 249 — the current count is
  243 on clean master; verify and report the true baseline first), `-Wall` clean, hlint clean.

- [ ] **Step 6: Commit** (`git add src/Prax/Types.hs src/Prax/Minds.hs test/Prax/MindsSpec.hs test/Spec.hs prax.cabal`), message: `Add Prax.Minds: nameable desires, motive-beliefs, derived common knowledge`.

---

### Task 2: `Prax.Sight`

**Files:**
- Create: `src/Prax/Sight.hs`
- Create: `test/Prax/SightSpec.hs`
- Modify: `prax.cabal` + `test/Spec.hs` (registration, next to Minds)

**Interfaces:**
- Consumes: `ForEach` (v19), the v18 clock idiom, `Calc`/`Cmp` conditions.
- Produces (Task 5): `sightName :: String`, `sightP :: [Condition] -> Practice` (sighting
  template over `Seer`/`Seen`/`Spot`), `sightChar :: Character`, `sightSetup :: [Outcome]`,
  `sightedWithin :: Int -> [Condition]` (scope fragment over `Actor`/`Witness`).

- [ ] **Step 1: Write the failing tests.** Create `test/Prax/SightSpec.hs`:

```haskell
module Prax.SightSpec (tests) where

import           Data.List (isInfixOf)
import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists, unify, valToString)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, possibleActions, performAction)
import           Prax.Sight

-- Two rooms; ute and vic share one, wes is alone in the other.
sighting :: [Condition]
sighting = [ Match "at.Seer!Spot", Match "at.Seen!Spot" ]

world :: PraxState
world = foldl (flip performOutcome) base (sightSetup ++ setup)
  where
    base = (definePractices [sightP sighting] emptyState)
             { characters = map character ["ute", "vic", "wes"] ++ [sightChar] }
    setup = [ Insert "at.ute!hall", Insert "at.vic!hall", Insert "at.wes!attic" ]

-- One tick of the perception clock.
tick :: PraxState -> PraxState
tick st = case possibleActions st sightName of
  (ga : _) -> performAction st ga
  []       -> error "the sight ticker has no action"

tests :: TestTree
tests = testGroup "Prax.Sight"
  [ testCase "the ticker advances the world turn" $ do
      assertBool "turn 0 at setup" (exists "turn!0" (db world))
      assertBool "turn 1 after a tick" (exists "turn!1" (db (tick world)))
      assertBool "turn 2 after two" (exists "turn!2" (db (tick (tick world))))

  , testCase "co-presence deposits sightings, both ways; the absent see nothing" $ do
      let st = tick world
      assertBool "ute sighted vic in the hall" (exists "ute.believes.at.vic!hall" (db st))
      assertBool "vic sighted ute" (exists "vic.believes.at.ute!hall" (db st))
      assertBool "stamped with the turn" (exists "ute.believes.atSince.vic!1" (db st))
      assertBool "nobody sighted wes" (not (exists "ute.believes.at.wes" (db st)))
      assertBool "wes sighted nobody" (not (exists "wes.believes.at.ute" (db st)))

  , testCase "a sighting persists after separation, and a new one overwrites it" $ do
      let st1 = tick world                                        -- ute sees vic in hall
          st2 = tick (performOutcome (Insert "at.vic!attic") st1) -- vic left; tick again
      assertBool "ute still believes vic is in the hall (stale)"
        (exists "ute.believes.at.vic!hall" (db st2))
      assertBool "and the stamp did not refresh" (exists "ute.believes.atSince.vic!1" (db st2))
      -- ute follows and re-sights: overwrite
      let st3 = tick (performOutcome (Insert "at.ute!attic") st2)
      assertBool "belief updated" (exists "ute.believes.at.vic!attic" (db st3))
      assertBool "old belief gone" (not (exists "ute.believes.at.vic!hall" (db st3)))
      assertBool "stamp refreshed" (exists "ute.believes.atSince.vic!3" (db st3))

  , testCase "sightedWithin is a window over the stamp" $ do
      let holds h st = not (null (unify' h st))
          unify' h st =
            Prax.Db.unify "x" (db st) Map.empty `seq`   -- (placeholder removed in impl step)
            []
      -- The real assertion, via a direct query of the fragment:
      let q h st = not (null (queryScope h st))
      True @?= True
  ]
```

The last case above is deliberately sketched: implement it as a direct `Prax.Query.query` of
`map (groundCondition (Map.fromList [("Actor", VStr "ute"), ("Witness", VStr "vic")]))
(sightedWithin 2)` against `readView` — asserting it HOLDS right after a sighting, still holds
after 2 further ticks, and FAILS after a 3rd (the window lapsed). Write it as real code in the
test file (the plan sketches it to avoid duplicating the import dance; the assertions and the
window arithmetic — sighted at turn 1, expiry 1+2=3, fails at turn 4 — are the requirements).

- [ ] **Step 2: Register, run, verify RED.**

- [ ] **Step 3: Implement.** Create `src/Prax/Sight.hs`:

```haskell
-- | Sightings: knowing where people are is itself information.
--
-- A bodiless per-round ticker (the v18 clock idiom — zero engine surface)
-- advances a global turn counter @turn!N@ and, via 'ForEach' over the world's
-- sighting template (reserved variables @Seer@\/@Seen@\/@Spot@), refreshes
-- location-beliefs for every co-present pair:
--
-- > <seer>.believes.at.<seen>!<place>      -- best guess (single-slot: overwritten)
-- > <seer>.believes.atSince.<seen>!<turn>  -- when it was formed
--
-- Sightings persist after separation ("last known location"), and
-- 'sightedWithin' turns the stamp into a prediction-scope window: the horizon
-- is an authored world parameter with stated meaning, not an engine constant.
module Prax.Sight
  ( sightName
  , sightP
  , sightChar
  , sightSetup
  , sightedWithin
  ) where

import           Prax.Query (Condition (..), CmpOp (..), CalcOp (..))
import           Prax.Types

-- | The ticker's name (bodiless; bound to its practice; blank label, so the
-- CLI's silent-action suppression hides it).
sightName :: String
sightName = "_sight"

-- | The perception clock: one tick per round.
sightP :: [Condition] -> Practice
sightP sighting = practice
  { practiceId = "sight"
  , practiceName = "time passes and people see each other"
  , roles = ["S"]
  , actions =
      [ action ""
          [ Eq "Actor" sightName
          , Match "turn!N"
          , Calc "M" Add "N" "1" ]
          [ Insert "turn!M"
          , ForEach (sighting ++ [ Neq "Seer" "Seen" ])
              [ Insert "Seer.believes.at.Seen!Spot"
              , Insert "Seer.believes.atSince.Seen!M" ]
          ]
      ]
  }

sightChar :: Character
sightChar = (character sightName) { charBoundTo = Just "sight" }

sightSetup :: [Outcome]
sightSetup = [ Insert "practice.sight.here", Insert "turn!0" ]

-- | Scope fragment over @Actor@\/@Witness@: the Witness was sighted within the
-- last @h@ ticks. Worlds @Or@ this with co-presence-now in their
-- 'predictionScope'.
sightedWithin :: Int -> [Condition]
sightedWithin h =
  [ Match "Actor.believes.atSince.Witness!Since"
  , Match "turn!Now"
  , Calc "Expiry" Add "Since" (show h)
  , Cmp Gte "Expiry" "Now" ]
```

- [ ] **Step 4: GREEN + gates** (suite, `-Wall`, hlint). Also confirm `prax check` accepts a
  fixture world using the ticker (the SightSpec world through `typeCheck` in one extra
  assertion, or note why not needed).

- [ ] **Step 5: Commit**: `Add Prax.Sight: turn clock + sighting beliefs (perception, compiled)`.

---

### Task 3: The Planner rewrite

**Files:**
- Rewrite: `src/Prax/Planner.hs`
- Rewrite/extend: `test/Prax/PlannerSpec.hs` (the two `worldValue` lines and module docs; keep every story-level test)

**Interfaces:**
- Consumes: `selfWants`/`believedWants` (Task 1), `predictionScope` (Task 1's Types field), `groundCondition`, `query`, `readView`.
- Produces: same API minus `worldValue`, plus
  `predictMove :: PraxState -> Character -> Character -> Maybe GroundedAction`.

- [ ] **Step 1: Write the failing tests.** In `test/Prax/PlannerSpec.hs`, REPLACE the
  `worldValue` assertions (lines ~82–83) with the same story asserted through the new API, and
  append the new-semantics cases. The fixture gains a second character where needed. New cases
  (complete code; adjust the fixture names to the file's style):

```haskell
  , testCase "lookahead: walking up is worthless immediately but valuable at depth 1" $ do
      -- score(walk) at depth 1 = eval(after walk = 0) + others (none believed: 0)
      --                        + 0.9 × best-next (order → 10) = 9.0   [§4 arithmetic]
      let scored = scoreActions 1 barState bethWantsCider
      lookup "beth: Walk up to bar" [ (gaLabel a, s) | (a, s) <- scored ] @?= Just 9.0
      fmap gaLabel (pickAction 1 barState bethWantsCider) @?= Just "beth: Walk up to bar"

  , testCase "predictMove is belief-relative: no belief, no prediction" $ do
      -- ada (a fresh character) holds no motive-beliefs about beth
      predictMove walkedUp (character "ada") bethWantsCider @?= Nothing

  , testCase "predictMove with a believed motive is the mover's motivated best" $ do
      let vocab = [ Desire "cider-craving"
                      (Want [ Match "practice.tendBar.Bartender.customer.Owner!order!cider" ] 10) ]
          beth' = (character "beth") { charDesires = ["cider-craving"] }
          st    = (walkedUp { desires = vocab
                            , characters = [ beth', character "ada" ] })
          st'   = performOutcome
                    (Insert "ada.believes.desires.beth.cider-craving.heard.gossip") st
      fmap gaLabel (predictMove st' (character "ada") beth') @?= Just "beth: Order cider"
      -- and motivated-only: a believed mind with nothing to gain predicts still
      let satisfied = performOutcome
                        (Insert "practice.tendBar.ada.customer.beth!order!cider") st'
      predictMove satisfied (character "ada") beth' @?= Nothing

  , testCase "a false belief predicts a move the mover would never take" $ do
      -- ada believes beth craves cider; beth actually wants nothing.
      let vocab = [ Desire "cider-craving"
                      (Want [ Match "practice.tendBar.Bartender.customer.Owner!order!cider" ] 10) ]
          plainBeth = character "beth"
          st  = walkedUp { desires = vocab, characters = [ plainBeth, character "ada" ] }
          st' = performOutcome
                  (Insert "ada.believes.desires.beth.cider-craving.presumed") st
      fmap gaLabel (predictMove st' (character "ada") plainBeth) @?= Just "beth: Order cider"
      pickAction 0 st' plainBeth @?= pickAction 0 st plainBeth   -- beth herself is unmoved

  , testCase "the round-walk credits a predicted enabling world (secret coordination)" $ do
      -- Conspirators: 'inge' will grab the relic once the gate is open (her
      -- desire is in the vocabulary); 'olaf' wants the relic grabbed and can
      -- open the gate. Olaf takes the enabling move IFF he is in on her motive.
      let vocab = [ Desire "covet-relic" (Want [ Match "grabbed.Owner" ] 10) ]
          grabP = practice
            { practiceId = "heist", roles = ["R"]
            , actions =
                [ action "[Actor]: grab the relic"
                    [ Match "gate.open", Not "grabbed.inge", Eq "Actor" "inge" ]
                    [ Insert "grabbed.inge" ]
                , action "[Actor]: open the gate"
                    [ Eq "Actor" "olaf", Not "gate.open" ]
                    [ Insert "gate.open" ]
                , action "[Actor]: Wait about"
                    [] [] ]
            }
          inge = (character "inge") { charDesires = ["covet-relic"] }
          olaf = (character "olaf") { charWants = [ Want [ Match "grabbed.inge" ] 6 ] }
          st0  = (foldl (flip performOutcome)
                    ((definePractices [grabP] emptyState)
                       { characters = [ olaf, inge ], desires = vocab })
                    [ Insert "practice.heist.here" ])
          told = performOutcome
                   (Insert "olaf.believes.desires.inge.covet-relic.heard.inge") st0
      fmap gaLabel (pickAction 1 told olaf) @?= Just "olaf: open the gate"
      assertBool "not in on it: opening the gate gains him nothing"
        (fmap gaLabel (pickAction 1 st0 olaf) /= Just "olaf: open the gate")

  , testCase "the round is sequential: the second prediction sees the first's effects" $ do
      -- Design a 3-character chain in the fixture style: A's candidate enables B's
      -- predicted move, which enables C's predicted move, which satisfies A's want;
      -- assert A's depth-1 score credits the chain ONLY when both motive-beliefs are
      -- held. (Write the fixture concretely following the heist pattern above.)
      assertBool "write as described" True

  , testCase "prediction scope gates participation" $ do
      -- reuse the heist: with a scope template requiring a shared room and the
      -- conspirators in different rooms, olaf no longer credits inge's move even
      -- though he holds the motive-belief; empty scope = everyone (the default
      -- already exercised above).
      assertBool "write as described" True
  ]
```

The last two cases are specified by construction recipe rather than verbatim code — write them
as real, complete fixtures per their comments (the heist pattern shows every ingredient). A
placeholder `assertBool "write as described" True` appearing in the committed test file is a
plan violation — the committed tests must implement the described assertions.

- [ ] **Step 2: RED** (compile errors: `predictMove` missing; `worldValue` gone from spec's rewrite).

- [ ] **Step 3: Implement.** Rewrite `src/Prax/Planner.hs`:

```haskell
-- | Utility-based action selection (Versu §IX) with a beyond-source lookahead
-- extension, redesigned in v23 (spec: docs/specs/2026-07-10-v23-planner-realism-design.md).
--
-- Selection is the paper's apply-and-evaluate: score each candidate by the
-- world it produces. The lookahead is a __round-walk over believed minds__:
-- one imagined round in which each other character within the actor's
-- 'predictionScope' takes one /motivated/ move chosen from the actor's
-- __believed model__ of them ("Prax.Minds" — which can be wrong), followed by
-- the actor's own next choice, recursively. Discounts: 0.9 own future move,
-- 0.5 another's. Accumulation is a discounted stream of absolute utilities
-- over the imagined round. Unknown minds and out-of-scope characters are
-- modeled as still — never as helpful.
module Prax.Planner
  ( evaluate
  , candidateActions
  , predictMove
  , scoreActions
  , pickAction
  ) where

import           Data.List (sortOn)
import qualified Data.Map.Strict as Map
import           Data.Maybe (listToMaybe)
import           Data.Ord (Down(..))

import           Prax.Db (Val (..))
import           Prax.Query (countSatisfying, groundCondition, query)
import           Prax.Types
import           Prax.Engine (readView, possibleActions, performAction)
import           Prax.Minds (selfWants, believedWants)

-- | Total utility of a world to a set of wants: @Σ utility × #satisfying@.
evaluate :: PraxState -> [Want] -> Int
evaluate st wants =
  sum [ wantUtility w * countSatisfying view (wantConditions w) Map.empty
      | w <- wants ]
  where view = readView st

-- | The actions a character may actually take (practice-bound filtering).
candidateActions :: PraxState -> Character -> [GroundedAction]
candidateActions st c =
  let as = possibleActions st (charName c)
  in case charBoundTo c of
       Nothing  -> as
       Just pid -> filter ((== pid) . gaPracticeId) as

-- | Is the mover within the actor's prediction scope? The world's template
-- (over @Actor@/@Witness@) is grounded to the pair and queried against the
-- view; the empty template means everyone.
inScope :: PraxState -> Character -> Character -> Bool
inScope st actor m =
  not (null (query (readView st) grounded Map.empty))
  where
    grounded = map (groundCondition binds) (predictionScope st)
    binds = Map.fromList [ ("Actor",   VStr (charName actor))
                         , ("Witness", VStr (charName m)) ]

-- | The predictor's guess at the mover's next move: the mover's best candidate
-- under the predictor's believed model of them — and only if it strictly
-- improves that model over doing nothing (unmotivated moves are noise, not
-- plan). 'Nothing' when the mind is unreadable or unmotivated.
predictMove :: PraxState -> Character -> Character -> Maybe GroundedAction
predictMove st p m =
  case believedWants st p m of
    []    -> Nothing
    model ->
      let still  = evaluate st model
          scored = sortOn (\(ga, s) -> (Down s, gaLabel ga))
                     [ (a, evaluate (performAction st a) model)
                     | a <- candidateActions st m ]
      in case scored of
           ((a, s) : _) | s > still -> Just a
           _                        -> Nothing

-- The other living characters, one full cycle in cast order starting after
-- the actor (the loop's round-robin order).
othersAfter :: PraxState -> Character -> [Character]
othersAfter st actor =
  filter ((/= charName actor) . charName) (drop (i + 1) cs ++ take (i + 1) cs)
  where
    cs = livingCharacters st
    i  = case [ k | (k, c) <- zip [0 :: Int ..] cs, charName c == charName actor ] of
           (k : _) -> k
           []      -> length cs - 1   -- an actor outside the cast walks everyone

-- | Score each candidate by the imagined round it opens (best first; ties
-- broken by label for determinism).
scoreActions :: Int -> PraxState -> Character -> [(GroundedAction, Double)]
scoreActions depth st actor =
  sortOn (\(ga, s) -> (Down s, gaLabel ga))
    [ (a, valueAfter depth (performAction st a)) | a <- candidateActions st actor ]
  where
    valueAfter d st1 = base + rest
      where
        base = fromIntegral (evaluate st1 (selfWants st1 actor))
        rest
          | d <= 0    = 0
          | otherwise = othersScore + selfNext
          where
            (afterRound, othersScore) = foldl step (st1, 0) (othersAfter st1 actor)
            step (s, acc) m
              | not (inScope s actor m) = (s, acc)
              | otherwise = case predictMove s actor m of
                  Nothing -> (s, acc)
                  Just ga ->
                    let s' = performAction s ga
                    in (s', acc + 0.5 * fromIntegral (evaluate s' (selfWants s' actor)))
            selfNext = case scoreActions (d - 1) afterRound actor of
              ((_, v) : _) -> 0.9 * v
              []           -> 0

-- | The actor's best action (deterministic), if any.
pickAction :: Int -> PraxState -> Character -> Maybe GroundedAction
pickAction depth st actor = fst <$> listToMaybe (scoreActions depth st actor)
```

`worldValue` is gone — no wrapper, no deprecation. (Its only external uses were the two
PlannerSpec lines rewritten in Step 1; verify with `grep -rn worldValue src app test`.)

- [ ] **Step 4: GREEN + full regression.** Entire suite must pass. Every failure gets read
  before it gets touched: story tests that fail are BLOCKED-with-trace material (the spec argues
  none should — shipped stories are self-want-driven); only old-arithmetic assertions may be
  rewritten. `-Wall`/hlint clean; `prax check` all worlds.

- [ ] **Step 5: Commit**: `Rewrite the planner: round-walk lookahead over believed minds`.

---

### Task 4: Intrigue migrates; theory-of-mind integration tests

**Files:**
- Modify: `src/Prax/Worlds/Intrigue.hs`
- Modify: `test/Prax/IntrigueSpec.hs` (append), `test/Prax/MindsSpec.hs` (append the motive-gossip case)

**Interfaces:** consumes Tasks 1+3; produces the spec §5 Intrigue behavior.

- [ ] **Step 1: Failing tests.** Append to `test/Prax/IntrigueSpec.hs` (match its helpers — it
  has a doAct-style runner; read the file first):

```haskell
  , testCase "the confidant can foresee the poisoning; the victim cannot" $ do
      -- cassia confides in marcus (the existing plot action), then:
      --   predictMove st marcus cassia = the poisoning (or its enabling move)
      --   predictMove st artus  cassia = Nothing
      -- Build st by performing the confide via the existing action-label helper.
      (write with the file's real helpers; the two assertions above are verbatim requirements)

  , testCase "a leaked motive changes who can see the plan" $ do
      -- Plant artus.believes.desires.cassia.kill-artus.heard.marcus directly
      -- (performOutcome Insert — the rumor fixture lives in MindsSpec); now
      -- predictMove st artus cassia is a move, not Nothing.
```

And to `test/Prax/MindsSpec.hs`, the motive-gossip smoke: a small fixture where
`gossip together [] "desires.Culprit.grudge-rex" "..."` spreads a motive-belief whose arrival
flips a third character's `predictMove` from `Nothing` to a move. (Complete fixture in the
file; reuse the MindsSpec vocabulary.)

- [ ] **Step 2: RED.**

- [ ] **Step 3: Implement.** In `src/Prax/Worlds/Intrigue.hs`:
  - Add `desires = [ Desire "kill-artus" (Want [ Match (deadSentence "artus") ] 100) ]` to the
    world's state (field on the constructed `PraxState`, feud-style record update).
  - cassia: `charWants` entry for the murder REMOVED; `charDesires = ["kill-artus"]`.
  - The confide action's outcomes gain
    `Insert "Ally.believes.desires.Schemer.kill-artus.heard.Schemer"` (beside the existing
    `practice.plot...confided` insert; `Ally`/`Schemer` are its existing variables).
  - The world asserts `character.<n>` facts if it doesn't already (check; the confide's
    `Match "character.Ally"` suggests it does).

- [ ] **Step 4: GREEN + regression** (IntrigueSpec's existing plot-runs-to-betrayal must hold —
  cassia's selfWants are unchanged in content). Gates as usual.

- [ ] **Step 5: Commit**: `Intrigue: the plot is a believed mind — confide shares it, prediction respects it`.

---

### Task 5: Sight + scope wiring (village and bar)

**Files:**
- Modify: `src/Prax/Worlds/Village.hs`, `src/Prax/Worlds/Bar.hs`
- Modify: `test/Prax/VillageSpec.hs`, `test/Prax/BarSpec.hs`/`test/Prax/LoopSpec.hs` (only as behavior legitimately shifts — see Step 3)

**Interfaces:** consumes Task 2; produces spec §5's wired scopes.

- [ ] **Step 1: Failing tests.** VillageSpec appends:

```haskell
  , testCase "the village keeps a perception clock and sightings" $ do
      -- after one full round of driveIdle, turn has advanced and square-mates
      -- hold sightings of each other; dana (at the mill) holds none of bob.

  , testCase "out of sight, out of mind: an unsighted mover is not predicted" $ do
      -- assert via predictMove + a planted motive-belief: dana holding a
      -- motive-belief about bob but no sighting within the horizon does not
      -- predict him; after co-presence (one shared-room tick) she does.
```

(Write both concretely against the village's vocabulary; the assertions in the comments are
the requirements.)

- [ ] **Step 2: RED.**

- [ ] **Step 3: Implement.**
  - Village: practices gain `sightP villageSighting` where
    `villageSighting = [ Match "practice.world.world.at.Seer!Spot", Match "practice.world.world.at.Seen!Spot" ]`;
    cast gains `sightChar`; setup gains `sightSetup`; and the state gains
    `predictionScope = [ Or [ together, sightedWithin 2 ] ]` — horizon **2 ticks**, authored:
    one tick per round, and two rounds is roughly a square↔mill round trip — "you assume
    people stay put for about as long as it takes to walk there and back."
  - Bar (`barWorld` and `barDirectorWorld`): same pattern over its `at` vocabulary
    (entrance/bar), same stated horizon rationale (one room away).
  - **Cast-size effects, handled honestly:** the ticker adds one turn per round, so
    `driveIdle`-style counts cover fewer rounds, and `LoopSpec`'s deterministic replay may
    shift. Turn-count raises are test parameters (state them); `LoopSpec`'s expected
    *sequence* may be re-derived from the observed trace ONLY where the story it asserts
    (greet → serve → take-offense → buy; the director beat; bex's belonging) still visibly
    happens — otherwise BLOCKED with the trace.

- [ ] **Step 4: GREEN + full regression + live observation.** Suite green; `prax check` all
  worlds; play a few turns of village and bar and capture (scene lines should be unchanged —
  sightings are beliefs, not scene lines; the ticker is silent).

- [ ] **Step 5: Commit**: `Wire perception and epistemic prediction scopes into the village and bar`.

---

### Task 6: Docs + final verification

**Files:** `docs/LEDGER.md`, `README.md`, `docs/WALKTHROUGH.md`

- [ ] **Step 1: LEDGER.** Legend `- **v23** — …` line. Rewrite row **#20** honestly: the
  lookahead is a beyond-source extension, now a round-walk over believed minds within an
  epistemic scope (spec reference); note `worldValue`'s optimistic model is gone. Backlog:
  mark the calendar item as partially seeded (`turn!N` exists); bank counterfactual placement
  and recency-salience notes (per spec residuals); check rows #18/#21/#23 for stale claims.
- [ ] **Step 2: README.** v23 bullet after v22's… v22 has no bullet yet (parked) — place after
  the v21 bullet, noting v22-in-progress ordering honestly, or renumber when v22 resumes;
  content: `Prax.Minds` + `Prax.Sight` + the planner redesign, in the established voice.
- [ ] **Step 3: WALKTHROUGH.** Grep and correct any wording the redesign falsified — check
  "looking two moves ahead" (§1) and "the planner sees that the violation→disapproval future
  scores far worse" (§8): re-verify each mechanism live and reword to the true mechanism if
  needed (bex's own −40 aversion at depth 0). Add a short §1-adjacent note that NPCs anticipate
  only minds they have beliefs about and people they know are around.
- [ ] **Step 4: Full verification** (build/-Wall, suite, hlint, `prax check` on all seven
  worlds) — real output in the report.
- [ ] **Step 5: Commit** (`Document v23: believed minds, sightings, honest lookahead`), do NOT
  push (controller pushes after final review).
