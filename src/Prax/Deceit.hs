-- | Secrets and deception: managing what is known.
--
-- __Concealment is authoring, not machinery.__ 'conceal' is a want that nobody
-- believe an event; the planner's lookahead already simulates the witness
-- deposits ("Prax.Witness"), so an agent who values a secret avoids being seen
-- /by planning/ — waiting for the room to empty falls out of utility.
--
-- __A lie is an assertion without evidence.__ 'lie' mirrors 'Prax.Rumor.gossip'
-- with two inversions: the speaker must hold /no/ evidence (if they ever hear
-- their own lie back, the lie action vanishes and plain gossip appears), and
-- the fabricated subject binds from world-supplied conditions (whom you
-- /could/ frame) rather than from a belief. The effect is identical to
-- gossip's — the deceived hold real hearsay, indistinguishable from truth, and
-- the whole rumor/reputation stack cascades on the falsehood unmodified.
--
-- __A lie marks the liar.__ The only residue is the liar's own memory —
-- @\<liar\>.lied.\<hearer\>.\<event\>@, rooted under their name like all
-- mental state: owned, forgettable (one 'Delete' on its root), perishing with
-- its bearer. There is deliberately no objective record: history persists
-- only through the marks it makes, and the truth can become unrecoverable.
-- Exculpation, if ever authored, flows through mark-bearers (confession,
-- testimony), never through consulting a ledger. What the mark buys now is
-- conscience: a trait that values your own @lied@ marks negatively
-- ("Prax.Persona") is a reason not to.
module Prax.Deceit
  ( conceal
  , lie
  ) where

import           Prax.Db (isVariable, pathNames)
import           Prax.Query (Condition (..))
import           Prax.Types (Action, Outcome (..), Want (..), action, authoredVarClash, authoredPatClash)
import           Prax.Beliefs (beliefAbout)
import           Prax.Witness (CoPresence, asRole)

-- | A desire that nobody believe @event@ — how much the secret is worth is
-- authored character. The event must be variable-free (a concealment want
-- quantifies over /observers/, not deeds); the variable @Anyone@ is reserved.
conceal :: String -> Int -> Want
conceal event k
  | any isVariable (pathNames event) =
      error ("conceal: event " ++ show event
             ++ " must be variable-free (a concealment want quantifies over"
             ++ " observers, not deeds)")
  | otherwise = Want [ Absent [ Match (beliefAbout "Anyone" event) ] ] k

-- | An action: assert an event you have no evidence for, to a co-present
-- hearer. The pattern's __first__ variable is the fabricated subject, bound by
-- the world-supplied fabrication conditions (whom you could frame); framing
-- yourself is excluded (that would be a confession, not a lie). Hearer gates
-- are gossip's: never the subject, never an eyewitness, one-shot per hearer,
-- plus the world's own gate.
lie :: CoPresence   -- ^ who can be told
    -> [Condition]  -- ^ the world's gate (may be @[]@)
    -> [Condition]  -- ^ fabrication: binds the pattern's variables
    -> String       -- ^ event pattern, e.g. @"stole.Culprit.loaf"@
    -> String       -- ^ action label
    -> Action
lie copresence gate fabrication pat label
  | (v : _) <- offenders =
      error ("Prax.Deceit.lie: " ++ show v ++ " in an authored gate, fabrication,"
             ++ " or event pattern -- the Prax namespace is reserved for machinery")
  | otherwise =
      action label conds
        [ Insert (beliefAbout "Hearer" pat ++ ".heard.Actor")
          -- the liar's own memory of the deed — a mark on their psyche, rooted
          -- under their name like all mental state; there is no world ledger
        , Insert ("Actor.lied.Hearer." ++ pat) ]
  where
    offenders = authoredVarClash [] (gate ++ fabrication) [] ++ authoredPatClash [] (pathNames pat)
    subject = case filter isVariable (pathNames pat) of
      (v : _) -> v
      []      -> error ("lie: event pattern " ++ show pat
                        ++ " names no one (a lie is about someone)")
    conds =
      fabrication
      ++ [ Neq subject "Actor"
         , Absent [ Match (beliefAbout "Actor" pat) ] ]   -- no evidence: what makes it a lie
      ++ asRole "Hearer" copresence
      ++ [ Neq "Hearer" "Actor"
         , Neq "Hearer" subject
         , Absent [ Match (beliefAbout "Hearer" pat ++ ".seen") ]
         , Not (beliefAbout "Hearer" pat ++ ".heard.Actor") ]
      ++ gate
