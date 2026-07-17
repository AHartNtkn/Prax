module Prax.CoreSpec (tests) where

import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool)

import           Prax.Db (dbToSentences)
import           Prax.Types
import           Prax.Engine (defineFunctions, performOutcome)
import           Prax.Core

-- A state with the core library registered.
base :: PraxState
base = defineFunctions coreFns emptyState

facts :: PraxState -> [String]
facts = dbToSentences . db

tests :: TestTree
tests = testGroup "Prax.Core"
  [ testGroup "relationships (numeric, asymmetric, with reason)"
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
  ]
