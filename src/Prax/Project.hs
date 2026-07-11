-- | Endeavors: long-horizon behavior as staged practices.
--
-- A project __type__ is authored vocabulary (like practices, deeds, and
-- desires); a project __instance__ emerges when a disposed character's planner
-- chooses the undertake action. The pursuit is a named 'Desire' counting the
-- owner's completed stages — __dormant__ without an instance (zero bindings,
-- zero utility), which is how disposed characters carry it permanently and
-- undertaking switches it on: conditioned wants /are/ injectable wants.
--
-- Progress itself is rewarding (+w per completed stage — the authored weight
-- is how invested this character is in this kind of work), so horizon length
-- is irrelevant: every next stage is locally visible to the ordinary planner.
-- The agent pursues because pursuing is rewarding, not because the planner
-- derives the end from the means — the truer model of commitment anyway.
--
-- Because the pursuit is a named desire, projects are theory-of-mind content
-- ("Prax.Minds"): whoever comes to believe you pursue one predicts your next
-- stage. Make a stage public by placing 'Prax.Witness.witnessed' in its
-- 'stageYields'.
module Prax.Project
  ( Stage (..)
  , endeavor
  ) where

import           Prax.Query (Condition (..))
import           Prax.Types

-- | One step of the work: its action label, what the world must provide, and
-- what completing it does to the world (beyond advancing the project).
data Stage = Stage
  { stageLabel  :: String
  , stageNeeds  :: [Condition]
  , stageYields :: [Outcome]
  }

-- | An authored endeavor: the undertake action (for the world to slot into
-- one of its own practices), the staged practice, and the named pursuit
-- desire (for the world's vocabulary). One instance per owner; a finished
-- instance persists as the record of the work.
endeavor :: String        -- ^ project id (a single path segment)
         -> Int           -- ^ pursuit weight: +w per completed stage
         -> String        -- ^ undertake label
         -> [Condition]   -- ^ undertake gate (may be @[]@)
         -> [Stage]
         -> (Action, Practice, Desire)
endeavor pid w ulabel gate stages
  | null stages =
      error ("endeavor: " ++ show pid ++ " has no stages (an endeavor is work)")
  | any (`elem` (".!" :: String)) pid =
      error ("endeavor: id " ++ show pid
             ++ " must be a single path segment (no '.' or '!')")
  | otherwise = (undertake, proj, pursuit)
  where
    inst suffix = "practice." ++ pid ++ suffix
    undertake = action ulabel
      (gate ++ [ Not (inst ".Actor") ])
      [ Insert (inst ".Actor") ]
    proj = practice
      { practiceId   = pid
      , practiceName = "[Owner] pursues " ++ pid
      , roles        = ["Owner"]
      , initOutcomes = [ Insert (inst ".Owner.stage!0") ]
      , actions      = [ stageAction k s | (k, s) <- zip [1 :: Int ..] stages ]
      }
    stageAction k s = action (stageLabel s)
      ([ Eq "Actor" "Owner"
       , Match (inst ".Owner.stage!" ++ show (k - 1)) ]
       ++ stageNeeds s)
      ([ Insert (inst ".Owner.stage!" ++ show k)
       , Insert (inst ".Owner.done.s" ++ show k) ]
       ++ stageYields s)
    pursuit = Desire ("pursues-" ++ pid)
                (Want [ Match (inst ".Owner.done.S") ] w)
