-- | Decay & drift: episodic state on the clock (spec
-- @docs/specs/2026-07-14-v36-drift.md@). A bodiless per-round drifter (the
-- v18\/'Prax.Sight' ticker idiom — zero engine surface) applies authored
-- __pulse rules__: each rule carries a @due.\<name\>!D@ fact and fires its
-- 'ForEach' body when the sight clock (@turn!N@) reaches the due, then
-- re-arms @period@ rounds from NOW (a stalled world does not rapid-fire on
-- resume; the period is an authored world parameter with stated meaning).
-- Time changes appetites and intoxication — never dispositions: grudges,
-- marks, trust, and standing change through ACTS, and no rule here may be
-- authored over them (the spec's principle; enforcement is review, the
-- mechanism is indifferent). The drifter depends on the sight clock;
-- 'Prax.TypeCheck.typeCheck' flags a drift practice in a clockless world.
module Prax.Drift
  ( DriftRule (..)
  , driftName
  , driftChar
  , driftP
  , driftSetup
  ) where

import           Data.List (nub, (\\))
import           Prax.Db (pathNames)
import           Prax.Query (Condition (..), CmpOp (..), CalcOp (..))
import           Prax.Types

-- | One authored pulse: every 'driftPeriod' rounds, apply each body clause
-- as a 'ForEach' (conditions may bind freely — every satisfying binding
-- fires). Variables @D@\/@D2@\/@Now@ are reserved by the due gate.
data DriftRule = DriftRule
  { driftRuleName :: String   -- ^ single segment (loud error otherwise)
  , driftPeriod   :: Int      -- ^ rounds between pulses; authored meaning
  , driftBody     :: [([Condition], [Outcome])]
  }

-- | The drifter's name (bodiless; bound to its practice; blank label, so
-- the CLI's silent-action suppression hides it).
driftName :: String
driftName = "_drift"

driftChar :: Character
driftChar = (character driftName) { charBoundTo = Just "drift" }

-- | Compile the rules into the drifter's single unconditional action: per
-- rule, its body clauses (due-gated) then its due re-arm. Between pulses
-- every gate fails and the action is a cheap no-op; under v35 the drifter's
-- motive signature never changes, so it serves its standing intention
-- forever after the first turn.
driftP :: [DriftRule] -> Practice
driftP rules
  | names /= nub names =
      error ("Prax.Drift: duplicate rule names would share one due path: "
             ++ show (names \\ nub names))
  | otherwise = practice
  { practiceId = "drift"
  , practiceName = "time works on bodies and moods"
  , roles = ["S"]
  , actions =
      [ action ""
          [ Eq "Actor" driftName ]
          (concatMap (compileRule . guardRule) rules)
      ]
  }
  where
    names = map driftRuleName rules
    compileRule r =
      [ ForEach (dueGate r ++ conds) outs | (conds, outs) <- driftBody r ]
      ++ [ ForEach (dueGate r)
             [ Insert ("due." ++ driftRuleName r ++ "!D2") ] ]
    dueGate r =
      [ Match ("due." ++ driftRuleName r ++ "!D")
      , Match "turn!Now"
      , Cmp Gte "Now" "D"
      , Calc "D2" Add "Now" (show (driftPeriod r)) ]

-- | Instance + one seeded due per rule, a full period out: the world starts
-- sated\/sober and the first pulse lands after one period (stated semantics).
driftSetup :: [DriftRule] -> [Outcome]
driftSetup rules =
  Insert "practice.drift.here"
    : [ Insert ("due." ++ driftRuleName r ++ "!" ++ show (driftPeriod r))
      | r <- rules ]

-- Loud construction-time guards: a multi-segment rule name would corrupt the
-- due path; a body using D/D2/Now would capture the gate's bindings.
guardRule :: DriftRule -> DriftRule
guardRule r
  | length (pathNames (driftRuleName r)) /= 1 =
      error ("Prax.Drift: rule name must be a single segment: "
             ++ driftRuleName r)
  | any (`elem` reserved) (bodyVars r) =
      error ("Prax.Drift: rule " ++ driftRuleName r
             ++ " uses a reserved variable (D/D2/Now)")
  | driftPeriod r < 1 =
      error ("Prax.Drift: rule " ++ driftRuleName r
             ++ " needs a positive period")
  | otherwise = r
  where
    reserved = ["D", "D2", "Now"]
    bodyVars rl = concat
      [ concatMap condNames cs ++ concatMap outNames os
      | (cs, os) <- driftBody rl ]
    condNames c = case c of
      Match s          -> pathNames s
      Not s            -> pathNames s
      Absent cs        -> concatMap condNames cs
      Exists cs        -> concatMap condNames cs
      Or cls           -> concatMap (concatMap condNames) cls
      Subquery v f w   -> v : f ++ concatMap condNames w
      Eq a b           -> [a, b]
      Neq a b          -> [a, b]
      Cmp _ a b        -> [a, b]
      Calc v _ a b     -> [v, a, b]
      Count v s        -> [v, s]
    outNames o = case o of
      Insert s       -> pathNames s
      Delete s       -> pathNames s
      Call _ as      -> as
      ForEach cs os  -> concatMap condNames cs ++ concatMap outNames os
