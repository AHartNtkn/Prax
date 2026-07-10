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
-- move at all.
module Prax.Worlds.Village
  ( villageWorld
  , playerName
  , together
  ) where

import           Prax.Query (Condition (..), CmpOp (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome)
import           Prax.Core (coreLib, adjustScore)
import           Prax.Derive (Axiom)
import           Prax.Witness
import           Prax.Rumor
import           Prax.Repute
import           Prax.Sight
import           Prax.Deceit

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

-- Village life: the theft (observable) and the belief-gated confrontation.
villageP :: Practice
villageP = practice
  { practiceId = "village"
  , practiceName = "Village life"
  , roles = ["V"]
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
      , action "[Actor]: confront [Thief] about the theft"
          [ saw "Actor" "stole.Thief.loaf"
          , Match "practice.world.world.at.Actor!P"
          , Match "practice.world.world.at.Thief!P"
          , Not "confronted.Actor.Thief" ]
          [ Insert "confronted.Actor.Thief"
          , adjustScore "Actor" "Thief" "trust" (-10) "sawTheft" ]

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
        -- shun them — reputation (a derived fact) gating behaviour.
      , action "[Actor]: shun [T]"
          [ regardedAs "Actor" "T" "thief"
          , Neq "T" "Actor"
          , Not "shunned.Actor.T" ]
          [ Insert "shunned.Actor.T" ]

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

        -- Fabrication: assert a theft you have no evidence for, binding the
        -- scapegoat from the village roster. The deceived hold real hearsay --
        -- the whole rumor/reputation stack cascades on the falsehood, and
        -- nobody in the village can tell it from truth.
      , lie together
          [ Absent [ Match "Actor.relationship.Hearer.trust.score!TrustScore"
                   , Cmp Lt "TrustScore" "0" ] ]
          [ Match "practice.world.world.at.Culprit!AnywhereQ" ]
          "stole.Culprit.loaf"
          "[Actor]: whisper to [Hearer] that [Culprit] stole the loaf"
      ]
  }

-- Reputation: evidence settles into standing (defeated by atonement, not by
-- forgetting), and three regarders -- the whole village save the thief --
-- make it common knowledge.
villageAxioms :: [Axiom]
villageAxioms =
  [ standingUnless "stole.Culprit.loaf" "atoned.Culprit" "thief"
  , notoriety "thief" 3
  ]

villageWorld :: PraxState
villageWorld = (foldl (flip performOutcome) base setup)
  { axioms = villageAxioms
    -- an epistemic prediction scope: you credit another's predicted move only
    -- if you're with them now, or you sighted them within the last 2 ticks —
    -- one tick per round, and two rounds is roughly a square<->mill round
    -- trip: "you assume people stay put for about as long as it takes to
    -- walk there and back."
  , predictionScope = [ Or [ together, sightedWithin 2 ] ]
  }
  where
    base = (definePractices [coreLib, worldP, villageP, sightP villageSighting] emptyState)
             { characters =
                 [ character "you"
                 , (character "bob")
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
                                   , conceal "stole.bob.loaf" 12 ] }
                 , (character "carol")
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
                                          , Absent [ Match "regards.carol.T.thief" ] ] (-5) ] }
                 , (character "dana")
                     { charWants = [ Want [ Match "confronted.dana.T" ] 5
                                   , Want [ Match "eyed.dana.T" ] 5
                                   , Want [ Match "shunned.dana.T", Match "regards.dana.T.thief" ] 5
                                   , Want [ Match "shunned.dana.T"
                                          , Absent [ Match "regards.dana.T.thief" ] ] (-5)
                                     -- loiters near the mill: she keeps to her own place
                                     -- rather than drifting on the wander/wait tie-break
                                   , Want [ Match "practice.world.world.at.dana!mill" ] 1 ] }
                 , (character "eve")
                     { charWants = [ -- authored malice: eve wants carol ill-regarded, per head
                                     -- (unnamed charWants are inherently unreadable in
                                     -- prediction — this is how eve's malice stays secret)
                                     Want [ Match "regards.W.carol.thief" ] 4 ] }
                 , sightChar
                 ] }
    setup =
      [ Insert "practice.village.here"
      , Insert "practice.world.world.connected.square.mill"
      , Insert "practice.world.world.connected.mill.square"
      , Insert "practice.world.world.at.you!square"
      , Insert "practice.world.world.at.bob!square"
      , Insert "practice.world.world.at.carol!square"
      , Insert "practice.world.world.at.dana!mill"
      , Insert "practice.world.world.at.eve!mill"
      , Insert "stall.loaf"
      ] ++ sightSetup
