-- | Interactive CLI for the storyworlds: round-robin turns, NPCs act
-- autonomously, and on the player's turn a numbered action menu is offered
-- (Versu's \"act / more\" interface). Choose a world on the command line:
-- @prax@ (the bar, default) or @prax intrigue@ (the dramatic episode).
module Main (main) where

import           Data.Maybe (listToMaybe)
import qualified Data.Map.Strict as Map
import qualified Data.Set as Set
import           System.Environment (getArgs)
import           System.IO (BufferMode (NoBuffering), hSetBuffering, isEOF, stdout)
import           Text.Read (readMaybe)

import           Prax.Db (Bindings, unify, valToString)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction)
import           Prax.Loop (advance, npcAct)
import           Prax.Stress
import           Prax.Persist (saveState, loadState)
import qualified Prax.Worlds.Bar as Bar
import qualified Prax.Worlds.Intrigue as Intrigue

-- How many plies of lookahead the NPCs use.
lookaheadDepth :: Int
lookaheadDepth = 2

-- Where an in-game save is written / resumed from.
saveFile :: FilePath
saveFile = "prax.save"

-- A world by name, for both playing and stress-testing.
worldNamed :: [String] -> (String, PraxState, String)
worldNamed ("intrigue" : _) = ("Intrigue (Rome)", Intrigue.intrigueWorld, Intrigue.playerName)
worldNamed _                = ("a night at the bar", Bar.barWorld, Bar.playerName)

-- @prax stress [world]@ — a QA report over many random all-AI playthroughs.
runStress :: [String] -> IO ()
runStress args = do
  let (name, world, _) = worldNamed args
      r = stressTest 200 50 world
  putStrLn ("stress-testing " ++ name ++ " — 200 random runs, cap 50 turns")
  putStrLn ("  endings:   " ++ show (Map.toList (srEndings r)))
  putStrLn ("  coverage:  " ++ show (Set.size (srCoverage r)) ++ " distinct actions fired")
  putStrLn ("  dead ends: " ++ show (srDeadEnds r))
  putStrLn ("  no ending: " ++ show (srNoEnding r) ++ " / " ++ show (srRuns r) ++ " runs")

main :: IO ()
main = do
  hSetBuffering stdout NoBuffering
  args <- getArgs
  case args of
    ("stress" : rest) -> runStress rest
    _                 -> play args

play :: [String] -> IO ()
play args = do
  let (title, blurb, world, player) = case args of
        ("intrigue" : _) ->
          ( "prax — Intrigue (Rome)"
          , "You are Marcus, the poet. The others act on their own."
          , Intrigue.intrigueWorld, Intrigue.playerName )
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
            Just ga -> putStrLn ("  " ++ gaLabel ga)
            Nothing -> pure ()          -- idle NPCs stay quiet to reduce noise
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
-- to act. Hide them from the player only.
playerActions :: PraxState -> Character -> [GroundedAction]
playerActions st actor = filter (not . isNoOp) (possibleActions st (charName actor))
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
            (locations ++ orders ++ held ++ tipsy ++ bell
              ++ deaths ++ chats ++ pending ++ trouble ++ arcs ++ beliefs ++ moods ++ feelings))
  where
    rows sentence = unify sentence (db st) Map.empty
    val k b = valToString <$> Map.lookup k (b :: Bindings)

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
      , Just who <- [val "Who" b], Just stage <- [val "Stage" b] ]

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
