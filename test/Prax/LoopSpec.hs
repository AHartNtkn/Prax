module Prax.LoopSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, (@?=), assertBool)

import           Data.List (isInfixOf)

import           Prax.Db (dbToSentences)
import           Prax.Types (db, Outcome (..))
import           Prax.Engine (performOutcome)
import           Prax.Loop (runNpcTicks)
import           Prax.Worlds.Bar (barWorld)

-- Deterministic narration from driving every character with the planner (depth
-- 2) for 25 round-robin turns (idle turns, and the silent sight ticker's turns,
-- produce no line). 25, not 20: the bar's cast now includes the bodiless sight
-- ticker (Prax.Sight), so each round is 5 turns, not 4 — the same 5 rounds this
-- golden trace always covered (20 = 5x4) now take 25 (5x5). A golden replay of
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
  , "bex: settle in, feeling you belong here"
  ]

tests :: TestTree
tests = testGroup "Prax.Loop"
  [ testCase "25-turn NPC replay matches the golden narration" $
      fst (runNpcTicks 2 25 barWorld) @?= expectedTrace

  , testCase "the emergent + director-driven outcomes hold after the replay" $ do
      let facts = dbToSentences (db (snd (runNpcTicks 2 25 barWorld)))
          has f = assertBool f (f `elem` facts)
      -- bex responded to ada's greeting via the reaction
      has "practice.greet.world.greeted.bex.ada"
      -- the player's ignored greeting left ada with a grievance
      has "practice.greet.world.grievance.ada.you"
      -- the director intervened once, injecting a rivalry between the two friends
      has "dm.stirred"
      has "practice.greet.world.grievance.ada.bex"
      -- bex's arc reaches belonging (its own warmth held even as the director
      -- soured ada toward it); no NPC ever chose the against-desires transformation
      has "bex.arc.belonging"
      assertBool "no NPC resigned to solitude"
        ("bex.arc.lonely" `notElem` facts && "you.arc.lonely" `notElem` facts)

  , testCase "a dead character is skipped in turn-taking" $ do
      -- mark bex dead; over a full run bex must never act again
      let dead = performOutcome (Insert "dead.bex") barWorld
          (tr, _) = runNpcTicks 2 16 dead
      assertBool "bex takes no turns once dead" (not (any ("bex:" `isInfixOf`) tr))
  ]
