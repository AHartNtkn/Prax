//! The Rust world registry.
//!
//! One entry per world the Rust side can BUILD. The S7 slices fill it in the
//! design's risk order (feud → intrigue → bar+dm → village); until a slice
//! lands, asking for its world is a LOUD error naming the slice, never a silent
//! skip — a comparator that quietly compares nothing is worse than one that
//! refuses.

use prax_core::engine::State;

/// The worlds S7 drives, in slice order. `probe` is not among them: it is the
/// harness's own fixture (see [`crate::probe`]). `bigfeud` is the scale variant
/// of the feud that S7 design [D-I8] pulls into slice 1 — the frozen
/// `Prax.Worlds.Feud.bigFeud` at the size `Prax.FeudSpec` pins (20), which is
/// what the frozen oracle's own `bigfeud` entry builds.
pub const S7_WORLDS: &[&str] = &["feud", "bigfeud", "intrigue", "bar", "dm", "village"];

/// The size `bigfeud` is driven at, on BOTH sides: `Prax.FeudSpec`'s scale
/// case. A different `n` on either side would be a shape divergence about
/// nothing.
pub const BIG_FEUD_N: usize = 20;

/// The slice that lands each world (for the not-yet-ported message).
fn slice_of(world: &str) -> Option<u8> {
    match world {
        "feud" | "bigfeud" => Some(1),
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
        "feud" => Ok(prax_worlds::feud::feud_world()),
        "bigfeud" => Ok(prax_worlds::feud::big_feud(BIG_FEUD_N)),
        "intrigue" => Ok(prax_worlds::intrigue::intrigue_world()),
        "bar" => Ok(prax_worlds::bar::bar_world()),
        "dm" => Ok(prax_worlds::bar::bar_director_world()),
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
        "feud" | "bigfeud" => Some(prax_worlds::feud::PLAYER_NAME),
        // NOT `intrigue`: the frozen `GoldenDriveSpec` drives it with
        // `driveLabels 12 Nothing`, so marcus is planner-driven like everyone
        // else. An idler here would make the trace differential a different walk
        // from the golden it has to agree with.
        //
        // The frozen `worldOf` (oracle/TraceMain.hs:83-84) pairs each bar world
        // with its own player: `bar` idles `Bar.playerName`, and `dm` idles
        // `Bar.directorName` — the DM is the human seat there, so idling `you`
        // (who does not exist in `dm`) would be a different walk.
        "bar" => Some(prax_worlds::bar::PLAYER_NAME),
        "dm" => Some(prax_worlds::bar::DIRECTOR_NAME),
        "village" => Some("you"),
        _ => None,
    }
}

/// Every world the Rust side can currently build.
pub fn ported() -> Vec<&'static str> {
    vec!["probe", "feud", "bigfeud", "intrigue", "bar", "dm"]
}
