module Prax.ScriptSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Either (isLeft)
import           Data.List (isInfixOf)
import           Data.Maybe (isJust, listToMaybe)
import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists, unify, valToString)
import           Prax.Query (Condition (..), CmpOp (..))
import           Prax.Sym (intern)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction, performOutcome, roundBoundary)
import           Prax.Core (adjustScore)
import           Prax.Loop (advance, npcAct)
import           Prax.Persist (serializeState, deserializeState)
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
      -- v46: there is no story manager; the cast is only real characters
      assertBool "no _narrator in the cast"
        ("_narrator" `notElem` map charName (characters playWorld))

  , testCase "a beat fires only in its scene and applies its effects" $ do
      -- in the opening scene Cassia's only move is to confide; it sets the facts
      let confide = actionMatching "cassia" "confide" playWorld
          st1     = performAction playWorld confide
      assertBool "confided asserted"   (exists "confided" (db st1))
      assertBool "marcus now knows"    (exists "marcusKnows" (db st1))
      -- the banquet's poison beat is NOT available while 'confidence' is current
      assertBool "no banquet beats yet"
        (null [ ga | ga <- possibleActions st1 "cassia", "poison" `isInfixOf` gaLabel ga ])

  , testCase "a transition junction fires silently at the round boundary (no story manager)" $ do
      let confide = actionMatching "cassia" "confide" playWorld
          st1     = performAction playWorld confide     -- confided holds
          st2     = roundBoundary st1                   -- the engine fires the story rule
      currentSceneOf st2 @?= Just "banquet"             -- scene advanced, no actor took a turn

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
  , testCase "a timed junction times out after N turns of inaction" $ do
      let sc = Script
            { scriptStart = "wait", scriptCast = [ player "p" ]
            , scriptScenes = [ (scene "wait") { sceneJunctions = [ timeout "gaveUp" 3 ] } ] }
          driven = driveIdle Nothing "p" 30 (compile sc)   -- nobody acts; the clock runs out
      endingOf driven @?= Just "gaveUp"

    -- The story law: one boundary, the existing schedule machinery (v46) ------
    -- Every clause carries its own gates; the executor threads state between
    -- clauses, so eviction and Absent-ending decide firing, not any mode.
  , testCase "same-scene co-enabled junctions: first in authored order fires, the eviction masks the second" $ do
      -- Both routes out of 's' are enabled at once; 'toX' is authored first, so
      -- it fires and its currentScene eviction masks 'toY' in the same boundary.
      let sc = Script
            { scriptStart = "s", scriptCast = [ player "p" ]
            , scriptScenes =
                [ (scene "s") { sceneJunctions =
                    [ goto "toX" "x" [ Match "go" ], goto "toY" "y" [ Match "go" ] ] }
                , scene "x", scene "y" ] }
          st = roundBoundary (performOutcome (Insert "go") (compile sc))
      currentSceneOf st @?= Just "x"          -- not "y": authored order + eviction

  , testCase "authored order, not alphabetical label order, resolves a simultaneous enable" $ do
      -- 'zzz' fires before 'aaa' though it sorts later — the old tiebreak was
      -- an accident of the planner's alphabetical sort; authored order is a
      -- statement. Alphabetical order would land on "y".
      let sc = Script
            { scriptStart = "s", scriptCast = [ player "p" ]
            , scriptScenes =
                [ (scene "s") { sceneJunctions =
                    [ goto "zzz" "x" [ Match "go" ], goto "aaa" "y" [ Match "go" ] ] }
                , scene "x", scene "y" ] }
          st = roundBoundary (performOutcome (Insert "go") (compile sc))
      currentSceneOf st @?= Just "x"

  , testCase "an ending masks every later clause in the same boundary" $ do
      -- 'endHere' fires (an ending) before 'toY'; the ending.E fact masks the
      -- transition's Absent-ending gate, so no scene change slips through.
      let sc = Script
            { scriptStart = "s", scriptCast = [ player "p" ]
            , scriptScenes =
                [ (scene "s") { sceneJunctions =
                    [ ending "endHere" [ Match "go" ], goto "toY" "y" [ Match "go" ] ] }
                , scene "y" ] }
          st = roundBoundary (performOutcome (Insert "go") (compile sc))
      endingOf st        @?= Just "endHere"
      currentSceneOf st  @?= Just "s"         -- the transition was masked

  , testCase "cross-scene cascade: a pass-through scene traverses in one boundary (documented eager semantics)" $ do
      -- Scene 'b's exit condition already holds on entry (both gates read 'go'),
      -- so a->b->c happens within one boundary: 'toB' fires, then 'toC' sees the
      -- fresh currentScene!b and fires too. The gates decide, not a mode.
      let sc = Script
            { scriptStart = "a", scriptCast = [ player "p" ]
            , scriptScenes =
                [ (scene "a") { sceneJunctions = [ goto "toB" "b" [ Match "go" ] ] }
                , (scene "b") { sceneJunctions = [ goto "toC" "c" [ Match "go" ] ] }
                , scene "c" ] }
          st = roundBoundary (performOutcome (Insert "go") (compile sc))
      currentSceneOf st @?= Just "c"          -- traversed a->b->c in one boundary

    -- Timed-junction / patience-marker compile guards (spec v50 T2) -----------
    -- The marker keys per (scene, junction-name), so junction names must be
    -- unique within a scene; a zero-delay "timed" junction is a plain junction
    -- and is where the marker form would diverge; and the compiler-owned
    -- scenePatience family is closed to authors (the collision hole).
  , testCase "compile rejects two junctions with the same name in one scene" $ do
      let sc = Script
            { scriptStart = "a", scriptCast = [ player "p" ]
            , scriptScenes =
                [ (scene "a") { sceneJunctions =
                    [ ending "dup" [ Match "x" ], ending "dup" [ Match "y" ] ] } ] }
      r <- try (evaluate (currentSceneOf (compile sc)))
      assertBool "duplicate junction names in one scene are rejected at compile"
        (isLeft (r :: Either ErrorCall (Maybe String)))

  , testCase "compile rejects a timeout with a zero delay (n=0 is a plain junction)" $ do
      let sc = Script
            { scriptStart = "a", scriptCast = [ player "p" ]
            , scriptScenes = [ (scene "a") { sceneJunctions = [ timeout "now" 0 ] } ] }
      r <- try (evaluate (currentSceneOf (compile sc)))
      assertBool "an n=0 timeout is rejected at compile"
        (isLeft (r :: Either ErrorCall (Maybe String)))

  , testCase "compile rejects an 'after' goto with a zero delay" $ do
      let sc = Script
            { scriptStart = "a", scriptCast = [ player "p" ]
            , scriptScenes = [ (scene "a") { sceneJunctions = [ after "toB" 0 "b" ] }
                             , scene "b" ] }
      r <- try (evaluate (currentSceneOf (compile sc)))
      assertBool "an n=0 after is rejected at compile"
        (isLeft (r :: Either ErrorCall (Maybe String)))

  , testCase "compile rejects an authored scenePatience read in a junction condition" $ do
      let sc = Script
            { scriptStart = "a", scriptCast = [ player "p" ]
            , scriptScenes =
                [ (scene "a") { sceneJunctions =
                    [ ending "e" [ Match "scenePatience.a.foo" ] ] } ] }
      r <- try (evaluate (currentSceneOf (compile sc)))
      assertBool "an authored scenePatience read (Match) is rejected at compile"
        (isLeft (r :: Either ErrorCall (Maybe String)))

  , testCase "compile rejects an authored scenePatience write in a scene setup" $ do
      let sc = Script
            { scriptStart = "a", scriptCast = [ player "p" ]
            , scriptScenes = [ (scene "a") { sceneSetup = [ Insert "scenePatience.a.foo" ] } ] }
      r <- try (evaluate (currentSceneOf (compile sc)))
      assertBool "an authored scenePatience write (Insert) is rejected at compile"
        (isLeft (r :: Either ErrorCall (Maybe String)))

  , testCase "compile rejects an authored scenePatience touch in a beat effect \
             \(a newly-swept list)" $ do
      let sc = Script
            { scriptStart = "a", scriptCast = [ player "p" ]
            , scriptScenes =
                [ (scene "a") { sceneBeats =
                    [ beat "meddle" [] [ Delete "scenePatience.a.foo" ] ] } ] }
      r <- try (evaluate (currentSceneOf (compile sc)))
      assertBool "an authored scenePatience touch in a beat effect (Delete) is rejected"
        (isLeft (r :: Either ErrorCall (Maybe String)))

  , testCase "compile rejects an authored scenePatience touch in a beat condition \
             \(a newly-swept list)" $ do
      let sc = Script
            { scriptStart = "a", scriptCast = [ player "p" ]
            , scriptScenes =
                [ (scene "a") { sceneBeats =
                    [ beat "peek" [ Match "scenePatience.a.foo" ] [] ] } ] }
      r <- try (evaluate (currentSceneOf (compile sc)))
      assertBool "an authored scenePatience read in a beat condition is rejected"
        (isLeft (r :: Either ErrorCall (Maybe String)))

  , testCase "compile rejects an authored scenePatience touch in a cast-desire \
             \condition (a newly-swept list, absence polarity)" $ do
      let sc = Script
            { scriptStart = "a"
            , scriptCast = [ player "p"
                           , member "d" `wanting` [ Want [ Not "scenePatience.a.foo" ] 5 ] ]
            , scriptScenes = [ scene "a" ] }
      r <- try (evaluate (currentSceneOf (compile sc)))
      assertBool "an authored scenePatience touch in a cast-desire condition (Not) is rejected"
        (isLeft (r :: Either ErrorCall (Maybe String)))

    -- v50: timed junctions ride patience markers (the scene stamp is gone) ----
    -- Each behaviour is driven boundary-by-boundary with 'roundBoundary' (the
    -- pure timing harness: nobody acts, the markers just expire on the clock).
  , testCase "the audience: 'dismissed' fires at boundary 5, not before (the fidelity pin)" $ do
      -- Byte-identical to the pre-move stamp: entry at boundary 0, timeout 5,
      -- so the patience runs out exactly at boundary 5. Exercises the START-scene
      -- entry path (the panel's Critical: marker emission must ride scene entry).
      let steps = iterate roundBoundary audienceWorld
      endingOf (steps !! 4) @?= Nothing              -- patience still holds
      endingOf (steps !! 5) @?= Just "dismissed"     -- boundary 5: it ran out

  , testCase "a timed 'after' goto fires at its delay boundary" $ do
      let sc = Script
            { scriptStart = "a", scriptCast = [ player "p" ]
            , scriptScenes = [ (scene "a") { sceneJunctions = [ after "toB" 3 "b" ] }
                             , scene "b" ] }
          steps = iterate roundBoundary (compile sc)
      currentSceneOf (steps !! 2) @?= Just "a"       -- not yet
      currentSceneOf (steps !! 3) @?= Just "b"       -- boundary 3: hands off

  , testCase "re-entry resets a timed junction: it times out n from the LAST entry" $ do
      let sc = Script
            { scriptStart = "a", scriptCast = [ player "p" ]
            , scriptScenes =
                [ (scene "a") { sceneJunctions =
                    [ goto "leave" "b" [ Match "leaveNow" ], timeout "out" 3 ] }
                , (scene "b") { sceneJunctions =
                    [ goto "back" "a" [ Match "backNow" ] ] } ] }
          w0  = compile sc
          w1  = roundBoundary w0                                    -- dawdle at a
          atB = roundBoundary (performOutcome (Insert "leaveNow") w1)  -- leave before timeout
          reA = roundBoundary (performOutcome (Insert "backNow")
                                 (performOutcome (Delete "leaveNow") atB))  -- re-enter a
      currentSceneOf atB @?= Just "b"
      currentSceneOf reA @?= Just "a"
      -- from the LAST entry the clock runs a fresh 3 boundaries, not from the first
      endingOf (roundBoundary (roundBoundary reA))                 @?= Nothing
      endingOf (roundBoundary (roundBoundary (roundBoundary reA))) @?= Just "out"

  , testCase "early exit is harmless: a pending patience marker fires no stray junction" $ do
      let sc = Script
            { scriptStart = "a", scriptCast = [ player "p" ]
            , scriptScenes =
                [ (scene "a") { sceneJunctions =
                    [ goto "leave" "b" [ Match "leaveNow" ], timeout "out" 3 ] }
                , scene "b" ] }
          w0  = compile sc
          w1  = roundBoundary w0                                    -- dawdle at a (marker mid-life)
          atB = roundBoundary (performOutcome (Insert "leaveNow") w1)  -- leave before the timeout
      currentSceneOf atB @?= Just "b"
      -- drive past the original due (boundary 3): the stale marker expires in 'b'
      -- with the currentScene gate false, so 'out' never fires
      endingOf (iterate roundBoundary atB !! 5) @?= Nothing

  , testCase "two timed junctions on one scene fire independently at their own delays" $ do
      -- distinct names (same-name is now rejected) and distinct delays; the
      -- shorter fires at ITS boundary, proving the markers are keyed separately
      -- (a collapsed single marker would delay 'quick' to 'slow's boundary).
      let sc = Script
            { scriptStart = "a", scriptCast = [ player "p" ]
            , scriptScenes = [ (scene "a") { sceneJunctions =
                                 [ after "quick" 2 "b", after "slow" 4 "c" ] }
                             , scene "b", scene "c" ] }
          steps = iterate roundBoundary (compile sc)
      currentSceneOf (steps !! 1) @?= Just "a"       -- boundary 1: neither yet
      currentSceneOf (steps !! 2) @?= Just "b"       -- boundary 2: 'quick' (delay 2) fired

  , testCase "mid-scene save/resume reaches the same timeout boundary (persistence symmetry)" $ do
      -- The patience marker is an ordinary fact; its pending expiry rides v44's
      -- due serialization, so a save partway through a timed scene resumes to the
      -- SAME dismissal boundary with no Persist code change.
      let mid     = iterate roundBoundary audienceWorld !! 2       -- boundary 2, patience pending
          resumed = deserializeState (serializeState mid) audienceWorld
      endingOf mid @?= Nothing
      endingOf (iterate roundBoundary resumed !! 2) @?= Nothing        -- boundary 4
      endingOf (iterate roundBoundary resumed !! 3) @?= Just "dismissed"  -- boundary 5

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
          -- enough round boundaries for the story rule to fire the ending;
          -- 'granted' (petitioned holds) lands well before the timeout clock
          -- could reach 'dismissed'.
      endingOf (driveIdle Nothing "envoy" 15 w2) @?= Just "granted"

  , testCase "the audience: the Duke's concern moves him (once)" $ do
      let driven = driveIdle Nothing "envoy" 6 audienceWorld
      assertBool "the Duke, concerned for favour, flattered unbidden"
                                                (exists "dukeSpoke" (db driven))
      assertBool "and the one-shot held: no flatter left on the Duke's menu"
        (not (any (("flatter" `isInfixOf`) . gaLabel) (possibleActions driven "duke")))
  ]
