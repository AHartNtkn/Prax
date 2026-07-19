//! Save/resume: a serde format for the runtime state (no cross-engine save
//! compatibility with the Haskell — a clean break, PLAN.md). Runtime-only fields
//! (cursor, intentions, dues, expiries, rng position) round-trip by name.
