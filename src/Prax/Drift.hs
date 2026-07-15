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
  , driftPracticeId
  , driftP
  , driftSetup
  , gathering
  ) where

import           Data.List (nub, (\\))
import           Prax.Db (pathNames)
import           Prax.Query (Condition (..), CmpOp (..), CalcOp (..))
import           Prax.Types

-- | One authored pulse: every 'driftPeriod' rounds, apply each body clause
-- as a 'ForEach' (conditions may bind freely — every satisfying binding
-- fires). The due gate's own machinery lives in the @Prax@ namespace
-- ('authoredVarClash') and may not appear in an authored body.
--
-- __Authoring periods for real games__: a round (everyone acts once) is a
-- few MINUTES of fiction — roughly 12 rounds an hour, ~150 to a waking day
-- — and games run hundreds to thousands of rounds, so author periods at
-- fiction scale: hunger ~72 rounds (two meals a day), a drink wearing off
-- ~12 (an hour), a daily market ~150. The SHIPPED worlds compress these
-- (hunger 3, metabolism 2, market 6) so the test suite's short drives can
-- reach the pulses — a deliberate, NON-STANDARD truncation for testing,
-- not a model for real authoring. Tests wanting a distant pulse should
-- clock-jump (@Insert "turn!N"@, the DriftSpec idiom) rather than inherit
-- compressed periods.
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
driftChar = (character driftName) { charBoundTo = Just driftPracticeId }

-- | The drift practice's id — exported for "Prax.Relevance", whose analyses
-- treat this practice's outcomes as ENVIRONMENT dynamics, not authored
-- affordances (spec 2026-07-14-v37: tickers change motives; a clock-moved
-- fact is exactly what an environment gate is FOR).
driftPracticeId :: String
driftPracticeId = "drift"

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
  { practiceId = driftPracticeId
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
             [ Insert ("due." ++ driftRuleName r ++ "!PraxD2") ] ]
    dueGate r =
      [ Match ("due." ++ driftRuleName r ++ "!PraxD")
      , Match "turn!PraxNow"
      , Cmp Gte "PraxNow" "PraxD"
      , Calc "PraxD2" Add "PraxNow" (show (driftPeriod r)) ]

-- | Instance + one seeded due per rule, a full period out: the world starts
-- sated\/sober and the first pulse lands after one period (stated semantics).
driftSetup :: [DriftRule] -> [Outcome]
driftSetup rules =
  Insert "practice.drift.here"
    : [ Insert ("due." ++ driftRuleName r ++ "!" ++ show (driftPeriod r))
      | r <- rules ]

-- | A recurring, self-closing convening: open effects fire every @period@
-- rounds, close effects @duration@ rounds after each opening (both authored
-- meanings; 0 < duration < period — a gathering that never closes, or never
-- opens, is not a gathering). Returns the two pulse rules (give them to
-- 'driftP' with the world's other rules) and their due seeds (append to the
-- setup AFTER 'driftSetup'; the first convening lands one full period in,
-- the v36 start-sated convention).
--
-- 'driftSetup' must NOT also receive the gathering's rules — it would seed
-- both dues at @period@, opening and closing simultaneously. Worlds pass
-- plain rules to 'driftSetup' and append the gathering's own seeds;
-- 'driftP' gets ALL rules, including the gathering's.
gathering :: String -> Int -> Int -> [Outcome] -> [Outcome]
          -> ([DriftRule], [Outcome])
gathering name period duration openOuts closeOuts
  | duration < 1 || duration >= period =
      error ("Prax.Drift: gathering " ++ name
             ++ " needs 0 < duration < period")
  | otherwise =
      ( [ openR, closeR ]
      , [ Insert ("due." ++ driftRuleName openR ++ "!" ++ show period)
        , Insert ("due." ++ driftRuleName closeR
                  ++ "!" ++ show (period + duration)) ] )
  where
    openR  = DriftRule (name ++ "Open")  period [ ([], openOuts) ]
    closeR = DriftRule (name ++ "Close") period [ ([], closeOuts) ]

-- Loud construction-time guards: a multi-segment rule name would corrupt the
-- due path; a body authoring the Prax namespace would capture the gate's
-- own machinery (no interface splices here — drift bodies are whole-condition
-- author fragments, so the shared guard's interface list is empty).
guardRule :: DriftRule -> DriftRule
guardRule r
  | length (pathNames (driftRuleName r)) /= 1 =
      error ("Prax.Drift: rule name must be a single segment: "
             ++ driftRuleName r)
  | (v : _) <- offenders =
      error ("Prax.Drift: rule " ++ driftRuleName r
             ++ " authors " ++ show v
             ++ " -- the Prax namespace is reserved for the due gate's own machinery")
  | driftPeriod r < 1 =
      error ("Prax.Drift: rule " ++ driftRuleName r
             ++ " needs a positive period")
  | otherwise = r
  where
    offenders = concat [ authoredVarClash [] cs os | (cs, os) <- driftBody r ]
