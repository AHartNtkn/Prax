module Prax.VillageSpec (tests) where

import           Data.List (isInfixOf, isPrefixOf)
import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (Val (..), exists, dbToSentences)
import           Prax.Query (Condition (..), groundCondition, query)
import           Prax.Sym (intern)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction, performOutcome, setDesires, groundOutcome)
import           Prax.Loop (advance, npcAct)
import           Prax.Core (adjustScore)
import           Prax.Planner (predictMove, pickAction, candidateActions, motiveSignature)
import           Prax.Sight (sightName)
import           Prax.Witness (witnessed)
import           Prax.Drift (driftChar)
import           Prax.Rng (rngSetup)
import           Prax.Emotion (feelToward, angry)
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
      -- 96 turns: 12 rounds (v36: the round grew an eighth member, the
      -- drifter -- bob, now hungry, eats the stolen loaf outright rather
      -- than ever returning it, so atonement waits on a second, honestly
      -- EARNED loaf; 10 rounds (70 turns under the old 7-member round) no
      -- longer reaches it, measured to land at 12)
      let st = postTheftAt 96
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
      -- 49 turns: 7 rounds. Unaffected by v37's market (period 6): the
      -- market's first opening (round 6) falls after his errand already
      -- completes, so this moment is unchanged from the pre-market world
      -- (re-verified against the live trace, not assumed).
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

    --------------------------------------------------------------------------
    -- v36: the hunger pulse (Prax.Drift's build-up cargo) closes bob's bread
    -- economy into a cycle -- earn, eat, hunger again.
    --------------------------------------------------------------------------

  , testCase "the hunger pulse: absent before turn 3, present exactly at it; the next pulse re-hungers a fed bob" $ do
      -- driftChar rides after sightChar, so within round 3 the clock has
      -- already reached turn!3 (sight's own tick) one step before the
      -- drifter reads it and fires -- "the clock advances before bodies
      -- feel it," stated in the wiring comment.
      let stBefore = freePlayAt 23
          stAfter  = freePlayAt 24
      assertBool "turn already reached 3" (exists "turn!3" (db stBefore))
      assertBool "hungry.bob still absent, one step short of the pulse"
        (not (exists "hungry.bob" (db stBefore)))
      assertBool "hungry.bob present once the drifter takes its turn"
        (exists "hungry.bob" (db stAfter))
      -- force a loaf into his hand (the spec's own sanctioned shortcut:
      -- "force holding.bob.loaf (or drive until earned)") and let him eat.
      let stFed = performOutcome (Insert "holding.bob.loaf") stAfter
          stAte = doAct "bob" "eat the loaf" stFed
      assertBool "hunger and the loaf are both spent by eating"
        (not (exists "hungry.bob" (db stAte))
         && not (exists "holding.bob.loaf" (db stAte)))
      -- the due re-armed to turn 6 (3 + the 3-round period) when it fired at
      -- turn 3, regardless of exactly when within the period bob ate -- so
      -- 3 more rounds (24 more idle-steps) lands squarely on the re-fire.
      let stNext = driveIdle "you" 24 stAte
      assertBool "the next pulse (3 rounds later) re-hungers him"
        (exists "hungry.bob" (db stNext))

  , testCase "the arithmetic pin: a hungry bob with bread and a finished project eats; a sated one does not" $ do
      -- construct the completed-endeavor state directly (deterministic,
      -- no free-play timing dependency): undertake, sweep, fetch flour,
      -- bake -- the lawful loaf, held, project done.
      let stBaked = doAct "bob" "bake and earn the loaf"
                    (doAct "bob" "Go to square"
                    (doAct "bob" "fetch flour from the mill"
                    (doAct "bob" "Go to mill"
                    (doAct "bob" "sweep the square"
                    (doAct "bob" "take up honest work" villageWorld)))))
      assertBool "the project is complete" (exists "practice.earnBread.bob.done.s3" (db stBaked))
      assertBool "he holds the loaf" (exists "holding.bob.loaf" (db stBaked))
      -- not hungry: eating is not even offered (hunger is a physical
      -- precondition of the affordance, not merely a utility factor) --
      -- larder plus finished work are worth more than a needless meal.
      assertBool "sated: eat is not among his candidates"
        (not (any (("eat the loaf" `isInfixOf`) . gaLabel) (possibleActions stBaked "bob")))
      -- now hungry: the relief (+22) beats keeping the loaf (+10) plus the
      -- finished endeavor's stage credit (+9, forfeit by the eat's own
      -- tear-down) by the stated +3 margin -- pickAction takes it.
      let stReady = performOutcome (Insert "hungry.bob") stBaked
      fmap gaLabel (pickAction 2 stReady (villager "bob")) @?= Just "bob: eat the loaf"
      -- the re-take: eating tears down the finished instance, so undertake's
      -- own Not-gate re-opens and the earn cycle can begin again.
      let stAte = doAct "bob" "eat the loaf" stReady
      assertBool "the endeavor instance is torn down"
        (not (exists "practice.earnBread.bob" (db stAte)))
      assertBool "undertake grounds again"
        (any (("take up honest work" `isInfixOf`) . gaLabel) (possibleActions stAte "bob"))

  , testCase "no appetite, no hunger: gale and carol never seed hungry, however long free play runs" $ do
      let st = freePlayAt 56
      assertBool "carol never hungers" (not (exists "hungry.carol" (db st)))
      assertBool "gale never hungers"  (not (exists "hungry.gale" (db st)))

    --------------------------------------------------------------------------
    -- v37: market day (Prax.Drift's 'gathering' combinator, period 6,
    -- duration 1) -- the calendar convenes the town in the square and
    -- disperses it again. Period 6 (not the original 2) is the shipped
    -- cadence: the 2/1 pairing left NO quiet rounds -- the attendance gate
    -- toggled every single round, every attendee re-deliberated every turn,
    -- and a 140-turn paired drive tripled (68.3s -> 193.2s, 33 -> 90
    -- deliberations, measured in Task 4's bench). The mechanism was exact
    -- throughout; the cadence was the defect. Turn counts below are
    -- observed, not assumed (probed against the live trace): the market's
    -- due seed (period=6) first fires at the drifter's turn opening round 6
    -- (turn 48), so round 6 (turns 48-55) is the market's first open round;
    -- its close due (period+duration=7) fires opening round 7 (turn 56).
    -- The golden's own free-play window no longer witnesses a market (the
    -- v36 hunger precedent: the cycle is pinned here, at real turn counts,
    -- not required to be golden-visible).
    --------------------------------------------------------------------------

  , testCase "convergence: attendees with no stronger stake converge on the square while the market holds" $ do
      -- pre-market (round 5, quiet): both dana and gale are at the mill --
      -- their own anchors (dana's +1 mill-want; gale has no location want
      -- at all) hold, unperturbed by any market pull that does not yet exist.
      let stQuiet = freePlayAt 47
      assertBool "dana still at the mill, market not yet open"
        (exists "practice.world.world.at.dana!mill" (db stQuiet))
      assertBool "gale still at the mill, market not yet open"
        (exists "practice.world.world.at.gale!mill" (db stQuiet))
      assertBool "market genuinely not yet open at turn 47"
        (not (exists "marketDay.square" (db stQuiet)))

      -- market round (round 6, turns 48-55 all taken): both have moved to
      -- the square -- dana's own turn (52) and gale's (54) each traded a
      -- one-point anchor (or no anchor at all) for the market's +3 draw.
      let stMarket = freePlayAt 55
      assertBool "the market is open" (exists "marketDay.square" (db stMarket))
      assertBool "dana converged on the square"
        (exists "practice.world.world.at.dana!square" (db stMarket))
      assertBool "gale converged on the square"
        (exists "practice.world.world.at.gale!square" (db stMarket))
      -- the difference the market made, stated directly: neither was there
      -- before it opened, both are once it has.
      assertBool "dana's convergence is the market's doing (she was not there before)"
        (not (exists "practice.world.world.at.dana!square" (db stQuiet))
         && exists "practice.world.world.at.dana!square" (db stMarket))
      assertBool "gale's convergence is the market's doing (she was not there before)"
        (not (exists "practice.world.world.at.gale!square" (db stQuiet))
         && exists "practice.world.world.at.gale!square" (db stMarket))

  , testCase "dispersal: a villager with no stronger stake leaves once the market closes; the cycle recurs" $ do
      -- turn 56: the drifter's close pulse has just fired -- both are still
      -- physically at the square (closing removes the marketDay fact, not
      -- anyone's position; dispersal is a DECISION, made at the villager's
      -- own next turn, not an instantaneous teleport).
      let stClosed = freePlayAt 56
      assertBool "the market is now closed" (not (exists "marketDay.square" (db stClosed)))
      assertBool "dana has not yet moved (closing is not teleportation)"
        (exists "practice.world.world.at.dana!square" (db stClosed))
      assertBool "gale has not yet moved either"
        (exists "practice.world.world.at.gale!square" (db stClosed))

      -- gale's next turn (62): no stronger stake ever attached to her at the
      -- square (spites-carol reads no location), so she disperses straight
      -- back to the mill -- the close's own wake, symmetric with the open's.
      -- (dana's own eyeing of carol, the pin the period-2 cadence once used
      -- to demonstrate "a stronger stake stays," resolved at turn 28 --
      -- long before this cadence's first market close reaches her -- so it
      -- no longer functions as a competing stake at the relevant moment;
      -- traced, not assumed, and dana in fact disperses too, at turn 60,
      -- before gale does. Not reproduced here rather than weakened.)
      let stDispersed = freePlayAt 62
      assertBool "gale disperses: back to the mill once the market's pull is gone"
        (exists "practice.world.world.at.gale!mill" (db stDispersed))

      -- the market recurs (period 6): round 12 (turn 96) reopens it, and by
      -- its close (turn 103) gale -- once again with no stronger stake --
      -- has converged a second time, confirming the cycle is genuinely
      -- periodic, not a one-off.
      let stReopened = freePlayAt 103
      assertBool "the market has reopened" (exists "marketDay.square" (db stReopened))
      assertBool "gale converges again on the second opening"
        (exists "practice.world.world.at.gale!square" (db stReopened))

  , testCase "percolation: a fact witnessed at market reaches more believers than the same fact witnessed on a quiet day" $ do
      -- a neutral fixture fact -- nothing in the vocabulary reads
      -- "spat.gale.carol"; it exists only to measure how far co-presence
      -- carries a witnessed event. Grounding Actor=gale (via 'groundOutcome')
      -- makes the witnesses "whoever currently shares gale's place" -- the
      -- percolation the market is FOR.
      let spatByGale = groundOutcome (witnessed together "spat.gale.carol")
                         (Map.singleton (intern "Actor") (VSym (intern "gale")))
          believers st = [ w | w <- ["you", "bob", "carol", "dana", "eve", "gale"]
                              , exists (w ++ ".believes.spat.gale.carol.seen") (db st) ]

          -- quiet day (turn 14, reset straight from villageWorld's own
          -- trace -- no market has ever opened): gale is at the mill with
          -- only dana for company. Unaffected by the period-6 recadence
          -- (the market's first due doesn't fire until turn 48, so turn 14
          -- is identical to a marketless world either way).
          stQuiet  = performOutcome spatByGale (freePlayAt 14)
          quietWitnesses = believers stQuiet

          -- market day (turn 55, reset independently from villageWorld's own
          -- trace -- the market's first opening, round 6): gale is at the
          -- square with you, bob, carol, and dana all gathered around her.
          stMarket = performOutcome spatByGale (freePlayAt 55)
          marketWitnesses = believers stMarket

      quietWitnesses  @?= ["dana"]
      marketWitnesses @?= ["you", "bob", "carol", "dana"]
      length quietWitnesses  @?= 1
      length marketWitnesses @?= 4
      assertBool "the market convening carries the same fact to more believers"
        (length marketWitnesses > length quietWitnesses)

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
      -- 56 turns: 7 rounds (v36: the round grew an eighth member, the
      -- drifter, so 7 rounds is now 56 turns, not 49 -- the same moment in
      -- the story, re-measured against the longer round). Unaffected by
      -- v37's market (period 6): its first opening (round 6, turn 48)
      -- falls inside this window but never draws eve and gale into a
      -- private moment before it -- re-verified against the live trace
      -- (the period-2 cadence once produced a market-cascaded second
      -- whisper here; at period 6 it does not recur within reasonable free
      -- play, so this pin reverts to its pre-v37 form rather than being
      -- forced).
      let vs = ["you", "bob", "carol", "dana", "eve", "gale"]
          st = freePlayAt 56
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

    --------------------------------------------------------------------------
    -- v32: eve's road back -- confession, absolution, re-offense,
    -- incorrigibility. Two arcs were probed (both traces in the task
    -- report): primary -- carol, the wronged, even with an arbitrarily
    -- generous professed "merciful" desire, never rationally beats eve's
    -- ordinary baseline (the momentary notoriety spike from a freshly
    -- informed third regarder is only HALF-relieved by the planner's own
    -- 0.5 discount on a PREDICTED absolution -- a ceiling no authored value
    -- clears); fallback -- gale, who already regards eve a slanderer from
    -- directly witnessing the whisper, costs nothing to confess to ("free
    -- below the brink," v30's own idiom) and does drive. Shipped: the
    -- fallback, forced here per the wedding/theft precedent (whisperArcSetup's
    -- own idiom).
    --------------------------------------------------------------------------

  , testCase "confessing to gale converts the mark and deposits the ACT, self-sourced, not the content" $ do
      let st = doAct "eve" "confess to gale about framing carol"
                 (tick (doAct "eve" "whisper to dana that carol stole the loaf" villageWorld))
      assertBool "the content-mark converted"
        (exists "eve.confessed.dana.stole.carol.loaf" (db st))
      assertBool "the lied-mark is gone"
        (not (exists "eve.lied.dana.stole.carol.loaf" (db st)))
      assertBool "gale learns the ACT, sourced from eve herself"
        (exists "gale.believes.whispered.eve.dana.heard.eve" (db st))
      assertBool "gale does NOT learn a re-assertion of the framed content"
        (not (exists "gale.believes.stole.carol.loaf.heard.eve" (db st)))

  , testCase "absolution inserts the defeater; slanderer regards dissolve; every belief persists" $ do
      let st = redemptionArcSetup
          v  = readView st
      assertBool "the defeater" (exists "recanted.eve" (db st))
      assertBool "dana's slanderer regard dissolved" (not (exists "regards.dana.eve.slanderer" v))
      assertBool "gale's own slanderer regard dissolved" (not (exists "regards.gale.eve.slanderer" v))
      assertBool "not notorious" (not (exists "notorious.eve.slanderer" v))
      assertBool "dana still remembers witnessing the whisper (memory persists)"
        (exists "dana.believes.whispered.eve.dana.seen" (db st))
      assertBool "gale still remembers both her own witness and eve's confession"
        (exists "gale.believes.whispered.eve.dana.seen" (db st)
         && exists "gale.believes.whispered.eve.dana.heard.eve" (db st))

  , testCase "re-offense: a fresh whisper snaps the defeater away, standing returns" $ do
      let st = reoffendArcSetup
          v  = readView st
      assertBool "the defeater is gone: standing snaps back from memory nobody lost"
        (not (exists "recanted.eve" (db st)))
      assertBool "dana's regard is back" (exists "regards.dana.eve.slanderer" v)
      assertBool "gale's regard is back too" (exists "regards.gale.eve.slanderer" v)
      -- the re-whisper happened in the crowded square this time (not the
      -- empty mill), so the SAME snap-back reaches every bystander there --
      -- a sharper consequence than the original mill-side whisper, not a
      -- weaker one.
      assertBool "every square bystander regards her too now"
        (all (\w -> exists ("regards." ++ w ++ ".eve.slanderer") v) ["you", "bob", "carol"])
      assertBool "the crowd makes her notorious"
        (exists "notorious.eve.slanderer" v)

  , testCase "incorrigibility: gale, now knowing two distinct instances, refuses further absolution" $ do
      let st = reoffendArcSetup
          v  = readView st
      assertBool "gale believes two distinct whispered instances by eve"
        (exists "gale.believes.whispered.eve.dana.seen" (db st)
         && exists "gale.believes.whispered.eve.bob.seen" (db st))
      assertBool "gale now regards eve incorrigible" (exists "regards.gale.eve.incorrigible" v)
      assertBool "gale's absolve affordance is gone -- her patience is spent"
        (not (any (("absolve eve" `isInfixOf`) . gaLabel) (possibleActions st "gale")))
      assertBool "dana, who witnessed only the original mill-side instance, is not yet fed up"
        (not (exists "regards.dana.eve.incorrigible" v))

  , testCase "free-play preservation: eve does not confess or get absolved unprompted" $ do
      -- her secret is expensive to spend: no motive exists for her to trade
      -- the momentary notoriety risk against, absent a believed merciful
      -- absolver at depth-2 (measured and found wanting -- see the task
      -- report). The affordance exists from t=0; free play never takes it.
      let st = freePlayAt 100
      assertBool "her lied-mark survives, unconfessed, through extended free play"
        (exists "eve.lied.dana.stole.carol.loaf" (db st))
      assertBool "no confession, ever" (not (exists "eve.confessed.dana.stole.carol.loaf" (db st)))
      assertBool "no absolution, ever" (not (exists "recanted.eve" (db st)))

    --------------------------------------------------------------------------
    -- v38: carol's temper (Prax.Rng's die + Prax.Emotion's feelings, cargo
    -- @docs/specs/2026-07-15-v38-chance-feelings.md@) -- onset at the shun
    -- action, priced as discomfort, discharged by confronting, faded if
    -- never vented. The shipped seed (1988) makes the golden's OWN dramatic
    -- beat -- dana's shun of carol, round 2 of 'Prax.GoldenDriveSpec' --
    -- draw a hit (verified directly below); free play's VISIBLE decisions
    -- are unaffected (the golden itself is unchanged, confirmed by its own
    -- passing test) because carol never gains a confront outlet inside that
    -- 21-turn window -- the anger simply sits, discomfort with nowhere to
    -- discharge, exactly what 'feelingsFade' is for.
    --------------------------------------------------------------------------

  , testCase "the golden's own beat: dana's shun of carol draws a hit at the shipped seed" $ do
      -- Replays 'Prax.GoldenDriveSpec.villageGolden' up to and including
      -- index 11 ("dana: dana: shun carol"), then checks the die's
      -- consequence directly -- the golden pins the LABELS only, never the
      -- feeling facts, so this is the one place that beat's actual draw
      -- outcome is asserted.
      let trace = iterate (idleStep playerName) villageWorld
          beforeShun = trace !! 11
          afterShun  = trace !! 12
      assertBool "dana has not yet shunned carol" (not (exists "shunned.dana.carol" (db beforeShun)))
      assertBool "carol not yet angry" (not (exists "carol.feels.angry.toward.dana" (db beforeShun)))
      assertBool "dana's turn was the shun" (exists "shunned.dana.carol" (db afterShun))
      assertBool "the die hit: carol is angry at dana"
        (exists "carol.feels.angry.toward.dana" (db afterShun))

  , testCase "onset arms across seeds: the same shun hits under one seed, misses under another" $ do
      -- bob (not short-tempered) is shunned by 'you' -- isolates the BASE
      -- arm (1 in 4) alone, since the trait arm's own guard can never pass
      -- for him. Seeds computed directly against the Lehmer stream
      -- (Prax.Rng's own constants): lehmerNext 4 = 33614 -> mod 4 == 0 (a
      -- hit); lehmerNext 2 = 33614/2... = 16807*2 mod (2^31-1) = 33614 ->
      -- mod 4 == 2 (a miss). Computed and cross-checked against the probe
      -- run, not assumed.
      let stTheft   = doAct "bob" "steal the loaf" villageWorld
          seeded n  = foldl (flip performOutcome) stTheft (rngSetup n)
          stHitPre  = seeded 4
          stMissPre = seeded 2
          stHit     = doAct "you" "shun bob" stHitPre
          stMiss    = doAct "you" "shun bob" stMissPre
      assertBool "before (hit branch): bob calm" (not (exists "bob.feels.angry.toward.you" (db stHitPre)))
      assertBool "after (seed 4): the base arm hits -- bob is angry"
        (exists "bob.feels.angry.toward.you" (db stHit))
      assertBool "the shun itself always lands regardless of the die"
        (exists "shunned.you.bob" (db stHit))
      assertBool "before (miss branch): bob calm" (not (exists "bob.feels.angry.toward.you" (db stMissPre)))
      assertBool "after (seed 2): the base arm misses -- bob stays calm"
        (not (exists "bob.feels.angry.toward.you" (db stMiss)))
      assertBool "the shun still lands on a miss (odds price the FEELING, not the act)"
        (exists "shunned.you.bob" (db stMiss))

  , testCase "the trait arm: short-tempered carol reaches where an un-tempered control does not" $ do
      -- Same seed (1), same two-draw arithmetic, two different shunned
      -- parties: carol bears 'shortTempered.carol' (seeded from t=0), bob
      -- does not. At seed 1: the base arm misses (lehmerNext 1 mod 4 == 3)
      -- but the trait arm's own roll hits (lehmerNext (lehmerNext 1) mod 4
      -- == 1 < 2) -- so ONLY the bearer flares; the control, gated off by
      -- 'shortTempered.T', does not, even though the arithmetic is
      -- identical for both (verified: both branches share the same seed).
      let stFramed        = performOutcome (Insert "you.believes.stole.carol.loaf.seen") villageWorld
          stFramedSeeded  = foldl (flip performOutcome) stFramed (rngSetup 1)
          stCarolShunned  = doAct "you" "shun carol" stFramedSeeded
          stTheftSeeded   = foldl (flip performOutcome) (doAct "bob" "steal the loaf" villageWorld) (rngSetup 1)
          stBobShunned    = doAct "you" "shun bob" stTheftSeeded
      assertBool "carol bears the trait" (exists "shortTempered.carol" (db villageWorld))
      assertBool "bob does not" (not (exists "shortTempered.bob" (db villageWorld)))
      assertBool "before: carol calm" (not (exists "carol.feels.angry.toward.you" (db stFramedSeeded)))
      assertBool "after (seed 1, trait arm): carol is angry"
        (exists "carol.feels.angry.toward.you" (db stCarolShunned))
      assertBool "before: bob calm" (not (exists "bob.feels.angry.toward.you" (db stTheftSeeded)))
      assertBool "after (seed 1, same arithmetic, no trait): bob stays calm"
        (not (exists "bob.feels.angry.toward.you" (db stBobShunned)))
      -- v35 note: onset flips carol's satisfaction vector -- she wakes
      -- (a fresh deliberation, not a stale standing intention, is what the
      -- signature mismatch forces on her next turn).
      let sigBefore = motiveSignature stFramedSeeded (villager "carol")
          sigAfter  = motiveSignature stCarolShunned (villager "carol")
      assertBool "before/after differ: onset wakes her (v35 signature mismatch)"
        (sigBefore /= sigAfter)

  , testCase "anger drives the confrontation: the smoulder discharged, feeling gone" $ do
      -- carol already picks "confront bob" the moment she witnesses his
      -- theft (her own +5 want dominates regardless of temper -- verified:
      -- her calm pick is identical); what v38 adds is that PERFORMING it,
      -- while angry, also vents the feeling -- both halves asserted.
      let stTheft = doAct "bob" "steal the loaf" villageWorld
          stAngry = performOutcome (feelToward "carol" angry "bob") stTheft
      fmap gaLabel (pickAction 2 stTheft (villager "carol"))
        @?= Just "carol: confront bob about the theft"
      fmap gaLabel (pickAction 2 stAngry (villager "carol"))
        @?= Just "carol: confront bob about the theft"
      assertBool "angry before confronting" (exists "carol.feels.angry.toward.bob" (db stAngry))
      let stConfront = doAct "carol" "confront bob" stAngry
      assertBool "the smoulder is discharged: not angry after confronting"
        (not (exists "carol.feels.angry.toward.bob" (db stConfront)))

  , testCase "fade catches the unvented (hand clock)" $ do
      -- No outlet offered here (no theft, no witness) -- the anger just
      -- sits until 'villageFade' (period 4) sweeps it. Absent one turn
      -- short of due, gone exactly at it -- the DriftSpec hand-clock idiom
      -- ('Insert "turn!N"'), matching 'Prax.Emotion.feelingsFade's own test.
      let atTurn k = performOutcome (Insert ("turn!" ++ show (k :: Int)))
          pulse st = snd (npcAct 2 driftChar st)
          stAngry  = performOutcome (feelToward "carol" angry "dana") villageWorld
      assertBool "angry from the outset" (exists "carol.feels.angry.toward.dana" (db stAngry))
      let st3 = pulse (atTurn 3 stAngry)
      assertBool "still angry, one turn short of the due (4)"
        (exists "carol.feels.angry.toward.dana" (db st3))
      let st4 = pulse (atTurn 4 st3)
      assertBool "faded exactly at the due pulse"
        (not (exists "carol.feels.angry.toward.dana" (db st4)))
      -- v35 note: fade flips the vector back -- she wakes again on the way out.
      let sigAngry = motiveSignature stAngry (villager "carol")
          sigFaded = motiveSignature st4 (villager "carol")
      assertBool "before/after differ: fade wakes her too (v35 signature mismatch)"
        (sigAngry /= sigFaded)

  , testCase "the liveness pin: smoulders is FloorCheck" $ do
      let tbl = liveness villageWorld
      tbl Map.! "smoulders" @?= FloorCheck

  , testCase "THE INVARIANT at world scale: carol's candidateActions is identical angry or calm" $ do
      let calmActs   = candidateActions villageWorld (villager "carol")
          angryWorld = performOutcome (feelToward "carol" angry "dana") villageWorld
          angryActs  = candidateActions angryWorld (villager "carol")
      assertBool "the angry world really does differ (the feeling is present)"
        (exists "carol.feels.angry.toward.dana" (db angryWorld)
         && not (exists "carol.feels.angry.toward.dana" (db villageWorld)))
      calmActs @?= angryActs
  ]
  where
    -- The fallback arc's own forced trajectory (whisperArcSetup's idiom):
    -- eve whispers to dana (gale, still at the mill, witnesses the act
    -- directly); eve confesses to gale (costless -- gale already regarded
    -- her); gale absolves.
    redemptionArcSetup :: PraxState
    redemptionArcSetup = foldl (flip ($)) villageWorld
      [ tick . doAct "eve" "whisper to dana that carol stole the loaf"
      , tick . doAct "eve" "confess to gale about framing carol"
      , tick . doAct "gale" "absolve eve of slander"
      ]

    -- Continues the trajectory: gale and eve relocate to the square (where
    -- bob/carol/you already are), and eve whispers again -- a genuinely NEW
    -- hearer (bob), so a genuinely new distinct 'whispered.eve.H' instance.
    reoffendArcSetup :: PraxState
    reoffendArcSetup = foldl (flip ($)) redemptionArcSetup
      [ tick . doAct "gale" "Go to square"
      , tick . doAct "eve" "Go to square"
      , tick . doAct "eve" "whisper to bob that carol stole the loaf"
      ]
