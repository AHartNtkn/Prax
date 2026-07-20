# KILLED pins — the meta-gate's accounted-for-but-not-re-expressed ledger

Each row is a Haskell test label (`testCase`/`testGroup`) that is NOT
re-expressed as a Rust `// H:` pin, with a category and a one-line reason. The
meta-gate (`rust/conformance/src/meta_gate.rs`) asserts every allowlisted
`HASKELL_PINS.txt` label is accounted for exactly once — either here or under a
single `// H:` comment in `rust/`.

Categories: `decimal` (a pin on exact decimal output whose contract is now the
accumulation ORDER, not the decimals) · `implementation` (a pin on a
Haskell-implementation shape the Rust design does not have) · `haskell-only` (a
pin on a hazard that cannot exist in the Rust design).

Format (one row per killed pin):

| SpecFile | Label | Category | Reason |
|----------|-------|----------|--------|
| SymSpec.hs | symName forces its argument before touching the pool (fresh, unforced Sym thunk) | haskell-only | Guards a lazy-thunk vs `unsafePerformIO` pool-write ordering race specific to Haskell's global `IORef` interner. The Rust interner is an owned, append-only value and evaluation is strict, so the entire class of bug is absent by construction — there is no thunk to force and no pool write to race. |
