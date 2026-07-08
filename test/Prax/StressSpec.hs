module Prax.StressSpec (tests) where

import qualified Data.Map.Strict as Map
import qualified Data.Set as Set
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Stress
import           Prax.Worlds.Bar (barWorld)
import           Prax.Worlds.Intrigue (intrigueWorld)
import           Prax.Worlds.Play (playWorld)

tests :: TestTree
tests = testGroup "Prax.Stress"
  [ testCase "random play of the episode: no dead ends, both active branches reached" $ do
      let r = stressTest 60 40 intrigueWorld
      assertBool "no dead ends"           (srDeadEnds r == 0)
      assertBool "no run stuck at the cap" (srNoEnding r == 0)
      -- with an active (random) protagonist, both branches Marcus can force are hit
      assertBool "loyalty reached"    (Map.member "loyalty"    (srEndings r))
      assertBool "complicity reached" (Map.member "complicity" (srEndings r))
      -- (betrayal needs a *passive* Marcus — proven deterministically in IntrigueSpec)

  , testCase "the bar survives random play with no dead ends and broad coverage" $ do
      let r = stressTest 20 30 barWorld
      srDeadEnds r @?= 0
      assertBool "many distinct actions exercised" (Set.size (srCoverage r) >= 10)

  , testCase "scene coverage: random play reaches both scenes and every ending" $ do
      let r = stressTest 200 50 playWorld
      -- both authored scenes are reached by random play (no unreachable scene)
      assertBool "confidence visited" (Map.member "confidence" (srScenes r))
      assertBool "banquet visited"    (Map.member "banquet"    (srScenes r))
      -- all three endings occur: Marcus can force loyalty/complicity, and when he
      -- instead romances (spending his turn) Cassia gets to poison — betrayal
      mapM_ (\e -> assertBool (e ++ " reached") (Map.member e (srEndings r)))
            ["betrayal", "loyalty", "complicity"]
      assertBool "no dead ends" (srDeadEnds r == 0)
  ]
