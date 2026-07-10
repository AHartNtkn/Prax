{-# LANGUAGE LambdaCase #-}
module Prax.TypeCheckSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Types
import           Prax.Engine (definePractices, performOutcome)
import           Prax.Query (Condition (..))
import           Prax.Derive (Axiom (..))
import           Prax.TypeCheck
import qualified Prax.Worlds.Bar as Bar
import qualified Prax.Worlds.Intrigue as Intrigue
import qualified Prax.Worlds.Play as Play
import qualified Prax.Worlds.Feud as Feud
import qualified Prax.Worlds.Village as Village

-- A one-practice world for the seeded-bug cases.
world1 :: Practice -> PraxState
world1 p = definePractices [p] emptyState

runOutcomes :: PraxState -> [Outcome] -> PraxState
runOutcomes = foldl (flip performOutcome)

isSortConflict :: TypeError -> Bool
isSortConflict SortConflict{} = True
isSortConflict _              = False

tests :: TestTree
tests = testGroup "Prax.TypeCheck"
  [ testCase "every shipped world is well-formed" $ do
      typeCheck Bar.barWorld          @?= []
      typeCheck Bar.barDirectorWorld  @?= []
      typeCheck Intrigue.intrigueWorld @?= []
      typeCheck Play.playWorld        @?= []
      typeCheck Feud.feudWorld        @?= []
      typeCheck Village.villageWorld  @?= []

  , testCase "an outcome variable bound by nothing is caught" $ do
      let p = practice
            { practiceId = "bug", roles = ["R"]
            , actions = [ action "[Actor]: x" [] [ Insert "foo.Ghost" ] ] }
      assertBool "UnboundVar Ghost"
        (any (\case UnboundVar _ "Ghost" _ -> True; _ -> False) (typeCheck (world1 p)))

  , testCase "an axiom head variable absent from the body is caught" $ do
      let w = emptyState { axioms = [ Axiom [ Match "p.X" ] [ "q.X.Y" ] ] }
      assertBool "UnboundVar Y"
        (any (\case UnboundVar "axiom" "Y" _ -> True; _ -> False) (typeCheck w))

  , testCase "a relation used as both ! and . is caught" $ do
      let p = practice
            { practiceId = "c"
            , actions = [ action "[Actor]: a" [] [ Insert "a.mood!happy" ]
                        , action "[Actor]: b" [] [ Insert "a.mood.sad" ] ] }
      assertBool "clash on a.mood" (CardinalityClash "a.mood" `elem` typeCheck (world1 p))

  , testCase "a Call to an undefined function is caught" $ do
      let p = practice
            { practiceId = "d", roles = ["R"]
            , actions = [ action "[Actor]: y" [] [ Call "nope" ["R"] ] ] }
      assertBool "UndefinedRef nope"
        (any (\case UndefinedRef _ "nope" -> True; _ -> False) (typeCheck (world1 p)))

  , testCase "spawning an undefined practice is caught" $ do
      let p = practice
            { practiceId = "e", roles = ["R"]
            , actions = [ action "[Actor]: z" [] [ Insert "practice.ghost.R" ] ] }
      assertBool "UndefinedRef practice.ghost"
        (any (\case UndefinedRef _ "practice.ghost" -> True; _ -> False) (typeCheck (world1 p)))

  , testCase "a correct little practice is well-formed" $ do
      let p = practice
            { practiceId = "ok", roles = ["R"]
            , actions = [ action "[Actor]: greet [R]"
                            [ Match "here.Actor", Match "here.R" ]
                            [ Insert "greeted.Actor.R" ] ] }
      typeCheck (world1 p) @?= []

    -- ML-style sort inference (only active when sorts are declared) -----------
  , testCase "no sort declarations ⇒ no sort errors" $ do
      let p = practice { practiceId = "z"
            , actions = [ action "[Actor]: a" [] [ Insert "cup.beer", Insert "cup.bar" ] ] }
      typeCheck (world1 p) @?= []                    -- with sorts=[] this is fine

  , testCase "a position given values of two sorts is caught" $ do
      let p = practice { practiceId = "menu"
            , actions = [ action "[Actor]: pour a beer" [] [ Insert "cup.beer" ]
                        , action "[Actor]: pour a bar!" [] [ Insert "cup.bar" ] ] }
          w = (world1 p) { sorts = [ ("beverage", ["beer"]), ("place", ["bar"]) ] }
      assertBool "SortConflict on cup"
        (any (\case SortConflict "cup" _ -> True; _ -> False) (typeCheck w))

  , testCase "a variable used in two different sorts is caught" $ do
      let p = practice { practiceId = "v", roles = ["X"]
            , actions = [ action "[Actor]: mix" [] [ Insert "cup.X", Insert "spot.X" ] ] }
          w = runOutcomes ((world1 p) { sorts = [ ("beverage", ["beer"]), ("place", ["bar"]) ] })
                          [ Insert "cup.beer", Insert "spot.bar" ]
      assertBool "SortConflict from X" (any isSortConflict (typeCheck w))

  , testCase "a constant declared in two sorts is caught" $ do
      let w = emptyState { sorts = [ ("agent", ["x"]), ("beverage", ["x"]) ] }
      assertBool "SortConflict on x"
        (any (\case SortConflict "x" d -> "agent" `elem` words' d && "beverage" `elem` words' d
                    _                  -> False) (typeCheck w))

    -- ForEach support --------------------------------------------------------
  , testCase "a variable bound by ForEach conditions is not unbound" $ do
      let p = practice
            { practiceId = "w", roles = ["R"]
            , actions = [ action "[Actor]: broadcast" []
                            [ ForEach [ Match "member.X" ] [ Insert "told.X" ] ] ] }
      typeCheck (world1 p) @?= []

  , testCase "a genuinely unbound variable inside ForEach is flagged" $ do
      let p = practice
            { practiceId = "w", roles = ["R"]
            , actions = [ action "[Actor]: broadcast" []
                            [ ForEach [ Match "member.X" ] [ Insert "told.Ghost" ] ] ] }
      assertBool "UnboundVar Ghost"
        (any (\case UnboundVar _ "Ghost" _ -> True; _ -> False) (typeCheck (world1 p)))

  , testCase "ForEach sub-inserts join the cardinality corpus" $ do
      -- The same relation asserted '!' at top level and '.' inside a ForEach must clash.
      let p = practice
            { practiceId = "w", roles = ["R"]
            , actions = [ action "[Actor]: a" [] [ Insert "mark.R!x" ]
                        , action "[Actor]: b" []
                            [ ForEach [ Match "member.X" ] [ Insert "mark.X.y" ] ] ] }
      assertBool "CardinalityClash detected"
        (any (\case CardinalityClash {} -> True; _ -> False) (typeCheck (world1 p)))

  , testCase "a dangling Call or spawn inside ForEach is caught" $ do
      let p = practice
            { practiceId = "w", roles = ["R"]
            , actions = [ action "[Actor]: broadcast" []
                            [ ForEach [ Match "member.X" ]
                                      [ Call "nope" ["X"]
                                      , Insert "practice.ghost.X" ] ] ] }
      assertBool "UndefinedRef nope"
        (any (\case UndefinedRef _ "nope" -> True; _ -> False) (typeCheck (world1 p)))
      assertBool "UndefinedRef practice.ghost"
        (any (\case UndefinedRef _ "practice.ghost" -> True; _ -> False) (typeCheck (world1 p)))
  ]
  where words' = words . map (\c -> if c == ',' then ' ' else c)
