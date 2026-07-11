-- | Utility-based action selection (Versu §IX) with a beyond-source lookahead
-- extension, redesigned in v23 (spec: docs/specs/2026-07-10-v23-planner-realism-design.md).
--
-- Selection is the paper's apply-and-evaluate: score each candidate by the
-- world it produces. The lookahead is a __round-walk over believed minds__:
-- one imagined round in which each other character within the actor's
-- 'predictionScope' takes one /motivated/ move chosen from the actor's
-- __believed model__ of them ("Prax.Minds" — which can be wrong), followed by
-- the actor's own next choice, recursively. Discounts: 0.9 own future move,
-- 0.5 another's. Accumulation is a discounted stream of absolute utilities
-- over the imagined round. Unknown minds and out-of-scope characters are
-- modeled as still — never as helpful.
module Prax.Planner
  ( evaluate
  , candidateActions
  , predictMove
  , scoreActions
  , pickAction
  ) where

import           Data.List (sortOn)
import qualified Data.Map.Strict as Map
import           Data.Maybe (listToMaybe)
import           Data.Ord (Down(..))

import           Prax.Db (Val (..))
import           Prax.Query (countSatisfying, groundCondition, query)
import           Prax.Types
import           Prax.Engine (readView, possibleActions, performAction)
import           Prax.Minds (selfWants, believedWants)

-- | Total utility of a world to a set of wants: @Σ utility × #satisfying@.
evaluate :: PraxState -> [Want] -> Int
evaluate st wants =
  sum [ wantUtility w * countSatisfying view (wantConditions w) Map.empty
      | w <- wants ]
  where view = readView st

-- | The actions a character may actually take (practice-bound filtering).
candidateActions :: PraxState -> Character -> [GroundedAction]
candidateActions st c =
  let as = possibleActions st (charName c)
  in case charBoundTo c of
       Nothing  -> as
       Just pid -> filter ((== pid) . gaPracticeId) as

-- | Is the mover within the actor's prediction scope? The world's template
-- (over @Actor@/@Witness@) is grounded to the pair and queried against the
-- view; the empty template means everyone.
inScope :: PraxState -> Character -> Character -> Bool
inScope st actor m =
  not (null (query (readView st) grounded Map.empty))
  where
    grounded = map (groundCondition binds) (predictionScope st)
    binds = Map.fromList [ ("Actor",   VStr (charName actor))
                         , ("Witness", VStr (charName m)) ]

-- | The predictor's guess at the mover's next move: the mover's best candidate
-- under the predictor's believed model of them — and only if it strictly
-- improves that model over doing nothing (unmotivated moves are noise, not
-- plan). 'Nothing' when the mind is unreadable or unmotivated.
predictMove :: PraxState -> Character -> Character -> Maybe GroundedAction
predictMove st p m =
  case believedWants st p m of
    []    -> Nothing
    model ->
      let still  = evaluate st model
          scored = sortOn (\(ga, s) -> (Down s, gaLabel ga))
                     [ (a, evaluate (performAction st a) model)
                     | a <- candidateActions st m ]
      in case scored of
           ((a, s) : _) | s > still -> Just a
           _                        -> Nothing

-- The other living characters, one full cycle in cast order starting after
-- the actor (the loop's round-robin order).
othersAfter :: PraxState -> Character -> [Character]
othersAfter st actor =
  filter ((/= charName actor) . charName) (drop (i + 1) cs ++ take (i + 1) cs)
  where
    cs = livingCharacters st
    i  = case [ k | (k, c) <- zip [0 :: Int ..] cs, charName c == charName actor ] of
           (k : _) -> k
           []      -> length cs - 1   -- an actor outside the cast walks everyone

-- | Score each candidate by the imagined round it opens (best first; ties
-- broken by label for determinism).
scoreActions :: Int -> PraxState -> Character -> [(GroundedAction, Double)]
scoreActions depth st actor =
  sortOn (\(ga, s) -> (Down s, gaLabel ga))
    [ (a, valueAfter depth (performAction st a)) | a <- candidateActions st actor ]
  where
    valueAfter d st1 = base + rest
      where
        base = fromIntegral (evaluate st1 (selfWants st1 actor))
        rest
          | d <= 0    = 0
          | otherwise = othersScore + selfNext
          where
            (afterRound, othersScore) = foldl step (st1, 0) (othersAfter st1 actor)
            step (s, acc) m
              | not (inScope s actor m) = (s, acc)
              | otherwise = case predictMove s actor m of
                  Nothing -> (s, acc)
                  Just ga ->
                    let s' = performAction s ga
                    in (s', acc + 0.5 * fromIntegral (evaluate s' (selfWants s' actor)))
            selfNext = case scoreActions (d - 1) afterRound actor of
              ((_, v) : _) -> 0.9 * v
              []           -> 0

-- | The actor's best action (deterministic), if any.
pickAction :: Int -> PraxState -> Character -> Maybe GroundedAction
pickAction depth st actor = fst <$> listToMaybe (scoreActions depth st actor)
