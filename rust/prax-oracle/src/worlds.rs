//! The Rust world registry.
//!
//! One entry per world the Rust side can BUILD. The S7 slices fill it in the
//! design's risk order (feud → intrigue → bar+dm → village); until a slice
//! lands, asking for its world is a LOUD error naming the slice, never a silent
//! skip — a comparator that quietly compares nothing is worse than one that
//! refuses.

use prax_core::engine::State;

/// The shipped worlds S7 drives, in slice order. `probe` is not among them: it
/// is the harness's own fixture (see [`crate::probe`]).
pub const S7_WORLDS: &[&str] = &["feud", "intrigue", "bar", "dm", "village"];

/// The slice that lands each world (for the not-yet-ported message).
fn slice_of(world: &str) -> Option<u8> {
    match world {
        "feud" => Some(1),
        "intrigue" => Some(2),
        "bar" | "dm" => Some(3),
        "village" => Some(4),
        _ => None,
    }
}

/// Build a world by name, or say precisely why not.
pub fn build(name: &str) -> Result<State, String> {
    match name {
        "probe" => Ok(crate::probe::probe_world()),
        other => match slice_of(other) {
            Some(slice) => Err(format!(
                "world `{other}` is not ported to Rust yet — it lands in S7 slice {slice}. \
                 The frozen side can still be driven (`prax-oracle worldshape {other}` via the \
                 cabal oracle), but there is nothing to compare it against."
            )),
            None => Err(format!(
                "unknown world `{other}` (ported: probe; planned: {})",
                S7_WORLDS.join(" ")
            )),
        },
    }
}

/// The world's default idler for the trace walk (the `driveLabels` player).
pub fn idler(name: &str) -> Option<&'static str> {
    match name {
        "probe" => Some(crate::probe::PROBE_IDLER),
        "village" => Some("you"),
        _ => None,
    }
}

/// Every world the Rust side can currently build.
pub fn ported() -> Vec<&'static str> {
    vec!["probe"]
}
