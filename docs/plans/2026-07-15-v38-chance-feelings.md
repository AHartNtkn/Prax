# v38 — Chance & Feelings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** the infrastructure round per `docs/specs/2026-07-15-v38-chance-feelings.md`: `Mod`,
the seeded drama die (`Prax.Rng`), coexisting feelings (`Prax.Emotion`, the mood system
replaced wholesale), with emotions as the demonstrating application (bar migration +
short-tempered carol).

**Architecture:** one operator in the evaluator; one new library module compiling draws to
ordinary outcomes over a `seed!N` fact; one library migration (Core's mood section deleted,
five consumers moved); world cargo. Shared AST walkers (`conditionVars`/`outcomeVars`) move
to `Prax.Query` as the one home (Drift's private copies deleted).

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- THE INVARIANT (user, load-bearing): emotions change decision-making, never what decisions
  can be made — the mechanism adds no availability gating; `candidateActions` identical in
  every mood (pinned). Feeling-content preconditions (an act that EXPRESSES the feeling)
  are ordinary authored conditions and stay.
- Determinism end-to-end: fixed authored seeds; same seed ⇒ same stream ⇒ same goldens.
  Every draw consumes exactly one stream step, hit or miss (the frozen-die law, pinned).
- No dual systems: the single-slot mood (`mood!`/`priorMood`/`because`/`setMood`/`moodIs`/
  `setMoodFn`) is DELETED in the same round that replaces it; all five consumers migrate.
- Authored numbers carry their sentence (odds, weights, fade periods — fade shipped
  test-compressed WITH the standard truncation label; real-authoring reference ~24-48
  rounds, hours). LCG constants are mechanism with cited provenance, never tuned.
- RED-first observed per task; goldens re-captured only in own commits, itemized;
  ViewInvariant green; zero warnings; hlint "No hints".
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: `Mod`, the shared AST walkers, and `Prax.Rng`

**Files:**
- Modify: `src/Prax/Query.hs` (CalcOp + applyCalc + the walkers), `src/Prax/Drift.hs`
  (guardRule refactors onto the walkers; its private copies deleted),
  `src/Prax/TypeCheck.hs` + `app/Main.hs` (`SeedlessDraw`), `prax.cabal`
- Create: `src/Prax/Rng.hs`
- Test: `test/Prax/QuerySpec.hs` (Mod pin), `test/Prax/RngSpec.hs` (new, registered like
  siblings), `test/Prax/DriftSpec.hs` only if the refactor moves an error message

**Interfaces produced:** `Mod :: CalcOp`; `Prax.Query.conditionVars :: Condition ->
[String]`, `outcomeVars :: Outcome -> [String]` (every name a condition/outcome mentions,
total walks — Drift's `condNames`/`outNames` verbatim, relocated); `Prax.Rng.rngSetup ::
Integer -> [Outcome]`, `draw :: Int -> Int -> [Condition] -> [Outcome] -> [Outcome]`,
`seedPath :: String`.

**Design.** `data CalcOp = Add | Sub | Mul | Mod`; `applyCalc Mod = mod` (Haskell
semantics: non-negative for positive divisor — say so in the haddock; the Praxish
no-division rationale was integrality, which Mod preserves). `-Wall` finds any
non-exhaustive CalcOp case; fix each exhaustively.

`src/Prax/Rng.hs` (complete):

```haskell
-- | A seeded drama die as ordinary world state (spec
-- @docs/specs/2026-07-15-v38-chance-feelings.md@): one @seed!N@ fact, a
-- Lehmer stream over it, and 'draw' — the only authoring surface. Because
-- the die is facts, determinism, goldens, replay, and 'Prax.Persist' all
-- survive for free; the initial seed is an AUTHORED world parameter (it
-- selects the playthrough's fate, and the goldens pin it). This is a drama
-- die, not a statistics library: low bits of a MINSTD stream are plenty to
-- decide whether a temper flares, and nothing here is fit for cryptography
-- or simulation science.
module Prax.Rng
  ( rngSetup
  , draw
  , seedPath
  ) where

import           Prax.Query (Condition (..), CmpOp (..), CalcOp (..),
                             conditionVars, outcomeVars)
import           Prax.Types

-- | The die's fact family. One slot: the stream position.
seedPath :: String
seedPath = "seed"

-- Park & Miller (1988), "Random number generators: good ones are hard to
-- find" — the MINSTD minimal standard. Mechanism with published provenance,
-- fixed here, never tuned: the AUTHORED numbers are 'draw''s odds.
lehmerA, lehmerM :: Integer
lehmerA = 16807
lehmerM = 2147483647   -- 2^31 - 1

-- | Seed the die (append to world setup). Loud on a seed outside the
-- stream's domain (0 and multiples of the modulus are fixed points — a die
-- that always rolls the same face).
rngSetup :: Integer -> [Outcome]
rngSetup s
  | s <= 0 || s >= lehmerM =
      error ("Prax.Rng: seed must lie in (0, " ++ show (lehmerM - 1) ++ "]")
  | otherwise = [ Insert (seedPath ++ "!" ++ show s) ]

-- | "With probability num\/den, where conds also hold, apply outs." The
-- fragment authors append to an action's outcomes. Compiles to TWO
-- ForEach outcomes: an unconditional stream advance, then the roll against
-- the fresh seed — so every draw consumes EXACTLY one step, hit or miss
-- (a failed roll must not freeze the die: a provocation that failed once
-- must not fail identically forever). Guards are loud: odds must be a real
-- chance (0 < num < den — certainty and impossibility are authored
-- dishonesty; use a plain outcome or nothing), and the caller's conditions
-- and outcomes may not use the reserved stream variables.
draw :: Int -> Int -> [Condition] -> [Outcome] -> [Outcome]
draw num den conds outs
  | num <= 0 || num >= den =
      error ("Prax.Rng: draw odds " ++ show num ++ "/" ++ show den
             ++ " must satisfy 0 < num < den")
  | any (`elem` reserved) callerVars =
      error "Prax.Rng: draw bodies may not use the reserved variables S/S2/S3/R"
  | otherwise =
      [ ForEach [ Match (seedPath ++ "!S")
                , Calc "S2" Mul "S" (show lehmerA)
                , Calc "S3" Mod "S2" (show lehmerM) ]
                [ Insert (seedPath ++ "!S3") ]
      , ForEach ([ Match (seedPath ++ "!S")
                 , Calc "R" Mod "S" (show den)
                 , Cmp Lt "R" (show num) ] ++ conds)
                outs
      ]
  where
    reserved = ["S", "S2", "S3", "R"]
    callerVars = concatMap conditionVars conds ++ concatMap outcomeVars outs
```

`Prax.Query` gains `conditionVars`/`outcomeVars` — Drift's `condNames`/`outNames` walkers
verbatim (total: every constructor, subquery internals included), exported;
`Prax.Drift.guardRule` refactors onto them and its private copies are deleted (one home).

`Prax.TypeCheck` gains `SeedlessDraw`: flagged when any registered practice's action (or
init, or function-case) outcomes contain a `ForEach` whose conditions read the
`seed` family (walk outcomes with the same walkers, test the pattern head against
`seedPath`) while the db has no `seed` fact. `app/Main.hs` describes it:
`"draw used but the die is unseeded: append Prax.Rng.rngSetup to the world's setup"`.

**Tests (RED-first — new names missing):**
- QuerySpec: `Calc "R" Mod "17" "5"` binds 2; negative-left case pinned to Haskell `mod`
  semantics (e.g. -3 `mod` 5 = 2).
- RngSpec (drive with `performOutcome` over a tiny world; read `seed!` between steps):
  - determinism: same seed, same three-draw stream of values (assert the exact Lehmer
    values, computed in the test by the same formula — self-checking arithmetic);
  - THE FROZEN-DIE LAW: two draws whose `conds` are unsatisfiable still advance the seed
    twice (assert seed = lehmer²(s₀) exactly);
  - a hit applies outs; a miss does not (choose seeds where the first roll hits/misses
    respectively — derive by computing, assert both);
  - guards: num 0, num == den, reserved-var use, bad rngSetup seed — all loud
    (try/evaluate/ErrorCall idiom);
  - `SeedlessDraw`: flagged on a world with a draw and no seed; cleared by rngSetup.
- DriftSpec: unchanged behavior after the walker refactor (its guard pins still pass).

- [ ] RED → implement → GREEN (`-p "Query"`, `-p "Rng"`, `-p "Drift"`) → full suite (no
  world changed; goldens byte-identical) → gates → commit
  `"Rng: a seeded die in world state — every draw spends one step"`.

---

### Task 2: `Prax.Emotion` and the mood replacement (mechanical consumers)

**Files:**
- Create: `src/Prax/Emotion.hs`; Test: `test/Prax/EmotionSpec.hs` (new)
- Modify: `src/Prax/Core.hs` (mood section DELETED: `setMood`, `moodIs`, `setMoodFn`, the
  Ekman list, priorMood/because machinery — coreLib's function list shrinks),
  `src/Prax/Reactions.hs`, `src/Prax/Worlds/Play.hs`, `src/Prax/Worlds/Intrigue.hs`,
  `test/Prax/CoreSpec.hs` (mood pins move to EmotionSpec), `test/Prax/DirectorSpec.hs`
  (fixture path), `prax.cabal`

**Design — `src/Prax/Emotion.hs` (complete):**

```haskell
-- | Coexisting episodic feelings (spec 2026-07-15-v38), replacing the
-- Versu-inherited single-slot mood: @\<who\>.feels.\<emotion\>@ and
-- @\<who\>.feels.\<emotion\>.toward.\<target\>@ are plain multi-valued
-- facts — angry at two people while afraid of a third all coexist, each
-- independent. THE INVARIANT (user, load-bearing): feelings change
-- decision-making, never what decisions can be made — nothing here touches
-- action availability; pricing is ordinary desires reading these facts,
-- authored per world. Authoring guidance: prefer NEGATIVE pricing (a
-- feeling as discomfort driving its own discharge) — the psychology is
-- right and v33's FloorCheck keeps the unfelt state planning-free, where
-- positively-priced feelings are action-insertable and thus AlwaysLive
-- (allowed; the cost is the cost). Onset is authored at the provoking
-- action ('Prax.Rng.draw' fragments); wear-off is a 'Prax.Drift' pulse
-- ('feelingsFade'). Feelings are EPISODIC (v36): they fade; dispositions
-- (traits, marks) never do — a trait makes a feeling LIKELIER, not longer.
module Prax.Emotion
  ( -- * An Ekman-based vocabulary (moved from Prax.Core; plain names)
    happy, sad, angry, afraid, disgusted, surprised, annoyed, pleased
    -- * Feeling and unfeeling (Outcomes)
  , feel, feelToward, unfeel, unfeelToward
    -- * Reading feelings (Conditions)
  , feeling, feelingToward
    -- * Wear-off
  , feelingsFade
  ) where

import           Prax.Drift (DriftRule (..))
import           Prax.Query (Condition (..))
import           Prax.Types

happy, sad, angry, afraid, disgusted, surprised, annoyed, pleased :: String
happy = "happy"; sad = "sad"; angry = "angry"; afraid = "afraid"
disgusted = "disgusted"; surprised = "surprised"
annoyed = "annoyed"; pleased = "pleased"

feelsPath :: String -> String -> String
feelsPath who emotion = who ++ ".feels." ++ emotion

-- | @who@ comes to feel @emotion@ (untargeted).
feel :: String -> String -> Outcome
feel who emotion = Insert (feelsPath who emotion)

-- | @who@ comes to feel @emotion@ toward @target@. Arguments may be action
-- variables, grounded when the outcome runs.
feelToward :: String -> String -> String -> Outcome
feelToward who emotion target =
  Insert (feelsPath who emotion ++ ".toward." ++ target)

-- | Discharge: the whole feeling goes, targets included (venting,
-- confronting, being won over — authored at the discharging action).
unfeel :: String -> String -> Outcome
unfeel who emotion = Delete (feelsPath who emotion)

unfeelToward :: String -> String -> String -> Outcome
unfeelToward who emotion target =
  Delete (feelsPath who emotion ++ ".toward." ++ target)

-- | @who@ currently feels @emotion@ (matches targeted instances too —
-- 'Match' sees subtrees).
feeling :: String -> String -> Condition
feeling who emotion = Match (feelsPath who emotion)

feelingToward :: String -> String -> String -> Condition
feelingToward who emotion target =
  Match (feelsPath who emotion ++ ".toward." ++ target)

-- | Feelings fade: one pulse sweeping every feeling at an authored period.
-- TEST-COMPRESSED in shipped worlds (see Prax.Drift's authoring note; real
-- authoring: hours, ~24-48 rounds). Coarse by design: every standing
-- feeling fades on the same pulse regardless of onset time (per-feeling
-- stamps are banked until a world needs them).
feelingsFade :: Int -> DriftRule
feelingsFade period = DriftRule "feelingsFade" period
  [ ( [ Match "W.feels.E" ], [ Delete "W.feels.E" ] ) ]
```

**The migration (mechanical this task — no behavior change intended):**
- `Prax.Core`: mood section deleted wholesale (exports, `setMoodFn`, priorMood/because).
  The Ekman names now live in Emotion; Core's export list shrinks; anything importing the
  names from Core switches to Emotion.
- `Prax.Reactions.disapprovalP`: `setMood "Onlooker" annoyed "Offender" "brokeANorm"` →
  `feelToward "Onlooker" annoyed "Offender"` (the cause argument dies with the slot;
  forgive's `pleased` likewise).
- `Play.hs`/`Intrigue.hs`: each `setMood w f t c` → `feelToward w f t`.
- `DirectorSpec`: the `bex.mood!annoyed.toward!cai` fixture assertion → the feels path.
- `CoreSpec`'s mood pins (incl. the priorMood pin, which dies with the machinery) move to
  EmotionSpec as feels pins.

**EmotionSpec (RED-first):** coexistence (feel angry-at-carol AND afraid-of-bob; both
present; unfeel one, other survives); `feeling` matches targeted instances; fade-on-pulse
(a DriftSpec-style hand clock: both feelings gone after the pulse, reappear-able); THE
INVARIANT PIN: a fixture character's `candidateActions` IDENTICAL (full `GroundedAction`
list equality) with and without every vocabulary feeling present.

Nothing prices feelings yet and no condition-reader migrates this task, so **all goldens
must stay byte-identical** (Play/Intrigue only WROTE moods; decisions never read them).
Any golden movement = BLOCK.

- [ ] RED → implement → GREEN (`-p "Emotion"`, `-p "Core"`, `-p "Reactions"`,
  `-p "Director"`) → full suite, goldens byte-identical → gates → commit
  `"Emotion: feelings coexist — the single-slot mood is gone"`.

---

### Task 3: The bar migration (gates become pricing)

**Files:** `src/Prax/Worlds/Bar.hs`, `test/Prax/BarSpec.hs`; bar goldens (GoldenDriveSpec
bar + LoopSpec narration) re-captured in ONE own commit.

**Design.** Audit every `mood` reference in Bar.hs against the invariant:
- CONTENT preconditions stay, as feels conditions: complaining about Subject requires
  `feelingToward "Actor" annoyed "Subject"` (the act expresses the feeling).
- PURE AVAILABILITY GATES (the `Not "….mood!annoyed.toward!Other"` guards on buying a
  round / greeting) are REMOVED from conditions and replaced by pricing: the character
  CAN act warmly while cross but won't want to. Pattern (weights authored with their
  sentence, calibrated against the bar's existing +/- scale — derive the exact numbers
  from the actions' current stakes and state them):

```haskell
-- Grudging courtesy: doing someone a kindness while cross with them grates
-- -- strongly enough to outweigh the kindness's ordinary appeal, so a cross
-- character declines the round they COULD buy (the v38 invariant: the gate
-- is gone; the reluctance is priced).
, Want [ Match "<who>.feels.annoyed.toward.T", Match "<kindness-state-for T>" ] (-k)
```

**BarSpec pins (RED-first):** the both-halves pin — with bex annoyed at a patron,
`candidateActions` still CONTAINS the buy-the-round grounding for them (availability
half), and `pickAction` does NOT choose it (pricing half); un-annoy her (unfeel) and she
buys as before. Existing tipsy/arc pins re-anchored only where the feels paths changed.
Disapproval-annoyance now fades (feelingsFade wired into barWorld's drift rules — period
authored + truncation-labeled): pin one onlooker's annoyance fading on the pulse.

**Goldens:** bar free-play + LoopSpec narration re-captured, own commit, itemized (the
gate removal and fade WILL move decisions — each line's cause named).

- [ ] RED → implement → GREEN (`-p "Bar"`) → suite (village/intrigue/feud untouched or
  BLOCK) → golden commit → gates → commits `"Bar: cross bartenders may pour, and won't"`
  + `"Bar goldens: priced reluctance (v38, itemized)"`.

---

### Task 4: Village cargo — short-tempered carol

**Files:** `src/Prax/Worlds/Village.hs`, `test/Prax/VillageSpec.hs`; village golden own
commit if free play moves.

**Design (all numbers with sentences):**
- `shortTempered.carol` trait fact (a disposition: never fades; seeded in setup).
- `rngSetup <seed>` appended to village setup — the seed is authored (pick one, state
  that it selects the playthrough; the goldens pin it).
- Onset at the SHUN action (the direct, victim-present provocation; two draw arms):

```haskell
-- Being shunned stings: anyone might flare (1 in 4); a short temper flares
-- on most slights (a further 2 in 4, so 3 in 4 overall for the
-- short-tempered -- each arm's odds authored, the trait makes the feeling
-- LIKELIER, never longer).
shunOutcomes ++ draw 1 4 [] [ feelToward "T" angry "Actor" ]
             ++ draw 2 4 [ Match "shortTempered.T" ] [ feelToward "T" angry "Actor" ]
```

  (Adapt the variable names to the shun action's actual binding for the shunned party;
  double-hit inserts are idempotent; two draws = two stream steps, by design.)
- Pricing, the discomfort shape (FloorCheck-friendly): vocabulary desire
  `Desire "smoulders" (Want [ Match "Owner.feels.angry" ] (-8))` — carol's charDesires
  gains it; weight sentence: anger outweighs her +5 event wants (she acts on it) but not
  conduct stakes.
- Discharge: her existing confront affordance gains `unfeelToward "Actor" angry "T"`
  (venting ends the feeling); `feelingsFade` (already in the drift rules from Task 3's
  wiring pattern — village gets its own, period test-compressed + labeled) catches the
  unvented.
- v35 note (verify in pins): onset flips her satisfaction vector → she wakes; discharge
  and fade likewise.

**VillageSpec pins (RED-first):** onset arms across seeds (drive the SAME provocation
under two authored seeds chosen so one hits and one misses — compute which, assert both
exactly); trait arm (short-tempered carol vs an un-tempered control under a seed where
only the trait arm hits); anger drives the confrontation (angry carol's next pick is the
confront — the smoulder discharged, feeling gone); fade catches the unvented (hand
clock); the liveness pin (`smoulders` ⇒ FloorCheck); THE INVARIANT PIN again at world
scale (carol's candidateActions identical angry/calm).

**Golden:** if the shipped seed's free play changes (dana's shun fires a draw...), it is a
deliberate world change: own commit, itemized; the seed line named as the fate-selector.

- [ ] RED → implement → GREEN → suite → golden if moved → gates → commits
  `"Village: carol's temper -- chance, trait, smoulder, vent"` (+ golden).

---

### Task 5: Docs

**Files:** `docs/LEDGER.md`, `README.md` if stale.

- [ ] LEDGER v38 legend row (house style): the user's reframing (infrastructure round,
  emotions as the example app), the invariant restated, Mod's rationale-consistency, the
  die (provenance, the frozen-die law, seed-as-authored-fate), the coexistence migration
  (what died with the slot), the bar's gate→pricing shift with its golden itemization
  summary, carol's temper with the odds sentences, and the suite/timing as measured. Mark
  the Emotions bank row done; bank per-feeling fade stamps, per-emotion periods, emotion
  visibility to other minds (deterrence-by-anger), intensity levels.
- [ ] Gates; commit `"Docs: v38 -- a die for the drama"`.
