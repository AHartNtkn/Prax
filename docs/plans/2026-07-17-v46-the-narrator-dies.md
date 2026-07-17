# v46 — The Narrator Dies Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** per `docs/specs/2026-07-16-v46-the-narrator-dies.md` (panel-reviewed, amended
f25d944) — the schedule gains FirstMatch clauses and a narration channel; the schedule
gets two doors; Script's junctions/memories become one engine-door story rule; the
narrator, `junctionsP`, and `storyAdvanced` die; Persist bumps to v3.

**Task shape:** T1 lands the engine surface with every existing rule migrated
blank-label/AllClauses — goldens BYTE-IDENTICAL (nothing labeled exists yet; the
channel is exercised by new pins only). T2 is the Script switch (re-captures there).
T3 docs.

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- Spec laws are binding verbatim: FirstMatch = FnCase semantics; memories-before-
  junctions within a scene; one story event per boundary, no cascade; authored-order
  tiebreak (a DELIBERATE law change from alphabetical-label — itemize every pin that
  shifts because of it); engine-door rules fire before authored-door rules, each in
  registration order (reproduces shipped order: `sightRule` leads every current list);
  cross-door duplicate-rule-name guard; blank clause labels are silent; `prax-state v3`.
- T1 exactness: goldens/AnalysisTable byte-identical (all migrated rules unlabeled).
  T2 re-captures itemized per licensed class (removed `_narrator` rows/lines,
  boundary-timing shifts, tiebreak-law changes) — anything else is a BLOCK.
- StressSpec's play-world coverage is RE-ARGUED, not re-pinned: if scenes/endings
  coverage or zero-dead-ends fails under the new dynamics, STOP and report — that is a
  finding for adjudication, not a number to update.
- RED-first throughout; suite green per commit; zero warnings; hlint; `prax check` ×7.
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: The engine surface — modes, labels, doors, the speaking boundary

**Files:** `src/Prax/Types.hs`, `Cooked.hs`, `Engine.hs`, `Loop.hs`, `Persist.hs`,
`Sight.hs`, `Schedule.hs`, `Stress.hs`, `app/Main.hs`, worlds (call-shape migration),
tests: `ScheduleRuleSpec`/`ScheduleSpec`, `LoopSpec`, `PersistSpec`, `SightSpec`,
`StressSpec` (thread-discard only), `TypeCheckSpec` (door guards).

**1a. Types.** `ScheduleRule` becomes:

```haskell
data ClauseMode = AllClauses | FirstMatch   -- FirstMatch = FnCase semantics
data ScheduleRule = ScheduleRule
  { srName   :: String
  , srPeriod :: Int
  , srMode   :: ClauseMode
  , srBody   :: [(String, [Condition], [Outcome])]   -- (label, when, outs); "" = silent
  }
```

Cooked mirror gains the mode and per-clause label. Every existing rule literal
(worlds' pulses, `gathering`, `sightRule`, fixtures) migrates mechanically:
`AllClauses`, label `""`. `gathering`/`sightRule` combinators emit the new shape.
`PraxState` gains `engineSchedule`/`cookedEngineSchedule` beside the authored fields
(one TYPE, two doors; the boundary concatenates engine-then-authored).

**1b. Doors.** `setSchedule` unchanged in charter (authored; fully guarded — its
existing guards now also reject any rule whose name collides ACROSS doors, checked
against both maps). NEW internal door `Prax.Engine.registerEngineRules ::
[ScheduleRule] -> PraxState -> PraxState` — no authoring guards (compiler-level, per
the v45 threat model; haddock says exactly that), seeds dues identically, same
duplicate-name guard across doors. `Prax.Sight` gains the world-facing setter that
routes through it:

```haskell
-- | Install perception: guards the AUTHORED template (Seer/Seen/Spot contract,
-- Prax namespace forbidden), then registers the machinery-bearing rule via the
-- engine door.
withSighting :: [Condition] -> PraxState -> PraxState
```

Worlds change shape: `setSchedule [hungerPulse, marketGathering]` + `withSighting
villageSighting` (order between doors is the LAW, not the call order). `sightRule`
becomes internal to Sight (or stays in Schedule consumed by Sight — one home,
implementer picks the smaller diff and says so).

**1c. The executor speaks.** The boundary's rule-firing loop grounds clauses ITSELF
(one implementation, both modes — no CForEach delegation): per due rule, per clause in
order — query the clause conditions against the view snapshot; `AllClauses`: apply
outcomes per binding, rendering the label per binding (blank = no line); `FirstMatch`:
first clause with ≥1 binding fires its FIRST binding only (deterministic: the query's
binding order), renders, and the rule is done this boundary. `roundBoundary ::
PraxState -> (PraxState, [String])`; `advance :: PraxState -> (Character, PraxState,
[String])`; `runNpcTicks` threads boundary lines into its trace in firing position;
`Stress` discards explicitly with a comment; `Main` prints them and MOVES THE SAVE
POINT post-boundary (resume lands on the player's turn without re-running/re-printing
the boundary — implementer verifies the resume path against Main's loop and pins it in
PersistSpec-or-LoopSpec).

**1d. Persist v3.** Header bump; the v2-rejection pin flips to v3-rejects-v2 (+ keeps
rejecting v1); dues/expiries lines unchanged; engine-door rules' dues serialize in the
same table (names are globally unique by the guard).

**1e. Label rendering** reuses the `[Var]` template rendering actions already use
(`renderText`-equivalent — find the one home, reuse it; no second renderer).

**Pins (RED-first):** FirstMatch semantics (first-with-binding fires, rest skipped,
next boundary re-evaluates, single binding); AllClauses labeled rule renders per
binding; blank silent; engine-before-authored order; cross-door name collision loud;
boundary narration appears in `runNpcTicks` traces in position; save-point/resume
no-re-narration; Persist v3 pins; `withSighting` template guard (Prax var in a
template errors; Seer/Seen/Spot contract free). Existing goldens/AnalysisTable pins
BYTE-IDENTICAL (everything migrated is blank/AllClauses; sighting registration order
preserved by the door law).

- [ ] RED per pin → implement → GREEN → suite byte-identical outside new pins → gates
      → commit `"The schedule speaks: clause modes, labels, and two doors"`.

---

### Task 2: The Script switch — the story rule; the narrator dies

**Files:** `src/Prax/Script.hs`, `app/Main.hs` (if narrator-specific rendering
exists), worlds none (Play/Audience compile via Script), tests: `ScriptSpec`,
`Script/JsonSpec` (labels unchanged in schema), `DirectorSpec`, `StressSpec`,
`AnalysisTableSpec` + goldens (re-captures), `GateSpec` (nothing to change — verify).

- `compile`: junctions + memories become ONE `FirstMatch` period-1 rule named
  `"story"`, registered via the engine door. Clause order: scenes in declaration
  order; within a scene, memories (authored order) THEN junctions (authored order).
  Clause shapes: memory = (memoryText, currentScene+latch+memoryWhen conds,
  [memoryFired insert]); junction = ("(story) " ++ junctionName — today's label,
  currentScene + no-ending + junctionWhen + clockReached-if-timed,
  transition/ending outcomes incl. the sceneEntered stamp). The `storyAdvanced`
  inserts and the narrator's Want DIE with `_narrator`/`narratorName`/`junctionsP`.
- `flowChart`/reach analysis: reads Script-level structure (pre-compile) — verify and
  adjust only if it touched compiled practices.
- ScriptSpec re-expression: junction/memory/timed-junction pins keep their FICTION
  (memories fire once with their text in the trace; dawdling still dismisses); the
  tiebreak-law and boundary-timing shifts itemized. StressSpec per the Global
  Constraint (re-argue; a coverage loss is a BLOCK).
- Re-captures itemized: AnalysisTable audience/play `_narrator` rows out; drive/trace
  goldens' narrator lines become boundary narration lines; nothing else.
- Deaths grep-proof: `_narrator|narratorName|storyAdvanced|junctionsP` return nothing
  live in src/.

- [ ] RED (story-rule pins vs pre-switch tree) → switch → GREEN → re-captures
      itemized + Stress re-argued → gates → commit
      `"The narrator dies: the story is a first-match rule, and the boundary tells it"`.

---

### Task 3: Docs

**Files:** `docs/LEDGER.md`.

- [ ] The v46 legend row (house style): the audit finding closed (the last ticker);
  the panel's three forced design moves (first-match clauses; two doors; per-clause
  narration — cite the review files and that the first draft was unsound three ways);
  the tiebreak law change stated as deliberate; the deferrals resolved
  (storyAdvanced dead; atSince reservation dropped as unbuildable, residue annotated);
  Persist v3; StressSpec's re-argued coverage outcome; suite counts as measured.
  Queue pointer to v47 (function registry).
- [ ] Gates; commit `"Docs: v46 — the last ticker"`.
