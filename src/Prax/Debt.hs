-- | Debt: an obligation with a beneficiary.
--
-- "Prax.Deontic"'s obligation already carries a duty-bearer
-- (@obliged.\<who\>.\<content\>@), but names no one to whom the duty is owed.
-- A debt is exactly that obligation /plus/ a beneficiary: the new fact
-- @debt.\<creditor\>.\<debtor\>.\<content\>@ sits beside the (unmodified)
-- obligation, so the rest of the deontic machinery
-- ('Prax.Deontic.fulfilled', 'Prax.Deontic.inBreach', 'Prax.Deontic.breach',
-- …) keeps working on @content@ exactly as authored — a debt is thin
-- vocabulary over Deontic, not a parallel mechanism.
--
-- __Stratification carries over unchanged__ (Deontic's rule, restated here):
-- @content@ is a /simple term/, one sentence, never a compound @∧@\/@∨@\/@→@ —
-- it may itself be a dotted path (e.g. @"repaid.dell.cora.coin"@), just never
-- a conjunction of separate obligations. What IS guarded here, loudly, in the
-- established segment idiom ("Prax.Project"'s @endeavor@, "Prax.Persona"):
-- a @creditor@\/@debtor@ name must be a single path segment (no @.@ or @!@) —
-- unlike @content@, these name a party, not a duty, and a dotted party name
-- would silently misparse the fact's four-part shape.
module Prax.Debt
  ( debtPath
  , owe
  , settle
  , owes
  ) where

import           Prax.Query (Condition (..))
import           Prax.Types (Outcome (..))
import           Prax.Deontic (oblige, discharge)

-- | The DB path of a debt: @debt.\<creditor\>.\<debtor\>.\<content\>@. Loudly
-- rejects a creditor or debtor name that isn't a single path segment.
debtPath :: String -> String -> String -> String
debtPath creditor debtor content
  | any (`elem` (".!" :: String)) creditor =
      error ("Prax.Debt.debtPath: creditor " ++ show creditor
             ++ " must be a single path segment (no '.' or '!')")
  | any (`elem` (".!" :: String)) debtor =
      error ("Prax.Debt.debtPath: debtor " ++ show debtor
             ++ " must be a single path segment (no '.' or '!')")
  | otherwise = "debt." ++ creditor ++ "." ++ debtor ++ "." ++ content

-- | @owe creditor debtor content@ — debtor now owes creditor: the debt fact
-- AND the underlying 'Prax.Deontic.oblige', in one call (a debt IS an
-- obligation with a beneficiary; both facts assert together, or not at all).
owe :: String -> String -> String -> [Outcome]
owe creditor debtor content =
  [ Insert (debtPath creditor debtor content), oblige debtor content ]

-- | @settle creditor debtor content@ — the debt is cleared: deletes the debt
-- fact and 'Prax.Deontic.discharge's the obligation. The world supplies
-- whatever transfer action earns this (a repay\/gift\/forgive act); 'settle'
-- is only the bookkeeping that follows.
settle :: String -> String -> String -> [Outcome]
settle creditor debtor content =
  [ Delete (debtPath creditor debtor content), discharge debtor content ]

-- | Condition: @debtor@ currently owes @creditor@ @content@.
owes :: String -> String -> String -> Condition
owes creditor debtor content = Match (debtPath creditor debtor content)
