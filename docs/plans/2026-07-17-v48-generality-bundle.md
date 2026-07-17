# v48 — Generality Bundle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

## The problem, and why each change exists

v15 baked Deontic's `obliged.Obligor.*` vocabulary into the general closure engine:
every axiom-bearing world's rules are □-lifted unconditionally, whether the world can
ever produce an obligation or not. Corrected census (the panel caught the first
draft's error through three independent lenses): Village and Feud carry lifted rules
today; Village genuinely produces `obliged.*` (comply → owe → oblige) and must KEEP
them (DEON property 1); Feud cannot, and its 8 lifted rows are provably-unfireable
tax. Items 2-5 are the same defect class at module level: a hardcoded verb, a
hardcoded coverage family, content in a mechanism module, a dead alias.

**Goal:** per `docs/specs/2026-07-17-v48-generality-bundle.md` (panel-reviewed,
rewritten 2562299 — its item-1 precisions and the v49 graduation are binding).

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- Exactness: exactly ONE pin changes in the whole round — `feudPin` loses its 8
  lifted rows (footprint ×6, axiomHead ×2), itemized. `villagePin` BYTE-IDENTICAL
  (Village lifts before and after — stripping its rows is the exact wrong move the
  panel warned about). All goldens byte-identical everywhere. Any other movement =
  BLOCK and trace.
- The panel's item-1 precisions are law: NO `producibleAtoms` reuse (self-read +
  self-fulfilling); the db leg reads retable-time facts (build-order invariant
  documented at `setAxioms`); the string path (`Derive.run`) stays ungated BY DESIGN
  with ViewInvariant stated as the gate's soundness net; setter-coherence pinned per
  setter.
- RED-first; suite green per commit; zero warnings; hlint; `prax check` ×7.
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: The lifting gate

**Files:** `src/Prax/Derive.hs`, `src/Prax/Engine.hs`, `src/Prax/Types.hs` (only if a
field comment moves), tests: `DeriveSpec`, `EngineSpec`, `AnalysisTableSpec` (feudPin),
`ViewInvariantSpec` (comment), new gate pins wherever `retable` pins live.

**1a. Derive:** `cookAxioms` gains the gate as an explicit parameter — the DECISION
lives in the caller (`retable`), the MECHANISM here:

```haskell
-- | Precompile axioms. @lift@ decides whether the auto-□-lifted forms are
-- included (spec v48): the engine lifts exactly when the world can produce
-- an @obliged.*@ fact — DEON property 1 for worlds that can invoke it, no
-- doubled rule set for worlds that cannot. The STRING path ('run'/'closure')
-- always lifts and is deliberately ungated: it has no producer pool, and its
-- unconditional lifting makes "Prax.ViewInvariantSpec" the gate's soundness
-- net — if the gate ever wrongly skips a producible world, the gated cooked
-- view diverges from the ungated reference and the net fires.
cookAxioms :: Bool -> [Axiom] -> [CookedRule]
cookAxioms lift axs =
  [ CookedRule (map cookCondition body) (map internTokens hs)
  | Axiom body hs <- axs ++ (if lift then mapMaybe liftObliged axs else []) ]
```

The `obliged` head literal gets ONE home in Derive (a constant shared by
`liftObliged`'s prefix and exported for the gate's test — the vocabulary coupling is
inherent to `liftObliged`; the constant just names it).

**1b. Engine:** `retable` computes `cookedRules` (the re-homing):

```haskell
-- inside retable, after the cooked tables are built:
  , cookedRules = cookAxioms (deonticProducible st') (axioms st0)
```

with the NEW pool query (NOT producibleAtoms — panel I1):

```haskell
-- | Can this world ever contain an @obliged.*@ fact? The □-lift gate's
-- decision (spec v48). Reads the UNLIFTED producers only — practice and
-- schedule insert atoms, db facts as of now, and unlifted axiom heads — so
-- there is no cycle with the cookedRules being computed and no
-- self-fulfilling read of lifted heads. Conservative: a variable-headed
-- insert atom counts (it could ground to @obliged@); conservatism here only
-- ever KEEPS a lift (the safe direction). Db facts are those present at
-- retable time — the stated build-order invariant (documented at
-- 'setAxioms'): obliged-producing setup facts must precede the final
-- retable; both shipped axiom worlds build setAxioms-outermost.
deonticProducible :: PraxState -> Bool
```

(implementation: fold the insert-side atoms of `cookedDefs`+`cookedSchedule` via the
existing `cookedOutcomeAtoms` insert halves, db heads via `childKeys`-or-equivalent,
plus unlifted `axiomThen` heads; true iff any head name is `obliged` or a variable).
`setAxioms` becomes `reclose (retable st { axioms = axs })` — the ordering guarantee
(cookedRules current before reclose) re-established by construction; its haddock
carries the build-order invariant. `setAxioms`' old direct `cookAxioms` call dies.

**1c. Pins (RED-first):**
- An `oblige`-bearing fixture lifts: □-closure OBSERVED (a base rule fires under an
  obliged.□ context — DEON property 1 as behavior, not just head presence).
- The same fixture minus the oblige practice: no `obliged.Obligor.*` in `axiomHeads`.
- Added-AFTER-setAxioms: definePractices with the oblige practice AFTER setAxioms
  still lifts (the re-homing's point).
- SETTER COHERENCE, one pin per producer-changing setter (definePractices,
  defineFunctions, setSchedule, setDesires, setCharacters): each, adding/removing the
  obliged producer as its last build step, leaves the lift decision current.
- The db leg: a setup-performed `oblige` fact (performOutcome before setAxioms) lifts.
- `axiomDerivable` consumer check (panel): feud's want-patterns unify no vanishing
  lifted head — assert once against the real world.
- feudPin re-captured −8 rows, each named as a lifted twin in the commit body;
  villagePin asserted UNTOUCHED (run it, don't trust it); ViewInvariant green
  (now doubling as the gate's net — comment updated in ViewInvariantSpec).
- DeriveSpec's lifted-form tests pass the flag explicitly (True preserves today's
  expectations; a False twin pins the gate-off shape).

- [ ] RED → gate + re-homing → GREEN → feudPin itemized, villagePin byte-identical →
      gates → commit `"The closure engine stops taxing the innocent: □-lifting gates on deontic producibility"`.

---

### Task 2: The four de-couplings

**Files:** `src/Prax/Confession.hs`, `src/Prax/Stress.hs`, `src/Prax/Reactions.hs`,
`src/Prax/Worlds/Bar.hs`, `src/Prax/Emotion.hs`, `src/Prax/Worlds/Village.hs`,
`app/Main.hs` (stress entries), tests: `ConfessionSpec`, `StressSpec`, `ScriptSpec`
(coverage uses), `ReactionsSpec`, `EmotionSpec`, `VillageSpec`.

- **Confess verb** (`confess` gains the verb argument, single-segment loud guard;
  call sites pass `"confessed"`; the discharge-path readers — grep `confessed.` —
  confirmed all flow from the same construction).
- **Stress coverage family**: `stressTest`/`runRandom`/`StressReport` gain a
  `Maybe String` coverage-family; `sceneReached` generalizes to it; app/Main's
  script stress entries pass `Just "currentScene"`; a non-Script coverage pin
  (village `marketDay` or arc stages) proves the second application.
- **disapprovalP → Bar** (value + any private helpers; ReactionsSpec gains its own
  minimal reaction fixture; import updates).
- **feelingSomeone collapse** (delete; `smoulders` and specs re-point to
  `feelingToward`; the per-target-pricing haddock guidance moves to the survivor;
  grep-proof).
- Behavior everywhere identical: goldens/pins byte-identical is the task's whole
  exactness claim.

- [ ] RED (new guards/params) → GREEN → byte-identical suite → gates → commit
      `"Four de-couplings: the verb, the family, the content, the alias"`.

---

### Task 3: Docs

**Files:** `docs/LEDGER.md`, README/WALKTHROUGH sweeps (stress usage text, confession
mentions, feelingSomeone).

- [ ] The v48 legend row: the corrected census AS the round's story (three lenses,
  one error, detection vindicated); the gate's design (pool, invariants, ViewInvariant
  as net); feudPin −8 itemized; the four de-couplings; the v49 graduation recorded
  with its five constraints and the (a)/(b) fork's resolution; queue state after: v49
  is the audit queue's last member.
- [ ] Sweeps; gates; commit `"Docs: v48 — the innocent untaxed; leverage graduates"`.
