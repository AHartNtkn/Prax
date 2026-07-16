-- | Sightings: knowing where people are is itself information.
--
-- A bodiless per-round ticker (the v18 clock idiom — zero engine surface)
-- advances a global turn counter (@turn!\<n\>@, bound internally by the
-- machinery variable @PraxN@) and, via 'ForEach' over the world's
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

import           Prax.Clock (turnPath, tickConditions, tickOutcome, clockSeed)
import           Prax.Query (Condition (..), CmpOp (..), CalcOp (..))
import           Prax.Types

-- | The ticker's name (bodiless; bound to its practice; blank label, so the
-- CLI's silent-action suppression hides it).
sightName :: String
sightName = "_sight"

-- | The perception clock: one tick per round. @sighting@ is spliced beside
-- the clock's own tick machinery inside the same 'ForEach', so it may not use
-- the @Prax@ namespace or @Actor@ (the ticker itself, never a mover);
-- @Seer@\/@Seen@\/@Spot@ are its CONTRACT variables — the sighting template
-- is expected and required to bind them (the ticker's own outcomes read them
-- straight back out), so they are NOT forbidden — only the namespace and
-- @Actor@ are.
sightP :: [Condition] -> Practice
sightP sighting
  | (v : _) <- offenders =
      error ("Prax.Sight: sighting template authors " ++ show v
             ++ " -- the Prax namespace is reserved for the ticker's own machinery,"
             ++ " and Actor is reserved for the ticker itself (Actor would bind the"
             ++ " ticker character, never a mover)")
  | otherwise = practice
      { practiceId = "sight"
      , practiceName = "time passes and people see each other"
      , roles = ["S"]
      , actions =
          [ action "" (Eq "Actor" sightName : tickConditions)
              (tickOutcome
              : [ ForEach (sighting ++ [ Neq "Seer" "Seen" ])
                    [ Insert "Seer.believes.at.Seen!Spot"
                    , Insert "Seer.believes.atSince.Seen!PraxM" ]
                ])
          ]
      }
  where
    offenders = authoredVarClash ["Actor"] sighting []

sightChar :: Character
sightChar = (character sightName) { charBoundTo = Just "sight" }

sightSetup :: [Outcome]
sightSetup = [ Insert "practice.sight.here", clockSeed ]

-- | Scope fragment over @Actor@\/@Witness@: the Witness was sighted within the
-- last @h@ ticks. Worlds @Or@ this with co-presence-now in their
-- 'predictionScope'.
sightedWithin :: Int -> [Condition]
sightedWithin h =
  [ Match "Actor.believes.atSince.Witness!Since"
  , Match (turnPath ++ "!Now")
  , Calc "Expiry" Add "Since" (show h)
  , Cmp Gte "Expiry" "Now" ]
