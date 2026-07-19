//! Interned path segments. `Arc<Interner>` lives in the engine state,
//! replacing the frozen tree's process-global `unsafePerformIO` pool. Variable-ness
//! is packed into id parity (the kept var-bit trick), so the hottest predicate
//! stays a bit test. Ids never leak into output — all observable text renders
//! through the name lookup (the determinism contract, PLAN.md).
