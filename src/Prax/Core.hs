-- | The Core Model: relationships (Versu paper §X).
--
-- Versu calls the agent's emotional and relationship state "the channel through
-- which the different practices communicate." It is ordinary DB state — this
-- module adds no new engine machinery, only a reusable standard library of
-- conventions on top of "Prax.Engine":
--
--   * relationship evaluations are numeric, multiple, asymmetric, and carry a
--     reason (@A.relationship.B.Role.score!N@, @A.relationship.B.Role.reason!Why@);
--   * a public "bond" is symmetric (@bond.A.B!State@ written both ways).
--
-- Emotions moved to "Prax.Emotion" (spec 2026-07-15-v38): coexisting episodic
-- @feels@ facts replaced this module's single-slot, auto-overriding
-- @mood!@/@priorMood@ machinery, which is deleted with its last consumer
-- (the bar's migration).
--
-- The read-modify-write pieces (seed-then-accumulate a score) are provided
-- once as 'Function's ('coreFns'), reusing the same @Call@/@Calc@ machinery
-- proven by the drink counter in "Prax.Worlds.Bar". Register 'coreFns' with
-- 'Prax.Engine.defineFunctions' to make the smart constructors below usable.
module Prax.Core
  ( -- * Registering the library
    coreFns
    -- * Relationship roles
  , warmth, respect
    -- * Effects (smart-constructor 'Outcome's)
  , adjustScore
  , setBond
    -- * Conditions
  , scoreAtLeast
  ) where

import           Data.Char (toUpper)

import           Prax.Query (Condition (..), CmpOp (..), CalcOp (..))
import           Prax.Types

-- Relationship roles -----------------------------------------------------------

-- | Example evaluation dimensions. A relationship is judged on any number of
-- these, independently in each direction.
warmth, respect :: String
warmth  = "warmth"
respect = "respect"

-- Effects ----------------------------------------------------------------------

-- | Adjust @a@'s evaluation of @b@ on @role@ by @delta@ (seeding it if this is
-- the first interaction), recording @reason@. Negative deltas cool the relation.
adjustScore :: String -> String -> String -> Int -> String -> Outcome
adjustScore a b role delta reason =
  Call "prax_adjustScore" [a, b, role, show delta, reason]

-- | Set the symmetric public bond between @a@ and @b@ (e.g. @"friends"@).
setBond :: String -> String -> String -> Outcome
setBond a b st = Call "prax_setBond" [a, b, st]

-- Conditions -------------------------------------------------------------------

-- | Require @a@'s evaluation of @b@ on @role@ to be at least @n@. Binds a
-- role-specific score variable (safe to combine across distinct roles).
scoreAtLeast :: String -> String -> String -> Int -> [Condition]
scoreAtLeast a b role n =
  [ Match (a ++ ".relationship." ++ b ++ "." ++ role ++ ".score!" ++ v)
  , Cmp Gte v (show n)
  ]
  where v = "Score" ++ capitalize role

capitalize :: String -> String
capitalize []       = []
capitalize (c : cs) = toUpper c : cs

-- The library functions -----------------------------------------------------------

-- | The reusable core-model functions. Register them with
-- 'Prax.Engine.defineFunctions'; they are found by name whenever an action
-- calls them.
coreFns :: [Function]
coreFns = [adjustScoreFn, setBondFn]

-- Add Delta to an existing score, or seed it with Delta on first interaction.
adjustScoreFn :: Function
adjustScoreFn = Function "prax_adjustScore" ["A", "B", "Role", "Delta", "Reason"]
  [ FnCase [ Match "A.relationship.B.Role.score!N", Calc "M" Add "N" "Delta" ]
      [ Insert "A.relationship.B.Role.score!M"
      , Insert "A.relationship.B.Role.reason!Reason" ]
  , FnCase []
      [ Insert "A.relationship.B.Role.score!Delta"
      , Insert "A.relationship.B.Role.reason!Reason" ]
  ]

-- Write the symmetric bond in both directions at once.
setBondFn :: Function
setBondFn = Function "prax_setBond" ["A", "B", "State"]
  [ FnCase [] [ Insert "bond.A.B!State", Insert "bond.B.A!State" ] ]
