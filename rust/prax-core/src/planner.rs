//! Utility-driven autonomous choice: bounded lookahead scoring candidate
//! futures by desire satisfaction. The discount order is PINNED (i32 utilities ×
//! f64 0.9/0.5 discounts, bit-exact accumulation order — ordering is the
//! contract, decimal pins die). Carries the v34 prediction reuse and v35
//! standing intentions; ties break in name order.
