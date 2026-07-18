# v50 — Machinery state leaves the db: the die into the engine; the scene stamp into nothing

First of the two extension rounds the queue-wide byte-identity assessment produced
(v50 → v51). The v45 finding this round executes: `seed!N` and `sceneEntered!N` are
engine mechanism living as queryable world facts — v45 FENCED them (access guards)
instead of fixing them, a deferral the assessment ruled out of order. Both families
now leave the world; both v45 fences are DELETED as obsolete.

AMENDED after the three-lens pre-gate panel (`.superpowers/sdd/v50-spec-review-*.md`):
the panel confirmed both load-bearing timing arguments byte-exact, converged on one
Critical (marker emission must ride scene ENTRY, not transitions — the only shipped
timed junction sits on a start scene entered at compile time), and its remaining
findings are folded below with [S]/[D]/[C] citations. One panel note governs the
whole plan: the cabal file is `-Wall` WITHOUT `-Werror`, and several outcome walkers
are list-comprehension filters that skip an unknown constructor SILENTLY — so the
walker enumeration below is load-bearing, not a courtesy [S-I2, C-C1, C-C2].

## Classification, stated first (the v49 lesson)

This round replaces the STATE RESIDENCE of two mechanisms whose SEMANTICS are
approved and stay: the RNG's Lehmer stream and frozen-die law (v38), and the timed
junction's fire-at-entry-plus-n fiction (v46). Byte-identical behavior is therefore
the fidelity EVIDENCE here — the same stream must advance at the same events; the
same junctions must fire at the same boundaries — not a constraint bending a design:
nothing about what these mechanisms DO is being redesigned, only where their
bookkeeping lives. Divergence = a residence-move bug, BLOCK and trace.

## 1. The die becomes engine state

- `PraxState` gains `rngSeed :: Maybe Integer` (`Nothing` = unseeded; Integer
  because the db's Calc arithmetic runs in Integer — byte-identity requires the
  same domain [plan review M3]). `Prax.Rng` keeps
  `draw`'s signature and guards; `rngSetup` DIES — worlds call
  `seedDie :: Integer -> PraxState -> PraxState` (Village's one call site swaps).
- `draw num den conds outs` compiles to a new outcome form — `Roll num den conds
  outs` / cooked `CRoll` — which `performCooked` executes against the engine state:
  advance the stream UNCONDITIONALLY (the frozen-die law: every draw spends one
  step, hit or miss, exactly as today's unconditional advance-ForEach), then roll on
  the advanced value and, on a hit, apply the guarded body exactly as today's
  roll-ForEach did (same snapshot semantics). Executing a `Roll` on an unseeded
  state is a LOUD error (impossible in a typechecked world — see below — and
  silence is banned).
- The `seed!*` family vanishes: no fact, no `seedPath`, nothing to guard — the v45
  reserved-table row is deleted WITH its fence-era pins (the family is
  unconstructable, which is strictly stronger than guarded).
- Consumers re-plumb: `SeedlessDraw` detects draws STRUCTURALLY (a `Roll` anywhere
  in authored outcomes) and checks `rngSeed` — stronger than today's
  seed-family-pattern sniffing (its report text in `app/Main.hs:88` names the dying
  `rngSetup` and re-words [C-I4]).
- **The walker enumeration — every `Outcome`/`CookedOutcome` traversal gains its
  arm, conservatively (a `Roll`'s body counts on both polarities: the roll may
  hit)** [S-I2, C-C1, C-C2, C-I1, D-M1]. Loud (`case`, `-Wall` warns): the codec's
  `ToJSON`, `groundOutcome`/`cookOutcome`/`groundCookedOutcome`/`writesOf`, Persist,
  Show sites. SILENT (list-comps/total walks — the ones only this list protects):
  `outcomeVars` (Types.hs:110 — the hygiene walk behind `authoredVarClash`; a
  missing arm is a partial function AND stops namespace-checking draw bodies),
  `outcomeCondReads` (Relevance.hs:297 — the READ-anchor side; skipping it
  under-approximates reads, the direction that sleeps through an arc),
  `outcomeRef` (TypeCheck.hs:185-194 — catch-all `_ -> []`: without an arm, a
  dangling function/practice reference inside a draw body silently stops being
  flagged; the plan review's Critical — the enumeration itself had a gap),
  `inserts` (TypeCheck.hs:212), `outcomeGuards` (:347), `forEachGuards` (:434),
  the sent-walks (:483-490), `cookedOutcomeAtoms`, `outcomeDeltaAnchors`,
  `outcomeUses` (TypeCheck.hs:111-118 — no wildcard, so merely loud, but listed:
  the checklist is checked by name, not by warning), and
  `FromJSON Outcome` (Script/Json.hs:133-142 — `<|>`-chained, a missing arm fails
  at PARSE time with no warning; the arm is totality, not an authoring surface
  [D-M4]).
- Persist: `rngseed <n>` line, emitted only for seeded states (most worlds are
  unseeded and their saves gain no line [C-Minor]); header → `prax-state v4`, the
  rejection ladder gains its row (the stream position was db-carried state and must
  survive saves; the v43 machinery does its job again).

## 2. The scene stamp dies entirely — timed junctions ride v44's own primitive

The elegant discovery: `sceneEntered` never needed to move — it needs to not exist.
A timed junction means "fire n boundaries after entry," and the engine already owns
exactly that shape: EXPIRY. **Scene ENTRY** — the shared entry effect (`setupOf`,
where today's dying stamp lives), covering ALL THREE entry paths: the compile-time
initial scene at turn 0, boundary transitions, and re-entry [S-C1, D-M3; the panel's
one Critical: the only shipped timed junction, Audience's `timeout "dismissed" 5`,
sits on a START scene with no transition — transition-only emission would fire it at
boundary 1] — compiles, per timed junction J with delay n, an
`InsertFor n (scenePatience.<sid>.<J>)` — a PATIENCE MARKER that the schedule
retracts n boundaries later. This covers `timeout` (ending) and `after` (timed goto)
alike: both ride the same junction expansion, gated per-junction [C-I2]. The timeout
clause's conditions become `currentScene!sid ∧ Not (scenePatience.<sid>.<J>)`: the
junction fires when the patience has RUN OUT (the fiction says itself). Timed delays
are guarded n ≥ 1 at `compile` — the consumption point, covering raw and JSON
construction alike, not just the combinators — loud [D-I2: at n=0 today's `≥` fires
same-boundary in a cascade while the marker form waits one boundary — a zero-delay
"timed" junction is a plain junction, and the divergent case becomes
unrepresentable rather than silently different]. `clockReached`, the `sceneEntered`
family, its stamp `ForEach`, its `turnPath` read in Script, and its v45 reserved-
table row ALL die.

Timing, argued exactly: entry at boundary B inserts the marker with lifetime n; the
v44 boundary order (expiries BEFORE rules) retracts it at boundary B+n before the
story rule evaluates — the timeout clause is first eligible at B+n, which is
precisely today's `turn − entered ≥ n` with the v46 stamp taken post-clock-advance.
Re-entry REFRESHES the marker (v44's supersession law — the clock resets, as
re-stamping does today). Early exit leaves a pending expiry that fires harmlessly
(the clause's `currentScene` gate is false; the retract is a no-op family-wise).
Multiple timed junctions per scene each carry their own marker and delay.

`scenePatience.*` rides v44's public expiry primitive, but the panel corrected the
first draft's category claim [D-I1]: it is COMPILER MACHINERY, not fiction — produced
only by `setupOf`, read only by the story rule (the same production/consumption shape
`sceneEntered` had). It is not added to the v45 reserved table for a STRUCTURAL
reason, stated honestly: the table's machinery-shape passkey cannot express it (the
marker's tail is literal, and the story rule lives in the schedule that `writeSites`
scans — reserving the family would trip the compiler's own insert). The protection is
instead a compile-time rejection in `Prax.Script`: an authored condition or outcome
touching `scenePatience.*` (either polarity) fails compilation loudly — the same
unrepresentability standard, enforced at the layer that can see who's asking. This
also closes the collision hole (an author's own `scenePatience.foo` insert would
otherwise silently corrupt a timeout). READING scene patience is not an authoring
feature; a story that wants visible impatience authors its own fiction.

## What dies

`rngSetup`, `seedPath`, the `seed!*` family and its reserved-table row + pins;
`sceneEnteredPath`, the `sceneEntered` family, its stamp machinery, `clockReached`,
and its reserved-table row + pins. Both v45 fences: deleted as obsolete, per the
assessment's charge — the design fix makes the guards unnecessary rather than
load-bearing.

## Exactness

Both halves behavior-identical BY THE CLASSIFICATION above — with the claim scoped
precisely [S-I1, D-M2, C-Minor]: FICTION TRANSCRIPTS byte-identical (the same
stream, the same junction boundaries — Audience's `timeout "dismissed" 5` fires at
the same fictional moment; the panel confirmed no golden dumps raw db); pins that
INSPECT the dying state necessarily move and are itemized here, not discovered
[C-C3, C-I4]:
- RngSpec's rngSetup/seed-fact model pins (~10) — re-founded at the engine form,
  same properties (stream, frozen-die law, sequential multi-draw);
- ScriptSpec:210-215 (reads `sceneEntered.E`, pins the stamp values) — dies with
  the stamp; its property (entry re-stamps; timing) re-pins on the marker form;
  ScriptSpec:174-185's goto-reuses-stamp premise is deleted with the premise;
- VillageSpec:802/827/829 (seed-fact touches) — re-plumbed;
- TypeCheckSpec: the v45 fence pins for both families AND the :327 exemption pin
  (which is not a fence pin and would survive a naive "delete the fence pins"
  sweep) — all deleted;
- PersistSpec: v3 fixtures → v4 + ladder row;
- AnalysisTable pins move ONLY where the dead families' atoms appeared, and the
  dead-condition lint's `lintSites` shrink (draw guards leave the authored-ForEach
  pool for the `Roll` walk) is part of that enumeration [C-I1]; the *decision*
  fields must not move.
Persist v4 with v3-rejection pinned. The junction half's persistence is SYMMETRIC
and free [C-I3]: the patience marker is an ordinary fact and its expiry rides v44's
existing due serialization — stated here so the plan pins it rather than assumes it.

## Verification

- The die: a fixed-seed world's draw sequence IDENTICAL across the move (the
  village temper goldens are the live pin); the frozen-die law re-pinned at the
  engine (a missed roll still advances — the v38 pin's engine-state form); unseeded
  `Roll` execution loud; `SeedlessDraw` structural pins (draw-without-seedDie flags;
  seeded world clean); Persist v4 round-trips the stream position (save mid-stream,
  resume, identical continuation — the pin the residence move makes newly
  meaningful).
- The junctions: Audience's dismissal at the same boundary (the fiction pin, and
  it exercises the START-scene entry path the panel's Critical protected);
  re-entry-resets pinned (a scene left and re-entered times out n from the LAST
  entry); early-exit harmlessness pinned; multiple timed junctions per scene pinned
  (distinct markers, distinct delays); a timed `after` (goto) pinned alongside
  `timeout` [C-I2]; n=0 rejection pinned [D-I2]; the authored `scenePatience.*`
  touch rejection pinned both polarities [D-I1]; a mid-scene save/resume continues
  to the SAME timeout boundary (the persistence symmetry pin [C-I3]).
- Deaths grep-proof: `rngSetup|seedPath|sceneEntered|clockReached` live nowhere in
  src/ OR app/ [C-I4: Main.hs's SeedlessDraw text names rngSetup today].
- Pre-gate: the three-lens panel ran on this document; verdicts UNSOUND-as-written /
  FLAWED / GAPS, all findings folded above; the amended spec is what gates.

## Out of scope

v51 (lifting leaves the engine — next and last). Generic fact-age queries ("how
long has F held") — the patience-marker pattern covers the one consumer without new
query semantics; banked if a second consumer ever appears. Any RNG-semantics change.
