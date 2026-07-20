//! # conformance — the mechanical fidelity gate
//!
//! Where semantic fidelity is proven, not asserted (PLAN.md). Two cross-cutting
//! pieces exist from S0; the per-Haskell-spec-file test modules are added as
//! each stage lands (one file per frozen spec file, each re-expressed test
//! carrying a `// H: <SpecFile> "<label>"` provenance comment).
//!
//! - [`fixtures`] — replays the committed `conformance/fixtures/{db,el,query,
//!   derive}.json` corpora against the Rust engine (the pre-world stages' ground
//!   truth).
//! - [`meta_gate`] — parses the committed `conformance/HASKELL_PINS.txt`
//!   manifest and asserts every label appears in exactly one `// H:` comment OR
//!   one `KILLED.md` row. Pin accounting is a red/green test, not a claim.

pub mod derive_props;
pub mod engine_deontic;
pub mod engine_replay;
pub mod fixtures;
pub mod meta_gate;
pub mod view_invariant;
