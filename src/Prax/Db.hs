-- | The exclusion-logic database underlying Praxis/Versu.
--
-- The world state is a trie: @data Db = Db Bool (Map String Db)@, where the 'Bool'
-- records whether the node's outgoing edges are __exclusive__. Every fact is a
-- path built from two operators — @.@ (ordinary, multi-valued descent) and @!@
-- (exclusion: the parent has exactly one child). Queries treat @.@ and @!@
-- identically, but the distinction is now /retained/ in the trie (not just applied
-- and forgotten at insert), so the world state is a faithful Exclusion-Logic model
-- — which is what lets 'Prax.EL.meet' detect a contradiction from either side of a
-- clash. 'dbToSentences' flattens the labels (for display/matching);
-- 'dbToLabeledSentences' re-emits them (for exact serialization).
--
-- See @docs/research/praxis-praxish-notes.md@ for the correspondence to Praxish's
-- @db.js@, including the corrected @!@ semantics (Praxish's own @insert@ has a
-- flagged bug that drops data; we implement the paper's rule instead).
module Prax.Db
  ( Db(..)
  , emptyDb
  , dbExcl
  , Val(..)
  , Bindings
  , valToString
  , isVariable
  , insert
  , insertToks
  , insertAll
  , retract
  , retractNames
  , unify
  , unifyNames
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
  ) where

import           Data.Char (isUpper)
import           Data.List (sort)
import           Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map

-- | The world state: a trie whose edges are symbols. The 'Bool' is the node's
-- __exclusion flag__ — 'True' when its edges are single-valued (@!@); a valid
-- exclusive node has one child. A leaf is a node with an empty child map.
data Db = Db Bool (Map String Db)
  deriving (Eq, Show)

-- | Whether a node's outgoing edges are exclusive (@!@).
dbExcl :: Db -> Bool
dbExcl (Db e _) = e

emptyDb :: Db
emptyDb = Db False Map.empty

-- | A value a logic variable can be bound to. 'unify' only ever produces
-- 'VStr' (trie keys are strings); 'VNum' and 'VSet' arise from query operators
-- (@calc@, subqueries) in "Prax.Query".
data Val
  = VStr !String
  | VNum !Integer
  | VSet ![[String]]
  deriving (Eq, Show)

-- | Map of logic-variable name to bound value.
type Bindings = Map String Val

-- | Render a value the way Praxish's @DB.ground@/@String()@ coercion does: sets
-- collapse to an opaque @\<Set(n)\>@ marker (they are not meant to be grounded
-- into sentences).
valToString :: Val -> String
valToString (VStr s) = s
valToString (VNum n) = show n
valToString (VSet xs) = "<Set(" ++ show (length xs) ++ ")>"

-- | A path segment is a variable iff its first character is uppercase
-- (Praxish's @DB.isVariable@ convention).
isVariable :: String -> Bool
isVariable (c:_) = isUpper c
isVariable []    = False

-- Split off leading/trailing ASCII whitespace.
trim :: String -> String
trim = f . f where f = reverse . dropWhile (`elem` " \t\n\r")

-- | Tokenize a sentence into @(name, punctuationAfterName)@ pairs, preserving
-- the operator following each name (used for 'ground', which must re-emit them).
tokens :: String -> [(String, Maybe Char)]
tokens = go . trim
  where
    go [] = []
    go s =
      let (name, rest) = span (\c -> c /= '.' && c /= '!') s
      in case rest of
           []          -> [(name, Nothing)]
           (op : more) -> (name, Just op) : go more

-- | Names only (both operators treated identically), for matching and retract.
parseNames :: String -> [String]
parseNames = map fst . tokens

-- | Split a sentence into its segment names (both @.@ and @!@ are separators).
pathNames :: String -> [String]
pathNames = parseNames

-- | Insert a sentence into the database, with the corrected exclusion rule.
--
-- @!@ after a name @n@ means @n@ is single-valued: after the insert, @n@ has
-- exactly one child — the next segment. We enforce this by clearing @n@'s
-- /other/ children while preserving the surviving child's existing subtree.
-- (Praxish's @DB.insert@ instead resets @n@ to empty, discarding that subtree;
-- see the regression test in @Prax.DbSpec@.)
insert :: String -> Db -> Db
insert = insertToks . tokens

insertToks :: [(String, Maybe Char)] -> Db -> Db
insertToks [] db = db
insertToks ((n, op) : rest) (Db e m) =
  let Db _ existing = Map.findWithDefault emptyDb n m
      -- @n@'s node is exclusive iff this insert reaches it via a @!@ operator.
      childExcl = op == Just '!'
      cleared = case (op, rest) of
        (Just '!', (nextName, _) : _) ->
          -- Exclusion: n keeps only the next child, with its subtree intact.
          Map.filterWithKey (\k _ -> k == nextName) existing
        _ -> existing
      child' = insertToks rest (Db childExcl cleared)
  in Db e (Map.insert n child' m)

-- | Insert many sentences left to right.
insertAll :: [String] -> Db -> Db
insertAll ss db = foldl (flip insert) db ss

-- | Retract: delete the leaf named by the final segment of the path. Missing
-- intermediate nodes make this a no-op (nothing to remove).
retract :: String -> Db -> Db
retract = retractNames . parseNames

-- | 'retract' with the sentence already split into names — for callers that
-- already hold the names (e.g. cooked outcomes) and must not re-parse them.
retractNames :: [String] -> Db -> Db
retractNames [] db = db
retractNames [n] (Db e m) = Db e (Map.delete n m)
retractNames (n : ns) (Db e m) =
  case Map.lookup n m of
    Nothing    -> Db e m
    Just child -> Db e (Map.insert n (retractNames ns child) m)

-- | 'unify' with the sentence already split into names — for callers that
-- evaluate one pattern against many binding sets ('Prax.Query' hoists the
-- parse out of that loop).
unifyNames :: [String] -> Db -> Bindings -> [Bindings]
unifyNames names db0 bindings =
  map snd (foldl step [(db0, bindings)] names)
  where
    step worlds part = concatMap (descend part) worlds
    descend part (Db _ m, b)
      | isVariable part =
          case Map.lookup part b of
            Just v  -> case Map.lookup (valToString v) m of
                         Just sub -> [(sub, b)]
                         Nothing  -> []
            Nothing -> [ (sub, Map.insert part (VStr k) b)
                       | (k, sub) <- Map.toList m ]
      | otherwise =
          case Map.lookup part m of
            Just sub -> [(sub, b)]
            Nothing  -> []

-- | Unify one sentence against the database under existing @bindings@, yielding
-- every consistent extension. An unbound uppercase segment branches over all
-- keys of the current subtree (the list-monad nondeterminism at the core of
-- pattern matching); a bound variable or constant descends deterministically.
unify :: String -> Db -> Bindings -> [Bindings]
unify sentence = unifyNames (parseNames sentence)

-- | Conjunctively unify a list of sentences, threading bindings through.
unifyAll :: [String] -> Db -> [Bindings]
unifyAll sentences db = foldl step [Map.empty] sentences
  where step bss s = concatMap (unify s db) bss

-- | 'ground' at the token level: substitute bindings into already-split
-- tokens. 'Prax.Derive.closure' grounds each axiom head once per binding —
-- tokenizing the head template once per closure, not once per binding.
--
-- __Invariant:__ a substituted value replaces its token as a single segment —
-- it is never re-split on @.@/@!@, unlike 'ground' followed by re-tokenizing.
-- This is safe only because trie keys (what 'unify' ever binds a variable to)
-- cannot themselves contain @.@ or @!@ (those characters are the tokenizer's
-- own segment separators); a caller feeding this output straight to
-- 'insertToks' (as 'Prax.Derive.closure' does) therefore relies on every
-- bound value being separator-free — true for all unify-produced bindings,
-- and required of any literal an axiom body binds via @Eq@.
groundTokens :: [(String, Maybe Char)] -> Bindings -> [(String, Maybe Char)]
groundTokens toks b = [ (value n, op) | (n, op) <- toks ]
  where
    value name
      | isVariable name = maybe name valToString (Map.lookup name b)
      | otherwise       = name

-- | Re-emit tokens as a sentence (inverse of 'tokens' up to trimming).
tokensToSentence :: [(String, Maybe Char)] -> String
tokensToSentence = concatMap emit
  where emit (name, mop) = name ++ maybe "" pure mop

-- | Substitute bound variables back into a sentence, preserving @.@/@!@.
ground :: String -> Bindings -> String
ground sentence b = tokensToSentence (groundTokens (tokens sentence) b)

-- | The keys directly beneath the node at a (constant) dotted path, or @[]@ if
-- the path is absent. Used to enumerate instantiated practices.
childKeys :: String -> Db -> [String]
childKeys path db = case walk (parseNames path) db of
  Just (Db _ m) -> Map.keys m
  Nothing       -> []
  where
    walk [] d            = Just d
    walk (n : ns) (Db _ m) = Map.lookup n m >>= walk ns

-- | Whether any node exists at the given constant path.
exists :: String -> Db -> Bool
exists path db = not (null (unify path db Map.empty))

-- | Enumerate all leaf paths (facts) in the database, sorted, joined by @.@.
-- Flattens the @.@/@!@ distinction; intended for display, matching and tests.
dbToSentences :: Db -> [String]
dbToSentences = sort . go
  where
    go (Db _ m)
      | Map.null m = []
      | otherwise  = concatMap expand (Map.toList m)
    expand (k, child@(Db _ cm))
      | Map.null cm = [k]
      | otherwise   = map ((k ++ ".") ++) (go child)

-- | Like 'dbToSentences' but __label-faithful__: each edge is re-emitted with
-- @!@ when its parent node is exclusive, else @.@. Inverse of 'insertAll' — the
-- basis for exact serialization ('Prax.Persist').
dbToLabeledSentences :: Db -> [String]
dbToLabeledSentences = sort . go
  where
    go (Db _ m) = concat
      [ case go child of
          []   -> [k]
          subs -> [ k ++ sep (dbExcl child) ++ s | s <- subs ]
      | (k, child) <- Map.toList m ]
    sep e = if e then "!" else "."
