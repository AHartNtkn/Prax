# v44 — The Schedule Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** per `docs/specs/2026-07-16-v44-the-schedule.md` (three-lens-reviewed) — the
engine owns time. Task 1 lands the core machinery INERT (lifetime inserts, the
exact-path expiry queue, the boundary function, the clock seed, Persist v2) with the
loop untouched; Task 2 is THE SWITCH (schedule surface, boundary wired, tickers and
Drift/Clock deleted, worlds migrated, tests re-expressed, goldens re-captured); Task 3
kills Script's scene clock; Task 4 docs.

**Why this split:** the boundary cannot be wired while tickers also tick (double-advance),
so ticker deletion + world migration + wiring is one unavoidable commit; everything that
CAN land green beforehand does, in Task 1.

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- Suite green per commit. Task 1 expects byte-identical goldens EXCEPT pins that
  enumerate raw fact sets of previously-clockless worlds (emptyState now carries
  `turn!0`) — those update in Task 1, itemized. Task 2/3 re-capture goldens and
  AnalysisTable pins DELIBERATELY, itemized per world in the commit body; never absorb
  an unexplained line.
- RED-first for every new behavior; the expiry-law and boundary tests are the round's
  core evidence — each law in the spec gets a named pin.
- Loud failures everywhere: unknown rule name on load, non-Insert under `lasts`,
  authored turn-writes.
- No dual systems: `feel`/`feelToward` duration variants COMPILE to the one lifetime
  outcome; the schedule bodies are walked by the same `cookedOutcomeAtoms`; Drift and
  Clock modules DIE (no wrappers).
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: The inert core — lifetime inserts, the queue, the boundary function, Persist v2

**Files:** `src/Prax/Types.hs`, `src/Prax/Cooked.hs`, `src/Prax/Engine.hs`,
`src/Prax/Persist.hs`, `src/Prax/Relevance.hs` (analysis arms), `src/Prax/TypeCheck.hs`
(outcome-walk arms), `prax.cabal` if a new spec file registers. Tests:
`test/Prax/ScheduleSpec.hs` (new — the boundary/expiry laws), `PersistSpec`, plus any
fact-enumerating pin that sees `turn!0`.

**1a. Types.** `Outcome` gains the lifetime insert; `ScheduleRule` and the state fields
land (inert — nothing populates them yet); `turnPath` moves here NOW (Task 2 deletes
`Prax.Clock`; landing the constant's one home early keeps that diff clean — Clock
re-exports NOTHING, it keeps its own definition until it dies):

```haskell
  | InsertFor Int String
    -- ^ assert a sentence that EXPIRES: the engine retracts it (whole
    -- subtree, so lifetimes belong on leaf facts) @n@ round boundaries
    -- after this insert. Re-inserting the exact path with a lifetime
    -- refreshes the due; re-inserting it bare cancels it (spec v44's
    -- supersession law).
```

```haskell
-- | One recurring engine-schedule rule (spec v44): every 'srPeriod' round
-- boundaries, ground each clause's conditions and apply its outcomes per
-- binding. Bodies may read the clock ('Prax.Types.turnPath') as a fact.
data ScheduleRule = ScheduleRule
  { srName   :: String   -- ^ single segment; the persist re-association key
  , srPeriod :: Int      -- ^ round boundaries between firings (authored meaning)
  , srBody   :: [([Condition], [Outcome])]
  }
  deriving (Eq, Show)
```

`CookedOutcome` gains `CInsertFor Int [(Sym, Maybe Char)]`; a cooked rule mirror
`CookedScheduleRule = CookedScheduleRule { csrName :: String, csrBody ::
[([CookedCondition], [CookedOutcome])] }`. `PraxState` gains:

```haskell
  , schedule       :: [ScheduleRule]              -- authored declarations
  , cookedSchedule :: [CookedScheduleRule]        -- cooked by the setter
  , scheduleDues   :: Map String Int              -- rule name -> next due turn
  , expiries       :: Map [(Sym, Maybe Char)] Int -- exact labeled path -> due turn
```

(`Sym` needs `Ord` — derive via its Int id if absent.) `emptyState`: the four fields
empty, and the db gains `insert (turnPath ++ "!0")` — the clock exists from
construction [spec: seeding]. `turnPath :: String; turnPath = "turn"` declared here
(haddock: the engine's clock family; v44).

**1b. Cooked/Engine.** `cookOutcome`/`groundCookedOutcome` arms mirror `CInsert`.
`performCooked` gains the queue bookkeeping:

```haskell
performCooked (CInsertFor n toks) st =
  performCooked (CInsert toks) st'
  where st' = st { expiries = Map.insert toks (currentTurn st + n) (expiries st) }
```

— and the EXISTING arms gain the supersession/purge law:
- `CInsert toks`: `Map.delete toks` from `expiries` first (bare insert CANCELS).
- `CDelete toks`: eagerly purge every entry AT OR UNDER the deleted path
  (`Map.filterWithKey` on the name-prefix — a subtree delete takes its descendants'
  timers).

`currentTurn :: PraxState -> Int` reads `turnPath`'s value from the db (loud error if
absent — impossible post-1a, and silence is banned). Eviction needs NO eager purge: the
fire-time existence guard (1c) is the spec's stated mechanism.

**1c. The boundary function** (Engine; NOT called by the loop yet):

```haskell
-- | One round boundary (spec v44): advance the clock, fire due expiries
-- (existence-guarded: an entry whose exact fact was evicted since drops
-- silently — no retract, no recompute), then due schedule rules in
-- declaration order, re-arming each period boundaries from NOW. A pure
-- function of the state; the loop runs it at rotation wrap (Task 2).
roundBoundary :: PraxState -> PraxState
roundBoundary st0 = foldl' fireRule stExpired dueRules
  where
    now  = currentTurn st0 + 1
    st   = performCooked (CInsert (internTokens (turnPath ++ "!" ++ show now))) st0
    (due, keep) = Map.partition (<= now) (expiries st)
    stExpired = foldl' expireOne (st { expiries = keep }) (Map.keys due)
    expireOne s toks
      | exists (tokensToSentence toks) (db s) = performCooked (CDelete toks) s
      | otherwise                             = s          -- evicted since: silent drop
    dueRules = [ r | r <- cookedSchedule st0
                   , Map.findWithDefault maxBound (csrName r) (scheduleDues st0) <= now ]
    fireRule s r =
      (foldl' (\s' (conds, outs) -> applyForEach conds outs s') s (csrBody r))
        { scheduleDues = Map.insert (csrName r) (now + periodOf r) (scheduleDues s) }
```

(Sketch-exact; the implementer reuses `performCooked (CForEach conds outs)`'s
snapshot-application for `applyForEach`, resolves `periodOf` from the string-side
`schedule` by name — loud error on a miss — and keeps the clock-advance on the same
`performCooked` path so relevance/closure tiers apply. The existence check may use the
token form directly if a `existsToks` is cleaner than re-rendering; one home either
way.)

**1d. Analyses.** `cookedOutcomeAtoms`/`outcomeDeltaAnchors`/`condPatterns`-successors:
`CInsertFor` arms treat it EXACTLY as `CInsert` (the deferred retract is environment —
spec). `producibleAtoms` additionally folds `cookedSchedule` bodies (same walker) and
gains `[[intern turnPath]]` beside the `contradiction` witness (the engine produces the
clock). `seedlessDrawErrors` and the DeadCondition `lintSites` fold `cookedSchedule`
too (site label `"schedule <name>"`). TypeCheck's string-side `outcomeUses`/
`assertedSentences`/`refErrors` walks gain `InsertFor` arms mirroring `Insert`.

**1e. Persist v2.** Header becomes `prax-state v2` (v1 now rejected by the existing
unsupported-version arm — that machinery landed in v43 for exactly this). New line
forms: `due <name> <turn>` per `scheduleDues` entry; `expiry <turn> <labeled-sentence>`
per `expiries` entry (turn FIRST — the sentence may contain spaces never, but paths
never contain spaces; keep the fixed-field prefix anyway for parse simplicity).
`deserializeState` re-associates dues BY NAME against the world's `schedule` — an
unknown name is a loud error; expiry sentences re-tokenize via `internTokens`.

**1f. Tests (RED-first, in the new `ScheduleSpec`):** each expiry-law pin drives
`performCooked`/`roundBoundary` directly on a tiny fixture world —
1. `InsertFor` inserts the fact and it holds for n boundaries, gone at the nth.
2. Refresh: re-`InsertFor` before due → survives past the old due, dies at the new.
3. Cancel: bare re-insert → never expires.
4. Delete-purge: authored delete → no later retract fires (pin via a probe on
   `expiries` emptiness AND behavior).
5. Eviction drop: `mood!a lasts n` then bare `mood!b`; at the due, `mood!b` still
   stands, no closure recompute observable (pin exact facts).
6. Subtree-retract: lifetime on an interior path takes descendants at expiry.
7. Boundary order: an expiring fact + a period-1 rule reading it — the rule does NOT
   see it at the expiry boundary (the ghost-observation repro).
8. Rule re-arm from now; declaration-order firing; unknown due name on load errors.
PersistSpec: v2 round-trip incl. populated dues+expiries; v1-header rejection pin.
Plus: update (itemized) any existing pin that enumerates a clockless world's raw facts
(now containing `turn!0`).

**Process:** RED observed per law (the constructors exist but arms unwired where
observable, or fixtures asserting post-fix behavior against pre-fix engine — record
per-pin evidence) → GREEN → full suite; goldens byte-identical EXCEPT the itemized
`turn!0` fact-set pins → gates → commit
`"The schedule, inert: lifetime inserts, the exact-path queue, the boundary function, prax-state v2"`.

- [ ] RED per law → GREEN → suite (goldens byte-identical, turn!0 pins itemized) →
      gates → commit.

---

### Task 2: The switch — the loop wires the boundary; the tickers die; the worlds migrate

**Files:** `src/Prax/Loop.hs`, `src/Prax/Engine.hs` (setter), NEW
`src/Prax/Schedule.hs` (authoring surface), `src/Prax/Sight.hs`, DELETE
`src/Prax/Drift.hs` + `src/Prax/Clock.hs`, `src/Prax/Emotion.hs`,
`src/Prax/Relevance.hs`, `src/Prax/TypeCheck.hs`, `app/Main.hs`,
`src/Prax/Worlds/Village.hs`, `src/Prax/Worlds/Bar.hs`, `prax.cabal`. Tests: see
inventory below.

**2a. The authoring surface (`Prax.Schedule`, new):**

```haskell
-- | The v44 authoring surface for engine time: declare recurring rules and
-- lifetimes; the engine owns every firing (spec 2026-07-16-v44).
module Prax.Schedule
  ( lasts        -- the ScheduleRule TYPE stays in Prax.Types (one home, no re-export);
  , gathering    -- this module is the combinator surface only
  , sightRule
  ) where

-- | Wrap plain inserts with a lifetime. Loud on anything but Insert —
-- a lifetime on a Delete/Call/ForEach has no meaning.
lasts :: Int -> Outcome -> Outcome
lasts n (Insert s) = InsertFor n s
lasts _ o = error ("Prax.Schedule.lasts: only an Insert can carry a lifetime, got: "
                   ++ show o)

-- | A recurring convening: ONE rule; the open effects fire every period and
-- their asserted facts live @duration@ boundaries (the close rule of the
-- v37 design is subsumed by expiry — one mechanism for a temporary fact).
gathering :: String -> Int -> Int -> [Outcome] -> ScheduleRule
gathering name period duration openOuts
  | duration < 1 || duration >= period =
      error ("Prax.Schedule: gathering " ++ name ++ " needs 0 < duration < period")
  | otherwise = ScheduleRule name period [ ([], map (lasts duration) openOuts) ]

-- | Perception as a period-1 rule: the authored sighting template, stamped
-- with the clock read as a fact (the tick machinery is the engine's now).
sightRule :: [Condition] -> ScheduleRule
sightRule sighting = ScheduleRule "sight" 1
  [ ( sighting ++ [ Neq "Seer" "Seen", Match (turnPath ++ "!Now") ]
    , [ Insert "Seer.believes.at.Seen!Spot"
      , Insert "Seer.believes.atSince.Seen!Now" ] ) ]
```

Guards: the v40/v43 splice checks (`authoredVarClash ["Actor"] …` on every clause of
every declared rule + the sighting template — there is no actor at all) move into the
schedule setter (2b) or rule construction; single-segment rule names, positive period,
duplicate rule names — the old `driftP`/`guardRule` guards survive re-homed, verbatim
semantics. NOTE `sightRule` binds `Now`/`Seer`/`Seen`/`Spot` as its contract exactly as
`sightP` did.

**2b. The setter (Engine):** `setSchedule :: [ScheduleRule] -> PraxState -> PraxState`
— guards (2a), stores `schedule`, cooks `cookedSchedule`, seeds `scheduleDues` (each
rule at `currentTurn + period`: start-sated), `retable` (analyses see the bodies).

**2c. The loop:** `advance` becomes wrap-aware:

```haskell
advance :: PraxState -> (Character, PraxState)
advance st0 =
  case nextLiving st0 of
    Nothing -> error "Prax.Loop.advance: no living characters"
    Just i
      | i <= cursor st0 ->                        -- wrap (equality: single survivor)
          let st1 = roundBoundary st0
          in case nextLiving st1 of               -- re-select: a rule may kill
               Just j  -> (characters st1 !! j, st1 { cursor = j })
               Nothing -> error "Prax.Loop.advance: no living characters"
      | otherwise -> (characters st0 !! i, st0 { cursor = i })
  where
    nextLiving st = listToMaybe
      [ i | k <- [1 .. n], let i = (cursor st + k) `mod` n, alive st i ]
      where n = length (characters st)
    alive st i = not (exists (deadSentence (charName (characters st !! i))) (db st))
```

(`cursor = -1` start: first advance selects 0, no wrap — no boundary before round 1.
Wrap runs the boundary ONCE then re-selects from index 0 in the new state; the
re-selection scans from `cursor st1` — carry the wrap's reset correctly: re-select with
cursor reset to `-1`-equivalent, i.e. scan from index 0. The implementer states the
exact re-scan and pins it.) `narrate`'s blank-label guard DELETED (no silent action
remains; Main's mirror too).

**2d. Deaths and re-homes:** `Prax.Drift` DELETED (`DriftRule`→`ScheduleRule` in Types;
gathering re-homed per 2a; the haddock's authoring-periods guidance moves to
`ScheduleRule`). `Prax.Clock` DELETED (`turnPath` already in Types from Task 1).
`Sight` keeps `sightedWithin` + gains nothing else (`sightRule` lives in Schedule;
sightP/sightChar/sightSetup/sightName die). `Emotion`: `feelingsFade` DELETED;
`feelFor n who emotion` / `feelTowardFor n who emotion target` = `InsertFor n
(feelsPath …)` (via `lasts`, the ONE mechanism); wear-off haddock rewritten.
`Relevance`: `worldAtomPools`' `Map.delete driftPracticeId` and the Drift import die;
`bearingTemplates`' stale no-drifter-exclusion comment updated. `TypeCheck`:
`ClocklessDrift` constructor/check/describe DELETED; new `EngineFamilyWrite`-style
TypeError — any authored outcome (`Insert`/`InsertFor`/axiom head) writing the
`turnPath` family is flagged (fixtures jump clocks via `performOutcome`, which this
never sees). `app/Main.hs`: describe arms updated; blank-label mirror deleted.

**2e. Worlds:**
- Village: roster loses `sightChar`/`driftChar`; `definePractices` loses
  `sightP`/`driftP`; `setSchedule [ sightRule villageSighting, hungerPulse',
  gathering "market" 6 1 [market-open-outs] ]` (hungerPulse rebuilt as ScheduleRule,
  same body/period; the market's open outcomes are the current openOuts, the close
  rule's explicit deletes DIE — expiry does it); setup loses
  `sightSetup`/`driftSetup`/gathering seeds; the two draw-nested `feelToward "T" angry
  "Actor"` become `feelTowardFor 4` (test-compressed, labeled — real authoring
  ~24-48); `villageFade` GONE.
- Bar: same shape; `metabolism` as ScheduleRule; every `feelToward` onset (~20,
  enumerate at migration) becomes `feelTowardFor 4` with the same label; `barFade`
  GONE.

**2f. Test inventory (each re-expressed, coverage preserved — from the completeness
review, binding):** DriftSpec→`ScheduleRuleSpec` (pulse timing/re-arm/multi-period
via `roundBoundary` and clock-jumps); ClockSpec dies (Task 1's ScheduleSpec owns the
clock); SightSpec against `sightRule`; EmotionSpec's fade group schedule-driven;
RelevanceSpec's three liveness-gate pins with the SCHEDULE as fact-mover (the
"clock-moved facts gate" unit coverage survives); VillageSpec/BarSpec `tick`/`pulse`
helpers → boundary-driven (incl. the onset/fade v35-wake pins and "fade catches the
unvented"); LoopSpec round arithmetic re-captured + NEW: wrap-under-death,
single-survivor wrap (equality), rule-kills-character-mid-wrap, no-boundary-before-
round-1; TypeCheckSpec: ClocklessDrift cases and the v42 composition pin OUT,
turn-write guard pins IN (authored write flagged; performOutcome jump not);
AnalysisTableSpec + GoldenDriveSpec re-captured (itemized per world: roster lines
gone, classifications where drift left `cookedDefs`); EngineSpec `caresAbout` rows.

**Process:** grep-verify no shipped drift/sighting fragment uses `Actor` (v43
established; re-verify) → RED where new (boundary wiring pins, guard pins) → the
switch → GREEN → full suite → golden/pin re-captures ITEMIZED (each world's diff
explained: removed ticker lines, shifted step arithmetic, fade-semantics changes and
nothing else — an unexplained decision change is a BLOCK) → decision-content
equivalence argued in the commit body incl. sighting/pulse TIMING relative to mover
turns → gates (zero warnings, hlint, `prax check` ×7) → commit
`"The switch: the engine runs time; the tickers are gone"`.

- [ ] Re-verify Actor grep → RED → switch → GREEN → re-captures itemized + equivalence
      argument → gates → commit.

---

### Task 3: Script's scene clock dies

**Files:** `src/Prax/Script.hs`, `src/Prax/Worlds/Audience.hs` (only if its source
names sceneClock/timeout internals — check), `test/Prax/ScriptSpec.hs`.

- `_clock`/`clockName`/`usesClock`/`sceneClock` machinery OUT. Scene entry stamps the
  live clock — at the initial scene's setup AND every junction transition:
  `ForEach [ Match (turnPath ++ "!Now") ] [ Insert "sceneEntered!Now" ]` (a plain
  stamp cannot capture a live value). Timed junction conditions become
  `[ Match "sceneEntered!E", Match (turnPath ++ "!Now"), Calc "D" Sub "Now" "E",
  Cmp Gte "D" (show n) ]`. `clockReached`/`after`/`timeout` re-express on this shape;
  scripts without timed junctions get no stamp (unchanged beyond the universal clock).
- Consumers: Audience's `timeout "dismissed" 5`; ScriptSpec's two timed-junction tests
  re-pinned on the new shape (same fiction: dawdling to "dismissed"); Play unaffected
  (untimed gotos). Goldens for play/audience re-captured, itemized (their rosters lose
  `_clock`).
- [ ] RED (timed-junction pins against the new shape) → re-express → GREEN → suite →
      re-captures itemized → gates → commit
      `"Script: scene time is a stamp against the engine clock, not a character"`.

---

### Task 4: Docs

**Files:** `docs/LEDGER.md`.

- [ ] The v44 legend row (house style): the paradigm correction and its origin (the
  v38 fade review → the user's engine-owns-scheduling directive, full blast radius);
  the three-lens isolated spec review as a NEW pre-gate practice (cite the review
  files; name what it changed: the gathering collapse, the lifetime-insert
  representation, the boundary predicate, the clock seed, the fade-migration
  commitment); the deaths (four ticker characters, Drift, Clock, due.*, feelingsFade,
  ClocklessDrift, blank-label suppression); the laws (supersession, purge/existence
  guard, expiries-before-rules with the ghost-observation reason, wrap-with-equality);
  the deliberate fiction change (per-onset lifetimes replace synchronized wipes — the
  v36/v38 episodic principle actually implemented); Persist v2; suite count as
  measured. Note superseded: the per-feeling-fade-stamps bank item (closed), v43's
  standalone clock (lived one day; the extraction made this round's diff smaller —
  say so honestly). Backlog pointers stay with the bank.
- [ ] Gates; commit `"Docs: v44 — time belongs to the engine"`.
