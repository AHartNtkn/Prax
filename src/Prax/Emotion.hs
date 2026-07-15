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
  , feeling, feelingToward, feelingSomeone
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

-- | @who@ currently feels @emotion@ — true the instant onset writes ANY
-- instance, targeted or not ('Match' sees subtrees). __Not safe for
-- pricing once discharge is possible.__ 'unfeelToward' (any partial
-- discharge) deletes only the targeted @.toward.\<target\>@ leaf, never the
-- @.toward@ ancestor it leaves behind, childless — 'Prax.Db.retract's
-- documented ambiguity (an asserted fact and a drained interior node are
-- indistinguishable in the trie until the banked "asserted-endpoint
-- marking" fix, tracked in the LEDGER, lands). This subtree 'Match' reads
-- that drained ancestor exactly as it would a live feeling, so after the
-- LAST targeted instance is discharged it still (wrongly) matches. Safe for
-- a content precondition read once, right after onset, before any
-- discharge is possible; for PRICING (any want that must actually fall to
-- zero on discharge) use 'feelingSomeone' instead.
feeling :: String -> String -> Condition
feeling who emotion = Match (feelsPath who emotion)

-- | @who@ feels @emotion@ toward the specific, already-known @target@.
feelingToward :: String -> String -> String -> Condition
feelingToward who emotion target =
  Match (feelsPath who emotion ++ ".toward." ++ target)

-- | Like 'feeling', but residue-safe: binds @targetVar@ (a fresh variable
-- the caller names) to an ACTUAL remaining target, rather than testing the
-- emotion node's own bare existence. Unifying a free variable requires a
-- real child to bind against, so a fully-drained @.toward@ ancestor (the
-- same trie ambiguity 'feeling's haddock documents) yields no binding here
-- where 'feeling' would still match. The shape any PRICING want over
-- "still feels this, toward whoever" must use.
feelingSomeone :: String -> String -> String -> Condition
feelingSomeone = feelingToward

-- | Feelings fade: one pulse sweeping every feeling at an authored period.
-- TEST-COMPRESSED in shipped worlds (see Prax.Drift's authoring note; real
-- authoring: hours, ~24-48 rounds). Coarse by design: every standing
-- feeling fades on the same pulse regardless of onset time (per-feeling
-- stamps are banked until a world needs them).
feelingsFade :: Int -> DriftRule
feelingsFade period = DriftRule "feelingsFade" period
  [ ( [ Match "W.feels.E" ], [ Delete "W.feels.E" ] ) ]
