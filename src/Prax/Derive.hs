-- | A forward-chaining derivation layer: domain knowledge as implication rules
-- @body → head@, closed to a fixpoint over the world (LEDGER #17, plus
-- entailment-closure for obligations — DEON property 1).
--
-- This is the paper's canonical-model construction @m(G,A)@ (Def 16–18): repeatedly
-- apply each implication to the model until it stops growing. The crucial part is
-- that facts are combined with the exclusion-logic 'Prax.EL.meet' (greatest lower
-- bound), so a rule that would force an exclusive slot to two values yields the
-- paper's @⊥@ — a __detected contradiction__, never a silent overwrite (the trap
-- the earlier naive spike fell into).
--
-- Two deliberate design choices:
--
--   * __Closure is a view.__ 'closure' takes a base 'Db' and returns the /closed/
--     one; it never mutates the base. Callers keep the base as the source of truth
--     and recompute — so a conclusion whose premise is retracted simply disappears
--     (defeasibility for free), which a churning sandbox needs.
--   * __Relational, not merely propositional.__ The paper's antecedent test @X ≤ A@
--     is generalized to /querying/ the body with "Prax.Query" so rules bind
--     variables (@parent.X.Y ∧ parent.Y.Z → grandparent.X.Z@). For a ground body
--     this coincides with @≤@; with variables it is the natural generalization.
--
-- __Auto-@□@-lifting__: every domain rule @A → B@ additionally contributes the
-- lifted rule @obliged.W.A → obliged.W.B@, so an obligation closes over the
-- consequences of its content (DEON property 1) with the rule written once.
module Prax.Derive
  ( Axiom(..)
  , axiom
  , Contradiction(..)
  , closure
  , derived
  , contradiction
  ) where

import           Control.Monad (foldM)
import           Data.Maybe (mapMaybe)
import qualified Data.Map.Strict as Map

import           Prax.Db (Db, insert, emptyDb, ground, dbToSentences)
import           Prax.Query (Condition (..), query)
import           Prax.EL (meet)

-- | An implication rule @axiomWhen → axiomThen@: when the body holds for some
-- binding of its variables, assert each (grounded) head sentence. Heads are
-- sentence /templates/ that keep their @!@/@.@ labels (so exclusion is honoured
-- when they are 'Prax.EL.meet'-ed into the model).
data Axiom = Axiom
  { axiomWhen :: [Condition]
  , axiomThen :: [String]
  }
  deriving (Eq, Show)

-- | @axiom body heads@.
axiom :: [Condition] -> [String] -> Axiom
axiom = Axiom

-- | A detected contradiction (@⊥@): the head sentence whose assertion was
-- incompatible with the model.
newtype Contradiction = Contradiction String
  deriving (Eq, Show)

-- | Close a world under a set of axioms: build the exclusion-logic model, apply
-- the rules (and their @□@-lifted forms) to a fixpoint, and project back to a
-- 'Db'. @Left@ reports the first contradiction. With no axioms the base is
-- returned unchanged (the identity that keeps un-axiomatised worlds free).
closure :: [Axiom] -> Db -> Either Contradiction Db
closure []  db0 = Right db0
closure axs db0 = fixpoint db0
  where
    rules = axs ++ mapMaybe liftObliged axs

    fixpoint m = case stepAll m of
      Left c -> Left c
      Right m'
        | m' == m   -> Right m'      -- fixpoint (compares labels too, so it is exact)
        | otherwise -> fixpoint m'

    -- one round: query every rule body against the round-start model, meet each
    -- grounded head into the growing model (⊥ if it clashes on an exclusive slot)
    stepAll m = foldM (fire m) m rules
    fire qdb m (Axiom body heads) =
      foldM meetOne m [ ground h b | b <- query qdb body Map.empty, h <- heads ]
    meetOne m h = maybe (Left (Contradiction h)) Right (meet m (insert h emptyDb))

-- Lift a purely-conjunctive domain rule under the obligation operator: prefix
-- @obliged.\<fresh\>.@ to every body match and head, so □A ⊢ □B whenever A ⊢ B.
-- Rules whose body uses non-'Match' conditions are not lifted (nothing sensible
-- to place under □).
liftObliged :: Axiom -> Maybe Axiom
liftObliged (Axiom body heads)
  | all isMatch body = Just (Axiom (map liftCond body) (map liftSent heads))
  | otherwise        = Nothing
  where
    isMatch (Match _) = True
    isMatch _         = False
    liftCond (Match s) = Match (liftSent s)
    liftCond c         = c
    liftSent s = "obliged.Obligor." ++ s

-- | The facts the axioms /add/ to a world (closure minus base). Empty on
-- contradiction.
derived :: [Axiom] -> Db -> [String]
derived axs db = case closure axs db of
  Right closed -> filter (`notElem` dbToSentences db) (dbToSentences closed)
  Left _       -> []

-- | The contradiction a world's axioms produce, if any (the @⊥@ witness).
contradiction :: [Axiom] -> Db -> Maybe String
contradiction axs db = case closure axs db of
  Left (Contradiction h) -> Just h
  Right _                -> Nothing
