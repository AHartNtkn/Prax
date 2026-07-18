-- | A seeded drama die as ENGINE STATE (spec
-- @docs/specs/2026-07-18-v50-machinery-state.md@): the Lehmer stream lives in
-- 'Prax.Types.rngSeed', not the fact base, and 'draw' is the only authoring
-- surface. This module keeps ALL the die math — the Park & Miller MINSTD
-- constants, the one-step advance ('rollStep'), and the seed-domain bounds
-- ('seedBounds', consumed by 'Prax.Engine.seedDie'); 'draw' compiles authored
-- odds to a first-class 'Roll' outcome that 'Prax.Engine.performCooked'
-- executes against the engine stream. This is a drama die, not a statistics
-- library: low bits of a MINSTD stream are plenty to decide whether a temper
-- flares, and nothing here is fit for cryptography or simulation science.
module Prax.Rng
  ( draw
  , rollStep
  , seedBounds
  ) where

import           Prax.Query (Condition)
import           Prax.Types

-- Park & Miller (1988), "Random number generators: good ones are hard to
-- find" — the MINSTD minimal standard. Mechanism with published provenance,
-- fixed here, never tuned: the AUTHORED numbers are 'draw''s odds.
lehmerA, lehmerM :: Integer
lehmerA = 16807
lehmerM = 2147483647   -- 2^31 - 1

-- | One Lehmer step: the advanced stream value. The roll basis IS this
-- advanced value (a state advance evicts the old seed, so the roll reads the
-- fresh one — spec v50). Integer arithmetic, matching the db's Calc domain, so
-- the residence move off the fact base is byte-identical.
rollStep :: Integer -> Integer
rollStep s = (s * lehmerA) `mod` lehmerM

-- | The seed's valid domain, inclusive: strictly between 0 and the modulus.
-- @0@ and multiples of the modulus are fixed points of the stream (a die that
-- always rolls the same face), so they are excluded — 'Prax.Engine.seedDie'
-- rejects a seed outside these bounds loudly.
seedBounds :: (Integer, Integer)
seedBounds = (1, lehmerM - 1)

-- | "With probability num\/den, where conds also hold, apply outs." The
-- fragment authors append to an action's outcomes. Compiles to a single
-- 'Roll' outcome — the engine advances the stream once and rolls on the
-- advanced value, so every draw consumes EXACTLY one step, hit or miss (a
-- failed roll must not freeze the die: a provocation that failed once must not
-- fail identically forever). Guards are loud: odds must be a real chance
-- (0 < num < den — certainty and impossibility are authored dishonesty; use a
-- plain outcome or nothing), and the caller's conditions and outcomes may not
-- use the reserved Prax namespace (v40 hygiene stands even though the die no
-- longer splices anything).
draw :: Int -> Int -> [Condition] -> [Outcome] -> [Outcome]
draw num den conds outs
  | num <= 0 || num >= den =
      error ("Prax.Rng: draw odds " ++ show num ++ "/" ++ show den
             ++ " must satisfy 0 < num < den")
  | (v : _) <- offenders =
      error ("Prax.Rng: draw body authors " ++ show v
             ++ " -- the Prax namespace is reserved for the die's own machinery")
  | otherwise = [ Roll num den conds outs ]
  where
    offenders = authoredVarClash [] conds outs
