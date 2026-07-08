-- | Exclusion Logic's model + lattice, from Evans' DEON 2010 paper (§1.5–1.6;
-- distilled in @docs/research/deon-notes.md@). This is the faithful formal core
-- the derivation layer ("Prax.Derive") builds on, and the substrate a future
-- static type checker (LEDGER #8) would reuse.
--
-- The paper interprets expressions in /labeled rooted trees/ (LRTs, Def 2): a
-- tree whose edges are labeled @!@ ("only child" — exclusion) or @*@ ("one of
-- possibly many" — multi). We represent an LRT as a trie whose /node/ carries an
-- @excl@ flag for its outgoing edges (a valid LRT's exclusive node has exactly one
-- child, Def 3), which is isomorphic to the paper's edge-labeling and far simpler
-- to compute with. The trie path to a node is its /signature/ (Def 4).
--
-- The three operations the paper needs and we implement faithfully:
--
--   * 'meet' — the greatest lower bound @⊓@ (Def 8), i.e. conjunction of models;
--     @Nothing@ is the paper's @⊥@ (Def 7 incompatibility: two models agree on a
--     parent but diverge to different children under an exclusive edge). This is
--     where a contradiction is detected /exactly/, not silently resolved.
--   * 'leq' — the information order @≤@ (Def 6): @a ≤ b@ iff @a@ carries at least
--     as much information as @b@ (all of @b@'s structure, with edge labels at
--     least as specific, @Excl ≤ Multi@). @a ≤ b@ means @a@ entails @b@.
--   * 'fromSentences'/'toSentences' — build a model from labeled sentences
--     (parsing @a!b@ vs @a.b@) and read it back.
--
-- The join @⊔@ (Def 9) is only needed for general entailment testing (#8); it is
-- deliberately omitted here.
module Prax.EL
  ( LNode(..)
  , leaf
  , sentenceToNode
  , fromSentences
  , toSentences
  , meet
  , leq
  ) where

import           Control.Monad (foldM)
import           Data.List (sort)
import           Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map

-- | A node of a labeled rooted tree: @excl@ is 'True' when this node's outgoing
-- edges are exclusive (the paper's @!@ — a valid such node has one child); its
-- children are keyed by their vertex symbol.
data LNode = LNode
  { excl :: Bool
  , kids :: Map String LNode
  }
  deriving (Eq, Show)

-- | The empty model (root with no children).
leaf :: LNode
leaf = LNode False Map.empty

-- Parse a sentence into a path of @(symbol, edge-into-it-is-exclusive)@ pairs.
-- The first symbol's edge (from the root @T@) is non-exclusive by convention; a
-- @!@ before a symbol marks its incoming edge exclusive, a @.@ marks it multi.
parseLabeled :: String -> [(String, Bool)]
parseLabeled s =
  let (first, rest) = break isSep s
  in (first, False) : go rest
  where
    isSep c = c == '.' || c == '!'
    go [] = []
    go (sep : cs) = let (sym, rest) = break isSep cs
                    in (sym, sep == '!') : go rest

-- | Build the single-path model denoted by one labeled sentence. A node holds
-- its child under the child's symbol and records (in @excl@) whether the edge to
-- that child is exclusive.
sentenceToNode :: String -> LNode
sentenceToNode = build . parseLabeled
  where
    build [] = leaf
    build ((sym, edgeExcl) : rest) = LNode edgeExcl (Map.singleton sym (build rest))

-- | Build a model from a set of labeled sentences (their conjunction, via
-- 'meet'). Assumes the set is self-consistent (as any live world DB is, since
-- exclusion is enforced at insert time); an internally-contradictory set folds to
-- the empty model rather than surfacing @⊥@ — use 'meet' directly when you need to
-- detect that.
fromSentences :: [String] -> LNode
fromSentences ss = maybe leaf id (foldM meet leaf (map sentenceToNode ss))

-- | All root-to-leaf paths of the model, as @.@-joined sentences (sorted). Edge
-- labels are not re-emitted — downstream querying (via "Prax.Db") is
-- label-agnostic; exclusivity has already done its work inside 'meet'.
toSentences :: LNode -> [String]
toSentences = sort . go
  where
    go (LNode _ ks) =
      concat [ prefixed k (go child) | (k, child) <- Map.toAscList ks ]
    prefixed k []   = [k]                       -- k is a leaf
    prefixed k subs = [ k ++ "." ++ s | s <- subs ]

-- | Greatest lower bound @⊓@ (Def 8) — the conjunction of two models. @Nothing@
-- is the paper's @⊥@: at some node exclusive in either operand, the two disagree
-- on the child (Def 7 incompatibility). Otherwise the children are merged
-- (recursively meeting shared subtrees) and a node is exclusive if /either/
-- operand marks it so.
meet :: LNode -> LNode -> Maybe LNode
meet (LNode e1 k1) (LNode e2 k2) = do
  merged <- foldM ins k1 (Map.toList k2)
  let e = e1 || e2
  if e && Map.size merged > 1
    then Nothing                                 -- exclusive node forced to two children ⇒ ⊥
    else Just (LNode e merged)
  where
    ins acc (k, v2) = case Map.lookup k acc of
      Nothing -> Just (Map.insert k v2 acc)
      Just v1 -> do v <- meet v1 v2; Just (Map.insert k v acc)

-- | The information order @≤@ (Def 6): @a `leq` b@ iff @a@ has every edge of @b@,
-- with labels at least as specific (@Excl ≤ Multi@), recursively — i.e. @a@
-- entails @b@.
leq :: LNode -> LNode -> Bool
leq a@(LNode _ ka) (LNode eb kb) =
  labelLeq (excl a) eb && all present (Map.toList kb)
  where
    labelLeq ea eb' = ea || not eb'              -- Excl ≤ Multi, Multi ⋠ Excl
    present (k, bChild) = case Map.lookup k ka of
      Nothing     -> False
      Just aChild -> leq aChild bChild
