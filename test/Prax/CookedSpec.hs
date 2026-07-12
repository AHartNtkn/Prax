module Prax.CookedSpec (tests) where

import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, (@?=))

import           Prax.Cooked
import           Prax.Db (Db, Val (..), emptyDb, insertAll)
import           Prax.Query (CmpOp (..), Condition (..), query, queryCooked)
import           Prax.Types (Outcome (..))

-- A fixture exercising every pattern-bearing construct: exclusion paths,
-- variables, negation, nested quantifiers, disjunction, subquery+count+cmp.
db :: Db
db = insertAll
  [ "at.bob!square", "at.eve!mill", "at.gale!mill"
  , "holding.bob.loaf", "regards.dana.carol.thief", "regards.gale.carol.thief"
  ] emptyDb

cases :: [[Condition]]
cases =
  [ [ Match "at.Who!Where" ]
  , [ Match "at.Who!mill", Not "holding.Who.loaf" ]
  , [ Match "at.Who!Where", Absent [ Match "regards.Who.carol.thief" ] ]
  , [ Exists [ Match "holding.H.loaf" ], Match "at.Who!square" ]
  , [ Or [ [ Match "at.Who!square" ], [ Match "regards.Who.carol.thief" ] ] ]
  , [ Subquery "Rs" ["W"] [ Match "regards.W.carol.thief" ]
    , Count "N" "Rs", Cmp Gte "N" "2" ]
  , [ Match "at.Who!Where", Neq "Who" "bob", Eq "Place" "Where" ]
  ]

tests :: TestTree
tests = testGroup "Prax.Cooked"
  [ testCase "queryCooked equals the string evaluator on every fixture case" $
      [ queryCooked db (map cookCondition cs) Map.empty | cs <- cases ]
        @?= [ query db cs Map.empty | cs <- cases ]
  , testCase "grounding cooked matches grounding strings (incl. '!' outcomes)" $ do
      let b = Map.fromList [ ("Who", VStr "bob"), ("Where", VStr "square") ]
      groundNames b ["at", "Who", "Where"] @?= ["at", "bob", "square"]
      groundCookedOutcome b (cookOutcome (Insert "at.Who!Where"))
        @?= cookOutcome (Insert "at.bob!square")
      groundCookedCondition b (cookCondition (Match "at.Who!Where"))
        @?= cookCondition (Match "at.bob!square")
  ]
