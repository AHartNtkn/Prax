-- | Interactive CLI for the storyworlds: round-robin turns, NPCs act
-- autonomously, and on the player's turn a numbered action menu is offered
-- (Versu's \"act / more\" interface). Choose a world on the command line:
-- @prax@ (the bar, default) or @prax intrigue@ (the dramatic episode).
module Main (main) where

import           Data.List (isSuffixOf)
import           Data.Maybe (listToMaybe)
import qualified Data.Map.Strict as Map
import qualified Data.Set as Set
import qualified Data.ByteString.Lazy.Char8 as BLC
import           System.Environment (getArgs)
import           System.Exit (exitFailure)
import           System.IO (BufferMode (NoBuffering), hPutStrLn, hSetBuffering, isEOF, stderr, stdout)
import           Text.Read (readMaybe)

import           Prax.Db (Bindings, unify, valToString)
import           Prax.Types
import           Prax.Engine (performAction, readView)
import           Prax.Planner (candidateActions)
import           Prax.Loop (advance, npcAct)
import           Prax.Stress
import           Prax.Persist (saveState, loadState)
import           Prax.TypeCheck (TypeError (..), typeCheck)
import           Prax.Script (Script, compile, flowChart, scriptPlayer)
import           Prax.Script.Json (encodeScript, loadScript)
import qualified Prax.Worlds.Bar as Bar
import qualified Prax.Worlds.Intrigue as Intrigue
import qualified Prax.Worlds.Play as Play
import qualified Prax.Worlds.Feud as Feud
import qualified Prax.Worlds.Audience as Audience

-- How many plies of lookahead the NPCs use.
lookaheadDepth :: Int
lookaheadDepth = 2

-- Where an in-game save is written / resumed from.
saveFile :: FilePath
saveFile = "prax.save"

-- A world by name, for both playing and stress-testing.
worldNamed :: [String] -> (String, PraxState, String)
worldNamed ("intrigue" : _) = ("Intrigue (Rome)", Intrigue.intrigueWorld, Intrigue.playerName)
worldNamed ("play" : _)     = ("the conspiracy (a play)", Play.playWorld, Play.playerName)
worldNamed ("dm" : _)       = ("the bar, and you direct it", Bar.barDirectorWorld, Bar.directorName)
worldNamed ("feud" : _)     = ("the feud (emergent sandbox)", Feud.feudWorld, Feud.playerName)
worldNamed ("audience" : _) = ("the royal audience", Audience.audienceWorld, Audience.playerName)
worldNamed _                = ("a night at the bar", Bar.barWorld, Bar.playerName)

-- @prax stress [world]@ — a QA report over many random all-AI playthroughs.
runStress :: [String] -> IO ()
runStress args = do
  let (name, world, _) = worldNamed args
      r = stressTest 200 50 world
  putStrLn ("stress-testing " ++ name ++ " — 200 random runs, cap 50 turns")
  putStrLn ("  endings:   " ++ show (Map.toList (srEndings r)))
  putStrLn ("  coverage:  " ++ show (Set.size (srCoverage r)) ++ " distinct actions fired")
  if Map.null (srScenes r)
    then pure ()
    else putStrLn ("  scenes:    " ++ show (Map.toList (srScenes r)) ++ " (runs visiting each)")
  putStrLn ("  dead ends: " ++ show (srDeadEnds r))
  putStrLn ("  no ending: " ++ show (srNoEnding r) ++ " / " ++ show (srRuns r) ++ " runs")

-- @prax check [world]@ — static well-formedness report for a world.
runCheck :: [String] -> IO ()
runCheck args = do
  let (name, world, _) = worldNamed args
  case typeCheck world of
    [] -> putStrLn ("well-formed: " ++ name)
    es -> do putStrLn (name ++ " — " ++ show (length es) ++ " problem(s):")
             mapM_ (putStrLn . ("  - " ++) . describe) es
  where
    describe (UnboundVar w v s) =
      "unbound variable " ++ v ++ " in \"" ++ s ++ "\" (" ++ w ++ ")"
    describe (CardinalityClash slot) =
      "relation " ++ slot ++ " is used both single-valued (!) and multi-valued (.)"
    describe (UndefinedRef w n) =
      "undefined reference " ++ n ++ " (" ++ w ++ ")"
    describe (SortConflict w d) =
      "sort conflict at " ++ w ++ ": " ++ d

main :: IO ()
main = do
  hSetBuffering stdout NoBuffering
  args <- getArgs
  case args of
    ("stress" : rest) -> runStress rest
    ("check" : rest)  -> runCheck rest
    ("dump-play" : _) -> BLC.putStrLn (encodeScript Play.playScript)
    ("flow" : rest)   -> do scr <- scriptFrom rest; putStr (flowChart scr)
    ("play" : file : _) | ".json" `isSuffixOf` file -> playFile file
    _                 -> play args

-- A play-script from a @.json@ file argument if given, else the built-in one.
scriptFrom :: [String] -> IO Script
scriptFrom args = case [ f | f <- args, ".json" `isSuffixOf` f ] of
  (f : _) -> loadOrDie f
  []      -> pure Play.playScript

-- Load a play-script or exit loudly (no silent fallback).
loadOrDie :: FilePath -> IO Script
loadOrDie f = do
  r <- loadScript f
  case r of
    Right s -> pure s
    Left e  -> do hPutStrLn stderr ("could not load " ++ f ++ ": " ++ e)
                  exitFailure

-- Play a script loaded from a file.
playFile :: FilePath -> IO ()
playFile file = do
  scr <- loadOrDie file
  let title = "prax — " ++ file
  putStrLn (replicate (length title + 4) '=')
  putStrLn ("  " ++ title)
  putStrLn (replicate (length title + 4) '=')
  loop (scriptPlayer scr) (compile scr)

play :: [String] -> IO ()
play args = do
  let (title, blurb, world, player) = case args of
        ("intrigue" : _) ->
          ( "prax — Intrigue (Rome)"
          , "You are Marcus, the poet. The others act on their own."
          , Intrigue.intrigueWorld, Intrigue.playerName )
        ("play" : _) ->
          ( "prax — the conspiracy (a play)"
          , "You are Marcus. The scene plays out around you (see `prax flow`)."
          , Play.playWorld, Play.playerName )
        ("dm" : _) ->
          ( "prax — you direct the evening"
          , "You are the drama manager: nudge the autonomous cast (ada, bex, cai)."
          , Bar.barDirectorWorld, Bar.directorName )
        ("feud" : _) ->
          ( "prax — the feud (emergent sandbox)"
          , "You are Alice. One wrong, and a feud assembles itself — make amends to dissolve it."
          , Feud.feudWorld, Feud.playerName )
        ("audience" : _) ->
          ( "prax — the royal audience"
          , "You are the envoy. Flatter the king, then petition — before the moment (or the Duke) passes."
          , Audience.audienceWorld, Audience.playerName )
        _ ->
          ( "prax — a night at the bar"
          , "You are 'you'. Others act on their own."
          , Bar.barWorld, Bar.playerName )
  putStrLn (replicate (length title + 4) '=')
  putStrLn ("  " ++ title)
  putStrLn (replicate (length title + 4) '=')
  putStrLn blurb
  world' <- if "resume" `elem` args
              then do putStrLn ("(resumed from " ++ saveFile ++ ")")
                      loadState saveFile world
              else pure world
  loop player world'

-- The first reached ending, if any (an @ending.\<key\>@ fact).
endingOf :: PraxState -> Maybe String
endingOf st =
  listToMaybe [ e | b <- unify "ending.E" (db st) Map.empty
                  , Just e <- [valToString <$> Map.lookup "E" b] ]

loop :: String -> PraxState -> IO ()
loop player st =
  case endingOf st of
    Just e -> do
      putStrLn ""
      putStrLn "=============================="
      putStrLn ("  THE END — " ++ e)
      putStrLn "=============================="
    Nothing -> do
      let (actor, st1) = advance st
      if charName actor == player
        then playerTurn player st st1 actor   -- `st` is the pre-advance save point
        else do
          let (mga, st2) = npcAct lookaheadDepth actor st1
          case mga of
            Just ga | not (all (== ' ') (gaLabel ga)) -> putStrLn ("  " ++ gaLabel ga)
            _ -> pure ()   -- idle NPCs (and the silent clock tick) stay quiet
          loop player st2

-- @savePoint@ is the state *before* advancing to the player, so resuming from it
-- replays the advance and lands back on the player's turn.
playerTurn :: String -> PraxState -> PraxState -> Character -> IO ()
playerTurn player savePoint st actor = do
  putStrLn ""
  putStrLn "-------------------- scene --------------------"
  putStr (renderScene st)
  let acts = playerActions st actor
  putStrLn ("Your move (" ++ charName actor ++ "):")
  mapM_ (\(i, a) -> putStrLn ("  " ++ show i ++ ") " ++ gaLabel a))
        (zip [1 :: Int ..] acts)
  putStrLn "  m) wait and let others act"
  putStrLn "  s) save    q) quit"
  choice <- prompt
  case choice of
    Quit -> putStrLn "Bye."
    Wait -> loop player st
    Save -> do
      saveState saveFile savePoint
      putStrLn ("(saved to " ++ saveFile ++ ")")
      playerTurn player savePoint st actor
    Pick i
      | i >= 1 && i <= length acts -> do
          let ga = acts !! (i - 1)
          putStrLn ("> " ++ gaLabel ga)
          loop player (performAction st ga)
      | otherwise -> do
          putStrLn "No such option."
          playerTurn player savePoint st actor

-- The player already has `m` to pass, so pure no-op actions (empty outcomes,
-- e.g. the world's "Wait a moment") are noise in the menu. They remain
-- available to NPCs, who need a "do nothing" affordance to avoid being forced
-- to act. Hide them from the player only. Uses 'candidateActions' so a
-- practice-bound player (e.g. the drama manager) is offered only the affordances
-- of its bound practice.
playerActions :: PraxState -> Character -> [GroundedAction]
playerActions st actor = filter (not . isNoOp) (candidateActions st actor)
  where
    isNoOp ga = case Map.lookup (gaPracticeId ga) (practiceDefs st) of
      Just def | (a : _) <- filter ((== gaActionId ga) . actionName) (actions def)
                 -> null (actionOutcomes a)
      _          -> False

data Choice = Quit | Wait | Save | Pick Int

prompt :: IO Choice
prompt = do
  putStr "> "
  eof <- isEOF
  if eof
    then pure Quit
    else do
      line <- getLine
      pure $ case words line of
        ["q"] -> Quit
        ["s"] -> Save
        ["m"] -> Wait
        [tok] | Just n <- readMaybe tok -> Pick n
        _     -> Pick (-1)

-- Scene rendering ---------------------------------------------------------------

renderScene :: PraxState -> String
renderScene st =
  unlines (map ("  - " ++)
            (act ++ locations ++ orders ++ held ++ tipsy ++ bell
              ++ deaths ++ grudges ++ shuns ++ chats ++ pending ++ trouble
              ++ arcs ++ beliefs ++ moods ++ feelings))
  where
    -- read the *closed* view, so derived facts (e.g. propagated enmity) are shown
    rows sentence = unify sentence (readView st) Map.empty
    val k b = valToString <$> Map.lookup k (b :: Bindings)

    -- derived (or authored) social structure in the emergent sandbox
    grudges =
      [ who ++ " resents " ++ target
      | b <- rows "resents.Who.Target"
      , Just who <- [val "Who" b], Just target <- [val "Target" b] ]
    shuns =
      [ who ++ " is shunning " ++ target
      | b <- rows "shunned.Who.Target"
      , Just who <- [val "Who" b], Just target <- [val "Target" b] ]

    -- the current act, in a play-script world
    act =
      [ "the scene: " ++ s
      | b <- rows "currentScene.S", Just s <- [val "S" b] ]

    locations =
      [ who ++ " is at the " ++ place
      | b <- rows "practice.world.world.at.Who!Place"
      , Just who <- [val "Who" b], Just place <- [val "Place" b] ]

    orders =
      [ who ++ " is waiting for a " ++ bev
      | b <- rows "practice.tendBar.Pl.Bartender.customer.Who!order!Bev"
      , Just who <- [val "Who" b], Just bev <- [val "Bev" b] ]

    held =
      [ who ++ " has a " ++ bev ++ " in hand"
      | b <- rows "practice.tendBar.Pl.Bartender.customer.Who!beverage!Bev"
      , Just who <- [val "Who" b], Just bev <- [val "Bev" b] ]

    tipsy =
      [ who ++ " is looking tipsy"
      | b <- rows "person.Who.tipsy", Just who <- [val "Who" b] ]

    bell =
      [ "the bar is busy — " ++ b' ++ " rang the bell"
      | b <- rows "practice.tendBar.Pl.Bartender.rang", Just b' <- [val "Bartender" b] ]

    -- who has died / been removed from play
    deaths =
      [ who ++ " is dead"
      | b <- rows "dead.Who", Just who <- [val "Who" b] ]

    -- ongoing conversations and their current topic
    chats =
      [ a ++ " and " ++ bb ++ " are chatting (" ++ topic ++ ")"
      | b <- rows "practice.converse.A.B.topic.T"
      , Just a <- [val "A" b], Just bb <- [val "B" b], Just topic <- [val "T" b] ]

    -- pending reactions / obligations
    pending =
      [ gd ++ " hasn't returned " ++ gr ++ "'s greeting"
      | b <- rows "practice.respondGreet.Gr.Gd"
      , Just gr <- [val "Gr" b], Just gd <- [val "Gd" b] ]
      ++
      [ p ++ " owes " ++ bt ++ " a tip"
      | b <- rows "practice.settleUp.P.B"
      , Just p <- [val "P" b], Just bt <- [val "B" b] ]

    -- norm violations and disapproval
    trouble =
      [ w ++ " broke a norm (" ++ n ++ ")"
      | b <- rows "violated.W.N", Just w <- [val "W" b], Just n <- [val "N" b] ]
      ++
      [ ol ++ " disapproves of " ++ off
      | b <- rows "practice.disapproval.Off.Ol"
      , Just off <- [val "Off" b], Just ol <- [val "Ol" b] ]

    -- a character's inner arc (their evening's through-line)
    arcPhrase "hopeful"   = "hopeful"
    arcPhrase "belonging" = "at home here"
    arcPhrase "lonely"    = "out of place"
    arcPhrase s           = s
    arcs =
      [ who ++ " feels " ++ arcPhrase stage
      | b <- rows "Who.arc!Stage"
      , Just who <- [val "Who" b], Just stage <- [val "Stage" b]
      , who `elem` map charName (characters st) ]  -- a real character's arc, not a practice.arc.<who> instance

    -- beliefs that diverge from the truth (here: believed grudges)
    beliefs =
      [ who ++ " believes " ++ subj ++ " resents them"
      | b <- rows "Who.believes.resentedBy.Subj.yes"
      , Just who <- [val "Who" b], Just subj <- [val "Subj" b] ]

    moods =
      [ who ++ " feels " ++ feeling ++ " toward " ++ target
      | b <- rows "Who.mood!Feeling.toward!Target"
      , Just who <- [val "Who" b], Just feeling <- [val "Feeling" b]
      , Just target <- [val "Target" b] ]

    feelings =
      [ a ++ "'s warmth toward " ++ bb ++ ": " ++ score
      | b <- rows "A.relationship.B.warmth.score!N"
      , Just a <- [val "A" b], Just bb <- [val "B" b], Just score <- [val "N" b] ]
