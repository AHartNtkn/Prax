# v52 — Plans as part-sets: the endeavor loses its cursor, and the completion ledger becomes the state

The `done.sN` bank item (LEDGER, v51's one carried bank) opened for design, and the
user's framing replaced the audit's: a plan has MANY MOVING PARTS, completable
independently and in parallel, NOT ALL REQUIRED for success. Under v24's linear
chain the accumulating `done.sN` family was a shadow of the `stage!` cursor; under
the corrected model the dependency inverts — with independent, partially-optional
parts there IS no single number encoding progress, so the set of completed-part
facts is the honest PRIMARY state, and it is the linear cursor that dies as the
artifact.

AMENDED after the three-lens pre-gate panel (`.superpowers/sdd/v52-spec-review-*`):
the panel made dependency edges structural [D-C1], renamed the completion record's
category honestly [D-I1], surfaced that the design is MORE expressive than the
first draft claimed [D-I3], and pinned the concrete Village re-authoring the
transcript claim hangs on [C-I1]. Findings folded with [S]/[D]/[C] citations.

## The recovered intent (v24, the committed record of the settled design)

What v24 validated and what this round preserves verbatim: **count-based progress
rewards make horizon length irrelevant** — every next piece of work locally
rewarding (+w per completed-work fact) at ordinary depth; types authored,
instances emergent via undertake; the pursuit a named, dormant,
theory-of-mind-visible desire; instances persistent and interleaved. The LINEAR
staged chain was v24's simplification of "plan," not its intent. The utility path
(`evaluate` = Σ wantUtility × countSatisfying, Planner.hs:36) is binding-counting,
so the part-set model needs ZERO engine change.

## The design

`Prax.Project` REPLACED (no dual system — `Stage`/the linear form dies):

```haskell
data Part = Part
  { partName   :: String       -- single path segment; the ledger key
  , partLabel  :: String       -- action label
  , partAfter  :: [String]     -- dependency edges: SIBLING part names [D-C1]
  , partNeeds  :: [Condition]  -- world resources; threshold gates (see below)
  , partYields :: [Outcome]
  }

endeavor :: String -> Int -> String -> [Condition] -> [Part]
         -> (Action, Practice, Desire)
```

- **Dependency edges are STRUCTURAL, not spelled paths [D-C1]**: `partAfter`
  names sibling parts; `endeavor` validates every name against the actual part
  set — a dangling or misspelled edge is a LOUD construction error (the module's
  existing guard idiom: empty stages and dotted pids already error). The
  compiler builds the ledger conditions itself; the fact-path convention stays
  PRIVATE (no `didPart` in the API — the first draft leaked machinery as
  authoring surface, and a typo'd edge would have failed silently as a
  never-available part).
- **Completing part p writes the PER-PART COMPLETION LEDGER**: `Insert
  ("practice.<pid>.Owner.did.<partName>")`. Named honestly [D-I1]: this is
  infrastructure — a progress ledger under the practice-instance subtree, born
  and dying with the instance — NOT a fiction-adjacent deed (the first draft's
  `threatened.*` analogy was the documented-divergence smell in category
  clothing). It is JUSTIFIED infrastructure: with parallel/optional parts,
  progress is a set; parts' world-yields are non-durable and heterogeneously
  named (`carrying.flour` is consumed at bake), so a uniform ledger family is
  the only honest countable form for one `Want`. A part whose completion should
  be world-visible fiction ALSO writes a real deed via `partYields`
  (`swept.Actor`, witnessed), exactly as v24 did — the ledger and the fiction
  are separate facts with separate jobs.
- **Each part's action**: gated `Eq Actor Owner` (the instance's existence and
  Owner's binding ride the practice-instance ENUMERATION itself, from the
  undertake fact — the plan review traced this: the stage gate never bound
  Owner, so no explicit instance match is needed and none is added), `Not` its
  own ledger entry, its compiled `partAfter` conditions, and `partNeeds`
  (v40-guarded at construction: `authoredVarClash` over needs and yields). NO cursor — every part whose gates the world meets is
  simultaneously available; the planner chooses among parallel parts by
  ordinary scoring. `stage!` DIES (nothing else reads the family — the
  instance-enumeration binds Owner via the undertake fact, not the seed
  [S-verified]).
- **Each part fires ONCE per instance — a stated boundary, not an accident
  [D-I2]**: the ledger entry doubles as the once-guard. Repeatable/counted
  parts (a patrol walked nightly) are thereby inexpressible — this is v24
  parity and the simple thing; decoupling the count-fact from the once-guard
  is PARKED until a world needs it.
- **"Not all required" is authored topology — and richer than edges [D-I3]**:
  a culminating part's `partAfter` names the REQUIRED parts; optional parts
  hang off the side, each +w when taken, never blocking. Beyond point edges,
  `partNeeds` carries genuine THRESHOLD gates today — `Subquery`/`Count`/`Cmp`
  over the ledger family expresses "fire when any 3 of 5 are done"; any-of-N
  culminations = N parts yielding the same payoff fact. This is why BOTH
  surfaces exist: `partAfter` for the common point-edge (structural, validated),
  `partNeeds` for world resources and thresholds (raw conditions, v40-guarded).
- **Uniform +w, consequence stated [D-M1]**: optional-vs-required is a topology
  distinction with no planner-visible priority — the agent is locally
  indifferent between a flourish and a load-bearing part until a culmination's
  payoff enters the horizon. Per-part weights need N desires or a
  value-weighted `Want` — parked, named in the bank.
- **The pursuit**: `Want [ Match "practice.<pid>.Owner.did.P" ] w` — +w per
  ledger entry, dormant without an instance; same name, dormancy, and
  theory-of-mind ride as v24 (`predictMove` makes no linearity assumption
  [S-verified]; with parallel parts the predicted move is whichever part
  scoring picks).
- **Undertake, one-instance-per-owner, teardown**: unchanged. Teardown
  correctness rests on `Delete` being a SUBTREE delete (Db.hs's retract prunes
  the named subtree) — the invariant is named here because the whole
  re-open-the-endeavor contract sits on it [D-M2]; Village's `eat` keeps
  working untouched.

## The shipped re-authoring, pinned [C-I1]

earnBread becomes parts `"sweep"` / `"fetch"` / `"bake"` (labels unchanged),
edges `fetch after ["sweep"]`, `bake after ["fetch"]` — the exact 3-edge chain.
Omitting an edge would make parts parallel and drift the golden; these two edges
ARE the transcript-identity claim. VillageSpec's `done.s3` asserts re-point to
`practice.earnBread.bob.did.bake`.

## What dies

`Stage` (→ `Part`), the `stage!` cursor and its init seed, the `done.sN` family
and its numbering, the implicit stage-order gate, the first draft's `didPart`
(never shipped). The `done.sN` LEDGER bank item closes SHIPPED.

## Classification

DESIGN REPLACEMENT: the internal fact language changes, pins re-point, PROPERTIES
are the contract. Behavioral expectation stays strong [S-verified ply-by-ply for
the chain]: identical offered sets and utilities at every state of the shipped
world, so FICTION TRANSCRIPTS identical — GoldenDriveSpec's village golden
(undertake→sweep→fetch by label) is the concrete must-re-run pin of that claim
[C-I3]; drift is itemized and adjudicated, never silently re-captured.

## The property contract (violating any is the BLOCK)

1. **Local reward carries the horizon** (v24's theorem re-pinned): a multi-part
   endeavor pursued to completion at depth 2, each part locally chosen.
2. **Parallel parts are genuinely parallel**: two dependency-free parts with met
   needs are BOTH offered; the un-chosen one remains available after.
3. **Optional parts are optional**: a culminating part fires with an optional
   part unperformed; performing it still pays +w.
4. **Threshold success is authorable**: an m-of-n culminating gate (Count over
   the ledger) fires exactly at m [D-I3 — pinned because the spec claims it].
5. **Dependencies gate loudly and correctly**: an unmet `partAfter` edge blocks
   the part; a dangling edge name is a construction-time error.
6. **Each part once; teardown re-opens**: a completed part is not re-offered
   within an instance; the subtree delete re-opens the whole endeavor.
7. **Dormancy and ToM survive**: instanceless pursuit = zero utility,
   `predictMove` Nothing; a believed pursuit predicts the next available part.
8. **The village arc is preserved**: bob undertakes, works, bakes, holds the
   earned loaf; opportunism (steals when the square empties) and
   observation→prediction hold; the v21-amended non-re-offense assertion
   re-points to `did.bake`.

## Verification

ProjectSpec re-founded on parts (the eight properties, RED-first each).
Must-re-run/re-point estate, enumerated [S-I1, C-C1, C-C2, C-I3]: VillageSpec
:251/:362/:419 (`done.s3` → `did.bake`); GoldenDriveSpec unchanged-and-re-run
(the transcript pin); RelevanceSpec :26 (the literal `…done.S` atom string →
`…did.P`), :55-57/:65 (improvable — expected to HOLD, ledger inserts may-unify
the same way, but observed not assumed), :160-163 (liveness comment prose +
re-derived classification); AnalysisTableSpec villagePin's three project rows
(label-derived — expected byte-stable, named here so stability is OBSERVED).
Docs: prose describing the staged/cursor model — README:199-206,
WALKTHROUGH:1247/:1364/:1994 — rewritten to parts [C-I2]; LEDGER:132 (v24's
row) is a RECORD and is NOT edited [C-I4 — the first draft self-contradicted
here]; the v52 row records the rename. Death greps: `done\.s|stage!|Stage` zero
in src/Prax/Project.hs; `done.S|done.s` zero in test/ outside historical spec
citations. Pre-gate: this panel ran; verdicts SOUND / PRINCIPLED / GAPS, folded.

## Out of scope (parked with names)

Per-part weights (the value-weighted `Want` question). Repeatable/counted parts
(decoupling the ledger from the once-guard) [D-I2]. Abandonment, cooperative
multi-owner projects, type synthesis (v24's parks, still parked).
