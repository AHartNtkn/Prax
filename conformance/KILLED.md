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
