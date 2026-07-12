module Prax.GoldenDriveSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, (@?=))

import           Prax.Types
import           Prax.Loop (advance, npcAct)
import           Prax.Worlds.Village (villageWorld, playerName)
import           Prax.Worlds.Bar (barWorld)
import           Prax.Worlds.Intrigue (intrigueWorld)

-- One planner-driven turn per cast member per round; the named character
-- idles (mirrors VillageSpec's driveIdle). Each turn contributes one line:
-- "<actor>: <label>" for a performed action, "<actor>: -" for idle/no move.
driveLabels :: Int -> Maybe String -> PraxState -> [String]
driveLabels n idle = go n
  where
    go 0 _  = []
    go k st =
      let (actor, st1) = advance st
      in if Just (charName actor) == idle
           then (charName actor ++ ": -") : go (k - 1) st1
           else case npcAct 2 actor st1 of
                  (mga, st2) ->
                    (charName actor ++ ": " ++ maybe "-" gaLabel mga)
                      : go (k - 1) st2

-- Captured from a live run of the pre-v26 planner (see the capture program in
-- this plan). These sequences ARE the planner's contract: any change that
-- perturbs a single decision fails here. Never edit them to match new
-- behavior — a failure means the change is wrong.
villageGolden :: [String]
villageGolden =
  [ "you: -"
  , "bob: bob: take up honest work at the stall"
  , "carol: carol: Wait a moment"
  , "dana: dana: Wait a moment"
  , "eve: eve: whisper to dana that carol stole the loaf"
  , "gale: gale: Go to square"
  , "_sight: "
  , "you: -"
  , "bob: bob: sweep the square"
  , "carol: carol: Wait a moment"
  , "dana: dana: shun carol"
  , "eve: eve: Go to square"
  , "gale: gale: Go to mill"
  , "_sight: "
  , "you: -"
  , "bob: bob: Go to mill"
  , "carol: carol: Wait a moment"
  , "dana: dana: Go to square"
  , "eve: eve: Go to mill"
  , "gale: gale: Go to square"
  , "_sight: "
  ]

barGolden :: [String]
barGolden =
  [ "you: you: Go to bar"
  , "ada: ada: Greet you"
  , "bex: bex: Go to bar"
  , "director: -"
  , "_sight: "
  , "you: you: Go to entrance"
  , "ada: ada: Greet bex"
  , "bex: bex: Order beer"
  , "director: -"
  , "_sight: "
  , "you: you: Go to bar"
  , "ada: ada: Fulfill bex's order"
  ]

intrigueGolden :: [String]
intrigueGolden =
  [ "marcus: marcus: bide your time"
  , "artus: artus: bide your time"
  , "cassia: cassia: confide the plot against artus to marcus"
  , "marcus: marcus: bide your time"
  , "artus: artus: bide your time"
  , "cassia: cassia: slip poison into artus's cup"
  , "marcus: marcus: bide your time"
  , "cassia: cassia: bide your time"
  , "marcus: marcus: bide your time"
  , "cassia: cassia: bide your time"
  , "marcus: marcus: bide your time"
  , "cassia: cassia: bide your time"
  ]

tests :: TestTree
tests = testGroup "Prax.GoldenDrive (decision-sequence exactness)"
  [ testCase "village: 3 rounds of free play, decision for decision" $
      driveLabels 21 (Just playerName) villageWorld @?= villageGolden
  , testCase "bar: 12 turns, decision for decision" $
      driveLabels 12 Nothing barWorld @?= barGolden
  , testCase "intrigue: 12 turns, decision for decision" $
      driveLabels 12 Nothing intrigueWorld @?= intrigueGolden
  ]
