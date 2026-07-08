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
  , Memory(..)
    -- * Smart constructors
  , member
  , player
  , wanting
  , concernedWith
  , withTraits
  , scene
  , beat
  , quip
  , goto
  , ending
  , after
  , timeout
  , memory
    -- * Compilation and tooling
  , narratorName
  , scriptPlayer
  , currentSceneOf
  , compile
  , flowChart
  ) where

import           Data.Char (isAlphaNum)
import           Data.List (find, isPrefixOf, stripPrefix)
import qualified Data.Map.Strict as Map

import           Prax.Db (unify, valToString)
import           Prax.Query (Condition (..), CmpOp (..), CalcOp (..))
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

-- | A character in the cast, with its desires written as FOL 'Want's. A
-- character /sketch/ adds 'concernedWith' (concerns compiled to wants) and
-- 'withTraits' (personality tags stored as queryable facts).
data CastMember = CastMember
  { castName     :: String
  , castPlayable :: Bool           -- ^ marks the player-controlled character
  , castDesires  :: [Want]
  , castTraits   :: [String]       -- ^ personality tags → @trait.\<who\>.\<t\>@ facts
  }
  deriving (Eq, Show)

-- | One scene: the unit of grouping. Its beats are available only while it is
-- current; its junctions are the ways it can end or hand off to another scene;
-- its memories are one-shot exposition fired the first time a trigger holds.
data Scene = Scene
  { sceneId      :: String
  , sceneOpening :: String         -- ^ narration shown on entering the scene
  , sceneSetup   :: [Outcome]      -- ^ facts asserted when the scene becomes current
  , sceneBeats   :: [Beat]
  , sceneJunctions :: [Junction]
  , sceneMemories :: [Memory]
  }
  deriving (Eq, Show)

-- | A memory: a one-shot line of exposition fired (as narration) the first time
-- @memoryWhen@ holds while its scene is current. Generalizes Prompter's "the
-- first time a conversation reaches a topic" to any first-time condition.
data Memory = Memory
  { memoryText :: String
  , memoryWhen :: [Condition]
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
member n = CastMember { castName = n, castPlayable = False, castDesires = [], castTraits = [] }

-- | The player-controlled cast member.
player :: String -> CastMember
player n = (member n) { castPlayable = True }

-- | Give a cast member desires: @member "cassia" \`wanting\` [Want …]@.
wanting :: CastMember -> [Want] -> CastMember
wanting c ws = c { castDesires = castDesires c ++ ws }

-- | Sketch a character's __concerns__: each @(dimension, weight)@ appends a want
-- that the character be regarded positively on that dimension — @+weight@ for
-- every other whose evaluation of them on @dimension@ is above zero (the natural
-- positive/negative boundary; the weight is author-supplied, so no magic
-- constants). Composes with 'wanting'.
concernedWith :: CastMember -> [(String, Int)] -> CastMember
concernedWith c pairs = c { castDesires = castDesires c ++ map want pairs }
  where
    want (dim, w) = Want
      [ Match ("Other.relationship." ++ castName c ++ "." ++ dim ++ ".score!N")
      , Neq "Other" (castName c)
      , Cmp Gt "N" "0" ] w

-- | Give a character personality __traits__ — stored as queryable
-- @trait.\<who\>.\<t\>@ facts (usable in preconditions). They are deliberately
-- /not/ compiled to behaviour: no source specifies a trait→desire mapping.
withTraits :: CastMember -> [String] -> CastMember
withTraits c ts = c { castTraits = castTraits c ++ ts }

-- | An empty scene with the given id; fill fields with record syntax.
scene :: String -> Scene
scene sid = Scene
  { sceneId = sid, sceneOpening = "", sceneSetup = []
  , sceneBeats = [], sceneJunctions = [], sceneMemories = [] }

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

-- The scene clock has ticked at least @n@ times (a scene-local turn counter,
-- reset on entry). Used to build timed junctions.
clockReached :: Int -> [Condition]
clockReached n = [ Match "sceneClock!Clk", Cmp Gte "Clk" (show n) ]

-- | A __timed transition__: @after name n toScene@ — hand off to @toScene@ once
-- @n@ turns have elapsed in the current scene (Prompter's timeout transition).
after :: String -> Int -> String -> Junction
after name n to = Junction name (Just to) (clockReached n)

-- | A __timeout ending__: @timeout name n@ — end the story with key @name@ after
-- @n@ turns in the scene (Prompter's @timeout_conclusion@).
timeout :: String -> Int -> Junction
timeout name n = Junction name Nothing (clockReached n)

-- | A __memory__: @memory text when@ — the one-shot exposition @text@, shown the
-- first time @when@ holds while the scene is current.
memory :: String -> [Condition] -> Memory
memory = Memory

-- Compilation -----------------------------------------------------------------

-- | The bodiless story manager that fires junctions. Underscore-prefixed so it
-- never collides with a cast name and is never a beat speaker.
narratorName :: String
narratorName = "_narrator"

-- | The player-controlled character (the first cast member marked @playable@).
scriptPlayer :: Script -> String
scriptPlayer scr = case [ castName c | c <- scriptCast scr, castPlayable c ] of
  (p : _) -> p
  []      -> error "Prax.Script.scriptPlayer: no playable cast member"

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

    base = (definePractices ([coreLib, beatsP, junctionsP] ++ [clockP | usesClock]) emptyState)
             { characters = castChars ++ [narrator] ++ [clockChar | usesClock] }

    beatsP = practice
      { practiceId = "beats", practiceName = "scene dialogue", roles = ["Stage"]
      , actions = concatMap compileBeats scenes }

    -- Junctions and memories are both narrator-fired (they raise its
    -- @storyAdvanced@ utility), so they live in one practice.
    junctionsP = practice
      { practiceId = "junctions", practiceName = "story flow", roles = ["Stage"]
      , actions = concatMap compileJunctions scenes ++ concatMap compileMemories scenes }

    compileBeats s = map (compileBeat (sceneId s)) (sceneBeats s)
    -- A quip is a *specific* speaker's action, so its compiled id bakes the
    -- speaker into the "[Actor]" slot: two speakers sharing the display text
    -- "[Actor]: flatter the king" become distinct actions ("duke: …" vs
    -- "envoy: …") and dispatch (which is by action id) can't cross them. The
    -- rendered label is unchanged, since "[Actor]" would render to the speaker
    -- anyway. Speaker-less beats keep their label verbatim.
    compileBeat sid b = action (maybe id bakeActor (beatSpeaker b) (beatLabel b))
      ( [ Match ("currentScene!" ++ sid), Match "character.Actor" ]
        ++ maybe [] (\spk -> [Eq "Actor" spk]) (beatSpeaker b)
        ++ beatConds b )
      (beatEffects b)

    -- Substitute a concrete speaker for every "[Actor]" token in a label.
    bakeActor spk = go
      where
        go [] = []
        go s@(c:cs) = case stripPrefix "[Actor]" s of
                        Just rest -> spk ++ go rest
                        Nothing   -> c : go cs

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

    -- A memory: fired once (label = its text ⇒ shown as narration) the first
    -- time its trigger holds while the scene is current.
    compileMemories s = zipWith (compileMemory (sceneId s)) [0 :: Int ..] (sceneMemories s)
    compileMemory sid i m =
      let key = sid ++ "_mem" ++ show i
      in action (memoryText m)
           ( [ Eq "Actor" narratorName
             , Match ("currentScene!" ++ sid)
             , Absent [ Match "ending.E" ]
             , Not ("memoryFired." ++ key) ]
             ++ memoryWhen m )
           [ Insert ("memoryFired." ++ key), Insert ("storyAdvanced." ++ key) ]

    -- The scene clock (only when the script uses a timed junction): a bound,
    -- silent character whose one affordance ticks @sceneClock@ each round, so
    -- time passes passively. Reset to 0 on every scene entry.
    usesClock = any (any timed . sceneJunctions) scenes
      where timed j = any isClock (junctionWhen j)
            isClock (Match s) = "sceneClock!" `isPrefixOf` s
            isClock _         = False
    clockName = "_clock"
    clockChar = (character clockName) { charBoundTo = Just "clock" }
    clockP = practice
      { practiceId = "clock", practiceName = "time passes", roles = ["Stage"]
      , actions =
          [ action ""            -- empty label ⇒ silent in narration
              [ Eq "Actor" clockName, Match "sceneClock!N"
              , Absent [ Match "ending.E" ], Calc "M" Add "N" "1" ]
              [ Insert "sceneClock!M" ] ] }

    setupOf sid = [ Insert "sceneClock!0" | usesClock ]
                  ++ maybe [] sceneSetup (find ((== sid) . sceneId) scenes)

    castChars = [ (character (castName c)) { charWants = castDesires c }
                | c <- scriptCast scr ]
    -- The narrator's one desire: advance the story. Every junction/memory it
    -- fires asserts a @storyAdvanced.\<key\>@ marker, so firing any available one
    -- strictly raises its utility — it acts the instant one is enabled.
    narrator = (character narratorName)
      { charWants = [ Want [ Match "storyAdvanced.J" ] 100 ]
      , charBoundTo = Just "junctions" }

    setup =
      [ Insert "practice.beats.stage", Insert "practice.junctions.stage" ]
      ++ [ Insert "practice.clock.stage" | usesClock ]
      ++ [ Insert ("character." ++ castName c) | c <- scriptCast scr ]
      ++ [ Insert ("trait." ++ castName c ++ "." ++ t)
         | c <- scriptCast scr, t <- castTraits c ]
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
