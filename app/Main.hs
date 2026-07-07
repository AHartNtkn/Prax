-- | Interactive CLI for the bar storyworld: round-robin turns, NPCs act
-- autonomously, and on the player's turn a numbered action menu is offered
-- (Versu's \"act / more\" interface).
module Main (main) where

import qualified Data.Map.Strict as Map
import           System.IO (BufferMode (NoBuffering), hSetBuffering, isEOF, stdout)
import           Text.Read (readMaybe)

import           Prax.Db (Bindings, unify, valToString)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction)
import           Prax.Loop (advance, npcAct)
import           Prax.Worlds.Bar (barWorld, playerName)

-- How many plies of lookahead the NPCs use.
lookaheadDepth :: Int
lookaheadDepth = 2

main :: IO ()
main = do
  hSetBuffering stdout NoBuffering
  putStrLn "============================================"
  putStrLn "  prax — a night at the bar"
  putStrLn "============================================"
  putStrLn "You are 'you'. Others act on their own."
  loop barWorld

loop :: PraxState -> IO ()
loop st = do
  let (actor, st1) = advance st
  if charName actor == playerName
    then playerTurn st1 actor
    else do
      let (mga, st2) = npcAct lookaheadDepth actor st1
      case mga of
        Just ga -> putStrLn ("  " ++ gaLabel ga)
        Nothing -> pure ()          -- idle NPCs stay quiet to reduce noise
      loop st2

playerTurn :: PraxState -> Character -> IO ()
playerTurn st actor = do
  putStrLn ""
  putStrLn "-------------------- scene --------------------"
  putStr (renderScene st)
  let acts = possibleActions st (charName actor)
  putStrLn ("Your move (" ++ charName actor ++ "):")
  mapM_ (\(i, a) -> putStrLn ("  " ++ show i ++ ") " ++ gaLabel a))
        (zip [1 :: Int ..] acts)
  putStrLn "  m) wait and let others act"
  putStrLn "  q) quit"
  choice <- prompt
  case choice of
    Quit -> putStrLn "Bye. The night winds down."
    Wait -> loop st
    Pick i
      | i >= 1 && i <= length acts -> do
          let ga = acts !! (i - 1)
          putStrLn ("> " ++ gaLabel ga)
          loop (performAction st ga)
      | otherwise -> do
          putStrLn "No such option."
          playerTurn st actor

data Choice = Quit | Wait | Pick Int

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
        ["m"] -> Wait
        [tok] | Just n <- readMaybe tok -> Pick n
        _     -> Pick (-1)

-- Scene rendering ---------------------------------------------------------------

renderScene :: PraxState -> String
renderScene st =
  unlines (map ("  - " ++) (locations ++ orders ++ held ++ tipsy ++ bell))
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
