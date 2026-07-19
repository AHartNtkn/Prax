//! # prax-script — the Prompter layer
//!
//! Versu's Prompter: authoring a story as scenes/acts with per-scene casts and
//! beats, compiled to engine schedule rules (the `story` rule) through the
//! compiler-level registration door (PLAN.md). The compiled scene machinery
//! writes reserved families (`currentScene`, `scenePatience`) that authored code
//! may not touch — provenance-exempted at type-check.
//!
//! [`compile`] lowers an authored [`script`] to runtime schedule rules;
//! [`json`] is the serde round-trip (`examples/play.json` must load unchanged
//! across the cut-over).

pub mod script;
pub mod compile;
pub mod json;
