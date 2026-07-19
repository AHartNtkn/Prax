//! # prax-oracle — the differential comparator
//!
//! The Rust side of the differential net (PLAN.md, Verification). It runs the
//! Rust engine to produce the same canonical trace/randtrace records the frozen
//! Haskell `prax-oracle` emits, then compares them record by record. On a
//! divergence it auto-reruns with candidate lists and classifies it
//! (ENUMERATION | DECISION | STATE | SCHEDULE | RNG) with a fact-level path
//! diff, consulting the adjudicated-divergences register so the matrix reads
//! "clean modulo adjudicated fixes" and fresh signal is never drowned.
//!
//! Matrix mode emits one line per (world, seed): clean | clean-mod-adjudicated |
//! DIVERGENT. Built once the Rust engine can produce traces (S1+); inert now.

fn main() {}
