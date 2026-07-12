module Prax.VillageSpec (tests) where

import           Data.List (isInfixOf, isPrefixOf)
import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (Val (..), exists, dbToSentences)
import           Prax.Query (Condition (..), groundCondition, query)
import           Prax.Sym (intern)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction, performOutcome, setDesires)
import           Prax.Loop (advance, npcAct)
import           Prax.Core (adjustScore)
import           Prax.Planner (predictMove, pickAction)
import           Prax.Sight (sightName)
import           Prax.Worlds.Village

-- Perform the named actor's action whose label mentions @needle@.
doAct :: String -> String -> PraxState -> PraxState
doAct who needle st =
  case [ ga | ga <- possibleActions st who, needle `isInfixOf` gaLabel ga ] of
    (ga : _) -> performAction st ga
    []       -> error ("no action for " ++ who ++ " matching " ++ show needle
                       ++ "; had: " ++ show (map gaLabel (possibleActions st who)))

-- One planner-driven turn, with @idle@'s turn consumed but not acted.
idleStep :: String -> PraxState -> PraxState
idleStep idle st =
  let (actor, st1) = advance st
  in if charName actor == idle then st1 else snd (npcAct 2 actor st1)

-- Run @k@ turns with everyone planner-driven except @idle@, who waits.
driveIdle :: String -> Int -> PraxState -> PraxState
driveIdle idle n st = iterate (idleStep idle) st !! n

-- The suite's two long trajectories, shared across tests: the planner is
-- deterministic, so a test wanting the state after N turns reads a snapshot
-- of the one trace instead of re-simulating the same N turns privately.
-- Top-level sharing: each trace is computed once per test-suite run.
freePlayAt :: Int -> PraxState
freePlayAt = (trace !!)
  where trace = iterate (idleStep "you") villageWorld

postTheftAt :: Int -> PraxState
postTheftAt = (trace !!)
  where trace = iterate (idleStep "you") (doAct "bob" "steal the loaf" villageWorld)

-- Fire the sight ticker directly (VillageSpec's own idiom, "out of sight,
-- out of mind"): makes a just-established co-presence concrete in each
-- character's own memory before the next decision is scored.
tick :: PraxState -> PraxState
tick st = case possibleActions st sightName of
  (ga : _) -> performAction st ga
  []       -> error "no sight action available"

-- The shakedown's forced trajectory (the theft tests' own convention: a
-- scripted opening, then free play). gale steps out of the mill first --
-- otherwise she'd be a third simultaneous witness to eve's whisper, tripping
-- notoriety at the instant of witnessing rather than leaving it "one
-- predicted exposure from the brink." Carol then arrives and witnesses
-- directly (no gossip relay needed); both return to the square, where real
-- bystanders (bob, you) make carol's exposure threat credible rather than
-- empty (the mill, once gale and dana already know, offers no one left to
-- expose to). The sight ticks after each move are what let each
-- character's own round-walk correctly price a threat from someone just
-- out of the room, not only someone still in it.
whisperArcSetup :: PraxState
whisperArcSetup = foldl (flip ($)) villageWorld
  [ tick . doAct "gale" "Go to square"
  , tick . doAct "carol" "Go to mill"
  , tick . doAct "eve" "whisper to dana that"
  , tick . doAct "carol" "Go to square"
  , tick . doAct "eve" "Go to square"
  ]

whisperArcAt :: Int -> PraxState
whisperArcAt n = iterate (idleStep "you") whisperArcSetup !! n

-- The named villager from the cast.
villager :: String -> Character
villager n = case [ c | c <- characters villageWorld, charName c == n ] of
  (c : _) -> c
  []      -> error ("no such villager: " ++ n)

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
      -- 42 turns: 6 full rounds over the 7-member v25 cast
      let st = postTheftAt 42
      assertBool "dana heard it from carol"
        (exists "dana.believes.stole.bob.loaf.heard.carol" (db st))

  , testCase "the arc completes on its own: dana eventually eyes bob" $ do
      -- 49 turns: 7 rounds
      let st = postTheftAt 49
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
      -- 70 turns: 10 rounds
      let st = postTheftAt 70
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
      -- amended under v24 (spec §3): asserts non-re-offense directly, not the
      -- old holds-no-loaf proxy.
      -- run the whole arc, then keep driving: the stall is restocked, bob's
      -- loaf-want is live again, but stealing would instantly restore his
      -- notoriety (-15 > +10) — so he never takes it.
      -- 105 turns: 15 rounds
      let st = postTheftAt 105
      assertBool "his atonement stands" (exists "atoned.bob" (db st))
      assertBool "the stall's loaf is untouched" (exists "stall.loaf" (db st))
      -- "bob holds no loaf" was a proxy for "he never re-steals"; v24's
      -- redemption falsifies the proxy (bob EARNS a loaf via earnBread) while
      -- strengthening the property. A re-steal would falsify both asserts
      -- above (it deletes stall.loaf AND atoned.bob); the loaf bob holds is
      -- the one he earned:
      assertBool "the loaf he holds is the one he earned"
        (exists "practice.earnBread.bob.done.s3" (db st))
      -- and the sharpest check: a re-steal at ANY point would have revoked
      -- the atonement and revived his notoriety (three believers never
      -- forgot); the derived view stays clear of it.
      assertBool "his notoriety never returned"
        (not (exists "notorious.bob.thief" (readView st)))

  , testCase "the village keeps a perception clock and sightings" $ do
      -- after one full round of free play (all seven cast members -- you, bob,
      -- carol, dana, eve, gale -- ending with the sight ticker), the perception
      -- clock has advanced and the square-mates (you, bob, and carol) hold
      -- sightings of each other; dana, at the mill the whole round, holds
      -- none of bob (and bob none of her).
      let st = freePlayAt 7
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
          st0   = setDesires vocab villageWorld
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
                   (Map.fromList [ (intern "Actor", VSym (intern actor))
                                 , (intern "Witness", VSym (intern witness)) ]))
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
      -- 28 turns: 4 rounds
      let st = freePlayAt 28
      assertBool "the loaf is still on the stall" (exists "stall.loaf" (db st))
      assertBool "no one believes any theft by bob"
        (not (any (\w -> exists (w ++ ".believes.stole.bob.loaf") (db st))
                  ["you", "carol", "dana", "eve", "gale"]))

  , testCase "the perfect crime: alone, bob steals and no one ever knows" $ do
      -- 14 turns: 2 rounds
      let st0 = doAct "carol" "Go to mill" (doAct "you" "Go to mill" villageWorld)
          st  = driveIdle "you" 14 st0
      assertBool "bob took it" (exists "holding.bob.loaf" (db st))
      assertBool "nobody saw"
        (not (any (\w -> exists (w ++ ".believes.stole.bob.loaf") (db st))
                  ["you", "carol", "dana", "eve", "gale"]))
      assertBool "no standing about bob ever derives"
        (not (exists "regards.carol.bob.thief" (readView st)))

  , testCase "the frame-up: eve's whisper becomes reputation and shunning" $ do
      let st = freePlayAt 40
      assertBool "dana heard the lie from eve"
        (exists "dana.believes.stole.carol.loaf.heard.eve" (db st))
      assertBool "the falsehood settled into standing"
        (exists "regards.dana.carol.thief" (readView st))
      assertBool "carol ends up wrongly shunned"
        (exists "shunned.dana.carol" (db st))

  , testCase "the framed have no amends: carol is offered no return" $ do
      let st = freePlayAt 40
      assertBool "carol was framed" (exists "regards.dana.carol.thief" (readView st))
      assertBool "carol cannot 'return' a loaf she never took"
        (not (any (("return the loaf" `isInfixOf`) . gaLabel)
                  (possibleActions st "carol")))

  , testCase "deterrence plus opportunity yields industry: watched bob earns his loaf" $ do
      -- from t=0 free play, with the whole square watching, bob undertakes
      -- honest work and completes it: he ends holding a loaf he BAKED —
      -- the stall's loaf untouched, no theft beliefs about him anywhere.
      -- 49 turns: 7 rounds
      let st = freePlayAt 49
      assertBool "bob undertook the endeavor" (exists "practice.earnBread.bob" (db st))
      assertBool "and finished it" (exists "practice.earnBread.bob.done.s3" (db st))
      assertBool "he holds a loaf" (exists "holding.bob.loaf" (db st))
      assertBool "the stall's loaf untouched" (exists "stall.loaf" (db st))
      assertBool "no one believes any theft by bob"
        (not (any (\w -> exists (w ++ ".believes.stole.bob.loaf") (db st))
                  ["you", "carol", "dana", "eve", "gale"]))

  , testCase "the opportunism stays honest: an empty square mid-project still tempts" $ do
      -- bob has undertaken and swept; then the square empties. Stealing (+10,
      -- secret kept) beats the next +3 stage — industry under observation,
      -- larceny in the dark.
      let st0 = doAct "bob" "sweep the square"
                  (doAct "bob" "take up honest work" villageWorld)
          st1 = doAct "carol" "Go to mill" (doAct "you" "Go to mill" st0)
      fmap gaLabel (pickAction 2 st1 (villager "bob"))
        @?= Just "bob: steal the loaf from the stall"

  , testCase "watching him work teaches the village his purpose" $ do
      -- carol witnesses the sweep -> the inference axiom presumes his pursuit.
      -- predictMove is MYOPIC, so the flour prediction fires once bob stands
      -- at the mill (the model must gain from an AVAILABLE move). dana never
      -- saw the sweep — co-present with bob at the mill, she STILL predicts
      -- nothing: prediction is belief-relative, not proximity-relative.
      let st0 = doAct "bob" "sweep the square"
                  (doAct "bob" "take up honest work" villageWorld)
      assertBool "carol saw the sweep" (exists "carol.believes.swept.bob.seen" (db st0))
      assertBool "and presumes the pursuit"
        (exists "carol.believes.desires.bob.pursues-earnBread.presumed" (readView st0))
      -- at the square, even carol's model gains from no available move:
      predictMove st0 (villager "carol") (villager "bob") @?= Nothing
      let st1 = doAct "bob" "Go to mill" st0
      fmap gaLabel (predictMove st1 (villager "carol") (villager "bob"))
        @?= Just "bob: fetch flour from the mill"
      predictMove st1 (villager "dana") (villager "bob") @?= Nothing

  , testCase "temperament is legible from t=0: the village presumes gale's conscience" $ do
      let v = readView villageWorld
      assertBool "carol presumes gale's conscience"
        (exists "carol.believes.desires.gale.clean-conscience.presumed" v)
      assertBool "dana presumes it too"
        (exists "dana.believes.desires.gale.clean-conscience.presumed" v)
      assertBool "her spite, unheralded, is presumed by no one"
        (not (exists "carol.believes.desires.gale.spites-carol" v))
      assertBool "no conscience is presumed of eve (she bears no trait)"
        (not (exists "carol.believes.desires.eve.clean-conscience" v))

  , testCase "same spite, different temperaments: eve whispers, gale never does" $ do
      let vs = ["you", "bob", "carol", "dana", "eve", "gale"]
          st = freePlayAt 49
      assertBool "eve's frame-up went ahead"
        (exists "dana.believes.stole.carol.loaf.heard.eve" (db st))
      assertBool "and eve carries the mark of it"
        (exists "eve.lied.dana.stole.carol.loaf" (db st))
      assertBool "gale, bearing the same spite, never lied (her psyche is unmarked)"
        (not (exists "gale.lied" (db st)))
      -- v30's threshold fear (spec §3/§4, the v22 retelling precedent):
      -- once carol and dana regard her a slanderer (round 1's whisper), any
      -- FURTHER whisper to someone new would land the third regarder and
      -- trip notoriety -- so eve, now prudent, never risks a second
      -- frame-up in free play. The pre-v30 world had her whisper three
      -- times by t=33 (dana, then you, then gale); the crispest fact for
      -- "exactly once, ever" is her own mark count.
      assertBool "exactly one whisper, ever -- the brink made her prudent"
        ([ s | s <- dbToSentences (db st), "eve.lied." `isPrefixOf` s ]
           == ["eve.lied.dana.stole.carol.loaf"])

      -- The v25 mechanism (an honest believer launders a lie she's been
      -- honestly deceived by) still holds -- it just needs a hand now that
      -- eve's own prudence stops her from ever handing gale the lie
      -- herself in free play. Force exactly the whisper her prudence
      -- declines (one-shot-per-hearer still permits it: gale has never
      -- heard this specific claim from anyone); drive a few turns first to
      -- reach a moment they're actually co-present (free play has them
      -- pass in and out of sync), then let gale relay it honestly, exactly
      -- as the v25 mechanism always has.
      let stCoPresent = driveIdle "you" 5 st
          stWhisper = doAct "eve" "whisper to gale that carol stole the loaf" stCoPresent
          stLaundered = driveIdle "you" 20 stWhisper
          heardFromGale =
            [ (w, c) | w <- vs, c <- vs
                     , exists (w ++ ".believes.stole." ++ c
                                 ++ ".loaf.heard.gale") (db stLaundered) ]
      assertBool "the lie traveled through the honest villager"
        (not (null heardFromGale))
      assertBool "whatever gale passed on, she honestly believes"
        (all (\(_, c) -> exists ("gale.believes.stole." ++ c ++ ".loaf") (db stLaundered))
             heardFromGale)

  , testCase "a told-about spite predicts eve's whisper but not gale's" $ do
      -- dana is told (planted) that each woman nurses the spite; gale's
      -- conscience she has presumed since t=0 (transparent). Believed malice
      -- alone predicts a whisper framing carol; believed malice netted
      -- against believed conscience (+4 - 6) predicts nothing.
      let st = foldl (flip performOutcome) villageWorld
                 [ Insert "dana.believes.desires.eve.spites-carol.heard.you"
                 , Insert "dana.believes.desires.gale.spites-carol.heard.you" ]
          p  = fmap gaLabel (predictMove st (villager "dana") (villager "eve"))
      -- eve has two equally-paying hearers at the mill (dana and gale), so
      -- assert the shape, not the tie-break
      assertBool ("predicted a carol-framing whisper, got " ++ show p)
        (maybe False (\l -> "whisper" `isInfixOf` l
                            && "carol stole" `isInfixOf` l) p)
      predictMove st (villager "dana") (villager "gale") @?= Nothing

  , testCase "threshold fear leaves eve's free-play whispering unchanged: still rational below the brink" $ do
      -- 5 idle-steps: you, bob, carol, dana, eve's own first turn -- the
      -- SAME decision v22/v25 always made, now made under a threshold fear
      -- that simply hasn't fired yet (dana + gale = 2 regarders < 3).
      let st = freePlayAt 5
      assertBool "eve still whispered, framing carol, exactly as before threshold fear existed"
        (exists "eve.lied.dana.stole.carol.loaf" (db st))
      assertBool "two regarders (dana, gale) -- still under the brink"
        (exists "regards.dana.eve.slanderer" (readView st)
         && exists "regards.gale.eve.slanderer" (readView st))
      assertBool "not yet notorious" (not (exists "notorious.eve.slanderer" (readView st)))

  , testCase "carol's shakedown: the threat fires once she holds eyewitness evidence" $ do
      let st = whisperArcAt 3
      assertBool "carol threatened eve" (exists "threatened.whisper.carol.eve" (db st))
      assertBool "the motive-belief deposit: eve hears carol's professed punitive intent"
        (exists "eve.believes.desires.carol.punishes-whisper.heard.carol" (db st))
      assertBool "still under the brink -- only carol and dana regard her"
        (not (exists "notorious.eve.slanderer" (readView st)))

  , testCase "carol's shakedown: eve complies -- the debt exists, the threat is gone, no exposure happened" $ do
      let st = whisperArcAt 6
      assertBool "the debt fact" (exists "debt.carol.eve.favor" (db st))
      assertBool "the obligation Debt composes it from" (exists "obliged.eve.favor" (db st))
      assertBool "the threat is gone" (not (exists "threatened.whisper.carol.eve" (db st)))
      assertBool "eve never defied" (not (exists "defied.whisper.eve.carol" (db st)))
      assertBool "no exposure happened -- no one new heard it from carol"
        (not (any (\w -> exists (w ++ ".believes.whispered.eve.dana.heard.carol") (db st))
                  ["bob", "you", "gale"]))
      assertBool "eve never crossed into notoriety -- the threat bought real silence"
        (not (exists "notorious.eve.slanderer" (readView st)))

  , testCase "carol's shakedown: the reputation stack is undisturbed for uninvolved parties" $ do
      let st = whisperArcAt 6
      assertBool "bob was never threatened over anything"
        (not (any (\s -> "threatened.whisper." `isInfixOf` s && ".bob" `isInfixOf` s)
                  (dbToSentences (db st))))
      assertBool "dana holds no debt or obligation of her own"
        (not (exists "debt.carol.dana.favor" (db st))
         && not (any (\s -> "obliged.dana." `isInfixOf` s) (dbToSentences (db st))))
      assertBool "gale is neither extorter nor victim of anything"
        (not (any (\s -> ("threatened.whisper." `isInfixOf` s || "debt." `isInfixOf` s)
                         && "gale" `isInfixOf` s)
                  (dbToSentences (db st))))
      assertBool "bob's own standing (thief/notoriety track) is untouched by any of this"
        (not (exists "notorious.bob.thief" (readView st)))
  ]
