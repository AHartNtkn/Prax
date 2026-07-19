//! # prax-core — the engine
//!
//! The runtime of the social simulation, re-expressed from the frozen Haskell
//! library under the DESIGN contract (the five semantic invariants, the specs,
//! the LEDGER), not the Haskell accidents (docs/rewrite/PLAN.md).
//!
//! Load-bearing decisions this crate commits to (PLAN.md, Architecture):
//!
//! - **One compiled representation.** The string-surfaced authoring AST
//!   ([`types`]) is a separate type family from the runtime types converted
//!   only at install by one compile choke point (the retable heir in
//!   [`engine`]). The raw/cooked mirror duality and the twin implementations
//!   die with the port.
//! - **`Arc<Interner>` in state** ([`interner`]) — the global
//!   `unsafePerformIO` intern pool is gone; the var-bit parity trick is kept.
//! - **Persistent exclusion trie** ([`db`]) as `Arc` path-copy nodes with
//!   sorted `SmallVec` children, preserving the planner's apply-and-discard
//!   clone model; corrected `!` semantics and the v39 asserted-flag law.
//! - **Determinism contract:** name-order at every enumeration point.
//!
//! Each module below is the home of one frozen-library concern; they are filled
//! in stage by stage (S1 Sym+Db+EL, S2 Query, S3 Derive+view, …).

pub mod error;
pub mod interner;
pub mod db;
pub mod el;
pub mod query;
pub mod derive;
pub mod types;
pub mod engine;
pub mod turn;
pub mod schedule;
pub mod rng;
pub mod planner;
pub mod relevance;
pub mod sight;
pub mod typecheck;
pub mod persist;
