-- | A seeded drama die as ordinary world state (spec
-- @docs/specs/2026-07-15-v38-chance-feelings.md@): one @seed!N@ fact, a
-- Lehmer stream over it, and 'draw' — the only authoring surface. Because
-- the die is facts, determinism, goldens, replay, and 'Prax.Persist' all
-- survive for free; the initial seed is an AUTHORED world parameter (it
-- selects the playthrough's fate, and the goldens pin it). This is a drama
-- die, not a statistics library: low bits of a MINSTD stream are plenty to
-- decide whether a temper flares, and nothing here is fit for cryptography
-- or simulation science.
module Prax.Rng
  ( rngSetup
  , draw
  , seedPath
  ) where

import           Prax.Query (Condition (..), CmpOp (..), CalcOp (..))
import           Prax.Types

-- | The die's fact family. One slot: the stream position.
seedPath :: String
seedPath = "seed"

-- Park & Miller (1988), "Random number generators: good ones are hard to
-- find" — the MINSTD minimal standard. Mechanism with published provenance,
-- fixed here, never tuned: the AUTHORED numbers are 'draw''s odds.
lehmerA, lehmerM :: Integer
lehmerA = 16807
lehmerM = 2147483647   -- 2^31 - 1

-- | Seed the die (append to world setup). Loud on a seed outside the
-- stream's domain (0 and multiples of the modulus are fixed points — a die
-- that always rolls the same face).
rngSetup :: Integer -> [Outcome]
rngSetup s
  | s <= 0 || s >= lehmerM =
      error ("Prax.Rng: seed must lie in (0, " ++ show (lehmerM - 1) ++ "]")
  | otherwise = [ Insert (seedPath ++ "!" ++ show s) ]

-- | "With probability num\/den, where conds also hold, apply outs." The
-- fragment authors append to an action's outcomes. Compiles to TWO
-- ForEach outcomes: an unconditional stream advance, then the roll against
-- the fresh seed — so every draw consumes EXACTLY one step, hit or miss
-- (a failed roll must not freeze the die: a provocation that failed once
-- must not fail identically forever). Guards are loud: odds must be a real
-- chance (0 < num < den — certainty and impossibility are authored
-- dishonesty; use a plain outcome or nothing), and the caller's conditions
-- and outcomes may not use the reserved stream variables.
draw :: Int -> Int -> [Condition] -> [Outcome] -> [Outcome]
draw num den conds outs
  | num <= 0 || num >= den =
      error ("Prax.Rng: draw odds " ++ show num ++ "/" ++ show den
             ++ " must satisfy 0 < num < den")
  | (v : _) <- offenders =
      error ("Prax.Rng: draw body authors " ++ show v
             ++ " -- the Prax namespace is reserved for the die's own machinery")
  | otherwise =
      [ ForEach [ Match (seedPath ++ "!PraxS")
                , Calc "PraxS2" Mul "PraxS" (show lehmerA)
                , Calc "PraxS3" Mod "PraxS2" (show lehmerM) ]
                [ Insert (seedPath ++ "!PraxS3") ]
      , ForEach ([ Match (seedPath ++ "!PraxS")
                 , Calc "PraxR" Mod "PraxS" (show den)
                 , Cmp Lt "PraxR" (show num) ] ++ conds)
                outs
      ]
  where
    offenders = authoredVarClash [] conds outs
