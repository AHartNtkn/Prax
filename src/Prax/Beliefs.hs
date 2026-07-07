-- | Beliefs (Versu paper §X).
--
-- Versu keeps a single shared world state, and stores /individual/ beliefs only
-- for the specific issues where an agent's view may diverge from the truth — for
-- "false beliefs, or factual disagreements." This module is the reusable
-- convention for those per-issue beliefs, on top of the existing engine (no new
-- machinery): a belief is just a fact under the believing agent.
--
--   @X.believes.\<issue\>!Value@
--
-- Beliefs are single-slot per issue (via the corrected @!@): a new value for an
-- issue overrides the old. Because a belief lives under the agent, two agents can
-- hold different values for the same issue (divergence), and either can differ
-- from the corresponding shared-world fact (a false belief).
--
-- Note: this models explicit per-issue beliefs, not the paper's un-built
-- quantified/nested beliefs (LEDGER #28). There is also no single "believe X, or
-- else fall back to the world" query operator — that needs disjunction (LEDGER
-- #7); author it as two conditioned actions instead.
module Prax.Beliefs
  ( beliefSentence
  , beliefAbout
  , believe
  , believesThat
  , forget
  ) where

import           Prax.Query (Condition (..))
import           Prax.Types (Outcome (..))

-- | The sentence @who.believes.\<issue\>!value@. @issue@ may be a dotted
-- sub-path (e.g. @"resentedBy.ada"@). Any part may be an action variable,
-- grounded when used in an outcome/condition.
beliefSentence :: String -> String -> String -> String
beliefSentence who issue value = who ++ ".believes." ++ issue ++ "!" ++ value

-- | The path @who.believes.\<issue\>@, for binding the believed value:
-- @Match (beliefAbout who issue ++ "!V")@.
beliefAbout :: String -> String -> String
beliefAbout who issue = who ++ ".believes." ++ issue

-- | @who@ comes to believe @issue@ has @value@ (overriding any prior value).
believe :: String -> String -> String -> Outcome
believe who issue value = Insert (beliefSentence who issue value)

-- | Condition: @who@ believes @issue@ is @value@.
believesThat :: String -> String -> String -> Condition
believesThat who issue value = Match (beliefSentence who issue value)

-- | @who@ drops any belief about @issue@.
forget :: String -> String -> Outcome
forget who issue = Delete (beliefAbout who issue)
