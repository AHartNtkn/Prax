# KILLED pins — the meta-gate's accounted-for-but-not-re-expressed ledger

Each row is a Haskell test label (`testCase`/`testGroup`) that is NOT
re-expressed as a Rust `// H:` pin, with a category and a one-line reason. The
meta-gate (`rust/conformance/src/meta_gate.rs`) asserts every allowlisted
`HASKELL_PINS.txt` label is accounted for exactly once — either here or under a
single `// H:` comment in `rust/`.

Categories: `decimal` (a pin on exact decimal output whose contract is now the
accumulation ORDER, not the decimals) · `implementation` (a pin on a
Haskell-implementation shape the Rust design does not have) · `haskell-only` (a
pin on a hazard that cannot exist in the Rust design) · `deferral` (a pin whose
subject is real and WILL be re-expressed, but at a later stage that owns the
surface it tests; each such row names the owing stage in a loud sentence — the
stage is not done while its deferral rows stand).

The `Owed` column carries that obligation MACHINE-READABLY: a row with `owed:
S<N>` MUST be re-expressed as a `// H:` pin — and this row REMOVED (the
exactly-once rule forces removal on re-expression) — by the time stage S<N> is
marked DONE on the PROGRAM.md status board. The meta-gate reads both and FAILS if
any owing stage is DONE with its row still standing (`meta_gate.rs`). A `—` owes
nothing. A row may owe even when its category is not `deferral` (a grounding pin
whose subject is born in a later stage). Format: `owed: S<N>` or `—`.

Format (one row per killed pin):

| SpecFile | Label | Category | Owed | Reason |
|----------|-------|----------|------|--------|
| SymSpec.hs | symName forces its argument before touching the pool (fresh, unforced Sym thunk) | haskell-only | — | Guards a lazy-thunk vs `unsafePerformIO` pool-write ordering race specific to Haskell's global `IORef` interner. The Rust interner is an owned, append-only value and evaluation is strict, so the entire class of bug is absent by construction — there is no thunk to force and no pool write to race. |
| CookedSpec.hs | queryCooked equals the string evaluator on every fixture case | implementation | — | The raw/cooked dual-path duality is a Haskell accident; the Rust design has ONE representation and ONE evaluator (ARCHITECTURE.md), so there is no second evaluator to agree with. The equivalence's underlying content — the seven exclusion-bearing fixture scenarios — is re-expressed as direct result pins on the one evaluator (`cooked_fixture_scenarios_on_the_one_evaluator`, H: CookedSpec.hs "Prax.Cooked"). Duality dies; coverage does not. |
| CookedSpec.hs | grounding cooked matches grounding strings (incl. '!' outcomes) | implementation | — | Same duality death: grounding runs once over one representation, so there is no cooked-vs-string pair to compare. Path grounding's `!`/`.` preservation is pinned by the DbSpec `ground` pins and by QuerySpec `groundCondition substitutes bindings through every constructor`. |
| CookedSpec.hs | groundCookedCondition matches groundCondition for every remaining construct | implementation | — | Same duality death: there is no cooked mirror of `groundCondition`. Grounding over every Condition constructor is re-expressed directly in QuerySpec `groundCondition substitutes bindings through every constructor`. |
| EngineSpec.hs | groundedDeltaAnchors: bounded effects, shadows, spawn opacity, Call resolution | deferral | owed: S6 | Tests `groundedDeltaAnchors`, the planner's prediction-reuse anchor walk (v34 reuse); its sole consumer lands at S6, so the walk and its `safeBinders` helper are deferred there (design §3). **DEFERRAL — S6 MUST land the groundedDeltaAnchors bounded-effects/shadows/spawn-opacity/Call-resolution pin. S6 is not done while this sentence stands.** |
| EngineSpec.hs | groundedDeltaAnchors: safe ForEach binders bound; unsafe heads stay opaque | deferral | owed: S6 | Tests `groundedDeltaAnchors`/`safeBinders`, the planner's prediction-reuse anchor walk (v34 reuse) — sole consumer at S6 (design §3). **DEFERRAL — S6 MUST land the safe-ForEach-binder/opaque-head pin. S6 is not done while this sentence stands.** |
| EngineSpec.hs | build-order death: setAxioms-first equals setAxioms-outermost (cookedRules and typeCheck) | deferral | owed: S9 | The label's `typeCheck`-equality clause is only expressible once `Prax.TypeCheck` exists (S9); consuming the label at S4 would silently drop that clause (the meta-gate accounts each label once, D-panel I3). S4 lands an INDEPENDENT compiled-rule-equality regression test that consumes no Haskell label (`set_axioms_order_independent_cooked_rules`). **DEFERRAL — S9 MUST re-express this label with the typeCheck-equality clause. S9 is not done while this sentence stands.** |
| GateSpec.hs | the scanner (mutation evidence: it must actually discriminate) | deferral | owed: S9 | The world-source string-literal scanner half of GateSpec reads `src/Prax/Worlds/*.hs`; it is retargeted at the `.rs` world sources at S9 (the GateSpec scanner half, design §5). The shared-guard half (`authoredVarClash` through `draw`) is re-expressed at S4. **DEFERRAL — S9 MUST land the retargeted-at-.rs scanner pin. S9 is not done while this sentence stands.** |
| GateSpec.hs | catches a Prax-namespaced token inside a quoted literal | deferral | owed: S9 | GateSpec scanner half, retargeted at the `.rs` worlds at S9 (design §5). **DEFERRAL — S9 owes the retargeted scanner pin. S9 is not done while this sentence stands.** |
| GateSpec.hs | catches more than one offender, in order | deferral | owed: S9 | GateSpec scanner half, retargeted at the `.rs` worlds at S9 (design §5). **DEFERRAL — S9 owes the retargeted scanner pin. S9 is not done while this sentence stands.** |
| GateSpec.hs | ignores ordinary quoted literals with no Prax-shaped token | deferral | owed: S9 | GateSpec scanner half, retargeted at the `.rs` worlds at S9 (design §5). **DEFERRAL — S9 owes the retargeted scanner pin. S9 is not done while this sentence stands.** |
| GateSpec.hs | ignores unquoted text (imports, comments) even if Prax-shaped | deferral | owed: S9 | GateSpec scanner half, retargeted at the `.rs` worlds at S9 (design §5). **DEFERRAL — S9 owes the retargeted scanner pin. S9 is not done while this sentence stands.** |
| GateSpec.hs | no world source file authors a Prax-namespaced variable in a quoted literal | deferral | owed: S9 | GateSpec scanner half: the durable world-source gate, retargeted from `src/Prax/Worlds/*.hs` to the `.rs` worlds at S9. **DEFERRAL — S9 owes the retargeted world-source gate. S9 is not done while this sentence stands.** |
