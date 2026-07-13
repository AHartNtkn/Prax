# v33 — State-conditioned relevance (exactness contract carried over)

User-directed: the v26 relevance filter reasons from vocabulary alone ("could any action EVER
improve this want-kind?"), which v32's `confess` made permanently true for consciences —
spending the filter's skip world-wide even though it remains provably sound in almost every
actual state. Measured: the loss alone is ~5.5× on 31 unchanged village tests (171.6s vs
31.1s). This round adds the missing dimension: **could it matter NOW?**

## The rule (unchanged)

Exact only: bit-for-bit identical decisions (goldens) and views (ViewInvariant net). The skip
may fire only when provable; all uncertainty keeps the pair.

## Design: two dead-now checks, one shared shape

The pair-skip in `predictMove` currently asks the static table "is any believed desire
improvable?" It gains a second, state-level question per believed desire — "is its
improvability LIVE in this state?" — decided by cheap existence/count queries against the
cached view, with the mover grounded as `Owner`:

1. **The floor check (negative want-kinds).** A negative rule is improved only by LOWERING
   its satisfaction count; if its count is **zero right now, it is at its floor** and no
   candidate action — whatever its preconditions or outcomes — can improve it. Soundness is
   unconditional on conjunct structure: count 0 is the minimum. One `countSatisfying`
   (equivalently: any-binding existence) of the desire's own Owner-grounded conditions.
   This tier alone reclaims the regression: `transparent` fills believed models with trait
   bundles, traits are predominantly negative conduct-valuations (v25), and a markless
   character's conscience is at its floor in almost every state.
2. **The environment-gate check (positive want-kinds).** A positive rule's count can rise
   only through a new binding of its WHOLE conjunction; a conjunct that (a) no authored
   action outcome can insert (decided statically from the existing atom pools) AND (b) is
   not axiom-derivable (the existing `derivable` conservatism) AND (c) currently has zero
   bindings, makes the rise impossible: the mover's one predicted move cannot create what
   nothing in the vocabulary creates. Statically compute each positive desire's
   environment-gated base-family conjuncts once per world; at runtime, one emptiness check
   per gate. This is the hunger shape: `Match "hungry.Owner"` gated by the ticker-only
   fact family — dormant hunger stops costing prediction work.

Both checks are **pair-level only**, preserving v26's principle: if ANY believed desire is
live, the FULL model (dead deterrents included) is evaluated — deterrents must keep
deterring.

## Mechanics

- `Prax.Relevance` gains the static classification: per desire, its polarity, its cooked
  Owner-template conditions (for the floor check), and its environment-gated conjuncts
  (for the gate check) — carried in a retable-maintained `PraxState` field like
  `improvables`/`footprint` (the established derived-field family, cooked per v28).
- `predictMove`'s skip becomes: statically dead OR dead-now, for every believed desire.
  The dead-now queries run against the cached view (v27) on cooked conditions (v28) —
  the check is orders cheaper than the candidate enumeration + scoring it avoids.
- Conservativity: weight 0 stays never-improvable (existing); desires whose conditions
  contain `Subquery`/`Count`/`Calc` stay statically-conservative as today (never reach the
  state checks); any gate uncertainty ⇒ the desire counts live.

## Verification

- Goldens byte-identical; ViewInvariant green; full suite green throughout.
- RelevanceSpec/PlannerSpec additions: the floor case (a markless conscience-bearer's pair
  skips; give them one mark and the pair goes live — BOTH directions); the env-gate case
  (a hunger-shaped desire dead while the gate fact is absent, live when present); the
  derived-conjunct conservative case (a gate on an axiom-derivable family never qualifies).
- **The perf acceptance is the v32 A/B re-run**: the same 31 pre-existing village tests,
  uncontended — target is recovery toward the 31.1s pre-v32 number, reported as measured.

## Out of scope

Precondition-chain analysis for positive desires beyond environment gates (the mover
enabling itself multi-step is invisible to the myopic predictor anyway — document, don't
build); anything approximate; new mechanics.
