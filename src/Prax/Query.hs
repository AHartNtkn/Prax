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
  , forAll
  , implies
  , groundCondition
  , query
  , satisfies
  , countSatisfying
  ) where

import           Data.List (nub)
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
  | Or [[Condition]]
    -- ^ Disjunction: at least one clause (each a conjunction) holds. A
    -- /generator/ — it unions the (deduplicated) bindings from every satisfying
    -- clause, so a disjunctive precondition can bind variables from either side.
  | Absent [Condition]
    -- ^ Negation-as-failure over a whole conjunction: keep the binding iff the
    -- inner conjunction has no solution. Generalizes 'Not' from a single
    -- sentence to @¬(compound)@ — e.g. @Absent [Match "leader.X", Match "X.sex!male"]@
    -- is "there is no male leader" (@¬∃@).
  | Exists [Condition]
    -- ^ Dual of 'Absent': keep the binding iff the inner conjunction /has/ a
    -- solution, discarding the witnesses (a boolean @∃@ that neither leaks nor
    -- multiplies bindings downstream).
  deriving (Eq, Show)

-- | Universal quantification: for every binding of @guard@, @body@ holds
-- (@∀ x. guard(x) → body(x)@). Defined as "there is no @guard@-binding for which
-- @body@ fails". E.g. @forAll [Match "patron.P"] [Match "P.hasDrink"]@ = "every
-- patron has a drink".
forAll :: [Condition] -> [Condition] -> Condition
forAll guard body = Absent (guard ++ [Absent body])

-- | Material implication @a → b@ (propositional: either @a@ is unsatisfiable
-- from the current binding, or @b@ holds).
implies :: [Condition] -> [Condition] -> Condition
implies a b = Or [ [Absent a], b ]

-- | Substitute bindings into every sentence/operand of a condition. Variables
-- not present in the bindings are left for the query to quantify.
groundCondition :: Bindings -> Condition -> Condition
groundCondition b c = case c of
  Match s          -> Match (ground s b)
  Not s            -> Not (ground s b)
  Eq x y           -> Eq (ground x b) (ground y b)
  Neq x y          -> Neq (ground x b) (ground y b)
  Cmp op x y       -> Cmp op (ground x b) (ground y b)
  Calc r op x y    -> Calc (ground r b) op (ground x b) (ground y b)
  Count r s        -> Count (ground r b) (ground s b)
  Subquery s f w   -> Subquery (ground s b) (map (`ground` b) f) (map (groundCondition b) w)
  Or clauses       -> Or (map (map (groundCondition b)) clauses)
  Absent cs        -> Absent (map (groundCondition b) cs)
  Exists cs        -> Exists (map (groundCondition b) cs)

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
  where
    step matches cond = case cond of
      -- parse the pattern once per condition, not once per binding
      Match s -> let names = pathNames s
                 in concatMap (unifyNames names db) matches
      Not s   -> let names = pathNames s
                 in concatMap (\b -> [ b | null (unifyNames names db b) ]) matches
      _       -> concatMap (evalCond inSub db cond) matches

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

evalCond inSub db (Or clauses) b =
  nub (concat [ queryWith inSub db clause b | clause <- clauses ])
evalCond inSub db (Absent conds) b = [ b | null  (queryWith inSub db conds b) ]
evalCond inSub db (Exists conds) b = [ b | not (null (queryWith inSub db conds b)) ]

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
