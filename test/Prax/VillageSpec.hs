module Prax.VillageSpec (tests) where

import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool)

import           Prax.Db (exists, dbToSentences)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction, performOutcome)
import           Prax.Loop (advance, npcAct)
import           Prax.Core (adjustScore)
import           Prax.Worlds.Village

-- Perform the named actor's action whose label mentions @needle@.
doAct :: String -> String -> PraxState -> PraxState
doAct who needle st =
  case [ ga | ga <- possibleActions st who, needle `isInfixOf` gaLabel ga ] of
    (ga : _) -> performAction st ga
    []       -> error ("no action for " ++ who ++ " matching " ++ show needle
                       ++ "; had: " ++ show (map gaLabel (possibleActions st who)))

-- Run @k@ turns with everyone planner-driven except @idle@, who waits.
driveIdle :: String -> Int -> PraxState -> PraxState
driveIdle idle = go
  where
    go 0 st = st
    go k st =
      let (actor, st1) = advance st
          st2 | charName actor == idle = st1
              | otherwise              = snd (npcAct 2 actor st1)
      in go (k - 1) st2

tests :: TestTree
tests = testGroup "Prax.Worlds.Village"
  [ testCase "the theft is witnessed by the square, not the mill" $ do
      let st = doAct "bob" "steal the loaf" villageWorld
      assertBool "carol (in the square) saw it"
        (exists "carol.believes.stole.bob.loaf.seen" (db st))
      assertBool "you (in the square) saw it"
        (exists "you.believes.stole.bob.loaf.seen" (db st))
      assertBool "dana (at the mill) holds no such belief"
        (not (exists "dana.believes.stole.bob.loaf.seen" (db st)))
      assertBool "bob is not his own witness"
        (not (exists "bob.believes.stole.bob.loaf.seen" (db st)))

  , testCase "movement is not news (undeclared actions deposit nothing)" $ do
      let st = doAct "bob" "Go to mill" villageWorld
      assertBool "no one 'believes' bob walked"
        (not (any (\w -> exists (w ++ ".believes.went.bob.seen") (db st))
                  ["you", "carol", "dana"]))

  , testCase "only a witness can confront the thief" $ do
      let st = doAct "bob" "steal the loaf" villageWorld
      assertBool "carol can confront"
        (any (("confront bob" `isInfixOf`) . gaLabel) (possibleActions st "carol"))
      assertBool "dana cannot"
        (not (any (("confront bob" `isInfixOf`) . gaLabel) (possibleActions st "dana")))

  , testCase "confronting cools the witness toward the thief, once" $ do
      let st  = doAct "carol" "confront bob" (doAct "bob" "steal the loaf" villageWorld)
      assertBool "trust dropped"
        ("carol.relationship.bob.trust.score.-10" `elem` dbToSentences (db st))
      assertBool "confront is one-shot"
        (not (any (("confront bob" `isInfixOf`) . gaLabel) (possibleActions st "carol")))

  , testCase "the rumor spreads on its own: carol carries the news to the mill" $ do
      let st = driveIdle "you" 24 (doAct "bob" "steal the loaf" villageWorld)
      assertBool "dana heard it from carol"
        (exists "dana.believes.stole.bob.loaf.heard.carol" (db st))

  , testCase "the arc completes on its own: dana eventually eyes bob" $ do
      let st = driveIdle "you" 40 (doAct "bob" "steal the loaf" villageWorld)
      assertBool "dana acted on the hearsay"
        (exists "eyed.dana.bob" (db st))

  , testCase "hearsay licenses suspicion, not confrontation" $ do
      let st0 = doAct "bob" "steal the loaf" villageWorld
      assertBool "carol (eyewitness) is never offered mere suspicion"
        (not (any (("eye bob" `isInfixOf`) . gaLabel) (possibleActions st0 "carol")))
      let st1 = doAct "carol" "tell dana" (doAct "carol" "Go to mill" st0)
          st2 = doAct "dana" "Go to square" st1
      assertBool "dana (hearsay) can eye bob"
        (any (("eye bob" `isInfixOf`) . gaLabel) (possibleActions st2 "dana"))
      assertBool "dana still cannot confront"
        (not (any (("confront bob" `isInfixOf`) . gaLabel) (possibleActions st2 "dana")))
      let st3 = doAct "dana" "eye bob" st2
      assertBool "milder trust hit"
        ("dana.relationship.bob.trust.score.-5" `elem` dbToSentences (db st3))
      assertBool "suspicion is one-shot"
        (not (any (("eye bob" `isInfixOf`) . gaLabel) (possibleActions st3 "dana")))

  , testCase "seen suppresses suspicion even alongside hearsay" $ do
      -- Construct the mixed-evidence state directly: carol saw it AND (planted)
      -- heard it. The Absent[.seen] gate — not the lack of hearsay — must be
      -- what blocks her suspicion affordance.
      let st = performOutcome (Insert "carol.believes.stole.bob.loaf.heard.you")
                 (doAct "bob" "steal the loaf" villageWorld)
      assertBool "carol still confronts rather than merely suspects"
        (not (any (("eye bob" `isInfixOf`) . gaLabel) (possibleActions st "carol")))

  , testCase "the distrust gate closes the village's gossip channel" $ do
      -- "you" starts in the square beside bob, so "you" is always an
      -- eyewitness the instant the theft happens (see "the theft is witnessed
      -- by the square" above) and Prax.Rumor's landed "no news value to an
      -- eyewitness" gate (RumorSpec: "a hearer who saw the event is not
      -- told") means carol can NEVER offer to tell "you" — trust or no trust.
      -- dana, elsewhere at the moment of the theft, is the only character who
      -- is ever a valid (non-witness) gossip hearer, so she is who this gate
      -- is exercised against; carol must first reach her.
      let st0 = doAct "carol" "Go to mill" (doAct "bob" "steal the loaf" villageWorld)
      assertBool "carol will tell dana while trust is unmarred"
        (any (("tell dana" `isInfixOf`) . gaLabel) (possibleActions st0 "carol"))
      let st1 = performOutcome (adjustScore "carol" "dana" "trust" (-5) "aSlight") st0
      assertBool "distrust closes the channel"
        (not (any (("tell dana" `isInfixOf`) . gaLabel) (possibleActions st1 "carol")))
  ]
