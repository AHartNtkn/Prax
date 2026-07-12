# v30 — Blackmail & Debt Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `Prax.Debt` (obligations with a beneficiary) and `Prax.Blackmail` (the probe-verified shakedown protocol), plus the village arc, per `docs/specs/2026-07-12-v30-blackmail-debt.md`.

**Architecture:** Two thin authored-vocabulary modules over shipped machinery (Deontic, Beliefs, Witness, Rumor's hearsay shape, Minds' motive-beliefs, v25 marks). No engine changes — if implementation finds otherwise, BLOCK and the spec is amended first. The session probe (scratchpad `V30Probe.hs`) is the reference fixture for `BlackmailSpec`.

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- Suite green after every task (329 baseline @ ~22-26s); ViewInvariant green; goldens byte-identical **except** the one sanctioned village-golden re-capture in Task 3 (its own commit, drift itemized line by line — an engine-caused drift is NOT sanctioned and is a BLOCK).
- Zero warnings; hlint "No hints"; `prax check` ×7; grep-gates empty.
- No heuristics: every weight in fixtures/demo is authored with stated meaning (the spec's compliance arithmetic); never tuned to force green — BLOCK with traces on surprises.
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: `Prax.Debt`

**Files:** Create `src/Prax/Debt.hs`, `test/Prax/DebtSpec.hs`; modify `prax.cabal` (expose after `Prax.Deontic`; test module after `Prax.DeonticSpec`), `test/Spec.hs`.

**Interfaces produced:**
```haskell
debtPath :: String -> String -> String -> String     -- "debt.<creditor>.<debtor>.<content>"
owe      :: String -> String -> String -> [Outcome]  -- debt fact + Deontic oblige (one call)
settle   :: String -> String -> String -> [Outcome]  -- delete debt + discharge
owes     :: String -> String -> String -> Condition  -- Match on the debt fact
```
(Check `Prax.Deontic`'s exact `oblige`/`discharge` argument orders before writing; content is
a single sentence — restate Deontic's stratification rule in the haddock and add the loud
guard: content containing no `.`-path is fine, but a creditor/debtor name containing `.`/`!`
errors, matching the established segment-guard idiom.)

- [ ] **Step 1: failing tests** (`test/Prax/DebtSpec.hs`, minimal inline fixture — follow
  DeceitSpec's style; use `performOutcome` folds):

```haskell
  -- lifecycle: owe creates BOTH facts; settle removes BOTH
  -- (assert exact sentences via exists: "debt.cora.dell.repaid.dell.cora.coin"
  --  and Deontic's obligationPath applied to the same content)
  -- demand->deadbeat: a fixture practice with a demand action that performs
  --  Deontic `breach` wrapped `observable` (Witness): after an UNWITNESSED
  --  default (no co-present witness) NO regards.*.dell.deadbeat derives; after
  --  a witnessed one it does (standingUnless "violated..." defeated by settle's
  --  cleanup — assert the defeat too)
  -- guards: dotted creditor name errors loudly
```
Write these as real testCases with exact asserted sentences (the implementer writes the
fixture; the properties above are the required assertions, each its own testCase).

- [ ] **Step 2** RED (module missing) → **Step 3** implement → **Step 4** GREEN (suite once,
  count reported), gates → **Step 5** commit `"Prax.Debt: a debt is an obligation with a beneficiary"`.

---

### Task 2: `Prax.Blackmail`

**Files:** Create `src/Prax/Blackmail.hs`, `test/Prax/BlackmailSpec.hs`; cabal/Spec.hs wiring.

**Interfaces produced:**
```haskell
shakedown :: String      -- id (single path segment; loud error)
          -> CoPresence  -- audience/co-presence template (Actor/Witness vars)
          -> String      -- evidence pattern; FIRST variable = the victim (lie's convention)
          -> String      -- the price (debt content suffix, e.g. "favor")
          -> Int         -- punitive weight: +w per believer once threatened/defied
          -> (Desire, [Action])
```
Generated (transcribe the session probe's shapes, generalized):
- fact paths id-scoped: `threatened.<id>.<extorter>.<victim>`, `defied.<id>.<victim>.<extorter>`.
- **threaten**: conds = evidence belief (`beliefAbout "Actor" pat`), `Neq victim "Actor"`,
  co-presence (`asRole victim copresence`), `Not threatened…`; outs = the threatened fact,
  the motive-belief deposit
  (`beliefAbout victim ("desires.Actor.punishes-" ++ sid) ++ ".heard.Actor"`), and the mark
  (`"Actor.extorted." ++ victim ++ "." ++ pat`).
- **comply** (victim as Actor, extorter as variable `E`): threat stands →
  `owe "E" "Actor" price ++ [Delete threatened…]` (Task 1's `owe` — the composition point).
- **defy**: threat stands → defied fact + delete threat.
- **expose**: `Or [[threatened Actor V], [defied V Actor]]` ∧ evidence ∧ hearer gates
  (asRole "Hearer", `Neq` Actor/victim, `Not (beliefAbout "Hearer" pat)`) →
  `Insert (beliefAbout "Hearer" pat ++ ".heard.Actor")` (Rumor's sourced-hearsay shape).
- The `Desire`: `punishes-<id>` = `Want [Or [[defied <id> D Owner],[threatened <id> Owner D]],
  Match (beliefAbout "W" patD)] w` where `patD` is the evidence pattern with the victim
  variable renamed to `D` (implement the rename op-preservingly via `tokens`/`tokensToSentence`).
- Labels generated (`"[Actor]: threaten [" ++ victim ++ "] with what you know"` etc., matching
  the probe's phrasing); a world wanting bespoke text wraps the actions itself.
- Guards: sid segment guard; evidence pattern must name a variable (lie's error text style).

- [ ] **Step 1: failing tests.** Port the session probe fixture
  (`/tmp/…/scratchpad/V30Probe.hs` — read it; if the scratchpad is gone, the shapes above and
  the spec §2 fully determine it) into `BlackmailSpec` USING `shakedown` + `Prax.Debt`, with
  vocabulary `fears-scandal` (−10) + `conventional`, and BOTH audience arities:
  - extorter threatens at depth 2 (holds the punitive desire via charDesires);
  - **two onlookers: victim complies** (pins: debt fact + obligation exist after, threat
    gone, expose unavailable);
  - **one onlooker: victim rationally defies** (the arithmetic IS the mechanic — both sides
    pinned per spec §4);
  - the threat deposited the motive-belief; victim's model predicts exposure after defiance
    (`predictMove`);
  - stalling ties defiance (assert via `scoreActions`: wait and defy scores equal under a
    standing threat);
  - the extorted mark exists after threatening; a `Trait` pricing `extorted.Owner.…`
    negatively deters the shakedown (pickAction declines — v25 composition, an
    unprincipled twin still threatens).
- [ ] **Step 2** RED → **Step 3** implement → **Step 4** GREEN + gates → **Step 5** commit
  `"Prax.Blackmail: a threat is a motive-belief deposit"`.

---

### Task 3: The village arc

**Files:** `src/Prax/Worlds/Village.hs`, `test/Prax/VillageSpec.hs`, `test/Prax/GoldenDriveSpec.hs` (the ONE sanctioned re-capture, separate commit).

- [ ] **Step 1: probe both arcs** (scratch drives, report traces for the chosen one):
  - **Preferred (spec §3): carol shakes down eve.** Needs the whispering ACT observable:
    wrap the village's `lie` action with `witnessed together "whispered.Actor.Hearer"`
    (one line — content stays secret, the act doesn't), an inference axiom is NOT needed —
    carol's evidence for the shakedown is the whispering belief itself
    (evidence pattern `"whispered.V.H"`); eve's fear: her frame-up collapses if the village
    learns she whispers — authored as her `fears-scandal`-shaped desire over
    `believes.whispered.eve` heads, weighted per the compliance arithmetic against the
    audience she faces. carol carries the punitive desire + a want for the favor.
  - **Fallback: dana shakes down bob** post-forced-theft (evidence `"stole.V.loaf"`,
    bob's existing conceal/notoriety weights already price compliance).
  - Choose by: which drives the full threaten→comply arc in free play (or from the standard
    forced start) without weight changes to EXISTING characters beyond additive desires.
    Ship ONE; record the other's trace and why in the report.
- [ ] **Step 2**: RED-first VillageSpec tests for the chosen arc (threat fires in the drive;
  compliance; the debt exists; no exposure happened; the reputation stack UNDISTURBED for
  uninvolved parties). Sanctioned turn-budget/watcher-list adjustments only if the cast
  grows (it should not — no new characters needed).
- [ ] **Step 3**: implement; suite green EXCEPT the village golden (expected drift — the
  world changed). **Separate commit** for the golden re-capture: fresh capture via the Task-1
  (v26) capture flow, with the commit message itemizing each changed decision and its cause
  (new affordances/desires). ViewInvariant must stay green THROUGHOUT (it recomputes, no
  pinned data).
- [ ] **Step 4**: gates; commits: `"Village: the shakedown arc"` + `"Golden re-capture: the village learned to blackmail"`.

---

### Task 4: Docs + gate

**Files:** `docs/LEDGER.md`, `docs/WALKTHROUGH.md`, `README.md` (if warranted).

- [ ] LEDGER: v30 legend row (probe findings: motive-belief threats, standing-threat
  exposure, the compliance arithmetic with both pinned sides, self-motivated credibility);
  Blackmail + Debt backlog rows → done; the v25 getting-caught parked item → partially
  landed if the carol/eve arc shipped (the ACT-observability line), noted precisely.
- [ ] WALKTHROUGH: new section for the shakedown arc — ALL transcripts from live runs;
  falsification-sweep any village sections the new vocabulary perturbs (the golden
  re-capture's drift list is the map of what to check).
- [ ] Full gate recorded (suite/warnings/hlint/prax check ×7/grep-gates); commit
  `"Docs: v30 — leverage, priced"`.
