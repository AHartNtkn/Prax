{-# LANGUAGE LambdaCase #-}
module Prax.TypeCheckSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, setAxioms, setDesires, setCharacters)
import           Prax.Query (Condition (..), CmpOp (..))
import           Prax.Derive (Axiom (..), axiom)
import           Prax.TypeCheck
import qualified Prax.Worlds.Bar as Bar
import qualified Prax.Worlds.Intrigue as Intrigue
import qualified Prax.Worlds.Play as Play
import qualified Prax.Worlds.Feud as Feud
import qualified Prax.Worlds.Village as Village
import qualified Prax.Worlds.Audience as Audience

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
        -- feud wires Prax.Kin.kinAxioms wholesale; the fixture's own haddock
        -- documents the un-derivable remainder (no parent.* base fact exists
        -- until a wedding inserts one) as deliberate — axiom bodies are out
        -- of the dead-condition lint's scope, so this stays clean.
      typeCheck Village.villageWorld  @?= []
        -- village's drawn-to-market desire reads marketDay.square, which
        -- only the drift practice inserts — the pin holds because
        -- 'producibleAtoms' includes the drifter, unlike the planner's
        -- worldAtomPools.
      typeCheck Audience.audienceWorld @?= []

  , testCase "an outcome variable bound by nothing is caught" $ do
      let p = practice
            { practiceId = "bug", roles = ["R"]
            , actions = [ action "[Actor]: x" [] [ Insert "foo.Ghost" ] ] }
      assertBool "UnboundVar Ghost"
        (any (\case UnboundVar _ "Ghost" _ -> True; _ -> False) (typeCheck (world1 p)))

  , testCase "an axiom head variable absent from the body is caught" $ do
      let w = setAxioms [ Axiom [ Match "p.X" ] [ "q.X.Y" ] ] emptyState
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
            , initOutcomes = [ Insert "here.someone" ]
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
            , initOutcomes = [ Insert "member.someone" ]
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

    -- Dead-condition lint -----------------------------------------------------
  , testCase "a dead action conjunct (typo'd predicate) is caught" $ do
      let p = practice
            { practiceId = "hunt"
            , initOutcomes = [ Insert "treasure.spot" ]
            , actions = [ action "[Actor]: dig" [ Match "tresure.spot" ] [ Insert "dug.Actor" ] ] }
      typeCheck (world1 p) @?= [ DeadCondition "hunt / [Actor]: dig" "tresure.spot" ]

  , testCase "the corrected twin of the typo is well-formed" $ do
      let p = practice
            { practiceId = "hunt"
            , initOutcomes = [ Insert "treasure.spot" ]
            , actions = [ action "[Actor]: dig" [ Match "treasure.spot" ] [ Insert "dug.Actor" ] ] }
      typeCheck (world1 p) @?= []

  , testCase "a dead positive inside Exists is caught" $ do
      let p = practice
            { practiceId = "hunt"
            , initOutcomes = [ Insert "treasure.spot" ]
            , actions = [ action "[Actor]: dig"
                            [ Exists [ Match "tresure.spot" ] ] [ Insert "dug.Actor" ] ] }
      typeCheck (world1 p) @?= [ DeadCondition "hunt / [Actor]: dig" "tresure.spot" ]

  , testCase "a dead ForEach guard is caught, sited as an effect guard" $ do
      let p = practice
            { practiceId = "hunt"
            , initOutcomes = [ Insert "treasure.spot" ]
            , actions = [ action "[Actor]: search" []
                            [ ForEach [ Match "tresure.spot" ] [ Insert "found" ] ] ] }
      typeCheck (world1 p) @?= [ DeadCondition "hunt / [Actor]: search (effect guard)" "tresure.spot" ]

  , testCase "a dead desire and a dead character want are each caught" $ do
      let desireW = Desire "wantGold" (Want [ Match "ghost.family" ] 5)
          vic = (character "vic") { charWants = [ Want [ Match "ghost.spirit" ] 3 ] }
          w = setCharacters [vic] (setDesires [desireW] emptyState)
      typeCheck w @?=
        [ DeadCondition "desire wantGold" "ghost.family"
        , DeadCondition "want of vic" "ghost.spirit" ]

  , testCase "a negation over a never-produced family is not flagged" $ do
      let p = practice
            { practiceId = "spookless"
            , actions = [ action "[Actor]: peek" [ Not "ghost.Actor" ] [ Insert "peeked.Actor" ] ] }
      typeCheck (world1 p) @?= []

  , testCase "a half-dead Or clause is not flagged" $ do
      let p = practice
            { practiceId = "hunt"
            , initOutcomes = [ Insert "treasure.spot" ]
            , actions = [ action "[Actor]: dig"
                            [ Or [ [ Match "tresure.spot" ], [ Match "treasure.spot" ] ] ]
                            [ Insert "dug.Actor" ] ] }
      typeCheck (world1 p) @?= []

  , testCase "a dead pattern inside a Subquery interior is not flagged" $ do
      let p = practice
            { practiceId = "hunt"
            , actions = [ action "[Actor]: check"
                            [ Subquery "S" [] [ Match "tresure.spot" ]
                            , Count "N" "S"
                            , Cmp Lte "N" "0" ]
                            [ Insert "checked.Actor" ] ] }
      typeCheck (world1 p) @?= []

  , testCase "a fully unanchored pattern (every segment a variable) is not flagged" $ do
      let p = practice
            { practiceId = "hunt", roles = ["X", "Y"]
            , actions = [ action "[Actor]: link" [ Match "X.Y" ] [ Insert "linked.X.Y" ] ] }
      typeCheck (world1 p) @?= []

  , testCase "a wild world (undefined Call) silences the lint, not the ref check" $ do
      let p = practice
            { practiceId = "hunt"
            , actions = [ action "[Actor]: dig" [ Match "tresure.spot" ] [ Call "missingFn" [] ] ] }
          es = typeCheck (world1 p)
      assertBool "UndefinedRef missingFn fires"
        (any (\case UndefinedRef _ "missingFn" -> True; _ -> False) es)
      assertBool "no DeadCondition"
        (not (any (\case DeadCondition {} -> True; _ -> False) es))

  , testCase "a dead axiom body is not flagged (axiom bodies are out of scope)" $ do
      let w = setAxioms [ axiom [ Match "parent.P.C" ] [ "kin.P.C" ] ] emptyState
      typeCheck w @?= []
  ]
  where words' = words . map (\c -> if c == ',' then ' ' else c)
