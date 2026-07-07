module Prax.LoopSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, (@?=), assertBool)

import           Prax.Db (dbToSentences)
import           Prax.Types (db)
import           Prax.Loop (runNpcTicks)
import           Prax.Worlds.Bar (barWorld)

-- The deterministic narration produced by driving every character with the
-- planner (depth 2) for 12 round-robin turns of the bar world. A golden replay:
-- it locks in the emergent social arc — greet, serve, take offense at a snub,
-- and (once warm enough) buy a friend a drink. ('you' has no wants, so it paces
-- and never reciprocates greetings — in the real CLI 'you' is the human.)
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
  , "ada: Take offense at you ignoring your greeting"
  , "bex: Buy ada a drink"
  ]

tests :: TestTree
tests = testGroup "Prax.Loop"
  [ testCase "12-turn NPC replay matches the golden narration" $
      fst (runNpcTicks 2 12 barWorld) @?= expectedTrace

  , testCase "the core-model feedback loop leaves the expected traces" $ do
      let facts = dbToSentences (db (snd (runNpcTicks 2 12 barWorld)))
          has f = assertBool f (f `elem` facts)
      -- bex was served and kept the beer
      has "practice.tendBar.bar.ada.customer.bex.beverage.beer"
      -- warmth accrued past the threshold, so the gated gift became available and bex took it
      has "practice.greet.world.bought.bex.ada"
      -- the un-reciprocated greeting: ada holds a grievance and her warmth toward you went negative
      has "practice.greet.world.grievance.ada.you"
      has "ada.relationship.you.warmth.score.-5"
      -- emotion memory: ada was annoyed before bex's drink cheered her back up
      has "ada.priorMood.annoyed"
      -- warmth is asymmetric — warm toward bex, cold toward you
      has "ada.relationship.bex.warmth.score.25"
      has "bex.relationship.ada.warmth.score.23"
  ]
