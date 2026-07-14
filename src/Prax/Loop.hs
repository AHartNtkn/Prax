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

import qualified Data.Map.Strict as Map

import           Prax.Db (exists)
import           Prax.Types
import           Prax.Engine (performAction)
import           Prax.Planner (pickAction, motiveSignature, candidateActions)

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
-- each performed action (idle turns produce no line, and neither do silent
-- bodiless tickers — an empty label is the authored signal for "acts, but
-- says nothing", the same convention the interactive CLI applies). Every
-- character is driven by the planner (used for deterministic replay tests).
runNpcTicks :: Int -> Int -> PraxState -> ([String], PraxState)
runNpcTicks depth steps = go steps []
  where
    go 0 acc st = (reverse acc, st)
    go k acc st =
      let (_actor, st1) = advance st
          (mga, st2)    = npcAct depth _actor st1
      in go (k - 1) (maybe acc (narrate acc) mga) st2
    narrate acc ga
      | all (== ' ') (gaLabel ga) = acc
      | otherwise                 = gaLabel ga : acc
