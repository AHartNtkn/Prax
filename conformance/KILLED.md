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
| CookedSpec.hs | groundCookedOutcome matches groundOutcome for every remaining construct | implementation | owed: S4 | Same duality death: there is no cooked mirror of `groundOutcome`. Outcome grounding is a Types/Engine concern (S4, where `Outcome` is born) and is re-expressed there against the one representation. **DEFERRAL — S4 MUST land a grounding pin covering this row's constructs; the S4 stage is not done while this sentence stands** (S2 review M3: the deferral is prose, so it is made loud here). |
| DeriveSpec.hs | obligedClose: a domain rule (written once) also closes under obligation | deferral | owed: S4 | Tests `Prax.Deontic.obligedClose` (the □-closure operator lives in vocab/deontic, S4), not the S3 closure surface — S3's `close`/`close_from` treat any □-lifted rules as ordinary rules (design §5, no lifting in derive). **DEFERRAL — S4 MUST land an obligedClose pin: closing over the expanded axiom list derives the sub-obligation, and bare closure does NOT lift. S4 is not done while this sentence stands.** |
| DeriveSpec.hs | axiomFootprint collects bodies (any polarity) and heads; obligedClose adds the lifted forms | deferral | owed: S4 | Tests `axiomFootprint`, an analysis-table builder consumed by `Engine.retable` (the S4 `compilepipe`), not the closure loop. Panel design I1: correctly out of S3. **DEFERRAL — S4 MUST land an axiomFootprint pin (body atoms at any polarity + heads; obligedClose contributes the lifted forms). S4 is not done while this sentence stands.** |
| DeriveSpec.hs | axiomNegPatterns collects exactly the negated interiors | deferral | owed: S4 | Tests `axiomNegPatterns`, an S4 analysis-table builder (the neg-footprint feeding the engine's continuation-tier router), not the closure loop. **DEFERRAL — S4 MUST land an axiomNegPatterns pin (exactly the interiors under a negation: `Absent`/`Not` contents, positive atoms excluded). S4 is not done while this sentence stands.** |
| DeriveSpec.hs | monotoneAxioms accepts the count-threshold shape and rejects anti-monotone | deferral | owed: S4 | Tests `monotoneAxioms`, the S4 analysis-table builder gating the engine's monotone continuation tier (`close_from`'s caller precondition, design §3), not the closure loop itself. **DEFERRAL — S4 MUST land a monotoneAxioms pin (Count+Cmp-Gte-literal safe; Cmp-Lt/anti-monotone, Calc, and Eq/Neq over an aggregate-bound var rejected). S4 is not done while this sentence stands.** |
