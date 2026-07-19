# Plan v52 — plans as part-sets

Governing spec: `docs/specs/2026-07-19-v52-plans-as-part-sets.md` (panel-amended;
its [S]/[D]/[C] citations govern). Two tasks: T1 the replacement + all pins, T2
docs. RED-first per property; GoldenDriveSpec's village golden is the transcript
pin and must NOT move.

## T1 — The replacement

**Why each piece exists:** the user's directive (parts independent, parallel,
partially optional) kills the cursor, which makes the completion ledger the
primary state; structural `partAfter` [D-C1] exists because a spelled path is
leaked machinery and a typo'd edge fails silent; `partNeeds` survives because
thresholds and world resources are raw conditions [D-I3]; the once-guard rides
the ledger entry as a stated boundary [D-I2].

- `src/Prax/Project.hs` rewritten (module haddock included — the "staged
  practices" story becomes the part-set story; keep the v24 psychology prose,
  it is unchanged truth):

  ```haskell
  data Part = Part
    { partName   :: String       -- single path segment; the ledger key
    , partLabel  :: String
    , partAfter  :: [String]     -- sibling names; validated, compiled privately
    , partNeeds  :: [Condition]
    , partYields :: [Outcome]
    }

  endeavor :: String -> Int -> String -> [Condition] -> [Part]
           -> (Action, Practice, Desire)
  endeavor pid w ulabel gate parts
    -- guards, all loud, construction-time:
    --   null parts                                   (an endeavor is work)
    --   pid contains '.' or '!'                      (existing guard)
    --   any partName contains '.' or '!'             (it is a path segment)
    --   duplicate partNames                          (the ledger key)
    --   any partAfter name ∉ map partName parts      (dangling edge [D-C1])
    --   a part listing itself in partAfter           (self-edge; also covers
    --                                                 the trivial cycle — full
    --                                                 cycle detection is NOT
    --                                                 built: a cycle among ≥2
    --                                                 parts makes those parts
    --                                                 permanently unavailable,
    --                                                 which property pins can
    --                                                 not distinguish from
    --                                                 unmet needs. Flag it in
    --                                                 the same validation pass
    --                                                 (reachability from the
    --                                                 edge-free parts) — a
    --                                                 part unreachable by
    --                                                 topology alone is a
    --                                                 LOUD error.)
  ```

  Generated pieces: undertake action unchanged in shape. The practice:
  `initOutcomes = []` (the stage!0 seed DIES — instance enumeration rides the
  undertake fact [S-verified]); part k's action =
  `action (partLabel p)
     ([ Eq "Actor" "Owner", Match instanceFact, Not (ledger p) ]
      ++ [ Match (ledger q) | q <- partAfter p ] ++ partNeeds p)
     (Insert (ledger p) : partYields p)`
  where `ledger p = "practice." ++ pid ++ ".Owner.did." ++ partName p` is
  PRIVATE. Pursuit: `Desire ("pursues-" ++ pid) (Want [ Match ("practice." ++
  pid ++ ".Owner.did.P") ] w)`.
- `src/Prax/Worlds/Village.hs` (~:91-104): the three `Stage`s become `Part`s —
  names `"sweep"`/`"fetch"`/`"bake"`, labels UNCHANGED, edges
  `fetch after ["sweep"]`, `bake after ["fetch"]`, needs/yields verbatim
  [C-I1: these two edges ARE the transcript claim]. The `eat` teardown comment
  gains the subtree-delete invariant note [D-M2]; the delete itself unchanged.

**Tests (RED observed per property — the spec's eight-property contract):**
- `ProjectSpec` re-founded (~same size; the oven fixture becomes parts):
  1. horizon regression re-pinned (4-part CHAIN to completion at depth 2 — the
     v24 probe, on edges);
  2. parallel parts (two edge-free parts both offered; un-chosen still offered
     after — RED by asserting against the old cursor semantics is impossible
     post-replacement, so RED = write the pin before the new module compiles
     the parallel gate, per the established neuter path);
  3. optional part (culmination fires with an optional part undone; the
     optional still pays +w after — score-compared via pickAction/evaluate);
  4. threshold success (a 5-part endeavor whose culmination needs
     Count ≥ 3 over the ledger family fires exactly at 3 [property 4 — the
     spec claims it, so it is pinned]);
  5. dependency gating (unmet edge blocks; met edge offers);
  6. each-part-once + teardown re-opens (complete a part, not re-offered;
     subtree delete, undertake offered again);
  7. dormancy + ToM (zero utility instanceless; predictMove Nothing;
     believed pursuit predicts the next available part);
  8. loud guards: empty parts, dotted pid, dotted partName, duplicate
     partNames, dangling edge, self-edge, unreachable-by-topology part.
- `VillageSpec` :251/:362/:419: `done.s3` → `practice.earnBread.bob.did.bake`;
  the earnBread arcs (redemption, opportunism, observation→prediction)
  re-observed green UNCHANGED otherwise.
- `GoldenDriveSpec`: UNCHANGED and re-run — the transcript-identity pin
  [C-I3]. Drift = BLOCK and itemize.
- `RelevanceSpec` [S-I1, C-C1]: :26 literal atom string → `…did.P`; :55-57/:65
  improvable pins re-OBSERVED (expected to hold — ledger inserts may-unify the
  same way — but observed, not assumed); :160-163 liveness comment prose
  updated, classification re-derived.
- `AnalysisTableSpec` villagePin's three project rows [C-C2]: expected
  byte-stable (label-derived); re-run and OBSERVED stable — any movement is
  itemized against the spec's classification.

Suite green at end (baseline 679; report the delta). Commit "v52 T1: ".

## T2 — Docs

LEDGER v52 row (the bank item closes SHIPPED; the framing inversion recorded —
the audit called `done.sN` the shadow, the user's parts-model made the CURSOR
the artifact; panel verdicts and the didPart death). README:199-206 and
WALKTHROUGH:1247/:1364/:1994 — the staged/cursor prose becomes the part-set
story [C-I2: keyed to Stage-model prose, not just `done.s3`]. LEDGER:132 (v24's
row) NOT touched [C-I4 — it is the record of what v24 shipped]. Death greps:
`done\.s|stage!|Stage ` zero in src/Prax/Project.hs; `done.S|done.s` zero in
test/ outside historical spec citations. Commit "Docs: v52 — ".

## Exactness ledger (what may move, nothing else)

Fiction transcripts (GoldenDrive), AnalysisTable decision fields, every
non-project pin: UNMOVED. Moves: ProjectSpec re-founding, the three VillageSpec
re-points, RelevanceSpec strings/prose, Village.hs's endeavor block, Project.hs
wholesale, the T2 doc prose. Anything else = BLOCK and trace.
