-- | Which desires can authored action even in principle improve? Computed once
-- per world from the vocabulary (spec
-- @docs/specs/2026-07-11-v26-planner-work.md@ §2) and consulted by the planner
-- to skip predictions that are provably fruitless: a believed model none of
-- whose desires any available action can improve admits no motivated move.
--
-- The analysis is __conservative__: it answers "not improvable" only when
-- that is provable from the authored patterns. Anything uncertain — outcomes
-- behind unresolvable 'Call's, wants over facts an axiom may derive, wants
-- gated by 'Subquery'\/'Count'\/'Calc' — counts as improvable. An unsound
-- "not improvable" is a planner behavior change and a defect; a spurious
-- "improvable" merely costs the evaluation we would have done anyway.
--
-- One stated invariant carries the conservativity (an assumption about
-- authored worlds, not a construction guarantee): __entity names never
-- collide with predicate-name literals__ — no character, place, or value is
-- named @lied@, @believes@, @regards@, and so on. 'mayUnify' spends it in
-- exactly one place (the anchor below). A world that named a character after
-- a predicate segment would void the analysis; every shipped world satisfies
-- the invariant, and the golden decision-sequence tests would surface a
-- violation as a dropped prediction.
module Prax.Relevance
  ( mayUnify
  , mayUnifySyms
  , improvableDesires
  , Liveness(..)
  , livenessOf
  , evictionShadows
  , evictionShadowNames
  ) where

import           Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map

import           Prax.Db (internTokens, pathNames, tokensToSentence)
import           Prax.Derive (Axiom (..))
import           Prax.Query (Condition (..), cookCondition)
import           Prax.Sym (Sym, intern, symIsVar)
import           Prax.Types

-- | Could a grounded instance of one path pattern be an instance (or a
-- prefix\/extension) of the other? Segments unify when either is a variable
-- or they are equal; length mismatch is prefix-compatible (a 'Match' sees
-- subtrees). A pair unifies only if some overlapping segment is a shared
-- /literal/ (both sides constant and equal) — Prax facts are identified by
-- their literal predicate-name segments, so an overlap covered entirely by
-- variables carries no evidence the two patterns denote the same predicate
-- at all (any string could occupy a variable slot, including another
-- pattern's unrelated literal, e.g. a role variable coincidentally lining up
-- against someone else's "lied"). Discarding such evidence-free overlaps is
-- where the module header's entity-names-vs-predicate-literals invariant is
-- spent: under it, a genuine correspondence between an authored effect and a
-- want always shares an aligned literal, so the anchor removes only
-- coincidence.
mayUnify :: String -> String -> Bool
mayUnify a b = mayUnifySyms (map intern (pathNames a)) (map intern (pathNames b))

-- | 'mayUnify' on pre-split, pre-interned paths — the planner-hot form
-- ('relevantDelta' classifies every primitive delta against every footprint
-- pattern). Variable-ness is the parity bit ('Prax.Sym.symIsVar'); a shared
-- literal segment is Int equality — the hottest classification in the
-- engine, now Int equality instead of String equality.
mayUnifySyms :: [Sym] -> [Sym] -> Bool
mayUnifySyms as bs = anchored && and (zipWith seg as bs)
  where
    seg x y = symIsVar x || symIsVar y || x == y
    anchored = or (zipWith literalMatch as bs)
    literalMatch x y = not (symIsVar x) && not (symIsVar y) && x == y

-- The insert- and delete-shaped atoms an outcome can produce, resolving
-- 'Call's through the worlds' declared functions (conservatively: all cases).
-- An @!@ path both asserts its value and evicts that value's __siblings__ —
-- whose names appear nowhere in the outcome — so the delete side carries the
-- path with every post-@!@ segment replaced by a fresh variable ('mayUnify'
-- wildcard: conservatively, any sibling). Returns Nothing for "unknown
-- effects" (unresolvable Call): the caller must treat that as
-- improves-everything.
outcomeAtoms :: Map String [Outcome] -> [String] -> Outcome
             -> Maybe ([String], [String])
outcomeAtoms fns visited o = case o of
  Insert s | '!' `elem` s -> Just ([s], s : evictionShadows s)
           | otherwise    -> Just ([s], [])
  Delete s                -> Just ([], [s])
  ForEach _ outs          -> mconcat' (map (outcomeAtoms fns visited) outs)
  Call fn _
    | fn `elem` visited   -> Just ([], [])           -- cycle: already counted
    | otherwise -> case Map.lookup fn fns of
        Nothing   -> Nothing                         -- unknown function: wild
        Just outs -> mconcat' (map (outcomeAtoms fns (fn : visited)) outs)
  where
    mconcat' ms = do
      pairs <- sequence ms
      pure (concatMap fst pairs, concatMap snd pairs)

-- | The eviction shadows of an exclusion insert, computed directly on
-- already-split, interned tokens — the single implementation (both
-- 'evictionShadows' below and 'Prax.Engine.performCooked'\'s cooked hot path
-- go through this, so there is one name-shape, one implementation). One
-- shadow per @!@ operator: the names up to and including that point,
-- followed by a fresh @"Evicted"@ segment (interns as a variable — uppercase
-- initial — so 'mayUnifySyms' treats it as the wildcard it denotes). Each
-- exclusion clears the displaced sibling's entire subtree — arbitrary depth
-- and shape — and 'mayUnifySyms' compares only up to the shorter path, so
-- the truncated shadow covers every want under it.
evictionShadowNames :: [(Sym, Maybe Char)] -> [[Sym]]
evictionShadowNames toks =
  [ map fst (take i toks) ++ [intern "Evicted"]
  | (i, (_, op)) <- zip [1 ..] toks, op == Just '!' ]

-- | The eviction shadows of an exclusion insert, as sentences — 'mayUnify'
-- (via 'pathNames', which discards punctuation) is the only consumer, so
-- WHICH separator re-joins the names is immaterial; this is
-- 'evictionShadowNames' re-joined with 'tokensToSentence' using @.@ between
-- every pair (a token's op-flag is emitted AFTER it, so every name but the
-- last needs one to keep the names from gluing together).
evictionShadows :: String -> [String]
evictionShadows s =
  [ tokensToSentence (dotted ns) | ns <- evictionShadowNames (internTokens s) ]
  where
    dotted ns = zip ns (replicate (length ns - 1) (Just '.') ++ [Nothing])

-- Positive and negated path patterns of a want's conditions. The Bool is
-- "uncertain": the want's satisfaction depends on machinery (numeric binds,
-- counts, subqueries) beyond pattern presence.
wantPatterns :: [Condition] -> ([String], [String], Bool)
wantPatterns = foldr step ([], [], False)
  where
    step c (pos, neg, unc) = case c of
      Match p      -> (p : pos, neg, unc)
      Not p        -> (pos, p : neg, unc)
      Absent cs    -> let (p', n', u') = wantPatterns cs
                      in (pos ++ n', neg ++ p', unc || u')
      Exists cs    -> let (p', n', u') = wantPatterns cs
                      in (pos ++ p', neg ++ n', unc || u')
      Or clauses   -> let parts = map wantPatterns clauses
                      in ( pos ++ concatMap (\(p', _, _) -> p') parts
                         , neg ++ concatMap (\(_, n', _) -> n') parts
                         , unc || any (\(_, _, u') -> u') parts )
      Eq _ _       -> (pos, neg, unc)
      Neq _ _      -> (pos, neg, unc)
      Cmp {}       -> (pos, neg, unc)
      Calc {}      -> (pos, neg, True)
      Count {}     -> (pos, neg, True)
      Subquery {}  -> (pos, neg, True)

-- Every effect an authored action can cause, resolved once per world: the
-- insert- and delete-shaped atom pools ('outcomeAtoms' over every action's
-- declared outcomes, plus every practice's 'initOutcomes' — spawning runs
-- those too), and whether any of them is "wild" (an unresolvable 'Call',
-- conservatively improves-everything). Shared by 'improvableDesires' and
-- 'livenessOf' — one atom-pool computation, not two.
data AtomPools = AtomPools
  { poolInserted :: [String]
  , poolDeleted  :: [String]
  , poolWild     :: Bool
  }

worldAtomPools :: Map String Practice -> AtomPools
worldAtomPools defs = AtomPools
  { poolInserted = concatMap (maybe [] fst) atoms
  , poolDeleted  = concatMap (maybe [] snd) atoms
  , poolWild     = Nothing `elem` atoms
  }
  where
    practices = Map.elems defs
    fns = Map.fromList [ (fnName f, concatMap caseOutcomes (fnCases f))
                       | p <- practices, f <- functions p ]
    atoms = [ outcomeAtoms fns [] o
            | p <- practices, a <- actions p, o <- actionOutcomes a ]
         ++ [ outcomeAtoms fns [] o | p <- practices, o <- initOutcomes p ]

-- Axiom heads, including their auto-□-lifted forms, count as derivable: a
-- want (or gate candidate) over a derivable pattern is conservatively
-- improvable\/never a gate. Shared by 'improvableDesires' and 'livenessOf'.
axiomDerivable :: [Axiom] -> String -> Bool
axiomDerivable axs p = any (mayUnify p) (heads ++ liftedHeads)
  where
    heads = concatMap axiomThen axs
    liftedHeads = [ "obliged.W." ++ h | h <- heads ]

-- | The names of the desires some authored action might improve. See the
-- module header for the conservativity contract.
improvableDesires :: Map String Practice -> [Axiom] -> [Desire] -> [String]
improvableDesires defs axs ds =
  [ desireName d | d <- ds, improvable d ]
  where
    pools = worldAtomPools defs
    wild = poolWild pools
    inserted = poolInserted pools
    deleted  = poolDeleted pools
    derivable = axiomDerivable axs
    improvable (Desire _ (Want conds u))
      | u == 0    = False
      | wild      = True
      | unc       = True
      | any derivable (pos ++ neg) = True
      | u > 0     = any (\i -> any (mayUnify i) pos) inserted
                    || any (\dl -> any (mayUnify dl) neg) deleted
      | otherwise = any (\dl -> any (mayUnify dl) pos) deleted
                    || any (\i -> any (mayUnify i) neg) inserted
      where (pos, neg, unc) = wantPatterns conds

-- | Classify every named desire's dead-now recipe (see 'Liveness'): a
-- negative want-kind is unconditionally 'FloorCheck'; a positive want-kind
-- gates on its top-level positive @Match@ conjuncts that are neither
-- action-insertable nor axiom-derivable (each such conjunct, cooked, is one
-- gate — 'null' qualifying conjuncts, an unresolvable\/wild outcome, or a
-- @Subquery@\/@Count@\/@Calc@-tainted want all fall back to 'AlwaysLive');
-- weight 0 is statically never-improvable already and is mapped 'AlwaysLive'
-- defensively (the static filter, 'improvableDesires', screens it first).
livenessOf :: Map String Practice -> [Axiom] -> [Desire] -> Map String Liveness
livenessOf defs axs ds =
  Map.fromList [ (desireName d, classify d) | d <- ds ]
  where
    pools = worldAtomPools defs
    derivable = axiomDerivable axs
    classify (Desire _ (Want conds u))
      | u < 0     = FloorCheck
      | u > 0     = positive conds
      | otherwise = AlwaysLive
    positive conds
      | unc || poolWild pools || null gates = AlwaysLive
      | otherwise = GateCheck gates
      where
        (_, _, unc) = wantPatterns conds
        candidates = [ p | Match p <- conds ]
        qualifies p = not (any (mayUnify p) (poolInserted pools))
                    && not (derivable p)
        gates = [ [cookCondition (Match p)] | p <- candidates, qualifies p ]
