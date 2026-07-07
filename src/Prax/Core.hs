-- | The Core Model: emotions and relationships (Versu paper §X).
--
-- Versu calls the agent's emotional and relationship state "the channel through
-- which the different practices communicate." It is ordinary DB state — this
-- module adds no new engine machinery, only a reusable standard library of
-- conventions on top of "Prax.Engine":
--
--   * emotions are single-slot and auto-overriding (the corrected @!@), remember
--     their target and cause, and keep the previous mood
--     (@X.mood!Feeling.toward!T.because!C@, @X.priorMood!Old@);
--   * relationship evaluations are numeric, multiple, asymmetric, and carry a
--     reason (@A.relationship.B.Role.score!N@, @A.relationship.B.Role.reason!Why@);
--   * a public "bond" is symmetric (@bond.A.B!State@ written both ways).
--
-- The read-modify-write pieces (remember-prior-mood, seed-then-accumulate a
-- score) are provided once as practice 'Function's on 'coreLib', reusing the
-- same @Call@/@Calc@ machinery proven by the drink counter in "Prax.Worlds.Bar".
-- Register 'coreLib' with 'Prax.Engine.definePractice' to make the smart
-- constructors below usable.
module Prax.Core
  ( -- * Registering the library
    coreLib
    -- * Emotions (an Ekman-based vocabulary)
  , happy, sad, angry, afraid, disgusted, surprised, annoyed, pleased
    -- * Relationship roles
  , warmth, respect
    -- * Effects (smart-constructor 'Outcome's)
  , setMood
  , adjustScore
  , setBond
    -- * Conditions
  , moodIs
  , scoreAtLeast
  ) where

import           Data.Char (toUpper)

import           Prax.Query (Condition (..), CmpOp (..), CalcOp (..))
import           Prax.Types

-- Emotions ---------------------------------------------------------------------

-- | An Ekman-based emotional vocabulary (Versu used "a fine-grained set of
-- emotional states, based on Ekman's work"). Plain symbols; extend freely.
happy, sad, angry, afraid, disgusted, surprised, annoyed, pleased :: String
happy     = "happy"
sad       = "sad"
angry     = "angry"
afraid    = "afraid"
disgusted = "disgusted"
surprised = "surprised"
annoyed   = "annoyed"
pleased   = "pleased"

-- Relationship roles -----------------------------------------------------------

-- | Example evaluation dimensions. A relationship is judged on any number of
-- these, independently in each direction.
warmth, respect :: String
warmth  = "warmth"
respect = "respect"

-- Effects ----------------------------------------------------------------------

-- | Set @who@'s current mood to @feeling@, directed at @target@ and caused by
-- @cause@, remembering the previous mood. Any argument may be a constant or an
-- action variable (e.g. @"Actor"@); it is grounded when the outcome runs.
setMood :: String -> String -> String -> String -> Outcome
setMood who feeling target cause = Call "prax_setMood" [who, feeling, target, cause]

-- | Adjust @a@'s evaluation of @b@ on @role@ by @delta@ (seeding it if this is
-- the first interaction), recording @reason@. Negative deltas cool the relation.
adjustScore :: String -> String -> String -> Int -> String -> Outcome
adjustScore a b role delta reason =
  Call "prax_adjustScore" [a, b, role, show delta, reason]

-- | Set the symmetric public bond between @a@ and @b@ (e.g. @"friends"@).
setBond :: String -> String -> String -> Outcome
setBond a b st = Call "prax_setBond" [a, b, st]

-- Conditions -------------------------------------------------------------------

-- | True when @who@'s current mood is @feeling@.
moodIs :: String -> String -> Condition
moodIs who feeling = Match (who ++ ".mood!" ++ feeling)

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

-- The library practice -----------------------------------------------------------

-- | A never-instantiated practice that simply carries the reusable core-model
-- functions. Register it with 'Prax.Engine.definePractice'; its functions are
-- found by name whenever an action calls them.
coreLib :: Practice
coreLib = practice
  { practiceId   = "core"
  , practiceName = "core model library"
  , functions    = [setMoodFn, adjustScoreFn, setBondFn]
  }

-- Set a new mood, remembering the prior one if there was one.
setMoodFn :: Function
setMoodFn = Function "prax_setMood" ["Who", "Feeling", "Target", "Cause"]
  [ FnCase [ Match "Who.mood!Old" ]
      [ Insert "Who.priorMood!Old"
      , Insert "Who.mood!Feeling.toward!Target"
      , Insert "Who.mood!Feeling.because!Cause" ]
  , FnCase []
      [ Insert "Who.mood!Feeling.toward!Target"
      , Insert "Who.mood!Feeling.because!Cause" ]
  ]

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
