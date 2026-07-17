# v46 — The Narrator Dies Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

## The problem, and why each change exists

**The hack being removed.** The scene layer (v12) invented `_narrator` — a hidden
bodiless cast member bribed by fabricated `storyAdvanced.<key>` facts — so that scene
transitions, endings, and one-shot narration would "happen" as someone's actions. A
fake person takes a real planner-driven turn each round to execute scheduling.

**The principle that sized the fix** (user-directed): fiction surfaces through
characters; the world's own dynamics fire silently. Omniscient memories are
presentation wearing world-content clothes — REMOVED as a feature, not re-homed.
Junction `"(story)"` labels are log markers — gone. With no words to carry, no
narration channel, clause modes, labels, boundary-signature or save-point changes
exist in this round at all. What remains:

1. Junctions/endings → ONE plain `AllClauses` period-1 schedule rule (`"story"`),
   clauses in authored order; the law is the EXISTING gates (`currentScene` eviction
   self-masks, `Absent ending` masks, declaration order resolves ties — replacing an
   accidental alphabetical-label tiebreak). Cross-scene cascade is permitted, stated
   eager semantics.
2. One internal registration function in Engine (`Script.compile`'s rules carry Prax
   machinery that `setSchedule` rightly rejects; compiler-level code enters by the
   engine door per v45's threat model). Sight is untouched — its rule is Prax-var-free
   and stays on `setSchedule`.
3. `prax-state v3` (v45-era script saves are format-identical but semantically dead).

Net ledger: deletions (narrator, storyAdvanced, junctionsP, the memory feature
end-to-end, story labels) decisively exceed additions (one internal door function, one
header bump). A true simplification round.

**Goal:** per `docs/specs/2026-07-16-v46-the-narrator-dies.md` (twice-rewritten; the
final form is the small one).

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- Spec laws binding: authored-order clauses; existing-gate self-masking; cascade =
  stated eager semantics; duplicate-rule-name guard across both entry points; v3.
- Re-captures itemized per licensed class ONLY: removed `_narrator` roster/pin rows,
  removed memory/story-marker trace lines, boundary-timing shifts, tiebreak-law
  changes. Anything else = BLOCK.
- StressSpec's play-world coverage RE-ARGUED, not re-pinned — a coverage loss is a
  finding for adjudication.
- RED-first; suite green per commit; zero warnings; hlint; `prax check` ×7.
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: The switch

**Files:** `src/Prax/Script.hs`, `src/Prax/Script/Json.hs`, `src/Prax/Engine.hs`
(the internal door + duplicate-name guard span), `src/Prax/Persist.hs` (v3),
`src/Prax/Worlds/Play.hs` + `Audience.hs` (memory content out), tests:
`ScriptSpec`, `Script/JsonSpec`, `DirectorSpec`, `StressSpec`, `PersistSpec`,
`AnalysisTableSpec` + goldens (re-captures), `ScheduleRuleSpec` (cascade/order pins).

**1a. Engine:** `registerEngineRules :: [ScheduleRule] -> PraxState -> PraxState` —
unexported from any authoring surface, haddock stating the v45 threat-model charter
(compiler-level door; no authoring guards); seeds dues exactly as `setSchedule`; BOTH
entry points share the duplicate-rule-name guard (loud, naming both homes).

**1b. Script:** `compile` emits the `"story"` rule via the door — clauses = scenes in
declaration order, each scene's junctions in declaration order (memories no longer
exist); clause shape: junction = (conditions: `currentScene!sid`, `Absent ending`,
`junctionWhen`, `clockReached`-expansion if `junctionAfter`; outcomes: today's
transition — `currentScene!to` eviction, `sceneEntered` stamp, destination setup — or
`ending!name` insert; NO `storyAdvanced` insert, NO label). DELETE: `_narrator`,
`narratorName`, the narrator's Want, `junctionsP`, the `storyAdvanced` inserts, the
`Memory` AST + `memory` constructor + `compileMemory` + `memoryFired` latch machinery,
JSON's memory field + its codec arms, the `"(story) "` label prefix. `flowChart`/reach
analysis reads Script-level structure — verify, adjust only if it touched the compiled
practice or memories.

**1c. Worlds:** Play and Audience lose their one `memory` line each (content removal,
itemized); everything else in them is untouched.

**1d. Persist:** header → `prax-state v3`; the version-rejection pins updated
(v3 rejects v2 and v1).

**1e. Pins (RED-first):**
- Story law: two co-enabled same-scene junctions → first in authored order fires,
  second masked (the eviction); post-ending masking; the cascade pin (a pass-through
  scene traverses in one boundary — DOCUMENTED semantics); authored-order-vs-label
  order (a pin whose junction names would sort differently alphabetically).
- Timed junction fiction (Audience dismissal at the same fictional point).
- Duplicate-name guard both directions (authored `story` in a script world; script
  compiled into a world with an authored `story` rule).
- Deaths grep-proof: `_narrator|narratorName|storyAdvanced|junctionsP|memoryFired|Memory`
  live nowhere in src/.
- StressSpec re-run and RE-ARGUED per the Global Constraint.
- Re-captures itemized; time-free non-Script worlds byte-identical.

- [ ] RED → switch → GREEN → Stress re-argued → re-captures itemized → gates →
      commit `"The narrator dies, and takes the narration with it"`.

---

### Task 2: Docs

**Files:** `docs/LEDGER.md`, plus README/WALKTHROUGH sweeps for memory/narrator
mentions (the v44 lesson: live docs must not teach dead machinery).

- [ ] The v46 legend row (house style): the hack named; the TWO rewrites and what
  forced each (the panel's unsoundness findings against draft one; the user's
  constraint-removal that collapsed draft two's premium — "the most principled option,
  including not supporting any of this"); memories REMOVED-BY-DESIGN with the
  principle stated (fiction surfaces through characters; presentation is not world
  content); the tiebreak accident fixed into authored order; cascade as stated
  semantics; the deletions ledger; Persist v3; Stress outcome; suite counts. Queue
  pointer to v47.
- [ ] README/WALKTHROUGH: memory-feature and narrator mentions updated (removal noted
  where the feature was advertised).
- [ ] Gates; commit `"Docs: v46 — the last ticker, and the words that went with it"`.
