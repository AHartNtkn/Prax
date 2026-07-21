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

pub mod adjudicated_register;
pub mod analysis_table_spec;
pub mod derive_props;
pub mod engine_deontic;
pub mod engine_replay;
pub mod fixtures;
pub mod golden_drive;
pub mod goldens;
pub mod loop_advance;
pub mod loop_bar;
pub mod meta_gate;
pub mod npc_replay;
pub mod planner_replay;
pub mod schedule_rule_spec;
pub mod script_json_spec;
pub mod script_spec;
pub mod script_supersession;
pub mod schedule_spec;
pub mod source_sweep;
pub mod supersession_world;
pub mod unchecked_split_gate;
pub mod view_invariant;
pub mod village;
pub mod village_relevance;
pub mod witness_templates;
