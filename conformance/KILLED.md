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
| GateSpec.hs | the scanner (mutation evidence: it must actually discriminate) | deferral | owed: S9 | The world-source string-literal scanner half of GateSpec reads `src/Prax/Worlds/*.hs`; it is retargeted at the `.rs` world sources at S9 (the GateSpec scanner half, design §5). The shared-guard half (`authoredVarClash` through `draw`) is re-expressed at S4. **DEFERRAL — S9 MUST land the retargeted-at-.rs scanner pin. S9 is not done while this sentence stands.** |
| GateSpec.hs | catches a Prax-namespaced token inside a quoted literal | deferral | owed: S9 | GateSpec scanner half, retargeted at the `.rs` worlds at S9 (design §5). **DEFERRAL — S9 owes the retargeted scanner pin. S9 is not done while this sentence stands.** |
| GateSpec.hs | catches more than one offender, in order | deferral | owed: S9 | GateSpec scanner half, retargeted at the `.rs` worlds at S9 (design §5). **DEFERRAL — S9 owes the retargeted scanner pin. S9 is not done while this sentence stands.** |
| GateSpec.hs | ignores ordinary quoted literals with no Prax-shaped token | deferral | owed: S9 | GateSpec scanner half, retargeted at the `.rs` worlds at S9 (design §5). **DEFERRAL — S9 owes the retargeted scanner pin. S9 is not done while this sentence stands.** |
| GateSpec.hs | ignores unquoted text (imports, comments) even if Prax-shaped | deferral | owed: S9 | GateSpec scanner half, retargeted at the `.rs` worlds at S9 (design §5). **DEFERRAL — S9 owes the retargeted scanner pin. S9 is not done while this sentence stands.** |
| GateSpec.hs | no world source file authors a Prax-namespaced variable in a quoted literal | deferral | owed: S9 | GateSpec scanner half: the durable world-source gate, retargeted from `src/Prax/Worlds/*.hs` to the `.rs` worlds at S9. **DEFERRAL — S9 owes the retargeted world-source gate. S9 is not done while this sentence stands.** |
| CoerceSpec.hs | v54: mid-racket save/resume (home: CoerceSpec — it exercises the racket's own cycle) | deferral | owed: S9 | Serializes a racket state carrying a PENDING complied-expiry due, reloads it and asserts the cycle resumes on schedule. `Prax.Persist` has no Rust twin until S9 (`rust/prax-core/src/persist.rs` is a doc header only; PROGRAM.md's S9 row: TypeCheck + AnalysisTable + Stress + **Persist** + Inspect + CLI). The racket cycle ITSELF -- the expiring `complied` marker, the blocked re-buy inside the bought period, the re-arm after expiry -- is pinned at S7 without persistence by `the_racket_cycles_under_an_expiring_complied_marker`; what is deferred is the SAVE/RESUME surface. **DEFERRAL -- S9 MUST land the mid-racket save/resume pin. S9 is not done while this sentence stands.** |
| CoerceSpec.hs | a save carrying the pending complied-expiry reloads and the cycle resumes on schedule | deferral | owed: S9 | The case under the group above: `deserializeState (serializeState st)` over a state whose `expiries` map is non-empty. `Prax.Persist` is S9. **DEFERRAL -- S9 owes the pending-expiry round-trip pin. S9 is not done while this sentence stands.** |
| ScriptSpec.hs | mid-scene save/resume reaches the same timeout boundary (persistence symmetry) | deferral | owed: S9 | Serializes the `audience` world at boundary 2 — mid-scene, with the timed junction's patience marker armed and its expiry PENDING — reloads it, and asserts the resumed world still reaches `dismissed` at the SAME absolute boundary 5. The claim is that a patience marker is an ordinary fact whose pending expiry rides v44's due serialization, so a save partway through a timed scene needs no Persist code of its own. `Prax.Persist` has no Rust twin until S9 (`rust/prax-core/src/persist.rs` is a doc header only; PROGRAM.md's S9 row: TypeCheck + AnalysisTable + Stress + **Persist** + Inspect + CLI). The TIMING behaviour itself -- entry at boundary 0, timeout 5, `dismissed` at boundary 5 and not before -- is pinned at S8 WITHOUT persistence by `conformance::script_spec::the_audience_dismissed_fires_at_boundary_5_not_before`, and the non-empty initial expiry map it rides on by `the_audience_carries_a_pending_expiry_before_any_turn_runs`; what is deferred is the SAVE/RESUME surface. **DEFERRAL — S9 MUST land the mid-scene save/resume pin over a script world's armed patience marker. S9 is not done while this sentence stands.** |
