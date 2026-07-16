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

import           Data.Maybe (listToMaybe)
import qualified Data.Map.Strict as Map

import           Prax.Db (exists)
import           Prax.Types
import           Prax.Engine (performAction, roundBoundary)
import           Prax.Planner (pickAction, motiveSignature, candidateActions)

-- | Advance to the next living character (fact @dead.\<name\>@ skipped) and
-- return it, running the engine's round boundary at each rotation wrap (spec
-- @docs/specs/2026-07-16-v44-the-schedule.md@). The next living index @i@ is a
-- WRAP when @i <= cursor@ — equality included, so a single-survivor cast wraps
-- every turn (strict @<@ would freeze engine time). Initial @cursor = -1@ means
-- no boundary fires before round 1. At a wrap: run 'Prax.Engine.roundBoundary'
-- ONCE (it advances the clock, fires due expiries then due rules — a rule may
-- kill a character), then RE-select the actor from the post-boundary aliveness,
-- scanning the fresh round from index 0 (the wrap left @cursor@ untouched and a
-- boundary can only kill, never revive, so re-scanning from @cursor@ finds the
-- same lowest-index survivor — the round starts fresh).
advance :: PraxState -> (Character, PraxState)
advance st0 =
  case nextLiving st0 of
    Nothing -> error "Prax.Loop.advance: no living characters"
    Just i
      | i <= cursor st0 ->                        -- wrap (equality: single survivor)
          let st1 = roundBoundary st0
          in case nextLiving st1 of               -- re-select: a rule may have killed
               Just j  -> (characters st1 !! j, st1 { cursor = j })
               Nothing -> error "Prax.Loop.advance: no living characters"
      | otherwise -> (characters st0 !! i, st0 { cursor = i })
  where
    nextLiving st = listToMaybe
      [ i | k <- [1 .. n], let i = (cursor st + k) `mod` n, alive st i ]
      where n = length (characters st)
    alive st i = not (exists (deadSentence (charName (characters st !! i))) (db st))

-- | Have an NPC act: if their motive signature equals the one their standing
-- intention was based on, act that intention WITHOUT deliberating (spec
-- 2026-07-13-v35 — commitment is the default); otherwise deliberate in full
-- ('pickAction', unchanged), act the result, and store the new intention.
-- A standing action whose grounding is no longer offered cannot be acted:
-- 'stillOffered' checks it against the current candidates directly (the
-- signature's bearing component would miss a vanished NON-bearing action,
-- movement above all).
npcAct :: Int -> Character -> PraxState -> (Maybe GroundedAction, PraxState)
npcAct depth actor st =
  case Map.lookup name (intentions st) of
    Just intent | intentBasis intent == sig, stillOffered (intentAct intent) ->
      act (intentAct intent) st
    _ ->
      let chosen = pickAction depth st actor
          st1 = st { intentions =
                       Map.insert name (Intention chosen sig) (intentions st) }
      in act chosen st1
  where
    name = charName actor
    sig  = motiveSignature st actor
    -- The standing action must still be offered, by full grounded equality —
    -- movement picks are rarely want-bearing yet must expire once acted
    -- (you arrived; decide THERE), and a stale grounding is never performed.
    stillOffered Nothing   = True
    stillOffered (Just ga) = ga `elem` candidateActions st actor
    act (Just ga) s = (Just ga, performAction s ga)
    act Nothing   s = (Nothing, s)

-- | Run @steps@ NPC turns from the given state, collecting the narration of
-- each performed action (idle turns — a 'Nothing' pick — produce no line).
-- Every character is driven by the planner (used for deterministic replay
-- tests). The engine's round boundary rides inside 'advance' (spec v44), so a
-- run of @steps@ turns crosses a boundary at every rotation wrap.
runNpcTicks :: Int -> Int -> PraxState -> ([String], PraxState)
runNpcTicks depth steps = go steps []
  where
    go 0 acc st = (reverse acc, st)
    go k acc st =
      let (actor, st1) = advance st
          (mga, st2)   = npcAct depth actor st1
      in go (k - 1) (maybe acc (\ga -> gaLabel ga : acc) mga) st2
