module Prax.CookedSpec (tests) where

import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (assertEqual, testCase, (@?=))

import           Prax.Cooked (cookOutcome, groundCookedOutcome)
import           Prax.Db (Bindings, Db, Val (..), emptyDb, insertAll)
import           Prax.Engine (groundOutcome)
import           Prax.Query (CalcOp (..), CmpOp (..), Condition (..), cookCondition,
                              groundCondition, groundCookedCondition, groundNames, query,
                              queryCooked)
import           Prax.Sym (intern, symName)
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

-- The binding grounded against below: every one of "Who"/"Where"/"N"/"M" is
-- actually bound, so every construct's grounding is exercised with a real
-- substitution, not a no-op pass-through.
groundBindings :: Bindings
groundBindings = Map.fromList
  [ (intern "Who", VSym (intern "bob")), (intern "Where", VSym (intern "square"))
  , (intern "N", VNum 5), (intern "M", VNum 3) ]

-- One condition per remaining 'CookedCondition' constructor (Match/Not are
-- pinned directly above; every other constructor gets its grounding branch
-- exercised here).
groundFixtureConditions :: [Condition]
groundFixtureConditions =
  [ Not "holding.Who.loaf"
  , Eq "Who" "bob"
  , Neq "Who" "eve"
  , Cmp Gte "N" "M"
  , Calc "R" Add "N" "M"
  , Count "R2" "N"
  , Subquery "Rs" ["Who"] [ Match "regards.Who.carol.thief" ]
  , Or [ [ Match "at.Who!square" ], [ Match "regards.Who.carol.thief" ] ]
  , Absent [ Match "regards.Who.carol.thief" ]
  , Exists [ Match "holding.Who.loaf" ]
  ]

-- One outcome per remaining 'CookedOutcome' constructor (Insert is pinned
-- directly above). 'ForEach' substitutes both its conditions and its nested
-- outcomes.
groundFixtureOutcomes :: [Outcome]
groundFixtureOutcomes =
  [ Delete "holding.Who.loaf"
  , Call "greet" ["Who", "bob"]
  , ForEach [ Match "regards.Who.carol.thief" ]
            [ Insert "shunned.Who!true", Delete "trusted.Who" ]
  ]

tests :: TestTree
tests = testGroup "Prax.Cooked"
  [ testCase "queryCooked equals the string evaluator on every fixture case" $
      [ queryCooked db (map cookCondition cs) Map.empty | cs <- cases ]
        @?= [ query db cs Map.empty | cs <- cases ]
  , testCase "grounding cooked matches grounding strings (incl. '!' outcomes)" $ do
      map symName (groundNames groundBindings (map intern ["at", "Who", "Where"]))
        @?= ["at", "bob", "square"]
      groundCookedOutcome groundBindings (cookOutcome (Insert "at.Who!Where"))
        @?= cookOutcome (Insert "at.bob!square")
      groundCookedCondition groundBindings (cookCondition (Match "at.Who!Where"))
        @?= cookCondition (Match "at.bob!square")
  , testCase "groundCookedCondition matches groundCondition for every remaining construct" $
      mapM_
        (\c -> assertEqual (show c)
                 (cookCondition (groundCondition groundBindings c))
                 (groundCookedCondition groundBindings (cookCondition c)))
        groundFixtureConditions
  , testCase "groundCookedOutcome matches groundOutcome for every remaining construct" $
      mapM_
        (\o -> assertEqual (show o)
                 (cookOutcome (groundOutcome o groundBindings))
                 (groundCookedOutcome groundBindings (cookOutcome o)))
        groundFixtureOutcomes
  ]
