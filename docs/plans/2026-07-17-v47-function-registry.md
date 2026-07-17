# v47 — Function Registry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

## The problem, and why each change exists

`Function`s can only live on a `Practice`, so the core-model library ships as
`coreLib` — a phantom never-instantiated practice every world registers, every
analysis folds over, and v43's trailing-dot bug already bit through. Function
resolution is ALREADY global (first-wins search, bare-name `Call`s, v43's global
uniqueness guard): practice-locality is fiction. So `Practice.functions` is DELETED —
not supplemented (two homes would be the dual-system ban) — and `PraxState` gains the
one registry. Every change below is either the field's deletion, the registry's
addition, or a consumer re-plumbing between them.

**Goal:** per `docs/specs/2026-07-17-v47-function-registry.md`.

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- Goldens and AnalysisTable pins BYTE-IDENTICAL (the phantom had no actions/roles/
  instances; no pin row names it). Any movement = BLOCK and trace.
- One home: no re-export of `coreLib`, no compatibility shim, no `functions` field
  survivor. Deaths grep-proof.
- RED-first for the new guards; the v43 collision pins re-express (same
  discriminating scenarios, registry home); behavioral pins (Bar drinking arithmetic,
  Core score/bond) pass UNTOUCHED — they are the migration's correctness evidence.
- Zero warnings; hlint; `prax check` ×7; suite green.
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: The registry

**Files:** `src/Prax/Types.hs`, `Cooked.hs`, `Engine.hs`, `Relevance.hs`,
`TypeCheck.hs`, `Core.hs`, `Script.hs`, `Worlds/*.hs` (every `coreLib` site + Bar's
trio), tests: `EngineSpec`, `RelevanceSpec`, `TypeCheckSpec`, `CoreSpec`/`BarSpec`
(behavioral pins untouched), `AnalysisTableSpec` (byte-identical), `prax.cabal` if
modules change.

**1a. Types:** `Practice.functions` DELETED (and from the `practice` default);
`CookedPractice.cpFns` DELETED. `PraxState` gains:

```haskell
  , worldFns  :: [Function]                 -- the authored registry (build-time vocabulary)
  , cookedFns :: Map String ([String], [([CookedCondition], [CookedOutcome])])
      -- ^ cooked once by 'Prax.Engine.defineFunctions'; the ONE home 'Call'
      -- resolution reads ('lookupCookedFn'). Keyed by 'fnName'; uniqueness is
      -- the setter's loud guard, so the Map never silently collapses a duplicate.
```

`emptyState` seeds both empty.

**1b. Engine:**

```haskell
-- | Register the world's functions (the one home — 'Practice' carries no
-- functions since v47; practice-locality was fiction: resolution was always
-- global). Loud on a duplicate 'fnName', within the batch or against the
-- already-registered set (v43's two per-practice collision arms collapse
-- into this one check).
defineFunctions :: [Function] -> PraxState -> PraxState
```

— cooks into `cookedFns` (reuse `Prax.Cooked`'s function-cooking, re-homed);
`retable` NOT required (functions carry no analysis tables of their own — but
`producibleAtoms`/pools read `cookedFns`, so `defineFunctions` must retable for the
lint's sake: check what consumes and call `retable`). `lookupCookedFn` = `Map.lookup
fn (cookedFns st)` — the practice fold and its first-wins comment DIE.
`definePractice`'s fn-collision arms DIE (nothing carries functions).

**1c. Cooked:** `cookPractice` stops cooking functions (the zero-role `cpInstanceNames`
comment citing coreLib updates — the phantom is gone, the direct-construction fix
stays on its own merits).

**1d. Relevance/TypeCheck:** `cookedFnPool` reads `cookedFns` directly (one arg —
callers adjust); the v41 bias footnote dies. TypeCheck: `unboundInPractice`'s `fn'`
walk becomes a registry-level `unboundInFunction` walk (site label `"fn <name>"`);
`refErrors`' `definedFns` = registry keys; `assertedSentences`' fn-insert
comprehension reads the registry; DeadCondition `lintSites`' and reserved-family
sites' fn entries likewise (labels drop the phantom prefix).

**1e. Core/Script/Worlds:** `coreLib` DELETED; `Core` exports `coreFns ::
[Function]` = the same two, unchanged. Every `definePractices [coreLib, …]` site
(Script.compile, Intrigue, Bar, Village — grep for the complete list) becomes the
honest practice list + `defineFunctions coreFns`. Bar's `tendBar` trio moves out of
the practice record into Bar's `defineFunctions (coreFns ++ [recordDrinkFn, …])`
call (name the three as top-level values, mirroring Core's style).

**1f. Pins:** registry-uniqueness RED-first (duplicate in one batch; across two
calls); `Call` to an unregistered name still `UndefinedRef`; unbound-in-function at
the new label; the v43 collision pins re-expressed; behavioral pins untouched-green;
deaths grep-proof (`coreLib|cpFns|functions =` + `lookupCookedFn`'s fold).

- [ ] RED → registry → consumer re-plumb → migration → GREEN → suite byte-identical
      outside re-expressed pins → gates → commit
      `"The function registry: one home for functions; the phantom practice dies"`.

---

### Task 2: Docs

**Files:** `docs/LEDGER.md`, plus README/WALKTHROUGH sweep for `coreLib`/"core model
library"/phantom-practice mentions (the v44 lesson).

- [ ] The v46-style legend row: the audit finding closed (C1); the probe's sharpening
  (locality was fiction → field deletion, not registry-beside-field); the deletions
  ledger (field, cpFns, phantom, two guard arms, the resolution-order subtlety and
  the v41 bias footnote); byte-identical exactness held; suite counts. Queue pointer
  to v48 (the last).
- [ ] README/WALKTHROUGH sweeps; gates; commit `"Docs: v47 — one home for functions"`.
