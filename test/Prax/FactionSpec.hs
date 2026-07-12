module Prax.FactionSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Either (isLeft)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool)

import           Prax.Db (exists)
import           Prax.Types (PraxState, Outcome (..), emptyState, db, readView)
import           Prax.Engine (performOutcome, setAxioms)
import           Prax.Faction

-- Two houses: hall (ana, ben) and yard (cass). One shared axiom set (comrades).
houses :: PraxState
houses = setAxioms [comrades] (foldl (flip performOutcome) emptyState setup)
  where
    setup = [ joins "ana" "hall", joins "ben" "hall", joins "cass" "yard" ]

tests :: TestTree
tests = testGroup "Prax.Faction"
  [ testCase "memberPath: single-slot exclusion fact" $
      assertBool "the ! separates who from faction"
        (memberPath "ana" "hall" == "member.ana!hall")

  , testCase "comrades: shared membership derives allied, both directions" $ do
      let v = readView houses
      assertBool "ana allied ben" (exists "allied.ana.ben" v)
      assertBool "ben allied ana" (exists "allied.ben.ana" v)

  , testCase "comrades: X<>Y guard — no self-alliance" $
      assertBool "ana is not allied with herself"
        (not (exists "allied.ana.ana" (readView houses)))

  , testCase "comrades: cross-faction negative — no shared house, no alliance" $ do
      let v = readView houses
      assertBool "ana (hall) not allied with cass (yard)" (not (exists "allied.ana.cass" v))
      assertBool "cass (yard) not allied with ana (hall)" (not (exists "allied.cass.ana" v))

  , testCase "defection un-derives: joining a new faction overwrites the old, retracting stale allied pairs" $ do
      let moved = performOutcome (joins "ana" "yard") houses
          v = readView moved
      assertBool "ana no longer allied with ben (old house)" (not (exists "allied.ana.ben" v))
      assertBool "ben no longer allied with ana (old house)" (not (exists "allied.ben.ana" v))
      assertBool "ana is now allied with cass (new house)" (exists "allied.ana.cass" v)
      assertBool "cass is now allied with ana (new house)" (exists "allied.cass.ana" v)
      assertBool "ana's old membership is gone from the base"
        (not (exists "member.ana!hall" (db moved)))
      assertBool "ana's new membership is the sole base fact"
        (exists "member.ana!yard" (db moved))

  , testCase "factionStanding: an unbelieved offense moves no one" $ do
      let world = setAxioms [factionStanding "struck.A.V" "brutal"]
                    (foldl (flip performOutcome) emptyState
                       [ joins "ana" "hall", joins "ben" "hall", joins "dave" "hall"
                       , joins "cass" "yard" ])
          v = readView world
      assertBool "no one regards ana as brutal (no belief asserted)"
        (not (any (`exists` v)
               [ "regards.ben.ana.brutal", "regards.dave.ana.brutal", "regards.cass.ana.brutal" ]))

  , testCase "factionStanding: a believed offense moves co-members only" $ do
      let world = setAxioms [factionStanding "struck.A.V" "brutal"]
                    (foldl (flip performOutcome) emptyState
                       [ joins "ana" "hall", joins "ben" "hall", joins "dave" "hall"
                       , joins "cass" "yard"
                       , Insert "dave.believes.struck.ana.ben"
                       , Insert "cass.believes.struck.ana.ben"
                       , Insert "ana.believes.struck.ana.ben"
                       ])
          v = readView world
      assertBool "dave (co-member of victim ben's faction) regards ana as brutal"
        (exists "regards.dave.ana.brutal" v)
      assertBool "cass (a different faction) believes it too but derives nothing"
        (not (exists "regards.cass.ana.brutal" v))
      assertBool "ana (the offender) derives no self-regard, even believing her own act"
        (not (exists "regards.ana.ana.brutal" v))

  , testCase "memberPath: an empty or separator-bearing name errors loudly" $ do
      r1 <- try (evaluate (length (memberPath "" "hall")))
      assertBool "empty who errors" (isLeft (r1 :: Either ErrorCall Int))
      r2 <- try (evaluate (length (memberPath "a.b" "hall")))
      assertBool "dotted who errors" (isLeft (r2 :: Either ErrorCall Int))
      r3 <- try (evaluate (length (memberPath "ana" "ha!ll")))
      assertBool "bang'd faction errors" (isLeft (r3 :: Either ErrorCall Int))

  , testCase "factionStanding: a pattern naming fewer than two variables errors loudly" $ do
      r <- try (evaluate (length (show (factionStanding "struck.A.constant" "brutal"))))
      assertBool "an offender-only pattern is an error, not a silent single-variable guard"
        (isLeft (r :: Either ErrorCall Int))

  , testCase "factionStanding: a pattern reusing W or F (the axiom's own join variables) errors loudly" $ do
      r1 <- try (evaluate (length (show (factionStanding "struck.W.V" "brutal"))))
      assertBool "an offender named W collides with the believer variable — must error, not silently no-op"
        (isLeft (r1 :: Either ErrorCall Int))
      r2 <- try (evaluate (length (show (factionStanding "struck.A.F" "brutal"))))
      assertBool "a victim named F collides with the shared-faction variable — must error, not silently no-op"
        (isLeft (r2 :: Either ErrorCall Int))

  , testCase "factionStanding: the victim's own belief of the offense against them derives their regard too" $ do
      let world = setAxioms [factionStanding "struck.A.V" "brutal"]
                    (foldl (flip performOutcome) emptyState
                       [ joins "ana" "hall", joins "ben" "hall"
                       , Insert "ben.believes.struck.ana.ben"
                       ])
          v = readView world
      assertBool "ben (the victim, a co-member of himself trivially) regards ana as brutal"
        (exists "regards.ben.ana.brutal" v)
  ]
