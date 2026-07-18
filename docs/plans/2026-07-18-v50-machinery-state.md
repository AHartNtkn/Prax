# Plan v50 — machinery state leaves the db

Governing spec: `docs/specs/2026-07-18-v50-machinery-state.md` (panel-amended; the
[S]/[D]/[C] citations there govern here). Three tasks, sequential, implementer +
reviewer each; RED-first per behavior; the fiction goldens are the fidelity pins
and must NOT move. `-Wall` has no `-Werror` — the walker enumeration in T1 is
checked off item by item, not left to warnings.

## T1 — The die becomes engine state

**Why each piece exists:** the round's charter (engine state must not be queryable
world fact) forces the seed out of the db; a fact-free die forces a first-class
outcome form (`Roll`) because conditions/outcomes can no longer reach the stream;
that new constructor forces an arm in EVERY outcome walker (several are silent
list-comps — the spec's enumeration is the checklist); state that moved residence
forces Persist v4; and the fence that guarded the fact dies with the fact.

- `Prax.Types`: add `Roll Int Int [Condition] [Outcome]` to `Outcome` and
  `CRoll Int Int [CookedCondition] [CookedOutcome]` to `CookedOutcome` (haddock:
  the drama die — advance the engine stream unconditionally, roll on the advanced
  value, on a hit apply the body as a `ForEach`; spec v50). `outcomeVars` gains
  `Roll _ _ cs os -> concatMap conditionVars cs ++ concatMap outcomeVars os`
  (same shape as the `ForEach` arm).
- `Prax.Rng` rewritten: `seedPath`/`rngSetup` DIE. Exports become `draw`,
  `rollStep`, `seedBounds` (whatever minimal names fit): the module keeps ALL die
  math — `lehmerA`/`lehmerM`, `rollStep :: Integer -> (Integer, Integer)` (the
  advanced seed and the advanced value — one Lehmer step), and the seed-domain
  bounds check. `draw num den conds outs` keeps its two guards verbatim (odds
  bounds; `authoredVarClash [] conds outs` — v40 hygiene stands even though the
  splice motive is gone) and compiles to `[ Roll num den conds outs ]`.
- `Prax.Engine`: `PraxState` gains `rngSeed :: Maybe Integer` (`Nothing` in
  `emptyState` — Integer, matching today's Calc arithmetic domain exactly).
  `seedDie :: Integer -> PraxState -> PraxState` beside the other state setters
  (`setSchedule`/`defineFunctions` precedent), loud on out-of-domain via Rng's
  bounds. `cookOutcome`/`groundOutcome`/`groundCookedOutcome`/`writesOf` gain
  their arms (body treated as `ForEach`'s is). `performCooked` gains `CRoll num
  den conds outs`:

  ```haskell
  CRoll num den conds outs -> case rngSeed st of
    Nothing -> error "Prax.Engine: Roll executed on an unseeded die \
                     \(a draw in a world that never called seedDie)"
    Just s  ->
      let s' = fst (rollStep s)          -- the frozen-die law: spent, hit or miss
          st1 = st { rngSeed = Just s' }
      in if s' `mod` fromIntegral den < fromIntegral num
           then perform st1 (CForEach conds outs)   -- same snapshot semantics
           else st1
  ```

  (Exact shapes to match the existing `performCooked` idiom; the byte-identity
  argument — roll on the ADVANCED value, guards inside the hit body — is the
  panel-verified mapping of today's two-ForEach compile.)
- `Prax.Relevance`: `outcomeCondReads` (:297) gains the `CRoll` arm (conds read
  like `CForEach`'s — the READ side the first draft missed [C-C2]);
  `cookedOutcomeAtoms`/`outcomeDeltaAnchors` gain write-side arms (body counts,
  the roll may hit).
- `Prax.TypeCheck`: `inserts` (:212), `outcomeGuards` (:347), `forEachGuards`
  (:434), the sent-walks (:483-490) each gain the arm — these are the SILENT
  ones; the reviewer checks each by name. The seed reserved-table row (:283) and
  `outcomeUsesSeed` DIE; `SeedlessDraw` becomes structural: any `Roll` reachable
  in authored outcomes (practices + schedule) with `rngSeed st == Nothing` flags;
  its `app/Main.hs:88` report text re-worded (no `rngSetup` mention).
- `Prax.Script/Json`: `ToJSON`/`FromJSON` `Outcome` arms for `Roll` (totality,
  not authoring [D-M4]; the `FromJSON` `<|>` chain is one of the silent sites).
- `Prax.Persist`: header → `prax-state v4`; a `rngseed <n>` line emitted iff
  `Just`, parsed back; v3 joins the rejection ladder.
- `Prax.Worlds.Village`: `rngSetup villageSeed` in setup → `seedDie villageSeed`
  wrapping the state build (the one call site).

**Tests (RED observed per behavior):** RngSpec re-founded — stream advance
(rollStep values pinned against the Park-Miller constants), frozen-die law (a
missed roll advances: two identical draws at the same stream position produce
DIFFERENT subsequent draws), sequential multi-draw (Village.hs:347-348 shape),
unseeded-Roll loud error, seedDie domain guard, draw odds/hygiene guards
(surviving pins re-pointed). VillageSpec :802/:827/:829 re-plumbed; the temper
goldens UNCHANGED (the fidelity pin — if they move, BLOCK and trace). TypeCheckSpec:
seed fence pins and the :327 exemption pin deleted; SeedlessDraw structural pins
(unseeded world with a draw flags; seeded world clean; a draw nested under
ForEach still found). PersistSpec: v4 fixtures, `rngseed` round-trip, unseeded
save has no line, v3-rejection row, and the die's mid-stream save/resume pin
(save between draws, resume, identical continuation).

## T2 — The scene stamp dies; timed junctions ride expiry

**Why each piece exists:** the charter kills `sceneEntered`; a timed junction
still needs "n boundaries after entry," which v44's expiry already expresses —
so entry emits a patience marker instead of a stamp; markers per (scene,
junction) force nothing new (junction names are already unique per scene at
compile); compiler-owned markers force the authored-touch rejection [D-I1]
because the v45 table structurally cannot hold them; and the n=0 divergence
forces the n ≥ 1 compile guard [D-I2].

- `Prax.Script`:
  - `sceneEnteredPath`, `clockReached`, `stampsSceneEntry`, and `setupOf`'s
    stamp-`ForEach` DIE.
  - `scenePatiencePath sid jname = "scenePatience." ++ sid ++ "." ++ jname`
    (internal). `setupOf sid` emits, for each timed junction `j` of scene `sid`:
    `InsertFor n (scenePatiencePath sid (junctionName j))` — a plain literal
    insert (today's stamp needed `ForEach` to capture the live clock; the marker
    needs nothing — simpler than what it replaces). All three entry paths ride
    `setupOf` already: compile-time start via `setup`, transitions via
    `storyClause`, re-entry via the same [S-C1]. Re-entry refresh = v44's
    supersession law, for free.
  - `storyClause`: `maybe [] clockReached (junctionAfter j)` becomes
    `maybe [] (\_ -> [ Not (scenePatiencePath sid (junctionName j)) ])
    (junctionAfter j)` (the `currentScene!sid` gate is already in the clause).
  - `compile` gains two loud guards, at the consumption point (uniform over
    smart-ctor / raw / JSON construction, per the existing hygiene-guard
    precedent stated in `compile`'s own haddock): (1) `junctionAfter j == Just n`
    with `n < 1` → error (a zero-delay timed junction is a plain junction — and
    n=0 is where the marker form diverges from the old arithmetic [D-I2]);
    (2) any authored condition or outcome `compile` consumes (sceneSetup, beat
    conditions/effects, junctionWhen — every authored list it already sweeps for
    hygiene) touching a path headed `scenePatience` (either polarity) → error
    naming the site (the collision hole [D-I1]).
- `Prax.TypeCheck`: the `sceneEntered` reserved row (:284) dies; import gone.
- Persist: NOTHING — the marker is an ordinary fact, its pending retract rides
  v44's due serialization (Persist.hs:88) [C-I3]; the pin below proves it.

**Tests (RED observed per behavior):** ScriptSpec — the stamp pins (:210-215)
and the goto-stamp premise (:174-185) die with the stamp; new pins: Audience's
`timeout "dismissed" 5` fires at the SAME boundary as before the move (the
fiction pin, on the start-scene path the panel's Critical protected — RED by
emitting from transitions only, observing boundary-1 firing, then GREEN via
`setupOf`); a timed `after` goto fires at its boundary [C-I2]; re-entry resets
(leave, re-enter, times out n from the LAST entry); early exit harmless (leave
before expiry; no stray firing later); two timed junctions on one scene with
distinct delays fire independently; `after`/`timeout` with n=0 rejected loud;
authored `scenePatience` touch rejected both polarities; mid-scene save/resume
reaches the SAME timeout boundary (the persistence-symmetry pin [C-I3]).
TypeCheckSpec: sceneEntered fence pins deleted. Full-suite green; Audience/story
goldens unchanged.

## T3 — Docs

LEDGER v50 row (residence-move classification stated; the panel's Critical and
the walker enumeration recorded; both v45 fences noted deleted-as-obsolete);
README (Rng bullet: the die is engine state, `seedDie`; Script bullet: patience
markers); WALKTHROUGH's draw/junction prose; Rng/Script/Engine haddocks already
updated in T1/T2 — docs task verifies module docs against shipped behavior
(provenance rules per house practice). Death grep across src/ AND app/ [C-I4]:
`rngSetup|seedPath|sceneEntered|clockReached` → zero.

## Exactness ledger (what may move, nothing else)

Fiction transcripts/goldens: BYTE-IDENTICAL (Village tempers, Audience
dismissal boundary). Moves, all itemized in the spec's Exactness section:
RngSpec re-founding, ScriptSpec stamp pins, VillageSpec seed touches,
TypeCheckSpec fence+exemption pins, PersistSpec v4, AnalysisTable rows where
seed/sceneEntered atoms appeared + the lint's authored-ForEach pool shrink
(draw guards now live under `Roll`) — decision fields must not move. Anything
outside this list moving = BLOCK and trace.
