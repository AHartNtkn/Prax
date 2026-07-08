-- | A static well-formedness checker for a world (LEDGER #8, first cut).
--
-- Versu ships a type checker to "find errors early" (Evans & Short 2014, §VII-E):
-- it is strongly but /implicitly/ typed — the author declares nothing and the
-- system complains when a consistent assignment cannot be found. This module is
-- the __sound, declaration-free subset__ of that: three static checks over every
-- sentence a world authors ('practiceDefs' + 'axioms' + facts), each of which
-- flags only unambiguous bugs (no false positives, so the report is trustworthy).
-- It adds no logic engine — just a pass over the existing sentence structure.
--
--   * __Unbound variables__ — a variable used in an outcome (or an axiom head) that
--     no precondition, role, or @Actor@ can bind is ungroundable: it silently
--     inserts a literal @\"X\"@ or a no-op. (The most common real bug.)
--   * __Exclusion-cardinality clashes__ — a relation used both single-valued (@!@)
--     and multi-valued (@.@); the paper's exclusion-information check, done
--     statically.
--   * __Dangling references__ — a @Call@ to an undefined function, or a spawn of an
--     undefined practice.
--
-- Full ML-style /sort/ inference (agent-vs-gender) needs a declaration layer and
-- is the documented next step; this checker does not attempt it.
module Prax.TypeCheck
  ( TypeError(..)
  , typeCheck
  ) where

import           Data.List (intercalate, nub)
import qualified Data.Map.Strict as Map

import           Prax.Db (isVariable, pathNames, tokens, dbToLabeledSentences)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Derive (Axiom (..))

-- | A well-formedness problem found in a world.
data TypeError
  = UnboundVar { teWhere :: String, teVar :: String, teSentence :: String }
    -- ^ @teVar@ in @teSentence@ (at @teWhere@) is bound by nothing.
  | CardinalityClash { teSlot :: String }
    -- ^ the (variable-normalized) slot @teSlot@ is used both @!@ and @.@.
  | UndefinedRef { teWhere :: String, teName :: String }
    -- ^ a @Call@/spawn (at @teWhere@) names something never defined.
  deriving (Eq, Show)

-- | Every well-formedness problem in a world (empty ⇒ the world is well-formed).
typeCheck :: PraxState -> [TypeError]
typeCheck st =
     concatMap unboundInPractice ps
  ++ concatMap unboundInAxiom (axioms st)
  ++ cardinalityErrors (assertedSentences st)
  ++ refErrors st
  where ps = Map.elems (practiceDefs st)

-- Variables mentioned in a sentence / condition -------------------------------

varsOf :: String -> [String]
varsOf = filter isVariable . pathNames

-- Every variable a condition /mentions/ (an over-approximation of what it can
-- bind — sound for the unbound check: a var mentioned by no condition truly
-- cannot be bound).
condVars :: Condition -> [String]
condVars c = case c of
  Match s          -> varsOf s
  Not s            -> varsOf s
  Eq a b           -> vs [a, b]
  Neq a b          -> vs [a, b]
  Cmp _ a b        -> vs [a, b]
  Calc r _ a b     -> vs [r, a, b]
  Count r s        -> vs [r, s]
  Subquery set f w -> vs (set : f) ++ concatMap condVars w
  Or clauses       -> concatMap (concatMap condVars) clauses
  Absent cs        -> concatMap condVars cs
  Exists cs        -> concatMap condVars cs
  where vs = filter isVariable

-- The variables an outcome /uses/, paired with the text they appear in.
outcomeUses :: Outcome -> [(String, String)]
outcomeUses (Insert s)     = [ (v, s) | v <- varsOf s ]
outcomeUses (Delete s)     = [ (v, s) | v <- varsOf s ]
outcomeUses (Call fn args) = [ (v, fn) | a <- args, v <- varsOf a ]

-- Check 1: unbound variables --------------------------------------------------

unboundInOutcomes :: String -> [String] -> [Outcome] -> [TypeError]
unboundInOutcomes loc bound outs =
  [ UnboundVar loc v s
  | o <- outs, (v, s) <- outcomeUses o, v `notElem` bound ]

unboundInPractice :: Practice -> [TypeError]
unboundInPractice p =
     unboundInOutcomes (practiceId p ++ " (init)") (roles p) (initOutcomes p)
  ++ concatMap action' (actions p)
  ++ concatMap fn' (functions p)
  where
    action' a =
      unboundInOutcomes (practiceId p ++ " / " ++ actionName a)
        ("Actor" : roles p ++ concatMap condVars (actionConditions a))
        (actionOutcomes a)
    fn' f =
      concatMap
        (\c -> unboundInOutcomes (practiceId p ++ " / fn " ++ fnName f)
                 (fnParams f ++ concatMap condVars (caseConditions c))
                 (caseOutcomes c))
        (fnCases f)

unboundInAxiom :: Axiom -> [TypeError]
unboundInAxiom ax =
  [ UnboundVar "axiom" v h | h <- axiomThen ax, v <- varsOf h, v `notElem` bound ]
  where bound = concatMap condVars (axiomWhen ax)

-- Check 2: exclusion-cardinality consistency ----------------------------------

-- Each edge of a sentence, keyed by the variable-normalized path to its parent,
-- paired with whether that edge is exclusive (@!@).
edgesOf :: String -> [(String, Bool)]
edgesOf s =
  [ (intercalate "." (take (i + 1) names), op == Just '!')
  | (i, (_, op)) <- zip [0 :: Int ..] ts, op /= Nothing ]
  where
    ts    = tokens s
    names = map (\(n, _) -> if isVariable n then "_" else n) ts

cardinalityErrors :: [String] -> [TypeError]
cardinalityErrors sentences =
  [ CardinalityClash slot
  | (slot, labels) <- Map.toList byslot, length (nub labels) > 1 ]
  where
    byslot = Map.fromListWith (++)
               [ (slot, [excl]) | s <- sentences, (slot, excl) <- edgesOf s ]

-- Check 3: dangling references ------------------------------------------------

refErrors :: PraxState -> [TypeError]
refErrors st = concatMap practiceRefs ps
  where
    ps          = Map.elems (practiceDefs st)
    definedFns  = [ fnName f | p <- ps, f <- functions p ]
    definedPrac = Map.keys (practiceDefs st)
    practiceRefs p = concatMap (outcomeRef (practiceId p)) (allOutcomes p)
    allOutcomes p =
      initOutcomes p ++ concatMap actionOutcomes (actions p)
      ++ [ o | f <- functions p, c <- fnCases f, o <- caseOutcomes c ]
    outcomeRef loc (Call fn _)
      | fn `notElem` definedFns = [ UndefinedRef loc fn ]
    outcomeRef loc (Insert s)
      | ("practice" : pid : _) <- pathNames s
      , pid `notElem` definedPrac = [ UndefinedRef loc ("practice." ++ pid) ]
    outcomeRef _ _ = []

-- The world's __asserting__ sentences, for the cardinality pass. Only inserts,
-- static data facts, axiom heads, and live facts /assert/ a slot's cardinality;
-- a query condition's @!@/@.@ is mere path syntax (unify ignores it), so
-- conditions are deliberately excluded — otherwise a legitimate @Match "ending.E"@
-- would falsely clash with an @Insert "ending!betrayal"@.
assertedSentences :: PraxState -> [String]
assertedSentences st =
     [ s | p <- ps
         , s <- dataFacts p
                ++ inserts (initOutcomes p)
                ++ concatMap (inserts . actionOutcomes) (actions p)
                ++ concatMap (\f -> concatMap (inserts . caseOutcomes) (fnCases f)) (functions p) ]
  ++ [ h | ax <- axioms st, h <- axiomThen ax ]
  ++ dbToLabeledSentences (db st)
  where
    ps = Map.elems (practiceDefs st)
    inserts os = [ s | Insert s <- os ]
