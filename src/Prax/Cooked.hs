-- | The cooked mirror of the outcome language, plus the re-exported cooked
-- condition API from "Prax.Query" — together, the authored world compiled to
-- token/name form once. Strings remain the authoring surface; they stop being
-- the computation surface (see @docs/specs/2026-07-12-v28-cooked-world.md@).
module Prax.Cooked
  ( CookedCondition(..)
  , CookedOutcome(..)
  , cookCondition
  , cookOutcome
  , groundNames
  , groundCookedCondition
  , groundCookedOutcome
  ) where

import           Prax.Db (Bindings, groundTokens, tokens)
import           Prax.Query (CookedCondition (..), cookCondition, groundCookedCondition,
                              groundNames)
import           Prax.Types (Outcome (..))

-- | The cooked mirror of 'Outcome': 'Insert'/'Delete' carry the sentence
-- already split into @(name, punctuationAfterName)@ tokens ('Prax.Db.tokens');
-- 'Call'/'ForEach' recurse.
data CookedOutcome
  = CInsert [(String, Maybe Char)]
  | CDelete [(String, Maybe Char)]
  | CCall String [String]
  | CForEach [CookedCondition] [CookedOutcome]
  deriving (Eq, Show)

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
