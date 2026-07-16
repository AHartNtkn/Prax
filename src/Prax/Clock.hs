-- | The world's turn counter, in one home. "Prax.Sight"'s ticker composes
-- these fragments (perception rides the clock); a world that wants time
-- without perception registers the standalone 'clockP'/'clockChar' instead.
-- __Instead, never alongside__: each registered ticker advances the counter
-- once per round, so a world carries exactly one — Sight's composed ticker
-- OR the standalone clock, or its rounds are double-counted.
-- "Prax.Drift" reads the counter through 'turnPath'; "Prax.TypeCheck"'s
-- ClocklessDrift check tests for it by the same name.
module Prax.Clock
  ( tickConditions
  , tickOutcome
  , clockSeed
  , clockName
  , clockP
  , clockChar
  , clockSetup
  ) where

import           Prax.Query (Condition (..), CalcOp (..))
import           Prax.Types

-- | Read-and-advance fragments: bind the current turn, compute the next.
-- @PraxN@\/@PraxM@ are machinery (v40 namespace).
tickConditions :: [Condition]
tickConditions = [ Match (turnPath ++ "!PraxN"), Calc "PraxM" Add "PraxN" "1" ]

tickOutcome :: Outcome
tickOutcome = Insert (turnPath ++ "!PraxM")

-- | The seed fact (turn 0). Part of 'clockSetup' and "Prax.Sight"'s setup.
clockSeed :: Outcome
clockSeed = Insert (turnPath ++ "!0")

-- | The standalone ticker (for drift-without-perception worlds): a bodiless
-- character whose blank-label action only advances the counter. Distinct
-- from "Prax.Script"'s scene-local @_clock@ (a different concept with its
-- own name and family).
clockName :: String
clockName = "_time"

clockP :: Practice
clockP = practice
  { practiceId = "time"
  , practiceName = "time passes"
  , roles = ["S"]
  , actions = [ action "" (Eq "Actor" clockName : tickConditions) [ tickOutcome ] ]
  }

clockChar :: Character
clockChar = (character clockName) { charBoundTo = Just "time" }

clockSetup :: [Outcome]
clockSetup = [ Insert "practice.time.here", clockSeed ]
