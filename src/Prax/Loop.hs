-- | The turn loop: round-robin turn taking over the cast (Praxish's @tick@).
--
-- Pure stepping primitives so the same core drives both the interactive CLI and
-- deterministic replay tests. The planner is deterministic, so 'runNpcTicks'
-- yields a reproducible narration trace.
module Prax.Loop
  ( advance
  , npcAct
  , runNpcTicks
  ) where

import           Prax.Types
import           Prax.Engine (performAction)
import           Prax.Planner (pickAction)

-- | Advance the round-robin cursor and return the character whose turn it is.
advance :: PraxState -> (Character, PraxState)
advance st
  | n == 0    = error "Prax.Loop.advance: no characters in the cast"
  | otherwise = (characters st !! i, st { cursor = i })
  where
    n = length (characters st)
    i = (cursor st + 1) `mod` n

-- | Have an NPC choose (looking @depth@ plies ahead) and perform its best
-- action, returning what it did (if anything) and the resulting state.
npcAct :: Int -> Character -> PraxState -> (Maybe GroundedAction, PraxState)
npcAct depth actor st = case pickAction depth st actor of
  Just ga -> (Just ga, performAction st ga)
  Nothing -> (Nothing, st)

-- | Run @steps@ NPC turns from the given state, collecting the narration of
-- each performed action (idle turns produce no line). Every character is driven
-- by the planner (used for deterministic replay tests).
runNpcTicks :: Int -> Int -> PraxState -> ([String], PraxState)
runNpcTicks depth steps = go steps []
  where
    go 0 acc st = (reverse acc, st)
    go k acc st =
      let (_actor, st1) = advance st
          (mga, st2)    = npcAct depth _actor st1
      in go (k - 1) (maybe acc (\ga -> gaLabel ga : acc) mga) st2
