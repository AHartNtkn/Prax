module Prax.VillageSpec (tests) where

import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool)

import           Prax.Db (exists, dbToSentences)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction, performOutcome, readView)
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

  , testCase "three regards make notoriety; standing has teeth" $ do
      let st0 = doAct "bob" "steal the loaf" villageWorld
      assertBool "two witnesses are not notoriety"
        (not (exists "notorious.bob.thief" (readView st0)))
      let st = doAct "carol" "tell dana" (doAct "carol" "Go to mill" st0)
          v  = readView st
      assertBool "carol regards bob a thief" (exists "regards.carol.bob.thief" v)
      assertBool "dana (hearsay) regards too" (exists "regards.dana.bob.thief" v)
      assertBool "you (eyewitness) regard"    (exists "regards.you.bob.thief" v)
      assertBool "bob holds no self-regard"   (not (exists "regards.bob.bob.thief" v))
      assertBool "the whole village knows: notorious" (exists "notorious.bob.thief" v)
      assertBool "carol may shun bob"
        (any (("shun bob" `isInfixOf`) . gaLabel) (possibleActions st "carol"))
      assertBool "bob may return the loaf"
        (any (("return the loaf" `isInfixOf`) . gaLabel) (possibleActions st "bob"))

  , testCase "amends is only offered under someone's regard" $ do
      let st = performOutcome (Insert "holding.bob.loaf") villageWorld
      assertBool "no regard, no apology affordance"
        (not (any (("return the loaf" `isInfixOf`) . gaLabel) (possibleActions st "bob")))

  , testCase "atonement dissolves standing while memory persists" $ do
      let st0 = doAct "bob" "steal the loaf" villageWorld
          st1 = doAct "carol" "shun bob" st0
          st2 = doAct "bob" "return the loaf" st1
          v   = readView st2
      assertBool "atoned" (exists "atoned.bob" (db st2))
      assertBool "no regard survives"    (not (exists "regards.carol.bob.thief" v))
      assertBool "no notoriety survives" (not (exists "notorious.bob.thief" v))
      assertBool "carol still remembers seeing it"
        (exists "carol.believes.stole.bob.loaf.seen" (db st2))
      assertBool "carol may now relent"
        (any (("relent toward bob" `isInfixOf`) . gaLabel) (possibleActions st2 "carol"))
      assertBool "the stall is restocked" (exists "stall.loaf" (db st2))

  , testCase "the whole arc runs itself: notoriety tips bob; forgiveness follows" $ do
      let st = driveIdle "you" 60 (doAct "bob" "steal the loaf" villageWorld)
          v  = readView st
      assertBool "bob atoned on his own" (exists "atoned.bob" (db st))
      assertBool "no regard survives"    (not (exists "regards.carol.bob.thief" v))
      assertBool "every shun relented"
        (not (any (\w -> exists ("shunned." ++ w ++ ".bob") (db st))
                  ["you", "carol", "dana"]))
      assertBool "memory persists throughout"
        (exists "carol.believes.stole.bob.loaf.seen" (db st))

  , testCase "re-offense revokes atonement: standing snaps back from memory" $ do
      let st0 = doAct "bob" "steal the loaf" villageWorld
          st1 = doAct "carol" "tell dana" (doAct "carol" "Go to mill" st0)
          st2 = doAct "bob" "return the loaf" st1
      assertBool "atoned, no standing" (not (exists "regards.carol.bob.thief" (readView st2)))
      let st3 = doAct "bob" "steal the loaf" st2
          v3  = readView st3
      assertBool "atonement revoked" (not (exists "atoned.bob" (db st3)))
      assertBool "carol's regard is back — nobody forgot anything"
        (exists "regards.carol.bob.thief" v3)
      assertBool "notoriety is back too" (exists "notorious.bob.thief" v3)

  , testCase "an atoned thief is deterred: the planner sees the snap-back" $ do
      -- run the whole arc, then keep driving: the stall is restocked, bob's
      -- loaf-want is live again, but stealing would instantly restore his
      -- notoriety (-15 > +10) — so he never takes it.
      let st = driveIdle "you" 90 (doAct "bob" "steal the loaf" villageWorld)
      assertBool "bob atoned along the way" (exists "atoned.bob" (db st))
      assertBool "the loaf is still on the stall" (exists "stall.loaf" (db st))
      assertBool "bob holds no loaf" (not (exists "holding.bob.loaf" (db st)))
  ]
