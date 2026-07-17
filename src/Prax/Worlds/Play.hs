-- | The Roman conspiracy, re-authored as a Prompter-lite __play-script__ — the
-- same drama as "Prax.Worlds.Intrigue" but written through "Prax.Script"'s
-- scene/beat/junction surface instead of hand-coded practices, and split across
-- two scenes so it exercises a scene __transition__ (a junction) and the auto
-- flow-chart.
--
-- Act I, /the confidence/: Cassia confides her plot to Marcus (the player). Once
-- she has, the story flows to Act II, /the banquet/, where — unless Marcus warns
-- Artus or strikes first himself — Cassia poisons the patron. Three endings:
-- betrayal (Cassia's poison), loyalty (Marcus warns), complicity (Marcus's hand);
-- Marcus may also warm to Cassia along the way (a romance, orthogonal to the
-- killing). This is a faithful recasting of "Prax.Worlds.Intrigue" — same cast,
-- same affordances, same endings — in fewer authored lines and split into scenes.
--
-- The engine's @"story"@ schedule rule (supplied by 'compile') advances the
-- scene and fires the ending silently at a round boundary, the moment a
-- junction's condition holds.
module Prax.Worlds.Play
  ( playScript
  , playWorld
  , playerName
  ) where

import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Core (adjustScore, setBond, warmth)
import           Prax.Emotion (feelToward, pleased)
import           Prax.Script

-- | The player is Marcus, the poet.
playerName :: String
playerName = "marcus"

-- | The whole episode as a play-script.
playScript :: Script
playScript = Script
  { scriptStart = "confidence"
  , scriptCast =
      [ player "marcus"
      , member "artus"
      , member "cassia" `wanting` [ Want [ Match (deadSentence "artus") ] 100 ]
      ]
  , scriptScenes = [ confidence, banquet ]
  }

-- Act I — Cassia lets Marcus in on the plot; that opens the way to the banquet.
confidence :: Scene
confidence = (scene "confidence")
  { sceneOpening = "A quiet portico. Cassia draws Marcus aside."
  , sceneBeats =
      [ quip "cassia" "[Actor]: confide the plot against artus to marcus"
          [ Not "confided" ]
          [ Insert "confided"
          , Insert "marcusKnows"
          , adjustScore "marcus" "cassia" warmth 5 "sharedASecret" ]
      ]
  , sceneJunctions =
      [ goto "toBanquet" "banquet" [ Match "confided" ] ]
  }

-- Act II — the poisoning plays out; whoever acts first fixes the ending.
banquet :: Scene
banquet = (scene "banquet")
  { sceneOpening = "The banquet hall. Artus reclines, oblivious, wine in hand."
  , sceneBeats =
      [ quip "cassia" "[Actor]: slip poison into artus's cup"
          [ Not (deadSentence "artus"), Absent [ Match "foiled" ] ]
          [ Insert "poisoned.artus.byCassia"
          , Insert (deadSentence "artus") ]

      , quip "marcus" "[Actor]: warn artus that cassia means to kill him"
          [ Match "marcusKnows", Not (deadSentence "artus"), Absent [ Match "foiled" ] ]
          [ Insert "foiled"
          , adjustScore "artus" "marcus" warmth 30 "savedMyLife"
          , feelToward "artus" pleased "marcus" ]

      , quip "marcus" "[Actor]: poison artus with your own hand"
          [ Match "marcusKnows", Not (deadSentence "artus"), Absent [ Match "foiled" ] ]
          [ Insert "poisoned.artus.byMarcus"
          , Insert (deadSentence "artus") ]

      -- Romance: warm to the conspirator you now share a secret with (orthogonal
      -- to the killing — it neither foils nor causes it).
      , quip "marcus" "[Actor]: warm to cassia's charms"
          [ Match "marcusKnows", Not "bond.marcus.cassia!lovers" ]
          [ setBond "marcus" "cassia" "lovers"
          , adjustScore "marcus" "cassia" warmth 15 "sweptUp"
          , feelToward "marcus" pleased "cassia" ]
      ]
  , sceneJunctions =
      [ ending "betrayal"   [ Match "poisoned.artus.byCassia" ]
      , ending "loyalty"    [ Match "foiled" ]
      , ending "complicity" [ Match "poisoned.artus.byMarcus" ]
      ]
  }

-- | The compiled, ready-to-run world.
playWorld :: PraxState
playWorld = compile playScript
