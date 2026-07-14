module Prax.PersistSpec (tests) where

import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, assertFailure, (@?=))

import           Prax.Db (dbToSentences)
import           Prax.Types (PraxState, Character (..), characters, cursor,
                              db, gaLabel, intentions)
import           Prax.Engine (possibleActions, performAction)
import           Prax.Loop (runNpcTicks, npcAct)
import           Prax.Persist (serializeState, deserializeState)
import           Prax.Worlds.Intrigue (intrigueWorld)

-- A mid-episode state (Cassia has confided; Marcus now knows the plot).
mid :: PraxState
mid = snd (runNpcTicks 2 3 intrigueWorld)

-- The same state saved and reloaded onto a fresh copy of the world.
reloaded :: PraxState
reloaded = deserializeState (serializeState mid) intrigueWorld

act :: PraxState -> String -> String -> IO PraxState
act st actor needle =
  case filter ((needle `isInfixOf`) . gaLabel) (possibleActions st actor) of
    (ga : _) -> pure (performAction st ga)
    []       -> assertFailure ("no " ++ show needle ++ " for " ++ actor) >> pure st

tests :: TestTree
tests = testGroup "Prax.Persist"
  [ testCase "save/load round-trips the fact database and cursor exactly" $ do
      db reloaded     @?= db mid
      cursor reloaded @?= cursor mid

  , testCase "save/load round-trips standing intentions exactly" $ do
      intentions reloaded @?= intentions mid

  , testCase "a reloaded standing intention is served without re-deliberating" $
      case filter ((== "marcus") . charName) (characters intrigueWorld) of
        (marcus : _) -> do
          let (contAct, _) = npcAct 2 marcus mid
              (reloAct, _) = npcAct 2 marcus reloaded
          fmap gaLabel contAct @?= Just "marcus: bide your time"
          reloAct              @?= contAct
        [] -> assertFailure "no marcus in intrigueWorld"

  , testCase "a reloaded session continues identically (Marcus can still warn)" $ do
      st <- act reloaded "marcus" "warn artus"
      assertBool "reaches the loyalty ending after reload"
        ("ending.loyalty" `elem` dbToSentences (db st))

  , testCase "the serialized form is human-readable, label-faithful facts" $ do
      let text = serializeState mid
      -- the value edge is single-valued, so it round-trips with its @!@ label
      assertBool "carries the belief Marcus formed"
        ("marcus.believes.plotAgainst.artus!yes" `isInfixOf` text)
      assertBool "has a cursor header" ("cursor " `isInfixOf` text)
  ]
