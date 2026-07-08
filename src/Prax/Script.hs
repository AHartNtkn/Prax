-- | A Prompter-lite scene-authoring layer (Versu's Prompter, Nelson 2014).
--
-- Prompter let Versu authors write drama as a /screenplay/ — a @CAST@ plus a
-- graph of @scene@s — instead of hand-coding Praxis practices, and this is what
-- brought Blood & Laurels (126 scenes) into reach. This module reconstructs that
-- surface as an eDSL that /compiles to our existing engine/. Nothing new is added
-- to the interpreter: a 'Script' lowers to ordinary 'Practice'/'Character' values
-- (the same target the hand-written "Prax.Worlds.Intrigue" uses), so every engine
-- feature — the core model, beliefs, FOL desires, cast removal — composes in.
--
-- The model (from Nelson 2014 and Emily Short's writing):
--
--   * a __playtext = CAST + scenes__; exactly one scene is "current" at a time;
--   * a __scene__ has opening text, a @setup@ (facts asserted on entry), a body of
--     scene-local __beats__ (dialogue/affordances, available only while that scene
--     is current), and one or more __junctions__ (labelled routes that fire when a
--     condition holds, either transitioning to another scene or ending the story);
--   * a __character__ carries its desires (FOL 'Want's) directly.
--
-- Compilation model:
--
--   * the current scene is the single-slot fact @currentScene!\<id\>@;
--   * beats become actions of a @beats@ practice, each gated on
--     @currentScene!\<id\>@ (+ @character.Actor@ so only cast, never the narrator,
--     may speak) and the beat's own conditions;
--   * junctions become actions of a @junctions@ practice, gated on the scene being
--     current and the junction's condition, performed by a bodiless __narrator__
--     (Versu's story manager) whose sole desire is to advance the story — so a
--     junction fires automatically the moment its condition is met;
--   * an ending reuses the engine-wide @ending.\<key\>@ convention the loop and
--     "Prax.Stress" already detect.
--
-- 'flowChart' renders the scene graph (Prompter's signature development view);
-- "Prax.Stress" reports which scenes random play reaches.
module Prax.Script
  ( -- * The AST
    Script(..)
  , CastMember(..)
  , Scene(..)
  , Beat(..)
  , Junction(..)
    -- * Smart constructors
  , member
  , player
  , wanting
  , scene
  , beat
  , quip
  , goto
  , ending
    -- * Compilation and tooling
  , narratorName
  , currentSceneOf
  , compile
  , flowChart
  ) where

import           Data.Char (isAlphaNum)
import           Data.List (find)
import qualified Data.Map.Strict as Map

import           Prax.Db (unify, valToString)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome)
import           Prax.Core (coreLib)

-- The AST ---------------------------------------------------------------------

-- | A whole playtext: a cast, a set of scenes, and the scene to open on.
data Script = Script
  { scriptCast  :: [CastMember]
  , scriptScenes :: [Scene]
  , scriptStart :: String          -- ^ id of the opening scene
  }
  deriving (Eq, Show)

-- | A character in the cast, with its desires written as FOL 'Want's.
data CastMember = CastMember
  { castName     :: String
  , castPlayable :: Bool           -- ^ marks the player-controlled character
  , castDesires  :: [Want]
  }
  deriving (Eq, Show)

-- | One scene: the unit of grouping. Its beats are available only while it is
-- current; its junctions are the ways it can end or hand off to another scene.
data Scene = Scene
  { sceneId      :: String
  , sceneOpening :: String         -- ^ narration shown on entering the scene
  , sceneSetup   :: [Outcome]      -- ^ facts asserted when the scene becomes current
  , sceneBeats   :: [Beat]
  , sceneJunctions :: [Junction]
  }
  deriving (Eq, Show)

-- | A beat: a line of dialogue or an affordance a character may take within the
-- scene. @beatSpeaker (Just c)@ restricts it to character @c@ (a Prompter quip is
-- spoken by a named actor); @Nothing@ leaves it open to any cast member. Its
-- conditions and effects are ordinary engine 'Condition's/'Outcome's, so
-- "Prax.Core"/"Prax.Beliefs" smart constructors drop straight in.
data Beat = Beat
  { beatLabel   :: String
  , beatSpeaker :: Maybe String
  , beatConds   :: [Condition]
  , beatEffects :: [Outcome]
  }
  deriving (Eq, Show)

-- | A junction: a labelled route out of a scene, fired (by the narrator) as soon
-- as @junctionWhen@ holds. @junctionTo (Just s)@ transitions to scene @s@;
-- @Nothing@ ends the story with @junctionName@ as the ending key.
data Junction = Junction
  { junctionName :: String
  , junctionTo   :: Maybe String
  , junctionWhen :: [Condition]
  }
  deriving (Eq, Show)

-- Smart constructors ----------------------------------------------------------

-- | A non-player cast member with no desires (override with 'wanting').
member :: String -> CastMember
member n = CastMember { castName = n, castPlayable = False, castDesires = [] }

-- | The player-controlled cast member.
player :: String -> CastMember
player n = (member n) { castPlayable = True }

-- | Give a cast member desires: @member "cassia" \`wanting\` [Want …]@.
wanting :: CastMember -> [Want] -> CastMember
wanting c ws = c { castDesires = ws }

-- | An empty scene with the given id; fill fields with record syntax.
scene :: String -> Scene
scene sid = Scene
  { sceneId = sid, sceneOpening = "", sceneSetup = []
  , sceneBeats = [], sceneJunctions = [] }

-- | A beat open to any cast member: @beat label conditions effects@.
beat :: String -> [Condition] -> [Outcome] -> Beat
beat lbl = Beat lbl Nothing

-- | A beat spoken by a named character: @quip speaker label conditions effects@.
quip :: String -> String -> [Condition] -> [Outcome] -> Beat
quip spk lbl = Beat lbl (Just spk)

-- | A transition junction: @goto name toScene when@.
goto :: String -> String -> [Condition] -> Junction
goto name to = Junction name (Just to)

-- | An ending junction: @ending name when@ (ending key = @name@).
ending :: String -> [Condition] -> Junction
ending name = Junction name Nothing

-- Compilation -----------------------------------------------------------------

-- | The bodiless story manager that fires junctions. Underscore-prefixed so it
-- never collides with a cast name and is never a beat speaker.
narratorName :: String
narratorName = "_narrator"

-- | The id of the currently-active scene, if any.
currentSceneOf :: PraxState -> Maybe String
currentSceneOf st =
  case [ v | b <- unify "currentScene.S" (db st) Map.empty
           , Just v <- [valToString <$> Map.lookup "S" b] ] of
    (s : _) -> Just s
    []      -> Nothing

-- | Compile a 'Script' into a ready-to-run 'PraxState'.
compile :: Script -> PraxState
compile scr = foldl (flip performOutcome) base setup
  where
    scenes = scriptScenes scr

    base = (definePractices [coreLib, beatsP, junctionsP] emptyState)
             { characters = castChars ++ [narrator] }

    beatsP = practice
      { practiceId = "beats", practiceName = "scene dialogue", roles = ["Stage"]
      , actions = concatMap compileBeats scenes }

    junctionsP = practice
      { practiceId = "junctions", practiceName = "story flow", roles = ["Stage"]
      , actions = concatMap compileJunctions scenes }

    compileBeats s = map (compileBeat (sceneId s)) (sceneBeats s)
    compileBeat sid b = action (beatLabel b)
      ( [ Match ("currentScene!" ++ sid), Match "character.Actor" ]
        ++ maybe [] (\spk -> [Eq "Actor" spk]) (beatSpeaker b)
        ++ beatConds b )
      (beatEffects b)

    compileJunctions s = map (compileJunction (sceneId s)) (sceneJunctions s)
    compileJunction sid j = action ("(story) " ++ junctionName j)
      ( [ Eq "Actor" narratorName
        , Match ("currentScene!" ++ sid)
        , Absent [ Match "ending.E" ] ]
        ++ junctionWhen j )
      ( case junctionTo j of
          Just next -> Insert ("currentScene!" ++ next)
                         : setupOf next
                         ++ [ Insert ("storyAdvanced." ++ junctionName j) ]
          Nothing   -> [ Insert ("ending!" ++ junctionName j)
                       , Insert ("storyAdvanced." ++ junctionName j) ] )

    setupOf sid = maybe [] sceneSetup (find ((== sid) . sceneId) scenes)

    castChars = [ (character (castName c)) { charWants = castDesires c }
                | c <- scriptCast scr ]
    -- The narrator's one desire: advance the story. Every junction it fires
    -- asserts a @storyAdvanced.\<name\>@ marker, so firing any available junction
    -- strictly raises its utility — it acts the instant a junction is enabled.
    narrator = (character narratorName)
      { charWants = [ Want [ Match "storyAdvanced.J" ] 100 ]
      , charBoundTo = Just "junctions" }

    setup =
      [ Insert "practice.beats.stage", Insert "practice.junctions.stage" ]
      ++ [ Insert ("character." ++ castName c) | c <- scriptCast scr ]
      ++ [ Insert ("currentScene!" ++ scriptStart scr) ]
      ++ setupOf (scriptStart scr)

-- Tooling ---------------------------------------------------------------------

-- | Render the scene graph as a Mermaid @graph TD@ (Prompter's auto-generated
-- flow-chart): a @start@ node into the opening scene, one node per scene, and a
-- labelled edge per junction — to the target scene, or to a terminal ending node.
flowChart :: Script -> String
flowChart scr = unlines $
  [ "graph TD"
  , "  _start((start)) --> " ++ nodeId (scriptStart scr) ]
  ++ concatMap sceneLines (scriptScenes scr)
  where
    sceneLines s =
      ("  " ++ nodeId (sceneId s) ++ "[\"" ++ sceneId s ++ "\"]")
        : map (edge (sceneId s)) (sceneJunctions s)
    edge from j = case junctionTo j of
      Just to -> "  " ++ nodeId from ++ " -->|" ++ junctionName j ++ "| "
                   ++ nodeId to
      Nothing -> "  " ++ nodeId from ++ " -->|" ++ junctionName j ++ "| "
                   ++ "_end_" ++ nodeId (junctionName j)
                   ++ "((" ++ junctionName j ++ "))"
    -- Mermaid node ids must be identifier-like; keep display text in the labels.
    nodeId = map (\c -> if isAlphaNum c then c else '_')
