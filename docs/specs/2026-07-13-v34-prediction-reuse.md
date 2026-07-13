# v34 — Prediction reuse (exactness contract carried over)

User-directed, from measured branch statistics. Over 70 village free-play turns (60 NPC picks
at depth 2):

- **The time is the recursion, not the state machinery.** The same picks cost 89 ms at
  depth 0 (ground + apply + score every candidate), 2.3 s at depth 1, **44.5 s at depth 2**
  — each lookahead level multiplies by ~20–26×. Every optimized subsystem (closure, cooked
  queries, interning) is already cheap; the cost is pure branching: candidate × predicted
  mover's candidates × their candidates.
- **One template is the tree.** The gossip whisper grounds over Hearer × Culprit and yields
  8–16 candidates per bearer per turn: 458 of 674 top-level groundings (68%), taken once.
  Whisper-bearing picks account for ~100% of all pick time (gale at 16 groundings: 2.1–4.3 s
  per pick; dana at 3: 60–150 ms).
- **The predictions being re-derived are the same predictions.** Within a pick, sibling
  post-states' predictions equal the parent state's prediction in **4,014 of 4,014**
  observed (candidate, mover) comparisons — overwhelmingly `Nothing` (only 4 of 360
  top-level pairs predict a move at all; 342 of 360 resolve cheaply via scope/model/dead
  checks). The 458 whisper groundings collapse to 66 per-pick distinct scores (bob's 12 tie
  at one score; gale's 16 split exactly two ways, truth vs. slander).

The waste is structural: `scoreActions` re-runs `predictMove` for every mover at every node
of the imagination tree, although a node's state differs from the pick's root only by a few
outcome tokens that provably cannot change most movers' predictions. This round makes that
proof and reuses the root's predictions where it holds.

## The rule (unchanged)

Exact only: bit-for-bit identical decisions (goldens) and views (ViewInvariant net). Reuse
may fire only when provable; all uncertainty recomputes the prediction live.

## Design: root-memo predictions, invalidated by delta anchors

Within one `pickAction`, the predictor is always the pick's actor. So one pick carries one
memo: **mover → the prediction computed at the pick's root state**, filled lazily. At every
tree node, `predictMove` for a mover is replaced by:

1. **The path delta.** Each node's state differs from the root by the grounded outcome
   tokens applied along the path to it (the candidate's outcomes, plus any predicted moves'
   outcomes applied by the round fold). The walk accumulates their **anchor families**
   (the `[[Sym]]`-style anchored segment lists of v26) — inserts and deletes both. This is
   collected from the grounded outcomes themselves; no dependence on which engine path
   (`applyDirect`/`applyGrow`/`withDb`) performed them.

   **The variable-headed insert rule** (amended after Task 2's attribution profiling: the
   original guard forced EVERY broadcast `ForEach` insert — `Witness.believes...`, the
   whisper's own shape, 68% of the tree — opaque, leaving reuse firing on 1% of calls and
   the A/B at −9.5%): an insert whose first segment is a variable can spawn a practice
   instance only if that variable can take the value `practice`. A **safe binder** — a
   variable bound at a NON-FIRST position of a top-level positive `Match` guard of the
   enclosing `ForEach`, and never occurring at the first position of any such guard —
   provably cannot: values read out of a fact's interior are entity/value names, and the
   authored-world structural invariant (the same family as Relevance's
   entity-names-vs-predicate-literals invariant, stated where spent) reserves the literal
   `practice` for registry roots — it is never an entity, place, value, or id name.
   Inserts headed by a safe binder are bounded with the variable as a `mayUnifySyms`
   wildcard — unless the whole path is variables: an evidence-free path carries no anchor
   for the anchored-literal rule and would clear against every read, so it stays opaque
   (inserts and deletes both). Every other variable head (first-position binders, which
   really can unify `practice` against the registry; `Exists`/`Or`/subquery-scoped
   variables, which do not bind outward; call-scoped parameters) stays opaque. Conservativity is unchanged in
   direction: uncertainty ⇒ opaque ⇒ live.
2. **The affected cone.** A static, per-world relation (computed in `retable`, alongside
   `footprint`): fact family A *feeds* an axiom when ANY body atom of the axiom may-unifies
   A; the axiom's head family is then affected; close transitively. Expanding the path
   delta's anchors through this relation yields every base **and derived** family the node's
   view could differ from the root's view in. Any delta anchor that cannot be resolved to
   families (e.g. a `Call` whose cases are somehow opaque) makes the node **opaque**: no
   reuse below it, recompute everything — uncertainty always falls toward the live path.
3. **The pair read set.** `predictMove st actor m` reads, and only reads:
   - the in-scope check: `predictionScope` patterns grounded `Actor:=actor, Witness:=m`;
   - the believed model: `<actor>.believes.desires.<m>.*` (Minds reads exactly this family);
   - the mover's affordances: every practice action's conditions with `Actor:=m` (plus the
     dead-check and instance/role machinery `candidateActions` consults);
   - evaluation and dead-now: every vocabulary desire's conditions with `Owner:=m`
     (`cookedDesires` — `deadNow` reads a subset of the same).
   Each is a statically enumerable pattern-anchor set, grounded per mover, computed once per
   pick (or cached per world where grounding permits).
4. **The test.** If the node's affected cone may-unifies nothing in the mover's read set,
   then the node's view and the root's view agree on every fact any of those reads can see —
   including derived facts, because a differing derived fact requires an axiom fed
   (transitively) by the delta, which the cone contains — so `predictMove` at the node
   equals the root memo. Reuse it (`Nothing` included: the memo's dominant value). Otherwise
   compute live at the node, exactly as today.

Soundness of the evaluation inside a reused pair, spelled out: `predictMove` evaluates
`closure(nodeView + moverCandidateOutcome)` against believed-model conditions. The mover's
candidate set is identical at node and root (their conditions are in the read set), the
grounded outcome is therefore identical, and any derived fact differing between
`closure(nodeView + o)` and `closure(rootView + o)` requires an axiom fed by the path delta
— in the cone. If the cone misses the read set, every count the evaluation takes is equal.

The anchored-literal discrimination does the per-pair precision for free: a whisper's cone
is `{…believes.<H>.stole…, regards.<W>.<C>.<label>…}`; a mover's fear reading
`regards.W.<m>.slanderer` unifies it only when the culprit IS that mover and the labels
match — exactly the pairs whose predictions could actually flip.

## What this is not

No candidate is pruned, no score approximated, no frequency consulted. Deterrents keep
deterring through the same mechanism as v33: a pair whose prediction COULD change (cone
meets read set) is recomputed in full. The whisper fan itself remains enumerated and scored
— its per-sibling cost falls to applies + evaluations (~0.13 ms each measured), which is the
depth-0 floor.

## Mechanics

- `Prax.Relevance` gains the static feeds/affects relation (family → transitively affected
  families) and the per-mover read-set builder; `PraxState` carries them retable-maintained
  (the established derived-field family).
- `Prax.Planner`: `scoreActions`/`valueAfter`/`predictMove` thread the pick-root memo and
  the accumulated delta anchors. `pickAction`'s signature and all exports unchanged.
- Grounded-outcome anchor extraction shared with the existing outcome-pool analysis (one
  home; no re-derivation).

## Verification

- Goldens byte-identical; ViewInvariant green; full suite green throughout.
- PlannerSpec, both directions (RED-first):
  - **Reuse must not miss a flip**: a fixture where the actor's candidate inserts the gate
    fact of a mover's believed desire (parent prediction `Nothing`, post-state prediction =
    the enabled move) — the chosen action must reflect the live recomputation. Same shape
    with a delta that crosses a Count threshold (a whisper delta waking a regards-reading
    fear), pinning the cone's derived-family expansion.
  - **Reuse fires**: a fixture where the delta is provably disjoint — pinned behaviorally
    (decisions identical to the unmemoized planner on the same state) with the A/B carrying
    the perf evidence.
- **The perf acceptance**: the same 31 pre-existing village tests (v32/v33 A/B epochs:
  31.11 s pre-v32 / 171.64 s post-v32 / 132.75 s post-v33), uncontended, best-of-3, plus the
  full suite (~175 s at v33). Reported as measured, wherever they land.

## Out of scope

Pruning or collapsing candidate groundings (subsumed: their subtree cost falls to the
depth-0 floor without touching the candidate set); memoization across picks or across turns
(a pick's root memo dies with the pick); per-desire partial reuse inside a live pair;
anything approximate.
