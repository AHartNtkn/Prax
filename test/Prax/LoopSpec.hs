module Prax.LoopSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, (@?=), assertBool)

import           Prax.Db (dbToSentences)
import           Prax.Types (db)
import           Prax.Loop (runNpcTicks)
import           Prax.Worlds.Bar (barWorld)

-- Deterministic narration from driving every character with the planner (depth
-- 2) for 20 round-robin turns (idle turns produce no line). A golden replay of
-- the whole emergent arc: greet, serve, respond, take offense at a snub, buy a
-- friend a drink, and then — once the room is warm — the director steps in and
-- turns two friends against each other, after which they cool. ('you' has no
-- wants, so it paces; in the CLI 'you' is the human.)
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
  , "director: turn ada against bex to stir up the evening"
  , "you: Go to bar"
  , "ada: Wait a moment"
  , "bex: Tip ada"
  ]

tests :: TestTree
tests = testGroup "Prax.Loop"
  [ testCase "20-turn NPC replay matches the golden narration" $
      fst (runNpcTicks 2 20 barWorld) @?= expectedTrace

  , testCase "the emergent + director-driven outcomes hold after the replay" $ do
      let facts = dbToSentences (db (snd (runNpcTicks 2 20 barWorld)))
          has f = assertBool f (f `elem` facts)
      -- bex responded to ada's greeting via the reaction
      has "practice.greet.world.greeted.bex.ada"
      -- the player's ignored greeting left ada with a grievance
      has "practice.greet.world.grievance.ada.you"
      -- bex respected the tipping norm
      has "bex.tipped.ada"
      -- the director intervened once, injecting a rivalry between the two friends
      has "dm.stirred"
      has "practice.greet.world.grievance.ada.bex"
  ]
