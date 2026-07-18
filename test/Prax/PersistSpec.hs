module Prax.PersistSpec (tests) where

import           Control.Exception (ErrorCall (..), evaluate, try)
import           Data.Either (isLeft)
import           Data.List (isInfixOf)
import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, assertFailure, (@?=))

import           Prax.Db (dbToSentences, insertAll, internTokens)
import           Prax.Types (PraxState, Character (..), Outcome (..), ScheduleRule (..),
                              characters, cursor, db, emptyState, expiries, gaLabel,
                              intentions, rngSeed, schedule, scheduleDues)
import           Prax.Engine (possibleActions, performAction, performOutcome, seedDie)
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

    , testCase "the previous format version (prax-state v1) is now rejected loudly" $ do
        r <- try (evaluate (length (dbToSentences (db (deserializeState "prax-state v1\ncursor 0\n" intrigueWorld)))))
        case r :: Either ErrorCall Int of
          Left (ErrorCall msg) -> assertBool ("unsupported-format message, got: " ++ msg)
                                    ("unsupported save format" `isInfixOf` msg)
          Right _ -> assertFailure "expected a v1-rejection error"

    , testCase "the immediately-prior format version (prax-state v2) is rejected under v46's v3 bump" $ do
        -- A v45-era Script save carries storyAdvanced.*/memoryFired.* facts and
        -- a junctions-practice instance but no story due -- byte-shaped like v2
        -- yet semantically dead now, so v2 must not load silently.
        r <- try (evaluate (length (dbToSentences (db (deserializeState "prax-state v2\ncursor 0\n" intrigueWorld)))))
        case r :: Either ErrorCall Int of
          Left (ErrorCall msg) -> assertBool ("unsupported-format message, got: " ++ msg)
                                    ("unsupported save format" `isInfixOf` msg)
          Right _ -> assertFailure "expected a v2-rejection error"

    , testCase "the immediately-prior format version (prax-state v3) is rejected under v50's v4 bump" $ do
        -- A v47-era save predates the die's residence move: it carries a seed!N
        -- fact instead of an rngseed line, so a v3 save must not load silently
        -- onto a v50 world that reads the die from engine state.
        r <- try (evaluate (length (dbToSentences (db (deserializeState "prax-state v3\ncursor 0\n" intrigueWorld)))))
        case r :: Either ErrorCall Int of
          Left (ErrorCall msg) -> assertBool ("unsupported-format message, got: " ++ msg)
                                    ("unsupported save format" `isInfixOf` msg)
          Right _ -> assertFailure "expected a v3-rejection error"
    ]

  , testGroup "v44: the schedule's runtime half (per-rule dues + the expiry queue) round-trips"
    [ testCase "populated dues and expiries survive save/load; dues re-associate by name" $ do
        -- A world that declares one schedule rule (so the reloaded dues have a
        -- rule to re-associate to). Task 1 has no schedule setter, so the
        -- declaration is installed on the field directly, exactly as a state
        -- carrying runtime dues/expiries would arrive.
        let scheduled = intrigueWorld { schedule = [ScheduleRule "beat" 3 []] }
            populated = scheduled
              { scheduleDues = Map.fromList [("beat", 5)]
              , expiries     = Map.fromList [(internTokens "mood!a", 7)] }
            reloadedPop = deserializeState (serializeState populated) scheduled
        scheduleDues reloadedPop @?= scheduleDues populated
        expiries     reloadedPop @?= expiries populated

    , testCase "a due naming a rule the reloaded world does not declare is a loud error" $ do
        -- The re-association is forced by touching the reloaded dues (lazy,
        -- like intention parsing): intrigueWorld declares no schedule.
        r <- try (evaluate (length (show (scheduleDues
               (deserializeState (formatVersion ++ "\ncursor 0\ndue ghost 3\n") intrigueWorld)))))
        case r :: Either ErrorCall Int of
          Left (ErrorCall msg) -> assertBool ("unknown-rule message, got: " ++ msg)
                                    ("unknown schedule rule" `isInfixOf` msg)
          Right _ -> assertFailure "expected an unknown-schedule-rule error"
    ]

  , testGroup "v50: the drama die's stream position (rngseed) round-trips"
    [ testCase "a seeded, mid-stream state round-trips its rngseed exactly" $ do
        -- Seed, advance the stream by one draw, then save/reload: the residence
        -- move makes the position db-external, so Persist must carry it.
        let st0 = seedDie 1988 emptyState
            st1 = performOutcome (Roll 1 2 [] [Insert "hit.mark"]) st0
            reloadedSt = deserializeState (serializeState st1) emptyState
        rngSeed reloadedSt @?= rngSeed st1

    , testCase "an unseeded state emits no rngseed line" $
        assertBool "no rngseed line for an unseeded (intrigue) save"
          (not ("rngseed" `isInfixOf` serializeState mid))

    , testCase "mid-stream save/resume continues the stream identically" $ do
        -- The pin the residence move makes newly meaningful: save between draws,
        -- resume, and the NEXT draw lands at the same stream position either way.
        let st1 = performOutcome (Roll 1 2 [] []) (seedDie 1988 emptyState)
            reloadedSt = deserializeState (serializeState st1) emptyState
            contDirect = performOutcome (Roll 1 2 [] []) st1
            contReload = performOutcome (Roll 1 2 [] []) reloadedSt
        rngSeed contReload @?= rngSeed contDirect
    ]
  ]
