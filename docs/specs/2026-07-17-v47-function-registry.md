# v47 — The function registry: functions get a real home, and Practice loses a fake one

Third of the four audit-queued rounds (v45 protected families → v46 the narrator dies →
**v47** → v48 generality bundle). The audit's HIGH finding C1: `coreLib` is a function
library masquerading as a practice — `Function`s can live ONLY on a `Practice`
(`functions` field → `cpFns` → `lookupCookedFn` folding `cookedDefs`), so the reusable
core-model functions ship as a phantom never-instantiated practice (`"core"`),
registered by every world, occupying the practice namespace, folded over by every
analysis — and already the source of one shipped bug (v43's trailing-dot find was
`cookPractice` choking on exactly this zero-role phantom).

## The probe's sharpening

The tree has exactly TWO function residences: `coreLib`'s pair
(`prax_adjustScore`/`prax_setBond`) and Bar's `tendBar` trio
(`recordDrink`/`checkTipsy`/`checkSober`). And function RESOLUTION is already global —
`lookupCookedFn` searches all practices first-wins, `Call` sites name bare function
names, and v43's collision guard enforces global uniqueness. Practice-locality of
functions is fiction: nothing scopes, nothing shadows, nothing could. So the fix is
not "add a registry beside the field" (two homes — the dual-system ban) but:

## The design

**`Practice` loses its `functions` field entirely; `PraxState` gains the one registry.**

- `defineFunctions :: [Function] -> PraxState -> PraxState` — the setter beside
  `definePractices`; cooks once into a `cookedFns :: Map String ([String], cases)`
  field; enforces the uniqueness guard (v43's cross/within-practice fn-collision arms
  COLLAPSE into this one registry check — a Map can't hold duplicates silently, the
  guard makes the attempt loud).
- `lookupCookedFn` becomes a plain registry lookup — the first-wins fold over
  practices dies, and with it the v41-era footnote about pool-vs-lookup bias (both
  read the same Map now, exactly).
- `Prax.Core.coreLib` DIES. Core exports `coreFns :: [Function]` (the same two,
  unchanged); worlds and `Script.compile` replace `definePractices [coreLib, …]` with
  `defineFunctions coreFns` + the honest practice list. Bar's trio moves to its
  `defineFunctions` call. The phantom `"core"` practice id leaves every world.
- Consumers re-plumb mechanically: `cookPractice` stops cooking functions;
  `cookedFnPool`/`producibleAtoms`/`worldAtomPools`/`bearingTemplates` take the
  Call-resolution pool from the registry; TypeCheck's function walks (unbound-variable
  fn cases, `refErrors`' defined-function set, the DeadCondition and reserved-family
  fn sites, `assertedSentences`' fn inserts) move from per-practice to registry, site
  labels dropping the phantom prefix (`"core / fn …"` → `"fn …"`).

**What becomes unrepresentable:** a function without a registry entry; a phantom
practice existing to carry code. **What this deletes:** a core-type field, a cooked
field, the phantom practice, per-world registration of it, two collision-guard arms,
and the resolution-order subtlety.

## Exactness

Goldens and AnalysisTable pins are expected BYTE-IDENTICAL: the phantom had no
actions, no roles, no instance facts (never spawned), so no pin row names it —
`cookedDefs` losing `"core"` changes no rendered field. Any pin movement = BLOCK and
trace. Persist untouched (functions are build-time vocabulary, like desires). No
format-header bump.

No pre-gate panel for this round: it is a field deletion with two migration sites,
not new engine surface — the audit record and this spec carry the rationale.

## Verification

- RED-first: registry uniqueness guard (duplicate fnName in one `defineFunctions`,
  and across two calls, both loud); a `Call` to an unregistered function still flags
  `UndefinedRef`; the unbound-variable fn walk still catches its cases at the new
  site label.
- The v43 guard pins re-express against the registry (same discriminating scenarios,
  new home); `EngineSpec`/`RelevanceSpec` fn-resolution pins re-point.
- Migration pins: Bar's drinking arithmetic and Core's score/bond behavior unchanged
  (the existing behavioral pins ARE this — they must pass untouched).
- Deaths grep-proof: `coreLib`, `cpFns`, `functions =` (the field), the fn arms of
  `definePractice`'s guard — gone from src/.

## Out of scope

v48 (generality bundle — `disapprovalP`'s placement question is related but queued
there). Any change to `Call`/`Function` semantics, first-match case evaluation, or
the JSON script format (scripts never declared functions).
