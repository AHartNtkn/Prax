-- | The condition language layered over 'Prax.Db.unify'.
--
-- A typed reconstruction of Praxish's @Praxish.query@ (its @praxish.js@ encodes
-- conditions as whitespace-split strings; we use a 'Condition' ADT instead). A
-- query is a conjunctive list of conditions evaluated left to right, threading a
-- growing set of variable bindings (the list-monad nondeterminism from 'unify').
--
-- Operand strings follow the language's variable convention: an operand whose
-- first character is uppercase is a logic variable (resolved against the current
-- bindings), otherwise it is a literal constant. When a comparison's operand
-- cannot be resolved (an unbound variable), that binding simply fails the
-- condition and is dropped — the defined behaviour of the DSL, matching Praxish
-- (e.g. the tic-tac-toe tie action relies on it). Structural errors that cannot
-- depend on runtime state (a subquery nested inside a subquery) crash loudly.
module Prax.Query
  ( CmpOp(..)
  , CalcOp(..)
  , Condition(..)
  , query
  , satisfies
  , countSatisfying
  ) where

import qualified Data.Map.Strict as Map
import           Text.Read (readMaybe)

import           Prax.Db

-- | Numeric comparison operators (@lt@, @lte@, @gt@, @gte@).
data CmpOp = Lt | Lte | Gt | Gte
  deriving (Eq, Show)

-- | Binary integer operators for 'Calc' (@add@, @sub@, @mul@; Praxish omits
-- division deliberately to keep the DB integer-valued).
data CalcOp = Add | Sub | Mul
  deriving (Eq, Show)

-- | A single condition in a query.
data Condition
  = Match String
    -- ^ A bare logic sentence; unified against the DB (extends bindings).
  | Not String
    -- ^ Negation as failure: keep the binding iff the sentence has no match.
  | Eq String String
    -- ^ Equality that doubles as assignment: if exactly one operand is an
    -- unbound variable, bind it to the other; if both are known, keep iff equal.
  | Neq String String
    -- ^ Keep the binding iff the two (resolvable) operands differ.
  | Cmp CmpOp String String
    -- ^ Numeric comparison of two resolvable operands.
  | Calc String CalcOp String String
    -- ^ @Calc result op lhs rhs@: bind @result@ to @lhs op rhs@ (integers).
  | Count String String
    -- ^ @Count result setVar@: bind @result@ to the size of a set-valued var.
  | Subquery
      { subSet   :: String     -- ^ variable to bind to the resulting set
      , subFind  :: [String]   -- ^ variables projected out of each sub-result
      , subWhere :: [Condition] -- ^ the sub-query's conditions
      }
    -- ^ Run a nested query and bind @subSet@ to the list of projected rows.
  deriving (Eq, Show)

-- | Evaluate a conjunctive list of conditions from a starting binding, yielding
-- every consistent binding that satisfies them all.
query :: Db -> [Condition] -> Bindings -> [Bindings]
query = queryWith False

-- | True iff the conditions are satisfiable from the given binding.
satisfies :: Db -> [Condition] -> Bindings -> Bool
satisfies db conds b = not (null (query db conds b))

-- | Number of consistent bindings satisfying the conditions (used for utility
-- scoring: a want scores once per satisfying instantiation).
countSatisfying :: Db -> [Condition] -> Bindings -> Int
countSatisfying db conds b = length (query db conds b)

-- The Bool flag marks whether we are already inside a subquery (subqueries may
-- not nest).
queryWith :: Bool -> Db -> [Condition] -> Bindings -> [Bindings]
queryWith inSub db conds b0 = foldl step [b0] conds
  where step matches cond = concatMap (evalCond inSub db cond) matches

evalCond :: Bool -> Db -> Condition -> Bindings -> [Bindings]
evalCond _ db (Match s) b = unify s db b
evalCond _ db (Not s)   b = [b | null (unify s db b)]

evalCond _ _ (Eq lhs rhs) b =
  case (resolve b lhs, resolve b rhs) of
    (Just l, Just r)   -> [b | valToString l == valToString r]
    (Just l, Nothing)  -> [Map.insert rhs l b]   -- rhs is an unbound variable
    (Nothing, Just r)  -> [Map.insert lhs r b]   -- lhs is an unbound variable
    (Nothing, Nothing) -> []                     -- unresolvable: fail the binding

evalCond _ _ (Neq lhs rhs) b =
  case (resolve b lhs, resolve b rhs) of
    (Just l, Just r) -> [b | valToString l /= valToString r]
    _                -> []

evalCond _ _ (Cmp op lhs rhs) b =
  case (resolve b lhs >>= num, resolve b rhs >>= num) of
    (Just l, Just r) -> [b | applyCmp op l r]
    _                -> []

evalCond _ _ (Calc result op lhs rhs) b =
  case (resolve b lhs >>= num, resolve b rhs >>= num) of
    (Just l, Just r) -> [Map.insert result (VNum (applyCalc op l r)) b]
    _                -> []

evalCond _ _ (Count result setVar) b =
  case resolve b setVar of
    Just (VSet xs) -> [Map.insert result (VNum (fromIntegral (length xs))) b]
    _              -> []

evalCond inSub db (Subquery setVar find conds) b
  | inSub = error "Prax.Query: subquery nested inside a subquery"
  | otherwise =
      let results = queryWith True db conds b
          rows = [ [ maybe lvar valToString (Map.lookup lvar r) | lvar <- find ]
                 | r <- results ]
      in [Map.insert setVar (VSet rows) b]

-- Resolve an operand: an uppercase-initial operand is a variable (look it up),
-- otherwise it is a literal constant. Returns Nothing for an unbound variable.
resolve :: Bindings -> String -> Maybe Val
resolve b s
  | isVariable s = Map.lookup s b
  | otherwise    = Just (VStr s)

num :: Val -> Maybe Integer
num (VNum n) = Just n
num (VStr s) = readMaybe s
num (VSet _) = Nothing

applyCmp :: CmpOp -> Integer -> Integer -> Bool
applyCmp Lt  = (<)
applyCmp Lte = (<=)
applyCmp Gt  = (>)
applyCmp Gte = (>=)

applyCalc :: CalcOp -> Integer -> Integer -> Integer
applyCalc Add = (+)
applyCalc Sub = (-)
applyCalc Mul = (*)
