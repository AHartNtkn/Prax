-- | Authored witnessing: information asymmetry from observation.
--
-- An action's public appearance is a semantic property its author states with
-- 'observable' — undeclared actions are not events (waiting is not news), and
-- the declared appearance may deliberately differ from what the action /does/
-- (poisoning the cup can look like pouring wine).
--
-- A witnessed event is an ordinary belief ("Prax.Beliefs"):
-- @\<witness\>.believes.\<event\>.seen@ — the @.seen@ leaf records /provenance/
-- (direct observation, /multi-valued/). The rumor layer (@Prax.Rumor@, v20) adds
-- @.heard.\<source\>@ edges beside @.seen@, one per teller, so evidence accumulates
-- instead of overwriting — an exclusive slot would let hearsay destroy an eyewitness
-- record, and mixing @!@ and @.@ on the slot is a @CardinalityClash@ the checker rejects.
--
-- Co-presence is __world vocabulary__ (the engine has no notion of place): each
-- world supplies a 'CoPresence' template once, relating the fixed variables
-- @Witness@ and @Actor@ in its own terms.
module Prax.Witness
  ( CoPresence
  , witnessed
  , observable
  , saw
  , asRole
  ) where

import qualified Data.Map.Strict as Map

import           Prax.Db (Val (..))
import           Prax.Query (Condition (..), groundCondition)
import           Prax.Types (Action (..), Outcome (..))
import           Prax.Beliefs (beliefAbout)

-- | Conditions relating the fixed variables @Witness@ and @Actor@ in the
-- world's own vocabulary (location facts, current scene, …). Everything that
-- constrains who can witness is the template's job; 'observable' adds only the
-- actor-exclusion.
type CoPresence = [Condition]

-- | The witness deposit as a first-class outcome: every co-present character
-- (except the actor) comes to believe @event@ with provenance @seen@. This is
-- what 'observable' appends; exported so generated actions (e.g.
-- "Prax.Project" stages) can carry observability in their own effects.
witnessed :: CoPresence -> String -> Outcome
witnessed copresence event =
  ForEach (copresence ++ [ Neq "Witness" "Actor" ])
          [ Insert (beliefAbout "Witness" event ++ ".seen") ]

-- | Declare an action's public appearance: every co-present character (except
-- the actor, who already knows what they did) comes to believe @event@ with
-- provenance @seen@. The event sentence may use the action's own variables.
observable :: CoPresence -> String -> Action -> Action
observable copresence event act =
  act { actionOutcomes = actionOutcomes act ++ [ witnessed copresence event ] }

-- | Condition: @who@ directly witnessed @event@.
saw :: String -> String -> Condition
saw who event = Match (beliefAbout who event ++ ".seen")

-- | Retarget a co-presence template: substitute a different variable for
-- @Witness@ (e.g. @Hearer@), so the template stays single-sourced in the
-- world while other layers quantify over their own role.
asRole :: String -> CoPresence -> [Condition]
asRole v = map (groundCondition (Map.singleton "Witness" (VStr v)))
