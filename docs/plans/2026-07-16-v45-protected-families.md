# v45 — Protected Families Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** per `docs/specs/2026-07-16-v45-protected-families.md` — `ClockWrite`
generalizes to one `ReservedFamily` check over a declared table (`turn`, `seed`,
`sceneEntered`, `contradiction`), with the read side scanned and the unforgeable
Prax-shape exemption. Guards on illegal input only: goldens byte-identical, no format
or engine change.

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- One check, one table, no dual: `ClockWrite` (constructor, check, describe arm, pins)
  is RENAMED into the general form — nothing keeps the old name.
- A STRENGTHENING the spec's table implies and the plan makes explicit: `Delete` is a
  write. The old `clockWriteErrors` scanned only `Insert`/`InsertFor`; an authored
  `Delete "turn"` (or `"seed"`) corrupts identically and now flags.
- The exemption is the spec's keystone — test it adversarially (mutations named below).
- RED-first per family × polarity; the all-shipped-worlds-clean pin is the exemption's
  load-bearing evidence (village/bar use `draw`; audience compiles a timed junction).
- Zero warnings; hlint; `prax check` ×7; suite green.
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: The check

**Files:**
- Modify: `src/Prax/TypeCheck.hs` (the generalization), `src/Prax/Script.hs` (export
  the family constant), `app/Main.hs` (describe arm), `test/Prax/TypeCheckSpec.hs`,
  plus whatever spec file carries the v44 turn-write pins (grep `ClockWrite`).

**1a. `Prax.Script`** exports the family constant beside its machinery (haddock: the
scene epoch family; engine-written; v45-reserved):

```haskell
sceneEnteredPath :: String
sceneEnteredPath = "sceneEntered"
```

(Replace the two string literals in `Script.hs` with the constant — one home.)

**1b. `Prax.TypeCheck` — the table, the shape test, the check (complete; replaces
`ClockWrite`/`clockWriteErrors` wholesale):**

```haskell
  | ReservedFamily { teFamily :: String, teWhere :: String, teSentence :: String }
    -- ^ an authored definition touches the engine-owned family @teFamily@
    -- (spec v45): its facts are machinery — written (and for some families
    -- read) only by compiled mechanism, whose accesses carry Prax-namespaced
    -- value variables no author can write (the v40 namespace ban makes the
    -- shape unforgeable).
```

```haskell
-- Check 5 (generalized, v45): engine-owned fact families. WritesForbidden
-- families (turn, contradiction) have NO legitimate authored writer at all —
-- reads stay free (turn is the documented time interface; a contradiction
-- read cannot corrupt). MachineryShapeOnly families (seed, sceneEntered) are
-- machinery in BOTH polarities: the only legal touch is the mechanism's own
-- compiled shape — every name after the family head a Prax-namespaced
-- variable. An authored literal, plain variable, or bare subtree pattern on
-- the family is a loud error: each mechanism assumes it is its family's
-- sole accessor, so an authored touch corrupts it silently otherwise.
data FamilyLaw = WritesForbidden | MachineryShapeOnly

reservedFamilies :: [(String, FamilyLaw)]
reservedFamilies =
  [ (turnPath,         WritesForbidden)
  , (seedPath,         MachineryShapeOnly)
  , (sceneEnteredPath, MachineryShapeOnly)
  , ("contradiction",  WritesForbidden)
  ]

reservedFamilyErrors :: PraxState -> [TypeError]
reservedFamilyErrors st =
     [ ReservedFamily fam loc s
     | (loc, os) <- writeSites st, o <- os, s <- writesOf o
     , Just (fam, law) <- [familyOf s], violatesWrite law s ]
  ++ [ ReservedFamily fam "axiom" h
     | ax <- axioms st, h <- axiomThen ax
     , Just (fam, law) <- [familyOf h], violatesWrite law h ]
  ++ [ ReservedFamily fam loc s
     | (loc, cs) <- readSites st, s <- concatMap condSents' cs
     , Just (fam, MachineryShapeOnly) <- [familyOf s], not (machineryShaped s) ]
  where
    familyOf s = case pathNames s of
      (h : _) -> (,) h <$> lookup h reservedFamilies
      []      -> Nothing
    violatesWrite WritesForbidden    _ = True
    violatesWrite MachineryShapeOnly s = not (machineryShaped s)
    -- The unforgeable signature: a non-empty tail, every name of which is a
    -- Prax-namespaced VARIABLE (authors cannot write those — v40).
    machineryShaped s = case pathNames s of
      (_ : rest@(_ : _)) -> all isPraxMachineryVar rest
      _                  -> False
    isPraxMachineryVar n = isVariable n && isPraxName n
    isPraxName ('P':'r':'a':'x':_:_) = True
    isPraxName _                     = False
    writesOf o = case o of
      Insert s      -> [s]
      InsertFor _ s -> [s]
      Delete s      -> [s]                    -- a delete is a write (NEW)
      ForEach _ os  -> concatMap writesOf os
      Call _ _      -> []
```

`writeSites` = the old `clockSites` (practice inits/actions/fn-cases) PLUS schedule
rule bodies' outcomes. `readSites` = every authored condition list: action conditions,
fn-case conditions, `ForEach` guards (via `writesOf`'s sibling recursion or the
existing outcome-condition walk), axiom bodies (`axiomWhen`), desires' and characters'
want conditions, and schedule rule bodies' conditions. `condSents'` = the existing
`condSents` (all pattern sentences, every polarity, subquery interiors included).
`isPraxName` duplicates `Prax.Types.isPraxVar`'s test — if `isPraxVar` is unexported,
EXPORT it from Types instead of duplicating (one home). `typeCheck`'s concat swaps
`clockWriteErrors` for `reservedFamilyErrors`; module-header bullet updated.

**1c. `app/Main.hs`:** the `ClockWrite` describe arm becomes:

```haskell
    describe (ReservedFamily fam w s) =
      "reserved family " ++ fam ++ ": " ++ show s ++ " (" ++ w
        ++ ") -- engine-owned; authored code may not touch it"
```

(Wording final per house voice; mention read-vs-write if the existing arm did.)

**1d. Tests (TypeCheckSpec; RED-first per pin — check implemented but unwired, observe
the failures, wire, observe green — the v42 idiom):**

1. Authored `Insert "turn!5"` flags (v44 pin updated to the new constructor/message);
   axiom-head turn write flags (same); `performOutcome` clock-jump still untouchable
   by the check (existing pin keeps passing).
2. NEW: authored `Delete "turn"` flags (the strengthening); authored `Delete "seed"`
   flags.
3. Authored `Insert "seed!7"` flags; `Insert "seed!X"` flags (plain variable);
   authored `Match "seed!S"` in an action condition flags (the READ side); bare
   `Match "seed"` flags; a schedule-rule body reading `seed!S` flags (site coverage).
4. THE EXEMPTION PIN: a fixture world whose action includes `draw` outcomes (and whose
   db carries `rngSetup`) typeChecks CLEAN — the die's own compiled read/write shapes
   pass. (The all-shipped-worlds pin is the real-world version: village/bar draw,
   audience compiles a timed junction — extend it over the new check unchanged.)
5. `sceneEntered`: authored write and read each flag; audience (in the all-worlds pin)
   stays clean.
6. `contradiction`: authored `Insert "contradiction"` flags; `Match "contradiction"`
   is clean (reads free).
7. `turn` reads free: a `sightedWithin`-shaped authored condition is clean (village in
   the all-worlds pin carries it live).

**Mutations after GREEN, each observed killing exactly its pin:**
- m1: drop the exemption (`machineryShaped _ = False`) → the exemption pin AND the
  all-shipped-worlds pin fail (draw worlds flag).
- m2: drop the read-side scan (third comprehension) → the seed-read pin alone fails.

**Process:** RED (unwired) → wire → GREEN → m1/m2 → full suite (goldens byte-identical;
expect 606 + new pins) → gates (zero warnings, hlint, `prax check` ×7) → commit
`"Protected families: one guard, one table, one unforgeable exemption"`.

- [ ] RED per pin → wire → GREEN → m1/m2 → suite + gates → commit.

---

### Task 2: Docs

**Files:** `docs/LEDGER.md`.

- [ ] The v45 legend row (house style), PLUS the audit record the queue came from: a
  short paragraph naming the four-auditor sweep (inventories in `.superpowers/sdd/`),
  the ranked findings, and THE QUEUE (v45 protected families → v46 the narrator dies →
  v47 function registry → v48 generality bundle), mirroring how the v40-43 queue was
  recorded. The row itself: the turn-only-whitelist finding (two auditors independently),
  the unforgeable-exemption design (v40's namespace ban doing double duty), the
  Delete-is-a-write strengthening, the read side, the two stated deferrals (`atSince` —
  contract-bound value; `storyAdvanced` — dies in v46), suite count as measured.
- [ ] Gates; commit `"Docs: v45 — the whitelist becomes a law"`.
