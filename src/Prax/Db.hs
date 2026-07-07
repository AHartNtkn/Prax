-- | The exclusion-logic database underlying Praxis/Versu.
--
-- The world state is a trie: @newtype Db = Db (Map String Db)@. Every fact is a
-- path built from two operators — @.@ (ordinary, multi-valued descent) and @!@
-- (exclusion: the parent has exactly one child). Queries treat @.@ and @!@
-- identically; the distinction only matters on 'insert'.
--
-- See @docs/research/praxis-praxish-notes.md@ for the correspondence to Praxish's
-- @db.js@, including the corrected @!@ semantics (Praxish's own @insert@ has a
-- flagged bug that drops data; we implement the paper's rule instead).
module Prax.Db
  ( Db(..)
  , emptyDb
  , Val(..)
  , Bindings
  , valToString
  , isVariable
  , insert
  , insertAll
  , retract
  , unify
  , unifyAll
  , ground
  , dbToSentences
  , childKeys
  , exists
  , pathNames
  ) where

import           Data.Char (isUpper)
import           Data.List (sort)
import           Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map

-- | The world state: a trie whose edges are symbols and whose nodes carry no
-- other data. A leaf is a node with an empty child map.
newtype Db = Db (Map String Db)
  deriving (Eq, Show)

emptyDb :: Db
emptyDb = Db Map.empty

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
insertToks ((n, op) : rest) (Db m) =
  let Db existing = Map.findWithDefault emptyDb n m
      base = case (op, rest) of
        (Just '!', (nextName, _) : _) ->
          -- Exclusion: n keeps only the next child, with its subtree intact.
          Db (Map.filterWithKey (\k _ -> k == nextName) existing)
        _ -> Db existing
      child' = insertToks rest base
  in Db (Map.insert n child' m)

-- | Insert many sentences left to right.
insertAll :: [String] -> Db -> Db
insertAll ss db = foldl (flip insert) db ss

-- | Retract: delete the leaf named by the final segment of the path. Missing
-- intermediate nodes make this a no-op (nothing to remove).
retract :: String -> Db -> Db
retract = retractNames . parseNames
  where
    retractNames [] db = db
    retractNames [n] (Db m) = Db (Map.delete n m)
    retractNames (n : ns) (Db m) =
      case Map.lookup n m of
        Nothing    -> Db m
        Just child -> Db (Map.insert n (retractNames ns child) m)

-- | Unify one sentence against the database under existing @bindings@, yielding
-- every consistent extension. An unbound uppercase segment branches over all
-- keys of the current subtree (the list-monad nondeterminism at the core of
-- pattern matching); a bound variable or constant descends deterministically.
unify :: String -> Db -> Bindings -> [Bindings]
unify sentence (Db root) bindings =
  map snd (foldl step [(Db root, bindings)] (parseNames sentence))
  where
    step worlds part = concatMap (descend part) worlds
    descend part (Db m, b)
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

-- | Conjunctively unify a list of sentences, threading bindings through.
unifyAll :: [String] -> Db -> [Bindings]
unifyAll sentences db = foldl step [Map.empty] sentences
  where step bss s = concatMap (unify s db) bss

-- | Substitute bound variables back into a sentence, preserving @.@/@!@.
ground :: String -> Bindings -> String
ground sentence b = concatMap emit (tokens sentence)
  where
    emit (name, mop) = value name ++ maybe "" pure mop
    value name
      | isVariable name = maybe name valToString (Map.lookup name b)
      | otherwise       = name

-- | The keys directly beneath the node at a (constant) dotted path, or @[]@ if
-- the path is absent. Used to enumerate instantiated practices.
childKeys :: String -> Db -> [String]
childKeys path db = case walk (parseNames path) db of
  Just (Db m) -> Map.keys m
  Nothing     -> []
  where
    walk [] d          = Just d
    walk (n : ns) (Db m) = Map.lookup n m >>= walk ns

-- | Whether any node exists at the given constant path.
exists :: String -> Db -> Bool
exists path db = not (null (unify path db Map.empty))

-- | Enumerate all leaf paths (facts) in the database, sorted, joined by @.@.
-- Loses the @.@/@!@ distinction; intended for display and tests.
dbToSentences :: Db -> [String]
dbToSentences = sort . go
  where
    go (Db m)
      | Map.null m = []
      | otherwise  = concatMap expand (Map.toList m)
    expand (k, child@(Db cm))
      | Map.null cm = [k]
      | otherwise   = map ((k ++ ".") ++) (go child)
