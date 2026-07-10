-- | Authored witnessing: information asymmetry from observation.
--
-- An action's public appearance is a semantic property its author states with
-- 'observable' — undeclared actions are not events (waiting is not news), and
-- the declared appearance may deliberately differ from what the action /does/
-- (poisoning the cup can look like pouring wine).
--
-- A witnessed event is an ordinary belief ("Prax.Beliefs"):
-- @\<witness\>.believes.\<event\>!seen@ — the @!seen@ value records /provenance/
-- (direct observation), so later layers can distinguish an eyewitness from
-- hearsay while all existing belief machinery works on both.
--
-- Co-presence is __world vocabulary__ (the engine has no notion of place): each
-- world supplies a 'CoPresence' template once, relating the fixed variables
-- @Witness@ and @Actor@ in its own terms.
module Prax.Witness
  ( CoPresence
  , observable
  , saw
  ) where

import           Prax.Query (Condition (..))
import           Prax.Types (Action (..), Outcome (..))
import           Prax.Beliefs (beliefSentence, believesThat)

-- | Conditions relating the fixed variables @Witness@ and @Actor@ in the
-- world's own vocabulary (location facts, current scene, …). Everything that
-- constrains who can witness is the template's job; 'observable' adds only the
-- actor-exclusion.
type CoPresence = [Condition]

-- | Declare an action's public appearance: every co-present character (except
-- the actor, who already knows what they did) comes to believe @event@ with
-- provenance @seen@. The event sentence may use the action's own variables.
observable :: CoPresence -> String -> Action -> Action
observable copresence event act =
  act { actionOutcomes =
          actionOutcomes act
            ++ [ ForEach (copresence ++ [ Neq "Witness" "Actor" ])
                         [ Insert (beliefSentence "Witness" event "seen") ] ] }

-- | Condition: @who@ directly witnessed @event@.
saw :: String -> String -> Condition
saw who event = believesThat who event "seen"
