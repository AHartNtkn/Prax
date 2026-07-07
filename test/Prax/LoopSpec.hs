module Prax.LoopSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, (@?=), assertBool)

import           Prax.Db (dbToSentences)
import           Prax.Types (db)
import           Prax.Loop (runNpcTicks)
import           Prax.Worlds.Bar (barWorld)

-- The deterministic narration produced by driving every character with the
-- planner (depth 2) for 12 round-robin turns of the bar world. This is a golden
-- replay: it locks in that the order -> fulfill -> serve arc emerges on its own.
-- ('you' has no wants, so it paces around — in the real CLI 'you' is the human.)
expectedTrace :: [String]
expectedTrace =
  [ "you: Go to bar"
  , "ada: Greet you"
  , "bex: Go to bar"
  , "you: Go to entrance"
  , "ada: Greet bex"
  , "bex: Order beer"
  , "you: Go to bar"
  , "ada: Fulfill bex's order"
  , "bex: Greet ada"
  , "you: Go to entrance"
  , "ada: Wait a moment"
  , "bex: Wait a moment"
  ]

tests :: TestTree
tests = testGroup "Prax.Loop"
  [ testCase "12-turn NPC replay matches the golden narration" $
      fst (runNpcTicks 2 12 barWorld) @?= expectedTrace

  , testCase "after the replay, bex has been served a beer" $
      let (_, st) = runNpcTicks 2 12 barWorld
      in assertBool "bex holds a beer"
           ("practice.tendBar.bar.ada.customer.bex.beverage.beer"
              `elem` dbToSentences (db st))
  ]
