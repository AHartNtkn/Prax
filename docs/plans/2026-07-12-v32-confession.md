# v32 — Confession & Absolution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `Prax.Confession` (confess/absolve/incorrigible) + eve's redemption arc, per `docs/specs/2026-07-12-v32-confession.md`.

**Architecture:** One thin module over shipped machinery (marks, hearsay, Repute's Count idiom, defeaters); the round's content is the design decisions already fixed in the spec. Probe-first inside Task 1 (v30's discipline): the spontaneous-confession and blackmail-defense arithmetic get MEASURED in the fixture before their assertions are pinned.

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- Suite green after every task (393 baseline @ ~55s); ViewInvariant green; village/bar/intrigue goldens byte-identical UNLESS the village vocabulary shift moves free play — then ONE itemized re-capture in its own commit (v30's discipline; an engine-caused drift is a BLOCK).
- Probe numbers are authored once measured — never tuned to force an assertion; BLOCK with scoreActions traces on surprises.
- Reserved-variable guards on every pattern argument at birth (the v30/v31 class).
- Zero warnings; hlint "No hints"; `prax check` ×7; grep-gates.
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: `Prax.Confession` + the probed arithmetic

**Files:** Create `src/Prax/Confession.hs`, `test/Prax/ConfessionSpec.hs`; wire prax.cabal (after `Prax.Blackmail`), test/Spec.hs.

**The module** (complete; adjust only what compilation forces, reporting any deviation):

```haskell
-- | Confession & absolution (spec @docs/specs/2026-07-12-v32-confession.md@).
-- Marks convert, never delete; confession self-incriminates through the
-- ordinary hearsay channel; absolution is a refusable second-party grant;
-- an absolver's patience is what they KNOW (per-regarder, permanent by
-- memory, fed by gossip and confession alike).
module Prax.Confession
  ( confess
  , absolve
  , incorrigible
  ) where

import           Prax.Db (isVariable, pathNames)
import           Prax.Query (Condition (..), CmpOp (..))
import           Prax.Types (Action, Outcome (..), action)
import           Prax.Derive (Axiom, axiom)
import           Prax.Beliefs (beliefAbout)
import           Prax.Witness (CoPresence, asRole)

segOk :: String -> Bool
segOk n = not (null n) && all (`notElem` (".!" :: String)) n

reservedIn :: String -> [String] -> [String]
reservedIn pat vs = [ v | v <- filter isVariable (pathNames pat), v `elem` vs ]

-- | Confess ONE deed (one mark binding) to a co-present hearer. The lied-mark
-- is the precondition and it CONVERTS — a deed can be confessed once; further
-- hearers learn by gossip. @H@ is the mark's own original-hearer slot and is
-- reserved in the event pattern.
confess :: String -> CoPresence -> String -> String -> Action
confess kind copresence pat label
  | not (segOk kind) =
      error ("confess: mark kind " ++ show kind ++ " must be a single path segment")
  | (v : _) <- reservedIn pat ["H", "Hearer", "Actor"] =
      error ("confess: event pattern " ++ show pat ++ " reserves variable " ++ show v
             ++ " (the mark's hearer slot / the action's own roles)")
  | otherwise = action label conds outs
  where
    liedPath      = "Actor." ++ kind ++ ".H." ++ pat
    confessedPath = "Actor.confessed.H." ++ pat
    conds = Match liedPath
          : asRole "Hearer" copresence
         ++ [ Neq "Hearer" "Actor" ]
    outs  = [ Delete liedPath
            , Insert confessedPath
            , Insert (beliefAbout "Hearer" pat ++ ".heard.Actor") ]

-- | Grant absolution: insert the world's defeater for a deed confessed TO YOU
-- (the belief must be heard from its own doer — gossip does not qualify),
-- unless your patience is spent (the incorrigibility regard).
absolve :: String -> String -> String -> String -> Action
absolve defeater pat incLabel label
  | not (segOk defeater) || not (segOk incLabel) =
      error ("absolve: defeater/label must be single path segments: "
             ++ show (defeater, incLabel))
  | (v : _) <- reservedIn pat ["Actor"] =
      error ("absolve: event pattern " ++ show pat ++ " reserves variable " ++ show v)
  | otherwise = case filter isVariable (pathNames pat) of
      [] -> error ("absolve: event pattern " ++ show pat
                   ++ " names no one (the FIRST variable is the confessor)")
      (confessor : _) -> action label
        [ Match (beliefAbout "Actor" pat ++ ".heard." ++ confessor)
        , Neq "Actor" confessor
        , Not ("regards.Actor." ++ confessor ++ "." ++ incLabel)
        , Not (defeater ++ "." ++ confessor) ]
        [ Insert (defeater ++ "." ++ confessor) ]

-- | Patience as knowledge: W regards the offender @label@ once W believes at
-- least @k@ distinct instances of the deed — however W learned them.
-- Notoriety's Count idiom pointed inward (continuation-safe: Cmp Gte, literal
-- right). The pattern's FIRST variable is the offender; W/Ds/N are reserved.
incorrigible :: String -> Int -> String -> Axiom
incorrigible pat k label
  | not (segOk label) =
      error ("incorrigible: label " ++ show label ++ " must be a single path segment")
  | (v : _) <- reservedIn pat ["W", "Ds", "N"] =
      error ("incorrigible: pattern " ++ show pat ++ " reserves variable " ++ show v)
  | otherwise = case filter isVariable (pathNames pat) of
      [] -> error ("incorrigible: pattern " ++ show pat ++ " names no offender")
      (offender : rest) -> axiom
        [ Match ("W.believes." ++ pat)
        , Subquery "Ds" rest [ Match ("W.believes." ++ pat) ]
        , Count "N" "Ds"
        , Cmp Gte "N" (show k) ]
        [ "regards.W." ++ offender ++ "." ++ label ]
```

(One structural check before writing tests: `Subquery "Ds" rest …` with `rest` = the
pattern's non-offender variables — confirm against `Prax.Repute.notoriety`'s exact shape
that the outer `Match` binds W and the offender while the subquery re-binds only the deed
variables; if `rest` is empty — a single-variable pattern — a threshold over ONE deed shape
degenerates to k≤1 semantics: make that a loud error (`incorrigible: pattern has no deed
variables to count`) rather than a silent constant.)

**ConfessionSpec** (RED-first throughout; fixtures in the DeceitSpec style; PROBE then pin):
1. Conversion mechanics: confessing converts exactly the confessed deed's mark (a second
   lied-mark survives), deposits the hearer's sourced belief, is not re-offered for that
   deed; a trait pricing `lied` at −6 and `confessed` at 0 shows the relief in
   `evaluate`/`selfWants` numbers (v25 composition).
2. Absolution: grant inserts the defeater; refusal gate (a planted incorrigibility regard
   blocks the affordance); gossip-sourced belief does NOT qualify (heard from a
   non-doer — assert the affordance absent); double-absolution blocked while the defeater
   stands.
3. Incorrigibility: k−1 believed deeds derive nothing, k derive the regard; gossip feeds
   the count; per-absolver independence (one fed-up, one fresh); permanence (nothing in
   the fixture can retract it — assert it survives the offender's absolution elsewhere).
4. **Probed arithmetic (measure FIRST with scoreActions, then pin both sides)**:
   spontaneous confession — a conscience-bearer with a mild secret confesses at depth 2;
   raise the concealment stake and they don't (report the measured scores in comments,
   v30's idiom). Blackmail-defense — construct the shakedown where the price outweighs
   the realized fear: the victim CONFESSES instead of complying, and the extorter's
   expose is dead afterward (deposits nothing new — assert the affordance/emptiness);
   converse case complies. If any probe contradicts the spec's expectation, BLOCK with
   the trace — the spec gets amended, not the weights.
5. Re-offense: a fixture lie action carrying `Delete "recanted.X"` snaps standing back
   (v21 idiom pinned here at module level, ahead of the village wiring).
6. All guards forced (try/evaluate), including the no-deed-variables incorrigible case.

- [ ] RED (module missing) → implement → probes → GREEN (suite once, count reported) →
  gates → commit `"Prax.Confession: conscience converts, absolvers refuse, memory is the patience"`.

---

### Task 2: Eve's redemption (the village arc)

**Files:** `src/Prax/Worlds/Village.hs`, `test/Prax/VillageSpec.hs` (additions + the ONE
sanctioned re-capture of the village golden IF free play drifts — separate commit,
itemized), `test/Prax/GoldenDriveSpec.hs` (only in that contingency).

- [ ] **Step 1: wire the vocabulary** — village gains: the whisper's
  `Delete "recanted.Actor"` (re-offense snaps the defeater); `confess "lied" together
  "stole.C.loaf" label` and an `absolve "recanted" "whispered.V.H" "incorrigible" label`…
  **STOP — reconcile the two event shapes first**: eve's lied-mark is
  `eve.lied.dana.stole.carol.loaf` (the CONTENT lie) while her slanderer standing keys on
  `whispered.eve.H` (the ACT). Confession must clear what the standing derives from:
  decide, comment, and implement ONE coherent wiring — the natural one: she confesses the
  whispering itself. But `whispered.*` is a witnessed act-fact, not a mark. The clean
  reconciliation (spell it out in the code comment): the confession converts her
  `lied.*` mark (conscience), its DEPOSIT is the whispered-act belief
  `Hearer.believes.whispered.eve.dana.heard.eve` (the standing's base — use `confess`'s
  pat = the act, conditioned on… the mark has the content-arity). If `confess`'s
  single-pattern design cannot express mark-of-one-shape/deposit-of-another, that is a
  REAL module-design finding: BLOCK, and the module gains an explicit
  deposit-pattern parameter by spec amendment — do not improvise privately.
- [ ] **Step 2: probe both arcs** (traces in the report): primary — carol (the wronged,
  with a professed `merciful` desire so eve's depth-2 sees confess→absolve through her
  believed model) absolves eve after confession, threshold drama measured; fallback —
  eve confesses to gale (already a believer: free confession, no new regard) and gale
  absolves. Ship the one that drives on a forced trajectory (the wedding/theft precedent);
  record the other.
- [ ] **Step 3**: RED-first VillageSpec additions for the chosen arc (confession, the
  deposit, absolution inserting `recanted.eve`, slanderer regards dissolved in the view,
  beliefs intact; re-whisper AFTER absolution snaps standing back AND now faces the
  incorrigibility ledger if wired — wire `incorrigible "whispered.V.H" k "incorrigible"`
  into villageAxioms with k authored and stated). Free-play preservation: eve does not
  confess unprompted (assert it — her secret is expensive; if the probe shows otherwise,
  that IS a golden drift: re-capture, itemize, justify).
- [ ] **Step 4**: gates; commit(s): `"Village: eve's road back"` (+ the re-capture commit
  only if needed).

---

### Task 3: Docs + gate

**Files:** `docs/LEDGER.md`, `docs/WALKTHROUGH.md`, `README.md` (if warranted).

- [ ] LEDGER: v32 legend row (the four design decisions, the probed arithmetic with
  measured numbers, the blackmail-defense composition); confession/absolution's origin
  rows updated (v25's parked discharge, v30's dangling recanted → closed); the banked
  trait-acquisition row with the charDesires-is-static obstacle; incorrigibility noted on
  the Repute row as the Count idiom's second use.
- [ ] WALKTHROUGH: the redemption section with live transcripts; sweep for falsification
  (eve's sections again — her free play must be re-verified unchanged, or retold).
- [ ] Full gate recorded; commit `"Docs: v32 — the road back is real, and it narrows"`.
