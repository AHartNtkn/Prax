-- | Utility-based reactive action selection (Versu §IX).
--
-- A character evaluates the /actual/ world that results from an action —
-- @performAction@ then score — and chooses the best. Utility is the sum over the
-- character's wants of @utility × (number of satisfying instantiations)@; every
-- separate binding that satisfies a want scores again (§IX-A). Because the state
-- is immutable, "apply and evaluate" needs no clone-and-undo: the speculative
-- state is simply discarded.
--
-- Lookahead is optimistic: the value of a world to @actor@ is its immediate
-- utility plus the best single improving move available to /any/ character next,
-- discounted 0.9 when that move is the actor's own and 0.5 when it is another's,
-- recursively to the given depth.
module Prax.Planner
  ( evaluate
  , worldValue
  , candidateActions
  , scoreActions
  , pickAction
  ) where

import           Data.List (sortBy)
import qualified Data.Map.Strict as Map
import           Data.Maybe (listToMaybe)
import           Data.Ord (Down(..), comparing)

import           Prax.Query (countSatisfying)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction)

-- | Total utility of a world to a set of wants: @Σ utility × #satisfying@.
evaluate :: PraxState -> [Want] -> Int
evaluate st wants =
  sum [ wantUtility w * countSatisfying (db st) (wantConditions w) Map.empty
      | w <- wants ]

-- | The actions a character may actually take: all their affordances, filtered
-- to a bound practice if the character is practice-bound.
candidateActions :: PraxState -> Character -> [GroundedAction]
candidateActions st c =
  let as = possibleActions st (charName c)
  in case charBoundTo c of
       Nothing  -> as
       Just pid -> filter ((== pid) . gaPracticeId) as

-- | Value of world @st@ to @actor@, looking @depth@ plies ahead.
worldValue :: Int -> PraxState -> Character -> Double
worldValue depth st actor = base + future
  where
    base = fromIntegral (evaluate st (charWants actor))
    future
      | depth <= 0 = 0
      | otherwise  =
          maximum (0 : [ discount mover
                          * (worldValue (depth - 1) (performAction st a) actor - base)
                       | mover <- characters st
                       , a <- candidateActions st mover ])
    discount mover
      | charName mover == charName actor = 0.9   -- the actor's own future move
      | otherwise                        = 0.5   -- a move controlled by someone else

-- | Score each of the actor's own candidate actions by the value of the world
-- that results, sorted best first (ties broken by label for determinism).
scoreActions :: Int -> PraxState -> Character -> [(GroundedAction, Double)]
scoreActions depth st actor =
  sortBy (comparing (\(ga, s) -> (Down s, gaLabel ga)))
    [ (a, worldValue depth (performAction st a) actor)
    | a <- candidateActions st actor ]

-- | The actor's best action (deterministic: first of the top-scoring), if any.
pickAction :: Int -> PraxState -> Character -> Maybe GroundedAction
pickAction depth st actor = fst <$> listToMaybe (scoreActions depth st actor)
