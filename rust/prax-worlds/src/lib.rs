//! # prax-worlds — the shipped storyworlds
//!
//! The six worlds, each composed from [`prax_vocab`] combinators over the
//! [`prax_core`] engine (one built from [`prax_script`]). The S7 vertical slices
//! land them in this order: feud → audience → intrigue → bar → village, each
//! with differential testing (trace + randtrace) switched on as it arrives.
//!
//! `bar` carries both the plain night and the drama-manager (`dm`) variant.

pub mod bar;
pub mod intrigue;
pub mod play;
pub mod feud;
pub mod audience;
pub mod village;
