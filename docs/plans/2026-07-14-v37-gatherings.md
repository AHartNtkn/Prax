# v37 — Calendar & Gatherings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** the clock convenes and the town shows up, per
`docs/specs/2026-07-14-v37-gatherings.md`: the ticker-as-environment reclassification (the
v36 gate-precision regression repaired; clock-moved facts gate desires again), the
`gathering` combinator, and village market day with the percolation payoff.

**Architecture:** one exclusion point in `Prax.Relevance.worldAtomPools` (the drift
practice's outcomes are environment dynamics, not authored affordances); one combinator in
`Prax.Drift` composing existing pulse rules; village cargo re-using every social system as
is. No planner/engine/query change.

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- Exactness argument as specced: the improvability screen concerns mover actions (a
  clock-only-improvable desire has no improving mover-action to find); gate checks read
  present state. Conservativity direction unchanged. Goldens: village re-captured (own
  commit, itemized, arcs re-verified with re-indexes traced); bar/intrigue/feud MUST NOT
  move (bar has drift but no gated positive desire — verify, don't assume: if the bar
  golden moves under Task 1, that is a real reclassification consequence to itemize, not
  noise — BLOCK and report).
- Authored meaning sentences on every period/duration/weight. Loud guards. Zero warnings;
  hlint "No hints"; ViewInvariant green; RED-first observed (v36 Task 2's lapse is not
  repeated).
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: The ticker is the environment

**Files:**
- Modify: `src/Prax/Drift.hs` (export the practice id), `src/Prax/Relevance.hs` (the
  exclusion), `test/Prax/RelevanceSpec.hs`.

**Design.** `Prax.Drift` gains (and exports, used by `driftP`'s own `practiceId` field so
there is one home for the name):

```haskell
-- | The drift practice's id — exported for "Prax.Relevance", whose analyses
-- treat this practice's outcomes as ENVIRONMENT dynamics, not authored
-- affordances (spec 2026-07-14-v37: tickers change motives; a clock-moved
-- fact is exactly what an environment gate is FOR).
driftPracticeId :: String
driftPracticeId = "drift"
```

`Prax.Relevance.worldAtomPools` excludes it at its single entry:

```haskell
worldAtomPools :: Map String Practice -> AtomPools
worldAtomPools allDefs = AtomPools
  { ... }   -- body unchanged
  where
    -- The drifter's outcomes are the world acting on itself: excluding them
    -- restores the v33 environment-gate semantics to clock-moved facts
    -- (hungry.*, marketDay.*) — a desire only the clock can improve has no
    -- improving MOVER action, so the static screen stays exact, and its
    -- liveness becomes a GateCheck the pulse flips (the v35 wake).
    defs = Map.delete driftPracticeId allDefs
    practices = Map.elems defs
    ...       -- the existing body reads `defs` via `practices`/`fns` as today
```

(Adapt mechanically: the current body binds `practices = Map.elems defs` from its
parameter — rename the parameter and add the delete; nothing else changes. Import
`driftPracticeId` from `Prax.Drift` — no cycle, Drift imports only Types/Query/Db.)
`moverReadAnchors`/`bearingTemplates` are NOT changed: reads are reads, and the drifter's
action never appears in a person's candidate set.

**RelevanceSpec pins (RED-first — the fixture pin FAILS against today's pools):**

```haskell
  , testCase "clock-moved facts are environment gates (the ticker is not an author)" $ do
      -- "festive.now" is inserted ONLY by a drift pulse; the desire needs it
      -- plus an action-reachable conjunct. Pre-v37 the drifter's outcomes
      -- polluted the insert pool and this classified AlwaysLive; it is a
      -- GateCheck on the festive conjunct.
      let p = practice
            { practiceId = "plaza", roles = ["R"]
            , actions = [ action "[Actor]: stroll the plaza"
                            [ Match "practice.plaza.here" ]
                            [ Insert "strolled.Actor" ] ]
            }
          pulse = DriftRule "festival" 4
            [ ( [], [ Insert "festive.now" ] ) ]
          vocab = [ Desire "loves-a-crowd"
                      (Want [ Match "festive.now", Match "strolled.Owner" ] 3) ]
          st = setDesires vocab
                 (setCharacters [character "ana", driftChar]
                    (definePractices [p, driftP [pulse]] emptyState))
      Map.lookup "loves-a-crowd" (liveness st)
        @?= Just (GateCheck [[cookCondition (Match "festive.now")]])

  , testCase "action-insertable facts still never gate; the clock cannot launder them" $ do
      -- same shape, but a PERSON action also inserts festive.now — the pool
      -- sees it, no gate, AlwaysLive (conservative as ever).
      ... (the same fixture plus an authored "light the lanterns" action
           inserting festive.now; assert AlwaysLive)

  , testCase "the village hunger want-shape regains its gate under the reclassification" $ do
      -- wants-food [hungry.Owner, meal.M] in a world where ONLY the drifter
      -- inserts hungry.*: GateCheck on hungry.Owner (v36 had silently
      -- degraded this to AlwaysLive the moment the drifter joined).
      ... (v33's eatery fixture + the v36 hunger pulse; assert GateCheck)
```

(Write the two elided fixtures in full following the first's shape — each is the same
world ± one action, each with an exact `liveness` assertion. The GateCheck payload
comparison needs `cookCondition` and `Liveness`'s Eq — both exist.)

- [ ] RED observed (first + third pins fail: AlwaysLive) → implement → GREEN
  (`-p "Relevance"`, `-p "Drift"`, `-p "Village"`) → FULL suite: expect all green INCLUDING
  byte-identical goldens (the village has no positive clock-gated desire yet; the bar's
  metabolism moves drinks!N which no positive desire reads — but VERIFY; any golden
  movement = BLOCK with the itemized diff for adjudication) → gates → commit
  `"Relevance: the ticker is the environment — clock-moved facts gate again"`.

---

### Task 2: The `gathering` combinator

**Files:** `src/Prax/Drift.hs`, `test/Prax/DriftSpec.hs`.

**Design** (in Drift, beside `driftSetup`, exported):

```haskell
-- | A recurring, self-closing convening: open effects fire every @period@
-- rounds, close effects @duration@ rounds after each opening (both authored
-- meanings; 0 < duration < period — a gathering that never closes, or never
-- opens, is not a gathering). Returns the two pulse rules (give them to
-- 'driftP' with the world's other rules) and their due seeds (append to the
-- setup AFTER 'driftSetup'; the first convening lands one full period in,
-- the v36 start-sated convention).
gathering :: String -> Int -> Int -> [Outcome] -> [Outcome]
          -> ([DriftRule], [Outcome])
gathering name period duration openOuts closeOuts
  | duration < 1 || duration >= period =
      error ("Prax.Drift: gathering " ++ name
             ++ " needs 0 < duration < period")
  | otherwise =
      ( [ openR, closeR ]
      , [ Insert ("due." ++ driftRuleName openR ++ "!" ++ show period)
        , Insert ("due." ++ driftRuleName closeR
                  ++ "!" ++ show (period + duration)) ] )
  where
    openR  = DriftRule (name ++ "Open")  period [ ([], openOuts) ]
    closeR = DriftRule (name ++ "Close") period [ ([], closeOuts) ]
```

Note in the haddock: `driftSetup` must NOT also receive the gathering's rules (it would
seed both dues at `period`, opening and closing simultaneously) — worlds pass plain rules
to `driftSetup` and append the gathering's own seeds; `driftP` gets ALL rules. The name
guards ride the existing `guardRule` (via `driftP`).

**DriftSpec pins (RED-first — `gathering` missing):** using the Task-1-style hand clock:
open fires at `turn == period` and not before; close at `period + duration`; the SECOND
cycle (open at 2×period, close at 2×period+duration) — recurrence proven over two full
cycles with the due facts traced; `duration >= period` and `duration 0` both loud errors;
misuse pin: passing the gathering's rules to `driftSetup` too would double-seed — assert
the documented usage by pinning the correct one (drive the correct wiring; the misuse
itself needs no pin, the haddock warns).

- [ ] RED → implement → GREEN (`-p "Drift"`) → suite (no world changed — goldens
  untouched) → gates → commit `"Drift: gatherings — the clock convenes and closes"`.

---

### Task 3: Village market day

> **Amended mid-round:** the cadence below (`gathering "market" 2 1`, "every other round")
> shipped as **period 6, duration 1** after Task 4's bench measured 2/1 leaving no quiet
> rounds — see spec §Cargo as amended (0bcf2a3) and the LEDGER v37 row; the code below is
> the baseline it replaced.

**Files:** `src/Prax/Worlds/Village.hs`, `test/Prax/VillageSpec.hs`,
`test/Prax/LoopSpec.hs` (the wake pin), then the village golden re-capture in
`test/Prax/GoldenDriveSpec.hs` as its OWN commit.

**Design.**

```haskell
-- The market practice: an instance the calendar spawns and tears down; its
-- presence IS the event (no market-only affordances — the draw is the
-- valuation, the payoff is co-presence density feeding sight/witnessing).
marketP :: Practice
marketP = practice { practiceId = "market", roles = ["Fair"] }

-- Market day every other round, for one round — a compressed town rhythm
-- matching the compressed drives (a "day" here is one round; the golden's
-- 21-turn window then witnesses one full open-close-reopen cycle).
marketCalendar :: ([DriftRule], [Outcome])
marketCalendar = gathering "market" 2 1
  [ Insert "practice.market.fair", Insert "marketDay.square" ]
  [ Delete "practice.market.fair", Delete "marketDay.square" ]

-- Everyone likes a market day: +3 for being at the square while it's on —
-- above the +1 loitering anchors (a market beats an idle preference), below
-- the +5 event wants and every conduct stake (drama outranks festivity).
drawnToMarket :: Desire
drawnToMarket = Desire "drawn-to-market"
  (Want [ Match "marketDay.square"
        , Match "practice.world.world.at.Owner!square" ] 3)
```

Wiring: `marketP` and the calendar's rules into `definePractices`/`driftP` (the hunger
pulse stays a plain rule: `driftP (hungerPulse : fst marketCalendar)`); `driftSetup
[hungerPulse] ++ snd marketCalendar` in setup; `drawnToMarket` into the vocabulary;
`charDesires` gains `"drawn-to-market"` for bob, carol, dana, eve, and gale (all five —
the market is the town's; the player is the player). Expected interactions to TRACE, not
assume, and itemize in the golden: bob's hunger-vs-market interleaving; gale/eve arcs
re-indexed; whisper opportunities widening at market rounds.

**Pins (RED-first):**
- VillageSpec — convergence: drive to a market round; every attendee with no stronger
  stake is at the square while `marketDay.square` holds; drive past close: they disperse
  (assert at exact turn counts, derived by observation and asserted exactly).
- VillageSpec — percolation (the round's point, pinned with counts): force one witnessed
  act (`witnessed together "spat.gale.carol"` — a neutral fixture fact) at a MARKET round
  and count `believes.spat.gale.carol` holders; reset, force the same act on a QUIET
  round, count again. Assert both exact counts and market > quiet.
- LoopSpec — the wake, end-to-end (the probe's trajectory as a test): a villager's
  standing intention holds through quiet rounds (their signature quiet); the market
  opening flips `drawn-to-market` live (assert the `motiveSignature` live-set component
  changes) and their next `npcAct` re-deliberates TO the square; the close disperses them.
  This is the v35+v37 integration pin — RED against Task 1-only state requires Task 3's
  village; write it in this task.
- `liveness villageWorld`: `drawn-to-market` ⇒ `GateCheck [[marketDay gate]]` (exact
  payload), extending the existing table pin.

**The golden (own commit):** expected LARGE — a synchronized town rhythm. Itemize every
line with cause (market convening; attendance movements; hunger interleave; arc
re-indexes). Drama pins re-verified; any postTheftAt-style re-index traced with a
trajectory probe (the v36 discipline). bar/intrigue/feud goldens untouched or BLOCK.

- [ ] RED → implement → GREEN → suite → the golden re-capture (own commit, itemized) →
  gates → commits `"Village: market day — the clock convenes the town"` +
  `"Village golden: the town keeps market day (v37, itemized)"`.

---

### Task 4: The measurement + docs

**Files:** scratchpad (bench), `docs/LEDGER.md`.

- [ ] Paired drive bench (V35DriveBench shape, current library, with a pre-v37 control at
  the Task-0 base commit like v36's): drive times, deliberation counts per character,
  serve-rate. Expected: attendees re-deliberate at each open/close boundary (bounded,
  synchronized); quiet rounds stay served. If the serve-rate collapses beyond the
  open/close beats, BLOCK (a want is reading sub-threshold state). Timed full suite.
- [ ] LEDGER: v37 legend row (the probe story — calendar worked first try, attendance
  exposed the v36 pool pollution; the user's "tickers change motives" adopted as the
  semantics; the reclassification's exactness argument; the combinator; the market with
  the percolation counts; bench as measured). AMEND THE v36 ROW IN PLACE (fix-don't-
  confess): its gate-precision regression, found and repaired here, stated with the
  measurements preserved. Mark the calendar bank row done; the emotions row gains its
  cross-reference (onset gates now flip on clock events — an emotion's trigger can be a
  gathering).
- [ ] Gates; commit `"Docs: v37 — the clock convenes, the town shows up"`.
