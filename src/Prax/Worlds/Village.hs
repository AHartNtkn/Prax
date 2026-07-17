-- | The village: the proving ground for the sandbox arc (spec
-- @docs/specs/2026-07-10-v19-witnessing-design.md@). v19 seeds it with the
-- witnessing keystone: bob steals a loaf in the square; whoever is /there/
-- comes to believe it and can act on the belief — whoever isn't, doesn't and
-- can't. v20 makes the news travel: carol tells; hearsay licenses suspicion,
-- not confrontation. v21 completes the arc: evidence settles into derived
-- standing, notoriety tips the thief into atonement, and forgiveness
-- follows; nothing is ever forgotten. v22 adds the adversarial layer: bob
-- keeps his secret by planning (waiting for an empty square), and eve's
-- whispered fabrication cascades through the same rumor/reputation
-- machinery as truth; the framed have no recourse (exculpation needs ground
-- truth the vocabulary deliberately lacks). v23 wires 'Prax.Sight' in:
-- everyone perceives (and, within a short horizon, remembers) where
-- everyone else is, which is what lets the planner's realistic,
-- belief-relative lookahead (@Prax.Planner@) predict a co-villager's next
-- move at all. v24 completes the moral arc: deterrence plus opportunity
-- yields industry (bob earns the loaf he cannot safely steal), with the
-- opportunism honestly kept.
--
-- v25 gives temperament teeth: eve and gale carry the same named spite, but
-- gale bears the honest trait — a conscience valuing her own lie-marks — so
-- eve whispers and gale never does, and anyone told of both spites predicts
-- the difference.
module Prax.Worlds.Village
  ( villageWorld
  , playerName
  , together
  ) where

import           Prax.Query (Condition (..), CmpOp (..))
import           Prax.Types
import           Prax.Engine (definePractices, defineFunctions, performOutcome, setAxioms, setDesires, setCharacters, setSchedule)
import           Prax.Core (coreFns, adjustScore)
import           Prax.Derive (Axiom, axiom)
import           Prax.Project
import           Prax.Witness
import           Prax.Rumor
import           Prax.Repute
import           Prax.Sight (sightedWithin)
import           Prax.Schedule (sightRule, gathering)
import           Prax.Deceit
import           Prax.Persona
import           Prax.Debt (owes)
import           Prax.Blackmail (shakedown)
import           Prax.Confession (confess, absolve, incorrigible)
import           Prax.Rng (rngSetup, draw)
import           Prax.Emotion (feelTowardFor, unfeelToward, angry, feelingToward)

-- | You are a villager — one agent among many.
playerName :: String
playerName = "you"

-- | Co-presence in the village: sharing a place.
together :: CoPresence
together = [ Match "practice.world.world.at.Actor!P"
           , Match "practice.world.world.at.Witness!P" ]

-- | The village's sighting template, over the same movement vocabulary as
-- 'together': whoever shares a place with you is someone you see.
villageSighting :: [Condition]
villageSighting = [ Match "practice.world.world.at.Seer!Spot"
                   , Match "practice.world.world.at.Seen!Spot" ]

-- Places and movement, in the bar's idiom.
worldP :: Practice
worldP = practice
  { practiceId = "world"
  , practiceName = "The village exists"
  , roles = ["World"]
  , actions =
      [ action "[Actor]: Go to [Place]"
          [ Match "practice.world.World.at.Actor!OtherPlace"
          , Match "practice.world.World.connected.OtherPlace.Place" ]
          [ Insert "practice.world.World.at.Actor!Place" ]
      , action "[Actor]: Wait a moment"
          [ Match "practice.world.World.at.Actor!Place" ]
          []
      ]
  }

-- Honest work: the lawful path to bread. Progress itself satisfies (+3 a
-- stage — real, but no substitute for bread in hand); the final stage earns
-- the loaf bob's +10 want has stared at since v19. Sweeping is public —
-- honest work is done in the open — so watching bob work teaches the
-- village his purpose (the inference axiom below).
earnBreadTake :: Action
earnBreadP :: Practice
earnBreadPursuit :: Desire
(earnBreadTake, earnBreadP, earnBreadPursuit) =
  endeavor "earnBread" 3 "[Actor]: take up honest work at the stall"
    [ Match "practice.world.world.at.Actor!square" ]
    [ Stage "[Actor]: sweep the square"
        [ Match "practice.world.world.at.Actor!square" ]
        [ witnessed together "swept.Actor" ]
    , Stage "[Actor]: fetch flour from the mill"
        [ Match "practice.world.world.at.Actor!mill" ]
        [ Insert "carrying.Actor.flour" ]
    , Stage "[Actor]: bake and earn the loaf"
        [ Match "practice.world.world.at.Actor!square"
        , Match "carrying.Actor.flour" ]
        [ Delete "carrying.Actor.flour"
        , Insert "holding.Actor.loaf" ]
    ]

-- Hunger, the build-up cargo (v36 spec @docs/specs/2026-07-14-v36-drift.md@):
-- an episodic engine-schedule rule that closes bob's bread economy into a
-- cycle instead of a one-shot want. TEST-COMPRESSED cadence (see
-- Prax.Schedule's authoring note): hunger every three rounds keeps the cycle
-- inside short test drives; real authoring is ~72 rounds — two meals a waking
-- day at ~5 minutes a round. The engine re-arms the rule a period FROM its
-- firing, so a fed bob stays fed for a full period regardless of when he ate.
hungerPulse :: ScheduleRule
hungerPulse = ScheduleRule "hunger" 3
  [ ( [ Match "appetite.X", Not "hungry.X" ], [ Insert "hungry.X" ] ) ]

-- The drama die's seed (Prax.Rng): an authored world parameter that selects
-- THIS playthrough's fate — every draw below reads off this one stream, and
-- the goldens pin its consequences. Picked as a nod to Park & Miller's own
-- 1988 publication (the die's mechanism, per 'Prax.Rng's haddock). At this
-- seed, the golden's own dramatic beat — dana's shun of carol — draws a hit
-- on both arms (computed: lehmerNext(1988) mod 4 == 0, so the base 1-in-4
-- arm alone already lands; lehmerNext(lehmerNext(1988)) mod 4 == 1 < 2, so
-- the trait arm would have too).
villageSeed :: Integer
villageSeed = 1988

-- Hunger, when it arrives, outranks pride and larder both: eating spends
-- the +10 loaf AND forfeits the finished endeavor's +9 stage credit (3
-- stages x 3, torn down by the eat), so the relief must beat 19 -- at -22,
-- a hungry bob eats (net +3) and the emptied hand re-opens the earn cycle.
-- Side effect to expect in the golden: a hungry, breadless bob weighs the
-- stall's loaf again -- hunger pressure re-opens the village's original
-- temptation. Itemize whatever the drive shows; that is the fiction
-- working, not drift noise.
suffersHunger :: Desire
suffersHunger = Desire "suffers-hunger" (Want [ Match "hungry.Owner" ] (-22))

-- Market day (v37 spec @docs/specs/2026-07-14-v37-gatherings.md@): a
-- recurring convening on the sight clock, formalizing the calendar probe.
-- The market practice: an instance the calendar spawns and tears down; its
-- presence IS the event (no market-only affordances — the draw is the
-- valuation, the payoff is co-presence density feeding sight/witnessing).
marketP :: Practice
marketP = practice { practiceId = "market", roles = ["Fair"] }

-- Market day: TEST-COMPRESSED cadence (see Prax.Schedule's authoring note —
-- real authoring: a daily morning market is ~150-round period, ~24-round
-- duration). Every sixth round, lasting one — most days are quiet. A
-- 'Prax.Schedule.gathering': one open rule whose inserts each carry
-- @lasts 1@, so the engine's expiry queue tears the market down a round later
-- (the v37 close rule is subsumed by expiry — one mechanism). The cycle is
-- pinned at real turn counts, not golden-visible (the v36 hunger precedent;
-- the original every-other-round cadence left no quiet rounds).
marketGathering :: ScheduleRule
marketGathering = gathering "market" 6 1
  [ Insert "practice.market.fair", Insert "marketDay.square" ]

-- Everyone likes a market day: +3 for being at the square while it's on —
-- above the +1 loitering anchors (a market beats an idle preference), below
-- the +5 event wants and every conduct stake (drama outranks festivity).
drawnToMarket :: Desire
drawnToMarket = Desire "drawn-to-market"
  (Want [ Match "marketDay.square"
        , Match "practice.world.world.at.Owner!square" ] 3)

-- Temperament: honesty as conduct, not prohibition. The weight is authored
-- meaning: a lie costs gale 6 per hearer per event — more than the +4 a
-- deceived head's contempt for carol is worth to her spite (so at these
-- stakes it never pays), yet nothing forbids it. The mark it values is the
-- liar's own memory (Prax.Deceit): conscience fires seen or unseen.
--
-- The confessed form is priced identically (v32's own "0 or a mild authored
-- residue" — this trait authors the FULL residue, not a mild one): a truly
-- honest person's conscience is about having deceived at all, not about
-- being caught or having covered it up, so confessing doesn't relieve it.
-- Without this second desire, 'Prax.Confession.confess' (now generally
-- available in the village, v32) opens a cheap-grace loophole this trait's
-- own design forbids: since 'confess' converts the mark away and this trait
-- would otherwise price ONLY the "lied" form, a bearer's depth-2 lookahead
-- would see "lie, then immediately confess" as a way to buy the +4/head
-- spite payoff for the price of a momentary, self-erasable -6 — defeating
-- the "never pays" invariant the weight above is authored to hold. Found by
-- measurement (VillageSpec regression, not by inspection): gale's own
-- free-play choice flipped to lying once confess became available, until
-- this residue was added.
honest :: Trait
honest = Trait "honest"
  [ Desire "clean-conscience"
      (Want [ Match "Owner.lied.H.stole.C.loaf" ] (-6))
  , Desire "conscience-remembers"
      (Want [ Match "Owner.confessed.H.stole.C.loaf" ] (-6)) ]

-- Pricing the smoulder (v38 spec): anger as discomfort, driving its own
-- discharge. -8 outweighs carol's own +5 event wants (confronted.carol.T;
-- shunned.carol.T-and-regards) so she acts to relieve it when she can, but
-- there is no conduct stake of hers in this world for it to outweigh; v33's
-- FloorCheck keeps the unfelt state planning-free (verified in the pins).
-- Bound to a real target ('feelingToward' with a fresh variable, not the
-- bare subtree 'feeling' Match) for its PER-TARGET semantics: two grudges
-- smoulder twice as hot (-8 each), and confront's discharge ('unfeelToward',
-- below) lifts exactly the vented grudge's price. (Historically this shape
-- also dodged the drained-ancestor residue v39's asserted-endpoint marking
-- has since removed — 'Prax.Db.retract' now prunes; the binding is kept for
-- the semantics, not for safety.)
smoulders :: Desire
smoulders = Desire "smoulders" (Want [ feelingToward "Owner" angry "T" ] (-8))

-- Malice with a name: wanting carol ill-regarded, per head. Naming it makes
-- it believable (a told-about spite enters prediction) but it stays
-- unheralded — nothing professes or derives it, so until someone is told it
-- is exactly as unreadable as the unnamed want it replaces (v22's eve).
spitesCarol :: Desire
spitesCarol = Desire "spites-carol" (Want [ Match "regards.W.carol.thief" ] 4)

-- The shakedown: carol, who already holds eyewitness evidence eve whispers
-- (v22's frame-up act made observable below), presses her for a favor.
-- Threshold fear (v30 §3, bob's own idiom): notoriety is nonlinear, so this
-- serves both masters a per-head cost couldn't — free below the brink,
-- catastrophic at it (see 'villageAxioms' and eve's own want below).
whisperShakedown :: (Desire, [Action])
whisperShakedown = shakedown "whisper" together "whispered.V.H" "favor" 6

punishesWhisper :: Desire
punishesWhisper = fst whisperShakedown

threatenWhisper, complyWhisper, defyWhisper, exposeWhisper :: Action
(threatenWhisper, complyWhisper, defyWhisper, exposeWhisper) =
  case snd whisperShakedown of
    [t, c, d, e] -> (t, c, d, e)
    acts -> error ("whisperShakedown: expected 4 actions, got "
                   ++ show (length acts))

-- Fabrication: assert a theft you have no evidence for, binding the
-- scapegoat from the village roster. The deceived hold real hearsay -- the
-- whole rumor/reputation stack cascades on the falsehood, and nobody in the
-- village can tell it from truth. Re-offending forfeits any amends already
-- made for the SLANDER (v21's re-steal idiom, mirrored from 'stole.Actor.loaf'
-- above): a fresh whisper deletes the standing defeater, snapping notoriety
-- back from the beliefs nobody lost, before a now-less-patient audience.
whisperAct :: Action
whisperAct =
  observable together "whispered.Actor.Hearer"
    rawLie { actionOutcomes = actionOutcomes rawLie ++ [ Delete "recanted.Actor" ] }
  where
    rawLie = lie together
      [ Absent [ Match "Actor.relationship.Hearer.trust.score!TrustScore"
               , Cmp Lt "TrustScore" "0" ] ]
      [ Match "practice.world.world.at.Culprit!AnywhereQ" ]
      "stole.Culprit.loaf"
      "[Actor]: whisper to [Hearer] that [Culprit] stole the loaf"

-- Confession & absolution over the whisper (v32, spec
-- @docs/specs/2026-07-12-v32-confession.md@): eve's conscience-mark from
-- 'whisperAct' is CONTENT-shaped ("stole.C.loaf" -- who she framed), but her
-- slanderer standing derives from the ACT ("whispered.V.H" above). One
-- pattern cannot serve both what the mark IS and what confessing it REVEALS
-- (the anticipated shape finding, confirmed empirically and resolved by
-- amending 'Prax.Confession.confess' to take the two patterns separately):
-- the MARK pattern matches her content-shaped conscience; the DEPOSIT
-- pattern is the act-shaped truth confessing it reveals, grounded straight
-- from the mark's own bindings (Actor, H) -- not a re-assertion of the
-- fabricated content.
confessWhisper :: Action
confessWhisper = confess "lied" "confessed" together "stole.C.loaf" "whispered.Actor.H"
  "[Actor]: confess to [Hearer] about framing [C]"

absolveWhisper :: Action
absolveWhisper = absolve "recanted" "whispered.V.H" "incorrigible"
  "[Actor]: absolve [V] of slander"

-- Village life: the theft (observable) and the belief-gated confrontation.
--
-- The role is named "Scene" (a v30 rename): the singleton instance key
-- silently pre-binds any action-local variable of the same name before that
-- action's own conditions are ever evaluated, so a role named "V" collided
-- with the shakedown evidence-pattern convention (found and fixed during
-- v30's implementation).
villageP :: Practice
villageP = practice
  { practiceId = "village"
  , practiceName = "Village life"
  , roles = ["Scene"]
  , actions =
      [ -- Anyone at the stall can steal — bob is merely the one who wants to.
        observable together "stole.Actor.loaf" $
        action "[Actor]: steal the loaf from the stall"
          [ Match "practice.world.world.at.Actor!square"
          , Match "stall.loaf" ]
          [ Delete "stall.loaf"
          , Insert "holding.Actor.loaf"
            -- stealing again forfeits any amends you'd made: standing (and
            -- notoriety) re-derive instantly from the beliefs nobody lost
          , Delete "atoned.Actor" ]

        -- Only someone who SAW the theft can call it out; it cools them toward
        -- the thief. dana, who was elsewhere, never gets this affordance.
        -- Discharge (v38 spec): confronting vents any anger held toward the
        -- very person confronted — the smoulder's own outlet, reusing this
        -- affordance rather than authoring a new one this round.
      , action "[Actor]: confront [Thief] about the theft"
          [ saw "Actor" "stole.Thief.loaf"
          , Match "practice.world.world.at.Actor!P"
          , Match "practice.world.world.at.Thief!P"
          , Not "confronted.Actor.Thief" ]
          [ Insert "confronted.Actor.Thief"
          , adjustScore "Actor" "Thief" "trust" (-10) "sawTheft"
          , unfeelToward "Actor" angry "Thief" ]

        -- Word travels: anyone with evidence can pass it on. Never told: bob
        -- (the subject), an eyewitness (no news value), or the same hearer
        -- twice. The village's own gate: you don't gossip with someone you
        -- distrust.
      , gossip together
          [ Absent [ Match "Actor.relationship.Hearer.trust.score!TrustScore"
                   , Cmp Lt "TrustScore" "0" ] ]
          "stole.Culprit.loaf"
          "[Actor]: tell [Hearer] that [Culprit] stole the loaf"

        -- Hearsay licenses suspicion, not confrontation — and an eyewitness
        -- confronts instead (seen suppresses the milder act).
      , action "[Actor]: eye [Thief] with suspicion"
          [ Match "practice.world.world.at.Actor!P"
          , Match "practice.world.world.at.Thief!P"
          , Neq "Thief" "Actor"
          , heard "Actor" "stole.Thief.loaf"
          , Absent [ Match "Actor.believes.stole.Thief.loaf.seen" ]
          , Not "eyed.Actor.Thief" ]
          [ Insert "eyed.Actor.Thief"
          , adjustScore "Actor" "Thief" "trust" (-5) "heardOfTheft" ]

        -- Standing has teeth: anyone who has come to regard [T] a thief may
        -- shun them — reputation (a derived fact) gating behaviour. Being
        -- shunned stings (v38 spec): anyone might flare (1 in 4 — the
        -- direct, victim-present provocation); a short temper flares on
        -- most slights too (a further 2 in 4, so 3 in 4 overall for the
        -- short-tempered — each arm's odds an authored sentence; the trait
        -- makes the feeling LIKELIER, never longer). Two draws, two stream
        -- steps, by design; a double hit's insert is idempotent. The anger
        -- carries a lifetime (v44): each onset lives 4 round boundaries from
        -- when it flared — TEST-COMPRESSED (see Prax.Schedule; real authoring
        -- ~24-48), replacing v38's synchronized fade sweep with a per-onset span.
      , action "[Actor]: shun [T]"
          [ regardedAs "Actor" "T" "thief"
          , Neq "T" "Actor"
          , Not "shunned.Actor.T" ]
          ( [ Insert "shunned.Actor.T" ]
            ++ draw 1 4 [] [ feelTowardFor 4 "T" angry "Actor" ]
            ++ draw 2 4 [ Match "shortTempered.T" ] [ feelTowardFor 4 "T" angry "Actor" ] )

        -- Atonement, not amnesia: returning the loaf defeats the standing --
        -- every regard dissolves on the next read -- while every belief (the
        -- memory of the deed) persists untouched.
      , action "[Actor]: return the loaf with apologies"
          [ Match "holding.Actor.loaf"
          , Exists [ Match "regards.W.Actor.thief" ] ]
          [ Delete "holding.Actor.loaf"
          , Insert "stall.loaf"
          , Insert "atoned.Actor" ]

        -- Forgiveness: you don't keep shunning someone you no longer condemn.
      , action "[Actor]: relent toward [T]"
          [ Match "shunned.Actor.T"
          , Absent [ Match "regards.Actor.T.thief" ] ]
          [ Delete "shunned.Actor.T" ]

        -- Fabrication, and its road back: confessing self-incriminates
        -- through the ordinary hearsay channel; absolution is a refusable
        -- second-party grant (see 'whisperAct'/'confessWhisper'/
        -- 'absolveWhisper' above).
      , whisperAct
      , confessWhisper
      , absolveWhisper

        -- The lawful alternative to the stall's temptation.
      , earnBreadTake

        -- Closing the loop: hunger is a physical precondition of eating (the
        -- ordinary practice sense, like holding the loaf), so the
        -- emotions-never-gate discipline does not apply here.
      , action "[Actor]: eat the loaf"
          [ Match "hungry.Actor", Match "holding.Actor.loaf" ]
          [ Delete "hungry.Actor", Delete "holding.Actor.loaf"
            -- eating ends the finished bread-project: the endeavor instance
            -- (stage counter included) is torn down, so undertake's own
            -- Not-gate re-opens and the work can begin again. Without this,
            -- 'endeavor' is one-shot and the cycle dies at one loaf. A no-op
            -- for eaters who never baked (Delete on a missing path).
          , Delete "practice.earnBread.Actor" ]

      , threatenWhisper
      , complyWhisper
      , defyWhisper
      , exposeWhisper
      ]
  }

-- Reputation: evidence settles into standing (defeated by atonement, not by
-- forgetting), and three regarders -- the whole village save the thief --
-- make it common knowledge.
villageAxioms :: [Axiom]
villageAxioms =
  [ standingUnless "stole.Culprit.loaf" "atoned.Culprit" "thief"
  , notoriety "thief" 3
    -- Watching him work teaches you his purpose: a witnessed sweep is enough
    -- to presume the pursuit (v21's inference pattern, aimed at a mind).
  , axiom [ Match "Regarder.believes.swept.bob" ]
          [ "Regarder.believes.desires.bob.pursues-earnBread.presumed" ]
    -- temperament is worn on the sleeve: the whole village presumes a
    -- bearer's conduct-valuations from t=0 (v25)
  , transparent
    -- Threshold fear, bob's own idiom (v30 §3): standing derives per
    -- believer of the whispering ACT (content stays secret); "recanted"
    -- names the never-exercised defeater (no atonement act for slander is
    -- authored this round — banked, per spec), kept for symmetry with
    -- 'stole.Culprit.loaf's own standingUnless.
  , standingUnless "whispered.V.H" "recanted.V" "slanderer"
  , notoriety "slanderer" 3
    -- Fed-up-ness is knowledge, not bookkeeping (v32 spec point 4): an
    -- absolver's patience is spent once she personally believes (witnessed
    -- or told, provenance doesn't matter) 2 DISTINCT whispered-lie instances
    -- by the same person -- a "two strikes" threshold, one warning's worth
    -- of patience before further absolution is refused. Permanent by memory
    -- (a later absolution elsewhere never un-learns the count) and per
    -- absolver (one villager's fed-up-ness is not another's).
  , incorrigible "whispered.V.H" 2 "incorrigible"
  ]

villageWorld :: PraxState
villageWorld =
  (setDesires ([ earnBreadPursuit, spitesCarol, punishesWhisper, suffersHunger
                 , drawnToMarket, smoulders ]
                 ++ personaVocabulary [honest])
     (setAxioms villageAxioms (foldl (flip performOutcome) base (setup ++ personaFacts))))
  -- an epistemic prediction scope: you credit another's predicted move only
  -- if you're with them now, or you sighted them within the last 2 ticks —
  -- one tick per round, and two rounds is roughly a square<->mill round
  -- trip: "you assume people stay put for about as long as it takes to
  -- walk there and back."
  { predictionScope = [ Or [ together, sightedWithin 2 ] ] }
  where
    (roster, personaFacts) = cast [honest]
      [ (character "you", [])
      , ((character "bob")
           { charWants = [ Want [ Match "holding.bob.loaf" ] 10
                           -- loiters near the stall (the bar's anchoring idiom:
                           -- an idle character needs a place it wants to be,
                           -- or it drifts on tie-break)
                         , Want [ Match "practice.world.world.at.bob!square" ] 1
                           -- bob can live with individuals' contempt; being the
                           -- village's NOTORIOUS thief outweighs the bread
                         , Want [ Match "notorious.bob.thief" ] (-15)
                           -- and better still that no one ever knows: the bread
                           -- is worth +10, the secret is worth more
                           -- (unnamed charWants are inherently unreadable in
                           -- prediction — this is how bob's concealment stays secret)
                         , conceal "stole.bob.loaf" 12 ]
             -- his disposition to honest work: dormant until he
             -- takes it up (undertaking is a live planner choice)
           , charDesires = ["pursues-earnBread", "suffers-hunger", "drawn-to-market"] }, [])
      , ((character "carol")
           { charWants = [ Want [ Match "confronted.carol.T" ] 5
                           -- keeps to the square unless something needs doing (the
                           -- anchoring idiom; genuinely needed once bob conceals —
                           -- with no early theft her first turns are zero-utility
                           -- ties, and unanchored she wanders off on tie-break)
                         , Want [ Match "practice.world.world.at.carol!square" ] 1
                           -- carol wants others to hear *what she knows* from her --
                           -- an unconditioned "believe it from me" would be
                           -- satisfiable by fabrication once `lie` exists
                         , Want [ Match "carol.believes.stole.bob.loaf"
                                , Match "Other.believes.stole.bob.loaf.heard.carol" ] 5
                         , Want [ Match "shunned.carol.T", Match "regards.carol.T.thief" ] 5
                         , Want [ Match "shunned.carol.T"
                                , Absent [ Match "regards.carol.T.thief" ] ] (-5)
                           -- the shakedown's price: carol wants the favor
                           -- eve's silence-money buys her (small — the
                           -- punitive desire is what motivates the threat;
                           -- this just makes the payoff concrete)
                         , Want [ owes "carol" "eve" "favor" ] 4 ]
           , charDesires = ["punishes-whisper", "drawn-to-market", "smoulders"] }, [])
      , ((character "dana")
           { charWants = [ Want [ Match "confronted.dana.T" ] 5
                         , Want [ Match "eyed.dana.T" ] 5
                         , Want [ Match "shunned.dana.T", Match "regards.dana.T.thief" ] 5
                         , Want [ Match "shunned.dana.T"
                                , Absent [ Match "regards.dana.T.thief" ] ] (-5)
                           -- loiters near the mill: she keeps to her own place
                           -- rather than drifting on the wander/wait tie-break
                         , Want [ Match "practice.world.world.at.dana!mill" ] 1 ]
           , charDesires = ["drawn-to-market"] }, [])
        -- eve's authored malice, named vocabulary since v25 (spitesCarol
        -- above): the same +4/head spite gale carries, and — unheralded —
        -- exactly as unreadable as the unnamed want it replaces. Her own
        -- threshold fear (v30 §3) mirrors bob's notorious -15 exactly —
        -- being the village's KNOWN slanderer destroys her; free below the
        -- brink (1-2 regards), catastrophic at it.
      , ((character "eve")
           { charWants = [ Want [ Match "notorious.eve.slanderer" ] (-15) ]
           , charDesires = ["spites-carol", "drawn-to-market"] }, [])
        -- gale: eve's contrast pair. The same spite, plus a temperament —
        -- her conscience (-6/lie) outprices what any single whisper buys
        -- (+4/head), so eve whispers and gale never does
      , ((character "gale") { charDesires = ["spites-carol", "drawn-to-market"] }, [honest])
      ]
    -- The engine owns time now (v44): the schedule fires sight (period 1),
    -- hunger (period 3), and the market gathering (period 6) at each round
    -- boundary, in declaration order — no ticker characters in the roster.
    base = setSchedule [ sightRule villageSighting, hungerPulse, marketGathering ]
             (setCharacters roster
                (defineFunctions coreFns
                   (definePractices [worldP, villageP, earnBreadP, marketP] emptyState)))
    setup =
      [ Insert "practice.village.here"
      , Insert "practice.world.world.connected.square.mill"
      , Insert "practice.world.world.connected.mill.square"
      , Insert "practice.world.world.at.you!square"
      , Insert "practice.world.world.at.bob!square"
      , Insert "practice.world.world.at.carol!square"
      , Insert "practice.world.world.at.dana!mill"
      , Insert "practice.world.world.at.eve!mill"
      , Insert "practice.world.world.at.gale!mill"
      , Insert "stall.loaf"
      , Insert "appetite.bob"
        -- Temperament, the round's stochastic cargo (v38 spec): a plain
        -- disposition fact, not a Trait bundle (it gates a draw's odds, not
        -- a conduct-desire) — like every disposition it never fades, unlike
        -- the episodic feeling it primes.
      , Insert "shortTempered.carol"
      ] ++ rngSetup villageSeed
