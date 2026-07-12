-- | The exclusion-logic database underlying Praxis/Versu.
--
-- The world state is a trie: @data Db = Db Bool (IntMap Db)@, where the
-- 'Bool' records whether the node's outgoing edges are __exclusive__, and
-- the 'IntMap' keys are interned segment ids ('Prax.Sym.symId'; spec
-- @docs/specs/2026-07-12-v29-interning.md@). Every fact is a path built from
-- two operators — @.@ (ordinary, multi-valued descent) and @!@ (exclusion:
-- the parent has exactly one child). Queries treat @.@ and @!@ identically,
-- but the distinction is now /retained/ in the trie (not just applied and
-- forgotten at insert), so the world state is a faithful Exclusion-Logic
-- model — which is what lets 'Prax.EL.meet' detect a contradiction from
-- either side of a clash. 'dbToSentences' flattens the labels (for
-- display/matching); 'dbToLabeledSentences' re-emits them (for exact
-- serialization).
--
-- The authoring/display surface (@insert@, @unify@, @exists@,
-- @dbToSentences@, @dbToLabeledSentences@, @ground@, …) stays String-facing —
-- interning at entry, rendering at exit — so worlds, tests, 'Prax.Persist'
-- and 'Prax.Inspect' are unaffected by the representation change. The engine
-- itself computes on 'Prax.Sym.Sym': 'Val'\/'Bindings' carry symbols
-- natively, and the Sym-level cores ('insertToks', 'retractNames',
-- 'unifySyms', 'groundTokens', 'tokensToSentence') are exported for callers
-- that already hold split, interned tokens (the cooked pipeline — "Prax.Cooked",
-- "Prax.Query", "Prax.Derive") and must not re-parse or re-render them. 'Sym'
-- ids are first-encounter ordered and therefore run-dependent
-- ('Prax.Sym'); anywhere the old code observed @Map String@'s alphabetical
-- iteration order (unify's unbound-variable branching, 'childKeys'), the
-- rewrite explicitly restores name order by sorting on 'symName' — id
-- (encounter) order must never leak into candidate order or output.
--
-- See @docs/research/praxis-praxish-notes.md@ for the correspondence to
-- Praxish's @db.js@, including the corrected @!@ semantics (Praxish's own
-- @insert@ has a flagged bug that drops data; we implement the paper's rule
-- instead).
module Prax.Db
  ( Db(..)
  , emptyDb
  , dbExcl
  , Val(..)
  , Bindings
  , valToString
  , valToSym
  , isVariable
  , insert
  , insertToks
  , insertAll
  , retract
  , retractNames
  , unify
  , unifyNames
  , unifySyms
  , unifyAll
  , ground
  , groundTokens
  , tokensToSentence
  , dbToSentences
  , dbToLabeledSentences
  , childKeys
  , exists
  , pathNames
  , tokens
  , internTokens
  ) where

import           Data.Bifunctor (first)
import           Data.Char (isUpper)
import           Data.IntMap.Strict (IntMap)
import qualified Data.IntMap.Strict as IntMap
import           Data.List (sort, sortOn)
import           Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map

import           Prax.Sym (Sym, intern, symId, symIsVar, symName, symOfId)

-- | The world state: a trie whose edges are interned segments. The 'Bool' is
-- the node's __exclusion flag__ — 'True' when its edges are single-valued
-- (@!@); a valid exclusive node has one child. A leaf is a node with an
-- empty child map.
data Db = Db Bool (IntMap Db)
  deriving (Eq, Show)

-- | Whether a node's outgoing edges are exclusive (@!@).
dbExcl :: Db -> Bool
dbExcl (Db e _) = e

emptyDb :: Db
emptyDb = Db False IntMap.empty

-- | A value a logic variable can be bound to. 'unify' only ever produces
-- 'VSym' (trie keys are symbols); 'VNum' and 'VSet' arise from query
-- operators (@calc@, subqueries) in "Prax.Query".
data Val
  = VSym !Sym
  | VNum !Integer
  | VSet ![[Sym]]
  deriving (Eq, Show)

-- | Map of logic-variable symbol to bound value — the representation the
-- engine computes over natively (spec
-- @docs/specs/2026-07-12-v29-interning.md@); Strings appear only at the
-- authoring/display boundary.
type Bindings = Map Sym Val

-- | Render a value the way Praxish's @DB.ground@/@String()@ coercion does:
-- sets collapse to an opaque @\<Set(n)\>@ marker (they are not meant to be
-- grounded into sentences).
valToString :: Val -> String
valToString (VSym s) = symName s
valToString (VNum n) = show n
valToString (VSet xs) = "<Set(" ++ show (length xs) ++ ")>"

-- | The symbol a value substitutes as, when grounding a Sym-token position
-- ('groundTokens'): a bound 'VSym' is returned as-is (no render/re-intern
-- round trip); any other 'Val' is rendered via 'valToString' and interned.
-- Consistent with 'valToString' by construction — 'Prax.Sym.intern' is a
-- deterministic, injective map on Strings, so @valToSym x == valToSym y@ iff
-- @valToString x == valToString y@ — which is what lets "Prax.Query" compare
-- bound values by 'Sym' (an 'Int') instead of by String equality.
valToSym :: Val -> Sym
valToSym (VSym s) = s
valToSym v         = intern (valToString v)

-- | A path segment is a variable iff its first character is uppercase
-- (Praxish's @DB.isVariable@ convention).
isVariable :: String -> Bool
isVariable (c:_) = isUpper c
isVariable []    = False

-- Split off leading/trailing ASCII whitespace.
trim :: String -> String
trim = f . f where f = reverse . dropWhile (`elem` " \t\n\r")

-- | Tokenize a sentence into @(name, punctuationAfterName)@ pairs,
-- preserving the operator following each name (used for 'ground', which
-- must re-emit them).
tokens :: String -> [(String, Maybe Char)]
tokens = go . trim
  where
    go [] = []
    go s =
      let (name, rest) = span (\c -> c /= '.' && c /= '!') s
      in case rest of
           []          -> [(name, Nothing)]
           (op : more) -> (name, Just op) : go more

-- | 'tokens', interning each segment name — the Sym-level entry every
-- computation path builds on: parsing an authored sentence into the tokens
-- the engine actually unifies\/grounds\/inserts against.
internTokens :: String -> [(Sym, Maybe Char)]
internTokens = map (first intern) . tokens

-- | Names only (both operators treated identically), for matching and
-- retract.
parseNames :: String -> [String]
parseNames = map fst . tokens

-- | Split a sentence into its segment names (both @.@ and @!@ are
-- separators).
pathNames :: String -> [String]
pathNames = parseNames

-- | Insert a sentence into the database, with the corrected exclusion rule.
--
-- @!@ after a name @n@ means @n@ is single-valued: after the insert, @n@ has
-- exactly one child — the next segment. We enforce this by clearing @n@'s
-- /other/ children while preserving the surviving child's existing subtree.
-- (Praxish's @DB.insert@ instead resets @n@ to empty, discarding that
-- subtree; see the regression test in @Prax.DbSpec@.)
insert :: String -> Db -> Db
insert = insertToks . internTokens

-- | Insert already-interned tokens — the Sym-level core: 'insert' is
-- @insertToks . internTokens@; the cooked hot path ('Prax.Engine') builds
-- its tokens once (at cook time or via 'internTokens' at the delta site) and
-- calls this directly, never re-parsing or re-interning a sentence.
insertToks :: [(Sym, Maybe Char)] -> Db -> Db
insertToks [] db = db
insertToks ((n, op) : rest) (Db e m) =
  let i = symId n
      Db _ existing = IntMap.findWithDefault emptyDb i m
      -- @n@'s node is exclusive iff this insert reaches it via a @!@ operator.
      childExcl = op == Just '!'
      cleared = case (op, rest) of
        (Just '!', (nextSym, _) : _) ->
          -- Exclusion: n keeps only the next child, with its subtree intact.
          IntMap.filterWithKey (\k _ -> k == symId nextSym) existing
        _ -> existing
      child' = insertToks rest (Db childExcl cleared)
  in Db e (IntMap.insert i child' m)

-- | Insert many sentences left to right.
insertAll :: [String] -> Db -> Db
insertAll ss db = foldl (flip insert) db ss

-- | Retract: delete the leaf named by the final segment of the path. Missing
-- intermediate nodes make this a no-op (nothing to remove).
retract :: String -> Db -> Db
retract = retractNames . map intern . parseNames

-- | 'retract' with the sentence already split into interned names — for
-- callers that already hold them (e.g. cooked outcomes) and must not
-- re-parse\/re-intern.
retractNames :: [Sym] -> Db -> Db
retractNames [] db = db
retractNames [n] (Db e m) = Db e (IntMap.delete (symId n) m)
retractNames (n : ns) (Db e m) =
  case IntMap.lookup (symId n) m of
    Nothing    -> Db e m
    Just child -> Db e (IntMap.insert (symId n) (retractNames ns child) m)

-- | 'unify' with the sentence already split into names — for callers that
-- evaluate one pattern against many binding sets ('Prax.Query' hoists the
-- parse out of that loop). Interns the pattern names, then delegates
-- directly to 'unifySyms' — 'Bindings' is already Sym-keyed, so there is no
-- render/re-intern boundary to cross.
unifyNames :: [String] -> Db -> Bindings -> [Bindings]
unifyNames names = unifySyms (map intern names)

-- | The Sym-level unification core: descends the trie by 'IntMap' lookup,
-- using the parity bit test ('Prax.Sym.symIsVar') for the hottest
-- predicate. An unbound variable branches over all children of the current
-- subtree.
--
-- __Ordering hazard__: 'IntMap.toList' yields id (first-encounter,
-- run-dependent) order, not the alphabetical order the old
-- @Map String Db@ gave for free — and that order feeds candidate order,
-- which feeds planner tie-breaks and the goldens. The unbound-variable
-- branch therefore sorts explicitly by 'symName' before branching (cost:
-- only at variable-branch points, over small per-node child lists).
unifySyms :: [Sym] -> Db -> Bindings -> [Bindings]
unifySyms syms db0 bindings =
  map snd (foldl step [(db0, bindings)] syms)
  where
    step worlds sym = concatMap (descend sym) worlds
    descend sym (Db _ m, b)
      | symIsVar sym =
          case Map.lookup sym b of
            Just v  -> case IntMap.lookup (symId (valToSym v)) m of
                         Just sub -> [(sub, b)]
                         Nothing  -> []
            Nothing ->
              [ (sub, Map.insert sym (VSym (symOfId k)) b)
              | (k, sub) <- sortOn (symName . symOfId . fst) (IntMap.toList m) ]
      | otherwise =
          case IntMap.lookup (symId sym) m of
            Just sub -> [(sub, b)]
            Nothing  -> []

-- | Unify one sentence against the database under existing @bindings@,
-- yielding every consistent extension. An unbound uppercase segment
-- branches over all keys of the current subtree (the list-monad
-- nondeterminism at the core of pattern matching); a bound variable or
-- constant descends deterministically.
unify :: String -> Db -> Bindings -> [Bindings]
unify sentence = unifyNames (parseNames sentence)

-- | Conjunctively unify a list of sentences, threading bindings through.
unifyAll :: [String] -> Db -> [Bindings]
unifyAll sentences db = foldl step [Map.empty] sentences
  where step bss s = concatMap (unify s db) bss

-- | 'ground' at the token level: substitute bindings into already-split,
-- interned tokens. 'Prax.Derive.closure' grounds each axiom head once per
-- binding — tokenizing the head template once per closure, not once per
-- binding.
--
-- __Invariant:__ a substituted value replaces its token as a single segment
-- — it is never re-split on @.@/@!@, unlike 'ground' followed by
-- re-tokenizing. This is safe only because trie keys (what 'unify' ever
-- binds a variable to) cannot themselves contain @.@ or @!@ (those
-- characters are the tokenizer's own segment separators); a caller feeding
-- this output straight to 'insertToks' (as 'Prax.Derive.closure' does)
-- therefore relies on every bound value being separator-free — true for all
-- unify-produced bindings, and required of any literal an axiom body binds
-- via @Eq@.
groundTokens :: [(Sym, Maybe Char)] -> Bindings -> [(Sym, Maybe Char)]
groundTokens toks b = [ (value n, op) | (n, op) <- toks ]
  where
    value n
      | symIsVar n = maybe n valToSym (Map.lookup n b)
      | otherwise  = n

-- | Re-emit interned tokens as a sentence (inverse of 'internTokens' up to
-- trimming).
tokensToSentence :: [(Sym, Maybe Char)] -> String
tokensToSentence = concatMap emit
  where emit (name, mop) = symName name ++ maybe "" pure mop

-- | Substitute bound variables back into a sentence, preserving @.@/@!@.
ground :: String -> Bindings -> String
ground sentence b = tokensToSentence (groundTokens (internTokens sentence) b)

-- | The keys directly beneath the node at a (constant) dotted path, sorted
-- by name, or @[]@ if the path is absent. Used to enumerate instantiated
-- practices. Explicit sort restores what @Map.keys@ gave for free under the
-- old String-keyed trie: 'IntMap.keys' yields id (encounter) order, which
-- is run-dependent and must never leak into an enumeration order that
-- callers (e.g. 'Prax.Engine.possibleActions') fold into candidate order.
childKeys :: String -> Db -> [String]
childKeys path db = case walk (map intern (parseNames path)) db of
  Just (Db _ m) -> sort (map (symName . symOfId) (IntMap.keys m))
  Nothing       -> []
  where
    walk [] d = Just d
    walk (n : ns) (Db _ m) = IntMap.lookup (symId n) m >>= walk ns

-- | Whether any node exists at the given constant path.
exists :: String -> Db -> Bool
exists path db = not (null (unify path db Map.empty))

-- | Enumerate all leaf paths (facts) in the database, sorted, joined by
-- @.@. Flattens the @.@/@!@ distinction; intended for display, matching and
-- tests. The final 'sort' operates on fully rendered strings, so it is safe
-- regardless of the (run-dependent) 'IntMap' traversal order feeding it.
dbToSentences :: Db -> [String]
dbToSentences = sort . go
  where
    go (Db _ m)
      | IntMap.null m = []
      | otherwise     = concatMap expand (IntMap.toList m)
    expand (k, child@(Db _ cm))
      | IntMap.null cm = [name]
      | otherwise      = map ((name ++ ".") ++) (go child)
      where name = symName (symOfId k)

-- | Like 'dbToSentences' but __label-faithful__: each edge is re-emitted
-- with @!@ when its parent node is exclusive, else @.@. Inverse of
-- 'insertAll' — the basis for exact serialization ('Prax.Persist'). As with
-- 'dbToSentences', the final 'sort' operates on rendered strings, so it is
-- unaffected by 'IntMap' traversal order.
dbToLabeledSentences :: Db -> [String]
dbToLabeledSentences = sort . go
  where
    go (Db _ m) = concat
      [ case go child of
          []   -> [name]
          subs -> [ name ++ sep (dbExcl child) ++ s | s <- subs ]
      | (k, child) <- IntMap.toList m, let name = symName (symOfId k) ]
    sep e = if e then "!" else "."
