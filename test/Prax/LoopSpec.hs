module Prax.LoopSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, (@?=), assertBool)

import           Prax.Db (dbToSentences)
import           Prax.Types (db)
import           Prax.Loop (runNpcTicks)
import           Prax.Worlds.Bar (barWorld)

-- Deterministic narration from driving every character with the planner (depth
-- 2) for 15 round-robin turns. A golden replay of the whole emergent social arc:
-- greet, serve, respond to a greeting, take offense at a snub, buy a friend a
-- drink, and tip (respecting the norm rather than stiffing). ('you' has no wants,
-- so it paces and never reciprocates — in the CLI 'you' is the human.)
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
  , "bex: Greet ada back"
  , "you: Go to entrance"
  , "ada: Take offense that you ignored your greeting"
  , "bex: Buy ada a drink"
  , "you: Go to bar"
  , "ada: Buy bex a drink"
  , "bex: Tip ada"
  ]

tests :: TestTree
tests = testGroup "Prax.Loop"
  [ testCase "15-turn NPC replay matches the golden narration" $
      fst (runNpcTicks 2 15 barWorld) @?= expectedTrace

  , testCase "the reaction/norm outcomes hold after the replay" $ do
      let facts = dbToSentences (db (snd (runNpcTicks 2 15 barWorld)))
          has f = assertBool f (f `elem` facts)
          hasNot f = assertBool ("not " ++ f) (f `notElem` facts)
      -- bex responded to ada's greeting via the reaction (greeted her back)
      has "practice.greet.world.greeted.bex.ada"
      -- the player ignored ada's greeting, so ada took offense (a reaction)
      has "practice.greet.world.grievance.ada.you"
      -- bex respected the tipping norm rather than stiffing ada
      has "bex.tipped.ada"
      hasNot "violated.bex.stiffedTheBartender"
  ]
