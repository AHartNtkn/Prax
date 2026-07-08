module Prax.ScriptSpec (tests) where

import           Data.List (isInfixOf)
import           Data.Maybe (listToMaybe)
import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists, unify, valToString)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction)
import           Prax.Loop (advance, npcAct)
import           Prax.Script
import           Prax.Worlds.Play (playScript, playWorld)

-- The ending reached, if any.
endingOf :: PraxState -> Maybe String
endingOf st =
  listToMaybe [ e | b <- unify "ending.E" (db st) Map.empty
                  , Just e <- [valToString <$> Map.lookup "E" b] ]

-- One of a character's currently-available actions whose label mentions @needle@.
actionMatching :: String -> String -> PraxState -> GroundedAction
actionMatching who needle st =
  case [ ga | ga <- possibleActions st who, needle `isInfixOf` gaLabel ga ] of
    (ga : _) -> ga
    []       -> error ("no action for " ++ who ++ " matching " ++ show needle)

-- Run the simulation with @idle@ (the player) never acting and everyone else
-- driven by the planner, until an ending, until @target@ becomes the current
-- scene, or for @k@ advances — whichever comes first.
driveIdle :: Maybe String -> String -> Int -> PraxState -> PraxState
driveIdle target idle = go
  where
    go 0 st = st
    go k st
      | endingOf st /= Nothing            = st
      | currentSceneOf st == target       = st
      | otherwise =
          let (actor, st1) = advance st
              st2 | charName actor == idle = st1                -- the player idles
                  | otherwise              = snd (npcAct 2 actor st1)
          in go (k - 1) st2

tests :: TestTree
tests = testGroup "Prax.Script"
  [ testCase "compile: the start scene is active and the cast is present" $ do
      currentSceneOf playWorld @?= Just "confidence"
      assertBool "marcus present" (exists "character.marcus" (db playWorld))
      assertBool "cassia present" (exists "character.cassia" (db playWorld))
      -- the narrator is a cast member (the story manager) but not a fact-character
      assertBool "narrator in cast"
        (narratorName `elem` map charName (characters playWorld))

  , testCase "a beat fires only in its scene and applies its effects" $ do
      -- in the opening scene Cassia's only move is to confide; it sets the facts
      let confide = actionMatching "cassia" "confide" playWorld
          st1     = performAction playWorld confide
      assertBool "confided asserted"   (exists "confided" (db st1))
      assertBool "marcus now knows"    (exists "marcusKnows" (db st1))
      -- the banquet's poison beat is NOT available while 'confidence' is current
      assertBool "no banquet beats yet"
        (null [ ga | ga <- possibleActions st1 "cassia", "poison" `isInfixOf` gaLabel ga ])

  , testCase "a transition junction is fired automatically by the narrator" $ do
      let confide = actionMatching "cassia" "confide" playWorld
          st1     = performAction playWorld confide     -- confided holds
      case [ c | c <- characters st1, charName c == narratorName ] of
        (narr : _) -> do
          let (mv, st2) = npcAct 2 narr st1
          assertBool "narrator acted" (mv /= Nothing)
          currentSceneOf st2 @?= Just "banquet"         -- scene advanced
        [] -> assertBool "narrator should be in the cast" False

  , testCase "idle player: the plot runs to betrayal across two scenes" $ do
      let st = driveIdle Nothing "marcus" 30 playWorld
      endingOf st @?= Just "betrayal"
      currentSceneOf st @?= Just "banquet"              -- the transition happened

  , testCase "the player can warn: ending loyalty" $ do
      let atBanquet = driveIdle (Just "banquet") "marcus" 20 playWorld
      currentSceneOf atBanquet @?= Just "banquet"
      let warned = performAction atBanquet (actionMatching "marcus" "warn" atBanquet)
          st     = driveIdle Nothing "marcus" 20 warned
      endingOf st @?= Just "loyalty"

  , testCase "the player can strike first: ending complicity" $ do
      let atBanquet = driveIdle (Just "banquet") "marcus" 20 playWorld
          killed = performAction atBanquet (actionMatching "marcus" "own hand" atBanquet)
          st     = driveIdle Nothing "marcus" 20 killed
      endingOf st @?= Just "complicity"

  , testCase "the player can romance the conspirator (as in Intrigue)" $ do
      let atBanquet = driveIdle (Just "banquet") "marcus" 20 playWorld
          loved = performAction atBanquet (actionMatching "marcus" "charms" atBanquet)
      assertBool "Marcus and Cassia become lovers"
        (exists "bond.marcus.cassia.lovers" (db loved))

  , testCase "flowChart names every scene and junction" $ do
      let chart = flowChart playScript
      mapM_ (\needle -> assertBool (needle ++ " in chart") (needle `isInfixOf` chart))
            [ "confidence", "banquet", "toBanquet"
            , "betrayal", "loyalty", "complicity", "graph TD" ]
  ]
