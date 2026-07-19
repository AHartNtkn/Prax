//! The meta-gate: parse `conformance/HASKELL_PINS.txt` and assert each of the
//! ~849 labels is accounted for exactly once — either re-expressed (a `// H:`
//! comment on a Rust test) or explicitly killed (a `KILLED.md` row with a
//! category and reason). Enforced as a test once the corpus starts filling.
