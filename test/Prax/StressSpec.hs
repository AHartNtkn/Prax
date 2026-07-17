module Prax.StressSpec (tests) where

import qualified Data.Map.Strict as Map
import qualified Data.Set as Set
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Stress
import           Prax.Script (Script(..), Scene(..), beat, compile, goto, member, player, scene)
import           Prax.Types (PraxState)
import           Prax.Worlds.Bar (barWorld)
import           Prax.Worlds.Intrigue (intrigueWorld)
import           Prax.Worlds.Play (playWorld)
import           Prax.Worlds.Village (villageWorld)

-- | The task-1 reviewer's repro (v46 finding 4): scene @"s"@ offers its cast no
-- beat at all, only a pending unconditional transition to @"s2"@ — the only way
-- forward is the engine's round-boundary story rule, not a character's move
-- (@"s2"@ offers an ordinary beat, so arriving there is not itself another dead
-- end — the point under test is crossing the move-less @"s"@). Real play
-- (`Prax.Loop.runNpcTicks`) crosses this fine; the stress harness's dead-end
-- detector must too, once it gives the boundary's wrap its turn.
deadEndRegressionWorld :: PraxState
deadEndRegressionWorld = compile Script
  { scriptCast   = [ player "p", member "q" ]
  , scriptScenes =
      [ (scene "s") { sceneJunctions = [ goto "go" "s2" [] ] }
      , (scene "s2") { sceneBeats = [ beat "linger" [] [] ] }
      ]
  , scriptStart  = "s"
  }

tests :: TestTree
tests = testGroup "Prax.Stress"
  [ testCase "a move-less scene with a pending transition is not a stress-harness \
             \false positive: the dead-end detector must give the round \
             \boundary's wrap its turn before declaring deadlock (v46 review \
             \finding 4)" $ do
      let r = stressTest 50 40 (Just "currentScene") deadEndRegressionWorld
      srDeadEnds r @?= 0
      assertBool "s2 reached" (Map.member "s2" (srScenes r))

  , testCase "random play of the episode: no dead ends, both active branches reached" $ do
      let r = stressTest 60 40 (Just "currentScene") intrigueWorld
      assertBool "no dead ends"           (srDeadEnds r == 0)
      assertBool "no run stuck at the cap" (srNoEnding r == 0)
      -- with an active (random) protagonist, both branches Marcus can force are hit
      assertBool "loyalty reached"    (Map.member "loyalty"    (srEndings r))
      assertBool "complicity reached" (Map.member "complicity" (srEndings r))
      -- (betrayal needs a *passive* Marcus — proven deterministically in IntrigueSpec)

  , testCase "the bar survives random play with no dead ends and broad coverage" $ do
      let r = stressTest 20 30 (Just "currentScene") barWorld
      srDeadEnds r @?= 0
      assertBool "many distinct actions exercised" (Set.size (srCoverage r) >= 10)

  , testCase "scene coverage: random play reaches both scenes and every ending" $ do
      let r = stressTest 200 50 (Just "currentScene") playWorld
      -- both authored scenes are reached by random play (no unreachable scene)
      assertBool "confidence visited" (Map.member "confidence" (srScenes r))
      assertBool "banquet visited"    (Map.member "banquet"    (srScenes r))
      -- all three endings occur: Marcus can force loyalty/complicity, and when he
      -- instead romances (spending his turn) Cassia gets to poison — betrayal
      mapM_ (\e -> assertBool (e ++ " reached") (Map.member e (srEndings r)))
            ["betrayal", "loyalty", "complicity"]
      assertBool "no dead ends" (srDeadEnds r == 0)

  , testCase "coverage family generalizes past Script's currentScene: the village's \
             \marketDay family is tracked when named, proving the second application" $ do
      let r = stressTest 80 60 (Just "marketDay") villageWorld
      assertBool "the market was observed open at least once"
        (Map.member "square" (srScenes r))
  ]
