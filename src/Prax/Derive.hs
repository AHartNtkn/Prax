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
  , closureFrom
  , derived
  , contradiction
  , axiomFootprint
  , axiomNegPatterns
  , monotoneAxioms
  ) where

import           Control.Monad (foldM)
import           Data.List (nub)
import           Data.Maybe (mapMaybe)
import qualified Data.Map.Strict as Map

import           Prax.Db (Db, insertToks, insertAll, emptyDb, tokens, groundTokens, tokensToSentence, dbToSentences)
import           Prax.Query (Condition (..), CmpOp (..), query)
import           Prax.EL (meet, leq)

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
closure axs db0 = run axs db0 db0

-- | Continue an ALREADY-CLOSED model with new base facts. Exactly
-- 'closure'’s semi-naive loop, entered at (model ∪ facts, delta = facts).
-- Sound only when the facts are monotone for these axioms — '!'-free and
-- unifying no negated body pattern, with 'monotoneAxioms' true — which is
-- the CALLER's obligation ('Prax.Engine.monotoneInsert'); a violation is
-- caught by the ViewInvariant net, not silently absorbed.
closureFrom :: [Axiom] -> Db -> [String] -> Either Contradiction Db
closureFrom axs closed facts =
  run axs (insertAll facts closed) (insertAll facts emptyDb)

-- The shared semi-naive engine (the former closure-local 'go', verbatim,
-- with 'rules' computed from the axiom list).
run :: [Axiom] -> Db -> Db -> Either Contradiction Db
run axs = go
  where
    rules = [ (body, map tokens hs)
            | Axiom body hs <- axs ++ mapMaybe liftObliged axs ]

    -- Semi-naive evaluation: each round fires the rules using at least one fact
    -- from @delta@ (the facts derived last round), so nothing already known is
    -- re-derived. @delta@ starts as the whole base, then shrinks to just what is
    -- new. Terminates when a round produces no fresh fact.
    go model delta =
      -- 'groundTokens' never re-splits a bound value on ./! (see its haddock
      -- invariant in Prax.Db) — relies on axiom heads binding only
      -- separator-free values, which holds for every unify-produced binding.
      let heads = [ groundTokens h b | (body, hs) <- rules
                                     , b <- deltaJoin model delta body, h <- hs ]
          fresh = nub (filter (not . entailed model) heads)
      in if null fresh
           then Right model
           else case foldM meetOne model fresh of
                  Left c  -> Left c
                  Right m -> go m (foldl (flip insertToks) emptyDb fresh)

    meetOne m h = maybe (Left (Contradiction (tokensToSentence h))) Right
                        (meet m (insertToks h emptyDb))
    entailed m h = leq m (insertToks h emptyDb)

    -- Bindings of @body@ in which at least one 'Match' atom is satisfied by a
    -- @delta@ fact (the others by the full model). With @delta@ = the base this is
    -- a full query; thereafter it visits only the newly-relevant joins.
    deltaJoin model delta body =
      case [ i | (i, Match _) <- zip [0 :: Int ..] body ] of
        []  -> query model body Map.empty        -- no positive atom: fire on the model
        pos -> nub (concatMap joinAt pos)
      where
        joinAt i = foldl step [Map.empty] (zip [0 :: Int ..] body)
          where step bs (j, c) = concatMap (query (if j == i then delta else model) [c]) bs

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

-- | Every path pattern the axioms can read or write: body atoms at any
-- polarity (including inside Absent\/Exists\/Or\/Subquery), head templates,
-- and the □-lifted forms of both. A ground delta that may-unify none of
-- these commutes with 'closure' (v27 spec theorem) — the basis of the
-- engine's delta-irrelevance fast path.
axiomFootprint :: [Axiom] -> [String]
axiomFootprint axs =
  concat [ concatMap condPatterns body ++ hs
         | Axiom body hs <- axs ++ mapMaybe liftObliged axs ]

-- All path patterns a condition mentions, any polarity.
condPatterns :: Condition -> [String]
condPatterns c = case c of
  Match s        -> [s]
  Not s          -> [s]
  Absent cs      -> concatMap condPatterns cs
  Exists cs      -> concatMap condPatterns cs
  Or clauses     -> concatMap (concatMap condPatterns) clauses
  Subquery _ _ w -> concatMap condPatterns w
  Eq {}          -> []
  Neq {}         -> []
  Cmp {}         -> []
  Calc {}        -> []
  Count {}       -> []

-- | Every pattern under a negation in any body: inserting a fact these
-- patterns match can UN-fire a rule (retraction), so such facts never take
-- the continuation tier.
axiomNegPatterns :: [Axiom] -> [String]
axiomNegPatterns axs = concat
  [ concatMap negOf body | Axiom body _ <- axs ++ mapMaybe liftObliged axs ]
  where
    negOf c = case c of
      Not s          -> [s]
      Absent cs      -> concatMap condPatterns cs   -- everything inside a ¬∃
      Exists cs      -> concatMap negOf cs
      Or clauses     -> concatMap (concatMap negOf) clauses
      Subquery _ _ w -> concatMap negOf w
      _              -> []

-- | Is the axiom set continuation-safe: does adding base facts only ever ADD
-- derived facts (given the caller also avoids negated patterns)? Conditions
-- must be monotone-up: Match/Not/Absent (negations are handled via
-- 'axiomNegPatterns'), recursion through Exists/Or/Subquery, Count freely,
-- Cmp only in the grows-only direction — the count side growing past a
-- numeric literal (Gt/Gte with the literal right, Lt/Lte with it left) — and
-- Eq/Neq only over pattern-bound variables. An Eq/Neq over an
-- aggregate-bound variable (a 'Count' result or a 'Subquery' set variable)
-- expresses exactly-k/not-k, which UN-fires as the aggregate grows past k —
-- anti-monotone despite Eq/Neq otherwise being a safe equality test. Calc
-- (and any other Cmp shape) disables the tier for the world; the fallback is
-- today's full reclose, correct just slower.
monotoneAxioms :: [Axiom] -> Bool
monotoneAxioms axs =
  all bodyOk [ body | Axiom body _ <- axs ++ mapMaybe liftObliged axs ]
  where
    bodyOk body = all (condOk (aggVars body)) body

    -- Every variable bound by an aggregate anywhere in the body (a body
    -- shares one binding environment, so a 'Count'/'Subquery' result nested
    -- under 'Exists'/'Or'/'Subquery' is still visible to an Eq/Neq
    -- elsewhere in the body).
    aggVars = concatMap collect
      where
        collect c = case c of
          Count r _      -> [r]
          Subquery s _ w -> s : aggVars w
          Exists cs      -> aggVars cs
          Or clauses     -> concatMap aggVars clauses
          _              -> []

    condOk aggs c = case c of
      Match _        -> True
      Not _          -> True
      Absent _       -> True
      Eq l r         -> l `notElem` aggs && r `notElem` aggs
      Neq l r        -> l `notElem` aggs && r `notElem` aggs
      Count _ _      -> True
      Exists cs      -> all (condOk aggs) cs
      Or clauses     -> all (all (condOk aggs)) clauses
      Subquery _ _ w -> all (condOk aggs) w
      Cmp op l r     -> case op of
        Gt  -> numeric r
        Gte -> numeric r
        Lt  -> numeric l
        Lte -> numeric l
      Calc {}        -> False
    numeric x = not (null x) && all (`elem` ("0123456789" :: String)) x
