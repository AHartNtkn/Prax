//! # prax-worlds — the shipped storyworlds
//!
//! The seven worlds, each composed from [`prax_vocab`] combinators over the
//! [`prax_core`] engine (one built from [`prax_script`]). The six S7/S8 worlds
//! land via the vertical slices — feud → audience → intrigue → bar → village —
//! each with differential testing (trace + randtrace) switched on as it arrives.
//! `vampire` is new Phase-1 content with no frozen counterpart, so it ships
//! single-engine, deliberately off the differential matrix.
//!
//! `bar` carries both the plain night and the drama-manager (`dm`) variant.

pub mod audience;
pub mod bar;
pub mod feud;
pub mod intrigue;
pub mod play;
pub mod vampire;
pub mod village;
