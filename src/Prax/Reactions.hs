-- | Reactions and norms (Versu paper §X and §VIII-D).
--
-- Versu implements reactivity as ordinary social practices: performing an action
-- spawns a /reaction/ practice that offers the affected characters responses,
-- and a response is itself an action that can spawn further reactions. Norms use
-- the same machinery — a norm-violating action marks a violation and spawns a
-- reaction (here, disapproval) offering bystanders a response.
--
-- Like "Prax.Core", this is a reusable standard library on top of the existing
-- engine; it adds no new machinery. A reaction instance is just a spawned
-- practice keyed on its participants; a response consumes it by deleting the
-- instance. Register 'disapprovalP' (and your own reaction practices) with
-- 'Prax.Engine.definePractice'.
module Prax.Reactions
  ( -- * Spawning and ending reactions
    reactionPath
  , spawnReaction
  , endReaction
  , reactionActive
    -- * Norm violations
  , markViolation
  , violationOf
    -- * A ready-made disapproval reaction
  , disapprovalP
  ) where

import           Data.List (intercalate)

import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Core (adjustScore, setMood, warmth, annoyed, pleased)

-- Reactions --------------------------------------------------------------------

-- | The DB path of a reaction instance: @practice.\<id\>.\<part…\>@. Parts may be
-- constants or action variables (grounded when the outcome runs).
reactionPath :: String -> [String] -> String
reactionPath pid parts = intercalate "." ("practice" : pid : parts)

-- | Spawn a reaction practice instance (offer its responses to the participants).
spawnReaction :: String -> [String] -> Outcome
spawnReaction pid parts = Insert (reactionPath pid parts)

-- | Consume a reaction instance (a response has been taken).
endReaction :: String -> [String] -> Outcome
endReaction pid parts = Delete (reactionPath pid parts)

-- | Condition: this reaction instance is currently pending.
reactionActive :: String -> [String] -> Condition
reactionActive pid parts = Match (reactionPath pid parts)

-- Norms ------------------------------------------------------------------------

-- Where a norm violation by @who@ is recorded.
violationPath :: String -> String -> String
violationPath who norm = intercalate "." ["violated", who, norm]

-- | Record that @who@ violated the named @norm@. Agents are given strong
-- negative wants on their own violations, so the planner avoids causing them.
markViolation :: String -> String -> Outcome
markViolation who norm = Insert (violationPath who norm)

-- | Condition matching a recorded violation of @norm@ by @who@.
violationOf :: String -> String -> Condition
violationOf who norm = Match (violationPath who norm)

-- Disapproval ------------------------------------------------------------------

-- | A ready-made reaction: when spawned as @practice.disapproval.\<Offender\>.\<Onlooker\>@,
-- it offers the onlooker a chance to disapprove of (or forgive) the offender.
-- Spawn it with @'spawnReaction' "disapproval" [offender, onlooker]@.
disapprovalP :: Practice
disapprovalP = practice
  { practiceId   = "disapproval"
  , practiceName = "[Onlooker] saw [Offender] break a norm"
  , roles        = ["Offender", "Onlooker"]
  , actions =
      [ action "[Actor]: Disapprove of [Offender]"
          [ Eq "Actor" "Onlooker" ]
          [ setMood "Onlooker" annoyed "Offender" "brokeANorm"
          , adjustScore "Onlooker" "Offender" warmth (-20) "brokeANorm"
          , endReaction "disapproval" ["Offender", "Onlooker"] ]
      , action "[Actor]: Let [Offender]'s lapse slide"
          [ Eq "Actor" "Onlooker" ]
          [ setMood "Onlooker" pleased "Offender" "chosenToForgive"
          , endReaction "disapproval" ["Offender", "Onlooker"] ]
      ]
  }
