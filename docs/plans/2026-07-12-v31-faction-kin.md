# v31 — Factions & Kinship Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `Prax.Faction` + `Prax.Kin` on the shared membership spine, the feud refactored onto factions with the cross-feud wedding beat, per `docs/specs/2026-07-12-v31-faction-kin.md`.

**Architecture:** Pure vocabulary + axiom round (the closure layer does the work; no engine changes). `FeudSpec` unmodified is the generalization's proof. One ontology note the spec implies and this plan makes explicit: base `allied.*` facts REMAIN legal vocabulary — not every alliance is a membership (`bigFeud`'s benchmark chain is pairwise BY DESIGN and keeps authored allieds, documented); `comrades` derives additional ones from shared membership.

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- Suite green after every task (360 baseline @ ~60s); ViewInvariant green (feud is in the net — its drives will shift with new cast/axioms, which is fine: it recomputes, nothing pinned); village/bar/intrigue goldens BYTE-IDENTICAL (this round must not touch those worlds — any drift is a BLOCK).
- `FeudSpec` unmodified through Task 3 (the refactor contract). If cast growth (esme) breaks a FeudSpec test, DO NOT edit it — use the contingency in Task 3.
- Zero warnings; hlint "No hints"; `prax check` ×7; grep-gates empty; loud-error guard idiom for all names.
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: `Prax.Faction`

**Files:** Create `src/Prax/Faction.hs`, `test/Prax/FactionSpec.hs`; wire prax.cabal (expose after `Prax.Repute`; spec after `Prax.ReputeSpec`), test/Spec.hs.

```haskell
-- | Factions: one membership spine (spec docs/specs/2026-07-12-v31-faction-kin.md §1).
-- Membership is a base, single-slot fact — @member.\<who\>!\<faction\>@ — and the
-- @!@ IS the semantics: joining, defecting, and marrying-in are all the same
-- exclusion overwrite. Base @allied.*@ facts remain legal vocabulary (not every
-- alliance is a membership); 'comrades' derives additional ones.
module Prax.Faction
  ( memberPath
  , joins
  , comrades
  , factionStanding
  ) where

import           Prax.Query (Condition (..))
import           Prax.Types (Outcome (..))
import           Prax.Derive (Axiom, axiom)
import           Prax.Db (isVariable, pathNames)

-- | @member.\<who\>!\<faction\>@ (single-slot: the primary allegiance).
memberPath :: String -> String -> String
memberPath who faction
  | bad who || bad faction =
      error ("Faction: names must be nonempty single path segments (no '.' or '!'): "
             ++ show (who, faction))
  | otherwise = "member." ++ who ++ "!" ++ faction
  where bad n = null n || any (`elem` (".!" :: String)) n

-- | Join (or defect to, or marry into) a faction: one exclusion overwrite.
joins :: String -> String -> Outcome
joins who faction = Insert (memberPath who faction)

-- | Shared membership derives alliance — the feud's old base facts, generalized.
-- The derived name stays @allied@ so every downstream consumer (mutuality,
-- enemy-of-my-ally, affordances) is unchanged.
comrades :: Axiom
comrades = axiom
  [ Match "member.X!F", Match "member.Y!F", Neq "X" "Y" ]
  [ "allied.X.Y" ]

-- | Belief-gated faction standing for K-discipline worlds: an offense against
-- my faction-mate, THAT I BELIEVE HAPPENED, makes me regard the offender.
-- @factionStanding pat label@: @pat@'s FIRST variable is the offender, SECOND
-- the victim (loud error otherwise) — e.g. @"struck.A.V"@ ⇒
-- @W.believes.struck.A.V ∧ member.V!F ∧ member.W!F ∧ W≠A ⇒ regards.W.A.\<label\>@.
factionStanding :: String -> String -> Axiom
factionStanding pat label =
  case filter isVariable (pathNames pat) of
    (offender : victim : _) -> axiom
      [ Match ("W.believes." ++ pat)
      , Match ("member." ++ victim ++ "!F")
      , Match ("member.W!F")
      , Neq "W" offender ]
      [ "regards.W." ++ offender ++ "." ++ label ]
    _ -> error ("factionStanding: pattern " ++ show pat
                ++ " must name an offender and a victim variable, in that order")
```

**FactionSpec** (RED-first; minimal inline fixtures, `readView` assertions):
comrades positive + X≠Y negative + cross-faction negative; defection un-derives (join a
different faction → old `allied` pairs gone from the view, new ones present — the
retraction-safety pin); `factionStanding`: unbelieved offense moves no one; believed offense
moves co-members only (assert a non-member believer derives nothing, the offender derives
nothing about themselves); both guards forced via try/evaluate.

- [ ] RED → implement → GREEN (suite once, report count) → gates → commit
  `"Prax.Faction: membership is the spine, exclusion is the semantics"`.

---

### Task 2: `Prax.Kin`

**Files:** Create `src/Prax/Kin.hs`, `test/Prax/KinSpec.hs`; wiring as usual (after `Prax.Faction`).

```haskell
-- | Kinship: base facts + derived closure (spec §2). Marriage moves membership —
-- the fold's payoff in one line.
module Prax.Kin
  ( kinAxioms
  , wed
  , succession
  ) where

import           Prax.Query (Condition (..))
import           Prax.Types (Action, Outcome (..), action)
import           Prax.Derive (Axiom, axiom)
import           Prax.Faction (joins)

-- | Marriage symmetry, siblings, grandparents, in-laws — all derived, all
-- retraction-safe (dissolve the base fact and the closure forgets with it).
kinAxioms :: [Axiom]
kinAxioms =
  [ axiom [ Match "married.A.B" ]                        [ "married.B.A" ]
  , axiom [ Match "parent.P.X", Match "parent.P.Y"
          , Neq "X" "Y" ]                                [ "sibling.X.Y" ]
  , axiom [ Match "parent.G.P", Match "parent.P.C" ]     [ "grandparent.G.C" ]
  , axiom [ Match "married.A.B", Match "parent.P.A" ]    [ "inLaw.P.B" ]
  , axiom [ Match "married.A.B", Match "sibling.A.S" ]   [ "inLaw.S.B" ]
  ]

-- | @wed joiner faction spouse@: the marriage fact plus the joiner's membership
-- overwrite. WHO moves households (and to which faction) is the author's choice
-- per wedding — world content, not module policy.
wed :: String -> String -> String -> [Outcome]
wed joiner faction spouse =
  [ Insert ("married." ++ joiner ++ "." ++ spouse)
  , joins joiner faction ]

-- | Succession as exclusion: any child of the dead holder may claim; the
-- single-slot office takes one — first motivated claimant wins. No invented
-- primogeniture (age does not exist in the vocabulary).
succession :: String -> Action
succession office
  | null office || any (`elem` (".!" :: String)) office =
      error ("succession: office " ++ show office ++ " must be a single path segment")
  | otherwise = action ("[Actor]: claim the office of " ++ office)
      [ Match ("office." ++ office ++ "!H")
      , Match "dead.H"
      , Match "parent.H.Actor"
      , Neq "Actor" "H" ]
      [ Insert ("office." ++ office ++ "!Actor") ]
```

**KinSpec** (RED-first): each kinAxiom positive + one negative each (e.g. sibling needs a
SHARED parent; inLaw only through marriage); wed's two facts + the membership overwrite
(joiner's old `allied` pairs un-derive — composition with Faction pinned here); dissolution
(Delete the married fact → in-laws gone from the view, membership UNCHANGED — the overwrite
was a base move, not a derivation; assert that explicitly, it is the designed asymmetry);
succession: not offered while the holder lives, only children may claim, a performed claim
overwrites the slot and closes the affordance for the other child (exclusion resolves the
race); guards.

- [ ] RED → implement → GREEN → gates → commit
  `"Prax.Kin: the closure knows your in-laws; marriage is one overwrite"`.

---

### Task 3: The feud refactored; the wedding

**Files:** `src/Prax/Worlds/Feud.hs`, `test/Prax/FeudSpec.hs` (ADDITIONS ONLY — existing tests unmodified).

- [ ] **Step 1**: refactor `feudWorld`: delete the two `allied.*` setup inserts; add
  `joins "bob" "kestrel"`, `joins "carol" "kestrel"`, `joins "dave" "kestrel"` to setup;
  `feudAxioms` gains `comrades` (keep the three existing axioms verbatim — mutuality now
  also mirrors derived allieds, harmlessly). `bigFeud` UNCHANGED (its pairwise chain is the
  benchmark's design — add the one-line comment saying so, citing the spec's ontology note).
  Run `-p "Feud"`: UNMODIFIED FeudSpec green (note: the old world derived `allied.bob.dave`
  only transitively through resents; the faction world derives it directly — if any existing
  FeudSpec assertion distinguishes those, that is a real behavioral difference: BLOCK with
  the trace, do not edit the test).
- [ ] **Step 2**: the wedding. Add `esme`: `grudgeBearer "esme"` + setup `joins "esme" "wren"`
  and `Insert "practice.society.here"`-style presence as needed. Import `Prax.Kin`. New
  FeudSpec tests (RED-first):
  - pre-wedding: esme derives no resentment, no kestrel alliances;
  - `wed "esme" "kestrel" "dave"` (the bride moves — authored direction) → readView flips:
    `allied.esme.bob/carol/dave` derived, `resents.esme.alice` derived (she inherits her
    in-laws' grudge against... the player who wronged them), her old wren alliance gone;
  - the driven beat: after the wedding, esme (a grudgeBearer: +5 per shun) SHUNS alice in a
    short driven window (`npcAct`-loop like FeudSpec's existing style, if it has one; else
    `pickAction` directly asserting the shun choice). If the planner does NOT pick it up,
    BLOCK with the scoreActions trace — never tune.
  - kin closure visible in the world: `married.dave.esme` derived symmetric.
  - **Contingency** (spec §3): if esme's presence breaks ANY existing FeudSpec test, revert
    her from `feudWorld` and build `weddingWorld` (a feudWorld variant + esme) in Feud.hs
    for the new tests instead; report which path was taken and why.
- [ ] **Step 3**: gates — suite green, FeudSpec pre-existing tests untouched (git diff
  check), village/bar/intrigue goldens byte-identical, ViewInvariant green, warnings/hlint/
  prax check ×7. Commit `"Feud: houses, not pairs — and a wedding across the lines"`.

---

### Task 4: Docs + gate

**Files:** `docs/LEDGER.md`, `docs/WALKTHROUGH.md`, `README.md` (if warranted).

- [ ] LEDGER: v31 legend row (the membership spine; FeudSpec-unmodified as the
  generalization proof; the base-allied ontology note; succession-as-exclusion; the
  village-wiring conditional recorded as deferred with `factionStanding` FactionSpec-pinned);
  Factions + Kinship backlog rows → done (with what remains banked: multi-affiliation,
  holdings inheritance, births, divorce-as-action, village faction wiring).
- [ ] WALKTHROUGH: a feud-section update or new section for the wedding beat — live
  transcripts only; sweep the existing feud section for falsification by the refactor.
- [ ] Full gate recorded; commit `"Docs: v31 — one spine, two generators"`.
