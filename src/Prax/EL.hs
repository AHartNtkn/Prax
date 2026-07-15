-- | Exclusion Logic's lattice, from Evans' DEON 2010 paper (§1.5–1.6; distilled in
-- @docs/research/deon-notes.md@). This is the faithful formal core the derivation
-- layer ("Prax.Derive") builds on, and the substrate a future static type checker
-- (LEDGER #8) would reuse.
--
-- The paper interprets expressions in /labeled rooted trees/ (LRTs, Def 2): a tree
-- whose edges are @!@ ("only child" — exclusion) or @*@ ("one of many" — multi).
-- Our 'Prax.Db.Db' /is/ exactly such a tree — each node carries an exclusion flag
-- ('Prax.Db.dbExcl') — so the lattice operates on it directly, with nothing lost.
--
--   * 'meet' — the greatest lower bound @⊓@ (Def 8), i.e. conjunction of models;
--     @Nothing@ is the paper's @⊥@ (Def 7 incompatibility): at a node exclusive in
--     either operand, the two disagree on the child. This detects a contradiction
--     /exactly/, from either side of the clash.
--   * 'leq' — the information order @≤@ (Def 6): @a `leq` b@ iff @a@ carries at
--     least as much information as @b@ (all of @b@'s structure, edge labels at
--     least as specific, @Excl ≤ Multi@); i.e. @a@ entails @b@.
--
-- The join @⊔@ (Def 9) is only needed for general entailment testing (#8) and is
-- deliberately omitted.
module Prax.EL
  ( meet
  , leq
  ) where

import           Control.Monad (foldM)
import qualified Data.IntMap.Strict as IntMap

import           Prax.Db (Db (..))

-- | Greatest lower bound @⊓@ (Def 8) — the conjunction of two models. @Nothing@ is
-- the paper's @⊥@: at some node exclusive in /either/ operand, the two disagree on
-- the child (Def 7 incompatibility). Otherwise children are merged (recursively
-- meeting shared subtrees) and a node is exclusive if either operand marks it so
-- (the more specific label wins, @Excl ≤ Multi@).
--
-- Assertedness (spec @docs/specs/2026-07-15-v39-asserted-endpoints.md@) extends
-- pointwise as __disjunction__: a node is asserted in the meet iff asserted in
-- /either/ operand, because meet conjoins the facts of both models and a path
-- asserted in either is a fact of the conjunction. This is the choice that keeps
-- the meet a lower bound — @meet a b `leq` a@ — of an asserted operand (see
-- 'leq'); conjunction here would drop the mark and break that law.
meet :: Db -> Db -> Maybe Db
meet (Db e1 a1 k1) (Db e2 a2 k2) = do
  merged <- foldM ins k1 (IntMap.toList k2)
  let e = e1 || e2
      a = a1 || a2
  if e && IntMap.size merged > 1
    then Nothing                          -- exclusive node forced to two children ⇒ ⊥
    else Just (Db e a merged)
  where
    ins acc (k, v2) = case IntMap.lookup k acc of
      Nothing -> Just (IntMap.insert k v2 acc)
      Just v1 -> do v <- meet v1 v2; Just (IntMap.insert k v acc)

-- | The information order @≤@ (Def 6): @a `leq` b@ iff @a@ has every edge of @b@,
-- with labels at least as specific (@Excl ≤ Multi@), recursively — i.e. @a@
-- entails @b@.
--
-- Assertedness enters exactly as the exclusion label does: if @b@ asserts a node
-- as a fact, @a@ must assert it too (@aa || not ab@) — asserting a path is
-- strictly more information than merely scaffolding it, so an asserted fact
-- entails its unasserted scaffold but not conversely (mirroring @Excl ≤ Multi@,
-- @Multi ⋠ Excl@).
leq :: Db -> Db -> Bool
leq (Db ea aa ka) (Db eb ab kb) =
  (ea || not eb) && (aa || not ab) && all present (IntMap.toList kb)
  where
    present (k, bChild) = case IntMap.lookup k ka of
      Nothing     -> False
      Just aChild -> leq aChild bChild
