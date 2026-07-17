-- | A short play-script that exercises two Prompter compilation features in one
-- story: a __character sketch__ (the ambitious Duke, whose /concern/ for royal
-- favour makes him flatter the king unbidden), and a __timed junction__ (the
-- audience ends of its own accord if you dawdle).
--
-- You are the envoy at a royal audience. Flatter the king to win a little favour,
-- then present your petition while you still have it — do it and the petition is
-- @granted@; dither and the king's patience runs out (@dismissed@). All the while
-- the Duke, who cares only for standing at court, works the room on his own.
module Prax.Worlds.Audience
  ( audienceScript
  , audienceWorld
  , playerName
  ) where

import           Prax.Query (Condition (..))
import           Prax.Types (PraxState, Outcome (..))
import           Prax.Core (adjustScore, scoreAtLeast)
import           Prax.Script

-- | You are the envoy.
playerName :: String
playerName = "envoy"

audienceScript :: Script
audienceScript = Script
  { scriptStart = "audience"
  , scriptCast =
      [ player "envoy"
        -- the sketch: the Duke is defined by what he's concerned with (standing
        -- at court) and who he is (ambitious); the concern compiles to a desire.
      , member "duke" `concernedWith` [ ("favor", 10) ] `withTraits` [ "ambitious" ]
      , member "king"
      ]
  , scriptScenes = [ audience ]
  }

audience :: Scene
audience = (scene "audience")
  { sceneOpening =
      "The throne room. You hold the king's ear — but not for long, and the Duke is circling."
  , sceneSetup = [ Insert "atCourt" ]
  , sceneBeats =
      [ quip "envoy" "[Actor]: flatter the king"
          [ Not "petitioned" ]
          [ adjustScore "king" "envoy" "favor" 5 "flattery" ]

      , quip "envoy" "[Actor]: present your petition"
          ( scoreAtLeast "king" "envoy" "favor" 5 ++ [ Not "petitioned" ] )
          [ Insert "petitioned" ]

        -- the Duke needs no wants of his own here — his *concern* for favour
        -- drives him to court the king unbidden (one telling gesture)
      , quip "duke" "[Actor]: flatter the king"
          [ Not "dukeSpoke", Not "petitioned" ]
          [ Insert "dukeSpoke", adjustScore "king" "duke" "favor" 5 "flattery" ]
      ]
  , sceneJunctions =
      [ ending "granted"    [ Match "petitioned" ]     -- you pressed your case in time
      , timeout "dismissed" 5                           -- …or the king's patience ran out
      ]
  }

-- | The compiled, ready-to-run audience.
audienceWorld :: PraxState
audienceWorld = compile audienceScript
