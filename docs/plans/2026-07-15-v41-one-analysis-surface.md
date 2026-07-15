# v41 — One Analysis Surface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** per `docs/specs/2026-07-15-v41-one-analysis-surface.md` — the world-model
analyses consume the cooked tables (`cookedDefs`/`cookedRules`/`cookedDesires`/
`cookedWants`); the string-side analysis walkers (`mayUnify`, `wantPatterns`,
`outcomeAtoms`, `condPatterns`, string `evictionShadows`) are DELETED. Re-plumbing, not
re-meaning: every derived table classification-identical, gated by observational pins
laid BEFORE the switch.

**Architecture:** `cookedReadAnchors` moves to `Prax.Query` (its subject is
`CookedCondition`; both `Prax.Derive` and `Prax.Relevance` consume it — Relevance
importing Derive rules out leaving it where it is). Derive's axiom analyses re-type over
`[CookedRule]`; Relevance's world analyses re-type over `PraxState` (read the cooked
fields), with `retable` becoming two-stage (cook, then analyze the cooked state).
`setAxioms` already sets `cookedRules` before calling `retable`, so the ordering holds.

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- Exactness: classification-identical tables (Task 1's pins pass unchanged across the
  switch), goldens byte-identical, ViewInvariant green, full suite green. Any pin or
  golden movement = BLOCK with the trace (a transcription bug or a discovered walker
  drift — understood, never absorbed).
- Order is part of the pin: the old walkers emit `footprint`/`negFootprint`/`axiomHeads`
  entries in rule order (originals then □-lifted forms), per rule body-patterns-then-heads,
  per condition in authored order. `cookAxioms` produces the same rule order and
  `cookedReadAnchors` the same condition order, so the cooked computation reproduces the
  sequences verbatim — no sorting anywhere.
- The deletions are real: no wrapper, no re-export, no dual path. The v38/v40 boundary
  walkers (`conditionVars`/`outcomeVars`, `authoredVarClash`/`authoredPatClash`) and the
  string closure path (`run`, `closure`, `liftObliged` — the ViewInvariant reference
  implementation) explicitly STAY: authoring boundary = string; world model = cooked.
- Two known-benign semantic notes, stated so nobody "discovers" them mid-task:
  1. **Fn-pool bias.** The old analyses build their Call-resolution map with
     `Map.fromList` over all practices' functions (last practice wins on a duplicate
     name); the engine's `lookupCookedFn` is first-wins. The cooked pool helper uses
     `Map.unions` over `Map.elems` (first-wins) — ALIGNING the analysis with the
     operational semantics. Observable only under a cross-practice duplicate `fnName`;
     no shipped world has one (three functions total, all unique — the durable guard is
     v43's). The pins are the committed evidence of no behavior change.
  2. **Lifted heads in `axiomDerivable`.** [AMENDED after Task 2 review — the original
     note claimed a mere variable-spelling swap (`obliged.W.` → `obliged.Obligor.`);
     that was wrong.] Old `axiomDerivable` manufactured `obliged.W.<head>` for EVERY
     axiom head unconditionally; the cooked rules carry □-lifted forms only for liftable
     (all-Match-body) axioms, per `liftObliged`. For a non-liftable axiom the old
     derivable set held a spurious □-head the new set correctly lacks — a rule with no
     □-form cannot derive one, so the shrink is strictly more correct. Unobservable in
     shipped worlds: no want or gate candidate anywhere anchors on the `obliged`
     literal (grep-confirmed at review), and the 7-world pins held byte-identical.
- RED-first where behavior is new; Task 1's pins are observational (captured from the
  live current code, the goldens pattern) — their discriminating power is the switch
  itself. Zero warnings; hlint; `prax check` ×7 worlds.
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: The equivalence net — per-world analysis-table pins

**Files:**
- Create: `test/Prax/AnalysisTableSpec.hs`; register in `prax.cabal` other-modules and
  the suite's `Main`/`Spec` aggregator (same wiring as siblings).

**The renderer and harness (complete; the snapshot lists are CAPTURED, see process):**

```haskell
module Prax.AnalysisTableSpec (tests) where

import           Data.List (intercalate)
import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, (@?=))

import           Prax.Query (CookedCondition (..))
import           Prax.Sym (symName)
import           Prax.Types
import           Prax.Worlds.Audience (audienceWorld)
import           Prax.Worlds.Bar (barDirectorWorld, barWorld)
import           Prax.Worlds.Feud (feudWorld)
import           Prax.Worlds.Intrigue (intrigueWorld)
import           Prax.Worlds.Play (playWorld)
import           Prax.Worlds.Village (villageWorld)

-- | Render every derived analysis table a world state carries, one line per
-- entry, in the exact order the state holds them — order is part of the pin
-- (the v41 rewrite must reproduce the old walkers' emission order, not just
-- their sets). Sym paths render dot-joined via 'symName'. 'GateCheck' renders
-- its gates' 'CMatch' paths — the only shape 'Prax.Relevance.livenessOf'
-- emits; anything else crashes loudly here, deliberately.
analysisTable :: PraxState -> [String]
analysisTable st =
  [ "contMonotone: " ++ show (contMonotone st) ]
  ++ [ "improvable: " ++ n | n <- improvables st ]
  ++ [ "liveness: " ++ n ++ " " ++ renderL l
     | (n, l) <- Map.toList (liveness st) ]
  ++ [ "caresAbout: " ++ n ++ " -> " ++ intercalate "; " as
     | (n, as) <- Map.toList (caresAbout st) ]
  ++ [ "footprint: " ++ path p | p <- footprint st ]
  ++ [ "negFootprint: " ++ path p | p <- negFootprint st ]
  ++ [ "axiomHead: " ++ path p | p <- axiomHeads st ]
  where
    path = intercalate "." . map symName
    renderL l = case l of
      FloorCheck      -> "FloorCheck"
      AlwaysLive      -> "AlwaysLive"
      GateCheck gates -> "GateCheck " ++ intercalate " | " (map gate gates)
    gate [CMatch p] = path p
    gate g = error ("AnalysisTableSpec: unexpected gate shape: " ++ show g)

-- Captured from the live pre-v41 analyses (string-side walkers). These lines
-- ARE the analyses' contract across the v41 representation switch: the cooked
-- computation must reproduce every classification AND its order. Never edit
-- them to match new output — a failure means the rewrite is wrong.
tests :: TestTree
tests = testGroup "Prax.AnalysisTable"
  [ testCase "village"      $ analysisTable villageWorld     @?= villagePin
  , testCase "bar"          $ analysisTable barWorld         @?= barPin
  , testCase "bar-director" $ analysisTable barDirectorWorld @?= barDirectorPin
  , testCase "intrigue"     $ analysisTable intrigueWorld    @?= intriguePin
  , testCase "feud"         $ analysisTable feudWorld        @?= feudPin
  , testCase "audience"     $ analysisTable audienceWorld    @?= audiencePin
  , testCase "play"         $ analysisTable playWorld        @?= playPin
  ]
```

**Process:** write the renderer + a scratchpad capture program (imports the spec module
or duplicates the renderer verbatim in scratch; prints `analysisTable` per world) → run
it → paste each world's lines as the `<world>Pin :: [String]` definitions → suite GREEN
(observational pins on current code; a mismatch at this stage means the capture and the
test disagree — fix the capture, never the code) → gates → commit
`"v41 net: per-world analysis-table pins — the classifications the rewrite must preserve"`.

- [ ] Renderer + capture → pins pasted → suite green → gates → commit.

---

### Task 2: The switch — analyses onto the cooked form, string walkers deleted

**Files:**
- Modify: `src/Prax/Query.hs` (gains `cookedReadAnchors`), `src/Prax/Derive.hs`,
  `src/Prax/Relevance.hs`, `src/Prax/Engine.hs` (retable, relevantDelta, imports).
- Test updates: `test/Prax/RelevanceSpec.hs`, `test/Prax/DeriveSpec.hs`,
  `test/Prax/QuerySpec.hs` (receives the moved `cookedReadAnchors` test).

**2a. `Prax.Query`:** move `cookedReadAnchors` here verbatim (code + haddock, from
Relevance), exported. It needs only `CookedCondition` and `Sym` — both already here.

**2b. `Prax.Derive`:** delete `condPatterns`. Re-type the four axiom analyses over
`[CookedRule]` (complete replacements; export list unchanged otherwise):

```haskell
-- | Every path pattern the axioms can read or write: body atoms at any
-- polarity (including inside Absent\/Exists\/Or\/Subquery — the
-- 'Prax.Query.cookedReadAnchors' walk), and head templates. Cooked rules
-- already carry the auto-□-lifted forms ('cookAxioms'), so lifting needs no
-- second enumeration here. A ground delta that may-unify none of these
-- commutes with 'closure' (v27 spec theorem) — the basis of the engine's
-- delta-irrelevance fast path.
axiomFootprint :: [CookedRule] -> [[Sym]]
axiomFootprint rules =
  concat [ cookedReadAnchors (crBody r) ++ map (map fst) (crHeads r) | r <- rules ]

-- | Every pattern under a negation in any body: inserting a fact these
-- patterns match can UN-fire a rule (retraction), so such facts never take
-- the continuation tier.
axiomNegPatterns :: [CookedRule] -> [[Sym]]
axiomNegPatterns rules = concat [ concatMap negOf (crBody r) | r <- rules ]
  where
    negOf c = case c of
      CNot p          -> [p]
      CAbsent cs      -> cookedReadAnchors cs   -- everything inside a ¬∃
      CExists cs      -> concatMap negOf cs
      COr clauses     -> concatMap (concatMap negOf) clauses
      CSubquery _ _ w -> concatMap negOf w
      _               -> []

-- | Every head template the axioms can write — □-lifted forms included, for
-- free, since 'cookAxioms' already emitted them as rules of their own. A
-- delta that feeds some axiom can change derived facts only in these
-- families.
axiomHeadPatterns :: [CookedRule] -> [[Sym]]
axiomHeadPatterns rules = concat [ map (map fst) (crHeads r) | r <- rules ]

-- | Is the axiom set continuation-safe: does adding base facts only ever ADD
-- derived facts (given the caller also avoids negated patterns)? Conditions
-- must be monotone-up: CMatch/CNot/CAbsent (negations are handled via
-- 'axiomNegPatterns'), recursion through CExists/COr/CSubquery, CCount
-- freely, CCmp only in the grows-only direction — the count side growing
-- past a numeric literal (Gt/Gte with the literal right, Lt/Lte with it
-- left) — and CEq/CNeq only over pattern-bound variables. An Eq/Neq over an
-- aggregate-bound variable (a 'CCount' result or a 'CSubquery' set variable)
-- expresses exactly-k/not-k, which UN-fires as the aggregate grows past k —
-- anti-monotone despite Eq/Neq otherwise being a safe equality test. CCalc
-- (and any other CCmp shape) disables the tier for the world; the fallback
-- is today's full reclose, correct just slower.
monotoneAxioms :: [CookedRule] -> Bool
monotoneAxioms rules = all (bodyOk . crBody) rules
  where
    bodyOk body = all (condOk (aggVars body)) body

    -- Every variable bound by an aggregate anywhere in the body (a body
    -- shares one binding environment, so a CCount/CSubquery result nested
    -- under CExists/COr/CSubquery is still visible to an Eq/Neq elsewhere
    -- in the body).
    aggVars = concatMap collect
      where
        collect c = case c of
          CCount r _      -> [r]
          CSubquery s _ w -> s : aggVars w
          CExists cs      -> aggVars cs
          COr clauses     -> concatMap aggVars clauses
          _               -> []

    condOk aggs c = case c of
      CMatch _        -> True
      CNot _          -> True
      CAbsent _       -> True
      CEq l r         -> l `notElem` aggs && r `notElem` aggs
      CNeq l r        -> l `notElem` aggs && r `notElem` aggs
      CCount _ _      -> True
      CExists cs      -> all (condOk aggs) cs
      COr clauses     -> all (all (condOk aggs)) clauses
      CSubquery _ _ w -> all (condOk aggs) w
      CCmp op l r     -> case op of
        Gt  -> numeric r
        Gte -> numeric r
        Lt  -> numeric l
        Lte -> numeric l
      CCalc {}        -> False
    numeric x = let s = symName x
                in not (null s) && all (`elem` ("0123456789" :: String)) s
```

Imports: `Prax.Query` gains `cookedReadAnchors` in Derive's import list; `Prax.Sym`
gains `symName`. `liftObliged`/`run`/`closure` untouched (the string reference path).

**2c. `Prax.Relevance`:** delete `mayUnify`, `wantPatterns`, `outcomeAtoms`,
`evictionShadows` (string), and the local `cookedReadAnchors` (moved). Export list
becomes `mayUnifySyms, improvableDesires, livenessOf, bearingTemplates,
evictionShadowNames, moverReadAnchors`. Module header: the conservativity contract and
the entity-names invariant stay, with `mayUnify` references retargeted to
`mayUnifySyms`; add the boundary sentence — __authoring boundary = string
('Prax.Types.conditionVars' and the v40 guards, which run before cooking exists); world
model = cooked (everything here)__. `mayUnifySyms` absorbs `mayUnify`'s haddock (the
evidence-free-overlap/anchored-literal explanation and where the invariant is spent —
merged with its existing planner-hot-form paragraph). New/changed definitions
(complete):

```haskell
-- | The Call-resolution pool: every function's cooked case outcomes, guards
-- ignored (conservatively: all cases), unioned across practices first-wins
-- in practice order — the same resolution order as
-- 'Prax.Engine.lookupCookedFn' (observable only under a cross-practice
-- duplicate 'fnName', which no shipped world has; v43's guard makes one
-- impossible).
cookedFnPool :: Map String CookedPractice -> Map String [CookedOutcome]
cookedFnPool defs =
  Map.unions [ Map.map (concatMap snd . snd) (cpFns cp) | cp <- Map.elems defs ]

-- The insert- and delete-shaped atoms an outcome can produce, resolving
-- 'CCall's through the pool (conservatively: all cases). An exclusion insert
-- both asserts its path and evicts that value's __siblings__ — whose names
-- appear nowhere in the outcome — so the delete side carries the path itself
-- plus its eviction shadows ('evictionShadowNames': conservatively, any
-- sibling). Returns Nothing for "unknown effects" (unresolvable 'CCall'):
-- the caller must treat that as improves-everything.
cookedOutcomeAtoms :: Map String [CookedOutcome] -> [String] -> CookedOutcome
                   -> Maybe ([[Sym]], [[Sym]])
cookedOutcomeAtoms fns visited o = case o of
  CInsert toks
    | any ((== Just '!') . snd) toks
                  -> Just ([map fst toks], map fst toks : evictionShadowNames toks)
    | otherwise   -> Just ([map fst toks], [])
  CDelete toks    -> Just ([], [map fst toks])
  CForEach _ outs -> mconcat' (map (cookedOutcomeAtoms fns visited) outs)
  CCall fn _
    | fn `elem` visited -> Just ([], [])           -- cycle: already counted
    | otherwise -> case Map.lookup fn fns of
        Nothing   -> Nothing                       -- unknown function: wild
        Just outs -> mconcat' (map (cookedOutcomeAtoms fns (fn : visited)) outs)
  where
    mconcat' ms = do
      pairs <- sequence ms
      pure (concatMap fst pairs, concatMap snd pairs)

-- Positive and negated path patterns of a want's cooked conditions. The Bool
-- is "uncertain": the want's satisfaction depends on machinery (numeric
-- binds, counts, subqueries) beyond pattern presence. Deliberately NOT
-- 'Prax.Query.cookedReadAnchors': subquery internals are machinery here
-- (taint), not read anchors — the two walks answer different questions.
cookedWantPatterns :: [CookedCondition] -> ([[Sym]], [[Sym]], Bool)
cookedWantPatterns = foldr step ([], [], False)
  where
    step c (pos, neg, unc) = case c of
      CMatch p     -> (p : pos, neg, unc)
      CNot p       -> (pos, p : neg, unc)
      CAbsent cs   -> let (p', n', u') = cookedWantPatterns cs
                      in (pos ++ n', neg ++ p', unc || u')
      CExists cs   -> let (p', n', u') = cookedWantPatterns cs
                      in (pos ++ p', neg ++ n', unc || u')
      COr clauses  -> let parts = map cookedWantPatterns clauses
                      in ( pos ++ concatMap (\(p', _, _) -> p') parts
                         , neg ++ concatMap (\(_, n', _) -> n') parts
                         , unc || any (\(_, _, u') -> u') parts )
      CEq {}       -> (pos, neg, unc)
      CNeq {}      -> (pos, neg, unc)
      CCmp {}      -> (pos, neg, unc)
      CCalc {}     -> (pos, neg, True)
      CCount {}    -> (pos, neg, True)
      CSubquery {} -> (pos, neg, True)

data AtomPools = AtomPools
  { poolInserted :: [[Sym]]
  , poolDeleted  :: [[Sym]]
  , poolWild     :: Bool
  }

worldAtomPools :: Map String CookedPractice -> AtomPools
worldAtomPools allDefs = AtomPools
  { poolInserted = concatMap (maybe [] fst) atoms
  , poolDeleted  = concatMap (maybe [] snd) atoms
  , poolWild     = Nothing `elem` atoms
  }
  where
    -- (keep the existing drifter-exclusion comment verbatim)
    defs = Map.delete driftPracticeId allDefs
    practices = Map.elems defs
    fns = cookedFnPool defs
    atoms = [ cookedOutcomeAtoms fns [] o
            | cp <- practices, a <- cpActions cp, o <- caOuts a ]
         ++ [ cookedOutcomeAtoms fns [] o | cp <- practices, o <- cpInits cp ]

-- Axiom heads count as derivable: a want (or gate candidate) over a
-- derivable pattern is conservatively improvable\/never a gate. □-lifted
-- heads arrive as rules of their own ('Prax.Derive.cookAxioms'). Shared by
-- 'improvableDesires' and 'livenessOf'.
axiomDerivable :: [CookedRule] -> [Sym] -> Bool
axiomDerivable rules p = any (mayUnifySyms p) heads
  where heads = [ map fst h | r <- rules, h <- crHeads r ]

-- | The names of the desires some authored action might improve. Reads the
-- state's cooked tables ('cookedDefs'\/'cookedRules'\/'cookedDesires' —
-- 'Prax.Engine.retable' cooks before it analyzes). See the module header
-- for the conservativity contract.
improvableDesires :: PraxState -> [String]
improvableDesires st =
  [ desireName d | d <- desires st, improvable d ]
  where
    pools = worldAtomPools (cookedDefs st)
    wild = poolWild pools
    inserted = poolInserted pools
    deleted  = poolDeleted pools
    derivable = axiomDerivable (cookedRules st)
    improvable d@(Desire _ (Want _ u))
      | u == 0    = False
      | wild      = True
      | unc       = True
      | any derivable (pos ++ neg) = True
      | u > 0     = any (\i -> any (mayUnifySyms i) pos) inserted
                    || any (\dl -> any (mayUnifySyms dl) neg) deleted
      | otherwise = any (\dl -> any (mayUnifySyms dl) pos) deleted
                    || any (\i -> any (mayUnifySyms i) neg) inserted
      where (pos, neg, unc) = cookedWantPatterns (cookedDesires st Map.! desireName d)

livenessOf :: PraxState -> Map String Liveness   -- (haddock as today)
livenessOf st =
  Map.fromList [ (desireName d, classify d) | d <- desires st ]
  where
    pools = worldAtomPools (cookedDefs st)
    derivable = axiomDerivable (cookedRules st)
    classify d@(Desire _ (Want _ u))
      | u < 0     = FloorCheck
      | u > 0     = positive (cookedDesires st Map.! desireName d)
      | otherwise = AlwaysLive
    positive conds
      | unc || poolWild pools || null gates = AlwaysLive
      | otherwise = GateCheck gates
      where
        (_, _, unc) = cookedWantPatterns conds
        candidates = [ p | CMatch p <- conds ]
        qualifies p = not (any (mayUnifySyms p) (poolInserted pools))
                    && not (derivable p)
        gates = [ [CMatch p] | p <- candidates, qualifies p ]

bearingTemplates :: PraxState -> Map String [String]   -- (haddock as today)
bearingTemplates st =
  Map.fromList [ (charName c, bearing (charPats c)) | c <- characters st ]
  where
    defs = cookedDefs st          -- NO drifter exclusion here (v37 scoped it
                                  -- to the pools; bearing keeps every action)
    fns = cookedFnPool defs
    actionAtoms = [ (caName a, atoms a)
                  | cp <- Map.elems defs, a <- cpActions cp ]
    atoms a = do
      pairs <- traverse (cookedOutcomeAtoms fns []) (caOuts a)
      pure (concatMap fst pairs ++ concatMap snd pairs)
    charPats c = cookedReadAnchors
      (  concat (Map.findWithDefault [] (charName c) (cookedWants st))
      ++ [ cc | d <- desires st, desireName d `elem` charDesires c
              , cc <- cookedDesires st Map.! desireName d ] )
    bearing pats =
      [ n | (n, m) <- actionAtoms
          , case m of
              Nothing -> True
              Just as -> any (\atom -> any (mayUnifySyms atom) pats) as ]
```

The `Map.!` on `cookedDesires` is a deliberate loud guard: `retable` builds that table
from the same `desires` list in the same call — a miss is an engine invariant violation,
not a recoverable state. Imports: drop `Prax.Derive (Axiom (..))` for
`Prax.Derive (CookedRule (..))`; drop `internTokens`/`tokensToSentence` from the Db
import if now unused; `cookedReadAnchors` now imported from `Prax.Query`.
`moverReadAnchors` and `evictionShadowNames` are untouched.

**2d. `Prax.Engine`:** `retable` two-stage, `relevantDelta` off the string shadows:

```haskell
retable :: PraxState -> PraxState
retable st0 =
  let st = st0
        { cookedDefs    = Map.map cookPractice (practiceDefs st0)
        , cookedWants   = Map.fromList
            [ (charName c, [ map cookCondition (wantConditions w) | w <- charWants c ])
            | c <- characters st0 ]
        , cookedDesires = Map.fromList
            [ (desireName d, map cookCondition (wantConditions (desireWant d)))
            | d <- desires st0 ] }
  in st
     { improvables  = improvableDesires st
     , caresAbout   = bearingTemplates st
     , liveness     = livenessOf st
     , footprint    = axiomFootprint (cookedRules st)
     , axiomHeads   = axiomHeadPatterns (cookedRules st)
                      ++ [[intern "contradiction"]]
     , negFootprint = axiomNegPatterns (cookedRules st)
     , contMonotone = monotoneAxioms (cookedRules st) }

relevantDelta :: String -> PraxState -> Bool
relevantDelta s = relevantNames (map fst toks) (evictionShadowNames toks)
  where toks = internTokens s
```

(Retable's haddock keeps its "axioms is the one exception" note — now stating the
analyses READ `cookedRules`, which `setAxioms` sets before calling here.) Imports: drop
`evictionShadows`; ensure `internTokens` is imported; remove any now-unused `pathNames`
use in retable (other Engine uses stay). The `Prax.Derive` import list keeps the same
names (their types changed, not their names).

**2e. Tests:**
- `RelevanceSpec`: the three `mayUnify` cases become `mayUnifySyms` via a local
  `u a b = mayUnifySyms (map intern (pathNames a)) (map intern (pathNames b))` — same
  fixtures, same assertions, test names updated to say `mayUnifySyms`. Every
  `improvableDesires (practiceDefs w) (axioms w) (desires w)` call becomes
  `improvableDesires w` (same for `livenessOf`); the two "field matches the module
  computation" assertions become `improvables villageWorld == improvableDesires
  villageWorld` (resp. `liveness`/`livenessOf`). The synthetic-fixture cases (shrine/
  temple/bakery/hates-mud/...) build their states through the public setters already
  imported (`definePractices`/`setDesires`/`setAxioms` on `emptyState`-shaped input, as
  the file's existing idiom does) and assert the same expected tables. The
  `cookedReadAnchors` test MOVES to `QuerySpec` (function moved; import from
  `Prax.Query`), verbatim.
- `DeriveSpec`: `axiomFootprint`/`axiomNegPatterns` cases wrap their axiom lists in
  `cookAxioms` and check membership via a local `has fp s = map intern (pathNames s)
  `elem` fp` — the same sentences asserted present/absent. (If an existing expectation
  names a lifted body pattern with the old `obliged.W.` spelling, the cooked output
  spells it `obliged.Obligor.` — update the LITERAL, not the meaning; both are
  variables. Flag it in the report if hit.)

**Process:** suite must be green at every commit; the REAL gate is Task 1's pins +
goldens byte-identical + ViewInvariant, all unchanged. Then: zero warnings; hlint;
`prax check` ×7; `grep -rn "mayUnify\b\|wantPatterns\|outcomeAtoms\|condPatterns\|evictionShadows\b" src/`
returns nothing (deletion proof). Commit
`"One analysis surface: the world-model analyses read the cooked form"`.

- [ ] Query move → Derive re-type → Relevance re-type → Engine → tests migrated →
      suite + pins + goldens unchanged → deletion grep clean → gates → commit.

---

### Task 3: Docs

**Files:** `docs/LEDGER.md`.

- [ ] The v41 legend row (house style): the queue context (second of four foundations
  passes); the split-walker defect and the one-surface rule (authoring boundary =
  string, world model = cooked); the free lifting (cooked rules already carry □-forms —
  three duplicate lift enumerations deleted); the two benign notes (fn-pool bias now
  matches `lookupCookedFn`; `axiomDerivable`'s spurious □-heads for non-liftable axioms
  dropped — a strictly-more-correct shrink, not the spelling swap the plan first
  claimed); the
  equivalence evidence (7-world analysis-table pins laid before the switch, unchanged
  across it; goldens byte-identical); suite count as measured. Mark the queue item done;
  queue pointer to v42 (dead-condition lint — the first new analysis on the unified
  surface).
- [ ] Gates; commit `"Docs: v41 — one surface"`.
