-- | Deontic Exclusion Logic — a first-class "should"/obligation layer
-- (Evans, "Introducing Exclusion Logic as a Deontic Logic", DEON 2010; the full
-- distillation is in @docs/research/deon-notes.md@).
--
-- The paper's result is that a deontic logic needs /no new semantics/ over
-- Exclusion Logic: @□P@ ("P should be the case") is mere syntactic sugar for the
-- ordinary sentence @Ob:P@. Our world DB is already an Exclusion-Logic model (our
-- @.@ is the paper's multi-valued operator; our @!@ is its exclusion), so this
-- module adds /no machinery/ — like "Prax.Core"/"Prax.Reactions" it is only a
-- library of smart constructors over the existing engine (no change to the DB,
-- the 'Prax.Query.Condition' language, or the planner).
--
-- An obligation "@who@ should bring about @content@" is the fact
-- @obliged.\<who\>.\<content\>@ (mirroring "Prax.Reactions"'s
-- @violated.\<who\>.\<norm\>@). Obligations are multi-valued, so one agent may
-- hold many at once. Iterated @□□@ — the reparative / contrary-to-duty
-- obligation — is the nested fact @obliged.\<who\>.obliged.\<who\>.\<content\>@
-- ('obligeReparative').
--
-- __Stratification is the whole point.__ It is what avoids the Standard-Deontic-
-- Logic paradoxes (Ross's paradox, "tautologies are obligatory", Chisholm): @□@
-- applies only to a /simple term/, never to @∧@/@∨@/@→@. So 'oblige'/'isObliged'
-- take a plain sentence 'String', never a @['Condition']@ — do not relax this.
--
-- __What this faithfully implements__ (paper §2.3): the /representation/ (□ = a
-- fact), /conflict detection/ (property 2 — incompatible obligations collapse to
-- ⊥: 'conflicts'), and /behavioural coupling/ (wants over obligations drive the
-- unchanged utility planner — 'wantFulfilled'/'avoidBreach').
--
-- __What it deliberately does not implement__ (documented, not hidden): closure
-- under implication (property 1 — @Ob.P@ together with @P→Q@ does /not/
-- auto-derive @Ob.Q@; our engine queries facts, it does not derive them), and the
-- LRT/@m(X)@ decision procedure (that machinery would belong to a future static
-- type checker, LEDGER #8).
--
-- __Conflict: detection vs. resolution.__ 'conflicts' is a /predictive/ test —
-- "are these two contents jointly satisfiable?". Two genuinely incompatible
-- obligations /cannot be co-held/ in the DB: asserting the second @!@-clears the
-- part of the first they disagree on, which is exactly the paper's ⊥ (you cannot
-- coherently owe both). Live drama instead comes from /distinct/ duties whose
-- fulfilments compete for an exclusive world-resource; those are held together and
-- __resolved emergently__ — the utility planner fulfils the higher-valued duty and
-- the loser falls 'inBreach'. An explicit priority ordering (Alchourrón–Makinson)
-- is a clean later extension, not built here.
module Prax.Deontic
  ( -- * Fact convention
    obligationPath
    -- * Asserting / discharging obligations (Outcomes)
  , oblige
  , discharge
  , breach
  , obligeReparative
    -- * Querying obligations (Conditions)
  , isObliged
  , fulfilled
  , inBreach
    -- * Norm-conflict detection (pure)
  , conflicts
  , incompatiblePairs
  , obligationsOf
    -- * Behavioural coupling (Wants)
  , wantFulfilled
  , avoidBreach
  ) where

import           Data.List (isPrefixOf, tails)

import           Prax.Db (Db, insertAll, emptyDb, exists, dbToSentences)
import           Prax.Query (Condition (..))
import           Prax.Types (Outcome (..), Want (..))
import           Prax.Reactions (markViolation, violationOf)

-- Fact convention --------------------------------------------------------------

-- | The DB path of an obligation: @obliged.\<who\>.\<content\>@. @content@ is a
-- /simple term/ (a sentence) and is kept verbatim, so an exclusion @!@ inside it
-- survives into the trie (this is what makes 'conflicts' meaningful).
obligationPath :: String -> String -> String
obligationPath who content = "obliged." ++ who ++ "." ++ content

-- Asserting / discharging ------------------------------------------------------

-- | @oblige who content@ — assert that @who@ should bring about @content@ (@□content@).
oblige :: String -> String -> Outcome
oblige who content = Insert (obligationPath who content)

-- | Retract an obligation (it has been met, or is cancelled).
discharge :: String -> String -> Outcome
discharge who content = Delete (obligationPath who content)

-- | Record that @who@ left a duty unmet. A breach /is/ a norm violation, so this
-- reuses "Prax.Reactions" verbatim (agents given a negative 'avoidBreach' want
-- steer clear of it, exactly as with any violation).
breach :: String -> String -> Outcome
breach = markViolation

-- | The reparative / contrary-to-duty obligation @□□content@ — "ideally you would
-- have brought about @content@; failing that, you now ought to". It is an
-- obligation /about an obligation/: the nested fact
-- @obliged.\<who\>.obliged.\<who\>.\<content\>@.
obligeReparative :: String -> String -> Outcome
obligeReparative who content = oblige who (obligationPath who content)

-- Querying ---------------------------------------------------------------------

-- | Condition: @who@ is currently obliged to bring about @content@.
isObliged :: String -> String -> Condition
isObliged who content = Match (obligationPath who content)

-- | Conditions for a met obligation: @who@ is obliged to @content@ /and/ it
-- actually holds. Ought does not collapse to is (paper §2.3, property 3), so both
-- conjuncts are required.
fulfilled :: String -> String -> [Condition]
fulfilled who content = [ isObliged who content, Match content ]

-- | Condition: @who@ is in breach of the duty @content@ (a recorded violation).
inBreach :: String -> String -> Condition
inBreach = violationOf

-- Norm-conflict detection ------------------------------------------------------

-- | The paper's incompatibility test (property 2): are @c1@ and @c2@ jointly
-- unsatisfiable? Assert both into a scratch model; they conflict iff one
-- @!@-clears the other so they cannot both survive (e.g. @conflicts "go!true"
-- "go!false" == True@). Symmetric; @False@ for identical or compatible contents.
-- Pure and cheap (the DB is persistent).
--
-- (There is deliberately no world-seeded variant: our @!@ exclusion is applied
-- per-insert rather than as a persistent schema, so a live world can never change
-- whether two contents mutually conflict — such a function would just duplicate
-- this one.)
conflicts :: String -> String -> Bool
conflicts c1 c2 =
  let d = insertAll [c1, c2] emptyDb
  in not (exists c1 d && exists c2 d)

-- | Every incompatible pair among a list of candidate contents — for vetting a
-- /proposed/ set of duties for internal consistency before assigning them (two
-- genuinely incompatible obligations cannot be co-held, so this checks candidates,
-- not the live DB).
incompatiblePairs :: [String] -> [(String, String)]
incompatiblePairs cs =
  [ (a, b) | (a : rest) <- tails cs, b <- rest, conflicts a b ]

-- | The contents @who@ is currently obliged to (for introspection / a drama
-- manager). Note: reading back from the trie flattens the @!@/@.@ distinction, so
-- results are for display, not to be fed back into 'conflicts'.
obligationsOf :: String -> Db -> [String]
obligationsOf who d =
  [ drop (length prefix) s | s <- dbToSentences d, prefix `isPrefixOf` s ]
  where prefix = "obliged." ++ who ++ "."

-- Behavioural coupling ---------------------------------------------------------

-- | A want to /fulfil/ an obligation: utility @k@ per met duty. Feeds the
-- unchanged planner ("Prax.Planner"), which then pursues the duty.
wantFulfilled :: String -> String -> Int -> Want
wantFulfilled who content = Want (fulfilled who content)

-- | A want to /avoid breaching/ a duty: applied as strong-negative @-|k|@ so the
-- planner steers away from the breach (the mechanism by which norms already shape
-- behaviour in "Prax.Worlds.Bar").
avoidBreach :: String -> String -> Int -> Want
avoidBreach who content k = Want [ inBreach who content ] (negate (abs k))
