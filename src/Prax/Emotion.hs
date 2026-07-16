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
-- action ('Prax.Rng.draw' fragments); wear-off is an authored LIFETIME on the
-- onset — 'feelFor'\/'feelTowardFor' assert through the engine's expiry queue
-- (spec @docs/specs/2026-07-16-v44-the-schedule.md@), so each feeling lives its
-- own @n@ rounds from its own onset (the v36 episodic principle, no synchronized
-- sweep). Feelings are EPISODIC: they fade; dispositions (traits, marks) never
-- do — a trait makes a feeling LIKELIER, not longer.
module Prax.Emotion
  ( -- * An Ekman-based vocabulary (moved from Prax.Core; plain names)
    happy, sad, angry, afraid, disgusted, surprised, annoyed, pleased
    -- * Feeling and unfeeling (Outcomes)
  , feel, feelToward, unfeel, unfeelToward
    -- * Feeling with a lifetime (fades on its own onset's clock)
  , feelFor, feelTowardFor
    -- * Reading feelings (Conditions)
  , feeling, feelingToward, feelingSomeone
  ) where

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

-- | Like 'feel', but the feeling EXPIRES @n@ round boundaries after onset (the
-- one lifetime mechanism, 'Prax.Types.InsertFor' — the same 'Prax.Schedule.lasts'
-- gives a schedule rule). Each onset lives its own @n@ rounds from when it
-- fired; re-feeling refreshes the timer, discharging ('unfeel') purges it.
feelFor :: Int -> String -> String -> Outcome
feelFor n who emotion = InsertFor n (feelsPath who emotion)

-- | Like 'feelToward', with an @n@-boundary lifetime (see 'feelFor').
feelTowardFor :: Int -> String -> String -> String -> Outcome
feelTowardFor n who emotion target =
  InsertFor n (feelsPath who emotion ++ ".toward." ++ target)

-- | Discharge: the whole feeling goes, targets included (venting,
-- confronting, being won over — authored at the discharging action).
unfeel :: String -> String -> Outcome
unfeel who emotion = Delete (feelsPath who emotion)

unfeelToward :: String -> String -> String -> Outcome
unfeelToward who emotion target =
  Delete (feelsPath who emotion ++ ".toward." ++ target)

-- | @who@ currently feels @emotion@ — true the instant onset writes ANY
-- instance, targeted or not ('Match' sees subtrees). Since v39
-- ('Prax.Db.retract' prunes unasserted childless nodes; spec
-- @docs/specs/2026-07-15-v39-asserted-endpoints.md@) this correctly falls
-- back to 'False' the moment the last instance is discharged: 'unfeelToward'
-- deletes the targeted @.toward.\<target\>@ leaf, and the now-childless,
-- never-asserted @.toward@ scaffold is pruned rather than left standing, so
-- there is no residue for a subtree 'Match' to read. For PRICING that should
-- scale with how many targets remain, prefer 'feelingSomeone' — not for
-- safety (the residue trap is gone) but for its per-target semantics.
feeling :: String -> String -> Condition
feeling who emotion = Match (feelsPath who emotion)

-- | @who@ feels @emotion@ toward the specific, already-known @target@.
feelingToward :: String -> String -> String -> Condition
feelingToward who emotion target =
  Match (feelsPath who emotion ++ ".toward." ++ target)

-- | Like 'feeling', but binds @targetVar@ (a fresh variable the caller
-- names) to an ACTUAL remaining target, so a want priced over it counts once
-- per standing grudge (v38's reviewer note: −8 per target is the better
-- semantics). Since v39 both this and 'feeling' correctly stop matching once
-- every instance is discharged (the drained scaffold is pruned), so the
-- choice between them is now about SEMANTICS — per-target pricing versus a
-- single presence test — not about avoiding a residue trap. The recommended
-- shape for any PRICING want over "still feels this, toward whoever".
feelingSomeone :: String -> String -> String -> Condition
feelingSomeone = feelingToward
