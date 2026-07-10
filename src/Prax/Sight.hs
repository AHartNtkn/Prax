-- | Sightings: knowing where people are is itself information.
--
-- A bodiless per-round ticker (the v18 clock idiom — zero engine surface)
-- advances a global turn counter @turn!N@ and, via 'ForEach' over the world's
-- sighting template (reserved variables @Seer@\/@Seen@\/@Spot@), refreshes
-- location-beliefs for every co-present pair:
--
-- > <seer>.believes.at.<seen>!<place>      -- best guess (single-slot: overwritten)
-- > <seer>.believes.atSince.<seen>!<turn>  -- when it was formed
--
-- Sightings persist after separation ("last known location"), and
-- 'sightedWithin' turns the stamp into a prediction-scope window: the horizon
-- is an authored world parameter with stated meaning, not an engine constant.
module Prax.Sight
  ( sightName
  , sightP
  , sightChar
  , sightSetup
  , sightedWithin
  ) where

import           Prax.Query (Condition (..), CmpOp (..), CalcOp (..))
import           Prax.Types

-- | The ticker's name (bodiless; bound to its practice; blank label, so the
-- CLI's silent-action suppression hides it).
sightName :: String
sightName = "_sight"

-- | The perception clock: one tick per round.
sightP :: [Condition] -> Practice
sightP sighting = practice
  { practiceId = "sight"
  , practiceName = "time passes and people see each other"
  , roles = ["S"]
  , actions =
      [ action ""
          [ Eq "Actor" sightName
          , Match "turn!N"
          , Calc "M" Add "N" "1" ]
          [ Insert "turn!M"
          , ForEach (sighting ++ [ Neq "Seer" "Seen" ])
              [ Insert "Seer.believes.at.Seen!Spot"
              , Insert "Seer.believes.atSince.Seen!M" ]
          ]
      ]
  }

sightChar :: Character
sightChar = (character sightName) { charBoundTo = Just "sight" }

sightSetup :: [Outcome]
sightSetup = [ Insert "practice.sight.here", Insert "turn!0" ]

-- | Scope fragment over @Actor@\/@Witness@: the Witness was sighted within the
-- last @h@ ticks. Worlds @Or@ this with co-presence-now in their
-- 'predictionScope'.
sightedWithin :: Int -> [Condition]
sightedWithin h =
  [ Match "Actor.believes.atSince.Witness!Since"
  , Match "turn!Now"
  , Calc "Expiry" Add "Since" (show h)
  , Cmp Gte "Expiry" "Now" ]
