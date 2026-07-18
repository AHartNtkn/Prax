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
--     is current), and one or more __junctions__ (routes that fire when a
--     condition holds, either transitioning to another scene or ending the story);
--   * a __character__ carries its desires (FOL 'Want's) directly.
--
-- Compilation model (spec v46 — /the world's own dynamics fire silently; only
-- characters' actions surface as fiction/):
--
--   * the current scene is the single-slot fact @currentScene!\<id\>@;
--   * beats become actions of a @beats@ practice, each gated on
--     @currentScene!\<id\>@ (+ @character.Actor@ so only a cast member may speak)
--     and the beat's own conditions;
--   * junctions and endings compile to ONE plain period-1 engine schedule rule,
--     @"story"@ — clauses in authored order (scenes in declaration order, a
--     scene's junctions in declaration order) — registered through the internal
--     'Prax.Engine.registerEngineRules' door (it carries Prax-namespaced
--     machinery 'Prax.Engine.setSchedule' rightly rejects). The engine fires it
--     silently at each round boundary; there is no story manager and no actor;
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
  , concernedWith
  , withTraits
  , scene
  , beat
  , quip
  , goto
  , ending
  , after
  , timeout
    -- * Compilation and tooling
  , scriptPlayer
  , currentSceneOf
  , compile
  , flowChart
  ) where

import           Data.Char (isAlphaNum)
import           Data.List (find, group, sort, stripPrefix)
import qualified Data.Map.Strict as Map

import           Prax.Db (pathNames, unify, valToString)
import           Prax.Query (Condition (..), CmpOp (..), condSents)
import           Prax.Sym (intern)
import           Prax.Types
import           Prax.Engine (definePractices, defineFunctions, performOutcome, setCharacters, registerEngineRules)
import           Prax.Core (coreFns)

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

-- | A junction: a route out of a scene, fired (by the engine's @"story"@
-- schedule rule at a round boundary) as soon as @junctionWhen@ holds AND (if
-- 'junctionAfter' is set) at least that many engine rounds have elapsed since
-- the scene was entered. @junctionTo (Just s)@ transitions to scene @s@;
-- @Nothing@ ends the story with @junctionName@ as the ending key.
--
-- @junctionWhen@ is always 100% author content — unlike the pre-v44-fix
-- design, it never carries spliced machinery, so 'compile' can (and does)
-- validate it uniformly with the same v40 hygiene guard 'sceneSetup' gets,
-- regardless of how the 'Junction' was built (Haskell smart constructor, raw
-- constructor, or decoded from JSON — see "Prax.Script.Json"'s @FromJSON
-- Junction@, the module's documented external-authoring surface). The timeout
-- machinery — the patience marker ('scenePatiencePath') 'setupOf' arms and
-- 'storyClause' reads as @Not@ — is expanded from 'junctionAfter' at compile
-- time, so it never appears in author-visible data at all.
data Junction = Junction
  { junctionName  :: String
  , junctionTo    :: Maybe String
  , junctionWhen  :: [Condition]
  , junctionAfter :: Maybe Int    -- ^ fire only >= this many rounds after scene entry
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
  , sceneBeats = [], sceneJunctions = [] }

-- | A beat open to any cast member: @beat label conditions effects@.
beat :: String -> [Condition] -> [Outcome] -> Beat
beat lbl = Beat lbl Nothing

-- | A beat spoken by a named character: @quip speaker label conditions effects@.
quip :: String -> String -> [Condition] -> [Outcome] -> Beat
quip spk lbl = Beat lbl (Just spk)

-- | A transition junction: @goto name toScene when@. @when@ is validated at
-- 'compile' time (uniformly with every other junction, however built) rather
-- than here: a constructor-level guard is bypassable through any authoring
-- surface that builds a 'Junction' without going through 'goto' — notably
-- "Prax.Script.Json"'s @FromJSON Junction@ — so the v40 hygiene check lives
-- at the actual consumption point instead.
goto :: String -> String -> [Condition] -> Junction
goto name to conds = Junction name (Just to) conds Nothing

-- | An ending junction: @ending name when@ (ending key = @name@).
ending :: String -> [Condition] -> Junction
ending name conds = Junction name Nothing conds Nothing

-- | The patience-marker family (spec v50): a timed junction @j@ of scene
-- @sid@ carries the fact @scenePatience.\<sid\>.\<j\>@, inserted with lifetime
-- @n@ on scene entry ('setupOf') and retracted @n@ boundaries later by the
-- v44 expiry schedule. The junction fires when its patience has RUN OUT (the
-- marker is absent). Compiler machinery, not fiction: produced only by
-- 'setupOf', read only by the story rule. Authors may not touch it — 'compile'
-- rejects any authored condition or outcome headed here (the collision hole).
scenePatienceFamily :: String
scenePatienceFamily = "scenePatience"

-- | The patience marker for timed junction @jname@ of scene @sid@.
scenePatiencePath :: String -> String -> String
scenePatiencePath sid jname = scenePatienceFamily ++ "." ++ sid ++ "." ++ jname

-- | A __timed transition__: @after name n toScene@ — hand off to @toScene@ once
-- @n@ rounds have elapsed in the current scene (Prompter's timeout transition).
after :: String -> Int -> String -> Junction
after name n to = Junction name (Just to) [] (Just n)

-- | A __timeout ending__: @timeout name n@ — end the story with key @name@ after
-- @n@ rounds in the scene (Prompter's @timeout_conclusion@).
timeout :: String -> Int -> Junction
timeout name n = Junction name Nothing [] (Just n)

-- Compilation -----------------------------------------------------------------

-- | The player-controlled character (the first cast member marked @playable@).
scriptPlayer :: Script -> String
scriptPlayer scr = case [ castName c | c <- scriptCast scr, castPlayable c ] of
  (p : _) -> p
  []      -> error "Prax.Script.scriptPlayer: no playable cast member"

-- | The id of the currently-active scene, if any.
currentSceneOf :: PraxState -> Maybe String
currentSceneOf st =
  case [ v | b <- unify "currentScene.S" (db st) Map.empty
           , Just v <- [valToString <$> Map.lookup (intern "S") b] ] of
    (s : _) -> Just s
    []      -> Nothing

-- | Compile a 'Script' into a ready-to-run 'PraxState'. Loud at the consumption
-- point (uniformly over every construction route — Haskell smart constructor,
-- raw constructor, or JSON decode) on five authoring faults:
--
--   * two junctions sharing a name within one scene — each timed junction keys
--     its own patience marker ('scenePatiencePath'), so names must be unique
--     (and same-named junctions are authored ambiguity regardless);
--   * a timed junction with @junctionAfter (Just n)@ and @n < 1@ — a zero-delay
--     \"timed\" junction is a plain junction (spec v50 §2);
--   * an authored condition or outcome headed 'scenePatienceFamily', in any of
--     the five lists 'compile' consumes ('sceneSetup', 'junctionWhen', beat
--     conditions, beat effects, cast-desire conditions) — the family is
--     compiler-owned (the collision hole);
--   * a 'sceneSetup' or 'junctionWhen' authoring a Prax-namespaced variable
--     (the v40 hygiene boundary; these splice into the story rule).
compile :: Script -> PraxState
compile scr
  | (sid, jn) : _ <- duplicateJunctionNames =
      error ("Prax.Script.compile: scene " ++ show sid ++ " has two junctions named "
             ++ show jn ++ " -- junction names must be unique within a scene "
             ++ "(each timed junction keys its own patience marker)")
  | (sid, jn, n) : _ <- zeroDelayJunctions =
      error ("Prax.Script.compile: scene " ++ show sid ++ "'s junction " ++ show jn
             ++ " has a timed delay of " ++ show n
             ++ " -- a timed junction needs at least one round (n=0 is a plain junction)")
  | (site, s) : _ <- scenePatienceOffenders =
      error ("Prax.Script.compile: " ++ site ++ " authors " ++ show s
             ++ " -- the " ++ show scenePatienceFamily
             ++ " family is reserved for the timed-junction machinery")
  | (sid, v) : _ <- sceneSetupOffenders =
      error ("Prax.Script.compile: scene " ++ show sid ++ "'s setup authors " ++ show v
             ++ " -- the Prax namespace is reserved for engine machinery")
  | (sid, jn, v) : _ <- junctionWhenOffenders =
      error ("Prax.Script.compile: scene " ++ show sid ++ "'s junction " ++ show jn
             ++ " authors " ++ show v
             ++ " -- the Prax namespace is reserved for engine machinery")
  | otherwise = foldl (flip performOutcome) (registerEngineRules [storyRule] base) setup
  where
    scenes = scriptScenes scr
    duplicateJunctionNames =
      [ (sceneId s, jn) | s <- scenes, jn <- repeated (map junctionName (sceneJunctions s)) ]
    zeroDelayJunctions =
      [ (sceneId s, junctionName j, n)
      | s <- scenes, j <- sceneJunctions s, Just n <- [junctionAfter j], n < 1 ]
    scenePatienceOffenders =
      [ (site, s) | (site, ss) <- authoredSentenceSites
      , s <- ss, headSegment s == Just scenePatienceFamily ]
    -- Every authored condition/outcome list 'compile' consumes, labelled — the
    -- scenePatience sweep is ENUMERATED here, not inherited from the v40
    -- hygiene sweep below (which covers only sceneSetup + junctionWhen); beat
    -- conditions, beat effects, and cast-desire conditions are the three lists
    -- newly swept for the patience-family guard.
    authoredSentenceSites =
         [ ("scene " ++ show (sceneId s) ++ "'s setup", outcomeSents (sceneSetup s))
         | s <- scenes ]
      ++ [ ("scene " ++ show (sceneId s) ++ "'s junction " ++ show (junctionName j) ++ " condition"
           , condSents (junctionWhen j)) | s <- scenes, j <- sceneJunctions s ]
      ++ [ ("scene " ++ show (sceneId s) ++ "'s beat " ++ show (beatLabel b) ++ " condition"
           , condSents (beatConds b)) | s <- scenes, b <- sceneBeats s ]
      ++ [ ("scene " ++ show (sceneId s) ++ "'s beat " ++ show (beatLabel b) ++ " effect"
           , outcomeSents (beatEffects b)) | s <- scenes, b <- sceneBeats s ]
      ++ [ ("cast member " ++ show (castName c) ++ "'s desire", condSents (wantConditions w))
         | c <- scriptCast scr, w <- castDesires c ]
    repeated xs = [ x | (x : _ : _) <- group (sort xs) ]
    headSegment s = case pathNames s of { (h : _) -> Just h; [] -> Nothing }
    sceneSetupOffenders =
      [ (sceneId s, v) | s <- scenes, v <- authoredVarClash [] [] (sceneSetup s) ]
    junctionWhenOffenders =
      [ (sceneId s, junctionName j, v)
      | s <- scenes, j <- sceneJunctions s, v <- authoredVarClash [] (junctionWhen j) [] ]

    base = setCharacters castChars
             (defineFunctions coreFns (definePractices [beatsP] emptyState))

    beatsP = practice
      { practiceId = "beats", practiceName = "scene dialogue", roles = ["Stage"]
      , actions = concatMap compileBeats scenes }

    -- Junctions and endings are the world's own dynamics, not a character's
    -- action, so they compile to ONE plain period-1 schedule rule the engine
    -- fires silently at each round boundary — clauses in authored order (scenes
    -- in declaration order, a scene's junctions in declaration order). Each
    -- clause's own gates self-mask: the transition's @currentScene@ eviction
    -- masks same-scene doubles, and @Absent ending@ masks everything after an
    -- ending. The fold is eager, forward-only, and order-sensitive: a clause
    -- is re-queried against the PRECEDING clause's post-state (not the
    -- boundary's start state), so a transition can cascade straight into a
    -- later-declared scene's own junction firing in the same boundary —
    -- including an ending right after a cross-scene transition — but never
    -- into an earlier-declared scene's clause, whose turn in the fold has
    -- already passed. Carries Prax-namespaced machinery ('setupOf''s scene
    -- entry — the patience markers a timed destination arms), so it registers
    -- through the engine door, not 'setSchedule'.
    storyRule = ScheduleRule "story" 1
      [ storyClause (sceneId s) j | s <- scenes, j <- sceneJunctions s ]

    -- A timed junction fires when its patience has RUN OUT: the marker that
    -- 'setupOf' armed on scene entry (lifetime n) has expired, so 'Not' the
    -- marker path holds. The v44 boundary order (expiries before rules)
    -- retracts the marker at entry+n exactly when this clause first becomes
    -- eligible.
    storyClause sid j =
      ( [ Match ("currentScene!" ++ sid)
        , Absent [ Match "ending.E" ] ]
        ++ junctionWhen j
        ++ maybe [] (\_ -> [ Not (scenePatiencePath sid (junctionName j)) ]) (junctionAfter j)
      , case junctionTo j of
          Just next -> Insert ("currentScene!" ++ next) : setupOf next
          Nothing   -> [ Insert ("ending!" ++ junctionName j) ] )

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

    -- Scene entry arms one patience marker per timed junction of the scene
    -- (spec v50): @InsertFor n scenePatience.<sid>.<j>@ — a plain literal
    -- insert whose lifetime IS the delay, retracted n boundaries later by the
    -- v44 expiry schedule. Re-entry refreshes the marker (v44's supersession
    -- law resets the clock, as re-stamping did). Runs on all three entry paths
    -- (compile-time start, transition, re-entry) because every path threads
    -- through 'setupOf'.
    setupOf sid = case find ((== sid) . sceneId) scenes of
      Nothing -> []
      Just s  ->
        [ InsertFor n (scenePatiencePath sid (junctionName j))
        | j <- sceneJunctions s, Just n <- [junctionAfter j] ]
        ++ sceneSetup s

    castChars = [ (character (castName c)) { charWants = castDesires c }
                | c <- scriptCast scr ]

    setup =
      [ Insert "practice.beats.stage" ]
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
