-- | Authoring inspector: "why is this action unavailable?" (Versu's runtime
-- inspector — "find out why an action's preconditions have failed").
--
-- An action's conditions are a conjunctive @[Condition]@ evaluated left to right.
-- 'firstFailing' finds the first condition whose prefix empties the binding set —
-- i.e. the one that ruled the action out — reviving Praxish's @killsPerStep@.
-- 'explain' surfaces that for every practice instance an action could apply to.
-- Reuses the query evaluator; no engine changes.
module Prax.Inspect
  ( firstFailing
  , explain
  ) where

import           Data.List (intercalate, isInfixOf)
import qualified Data.Map.Strict as Map
import           Data.Maybe (listToMaybe)

import           Prax.Db (Bindings, Db, Val (..), childKeys, unify)
import           Prax.Query (Condition, query)
import           Prax.Sym (intern)
import           Prax.Types
import           Prax.Engine (renderText)

-- | The first condition (in evaluation order) after which the conjunction has no
-- solution from @b0@ — the condition that blocks the query — or 'Nothing' if the
-- whole conjunction is satisfiable.
firstFailing :: Db -> [Condition] -> Bindings -> Maybe Condition
firstFailing d conds b0 =
  listToMaybe [ conds !! (k - 1)
              | k <- [1 .. length conds]
              , null (query d (take k conds) b0) ]

-- | For @actor@, explain every action whose name contains @needle@: for each
-- practice instance it could apply to, either it is @AVAILABLE@ or it is
-- @blocked by@ a specific condition.
explain :: PraxState -> String -> String -> [String]
explain st actor needle =
  [ label ++ verdict
  | pid <- childKeys "practice" (db st)
  , Just def <- [Map.lookup pid (practiceDefs st)]
  , let instanceQuery = "practice." ++ pid ++ "." ++ intercalate "." (roles def)
  , inst <- unify instanceQuery (db st) (Map.singleton (intern "Actor") (VSym (intern actor)))
  , a <- actions def
  , let label = renderText (actionName a) inst
  , needle `isInfixOf` label
  , let verdict = case firstFailing (db st) (actionConditions a) inst of
                    Nothing -> " — AVAILABLE"
                    Just c  -> " — blocked by: " ++ show c ]
