# v43 — Hygiene Bundle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** per `docs/specs/2026-07-15-v43-hygiene-bundle.md` — six guards/extractions,
zero shipped-world behavior change. Task 1 lands the five guards (fn names, action
names, Persist header, trailing operators, splice points); Task 2 lands the Clock
extraction in isolation (pure code motion, byte-identical goldens); Task 3 docs.

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- Goldens byte-identical THROUGHOUT (every item is a guard on illegal input or code
  motion; legal worlds cannot observe any of it). Any golden movement = BLOCK + trace.
- Every guard RED-first: the triggering fixture observed erroring with the pinned
  message, the legal twin observed quiet. Loud errors name the offender and the fix.
- If a shipped world or committed test trips a new guard, that is an ADJUDICATION, not
  an accommodation: report it (an `Actor`-using drift body would be a live bug find; a
  trailing-operator test sentence is either a caller bug to fix or the trivia itself —
  flip such a test to pin the error). Never weaken a guard to make existing code pass
  silently.
- Zero warnings; hlint; `prax check` ×7; suite green per commit.
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: The five guards

**Files:**
- Modify: `src/Prax/Engine.hs` (definePractice guards), `src/Prax/Db.hs` (tokens),
  `src/Prax/Persist.hs` (header), `src/Prax/Drift.hs` + `src/Prax/Sight.hs`
  (forbid Actor), `src/Prax/Rumor.hs` + `src/Prax/Deceit.hs` (namespace guards).
- Tests: `EngineSpec` (collision guards), `DbSpec` (trailing op), `PersistSpec`
  (header + version rejection), `DriftSpec`/`SightSpec`-or-sibling (Actor forbid),
  `RumorSpec`/`DeceitSpec` (namespace guards) — each in its file's idiom
  (try/evaluate/ErrorCall for construction-time errors, per house pattern).

**1a. `Prax.Engine.definePractice` — both collision guards (complete):**

```haskell
definePractice :: Practice -> PraxState -> PraxState
definePractice p st
  | a : _ <- dupActions =
      error ("Prax.Engine.definePractice: practice " ++ show (practiceId p)
             ++ " declares two actions named " ++ show a
             ++ " -- action names are lookup keys (delta anchors, standing"
             ++ " intentions); rename one")
  | (fn, home) : _ <- fnCollisions =
      error ("Prax.Engine.definePractice: function " ++ show fn ++ " in practice "
             ++ show (practiceId p) ++ " is already declared by practice "
             ++ show home
             ++ " -- Call resolution is by bare name (lookupCookedFn); rename one")
  | otherwise =
      retable (withDb (insertAll (map (prefix ++) (dataFacts p))) st)
        { practiceDefs = Map.insert (practiceId p) p (practiceDefs st) }
  where
    prefix = "practiceData." ++ practiceId p ++ "."
    names = map actionName (actions p)
    dupActions = [ n | (n, i) <- zip names [0 :: Int ..], n `elem` take i names ]
    ownFns = map fnName (functions p)
    fnCollisions =
      [ (fn, practiceId p)
      | (fn, i) <- zip ownFns [0 :: Int ..], fn `elem` take i ownFns ]
      ++ [ (fnName f, practiceId q)
         | q <- Map.elems (practiceDefs st), practiceId q /= practiceId p
         , f <- functions p, fnName f `elem` map fnName (functions q) ]
```

(Excluding `practiceId q == practiceId p` lets a practice be legally re-defined over
its own old version. The within-practice duplicate arms close the same holes
`cpFns`'s first-wins fold and `groundedDeltaAnchors`' first-match papered over.)

**1b. `Prax.Db.tokens` — trailing-operator rejection (complete):**

```haskell
tokens :: String -> [(String, Maybe Char)]
tokens s0 = go (trim s0)
  where
    go [] = []
    go s =
      let (name, rest) = span (\c -> c /= '.' && c /= '!') s
      in case rest of
           []   -> [(name, Nothing)]
           [op] -> error ("Prax.Db.tokens: trailing operator " ++ show op
                          ++ " in " ++ show s0
                          ++ " -- a sentence ends in a name, not an operator")
           (op : more) -> (name, Just op) : go more
```

Haddock gains one sentence: a trailing operator is rejected loudly — it would set a
leaf's exclusion flag that nothing ever reads (write-only state breaking `Db` equality
and serialize round-trip identity; the v43 spec's probe). `insertToks` untouched.
DbSpec pins: `insert "at.bob!"` errors (try/evaluate — the RED flip of the probe);
`insert "at.bob"` and interior-`!` forms (`"turn!0"`) unaffected. If any committed test
currently constructs a trailing-op sentence, adjudicate per Global Constraints.

**1c. `Prax.Persist` — version header (complete):**

```haskell
-- | The save-format tag, first line of every serialized state. Bump it when
-- the line format below changes; 'deserializeState' rejects anything else
-- loudly — no silent misparse of a save from another era.
formatVersion :: String
formatVersion = "prax-state v1"
```

`serializeState` puts `formatVersion` first: `unlines (formatVersion : ("cursor " ++ …) : …)`.
`deserializeState`:

```haskell
deserializeState :: String -> PraxState -> PraxState
deserializeState text world =
  case lines text of
    (v : hd : rest)
      | v == formatVersion, ["cursor", n] <- words hd, Just c <- readMaybe n ->
          … (body exactly as today, over rest) …
    (v : _)
      | v /= formatVersion ->
          error ("Prax.Persist.deserializeState: unsupported save format "
                 ++ show v ++ " (expected " ++ show formatVersion ++ ")")
    _ -> error "Prax.Persist.deserializeState: malformed save (expected the format header, then 'cursor <n>')"
```

Export `formatVersion`. PersistSpec: round-trip pins updated for the header IN THIS
COMMIT (itemized in the message body as a deliberate format change — the only
serialization-format commit of the round); new pins: headerless input errors with the
malformed message; a `"prax-state v0"` first line errors with the unsupported message.

**1d. `Prax.Drift` / `Prax.Sight` — forbid `Actor` in authored fragments:**

- Drift's `guardRule`: `authoredVarClash [] cs os` → `authoredVarClash ["Actor"] cs os`;
  its error text gains the reason ("drift bodies run as the ticker — @Actor@ would
  bind the ticker character, never a mover").
- `sightP`: `authoredVarClash [] sighting []` → `authoredVarClash ["Actor"] sighting []`;
  same reason in its existing error text. `Seer`/`Seen`/`Spot` stay free (contract).
- FIRST verify no shipped fragment uses `Actor` (grep the worlds' drift bodies and
  sighting templates); a hit = BLOCK + surface. Pins: an `Actor`-using drift body and
  an `Actor`-using sighting template each error naming Actor; the shipped-shaped legal
  twins stay quiet.

**1e. `Prax.Rumor.gossip` / `Prax.Deceit.lie` — the missing namespace guards:**

Both gain a guard clause in the v40 house shape (check `Prax.Types.authoredVarClash`/
`authoredPatClash` for exact signatures; forbiddenSplices `[]` — the Prax namespace is
the ban; `Hearer`/`Actor`/the pattern's variables are contract):

```haskell
gossip copresence gate pat label
  | (v : _) <- offenders =
      error ("Prax.Rumor.gossip: " ++ show v ++ " in an authored gate or event"
             ++ " pattern -- the Prax namespace is reserved for machinery")
  | otherwise = … (body exactly as today) …
  where
    offenders = authoredVarClash [] gate [] ++ authoredPatClash [] [pat]
    … (existing where-bindings) …
```

`lie` identically, over `gate ++ fabrication` and `[pat]`. Pins per combinator: a
`PraxD`-using gate errors; the legal shapes (RumorSpec/DeceitSpec's existing fixtures)
stay quiet — the existing specs passing IS that pin, plus one explicit legal-twin case
if the file lacks a construction-time case to anchor it.

**Process:** RED first for every guard (fixtures observed erroring / expectations
failing before each guard lands, in whatever per-guard order is convenient — record
per-guard RED evidence in the report) → GREEN → full suite + goldens → gates → commit
`"Hygiene: five loud guards — names, formats, boundaries"`.

- [ ] Shipped-fragment Actor grep clean (or BLOCK) → per-guard RED → GREEN → suite +
      goldens byte-identical → gates → commit.

---

### Task 2: Clock extraction from Sight

**Files:**
- Create: `src/Prax/Clock.hs` (+ `prax.cabal` exposed-modules).
- Modify: `src/Prax/Sight.hs`, `src/Prax/Drift.hs`, `src/Prax/TypeCheck.hs`,
  `app/Main.hs` (ClocklessDrift message).
- Tests: new `ClockSpec` (standalone ticker) or cases in DriftSpec; existing specs
  unchanged.

**`Prax.Clock` (complete):**

```haskell
-- | The world's turn counter, in one home. "Prax.Sight"'s ticker composes
-- these fragments (perception rides the clock); a world that wants time
-- without perception registers the standalone 'clockP'/'clockChar' instead.
-- "Prax.Drift" reads the counter through 'turnPath'; "Prax.TypeCheck"'s
-- ClocklessDrift check tests for it by the same name.
module Prax.Clock
  ( turnPath
  , tickConditions
  , tickOutcome
  , clockSeed
  , clockName
  , clockP
  , clockChar
  , clockSetup
  ) where

import           Prax.Query (Condition (..))
import           Prax.Types

-- | The counter's path family. The one spelling of @turn@ in the tree.
turnPath :: String
turnPath = "turn"

-- | Read-and-advance fragments: bind the current turn, compute the next.
-- @PraxN@/@PraxM@ are machinery (v40 namespace).
tickConditions :: [Condition]
tickConditions = [ Match (turnPath ++ "!PraxN"), Calc "PraxM" Add "PraxN" "1" ]

tickOutcome :: Outcome
tickOutcome = Insert (turnPath ++ "!PraxM")

-- | The seed fact (turn 0). Part of 'clockSetup' and "Prax.Sight"'s setup.
clockSeed :: Outcome
clockSeed = Insert (turnPath ++ "!0")

-- | The standalone ticker (for drift-without-perception worlds): a bodiless
-- character whose blank-label action only advances the counter. Distinct
-- from "Prax.Script"'s scene-local @_clock@ (a different concept with its
-- own name and family).
clockName :: String
clockName = "_time"

clockP :: Practice
clockP = practice
  { practiceId = "time"
  , practiceName = "time passes"
  , actions = [ action "" (Eq "Actor" clockName : tickConditions) [ tickOutcome ] ]
  }

clockChar :: Character
clockChar = (character clockName) { charBoundTo = Just "time" }

clockSetup :: [Outcome]
clockSetup = [ Insert "practice.time.here", clockSeed ]
```

(Adapt `practice`/`action`/`character` record shapes to the tree's actual smart
constructors — mirror `sightP`/`sightChar`/`sightSetup` exactly; `roles` if sight's
shape requires one.)

**Consumers:**
- `Sight`: `sightP`'s action becomes
  `action "" (Eq "Actor" sightName : tickConditions) (tickOutcome : [ForEach …])` and
  `sightSetup = [ Insert "practice.sight.here", clockSeed ]` — BYTE-IDENTICAL
  conditions/outcomes/facts to today (same strings, same order); the goldens prove it.
- `Drift`: `Match "turn!PraxNow"` → `Match (turnPath ++ "!PraxNow")`; import Clock.
- `TypeCheck`: `exists "turn" (db st)` → `exists turnPath (db st)`; ClocklessDrift
  haddock names both providers.
- `app/Main.hs` ClocklessDrift describe: "add a clock: the sight ticker (sightSetup)
  or the standalone Prax.Clock ticker (clockSetup) before driftSetup".
- Sweep: `grep -rn '"turn' src/` afterward — the only remaining literal spelling is
  `turnPath`'s own definition (tests may keep authored literals; they pin the public
  name).

**Tests:** a standalone-clock fixture (clockP + clockChar + clockSetup + driftP, no
sight) advances `turn` over rounds and satisfies TypeCheck (no ClocklessDrift, no
DeadCondition — the lint's pool sees clockP's tick insert); RED-first for the NEW
capability (fixture written against the pre-Clock tree fails to compile / the
sightless-drift world flags ClocklessDrift... the honest RED here is the fixture
asserting `typeCheck == []` against a sightless drift world with no clock module —
observed failing — then Clock lands and it passes).

**Process:** RED (the sightless fixture) → Clock + recomposition → GREEN → full suite,
GOLDENS BYTE-IDENTICAL (the extraction's entire exactness claim) → gates → commit
`"Clock: the turn counter gets one home; Sight composes it, drift can ride it alone"`.

- [ ] RED → extract + recompose → GREEN → goldens byte-identical → sweep grep → gates
      → commit.

---

### Task 3: Docs

**Files:** `docs/LEDGER.md`.

- [ ] The v43 legend row (house style): last of the four foundations passes; the six
  items each in a clause with its defect class (silent shadowing, silent wrong lookup,
  silent misparse, write-only state breaking Eq/round-trip, ticker capture, unguarded
  splice points); the v41 promise kept (the fn-collision guard it forward-referenced);
  the one deliberate format change (Persist header, pins updated same-commit); the
  standalone clock as the extraction's new capability; suite count as measured. Mark
  the QUEUE COMPLETE — all four user-directed foundations passes shipped; note the
  backlog reverts to the bank (emotion visibility, chronicler, per-feeling fades,
  intensity, new fixture when a design needs it).
- [ ] Gates; commit `"Docs: v43 — the bundle; the queue closes"`.
