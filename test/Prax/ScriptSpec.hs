module Prax.ScriptSpec (tests) where

import           Data.List (isInfixOf)
import           Data.Maybe (isJust, listToMaybe)
import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists, unify, valToString)
import           Prax.Query (Condition (..), CmpOp (..))
import           Prax.Sym (intern)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction)
import           Prax.Core (adjustScore)
import           Prax.Loop (advance, npcAct)
import           Prax.Script
import           Prax.Worlds.Play (playScript, playWorld)
import           Prax.Worlds.Audience (audienceWorld)

-- The ending reached, if any.
endingOf :: PraxState -> Maybe String
endingOf st =
  listToMaybe [ e | b <- unify "ending.E" (db st) Map.empty
                  , Just e <- [valToString <$> Map.lookup (intern "E") b] ]

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
      | isJust (endingOf st)              = st
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
          assertBool "narrator acted" (isJust mv)
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

    -- Prompter compilation features ------------------------------------------
  , testCase "a memory fires once, the first time its trigger holds" $ do
      let sc = Script
            { scriptStart = "s"
            , scriptCast  = [ player "p", member "npc" `wanting` [ Want [ Match "done" ] 10 ] ]
            , scriptScenes =
                [ (scene "s")
                    { sceneBeats = [ quip "npc" "[Actor]: act"
                                       [ Not "done" ] [ Insert "done", Insert "trigger" ] ]
                    , sceneMemories = [ memory "RECALL" [ Match "trigger" ] ] } ] }
          driven = driveIdle Nothing "p" 12 (compile sc)
      assertBool "npc acted (trigger set)" (exists "trigger" (db driven))
      assertBool "memory fired"            (exists "memoryFired.s_mem0" (db driven))
      assertBool "one-shot: memory no longer on offer"
        (not (any (("RECALL" `isInfixOf`) . gaLabel) (possibleActions driven narratorName)))

  , testCase "a timed junction times out after N turns of inaction" $ do
      let sc = Script
            { scriptStart = "wait", scriptCast = [ player "p" ]
            , scriptScenes = [ (scene "wait") { sceneJunctions = [ timeout "gaveUp" 3 ] } ] }
          driven = driveIdle Nothing "p" 30 (compile sc)   -- nobody acts; the clock runs out
      endingOf driven @?= Just "gaveUp"

  , testCase "a character sketch compiles concerns to wants and traits to facts" $ do
      let cm = member "vain" `concernedWith` [ ("beauty", 50) ] `withTraits` [ "proud" ]
      castDesires cm @?=
        [ Want [ Match "Other.relationship.vain.beauty.score!N"
               , Neq "Other" "vain", Cmp Gt "N" "0" ] 50 ]
      castTraits cm @?= [ "proud" ]

  , testCase "a concern actually drives behaviour" $ do
      let sc = Script
            { scriptStart = "s"
            , scriptCast  = [ player "p", member "vain" `concernedWith` [ ("beauty", 50) ] ]
            , scriptScenes =
                [ (scene "s")
                    { sceneBeats = [ quip "vain" "[Actor]: preen for p"
                                       [ Not "preened" ]
                                       [ Insert "preened"
                                       , adjustScore "p" "vain" "beauty" 5 "dazzled" ] ] } ] }
          w0 = compile sc
          driven = driveIdle Nothing "p" 12 w0
      assertBool "trait/desire wiring lets vain be moved by regard: it preens"
        (exists "preened" (db driven))

  , testCase "same-labelled quips by different speakers dispatch to their own effects" $ do
      -- Two quips share the display text "[Actor]: act"; performing b's must run
      -- b's outcome, not a's. (A quip is a specific speaker's action, so the
      -- compiled action id must distinguish the speakers.)
      let sc = Script
            { scriptStart = "s", scriptCast = [ member "a", member "b" ]
            , scriptScenes =
                [ (scene "s")
                    { sceneBeats =
                        [ quip "a" "[Actor]: act" [] [ Insert "aDid" ]
                        , quip "b" "[Actor]: act" [] [ Insert "bDid" ] ] } ] }
          w      = compile sc
          afterB = performAction w (actionMatching "b" "act" w)
      assertBool "b's beat set b's fact"        (exists "bDid" (db afterB))
      assertBool "b's beat did NOT run a's beat" (not (exists "aDid" (db afterB)))

    -- The audience: one story that uses all three features together ----------
  , testCase "the audience: dawdling lets the clock run out to 'dismissed'" $ do
      let driven = driveIdle Nothing "envoy" 40 audienceWorld
      endingOf driven @?= Just "dismissed"

  , testCase "the audience: flatter then petition reaches 'granted'" $ do
      let w0 = audienceWorld
          w1 = performAction w0 (actionMatching "envoy" "flatter" w0)
          w2 = performAction w1 (actionMatching "envoy" "petition" w1)
          -- enough narrator turns for both the pending memory and the ending to
          -- fire (they both count as advancing the story); granted lands well
          -- before the timeout clock could reach 'dismissed'.
      endingOf (driveIdle Nothing "envoy" 15 w2) @?= Just "granted"

  , testCase "the audience: memory fires and the Duke's concern moves him (once)" $ do
      let driven = driveIdle Nothing "envoy" 6 audienceWorld
      assertBool "memory fired"                 (exists "memoryFired.audience_mem0" (db driven))
      assertBool "the Duke, concerned for favour, flattered unbidden"
                                                (exists "dukeSpoke" (db driven))
      assertBool "and the one-shot held: no flatter left on the Duke's menu"
        (not (any (("flatter" `isInfixOf`) . gaLabel) (possibleActions driven "duke")))
  ]
