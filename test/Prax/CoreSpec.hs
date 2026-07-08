module Prax.CoreSpec (tests) where

import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (dbToSentences)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractice, performOutcome)
import           Prax.Core

-- A state with the core library registered.
base :: PraxState
base = definePractice coreLib emptyState

facts :: PraxState -> [String]
facts = dbToSentences . db

tests :: TestTree
tests = testGroup "Prax.Core"
  [ testGroup "emotions (single-slot, target/cause, prior)"
    [ testCase "setMood records feeling with its target and cause" $ do
        let st = performOutcome (setMood "ada" happy "bex" "goodTip") base
            fs = facts st
        assertBool "mood is happy toward bex" ("ada.mood.happy.toward.bex" `elem` fs)
        assertBool "mood records the cause" ("ada.mood.happy.because.goodTip" `elem` fs)

    , testCase "a new mood overrides the old one and remembers it as prior" $ do
        let st0 = performOutcome (setMood "ada" happy "bex" "goodTip") base
            st1 = performOutcome (setMood "ada" angry "cid" "spill") st0
            fs  = facts st1
        -- new mood present…
        assertBool "now angry at cid" ("ada.mood.angry.toward.cid" `elem` fs)
        -- …old feeling and its target/cause gone (exclusion cleared the subtree)…
        assertBool "old happy mood cleared"
          (not (any ("mood.happy" `isInfixOf`) fs))
        -- …but the previous feeling is remembered.
        assertBool "prior mood remembered" ("ada.priorMood.happy" `elem` fs)
    ]

  , testGroup "relationships (numeric, asymmetric, with reason)"
    [ testCase "adjustScore seeds on first use, then accumulates" $ do
        let st0 = performOutcome (adjustScore "ada" "bex" warmth 10 "greeting") base
        assertBool "seeded to 10" ("ada.relationship.bex.warmth.score.10" `elem` facts st0)
        let st1 = performOutcome (adjustScore "ada" "bex" warmth 5 "served") st0
            fs  = facts st1
        assertBool "accumulated to 15" ("ada.relationship.bex.warmth.score.15" `elem` fs)
        assertBool "single score value" ("ada.relationship.bex.warmth.score.10" `notElem` fs)
        assertBool "reason updated" ("ada.relationship.bex.warmth.reason.served" `elem` fs)

    , testCase "a negative delta cools the relationship" $ do
        let st0 = performOutcome (adjustScore "ada" "bex" warmth 10 "greeting") base
            st1 = performOutcome (adjustScore "ada" "bex" warmth (-30) "insulted") st0
        assertBool "score went negative"
          ("ada.relationship.bex.warmth.score.-20" `elem` facts st1)

    , testCase "evaluations are asymmetric" $ do
        let st = performOutcome (adjustScore "ada" "bex" warmth 10 "greeting") base
            fs = facts st
        assertBool "ada judges bex" ("ada.relationship.bex.warmth.score.10" `elem` fs)
        assertBool "bex does not (yet) judge ada"
          (not (any ("bex.relationship.ada" `isInfixOf`) fs))
    ]

  , testGroup "public bond (symmetric)"
    [ testCase "setBond writes both directions" $ do
        let st = performOutcome (setBond "ada" "bex" "friends") base
            fs = facts st
        assertBool "a->b" ("bond.ada.bex.friends" `elem` fs)
        assertBool "b->a" ("bond.bex.ada.friends" `elem` fs)
    ]

  , testGroup "condition helpers"
    [ testCase "moodIs builds the expected match" $
        moodIs "Actor" annoyed @?= Match "Actor.mood!annoyed"
    ]
  ]
