//! The engine schedule (v44): recurring rules fired every N round boundaries in
//! declaration order, and the one-shot expiry queue (a fact with lifetime n is
//! present rounds onset..onset+n-1 and gone at the boundary; expiries fire
//! before rules). Two registration doors: authored and compiler-level.
