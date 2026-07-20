//! # prax-vocab — the durable social vocabulary
//!
//! The content combinators: pure value-builders that assemble practices,
//! axioms, and desires a world composes. This is the layer with the longest
//! half-life (PLAN.md) — the social primitives (emotion, debt, rumor, factions,
//! …) outlive any one world. Nothing here runs; everything here BUILDS the
//! authoring-AST values [`prax_core::types`] compiles at install.
//!
//! One module per combinator family, mirroring the frozen library's content
//! modules. Engine-side analyses (relevance, sight, schedule) live in
//! [`prax_core`], not here — this crate is values only.

pub mod arc;
pub mod beliefs;
pub mod blackmail;
pub mod coerce;
pub mod confession;
pub mod conversation;
pub mod core_model;
pub mod debt;
pub mod deceit;
pub mod deontic;
pub mod emotion;
pub mod faction;
pub mod kin;
pub mod minds;
pub mod persona;
pub mod project;
pub mod reactions;
pub mod repute;
pub mod rumor;
pub mod witness;
