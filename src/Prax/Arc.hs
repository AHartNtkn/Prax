-- | Character arcs (Versu paper §X).
--
-- Where a social practice offers /external, low-level/ actions, a character arc
-- represents a character's /internal, high-level/ state — the through-line of
-- their evening. It is a single fact @X.arc!\<stage\>@ (single-slot via the
-- corrected @!@, so entering a new stage overrides the old). A character's wants
-- can be gated on their arc stage, so advancing the arc reshapes what they
-- pursue; and the arc advances in response to what happens to them.
--
-- A tiny reusable library on the existing engine, like "Prax.Beliefs".
module Prax.Arc
  ( arcSentence
  , arcOf
  , arcIs
  , enterArc
  ) where

import           Prax.Query (Condition (..))
import           Prax.Types (Outcome (..))

-- | The sentence @who.arc!stage@.
arcSentence :: String -> String -> String
arcSentence who stage = who ++ ".arc!" ++ stage

-- | The path @who.arc@ (to bind the current stage: @Match (arcOf who ++ "!S")@).
arcOf :: String -> String
arcOf who = who ++ ".arc"

-- | Condition: @who@ is currently in arc stage @stage@.
arcIs :: String -> String -> Condition
arcIs who stage = Match (arcSentence who stage)

-- | @who@ enters arc stage @stage@ (overriding any previous stage).
enterArc :: String -> String -> Outcome
enterArc who stage = Insert (arcSentence who stage)
