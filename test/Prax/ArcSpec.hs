module Prax.ArcSpec (tests) where

import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (dbToSentences)
import           Prax.Query (Condition (..))
import           Prax.Types (Outcome (..), PraxState (..), emptyState)
import           Prax.Engine (performOutcome)
import           Prax.Arc

run :: [Outcome] -> [String]
run outs = dbToSentences (db (foldl (flip performOutcome) emptyState outs))

tests :: TestTree
tests = testGroup "Prax.Arc"
  [ testCase "arcSentence / arcIs build the expected fact" $ do
      arcSentence "bex" "hopeful" @?= "bex.arc!hopeful"
      arcIs "bex" "hopeful" @?= Match "bex.arc!hopeful"

  , testCase "enterArc records the stage" $
      assertBool "stage recorded" ("bex.arc.hopeful" `elem` run [ enterArc "bex" "hopeful" ])

  , testCase "entering a new stage overrides the old (single-slot)" $ do
      let fs = run [ enterArc "bex" "hopeful", enterArc "bex" "belonging" ]
      assertBool "new stage" ("bex.arc.belonging" `elem` fs)
      assertBool "old stage gone" (not (any ("arc.hopeful" `isInfixOf`) fs))
  ]
