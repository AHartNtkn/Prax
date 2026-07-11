module Prax.DirectorSpec (tests) where

import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool)

import           Prax.Db (exists)
import           Prax.Types
import           Prax.Engine (performAction)
import           Prax.Planner (candidateActions)
import           Prax.Worlds.Bar (barDirectorWorld, directorName)

-- The director character (the player-controlled drama manager).
directorChar :: Character
directorChar =
  case [ c | c <- characters barDirectorWorld, charName c == directorName ] of
    (c : _) -> c
    []      -> error ("director character " ++ show directorName ++ " not found in barDirectorWorld")

-- Perform the director's affordance whose label contains @needle@ (fails the
-- test loudly if there is none).
direct :: String -> PraxState -> PraxState
direct needle st =
  case [ ga | ga <- candidateActions st directorChar, needle `isInfixOf` gaLabel ga ] of
    (ga : _) -> performAction st ga
    []       -> error ("no director nudge matching " ++ show needle
                        ++ "; had: " ++ show (map gaLabel (candidateActions st directorChar)))

-- bex's currently available action labels.
bexCan :: PraxState -> [String]
bexCan st = map gaLabel (candidateActions st bex)
  where bex = case [ c | c <- characters st, charName c == "bex" ] of
                (c : _) -> c
                []      -> error "bex not found in given state"

tests :: TestTree
tests = testGroup "Prax.Worlds.Bar (player-as-DM)"
  [ testCase "the drama manager is offered only metalevel affordances" $ do
      let acts = candidateActions barDirectorWorld directorChar
      assertBool "the DM has nudges to make" (not (null acts))
      assertBool "every DM affordance is a metalevel 'direct' move (never embodied)"
        (all ((== "direct") . gaPracticeId) acts)
      -- concretely, the palette includes stir / kindle / pall
      let labels = unwords (map gaLabel acts)
      mapM_ (\n -> assertBool (n ++ " offered") (n `isInfixOf` labels))
            ["stir up a rivalry", "kindle warmth", "cast a pall"]

  , testCase "a DM nudge reshapes the world without the DM embodying anyone" $ do
      -- stirring bex against cai plants the annoyance + grievance in the world…
      let st = direct "stir up a rivalry between bex and cai" barDirectorWorld
      assertBool "bex is now annoyed at cai"
        (exists "bex.mood!annoyed.toward!cai" (db st))
      assertBool "a grievance is recorded"
        (exists "practice.greet.world.grievance.bex.cai" (db st))
      -- …and it is one dramatic beat: the same nudge is not offered again
      assertBool "the stir is one-shot for that pair"
        (null [ () | ga <- candidateActions st directorChar
                   , "rivalry between bex and cai" `isInfixOf` gaLabel ga ])

  , testCase "a DM nudge opens a new affordance for an autonomous character" $ do
      -- baseline: cold, bex cannot yet stand ada a drink (needs warmth ≥ 15)
      assertBool "bex can't buy ada a drink while cold"
        (not (any ("Buy ada a drink" `isInfixOf`) (bexCan barDirectorWorld)))
      -- the DM kindles warmth between them; now the affordance is available to bex
      let st = direct "kindle warmth between bex and ada" barDirectorWorld
      assertBool "after the DM kindles warmth, bex may buy ada a drink"
        (any ("Buy ada a drink" `isInfixOf`) (bexCan st))
  ]
