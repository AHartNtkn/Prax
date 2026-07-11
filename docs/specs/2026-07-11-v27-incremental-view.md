# v27 — Incremental view maintenance (LEDGER #17), exactness contract carried over

The v26 residual, measured: one axiom-closure per **distinct** state the planner's search
visits — 11,840 `reclose` entries per profiled village round, 71.8% of post-v26 planner time.
v26 removed every *duplicate* closure; this round removes the from-scratch recomputation for
deltas that provably cannot change what the axioms derive.

## The rule (unchanged from v26)

Every change is **exact**: bit-for-bit identical planner decisions and bit-for-bit identical
`readView` contents. The golden decision-sequence tests remain the net; this round adds a
stronger invariant check (below) because the cached view now has a second construction path.

## Design: the delta-irrelevance fast path

The observation: most lookahead deltas are movement, waiting, and practice bookkeeping —
facts no axiom looks at. For such a delta, closing from scratch is pure waste:

> **Theorem (fast path).** Let `F` be the axioms' *footprint*: every body atom (any
> polarity, including patterns inside `Absent`/`Or`/`Subquery`), every head template, and
> the `obliged.`-lifted forms of each (the □-lifting contributes rules too). If a ground
> sentence `s` — together with its eviction shadows, when `s` contains `!` — may-unify
> nothing in `F`, then:
> `closure axs (insert s db) = insert s (closure axs db)` and
> `closure axs (retract s db) = retract s (closure axs db)`.
>
> Sketch: `s` (and anything its exclusions evict) can satisfy no body atom (fires no rule),
> can defeat no negated body atom (un-fires no rule), and coincides with no derivable head
> instance (so base-presence and view-presence rise and fall together). Hence the derived
> set is unchanged and the base delta commutes with closure. Conservativity: `mayUnify`
> never returns a false negative under the stated v26 invariants (entity names vs predicate
> literals; eviction shadows cover displaced subtrees), so any uncertain `s` simply takes
> the slow path.

Mechanism:

1. **`axiomFootprint`** (in `Prax.Relevance`, beside `improvableDesires`): computed once per
   world from the axioms, carried as a `PraxState` field maintained by the same helpers
   (`setAxioms`; `definePractices`/`setDesires` don't touch it).
2. **`performOutcome` fast path**: for `Insert s` / `Delete s`, if `s` (plus eviction
   shadows) is footprint-irrelevant, apply the same primitive to `db` **and** `readView` in
   lockstep and skip `reclose`; otherwise reclose exactly as today. `ForEach`/`Call`/spawn
   already decompose into these primitives, so they inherit the split per primitive.
3. `withDb` (the opaque-function helper) keeps the reclose path — it cannot see the delta;
   its callers are cold paths (Persist load, dataFacts). The hot path is `performOutcome`.

Expected effect, to be measured (Task 0 probe): movement/wait/undertake/stage-bookkeeping
deltas — the bulk of candidate applications — go fast-path; belief deposits (`believes.*` is
in the village footprint via `standingUnless`/`transparent`/the swept-inference axiom) stay
slow-path, correctly. The probe reports the fast-path hit rate over a representative drive
before implementation is committed.

## The invariant net (new, stronger than goldens alone)

A property-style regression test: drive the village (and bar/intrigue) some turns through the
REAL loop, and after every single turn assert
`dbToLabeledSentences (readView st) == dbToLabeledSentences (closureOf st)` where `closureOf`
recomputes from scratch. Any divergence between the two construction paths fails loudly with
the turn number. This is the direct statement of what incrementality must preserve — the
goldens then additionally pin that decisions didn't move.

## Phase 2, gated on the Task 0 probe: truth maintenance

If the measured fast-path hit rate leaves the closure share dominant (the probe and a
post-fast-path re-profile decide; criterion: closure remains the top cost centre), the
axiom-relevant residue gets real incrementality — DRed-style support tracking (count
supports per derived fact; on delete, over-delete then re-derive survivors) over the
semi-naive loop. This is the honest-cost path (our axioms are non-monotone: inserts can
retract derived facts through defeaters, deletes can create them), and it lands only with
the measurement that justifies its complexity. If the fast path already demotes closure,
Phase 2 is recorded as not warranted — a stated decision, like v26 §3's.

## Verification

- Goldens byte-identical after every task; full suite green throughout.
- The per-turn invariant net (above), run over ≥ 3 village rounds + bar + intrigue drives.
- Zero warnings; hlint clean; `prax check` on all 7 worlds; the v26 grep-gates still empty.
- Honest perf report: fast-path hit rate, closures per round before/after, suite and Village
  group times before/after, one-round profile before/after.

## Out of scope

- Approximate pruning, depth changes, parallelism (unchanged bans).
- Query-level laziness / magic sets (a different, larger redesign; bank).
- Any new mechanics until this lands (standing user directive).
