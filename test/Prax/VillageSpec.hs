module Prax.VillageSpec (tests) where

import           Data.List (isInfixOf)
import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (Val (..), exists, dbToSentences)
import           Prax.Query (Condition (..), groundCondition, query)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction, performOutcome, readView)
import           Prax.Loop (advance, npcAct)
import           Prax.Core (adjustScore)
import           Prax.Planner (predictMove)
import           Prax.Sight (sightName)
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
      -- 36 turns = the original 6 full rounds over the grown 6-member cast
      -- (24 was 6 rounds of the pre-ticker, pre-eve 4-member village).
      let st = driveIdle "you" 36 (doAct "bob" "steal the loaf" villageWorld)
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

  , testCase "the village keeps a perception clock and sightings" $ do
      -- after one full round of driveIdle (all six cast members -- you, bob,
      -- carol, dana, eve -- ending with the sight ticker), the perception
      -- clock has advanced and the square-mates (you, bob, and carol) hold
      -- sightings of each other; dana, at the mill the whole round, holds
      -- none of bob (and bob none of her).
      let st = driveIdle "you" 6 villageWorld
      assertBool "the clock ticked once" (exists "turn!1" (db st))
      assertBool "you sighted bob in the square"
        (exists "you.believes.at.bob!square" (db st))
      assertBool "bob sighted you back"
        (exists "bob.believes.at.you!square" (db st))
      assertBool "carol sighted bob"
        (exists "carol.believes.at.bob!square" (db st))
      assertBool "bob sighted carol back"
        (exists "bob.believes.at.carol!square" (db st))
      assertBool "dana, at the mill, holds no sighting of bob"
        (not (exists "dana.believes.at.bob" (db st)))
      assertBool "and bob holds none of dana"
        (not (exists "bob.believes.at.dana" (db st)))

  , testCase "out of sight, out of mind: an unsighted mover is not predicted" $ do
      -- dana holds a planted motive-belief that bob craves the loaf (not
      -- derived from gossip or sight — just planted directly, to isolate the
      -- scope gate). predictMove finds bob's motivated best move the moment
      -- we ask: the mind-reading itself is live and correct...
      let vocab = [ Desire "wantsLoaf" (Want [ Match "holding.Owner.loaf" ] 10) ]
          st0   = villageWorld { desires = vocab }
          st1   = performOutcome (Insert "dana.believes.desires.bob.wantsLoaf.seen") st0
          charByName n = case [ c | c <- characters st1, charName c == n ] of
            (c : _) -> c
            []      -> error ("no character named " ++ n)
          dana  = charByName "dana"
          bob   = charByName "bob"
          -- the wiring under test: villageWorld's own predictionScope,
          -- grounded to the dana/bob pair (the same check the planner's
          -- round-walk performs before ever calling predictMove).
          inScopeOf st actor witness =
            not (null (query (readView st) (groundedScope st actor witness) Map.empty))
          groundedScope st actor witness =
            map (groundCondition
                   (Map.fromList [ ("Actor", VStr actor), ("Witness", VStr witness) ]))
                (predictionScope st)
      fmap gaLabel (predictMove st1 dana bob) @?= Just "bob: steal the loaf from the stall"
      -- ...but dana (at the mill) has never sighted bob (at the square, and
      -- the clock has never ticked): out of scope, so the planner's
      -- round-walk would never call predictMove for him at all.
      assertBool "unsighted: dana is out of bob's prediction scope"
        (not (inScopeOf st1 "dana" "bob"))
      -- one shared-room tick: dana walks to the square (co-presence with bob)
      -- and the perception clock ticks once, both bringing her into scope
      -- directly (co-presence, "together") and depositing a fresh sighting
      -- ("sightedWithin") that would outlast the moment.
      let st2 = performOutcome (Insert "practice.world.world.at.dana!square") st1
          st3 = case possibleActions st2 sightName of
                  (ga : _) -> performAction st2 ga
                  []       -> error "the sight ticker has no action"
      assertBool "co-present after the shared-room tick: dana is now in scope"
        (inScopeOf st3 "dana" "bob")

  , testCase "a secret keeps: bob will not steal while the square watches" $ do
      let st = driveIdle "you" 20 villageWorld
      assertBool "the loaf is still on the stall" (exists "stall.loaf" (db st))
      assertBool "no one believes any theft by bob"
        (not (any (\w -> exists (w ++ ".believes.stole.bob.loaf") (db st))
                  ["you", "carol", "dana", "eve"]))

  , testCase "the perfect crime: alone, bob steals and no one ever knows" $ do
      let st0 = doAct "carol" "Go to mill" (doAct "you" "Go to mill" villageWorld)
          st  = driveIdle "you" 12 st0
      assertBool "bob took it" (exists "holding.bob.loaf" (db st))
      assertBool "nobody saw"
        (not (any (\w -> exists (w ++ ".believes.stole.bob.loaf") (db st))
                  ["you", "carol", "dana", "eve"]))
      assertBool "no standing about bob ever derives"
        (not (exists "regards.carol.bob.thief" (readView st)))

  , testCase "the frame-up: eve's whisper becomes reputation and shunning" $ do
      let st = driveIdle "you" 40 villageWorld
      assertBool "dana heard the lie from eve"
        (exists "dana.believes.stole.carol.loaf.heard.eve" (db st))
      assertBool "the falsehood settled into standing"
        (exists "regards.dana.carol.thief" (readView st))
      assertBool "carol ends up wrongly shunned"
        (exists "shunned.dana.carol" (db st))

  , testCase "the framed have no amends: carol is offered no return" $ do
      let st = driveIdle "you" 40 villageWorld
      assertBool "carol was framed" (exists "regards.dana.carol.thief" (readView st))
      assertBool "carol cannot 'return' a loaf she never took"
        (not (any (("return the loaf" `isInfixOf`) . gaLabel)
                  (possibleActions st "carol")))
  ]
