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

## 2. Blackmail generalizes FULLY: demand, punishment, and motivation are authored

[AMENDED at the gate, twice — the first draft banked the threat side claiming a "fear
model" needed designing; the user rejected the deferral, and the v30 record then
showed the premise was doubly false: the general model was APPROVED IN v30 (LEDGER:
"a threat is a motive-belief deposit … credibility is self-motivation"), and the
implementation narrowed it to its demo's content without the narrowing ever being
surfaced. This item is a FIDELITY RESTORATION to the approved v30 model, not a new
generalization.] v30's fear was never an engine model: it is three existing pieces composing
— the EXTORTER carries an authored punitive desire that makes punishing rational
after defiance; the VICTIM's own wants price the punishment's outcome (eve fears
exposure through her own notoriety want, not through engine audience-semantics); and
the prediction machinery lets the victim foresee the extorter acting. The
believer-counting want was one authored motivation SHAPE that v30 baked into the
mechanism — application content in mechanism clothing, this audit's exact defect
class. Therefore `shakedown` parameterizes on all three application axes:

- **demand** (`[Outcome]` — what compliance does; a favor, a feeling, a fact, or the
  old debt; `Prax.Debt` stops being a hard dependency of leverage),
- **punishment** (label + `[Outcome]` — what the extorter does on defiance; exposure's
  reveal-to-witnesses, a burned barn, anything actionable),
- **the extorter's punitive want** (authored — the motivation that makes the
  punishment credible post-defiance; weight-per-believer for exposure, a plain
  vengeance want for violence).

What stays is the module's IDENTITY: the evidence pattern as the threat's trigger —
leverage means holding something over someone. Evidence-free coercion (a protection
racket) is a DIFFERENT mechanic by charter, not a deferred piece of this one. The
threaten/comply/defy/punish skeleton, the marker plumbing, and the credibility
plumbing (how the victim comes to predict the extorter) are the mechanism and are
untouched. Village AUTHORS the exposure/debt instantiation in the new parameters
(any genuinely mechanical fragment of the old reveal — e.g. a witness-belief
insertion helper — is exported as mechanism if the authored form needs it; the plan
probes this). BlackmailSpec pins BOTH: the v30 exposure/debt arithmetic unchanged
(both compliance regimes), and a full non-debt, non-exposure arc (favor demand,
harm punishment) — the second-application test executed, not asserted.

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
- Item 2: a full non-debt, NON-EXPOSURE arc pinned end-to-end (favor demand, harm
  punishment, authored vengeance want — threaten/comply and threaten/defy/punish both
  driven, the victim's compliance arithmetic observed through its own want over the
  harm); the shipped exposure/debt behavior unchanged (existing pins untouched);
  Village's instantiation authored in the new parameters, byte-identical goldens.
- Items 3-6: guard/behavior pins at the new parameter surfaces; the mechanism-level
  ReactionsSpec fixture; deaths/moves grep-proof (`feelingSomeone`, `disapprovalP`
  out of Reactions).

## Out of scope

Evidence-free coercion (a different mechanic by charter, named in item 2). Sort scoping that models
Call bindings (v47's recorded successor, unforced). The queue ends here; the bank
holds what remains.
