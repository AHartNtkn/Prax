# v52 — Plans as part-sets: the endeavor loses its cursor, and the deed record becomes the state

The `done.sN` bank item (LEDGER, v51's one carried bank) opened for design, and the
user's framing replaced the audit's: a plan has MANY MOVING PARTS, completable
independently and in parallel, NOT ALL REQUIRED for success. Under v24's linear
chain the accumulating `done.sN` family was a shadow of the `stage!` cursor; under
the corrected model the dependency inverts — with independent, partially-optional
parts there IS no single number encoding progress, so the set of completed-part
facts is the honest PRIMARY state, and it is the linear cursor that dies as the
artifact.

## The recovered intent (v24, the committed record of the settled design)

What v24 validated and what this round must preserve, verbatim from its spec:
**count-based progress rewards make horizon length irrelevant** — every next piece
of work locally rewarding (+w per completed-work fact) at ordinary depth; types
authored, instances emergent via undertake; the pursuit a named, dormant,
theory-of-mind-visible desire; instances persistent and interleaved. The LINEAR
staged chain was v24's simplification of "plan," not its intent. The utility path
(`evaluate` = Σ wantUtility × countSatisfying, Planner.hs:36) is binding-counting,
so the part-set model needs ZERO engine change: one want counting completed parts
reproduces the validated psychology exactly.

## The design

`Prax.Project` REPLACED (no dual system — `Stage`/`endeavor`'s linear form dies):

```haskell
data Part = Part
  { partName   :: String       -- single path segment; the deed's name
  , partLabel  :: String       -- action label
  , partNeeds  :: [Condition]  -- world resources AND authored dependencies
  , partYields :: [Outcome]
  }

didPart :: String -> String -> Condition   -- the dependency vocabulary:
didPart pid part = Match ("practice." ++ pid ++ ".Owner.did." ++ part)

endeavor :: String -> Int -> String -> [Condition] -> [Part]
         -> (Action, Practice, Desire)
```

- **Completing part p records the DEED, by name**: `Insert
  ("practice.<pid>.Owner.did.<partName>")`. Named deed records ("did.sweep",
  "did.bake") are fiction-adjacent facts — a record of work performed, gateable
  by anyone (the v49 `threatened.*` category) — where "done.s3" was a number in
  bookkeeping's clothes. The family is NOT reserved.
- **Each part's action**: gated on the instance (`Eq Actor Owner` + the instance
  fact), on `Not` its own deed (each part once per instance), and on
  `partNeeds`. NO cursor gate — every part whose needs the world meets is
  simultaneously available; the planner chooses among parallel parts by
  ordinary scoring. `stage!` DIES.
- **Dependencies are authored edges, not a forced chain**: a part that requires
  prior work names it — `partNeeds = [ didPart pid "sweep", ... ]`. Any DAG,
  including none. A linear plan remains expressible as a chain of edges;
  v24's earnBread becomes exactly that.
- **"Not all required for success" costs NOTHING**: there is no completion
  machinery. A culminating part encodes its requirements as dependency edges on
  the parts that ARE required; optional parts hang off the side, each worth +w
  when taken, never blocking. An endeavor may equally have no culmination
  (open-ended work). Success is authored topology, not engine state.
- **The pursuit**: `Want [ Match "practice.<pid>.Owner.did.P" ] w` — +w per
  completed part, dormant without an instance. Same name, same dormancy, same
  theory-of-mind ride (v23/v24 unchanged).
- **Undertake, one-instance-per-owner, teardown**: unchanged — the instance
  subtree delete (Village's `eat`, Village.hs:385-389) kills deeds and instance
  alike and re-opens undertake.

## What dies

`Stage` (→ `Part`), the `stage!` cursor and its `init` seed, the `done.sN`
family and its numbering, the implicit stage-order gate. The `done.sN` LEDGER
bank item closes SHIPPED.

## Classification

DESIGN REPLACEMENT (the v49 lesson, stated up front): the internal fact language
changes (`done.s3` → `did.bake`; no `stage!`), so pins reading those facts
re-point and this round pins PROPERTIES, not bytes. The behavioral expectation
is still strong: for the shipped world (earnBread re-expressed as a 3-edge
chain), part availability, utilities (same counts × same weight), and therefore
choices and FICTION TRANSCRIPTS are expected identical — any transcript drift is
itemized in mechanics terms and adjudicated, not silently re-captured.

## The property contract (violating any is the BLOCK)

1. **Local reward carries the horizon** (v24's theorem, re-pinned): a multi-part
   endeavor is pursued to completion at depth 2, each part locally chosen.
2. **Parallel parts are genuinely parallel**: with two dependency-free parts
   whose needs are met, BOTH are offered; the planner's choice between them
   follows ordinary scoring, and the un-chosen part remains available after.
3. **Optional parts are optional**: a culminating part fires once its named
   dependencies hold, with an optional part still unperformed; performing the
   optional part still pays +w.
4. **Dependencies gate**: a part whose `didPart` edge is unmet is not offered.
5. **Each part once**: a completed part's action is not re-offered within an
   instance; teardown re-opens the whole endeavor.
6. **Dormancy and ToM survive**: instanceless pursuit = zero utility and
   `predictMove` Nothing; the believed pursuit predicts the next available part.
7. **The village arc is preserved**: bob undertakes, works, bakes, holds the
   earned loaf; the opportunism test (steals when the square empties) and the
   observation→prediction chain hold; the v21 amended non-re-offense assertion
   re-points from `done.s3` to the culminating deed.

## Verification

ProjectSpec re-founded on parts (the seven properties above, each RED-first);
VillageSpec earnBread arcs re-pointed (`done.s3` → the deed record; transcripts
compared and drift itemized); the LEDGER:132 and any doc prose naming `done.s3`
updated (T-docs); death greps: `done\.s|stage!` zero in src/Prax/Project.hs and
no `Stage` constructor survives. Pre-gate: the three-lens panel runs on this
document (a design replacement warrants it).

## Out of scope (parked with names)

Per-part weights (richer than uniform +w; needs either N desires or a
value-weighted `Want` extension — bank as the Want-extension question).
Abandonment and cooperative multi-owner projects (v24's banks, still banked).
Project-type synthesis (research tier, v24's park).
