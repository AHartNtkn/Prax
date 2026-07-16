module Prax.ClockSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, possibleActions,
                               performAction, setCharacters)
import           Prax.TypeCheck (typeCheck)
import           Prax.Drift
import           Prax.Clock

-- A drift-only world: the standalone clock ticks 'turn'; drift reads it
-- through 'turnPath'. No sight ticker at all -- this is the capability the
-- extraction adds (v43 could only run drift alongside perception).
markR :: DriftRule
markR = DriftRule "mark" 2 [([Match "flag.X"], [Insert "marked.X"])]

world :: PraxState
world = foldl (flip performOutcome) base (clockSetup ++ driftSetup [markR])
  where
    base = setCharacters [clockChar, driftChar]
             (definePractices [clockP, driftP [markR]] emptyState)

-- One tick of the standalone clock.
tick :: PraxState -> PraxState
tick st = case possibleActions st clockName of
  (ga : _) -> performAction st ga
  []       -> error "the standalone clock has no action"

-- One drifter turn.
pulse :: PraxState -> PraxState
pulse st = case possibleActions st driftName of
  (ga : _) -> performAction st ga
  []       -> error "the drifter has no action"

tests :: TestTree
tests = testGroup "Prax.Clock"
  [ testCase "the standalone ticker advances turn with no perception at all" $ do
      assertBool "turn 0 at setup" (exists (turnPath ++ "!0") (db world))
      assertBool "turn 1 after a tick" (exists (turnPath ++ "!1") (db (tick world)))
      assertBool "turn 2 after two" (exists (turnPath ++ "!2") (db (tick (tick world))))

  , testCase "a sightless drift world rides the standalone clock: well-formed, no ClocklessDrift, no DeadCondition on the due-gate" $ do
      -- markR's guard reads flag.X; register a producer so the v42 dead-condition
      -- lint doesn't flag it too (the DriftSpec "drifty fixture" pattern) -- this
      -- test's claim is the clock composition, not that 'world' alone seeds flag.*.
      let flagSeed = practice { practiceId = "flagSeed", initOutcomes = [ Insert "flag.seed" ] }
          st = definePractices [flagSeed] world
      typeCheck st @?= []

  , testCase "drift pulses fire off the standalone clock's turn, with no sight practice registered" $ do
      let seeded = performOutcome (Insert "flag.a") world
          atTurn2 = tick (tick seeded)
          st = pulse atTurn2
      assertBool "marked.a inserted at the due, off the standalone clock" (exists "marked.a" (db st))
  ]
