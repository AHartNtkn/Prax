module Prax.PersistSpec (tests) where

import           Control.Exception (ErrorCall (..), evaluate, try)
import           Data.Either (isLeft)
import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, assertFailure, (@?=))

import           Prax.Db (dbToSentences, insertAll)
import           Prax.Types (PraxState, Character (..), characters, cursor,
                              db, gaLabel, intentions)
import           Prax.Engine (possibleActions, performAction)
import           Prax.Loop (runNpcTicks, npcAct)
import           Prax.Persist (serializeState, deserializeState, formatVersion)
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

  , testCase "save/load round-trips an asserted interior node with transient children (marks included)" $ do
      -- A spawned practice instance is an asserted fact that ALSO parents
      -- transient per-customer children; the assertedness mark must survive
      -- serialization or the instance reloads as mere scaffolding. Inject one
      -- into a live state, round-trip, and assert full db equality (marks
      -- included) plus that the instance re-emits as its own fact.
      let withInstance = mid { db = insertAll
            [ "practice.tendBar.bar.ada"
            , "practice.tendBar.bar.ada.customer.you" ] (db mid) }
          reloadedInst = deserializeState (serializeState withInstance) intrigueWorld
      db reloadedInst @?= db withInstance
      assertBool "the asserted instance re-emits as its own fact after reload"
        ("practice.tendBar.bar.ada" `elem` dbToSentences (db reloadedInst))

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

  , testGroup "v43: the save-format version header (previously latent: a save from another era misparsed silently)"
    [ testCase "the serialized form's first line is the format version tag" $ do
        case lines (serializeState mid) of
          (v : _) -> v @?= formatVersion
          []      -> assertFailure "serializeState produced no lines"

    , testCase "a header with no cursor line is a loud, malformed-save error" $ do
        r <- try (evaluate (length (dbToSentences (db (deserializeState (formatVersion ++ "\n") intrigueWorld)))))
        case r :: Either ErrorCall Int of
          Left (ErrorCall msg) -> assertBool ("malformed message, got: " ++ msg)
                                     ("malformed save" `isInfixOf` msg)
          Right _ -> assertFailure "expected a malformed-save error"

    , testCase "an unsupported format version (prax-state v0) is a loud, version-mismatch error" $ do
        r <- try (evaluate (length (dbToSentences (db (deserializeState "prax-state v0\ncursor 0\n" intrigueWorld)))))
        case r :: Either ErrorCall Int of
          Left (ErrorCall msg) -> assertBool ("unsupported-format message, got: " ++ msg)
                                     ("unsupported save format" `isInfixOf` msg)
          Right _ -> assertFailure "expected an unsupported-format error"

    , testCase "a save with no header at all is a loud, malformed-save error" $ do
        r <- try (evaluate (length (dbToSentences (db (deserializeState "" intrigueWorld)))))
        assertBool "completely empty input rejected" (isLeft (r :: Either ErrorCall Int))
    ]
  ]
