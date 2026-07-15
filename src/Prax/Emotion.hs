-- | Coexisting episodic feelings (spec 2026-07-15-v38), replacing the
-- Versu-inherited single-slot mood: @\<who\>.feels.\<emotion\>@ and
-- @\<who\>.feels.\<emotion\>.toward.\<target\>@ are plain multi-valued
-- facts — angry at two people while afraid of a third all coexist, each
-- independent. THE INVARIANT (user, load-bearing): feelings change
-- decision-making, never what decisions can be made — nothing here touches
-- action availability; pricing is ordinary desires reading these facts,
-- authored per world. Authoring guidance: prefer NEGATIVE pricing (a
-- feeling as discomfort driving its own discharge) — the psychology is
-- right and v33's FloorCheck keeps the unfelt state planning-free, where
-- positively-priced feelings are action-insertable and thus AlwaysLive
-- (allowed; the cost is the cost). Onset is authored at the provoking
-- action ('Prax.Rng.draw' fragments); wear-off is a 'Prax.Drift' pulse
-- ('feelingsFade'). Feelings are EPISODIC (v36): they fade; dispositions
-- (traits, marks) never do — a trait makes a feeling LIKELIER, not longer.
module Prax.Emotion
  ( -- * An Ekman-based vocabulary (moved from Prax.Core; plain names)
    happy, sad, angry, afraid, disgusted, surprised, annoyed, pleased
    -- * Feeling and unfeeling (Outcomes)
  , feel, feelToward, unfeel, unfeelToward
    -- * Reading feelings (Conditions)
  , feeling, feelingToward
    -- * Wear-off
  , feelingsFade
  ) where

import           Prax.Drift (DriftRule (..))
import           Prax.Query (Condition (..))
import           Prax.Types

happy, sad, angry, afraid, disgusted, surprised, annoyed, pleased :: String
happy = "happy"; sad = "sad"; angry = "angry"; afraid = "afraid"
disgusted = "disgusted"; surprised = "surprised"
annoyed = "annoyed"; pleased = "pleased"

feelsPath :: String -> String -> String
feelsPath who emotion = who ++ ".feels." ++ emotion

-- | @who@ comes to feel @emotion@ (untargeted).
feel :: String -> String -> Outcome
feel who emotion = Insert (feelsPath who emotion)

-- | @who@ comes to feel @emotion@ toward @target@. Arguments may be action
-- variables, grounded when the outcome runs.
feelToward :: String -> String -> String -> Outcome
feelToward who emotion target =
  Insert (feelsPath who emotion ++ ".toward." ++ target)

-- | Discharge: the whole feeling goes, targets included (venting,
-- confronting, being won over — authored at the discharging action).
unfeel :: String -> String -> Outcome
unfeel who emotion = Delete (feelsPath who emotion)

unfeelToward :: String -> String -> String -> Outcome
unfeelToward who emotion target =
  Delete (feelsPath who emotion ++ ".toward." ++ target)

-- | @who@ currently feels @emotion@ (matches targeted instances too —
-- 'Match' sees subtrees).
feeling :: String -> String -> Condition
feeling who emotion = Match (feelsPath who emotion)

feelingToward :: String -> String -> String -> Condition
feelingToward who emotion target =
  Match (feelsPath who emotion ++ ".toward." ++ target)

-- | Feelings fade: one pulse sweeping every feeling at an authored period.
-- TEST-COMPRESSED in shipped worlds (see Prax.Drift's authoring note; real
-- authoring: hours, ~24-48 rounds). Coarse by design: every standing
-- feeling fades on the same pulse regardless of onset time (per-feeling
-- stamps are banked until a world needs them).
feelingsFade :: Int -> DriftRule
feelingsFade period = DriftRule "feelingsFade" period
  [ ( [ Match "W.feels.E" ], [ Delete "W.feels.E" ] ) ]
