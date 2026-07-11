-- | Which desires can authored action even in principle improve? Computed once
-- per world from the vocabulary (spec
-- @docs/specs/2026-07-11-v26-planner-work.md@ §2) and consulted by the planner
-- to skip predictions that are provably fruitless: a believed model none of
-- whose desires any available action can improve admits no motivated move.
--
-- The analysis is __conservative by construction__: it may only ever answer
-- "not improvable" when that is provable from the authored patterns. Anything
-- uncertain — outcomes behind unresolvable 'Call's, wants over facts an axiom
-- may derive, wants gated by 'Subquery'\/'Count'\/'Calc' — counts as
-- improvable. An unsound "not improvable" is a planner behavior change and a
-- defect; a spurious "improvable" merely costs the evaluation we would have
-- done anyway.
module Prax.Relevance
  ( mayUnify
  , improvableDesires
  ) where

import           Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map

import           Prax.Db (isVariable, pathNames)
import           Prax.Derive (Axiom (..))
import           Prax.Query (Condition (..))
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
-- against someone else's "lied"). Requiring an anchor only ever removes
-- coincidental, evidence-free matches; every genuine correspondence between
-- an authored effect and a want (Match's own literal path segments) shares
-- at least one literal, so this never introduces a false negative.
mayUnify :: String -> String -> Bool
mayUnify a b = anchored && and (zipWith seg (pathNames a) (pathNames b))
  where
    seg x y = isVariable x || isVariable y || x == y
    anchored = or (zipWith literalMatch (pathNames a) (pathNames b))
    literalMatch x y = not (isVariable x) && not (isVariable y) && x == y

-- The insert- and delete-shaped atoms an outcome can produce, resolving
-- 'Call's through the worlds' declared functions (conservatively: all cases).
-- An @!@ path both asserts its value and evicts siblings, so it counts on
-- both sides. Returns Nothing for "unknown effects" (unresolvable Call):
-- the caller must treat that as improves-everything.
outcomeAtoms :: Map String [Outcome] -> [String] -> Outcome
             -> Maybe ([String], [String])
outcomeAtoms fns visited o = case o of
  Insert s | '!' `elem` s -> Just ([s], [s])
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

-- | The names of the desires some authored action might improve. See the
-- module header for the conservativity contract.
improvableDesires :: Map String Practice -> [Axiom] -> [Desire] -> [String]
improvableDesires defs axs ds =
  [ desireName d | d <- ds, improvable d ]
  where
    practices = Map.elems defs
    fns = Map.fromList [ (fnName f, concatMap caseOutcomes (fnCases f))
                       | p <- practices, f <- functions p ]
    -- every effect an authored action can cause: its declared outcomes, plus
    -- the initOutcomes of any practice (spawning runs them)
    atoms = [ outcomeAtoms fns [] o
            | p <- practices, a <- actions p, o <- actionOutcomes a ]
         ++ [ outcomeAtoms fns [] o | p <- practices, o <- initOutcomes p ]
    wild = Nothing `elem` atoms
    inserted = concatMap (maybe [] fst) atoms
    deleted  = concatMap (maybe [] snd) atoms
    -- axiom heads, including their auto-□-lifted forms, count as derivable:
    -- a want over a derivable pattern is conservatively improvable.
    heads = concatMap axiomThen axs
    liftedHeads = [ "obliged.W." ++ h | h <- heads ]
    derivable p = any (mayUnify p) (heads ++ liftedHeads)
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
