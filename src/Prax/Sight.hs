-- | Sightings: knowing where people are is itself information.
--
-- Perception rides the engine schedule (spec
-- @docs/specs/2026-07-16-v44-the-schedule.md@): a world declares
-- 'Prax.Schedule.sightRule' over its sighting template (reserved variables
-- @Seer@\/@Seen@\/@Spot@), a period-1 recurring rule the engine fires at every
-- round boundary, refreshing location-beliefs for every co-present pair:
--
-- > <seer>.believes.at.<seen>!<place>      -- best guess (single-slot: overwritten)
-- > <seer>.believes.atSince.<seen>!<turn>  -- when it was formed
--
-- Sightings persist after separation ("last known location"), and
-- 'sightedWithin' turns the stamp into a prediction-scope window: the horizon
-- is an authored world parameter with stated meaning, not an engine constant.
module Prax.Sight
  ( sightedWithin
  ) where

import           Prax.Query (Condition (..), CmpOp (..), CalcOp (..))
import           Prax.Types (turnPath)

-- | Scope fragment over @Actor@\/@Witness@: the Witness was sighted within the
-- last @h@ ticks. Worlds @Or@ this with co-presence-now in their
-- 'predictionScope'.
sightedWithin :: Int -> [Condition]
sightedWithin h =
  [ Match "Actor.believes.atSince.Witness!Since"
  , Match (turnPath ++ "!Now")
  , Calc "Expiry" Add "Since" (show h)
  , Cmp Gte "Expiry" "Now" ]
