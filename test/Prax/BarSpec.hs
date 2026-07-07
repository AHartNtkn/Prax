module Prax.BarSpec (tests) where

import           Control.Monad (foldM)
import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, assertFailure)

import           Prax.Db (dbToSentences)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction, performOutcome)
import           Prax.Core (adjustScore, setMood, warmth, annoyed)
import           Prax.Worlds.Bar (barWorld)

-- Perform the first action whose label contains `needle` for `actor`.
act :: PraxState -> (String, String) -> IO PraxState
act st (actor, needle) =
  case filter ((needle `isInfixOf`) . gaLabel) (possibleActions st actor) of
    (ga : _) -> pure (performAction st ga)
    []       -> do
      _ <- assertFailure
             ("no action matching " ++ show needle ++ " for " ++ actor
              ++ "; available: " ++ show (map gaLabel (possibleActions st actor)))
      pure st

runSteps :: PraxState -> [(String, String)] -> IO PraxState
runSteps = foldM act

adaLabels :: PraxState -> [String]
adaLabels st = map gaLabel (possibleActions st "ada")

tests :: TestTree
tests = testGroup "Prax.Worlds.Bar (feature integration)"
  [ testCase "drinking two beers makes you tipsy (init/Call/Calc/Cmp)" $ do
      -- One beer: counter goes 0 -> 1, not yet tipsy.
      afterOne <- runSteps barWorld
        [ ("you", "Go to bar"), ("you", "Order beer")
        , ("ada", "Fulfill you"), ("you", "Drink the beer") ]
      let facts1 = dbToSentences (db afterOne)
      assertBool "counter at 1" ("practice.patron.you.drinks.1" `elem` facts1)
      assertBool "not tipsy yet" ("person.you.tipsy" `notElem` facts1)

      -- Second beer: counter 1 -> 2, crosses the threshold -> tipsy.
      afterTwo <- runSteps afterOne
        [ ("you", "Order beer"), ("ada", "Fulfill you"), ("you", "Drink the beer") ]
      let facts2 = dbToSentences (db afterTwo)
      assertBool "counter at 2" ("practice.patron.you.drinks.2" `elem` facts2)
      assertBool "now tipsy" ("person.you.tipsy" `elem` facts2)

  , testCase "the bell requires two customers (Subquery/Count/Cmp)" $ do
      -- One customer (bex ordered): no bell.
      oneCust <- runSteps barWorld [ ("bex", "Go to bar"), ("bex", "Order beer") ]
      assertBool "no bell with one customer"
        (not (any ("Ring the bell" `isInfixOf`) (adaLabels oneCust)))

      -- Two customers (you also ordered): the bell becomes available.
      twoCust <- runSteps oneCust [ ("you", "Go to bar"), ("you", "Order cider") ]
      assertBool "bell available with two customers"
        (any ("Ring the bell" `isInfixOf`) (adaLabels twoCust))

  , testCase "the buy-a-drink affordance is gated on relationship warmth" $ do
      -- Co-locate bex with ada at the bar.
      let atBar = performOutcome (Insert "practice.world.world.at.bex!bar") barWorld
          bexCanBuy s = any ("Buy ada a drink" `isInfixOf`) (map gaLabel (possibleActions s "bex"))
      -- Below the threshold: the gift is not offered.
      let cool = performOutcome (adjustScore "bex" "ada" warmth 10 "acquaintance") atBar
      assertBool "no buy option when only mildly warm" (not (bexCanBuy cool))
      -- Past the threshold: the gift appears — a relationship creating a new goal.
      let warm = performOutcome (adjustScore "bex" "ada" warmth 20 "fondness") atBar
      assertBool "buy option appears once warm enough" (bexCanBuy warm)

  , testCase "an annoyed mood withholds the friendly buy action" $ do
      let atBar = performOutcome (Insert "practice.world.world.at.bex!bar") barWorld
          bexCanBuy s = any ("Buy ada a drink" `isInfixOf`) (map gaLabel (possibleActions s "bex"))
      -- Warm enough to buy…
      let warm = performOutcome (adjustScore "bex" "ada" warmth 20 "fondness") atBar
      assertBool "warm bex can buy" (bexCanBuy warm)
      -- …but once annoyed at ada, bex withholds the gesture.
      let sulky = performOutcome (setMood "bex" annoyed "ada" "wasRude") warm
      assertBool "annoyed bex will not buy" (not (bexCanBuy sulky))
  ]
