# v48 — The generality bundle: six hardcodings unpicked; the queue closes

Fourth and LAST of the audit-queued rounds (v45 protected families → v46 the narrator
dies → v47 function registry → **v48**). The audit's remaining MED/LOW findings: places
where one application's choices are baked into general machinery, or content squats in
a mechanism module. Bundled because none carries a round alone; each is a parameter, a
gate, a move, or a deletion — no new engine surface, so no pre-gate panel (the audit
record and this spec carry the rationale).

## 1. □-lifting gates on deontic producibility (the engine stops taxing every world)

`liftObliged` adds an `obliged.Obligor.*`-lifted twin for EVERY all-Match axiom in
EVERY world — but the tree has exactly ONE deontic consumer (Bar's tipping
obligation). Six worlds carry doubled rule sets, doubled footprints, and a perturbed
`obliged`-shaped namespace for a guarantee none of them can ever invoke (a lifted rule
can only fire if an `obliged.*` fact can exist). The fix is DETECTION, not a flag: an
author who uses obligations must get DEON property 1 automatically — a forgotten
opt-in flag would silently break entailment closure, the loud-safe choice is to lift
exactly when the world can produce `obliged.*` facts. The decision pool is the
NON-LIFTED producers (practice/schedule insert atoms, db facts, UNLIFTED axiom heads —
no cycle: lifted heads are exactly what's being decided). Mechanically this re-homes
`cookedRules` from `setAxioms` into `retable` (the lift decision depends on
practices/schedule, which change after `setAxioms`): `setAxioms` becomes
`retable`-then-`reclose`, and every setter keeps the tables coherent —
the v41 one-surface pattern, extended to the rule table. Exactness: Bar's tables
byte-identical (it lifts today, it lifts after); the six non-deontic worlds' lifted
rules VANISH — their `footprint`/`axiomHeads` AnalysisTable pins re-capture with
exactly the lifted rows removed, itemized, and their goldens must be byte-identical
(the vanished rules could never fire).

## 2. Blackmail's demand parameterizes; threat generalization is honestly BANKED

`shakedown`'s price is hardwired debt-content (comply → `Prax.Debt` currency). The
demand becomes authored outcomes (`[Outcome]` — compliance applies them; a favor, a
feeling, a fact, or the old debt all expressible; `Prax.Debt` stops being a hard
dependency of leverage). The THREAT side stays exposure — deliberately: the punitive
fear model (weight-per-believer-of-evidence) IS exposure's semantics, and a
non-exposure threat ("comply or I hurt X") needs its own fear design (pricing a
threatened consequence, not an audience). That is a real design round, not a
parameter — banked with this reason, closing the audit item honestly rather than
gesturing at generality the fear model cannot back.

## 3. Confession's discharge verb parameterizes

`confess` hardcodes `confessed` into the discharge path. The verb becomes an argument
(the caller names the conversion: recant, boast, admit — same machinery). The shipped
call sites pass `"confessed"`; behavior identical, goldens byte-identical.

## 4. Stress's coverage family parameterizes

`sceneReached` hardcodes Script's `currentScene` — the general harness special-cases
one application. Coverage becomes a declarable family parameter (which path prefix
marks a coverage bucket), defaulted by the CLI's script entry points to
`currentScene`; a non-Script world can now ask for market-phase or arc-stage coverage
without touching the module.

## 5. `disapprovalP` moves to its consumer

Shipped content (fixed magnitudes, specific emotions) lives in the `Reactions`
mechanism module — the same shape as the dead `coreLib` at lower stakes. It moves to
`Prax.Worlds.Bar` (its only shipped consumer); `ReactionsSpec` builds its own minimal
test reaction so the MECHANISM keeps unit coverage independent of Bar's content.

## 6. `feelingSomeone` collapses into `feelingToward`

A literal alias since v39 killed the residue trap. One name survives
(`feelingToward`); the per-target-pricing guidance (the reason the shape was kept)
moves onto the survivor's haddock. Village's `smoulders` and the specs re-point.

## Exactness

Items 2-6: goldens and pins byte-identical (parameter defaults preserve every shipped
behavior; the alias and the move are name-level). Item 1: the six non-deontic worlds'
AnalysisTable `footprint`/`axiomHeads` rows lose exactly their lifted entries
(itemized re-capture — each removed row named as a lifted twin); ALL goldens
byte-identical everywhere (vanished rules were unfireable). Any other movement =
BLOCK. Persist untouched.

## Verification

- Item 1 RED-first: a fixture with an `oblige`-bearing practice lifts (□-closure pin —
  DEON property 1 observed); the same fixture without it doesn't (no lifted heads in
  `axiomHeads`); adding the oblige practice AFTER `setAxioms` still lifts (the
  re-homing's whole point — pinned); Bar's obligation behavior byte-identical.
- Item 2: a non-debt demand (a favor-fact) drives a full shakedown arc; the debt
  shipped behavior unchanged (existing pins untouched).
- Items 3-6: guard/behavior pins at the new parameter surfaces; the mechanism-level
  ReactionsSpec fixture; deaths/moves grep-proof (`feelingSomeone`, `disapprovalP`
  out of Reactions).

## Out of scope

Non-exposure threats (banked, with the fear-model reason). Sort scoping that models
Call bindings (v47's recorded successor, unforced). The queue ends here; the bank
holds what remains.
