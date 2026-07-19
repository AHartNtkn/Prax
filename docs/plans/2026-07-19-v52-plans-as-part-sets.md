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
  ```

  The full body (transcribe, don't redesign — the review pinned every guard
  and the reachability algorithm as TRANSITIVE, a fixpoint, because a one-hop
  check would miss `A after [B], B after [C], C after [B]`; reachability from
  edge-free roots IS complete cycle detection — a graph with no edge-free node
  contains a cycle, and every cycle participant or dependent is unreachable):

  ```haskell
  endeavor pid w ulabel gate parts
    | null parts =
        error ("endeavor: " ++ show pid ++ " has no parts (an endeavor is work)")
    | any (`elem` (".!" :: String)) pid =
        error ("endeavor: id " ++ show pid ++ " must be a single path segment")
    | (n : _) <- [ n | n <- names, any (`elem` (".!" :: String)) n ] =
        error ("endeavor " ++ show pid ++ ": part name " ++ show n
               ++ " must be a single path segment (it keys the ledger)")
    | (n : _) <- names \\ nub names =
        error ("endeavor " ++ show pid ++ ": duplicate part name " ++ show n)
    | ((p, e) : _) <- [ (partName p, e) | p <- parts
                      , e <- partAfter p, e `notElem` names ] =
        error ("endeavor " ++ show pid ++ ": part " ++ show p
               ++ " depends on " ++ show e ++ ", which is not a part")
    | (v : _) <- concat [ authoredVarClash [] (partNeeds p) (partYields p)
                        | p <- parts ] =
        error ("endeavor " ++ show pid ++ ": part authors " ++ show v
               ++ " -- the Prax namespace is reserved")
    | (n : _) <- filter (`notElem` reachable) names =
        error ("endeavor " ++ show pid ++ ": part " ++ show n
               ++ " is unreachable (its dependency edges form or feed a cycle)")
    | otherwise = (undertake, proj, pursuit)
    where
      names = map partName parts
      -- Transitive reachability from the edge-free roots (fixpoint).
      reachable = go [ partName p | p <- parts, null (partAfter p) ]
        where
          go acc =
            let acc' = nub (acc ++ [ partName p | p <- parts
                                   , all (`elem` acc) (partAfter p) ])
            in if length acc' == length acc then acc else go acc'
      inst suffix = "practice." ++ pid ++ suffix
      ledger n = inst (".Owner.did." ++ n)
      undertake = action ulabel (gate ++ [ Not (inst ".Actor") ])
                    [ Insert (inst ".Actor") ]
      -- No instance-fact Match: instance existence and Owner's binding ride
      -- the practice-instance ENUMERATION (the undertake fact's trie node),
      -- exactly as they always did — the review traced that the old stage
      -- gate never bound Owner either [review #1]. No init seed: the stage!0
      -- cursor is dead and nothing reads the family.
      partAction p = action (partLabel p)
        ([ Eq "Actor" "Owner", Not (ledger (partName p)) ]
         ++ [ Match (ledger d) | d <- partAfter p ]
         ++ partNeeds p)
        (Insert (ledger (partName p)) : partYields p)
      proj = practice
        { practiceId   = pid
        , practiceName = "[Owner] pursues " ++ pid
        , roles        = ["Owner"]
        , initOutcomes = []
        , actions      = map partAction parts }
      pursuit = Desire ("pursues-" ++ pid)
                  (Want [ Match (inst ".Owner.did.P") ] w)
  ```

  (The v40 `authoredVarClash` guard is NEW versus today's endeavor — the spec
  claims "v40-guarded" and the review found neither code nor plan backing it
  [review #7]; option (a), add it, chosen: cheap, loud, matches the spec.)
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
     after). RED neuter [review #5]: compile the gate so a completed part
     suppresses its SIBLINGS (add `Not` of every other ledger entry) — the pin
     asserting the un-chosen edge-free part still offered FAILS, for the named
     reason; then ship the real gate, GREEN.
  3. optional part (culmination fires with an optional part undone; the
     optional still pays +w after — score-compared via evaluate). RED neuter:
     compile the culmination's gate over ALL parts rather than its `partAfter`
     list — the pin asserting it fires with the optional undone FAILS.
  4. threshold success (a 5-part endeavor whose culminating part carries
     `[ Subquery "Done" ["P"] [ Match ("practice.<pid>.Owner.did.P") ]
      , Count "N" "Done", Cmp Gte "N" "3" ]` — the review traced this exact
     shape through queryCooked, arities and semantics confirmed [review #4] —
     fires exactly at 3 of 5). RED neuter: drop or off-by-one the `Cmp` — the
     fires-at-3-not-2 assertion FAILS.
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
