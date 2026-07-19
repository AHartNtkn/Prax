//! The exclusion-logic database: a trie of interned segments whose edges are
//! `.` (multi) or `!` (exclusion). Represented as `Arc` path-copy persistent
//! nodes with sorted `SmallVec` children, so the planner's apply-and-discard
//! clone model is a cheap structural share. Carries the corrected `!` semantics
//! (siblings cleared, the surviving child's subtree preserved) and the v39
//! asserted-flag law (no unasserted childless node survives).
