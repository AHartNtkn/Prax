-- | The v44 authoring surface for engine time (spec
-- @docs/specs/2026-07-16-v44-the-schedule.md@): declare recurring rules and
-- lifetimes; the engine owns every firing. The 'ScheduleRule' TYPE stays in
-- "Prax.Types" (one home, no re-export) — this module is the combinator
-- surface only. A world installs its schedule with
-- 'Prax.Engine.setSchedule', beside 'Prax.Engine.setDesires' /
-- 'Prax.Engine.definePractices'.
--
-- __Authoring periods for real games__: a round (everyone acts once) is a few
-- MINUTES of fiction — roughly 12 rounds an hour, ~150 to a waking day — and
-- games run hundreds to thousands of rounds, so author periods at fiction
-- scale: hunger ~72 rounds (two meals a day), a drink wearing off ~12 (an
-- hour), a daily market ~150, a feeling wearing off ~24-48 (hours). The
-- SHIPPED worlds compress these (hunger 3, metabolism 2, market 6, feelings
-- 4) so the test suite's short drives can reach the pulses — a deliberate,
-- NON-STANDARD truncation for testing, not a model for real authoring. Tests
-- wanting a distant pulse should clock-jump (drive 'Prax.Engine.roundBoundary'
-- or 'Prax.Engine.performOutcome' the clock directly) rather than inherit
-- compressed periods.
module Prax.Schedule
  ( lasts
  , gathering
  , sightRule
  ) where

import           Prax.Query (Condition (..))
import           Prax.Types

-- | Wrap plain inserts with a lifetime: the asserted fact lives @n@ round
-- boundaries, then the engine retracts it (the ONE expiry mechanism —
-- 'Prax.Types.InsertFor'). Loud on anything but an 'Insert' — a lifetime on a
-- 'Delete'\/'Call'\/'ForEach' has no meaning. NOTE the retract takes the
-- path's whole subtree, so lifetimes belong on leaf facts.
lasts :: Int -> Outcome -> Outcome
lasts n (Insert s) = InsertFor n s
lasts _ o = error ("Prax.Schedule.lasts: only an Insert can carry a lifetime, got: "
                   ++ show o)

-- | A recurring convening: ONE rule; the open effects fire every @period@
-- boundaries and their asserted facts live @duration@ boundaries (the close
-- rule of the v37 design is subsumed by expiry — one mechanism for a
-- temporary fact). @0 < duration < period@; a gathering that never opens, or
-- one whose opening never lapses before the next, is not a gathering.
gathering :: String -> Int -> Int -> [Outcome] -> ScheduleRule
gathering name period duration openOuts
  | duration < 1 || duration >= period =
      error ("Prax.Schedule: gathering " ++ name ++ " needs 0 < duration < period")
  | otherwise = ScheduleRule name period [ ([], map (lasts duration) openOuts) ]

-- | Perception as a period-1 recurring rule: the authored sighting template,
-- stamped with the clock read as a fact ('Prax.Types.turnPath' — the tick
-- machinery is the engine's now). @Now@\/@Seer@\/@Seen@\/@Spot@ are its
-- CONTRACT variables — the template binds them and the outcomes read them
-- straight back out — so, like every other rule body, only the @Prax@
-- namespace and @Actor@ are forbidden in the authored @sighting@
-- ('Prax.Engine.setSchedule' enforces it).
sightRule :: [Condition] -> ScheduleRule
sightRule sighting = ScheduleRule "sight" 1
  [ ( sighting ++ [ Neq "Seer" "Seen", Match (turnPath ++ "!Now") ]
    , [ Insert "Seer.believes.at.Seen!Spot"
      , Insert "Seer.believes.atSince.Seen!Now" ] ) ]
