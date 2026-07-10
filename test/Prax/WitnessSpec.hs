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
