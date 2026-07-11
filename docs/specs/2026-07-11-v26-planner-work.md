# v26 — Planner work elimination (exact only)

User directive: the suite went ~39s → ~12 minutes in v25; characterize the planner work that
does not contribute to final decisions and pre-filter it, before any further mechanics.

## Findings (measured this session; probes under the session scratchpad)

Profiled one village free-play round (7 turns, depth 2) plus a counting probe over all
(predictor, mover) pairs and a 7-round growth probe:

1. **`readView` is entered ~15,470 times per round and accounts for ~82% of runtime.** Every
   call recomputes the full axiom closure from scratch (`Engine.readView` →
   `Derive.closure`); the same state's view is recomputed dozens of times (by `evaluate`,
   `believedWants`, `inScope`, `possibleActions`) with nothing memoized.
2. **~48% of all time is string re-tokenization inside `Prax.Db`** (`tokens.go`, `trim.f`,
   `parseNames`, `insertToks`): the trie's operations take dot-strings and re-split them on
   every insert/query — most of it under closure's derived-fact churn, so much of this
   collapses once views stop being recomputed.
3. **`predictMove` runs 1,373 times per round (27.5% of runtime) and essentially never
   contributes.** At sampled states, of 42 ordered pairs: ~30 are out of prediction scope
   (each still pays an `inScope` view query per round-walk node), 10–15 have empty believed
   models, and the remainder — all conscience-only models of gale (via `transparent`) or
   pursuit models of bob — are **fully candidate-enumerated and evaluated only to return
   `Nothing`** at every node of the depth-2 recursion. Zero predictions fired at either
   sampled state.
4. **Fact growth is NOT the bottleneck**: the db grows only 51 → 97 sentences over 7 rounds.
   Per-round cost (3s → 33s at peak) tracks *believed-model density* — as hearsay spreads,
   more pairs hold nonempty models and the round-walk multiplies. Deleting facts from memory
   would not address the cost; no fact-GC round is warranted by this evidence.
5. Baseline for acceptance: full suite ~726s (292 tests), `Prax.VillageSpec` group
   ~580–660s alone.

## The rule for this round

**Every change must be exact**: the planner must make bit-for-bit identical decisions before
and after (verified, not assumed — see §Verification). No approximations, no tuned
thresholds, no behavior-relevant pruning. The round removes only work whose irrelevance to
the decision is *provable*.

## 1. Cached closed view (kills finding 1)

`PraxState` gains a `view :: Db` field holding the closure of `db` under `axioms`, computed
**once per state** by construction (lazily, so states whose view is never read pay nothing).
`readView` becomes a field access.

- All state construction/mutation goes through smart helpers that rebuild the thunk:
  `performOutcome`/`performAction`/`definePractices` (Engine), `emptyState` (Types), the
  Persist load site, and every world's record-update site that touches `db`, `axioms`,
  `desires`, or `practiceDefs` (worlds currently do `st { axioms = … }` — these must route
  through a helper or re-derive the view; a raw record update that changes `db`/`axioms`
  without refreshing `view` is a stale-view bug).
- Enforcement is structural, not conventional: the plan adds a gate that no raw record
  update of `db`/`axioms` on an already-built state exists outside the designated helpers
  (grep-gate in the verification task), and the helpers are the only exported way to do it.

## 2. Prediction improvability pre-filter (kills finding 3)

A predicted move exists only if some candidate **strictly raises** the believed model's
utility. Strictly raising it requires changing the satisfaction count of at least one model
want in the improving direction, and an action can only do that if one of its outcome
patterns *may-unify* with the relevant condition patterns of that want. This is decidable
conservatively from authored vocabulary alone, once per world:

- For each (action template, desire) pair compute `mayImprove`:
  - want weight **> 0** (improving = raising count): TRUE if any `Insert` outcome may-unify
    with a positive `Match` in the want's conditions, or any `Delete` outcome may-unify with
    a pattern under one of its `Absent`/`Not` conditions (removing a blocker raises count).
  - want weight **< 0** (improving = lowering count): the mirror — `Delete` against a
    positive `Match`, or `Insert` against an `Absent`/`Not` pattern.
  - Conservative TRUE wherever analysis is uncertain: `Call`/`ForEach`/`Calc`-dependent
    outcomes, `Subquery`/`Cmp` conditions, variable-heavy patterns. The filter may only ever
    say "cannot improve" when that is provable. One stated invariant carries the analysis
    (found in implementation: without it, pattern unification anchored on nothing makes
    every pair "unifiable" through variable slots and the filter is provably useless):
    **entity names never collide with predicate-name literals** — may-unification requires
    at least one aligned shared literal segment. This is an assumption about authored
    worlds, documented in the module header, not a construction guarantee; the golden
    decision-sequence tests would surface a violation as a dropped prediction.
- The round-walk (and `predictMove`) skips a (predictor, mover) pair outright when **no
  action template available to the mover** (its practices' templates, respecting
  `charBoundTo`) may-improve **any** desire in the believed model — before any grounding,
  candidate enumeration, or evaluation.
- Effect on v25's cost: everyone holds a conscience-only model of gale; the village's only
  `lied`-touching outcome is an `Insert` (the lie's mark) against a negative-weight want —
  provably non-improving — so those pairs skip exactly. Pursuit models of bob (stage
  `Insert`s against a positive pursuit want) correctly stay live: those predictions are the
  ones that actually fire (the flour-trip prediction).
- The table lives with the world state (derived from `practiceDefs` + `desires`, rebuilt by
  the same helpers as §1 when either changes).

## 3. Single tokenization in `Prax.Db` (finding 2 — gated on re-measurement)

After §1 and §2 land, re-profile the same one-round drive. If string tokenization remains
the top cost centre, convert `Db`'s internal operations to parse each sentence **once** at
the String boundary (the authoring surface stays strings; the trie operates on token
paths). If the §1 collapse has already demoted it, record the measurement in the round's
report and skip the rewrite — a decision stated openly, not a silent omission. (YAGNI over
speculative rewrites; the criterion is the measured profile, not a tuned threshold.)

## Verification

- **Golden decision sequences (the exactness proof, and a permanent regression net)**:
  before any change, capture from current master the exact action-label sequences of
  scripted drives — village 7 rounds free play (depth 2, "you" idle) plus shorter drives of
  bar and intrigue — into `test/Prax/GoldenDriveSpec.hs`. These tests must pass **before**
  the round's changes (observed) and after every task; any decision drift fails loudly.
  The planner is deterministic (ties broken by label), so the sequences are stable by
  construction.
- Full suite green (292 + goldens), zero warnings (`cabal build all`), hlint clean,
  `prax check` on all 7 worlds.
- **Honest perf report**: measured before/after times for the full suite, the Village
  group, and the one-round profile — reported as measurements, with no target number
  invented in advance.

## Out of scope (parked deliberately)

- Fact GC / memory decay — measured to be a non-bottleneck (finding 4); revisit only if a
  future measurement says otherwise.
- Approximate or utility-threshold pruning of lookahead — banned (inexact).
- Depth changes, parallelism, incremental closure maintenance across states (the LEDGER's
  incremental-view item stays open; per-state caching here is its lite form).
- Any new mechanics until this lands (user directive).
