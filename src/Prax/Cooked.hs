-- | The cooked mirror of the outcome language — the authored world compiled
-- to token/name form once. Strings remain the authoring surface; they stop
-- being the computation surface (see
-- @docs/specs/2026-07-12-v28-cooked-world.md@). The cooked condition side
-- ('CookedCondition', 'cookCondition', 'groundNames', 'groundCookedCondition')
-- lives in "Prax.Query" — its one home — and is imported here, not
-- re-exported; callers needing it import "Prax.Query" directly.
--
-- 'CookedOutcome'\/'CookedAction'\/'CookedPractice' themselves are declared in
-- "Prax.Types" (not here): 'Prax.Types.PraxState' embeds a
-- 'Prax.Types.CookedPractice' in its 'Prax.Types.cookedDefs' field, and
-- "Prax.Cooked" already depends on "Prax.Types" for 'Outcome'\/'Practice' — a
-- two-module cycle, the same shape Task 1 hit for the condition side. This
-- module owns the conversion functions only; callers needing the types import
-- "Prax.Types" directly (already the case for every module that touches
-- 'PraxState').
module Prax.Cooked
  ( cookOutcome
  , groundCookedOutcome
  , cookPractice
  ) where

import           Data.List (intercalate)
import qualified Data.Map.Strict as Map

import           Prax.Db (Bindings, groundTokens, pathNames, tokens)
import           Prax.Query (cookCondition, groundCookedCondition, groundNames)
import           Prax.Types

-- | Compile an 'Outcome' to its cooked form (see 'CookedOutcome').
cookOutcome :: Outcome -> CookedOutcome
cookOutcome o = case o of
  Insert s          -> CInsert (tokens s)
  Delete s          -> CDelete (tokens s)
  Call fn args      -> CCall fn args
  ForEach conds outs -> CForEach (map cookCondition conds) (map cookOutcome outs)

-- | Substitute bindings into a cooked outcome. 'CInsert'/'CDelete' reuse
-- 'Prax.Db.groundTokens' directly — no string rebuild; 'CCall' args are
-- single names, substituted the same way as 'groundNames'.
groundCookedOutcome :: Bindings -> CookedOutcome -> CookedOutcome
groundCookedOutcome b o = case o of
  CInsert toks        -> CInsert (groundTokens toks b)
  CDelete toks         -> CDelete (groundTokens toks b)
  CCall fn args        -> CCall fn (groundNames b args)
  CForEach conds outs  -> CForEach (map (groundCookedCondition b) conds)
                                    (map (groundCookedOutcome b) outs)

-- | Compile a 'Practice' to its cooked form (see 'CookedPractice'): every
-- action's conditions/outcomes, every init outcome, and every function case's
-- conditions/outcomes precooked; the instance-unification pattern
-- (@practice.\<pid\>.\<Role1\>...@) pre-split once.
cookPractice :: Practice -> CookedPractice
cookPractice p = CookedPractice
  { cpInstanceNames = pathNames ("practice." ++ practiceId p ++ "." ++ intercalate "." (roles p))
  , cpActions = map cookAction (actions p)
  , cpInits   = map cookOutcome (initOutcomes p)
  , cpFns     = Map.fromListWith (\_new old -> old)
      [ (fnName f, (fnParams f, [ (map cookCondition (caseConditions c), map cookOutcome (caseOutcomes c))
                                 | c <- fnCases f ]))
      | f <- functions p ]
  }
  where
    cookAction a = CookedAction
      { caName  = actionName a
      , caConds = map cookCondition (actionConditions a)
      , caOuts  = map cookOutcome (actionOutcomes a)
      }
