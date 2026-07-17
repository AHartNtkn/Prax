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
-- __Auto-@□@-lifting__: every all-@Match@ domain rule @A → B@ can additionally
-- contribute the lifted rule @obliged.Obligor.A → obliged.Obligor.B@, so an
-- obligation closes over the consequences of its content (DEON property 1) with
-- the rule written once. In the STRING path ('closure'\/'run') this lifting is
-- unconditional (there is no producer pool to consult and its unconditional
-- form is "Prax.ViewInvariantSpec"'s reference); in the cooked\/engine path
-- ('cookAxioms', driven by 'Prax.Engine.retable') it is GATED on the world's
-- ability to produce an @obliged.*@ fact (spec v48) — a world that can never
-- invoke an obligation carries no unfireable lifted rules.
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
  , axiomHeadPatterns
  , monotoneAxioms
  , CookedRule(..)
  , cookAxioms
  , obligedHead
  , runCooked
  ) where

import           Control.Monad (foldM)
import           Data.List (nub)
import           Data.Maybe (mapMaybe)
import qualified Data.Map.Strict as Map

import           Prax.Db (Db, insertToks, insertAll, emptyDb, internTokens, groundTokens, tokensToSentence, dbToSentences)
import           Prax.Query (Condition (..), CmpOp (..), query, CookedCondition (..), cookCondition, cookedReadAnchors, queryCooked)
import           Prax.EL (meet, leq)
import           Prax.Sym (Sym, symName)

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

-- Shared by 'run' and 'runCooked': does @m@ already entail head tokens @h@,
-- and meet a fresh head into the model (⊥ iff incompatible).
entailed :: Db -> [(Sym, Maybe Char)] -> Bool
entailed m h = leq m (insertToks h emptyDb)

meetOne :: Db -> [(Sym, Maybe Char)] -> Either Contradiction Db
meetOne m h = maybe (Left (Contradiction (tokensToSentence h))) Right
                    (meet m (insertToks h emptyDb))

-- The shared semi-naive engine (the former closure-local 'go', verbatim,
-- with 'rules' computed from the axiom list).
run :: [Axiom] -> Db -> Db -> Either Contradiction Db
run axs = go
  where
    rules = [ (body, map internTokens hs)
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

-- | An axiom's body and heads, precompiled once per world (see 'cookAxioms'):
-- the body pattern-split ('Prax.Query.cookCondition') and the head sentences
-- pre-tokenized and interned ('Prax.Db.internTokens') — the same compilation
-- 'run'\'s local @rules@ otherwise redoes on every call.
data CookedRule = CookedRule
  { crBody  :: [CookedCondition]
  , crHeads :: [[(Sym, Maybe Char)]]
  }
  deriving (Eq, Show)

-- | Precompile a domain's axioms to 'CookedRule's. @lift@ decides whether the
-- auto-□-lifted forms (see 'liftObliged') are included (spec v48): the engine
-- lifts exactly when the world can produce an @obliged.*@ fact — DEON property
-- 1 for worlds that can invoke it, no doubled rule set for worlds that cannot.
-- The DECISION lives in the caller ('Prax.Engine.retable', via
-- 'Prax.Relevance.deonticProducible'); the MECHANISM is here. The STRING path
-- ('closure'\/'run') always lifts and is deliberately ungated: it has no
-- producer pool, and its unconditional lifting makes "Prax.ViewInvariantSpec"
-- the gate's soundness net — if the gate ever wrongly skips a producible world,
-- the gated cooked view diverges from the ungated reference and the net fires.
--
-- The caller stores the result in 'Prax.Types.cookedRules', so 'runCooked' —
-- invoked once per 'Prax.Engine.reclose'\/'Prax.Engine.applyGrowToks',
-- thousands of times a round — never re-cooks the axiom set.
cookAxioms :: Bool -> [Axiom] -> [CookedRule]
cookAxioms lift axs =
  [ CookedRule (map cookCondition body) (map internTokens hs)
  | Axiom body hs <- axs ++ (if lift then mapMaybe liftObliged axs else []) ]

-- | The obligation operator's head literal: the first segment of every
-- □-lifted fact ('liftObliged' prefixes @obliged.Obligor.@) and the name the
-- □-lift gate ('Prax.Relevance.deonticProducible') screens producers for. One
-- home for the vocabulary coupling that 'liftObliged' is inherently built on.
obligedHead :: String
obligedHead = "obliged"

-- | 'run'\'s cooked mirror: case-for-case the same semi-naive loop
-- (@go@\/@deltaJoin@), typed over precompiled 'CookedRule's and
-- 'Prax.Query.queryCooked' instead of re-deriving @rules@ from @axs@ and
-- re-splitting each body pattern via 'Prax.Query.query' on every call. A
-- deliberately independent, hand-verified parallel implementation — not a
-- delegation through 'run' — so 'closure'\/'closureFrom' (which still call
-- 'run') remain a genuinely separate code path from the engine's hot loop,
-- exactly the property "Prax.ViewInvariantSpec"'s net depends on to recompute
-- against something other than the code under test.
runCooked :: [CookedRule] -> Db -> Db -> Either Contradiction Db
runCooked rules = go
  where
    go model delta =
      let heads = [ groundTokens h b | CookedRule body hs <- rules
                                      , b <- deltaJoinCooked model delta body, h <- hs ]
          fresh = nub (filter (not . entailed model) heads)
      in if null fresh
           then Right model
           else case foldM meetOne model fresh of
                  Left c  -> Left c
                  Right m -> go m (foldl (flip insertToks) emptyDb fresh)

    deltaJoinCooked model delta body =
      case [ i | (i, CMatch _) <- zip [0 :: Int ..] body ] of
        []  -> queryCooked model body Map.empty
        pos -> nub (concatMap joinAt pos)
      where
        joinAt i = foldl step [Map.empty] (zip [0 :: Int ..] body)
          where step bs (j, c) = concatMap (queryCooked (if j == i then delta else model) [c]) bs

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
    liftSent s = obligedHead ++ ".Obligor." ++ s

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
-- polarity (including inside Absent\/Exists\/Or\/Subquery — the
-- 'Prax.Query.cookedReadAnchors' walk), and head templates. Whatever □-lifted
-- forms the gate admitted are already rules in their own right ('cookAxioms'),
-- so lifting needs no second enumeration here. A ground delta that may-unify none of these
-- commutes with 'closure' (v27 spec theorem) — the basis of the engine's
-- delta-irrelevance fast path.
axiomFootprint :: [CookedRule] -> [[Sym]]
axiomFootprint rules =
  concat [ cookedReadAnchors (crBody r) ++ map (map fst) (crHeads r) | r <- rules ]

-- | Every pattern under a negation in any body: inserting a fact these
-- patterns match can UN-fire a rule (retraction), so such facts never take
-- the continuation tier.
axiomNegPatterns :: [CookedRule] -> [[Sym]]
axiomNegPatterns rules = concat [ concatMap negOf (crBody r) | r <- rules ]
  where
    negOf c = case c of
      CNot p          -> [p]
      CAbsent cs      -> cookedReadAnchors cs   -- everything inside a ¬∃
      CExists cs      -> concatMap negOf cs
      COr clauses     -> concatMap (concatMap negOf) clauses
      CSubquery _ _ w -> concatMap negOf w
      _               -> []

-- | Every head template the axioms can write — whatever □-lifted forms the
-- gate admitted included, for free, since 'cookAxioms' already emitted them as
-- rules of their own. A delta that feeds some axiom can change derived facts
-- only in these families.
axiomHeadPatterns :: [CookedRule] -> [[Sym]]
axiomHeadPatterns rules = concat [ map (map fst) (crHeads r) | r <- rules ]

-- | Is the axiom set continuation-safe: does adding base facts only ever ADD
-- derived facts (given the caller also avoids negated patterns)? Conditions
-- must be monotone-up: CMatch/CNot/CAbsent (negations are handled via
-- 'axiomNegPatterns'), recursion through CExists/COr/CSubquery, CCount
-- freely, CCmp only in the grows-only direction — the count side growing
-- past a numeric literal (Gt/Gte with the literal right, Lt/Lte with it
-- left) — and CEq/CNeq only over pattern-bound variables. An Eq/Neq over an
-- aggregate-bound variable (a 'CCount' result or a 'CSubquery' set variable)
-- expresses exactly-k/not-k, which UN-fires as the aggregate grows past k —
-- anti-monotone despite Eq/Neq otherwise being a safe equality test. CCalc
-- (and any other CCmp shape) disables the tier for the world; the fallback
-- is today's full reclose, correct just slower.
monotoneAxioms :: [CookedRule] -> Bool
monotoneAxioms = all (bodyOk . crBody)
  where
    bodyOk body = all (condOk (aggVars body)) body

    -- Every variable bound by an aggregate anywhere in the body (a body
    -- shares one binding environment, so a CCount/CSubquery result nested
    -- under CExists/COr/CSubquery is still visible to an Eq/Neq elsewhere
    -- in the body).
    aggVars = concatMap collect
      where
        collect c = case c of
          CCount r _      -> [r]
          CSubquery s _ w -> s : aggVars w
          CExists cs      -> aggVars cs
          COr clauses     -> concatMap aggVars clauses
          _               -> []

    condOk aggs c = case c of
      CMatch _        -> True
      CNot _          -> True
      CAbsent _       -> True
      CEq l r         -> l `notElem` aggs && r `notElem` aggs
      CNeq l r        -> l `notElem` aggs && r `notElem` aggs
      CCount _ _      -> True
      CExists cs      -> all (condOk aggs) cs
      COr clauses     -> all (all (condOk aggs)) clauses
      CSubquery _ _ w -> all (condOk aggs) w
      CCmp op l r     -> case op of
        Gt  -> numeric r
        Gte -> numeric r
        Lt  -> numeric l
        Lte -> numeric l
      CCalc {}        -> False
    numeric x = let s = symName x
                in not (null s) && all (`elem` ("0123456789" :: String)) s
