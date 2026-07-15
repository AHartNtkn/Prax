# v40 — Hygienic Machinery Variables Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** per `docs/specs/2026-07-15-v40-hygienic-variables.md` — machinery variables move
to the `Prax` namespace, the five bespoke guards collapse into one shared boundary check
(whose lists shrink to genuine interface bindings), and the world-source ban becomes a
committed gate. Pure alpha-renaming: goldens byte-identical or BLOCK.

**Architecture:** one shared guard helper beside the v38 walkers; a mechanical rename with
an exact inventory (below — the implementer follows it, extending only with flagged
discoveries); one new source-reading GateSpec (the historical manual grep-gates made
durable). No engine, cooked-format, or `symIsVar` change.

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- THE RULE the round establishes, stated everywhere it applies: a machinery variable is
  one a combinator SPLICES INTO THE SAME CONDITION/OUTCOME LIST as author-supplied
  fragments; all such variables are `Prax`-prefixed. Call-scoped `Function` parameters
  (never mixed with author text) are NOT in scope. Interface variables (`Actor`, `Owner`,
  `Witness`, `Hearer`, `Seer`, `Seen`, `Spot`, `Anyone`, caller-supplied target vars)
  keep their names — they are the authoring contract.
- Alpha-invariance is the exactness claim: goldens byte-identical, ViewInvariant green,
  suite green. Any golden movement = a variable name leaked into semantics = BLOCK with
  the trace (a discovered defect, not noise).
- RED-first where behavior is new (the shared guard, the gate); rename-coverage pins
  update by exact inventory. Zero warnings; hlint; prax check.
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: The namespace, the guard, the gate

**Files:**
- Modify: `src/Prax/Types.hs` (the shared guard, beside `outcomeVars`), `src/Prax/Drift.hs`,
  `src/Prax/Rng.hs`, `src/Prax/Faction.hs`, `src/Prax/Confession.hs`,
  `src/Prax/Blackmail.hs`, `src/Prax/Sight.hs`, `src/Prax/Emotion.hs`,
  `src/Prax/Relevance.hs` (+ `src/Prax/Engine.hs` if it renders the eviction wildcard),
  `src/Prax/Deceit.hs`/`src/Prax/Kin.hs` (audit; rename any splice-adjacent internals found)
- Create: `test/Prax/GateSpec.hs` (registered like siblings; `prax.cabal`)
- Test updates: `DriftSpec`, `RngSpec`, `FactionSpec`, `ConfessionSpec`, `BlackmailSpec`,
  `RelevanceSpec` (the `believes...D` and `Evicted` anchor pins), `EngineSpec`
  (`coin.ada.Evicted`), `EmotionSpec`, plus each old guard's error-message pins.

**The shared guard (in Types, beside `outcomeVars` — complete):**

```haskell
-- | The v40 hygiene boundary: variables an author-supplied fragment may not
-- use when a combinator splices it into generated conditions. Two sources of
-- capture, one check: the @Prax@ namespace (ALL machinery variables live
-- there — see the spec; authors can never collide with them by accident) and
-- the combinator's own interface bindings (e.g. 'Prax.Confession.confess'
-- grounds @Actor@\/@H@ — an authored mark pattern using them would capture).
-- Returns the offenders; each combinator raises its own contextual error.
authoredVarClash :: [String]      -- ^ the combinator's spliced interface vars
                 -> [Condition] -> [Outcome] -> [String]
authoredVarClash interface conds outs =
  [ v | v <- vars, isPraxVar v || v `elem` interface ]
  where
    vars = filter isVariable (concatMap conditionVars conds
                              ++ concatMap outcomeVars outs)
    isPraxVar ('P':'r':'a':'x':_:_) = True
    isPraxVar _                     = False
```

(`isVariable` = the uppercase-first test the modules already use — import or share the
existing one; do not duplicate. String-pattern arguments that are not `Condition`s —
Confession's mark patterns — go through a sibling `authoredPatClash` over `pathNames`,
same lists, same home.)

**The rename inventory (exact; extend only with flagged discoveries reported):**
- `Drift`: `D`→`PraxD`, `D2`→`PraxD2`, `Now`→`PraxNow` (dueGate + gathering seeds);
  guardRule's reserved-list arm becomes an `authoredVarClash []`-based check (no interface
  splices — drift bodies are whole-condition author fragments).
- `Rng`: `S`→`PraxS`, `S2`→`PraxS2`, `S3`→`PraxS3`, `R`→`PraxR`; the reserved-list guard
  arm becomes the shared call.
- `Sight`: the ticker's `N`→`PraxN`, `M`→`PraxM` — THE DISCOVERED LATENT INSTANCE: the
  authored sighting template is spliced beside them with NO guard today; `sightP` gains
  the shared guard call (interface list `["Seer","Seen","Spot"]` are its CONTRACT vars —
  they stay, and are NOT forbidden; forbid only the namespace). `Seer`/`Seen`/`Spot`
  unchanged.
- `Faction`: `W`→`PraxW`, `F`→`PraxF` in the generated axioms; `reservedClash` deleted;
  the guard call's interface list shrinks to whatever `factionStanding` genuinely splices
  (derive from the source; the W/F ban VANISHES for authors — note the usability win in
  the haddock).
- `Confession`: `W`→`PraxW`, `Ds`→`PraxDs`, `N`→`PraxN` (the notoriety Count idiom);
  `reservedIn` deleted; guard calls keep interface lists (`["H","Hearer","Actor"]` etc.
  per site, verbatim from the current lists MINUS the renamed machinery names).
- `Blackmail`: machinery `D`→`PraxD`, `W`→`PraxW`; list shrinks to
  `["Owner","Actor","Hearer"]`.
- `Emotion`: `feelingsFade`'s `W`→`PraxW`, `E`→`PraxE`.
- `Relevance`/`Engine`: the eviction wildcard `Evicted`→`PraxEvicted`
  (`evictionShadowNames`); `moverReadAnchors`' believes-family wildcard `D`→`PraxD`.
- `Deceit`/`Kin`/`Core`/`Reactions`: AUDIT for splice-adjacent internals; rename any found
  (report each); `Anyone` and Function params stay.

**GateSpec (new — the manual grep-gates made durable):** an IO test that reads every
`src/Prax/Worlds/*.hs` source file and asserts no string literal contains a
`Prax`-prefixed VARIABLE token (scan quoted strings for `Prax[A-Z]`-shaped path segments;
keep the scanner simple and loud — false positives are acceptable-conservative, absence
of the scanner is not). A second case pins the guard itself: a combinator invoked with a
`PraxD`-using fragment errors loudly (try/evaluate/ErrorCall, per house idiom).

**Process:** update the guard-message pins + write GateSpec + the shared-guard unit
(RED: names missing / new messages unmatched) → implement the rename by inventory + the
guard consolidation → GREEN per touched spec → FULL suite: goldens byte-identical or
BLOCK → gates (zero warnings; hlint; prax check ×7) → commit
`"Hygiene: machinery variables live in the Prax namespace — one guard, no capture"`.

- [ ] RED observed → rename + consolidate → GREEN → suite + nets → gates → commit.

---

### Task 2: Docs

**Files:** `docs/LEDGER.md`.

- [ ] The v40 legend row (house style): the queue context (first of four user-directed
  foundations passes); the two-tier finding (interface vars are the contract; machinery
  vars were the defect); the discovered unguarded Sight splice; the guard consolidation
  arithmetic (five bespoke walkers → one helper; lists shrunk to genuine interface
  bindings — name the W/F usability win); alpha-invariance held (goldens byte-identical);
  the durable GateSpec replacing manual grep-gates; suite count as measured. Mark the
  foundations-queue item done; queue pointer to v41.
- [ ] Gates; commit `"Docs: v40 — no capture"`.
