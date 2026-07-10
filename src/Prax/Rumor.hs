-- | Rumor propagation: telling what you have evidence for.
--
-- With witnessing ("Prax.Witness") this closes the information loop: what
-- happens in front of people travels beyond them. A tell plants the same
-- event-belief in the hearer with __hearsay provenance__ —
-- @\<hearer\>.believes.\<event\>.heard.\<teller\>@, one edge per teller, beside any
-- @.seen@ edge — so evidence accumulates (corroboration is countable) and an
-- eyewitness record is never overwritten.
--
-- Like observability, __what is tellable is authored__: 'gossip' declares one
-- tell-action per event pattern (a generic "share any belief" is impossible
-- anyway — a query variable binds a single path segment). Spreading is
-- want-driven: author a character who wants others to know, and the ordinary
-- planner carries the news.
module Prax.Rumor
  ( gossip
  , heard
  ) where

import           Prax.Db (isVariable, pathNames)
import           Prax.Query (Condition (..))
import           Prax.Types (Action, Outcome (..), action)
import           Prax.Beliefs (beliefAbout)
import           Prax.Witness (CoPresence, asRole)

-- | An action: tell a co-present hearer about an event you have evidence for.
--
-- The event pattern may use variables (e.g. @\"stole.Culprit.Item\"@); its
-- __first__ variable is the rumor's subject, who is never offered as a hearer
-- (you don't tell bob about bob's theft). Also never offered: the teller
-- themself, an eyewitness (no news value), and anyone this teller has already
-- told (@.heard.\<teller\>@ doubles as the one-shot marker). The world adds its
-- own gate (e.g. "not someone you distrust") and its 'CoPresence' template —
-- written over @Witness@, mechanically renamed here to bind the @Hearer@.
--
-- The evidence condition is a /prefix/ match on @believes.\<event\>@: the node
-- exists iff some provenance edge sits beneath it, and matching the prefix
-- binds the pattern's variables exactly once per known event no matter how
-- many provenance edges there are — no duplicate tells for a teller who both
-- saw and heard. The event pattern's namespace must not overlap any valued-belief
-- issue path in the same world — 'Witness' and gossip deposits must be the only
-- writers under the pattern's @believes.@ prefix.
gossip :: CoPresence -> [Condition] -> String -> String -> Action
gossip copresence gate pat label =
  action label conds [ Insert (beliefAbout "Hearer" pat ++ ".heard.Actor") ]
  where
    subject = case filter isVariable (pathNames pat) of
      (v : _) -> v
      []      -> error ("gossip: event pattern " ++ show pat
                        ++ " names no one (a rumor is about someone)")
    conds =
      [ Match (beliefAbout "Actor" pat) ]   -- any evidence; binds the pattern's variables
      ++ asRole "Hearer" copresence
      ++ [ Neq "Hearer" "Actor"
         , Neq "Hearer" subject
         , Absent [ Match (beliefAbout "Hearer" pat ++ ".seen") ]
         , Not (beliefAbout "Hearer" pat ++ ".heard.Actor") ]
      ++ gate

-- | Condition: @who@ has hearsay evidence of @event@ (from anyone). A boolean
-- ∃, so multiple sources yield one row — corroboration never duplicates an
-- affordance.
heard :: String -> String -> Condition
heard who event = Exists [ Match (beliefAbout who event ++ ".heard.Src") ]
