-- | A dramatic vertical slice — "mini Blood & Laurels" — to verify empirically
-- that Versu-style drama is expressible on our primitives: a murder, a
-- character who can die (and leave the cast), betrayal vs. loyalty vs.
-- complicity, a light romance, and multiple distinct endings.
--
-- Rome. @cassia@ means to poison the patron @artus@; she confides the plot to
-- @marcus@ (the player). If no one warns Artus, Cassia poisons him — he dies and
-- is removed from play (ending: betrayal). Marcus, once he knows, may warn Artus
-- (ending: loyalty), do the deed himself (ending: complicity), or warm to
-- Cassia along the way. Left to the autonomous cast, the plot runs its course.
--
-- Uses: beliefs (Marcus learns the plot), the core model (gratitude/romance),
-- the FOL @Absent@ (freeze once an ending is reached), and the cast-removal
-- (`dead.<name>`) mechanic.
module Prax.Worlds.Intrigue
  ( intrigueWorld
  , playerName
  ) where

import           Prax.Query
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, setDesires, setCharacters)
import           Prax.Core (coreLib, adjustScore, setBond, warmth)
import           Prax.Emotion (feelToward, pleased)
import           Prax.Beliefs

-- | The player is Marcus, the poet.
playerName :: String
playerName = "marcus"

-- Everyone present may simply do nothing — so an unmotivated character (and an
-- idle player) waits rather than being forced into a dramatic act.
presenceP :: Practice
presenceP = practice
  { practiceId = "presence"
  , practiceName = "the company at [Place]"
  , roles = ["Place"]
  , actions = [ action "[Actor]: bide your time" [ Match "character.Actor" ] [] ]
  }

-- The conspiracy. Instance: practice.plot.<Schemer>.<Target>.
plotP :: Practice
plotP = practice
  { practiceId = "plot"
  , practiceName = "[Schemer] conspires against [Target]"
  , roles = ["Schemer", "Target"]
  , actions =
      [ -- Recruit/inform an ally — which also lets that ally warn the victim.
        -- Confiding shares not just the fact of danger but the schemer's own
        -- believed mind: the ally comes to hold a motive-belief over the named
        -- vocabulary desire, sourced from the schemer herself, so the ally's
        -- planner can predict the schemer's next move.
        action "[Actor]: confide the plot against [Target] to [Ally]"
          [ Eq "Actor" "Schemer"
          , Match "character.Ally", Neq "Ally" "Schemer", Neq "Ally" "Target"
          , Not "practice.plot.Schemer.Target.confided.Ally" ]
          [ Insert "practice.plot.Schemer.Target.confided.Ally"
          , believe "Ally" "plotAgainst.Target" "yes"
          , Insert "Ally.believes.desires.Schemer.kill-artus.heard.Schemer"
          , adjustScore "Ally" "Schemer" warmth 5 "sharedASecret" ]

        -- The murder: needs a confided accomplice, no warning, victim alive, no
        -- ending yet. Kills the target and removes them from the cast.
      , action "[Actor]: slip poison into [Target]'s cup"
          [ Eq "Actor" "Schemer"
          , Match "practice.plot.Schemer.Target.confided.Accomplice"
          , Not "practice.plot.Schemer.Target.foiled"
          , Not "dead.Target"
          , Absent [ Match "ending.E" ] ]
          [ Insert "dead.Target"
          , Insert "ending!betrayal" ]

        -- Loyalty: anyone who knows can warn the victim, foiling the plot.
      , action "[Actor]: warn [Target] that [Schemer] means to kill them"
          [ believesThat "Actor" "plotAgainst.Target" "yes"
          , Neq "Actor" "Schemer", Neq "Actor" "Target"
          , Not "practice.plot.Schemer.Target.foiled"
          , Not "dead.Target"
          , Absent [ Match "ending.E" ] ]
          [ Insert "practice.plot.Schemer.Target.foiled"
          , adjustScore "Target" "Actor" warmth 30 "savedMyLife"
          , feelToward "Target" pleased "Actor"
          , Insert "ending!loyalty" ]

        -- Complicity: the ally does the deed themselves (a dark player choice).
      , action "[Actor]: poison [Target] with your own hand"
          [ believesThat "Actor" "plotAgainst.Target" "yes"
          , Neq "Actor" "Schemer", Neq "Actor" "Target"
          , Not "practice.plot.Schemer.Target.foiled"
          , Not "dead.Target"
          , Absent [ Match "ending.E" ] ]
          [ Insert "dead.Target"
          , Insert "ending!complicity" ]

        -- Romance: warm to the conspirator you now share a secret with.
      , action "[Actor]: warm to [Schemer]'s charms"
          [ believesThat "Actor" "plotAgainst.Target" "yes"
          , Neq "Actor" "Schemer", Neq "Actor" "Target"
          , Not "bond.Actor.Schemer!lovers" ]
          [ setBond "Actor" "Schemer" "lovers"
          , adjustScore "Actor" "Schemer" warmth 15 "sweptUp"
          , feelToward "Actor" pleased "Schemer" ]
      ]
  }

-- Cast --------------------------------------------------------------------------

marcus :: Character
marcus = character playerName   -- the player

artus :: Character
artus = character "artus"       -- the oblivious patron; no wants

-- The schemer wants the patron dead; her lookahead makes her confide first
-- (which enables the poisoning) then strike. The motive is a named vocabulary
-- desire, not a plain want: naming it is what lets a confidant's belief about
-- it (and the planner's theory-of-mind) get any purchase on it at all.
cassia :: Character
cassia = (character "cassia") { charDesires = ["kill-artus"] }

-- Initial world ----------------------------------------------------------------

-- | The fully set-up episode.
intrigueWorld :: PraxState
intrigueWorld =
  foldl (flip performOutcome) withPractices setup
  where
    withPractices =
      setDesires [ Desire "kill-artus" (Want [ Match (deadSentence "artus") ] 100) ]
        (setCharacters [marcus, artus, cassia]
          (definePractices [ coreLib, presenceP, plotP ] emptyState))
    setup =
      [ Insert "character.marcus"
      , Insert "character.artus"
      , Insert "character.cassia"
      , Insert "practice.presence.rome"
      , Insert "practice.plot.cassia.artus"
      ]
