# v24 — Projects (`Prax.Project`): endeavors as staged practices

Backlog item **Projects / endeavors** (`docs/LEDGER.md`, "Sandbox extension backlog", Tier 2):
long-horizon behavior for long-lived characters — "the symbolic answer to bounded lookahead.
Without this, a long-lived NPC is just a mood that walks around."

The load-bearing mechanism was verified against the live engine before this spec was written:
**count-based progress rewards make horizon length irrelevant** — a four-stage endeavor with one
want (+k per completed `done` fact) was pursued stage-by-stage to completion at depth 2, each
step locally rewarding, nothing end-loaded.

## Design decisions (settled in review with the user)

- **Types are authored; instances emerge.** A project *type* is hand-crafted vocabulary, like
  practices, deeds, and desires (the sim does not invent endeavors — means-end synthesis is a
  distant research tier, not smuggled in). A project *instance* spawns spontaneously when a
  disposed character's planner chooses the authored **undertake** action — nothing scheduled,
  nothing scripted.
- **The pursuit-desire is carried but dormant.** The named desire `pursues-<id>` counts the
  owner's own instance's completed stages; with no instance it has zero bindings and zero
  utility. Disposed characters carry the name permanently in `charDesires`; undertaking
  activates it. This resolves dynamic desire acquisition (old #21) with no engine change:
  conditioned wants *are* injectable wants.
- **Long-term by construction**: the `stage!` slot is ordinary persistent state — projects
  survive interruption, save/resume, and interleave freely with the social simulation.
  Multiple owners run the same type independently (role-agnostic practices); one owner may run
  several types concurrently (one instance per type per owner).
- **The honest psychology**: the agent pursues because *progress itself is rewarding* (the
  authored pursuit weight = how invested this character is in this kind of work), not because
  the planner derives the end from the means at full depth — which is the truer model of
  commitment anyway.

Provenance: the staged-practice shape is source-native (the paper's practices are exactly
"hierarchical collections of affordances"); the progress-reward pattern and the named pursuit
are our authored idiom over `Prax.Minds`.

## 1. API (`Prax.Project`)

```haskell
data Stage = Stage
  { stageLabel  :: String       -- action label, e.g. "[Actor]: sweep the square"
  , stageNeeds  :: [Condition]  -- extra preconditions (world resources, places, …)
  , stageYields :: [Outcome]    -- extra effects (consume/produce world facts)
  }

-- | An authored endeavor: returns the undertake action (for the world to slot
-- into one of its own practices), the staged practice definition, and the
-- named pursuit desire (for the world's vocabulary).
endeavor :: String        -- ^ project id, e.g. "earnBread"
         -> Int           -- ^ pursuit weight: +w per completed stage (authored investment)
         -> String        -- ^ undertake label, e.g. "[Actor]: take up honest work"
         -> [Condition]   -- ^ undertake gate (who/when may start; may be [])
         -> [Stage]
         -> (Action, Practice, Desire)
```

Generated pieces, for `endeavor pid w ulabel gate stages`:

- **Undertake action**: `action ulabel (gate ++ [ Not ("practice." ++ pid ++ ".Actor") ])
  [ Insert ("practice." ++ pid ++ ".Actor") ]` — spawning the instance; the practice's `init`
  seeds `practice.<pid>.Owner.stage!0`. The `Not` gives one-instance-per-owner (undertaking
  again while one runs, or after completion, is not offered — a finished instance persists as
  the record of the work).
- **The staged practice** (`practiceId = pid`, `roles = ["Owner"]`): stage `k` (1-based) is
  `action (stageLabel s) ([ Eq "Actor" "Owner"
                          , Match ("practice." ++ pid ++ ".Owner.stage!" ++ show (k-1)) ]
                          ++ stageNeeds s)
    ([ Insert ("practice." ++ pid ++ ".Owner.stage!" ++ show k)
     , Insert ("practice." ++ pid ++ ".Owner.done.s" ++ show k) ]
     ++ stageYields s)` — exclusion-slot progression plus an accumulating done-record.
- **The pursuit desire**:
  `Desire ("pursues-" ++ pid) (Want [ Match ("practice." ++ pid ++ ".Owner.done.S") ] w)` —
  dormant without an instance; +w per completed stage with one.

Loud errors (the established guard idiom): an empty stage list, or a `pid` containing `.`/`!`
(it becomes a path segment).

### Theory-of-mind, for free

Because the pursuit is a **named desire**, everything from v23 applies: a character who comes
to believe `P.believes.desires.bob.pursues-earnBread` (professed, presumed, confided,
gossiped — or *inferred from watching him work*, below) predicts his next stage with
`predictMove`. A project kept quiet is invisible industry; a project observed is legible
purpose. Since `endeavor` *generates* its stage actions, observability rides in `stageYields`:
`Prax.Witness` exports the v19 deposit as a first-class outcome —
`witnessed :: CoPresence -> String -> Outcome` (with `observable` refactored to append it, a
behavior-preserving one-liner) — so a world writes `stageYields = [ witnessed together
"swept.Actor" ]` and `Prax.Project` stays decoupled from Witness. A v21-style inference axiom
then turns the witnessed stage into a presumed pursuit — learning someone's project by seeing
them at it.

## 2. Demo: bob's redemption (the village completes its moral arc)

The village gains the endeavor **`earnBread`** (pursuit weight **+3**, authored: the steady
satisfaction of honest work — real, but no substitute for bread in hand):

1. `[Actor]: sweep the square` — at the square; **observable** (`swept.Actor`): honest work is
   public.
2. `[Actor]: fetch flour from the mill` — at the mill (movement integrates; the endeavor walks
   him across the map).
3. `[Actor]: bake and earn the loaf` — at the square; yields `holding.Owner.loaf`.

- **bob is the disposed character**: `charDesires` gains `pursues-earnBread` (dormant). The
  undertake action sits in `villageP` with gate `[ Match "practice.world.world.at.Actor!square" ]`.
- **The point**: bob's unchanged `+10` loaf-want finally has a lawful path. In free play from
  t=0 — where concealment has him waiting forever, watched — undertaking is his best move: the
  planner sees stage 1 completing at the next ply. Deterrence (v21–23) plus opportunity (v24)
  yields **industry**, with no edits to any older want.
- **The tension stays honest**: bob remains an opportunist. If the square genuinely empties
  mid-project, stealing (+10 now, secret kept) still beats the next +3 stage — industry under
  observation, larceny in the dark. This is asserted as a test, not smoothed over: the
  redemption is circumstantial, which is exactly what the deterrence model claims.
- **Watching him work teaches the village his purpose**: stage 1 is observable, and one
  inference axiom (`Regarder.believes.swept.bob ⇒
  Regarder.believes.desires.bob.pursues-earnBread.presumed`) lets witnesses infer the project —
  after which `predictMove` anticipates his mill trip. Purpose becomes legible from behavior,
  through authored inference, not mind-reading.

## 3. Tests (TDD)

- `ProjectSpec` (minimal inline fixture): undertake spawns the instance and `init` seeds
  `stage!0`; undertaking twice is not offered; stages fire only in order and only for the
  owner; `stageNeeds` gate (a stage blocked until the world provides); `stageYields` fire;
  `done` facts accumulate; the pursuit desire's exact shape; **dormancy** (zero utility, and
  `predictMove` of a believed-but-instanceless pursuer is `Nothing`); the horizon regression
  (a 4-stage endeavor pursued to completion at depth 2 — the probe, pinned); loud errors
  (empty stages; a dotted pid).
- `VillageSpec` additions: from t=0 free play, watched bob undertakes and completes the
  endeavor (holds the loaf, stall intact, no theft beliefs about him); **the opportunism
  test** — square emptied mid-project, bob steals instead of continuing; the
  observation→prediction chain (a witness of the sweep comes to presume the pursuit and
  `predictMove`s the flour trip; a non-witness doesn't).
- Regression: full suite green (268 baseline); older village arcs unaffected (they force the
  theft before bob's first free choice, or run post-theft states where the endeavor's rewards
  don't change the analyzed decisions — verify empirically, BLOCKED with traces if not).
  **One sanctioned amendment, found in implementation**: the v21 deterrence test asserted
  "bob holds no loaf" as a *proxy* for "he never re-steals" — redemption falsifies the proxy
  (he now EARNS a loaf) while strengthening the property. The test is amended to assert
  non-re-offense directly: the stall's loaf untouched ∧ `atoned.bob` intact (a re-steal would
  delete both) ∧ the held loaf is the earned one (`done.s3`). Tests must verify the behavior,
  not an era's proxy for it;
  `prax check` all 7 worlds; `cabal build all` zero warnings; hlint clean.

## 4. Out of scope (parked deliberately)

- Project-type synthesis (agents inventing endeavors) — generative planning, research tier.
- Abandonment (retracting an instance) and cooperative multi-owner projects — banked.
- Persona (v25): traits as bundles of vocabulary desires — including project dispositions —
  turning the v18 sketches into a cast generator. Next round.
