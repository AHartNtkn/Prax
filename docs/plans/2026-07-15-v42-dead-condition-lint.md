# v42 — Dead-Condition Lint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** per `docs/specs/2026-07-15-v42-dead-condition-lint.md` — a new `Prax.TypeCheck`
check flagging positive conjuncts no producer can ever match, built on the v41 cooked
surface (`producibleAtoms` exported from `Prax.Relevance`, `mayUnifySyms` as the
matcher). No engine, planner, or serialization change; goldens byte-identical.

**Architecture:** one new Relevance export (the producer pool, sharing
`cookedFnPool`/`cookedOutcomeAtoms`), one new `TypeError` case + check in TypeCheck, one
`describe` line in the CLI. The lint scans affordance/motive sites only (spec's
probe-decided scope); axiom bodies excluded.

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- The checker's charter binds: __no false positives__. If ANY shipped world flags under
  the finished lint, that is a BLOCK-and-report, not a pin to edit — either the site
  scope needs adjudication (a spec question) or it is a genuine world bug (also a spec
  question). The controller decides; the implementer never widens the pool or narrows
  the scan to make a world pass silently.
- Conservativity direction: everything uncertain silences the lint (wild world → no
  DeadCondition; axiom heads count as producers whether or not their rules can fire; an
  UNANCHORED pattern — every segment a variable — matches everything and is never dead).
- RED-first: the new spec cases must be OBSERVED failing before the check is wired into
  `typeCheck`'s concat (implement the type + check function, run the new cases, watch
  them fail, then wire and watch them pass). Two mutations after GREEN (named below).
- Goldens byte-identical, ViewInvariant green, full suite green (547 + new), zero
  warnings, hlint, `prax check` ×7 all "well-formed".
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: The lint

**Files:**
- Modify: `src/Prax/Relevance.hs` (export `producibleAtoms`), `src/Prax/TypeCheck.hs`,
  `app/Main.hs`, `test/Prax/TypeCheckSpec.hs`.

**1a. `Prax.Relevance` (complete; `dbToSentences` joins the Db import):**

```haskell
-- | Everything the registered world can ever contain, as pattern anchors:
-- the initial db's facts, every practice's insert-side atoms
-- ('cookedOutcomeAtoms'\'s insert half over ALL practices — the drifter
-- INCLUDED, unlike 'worldAtomPools': the consumer asks "can this fact ever
-- exist", and clock-moved facts exist), every axiom head ('cookedRules'\'
-- 'crHeads', □-lifted forms included — heads count regardless of whether
-- their rules can fire; conservative, which here only ever silences the
-- consumer), and the engine's own @contradiction@ witness
-- ('Prax.Engine.reclose' inserts it at ⊥). @Nothing@ = wild (an
-- unresolvable 'CCall'): the caller must go silent. First consumer:
-- "Prax.TypeCheck"'s dead-condition lint
-- (spec @docs/specs/2026-07-15-v42-dead-condition-lint.md@).
producibleAtoms :: PraxState -> Maybe [[Sym]]
producibleAtoms st = do
  pairs <- sequence ( [ cookedOutcomeAtoms fns [] o
                      | cp <- practices, a <- cpActions cp, o <- caOuts a ]
                   ++ [ cookedOutcomeAtoms fns [] o
                      | cp <- practices, o <- cpInits cp ] )
  pure ( concatMap fst pairs
      ++ [ map intern (pathNames s) | s <- dbToSentences (db st) ]
      ++ [ map fst h | r <- cookedRules st, h <- crHeads r ]
      ++ [[intern "contradiction"]] )
  where
    practices = Map.elems (cookedDefs st)
    fns = cookedFnPool (cookedDefs st)
```

(Function-case outcomes contribute only via a reachable `CCall`, same as
`worldAtomPools` — an uncalled function's inserts are correctly not producible.)

**1b. `Prax.TypeCheck` (new case + check 7; imports gain
`Prax.Relevance (mayUnifySyms, producibleAtoms)`, `Prax.Query (CookedCondition (..))`,
`Prax.Sym (symName, symIsVar)`, and the cooked types from `Prax.Types` are already in
scope via its import):**

New `TypeError` case (with the others):

```haskell
  | DeadCondition { teWhere :: String, teSentence :: String }
    -- ^ the positive pattern @teSentence@ (at @teWhere@) may-unifies nothing
    -- the world can ever contain: the site can never fire.
```

`typeCheck` gains `++ deadConditionErrors st` at the end of its concat.

```haskell
-- Check 7: dead conditions ----------------------------------------------------

-- A positive Match conjunct whose pattern can never match anything the world
-- can ever contain ('Prax.Relevance.producibleAtoms') makes its site
-- unreachable: the action, function case, ForEach effect, or want silently
-- never fires — the unambiguous-bug class. Scanned sites are affordances and
-- motives only; axiom bodies are deliberately OUT of scope (a rule library
-- included wholesale routinely leaves some rules inert — feud's kinAxioms,
-- documented deliberate in the fixture — and an unfireable rule is
-- harmless). Flagged positions are conjunctive positives only: top level and
-- inside Exists (a dead positive there kills the Exists, killing the site).
-- NOT flagged: negations (vacuously true is plausible defensive authoring),
-- Or clauses (a dead clause doesn't kill the Or), Subquery interiors (an
-- always-empty set can be a Count comparison's intended meaning), and
-- unanchored patterns (every segment a variable: matches everything —
-- 'mayUnifySyms' would discard the evidence-free overlap, so without this
-- exemption the lint would flag exactly the patterns that can never be
-- dead). A wild world (unresolvable Call) silences the lint entirely.
deadConditionErrors :: PraxState -> [TypeError]
deadConditionErrors st = case producibleAtoms st of
  Nothing   -> []
  Just pool ->
    [ DeadCondition loc (intercalate "." (map symName p))
    | (loc, conds) <- lintSites st
    , p <- concatMap positives conds
    , any (not . symIsVar) p
    , not (any (mayUnifySyms p) pool) ]
  where
    positives c = case c of
      CMatch p   -> [p]
      CExists cs -> concatMap positives cs
      _          -> []

-- The affordance/motive sites the lint scans, with author-legible labels:
-- action conditions, ForEach guards (recursively — action outcomes, init
-- outcomes, function-case outcomes), function-case conditions, desires,
-- character wants.
lintSites :: PraxState -> [(String, [CookedCondition])]
lintSites st =
     [ (pid ++ " / " ++ caName a, caConds a)
     | (pid, cp) <- defs, a <- cpActions cp ]
  ++ [ (pid ++ " / " ++ caName a ++ " (effect guard)", gs)
     | (pid, cp) <- defs, a <- cpActions cp, gs <- forEachGuards (caOuts a) ]
  ++ [ (pid ++ " (init guard)", gs)
     | (pid, cp) <- defs, gs <- forEachGuards (cpInits cp) ]
  ++ [ (pid ++ " / fn " ++ fn, cs)
     | (pid, cp) <- defs, (fn, (_, cases)) <- Map.toList (cpFns cp)
     , (cs, _) <- cases ]
  ++ [ (pid ++ " / fn " ++ fn ++ " (effect guard)", gs)
     | (pid, cp) <- defs, (fn, (_, cases)) <- Map.toList (cpFns cp)
     , (_, os) <- cases, gs <- forEachGuards os ]
  ++ [ ("desire " ++ n, cs) | (n, cs) <- Map.toList (cookedDesires st) ]
  ++ [ ("want of " ++ n, cs)
     | (n, css) <- Map.toList (cookedWants st), cs <- css ]
  where defs = Map.toList (cookedDefs st)

forEachGuards :: [CookedOutcome] -> [[CookedCondition]]
forEachGuards outs = concat [ conds : forEachGuards os | CForEach conds os <- outs ]
```

Module header: the check list gains one bullet (dead conditions), phrased like the
existing three, noting it is the first check on the v41 cooked surface.

**1c. `app/Main.hs`** — one `describe` case, same voice as its siblings:

```haskell
    describe (DeadCondition w s) =
      "dead condition \"" ++ s ++ "\" (" ++ w
        ++ "): no action, initial fact, or axiom head can ever produce a match"
```

**1d. `test/Prax/TypeCheckSpec.hs`** — new cases, built in the file's existing fixture
idiom (see the unbound-variable case's `practice`/`action` shapes; every fixture below
must be otherwise well-formed so ONLY the lint speaks). Semantic content is binding;
mirror the file's constructors exactly:

1. **Dead action conjunct**: a practice whose action reads `Match "tresure.spot"` while
   its init inserts `treasure.spot` (the typo class). Expect exactly
   `[ DeadCondition "<pid> / <action>" "tresure.spot" ]`.
2. **Corrected twin**: same fixture with the spelling fixed → `[]`.
3. **Dead positive inside Exists**: the same dead pattern under
   `Exists [ Match "tresure.spot" ]` → flags with the same rendering.
4. **Dead ForEach guard**: an action whose outcome is
   `ForEach [ Match "tresure.spot" ] [ Insert "found" ]` → flags at
   `"<pid> / <action> (effect guard)"`.
5. **Dead desire / dead char want**: a `setDesires` desire and a `charWants` want over a
   never-produced family → flag at `"desire <name>"` / `"want of <char>"`.
6. **Negation not flagged**: `Not "ghost.Actor"` (nothing produces `ghost.*`) → `[]`.
7. **Half-dead Or not flagged**: `Or [[Match "tresure.spot"], [Match "treasure.spot"]]`
   → `[]`.
8. **Subquery interior not flagged**: a `Subquery` whose inner `Match` is dead, used
   with `Count`/`Cmp` → no `DeadCondition`.
9. **Unanchored pattern not flagged**: an action condition `Match "X.Y"` (with both
   variables bound elsewhere so the unbound check stays quiet — e.g. a preceding
   anchored `Match` binds them) → no `DeadCondition`.
10. **Wild world silent**: an action whose outcome `Call`s an undefined function AND
    whose condition is the dead pattern → the `UndefinedRef` fires, `DeadCondition`
    does NOT.
11. **Dead axiom body not flagged (synthetic)**: `setAxioms [ axiom [ Match
    "parent.P.C" ] [ "kin.P.C" ] ]` on a world producing no `parent.*` → no
    `DeadCondition` (and note the REAL pin: feud, kinAxioms wholesale, in the
    all-worlds case).
12. **The all-shipped-worlds pin** (`typeCheck w @?= []`) gains `audienceWorld` — it is
    missing from the existing list; the probe showed it clean. Add a comment on the
    village line: its `drawn-to-market` desire reads `marketDay.square`, which only the
    drifter inserts — the pin holds because the lint's pool INCLUDES the drift practice.

**Process:** write 1d's cases FIRST with the `DeadCondition` type + check function
implemented but NOT wired into `typeCheck` → run the new cases, OBSERVE the expectation
failures (RED: fixtures expecting `DeadCondition` get `[]`) → wire `++
deadConditionErrors st` → GREEN → two mutations, each observed failing exactly its pin:
(m1) drop the `CExists` recursion in `positives` → case 3 fails; (m2) drop the db-facts
line from `producibleAtoms` → the all-worlds pin fails (initial facts become
"unproducible" and legitimate conditions flag). Revert both → full suite + goldens →
gates (zero warnings; hlint; `prax check` ×7) → commit
`"Dead-condition lint: flag what the world can never satisfy"`.

- [ ] RED observed (unwired check) → wire → GREEN → mutations m1/m2 → suite + nets →
      gates → commit.

---

### Task 2: Docs

**Files:** `docs/LEDGER.md`.

- [ ] The v42 legend row (house style): third foundations pass; the probe-decided scope
  (feud's wholesale `kinAxioms` and the □-lifted bodies drew the axiom-body exclusion —
  cite the fixture's own "harmless" haddock); the first-new-analysis-on-the-v41-surface
  point (written once, cooked-only, no string walker returned); the conservativity
  ledger (wildness silences, heads count fireable-or-not, unanchored patterns exempt);
  the engine's `contradiction` witness in the pool; suite count as measured. Mark the
  queue item done; queue pointer to v43 (the hygiene bundle, last of the four).
- [ ] Gates; commit `"Docs: v42 — the lint that reads the one surface"`.
