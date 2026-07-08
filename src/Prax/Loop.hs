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

import           Prax.Db (exists)
import           Prax.Types
import           Prax.Engine (performAction)
import           Prax.Planner (pickAction)

-- | Advance the round-robin cursor to the next living character and return it.
-- Dead characters (fact @dead.\<name\>@) are skipped; the cursor stays an index
-- into the full cast list so ordering is preserved.
advance :: PraxState -> (Character, PraxState)
advance st =
  case [ i | k <- [1 .. n], let i = (cursor st + k) `mod` n, alive i ] of
    (i : _) -> (characters st !! i, st { cursor = i })
    []      -> error "Prax.Loop.advance: no living characters"
  where
    n = length (characters st)
    alive i = not (exists (deadSentence (charName (characters st !! i))) (db st))

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
