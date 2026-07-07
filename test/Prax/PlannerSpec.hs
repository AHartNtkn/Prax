module Prax.PlannerSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, (@?=), assertBool)

import           Prax.Query
import           Prax.Types
import           Prax.Engine
import           Prax.Planner

-- A minimal tend-bar practice (walk up + order) for planner tests.
tendBarP :: Practice
tendBarP = practice
  { practiceId = "tendBar"
  , practiceName = "[Bartender] is tending bar"
  , roles = ["Bartender"]
  , dataFacts =
      [ "beverageType.beer!alcoholic", "beverageType.cider!alcoholic"
      , "beverageType.soda!nonalcoholic" ]
  , actions =
      [ action "[Actor]: Walk up to bar"
          [ Neq "Actor" "Bartender"
          , Not "practice.tendBar.Bartender.customer.Actor" ]
          [ Insert "practice.tendBar.Bartender.customer.Actor" ]
      , action "[Actor]: Order [Beverage]"
          [ Match "practice.tendBar.Bartender.customer.Actor"
          , Not "practice.tendBar.Bartender.customer.Actor!beverage"
          , Match "practiceData.tendBar.beverageType.Beverage" ]
          [ Insert "practice.tendBar.Bartender.customer.Actor!order!Beverage" ]
      ]
  }

-- beth wants, above all, to have a cider on order.
bethWantsCider :: Character
bethWantsCider = (character "beth")
  { charWants = [ Want [ Match "practice.tendBar.Bartender.customer.beth!order!cider" ] 10 ] }

-- World with bar instance (ada tending) and beth present.
barState :: PraxState
barState =
  let st0 = (definePractices [tendBarP] emptyState) { characters = [bethWantsCider] }
  in performOutcome (Insert "practice.tendBar.ada") st0

-- After beth has walked up to the bar.
walkedUp :: PraxState
walkedUp = performOutcome (Insert "practice.tendBar.ada.customer.beth") barState

tests :: TestTree
tests = testGroup "Prax.Planner"
  [ testCase "evaluate sums utility over satisfying instantiations" $ do
      -- No cider order yet: want unsatisfied, utility 0.
      evaluate walkedUp (charWants bethWantsCider) @?= 0
      -- After ordering cider, the want is satisfied once: utility 10.
      let ordered = performOutcome
            (Insert "practice.tendBar.ada.customer.beth!order!cider") walkedUp
      evaluate ordered (charWants bethWantsCider) @?= 10

  , testCase "pickAction chooses the want-satisfying order over the alternatives" $ do
      -- beth can order beer, cider, or soda; only cider satisfies her want.
      let best = pickAction 0 walkedUp bethWantsCider
      fmap gaLabel best @?= Just "beth: Order cider"

  , testCase "top-scoring order beats the others; non-cider orders score 0" $ do
      let scored = scoreActions 0 walkedUp bethWantsCider
      -- Ordering cider is uniquely top with value 10.
      case scored of
        ((ga, s) : _) -> do gaLabel ga @?= "beth: Order cider"; s @?= 10.0
        []            -> assertBool "expected some actions" False
      -- Every non-cider action scores 0.
      let others = [ s | (ga, s) <- scored, gaLabel ga /= "beth: Order cider" ]
      all (== 0.0) others @?= True

  , testCase "lookahead: walking up is worthless immediately but valuable at depth 1" $ do
      -- Before walking up, beth's only move is to walk up. That move yields no
      -- immediate utility, but at depth 1 the 0.9-discounted future cider order
      -- makes it worth 9.
      case pickAction 0 barState bethWantsCider of
        Nothing -> assertBool "expected a walk-up action" False
        Just walk -> do
          gaLabel walk @?= "beth: Walk up to bar"
          let afterWalk = performAction barState walk
          worldValue 0 afterWalk bethWantsCider @?= 0.0
          worldValue 1 afterWalk bethWantsCider @?= 9.0
  ]
