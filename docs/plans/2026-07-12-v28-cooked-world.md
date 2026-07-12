# v28 — Cooked World Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compile authored conditions/outcomes to token form once per world and run the hot paths on names end to end, per `docs/specs/2026-07-12-v28-cooked-world.md` — bit-for-bit identical decisions and views.

**Architecture:** A `Prax.Cooked` module (cooked mirrors + cook/ground), a cooked-condition query evaluator in `Prax.Query` (transcribed case-for-case from the string evaluator, pinned by an equivalence property), cooked companions carried in `PraxState` and maintained by the Engine helper family (like `improvables`/`footprint`), and consumer switches in `possibleActions`/`performAction`/`evaluate`/`closure`. Strings stay the authoring surface.

**Tech Stack:** Haskell (GHC 9.10, cabal), tasty/tasty-hunit (existing deps only).

## Global Constraints

- **Exactness**: GoldenDriveSpec byte-identical and ViewInvariantSpec green after every task; full suite green (314 baseline, ~55–60s). A net failure means the change is WRONG — BLOCK with the trace.
- The cooked evaluator must be a faithful transcription: every `Condition` case mirrored, no semantic "improvements". The equivalence property (Task 1) is the pin.
- Zero warnings; hlint "No hints"; `prax check` ×7; grep-gates (extended in Task 3) empty.
- No heuristics/magic numbers.
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: `Prax.Cooked` + the cooked query evaluator + the equivalence pin

**Files:**
- Create: `src/Prax/Cooked.hs`
- Modify: `src/Prax/Query.hs` (add `queryCooked`; export it)
- Create: `test/Prax/CookedSpec.hs`
- Modify: `prax.cabal` (expose `Prax.Cooked` after `Prax.Query`; test module after `Prax.QuerySpec`), `test/Spec.hs`

**Interfaces produced:**
```haskell
data CookedCondition
  = CMatch [String] | CNot [String]
  | CEq String String | CNeq String String
  | CCmp CmpOp String String | CCalc String CalcOp String String
  | CCount String String
  | CSubquery String [String] [CookedCondition]
  | COr [[CookedCondition]] | CAbsent [CookedCondition] | CExists [CookedCondition]
  deriving (Eq, Show)

data CookedOutcome
  = CInsert [(String, Maybe Char)]
  | CDelete [(String, Maybe Char)]
  | CCall String [String]
  | CForEach [CookedCondition] [CookedOutcome]
  deriving (Eq, Show)

cookCondition :: Condition -> CookedCondition       -- Match/Not via pathNames; recurse
cookOutcome   :: Outcome -> CookedOutcome           -- Insert/Delete via tokens; recurse
groundNames   :: Bindings -> [String] -> [String]   -- substitute; NO string rebuild
groundCookedCondition :: Bindings -> CookedCondition -> CookedCondition
groundCookedOutcome   :: Bindings -> CookedOutcome -> CookedOutcome
  -- Insert/Delete via Db.groundTokens; Call args via one-name substitution;
  -- ForEach recurses both parts
sentenceOf :: [(String, Maybe Char)] -> String      -- = Db.tokensToSentence (re-export or alias NOT
                                                    -- duplicated: use tokensToSentence directly)
```
(Drop `sentenceOf` — callers use `Db.tokensToSentence`. Substitution rule everywhere: a name is
replaced iff `isVariable` and bound, via `valToString`; identical to `Db.groundTokens`'s rule —
`groundNames b = map subst` with the same `subst`, and `groundCookedOutcome` reuses
`groundTokens` itself for Insert/Delete.)

`Prax.Query.queryCooked :: Db -> [CookedCondition] -> Bindings -> [Bindings]`: transcribe
`queryWith`/`evalCond` case-for-case — `CMatch ns → unifyNames ns db` hoisted per condition,
`CNot` likewise, every other case copied with cooked recursion (`CAbsent`/`CExists`/`COr`/
`CSubquery` recurse through the cooked evaluator with the same in-subquery flag semantics).
Where `evalCond` grounds or string-manipulates operands (`Eq`/`Cmp`/`Calc`/`Count` resolve
via `Bindings` — no parsing), copy verbatim.

- [ ] **Step 1: Write the failing tests.** `test/Prax/CookedSpec.hs`:

```haskell
module Prax.CookedSpec (tests) where

import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, (@?=))

import           Prax.Db (Val (..), insertAll, emptyDb)
import           Prax.Query (Condition (..), CmpOp (..), query, queryCooked)
import           Prax.Cooked

-- A fixture exercising every pattern-bearing construct: exclusion paths,
-- variables, negation, nested quantifiers, disjunction, subquery+count+cmp.
db :: _
db = insertAll
  [ "at.bob!square", "at.eve!mill", "at.gale!mill"
  , "holding.bob.loaf", "regards.dana.carol.thief", "regards.gale.carol.thief"
  ] emptyDb

cases :: [[Condition]]
cases =
  [ [ Match "at.Who!Where" ]
  , [ Match "at.Who!mill", Not "holding.Who.loaf" ]
  , [ Match "at.Who!Where", Absent [ Match "regards.Who.carol.thief" ] ]
  , [ Exists [ Match "holding.H.loaf" ], Match "at.Who!square" ]
  , [ Or [ [ Match "at.Who!square" ], [ Match "regards.Who.carol.thief" ] ] ]
  , [ Subquery "Rs" ["W"] [ Match "regards.W.carol.thief" ]
    , Count "N" "Rs", Cmp Gte "N" "2" ]
  , [ Match "at.Who!Where", Neq "Who" "bob", Eq "Place" "Where" ]
  ]

tests :: TestTree
tests = testGroup "Prax.Cooked"
  [ testCase "queryCooked equals the string evaluator on every fixture case" $
      [ queryCooked db (map cookCondition cs) Map.empty | cs <- cases ]
        @?= [ query db cs Map.empty | cs <- cases ]
  , testCase "grounding cooked matches grounding strings (incl. '!' outcomes)" $ do
      let b = Map.fromList [ ("Who", VStr "bob"), ("Where", VStr "square") ]
      groundNames b ["at", "Who", "Where"] @?= ["at", "bob", "square"]
      groundCookedOutcome b (cookOutcome (Insert "at.Who!Where"))
        @?= cookOutcome (Insert "at.bob!square")
  ]
```
(Fill the fixture's type hole with `Db` and adjust imports to compile; the two tests are the
equivalence pins — if the transcription drifts anywhere, case-by-case results diverge.)

- [ ] **Step 2: Observe RED** (module missing). **Step 3: Implement** per the interfaces above.
- [ ] **Step 4: GREEN**: CookedSpec + full suite once (316 expected), zero warnings, hlint.
- [ ] **Step 5: Commit** `"Prax.Cooked: the authored world compiles to names once"`.

---

### Task 2: Cooked practices in the state; Engine consumes them

**Files:** `src/Prax/Types.hs`, `src/Prax/Engine.hs`, `src/Prax/Cooked.hs` (the practice container), `test/Prax/EngineSpec.hs` (one wiring test).

**Design:**
- `Prax.Cooked` gains:
```haskell
data CookedAction = CookedAction
  { caName :: String, caConds :: [CookedCondition], caOuts :: [CookedOutcome] }
data CookedPractice = CookedPractice
  { cpInstanceNames :: [String]      -- pathNames of "practice.<pid>.<Role1>...", precomputed
  , cpActions :: [CookedAction]
  , cpInits   :: [CookedOutcome]
  , cpFns     :: Map String [([CookedCondition], [CookedOutcome])] }
cookPractice :: Practice -> CookedPractice
```
- `PraxState` gains `cookedDefs :: Map String CookedPractice` (haddock: derived from
  `practiceDefs` by the Engine helpers; emptyState `Map.empty`), maintained in `retable`
  (`cookedDefs = Map.map cookPractice (practiceDefs st)`).
- `possibleActions`: instance unification via `unifyNames (cpInstanceNames …)`; the inner
  loop becomes `queryCooked view (caConds ca) inst`; `gaActionId`/label from `caName`;
  `gaInstanceId` rendering unchanged (cold).
- `performAction`: look up the cooked action (by `gaActionId` within the cooked practice) and
  apply `map (groundCookedOutcome (gaBindings ga)) (caOuts ca)` through a new
  `performCooked :: CookedOutcome -> PraxState -> PraxState` that is `performOutcome`
  case-for-case on names: CInsert toks → classification via the token names directly
  (`relevantDelta` gains a names-level core: `relevantNames :: [String] -> [String] -> PraxState -> Bool`
  taking the names and shadow names — derive shadows from toks with the existing
  `evictionShadows` logic lifted to tokens), spawn check via the names (no re-parse),
  `insertToks`/`retract`-by-names; CForEach queries cooked and recurses; CCall resolves via
  `cpFns` with cooked cases. **The string `performOutcome` remains the public entry and
  delegates: `performOutcome o = performCooked (cookOutcome o)` — one engine, two doors.**
- Everything the old string path did — classification order (irrelevant → monotone →
  reclose), `applyGrow`, spawn-runs-inits — preserved structurally; only the parsing moves.

- [ ] Steps: one EngineSpec wiring test RED-first (cookedDefs present and
  `possibleActions`-visible: e.g. after `definePractices`, `Map.keys (cookedDefs st) @?= Map.keys (practiceDefs st)`);
  implement; nets (`-p "ViewInvariant"`, `-p "GoldenDrive"`, `-p "Village"`) then full suite once;
  gates; commit `"Engine runs on the cooked world: names end to end"`.
- **BLOCK rather than improvise** if any behavior can't be preserved by pure transcription.

---

### Task 3: Cooked wants and cooked rules

**Files:** `src/Prax/Types.hs`, `src/Prax/Engine.hs`, `src/Prax/Minds.hs`, `src/Prax/Planner.hs`, `src/Prax/Derive.hs`, plus every post-construction `{ characters = … }` record-update site (grep; fixtures switch to a new `setCharacters`), `test/Prax/RelevanceSpec.hs` or `test/Prax/MindsSpec.hs` (wiring test).

**Design:**
- `PraxState` gains `cookedWants :: Map String [[CookedCondition]]` keyed by character name
  (each want's cooked conditions, same order as `charWants`) and
  `cookedDesires :: Map String [CookedCondition]` keyed by desire name (the Owner-template,
  cooked once). Maintained by `retable`; **characters now need a helper**:
  `setCharacters :: [Character] -> PraxState -> PraxState` (retable), all post-construction
  `{ characters = … }` updates route through it, and the grep-gate extends to `characters`.
- `evaluate` gains the cooked form used by the Planner:
  `evaluateCooked :: PraxState -> [( [CookedCondition], Int )] -> Int`. `selfWants`/
  `believedWants` keep their public string forms (tests use them); the Planner's internal
  scoring path (`scoreActions`/`predictMove`/`pickAction`) switches to cooked lookups:
  self = `cookedWants ! actor` (+ own desires via `groundNames` of the Owner slot on
  `cookedDesires`), believed = believed desire names → cooked templates → Owner-ground.
  Utilities pair with the string forms by construction (same source lists, same order —
  state the invariant in a comment).
- `Derive`: `run`'s `rules` cook bodies once per call is already cheap, but closure is
  invoked ~5,400×/round — `PraxState` gains `cookedRules` maintained by `setAxioms`…
  **only if** `run` can accept them without changing `closure`'s public signature: add
  `runCooked` consumed by Engine's `reclose`/`applyGrow` (which have the state at hand),
  keeping `closure`/`closureFrom` as the public string-facing wrappers used elsewhere
  (Persist-free paths, tests, ViewInvariantSpec's recompute — which SHOULD stay on the
  independent string path precisely so the net's recomputation is not the same code under test).
- [ ] Steps: wiring test RED-first; implement; nets + full suite once; gates incl. the
  extended grep-gate (`characters` joins db/axioms/desires); commit
  `"Planner and closure score the cooked world"`.

---

### Task 4: Profile; decide the interning gate (controller)

Scratchpad-only, as v27 Task 4: profiled one-round drive; record total, top centres,
`tokens`/`parseNames` share; report. The controller decides v29 (interning) per the spec's
criterion — segment comparison/allocation still dominant, or not.

---

### Task 5: Docs

`docs/LEDGER.md` v28 legend row + backlog note (interning decision, measured numbers only);
`README.md` if warranted; full gate recorded; commit `"Docs: v28 — the world compiles once"`.
