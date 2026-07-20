//! The Rust world registry.
//!
//! One entry per world the Rust side can BUILD. The S7 slices filled it in the
//! design's risk order (feud → intrigue → bar+dm → village); with slice 4 landed,
//! EVERY S7 world builds, so the not-yet-ported arm (and the slice table behind
//! it) is gone rather than left as unreachable scaffolding. An unknown name is
//! still a LOUD error — a comparator that quietly compares nothing is worse than
//! one that refuses — and S8's script worlds will add their own entries.

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

/// Build a world by name, or say precisely why not.
pub fn build(name: &str) -> Result<State, String> {
    match name {
        "probe" => Ok(crate::probe::probe_world()),
        "feud" => Ok(prax_worlds::feud::feud_world()),
        "bigfeud" => Ok(prax_worlds::feud::big_feud(BIG_FEUD_N)),
        "intrigue" => Ok(prax_worlds::intrigue::intrigue_world()),
        "bar" => Ok(prax_worlds::bar::bar_world()),
        "dm" => Ok(prax_worlds::bar::bar_director_world()),
        "village" => Ok(prax_worlds::village::village_world()),
        other => Err(format!(
            "unknown world `{other}` (ported: {})",
            ported().join(" ")
        )),
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
        "village" => Some(prax_worlds::village::PLAYER_NAME),
        _ => None,
    }
}

/// Every world the Rust side can currently build: the harness's own fixture plus
/// every S7 world. Derived from [`S7_WORLDS`] rather than restated, so the list
/// the CLI prints and the list `matrix` sweeps cannot drift apart from the list
/// [`build`] answers to.
pub fn ported() -> Vec<&'static str> {
    let mut out = vec!["probe"];
    out.extend(S7_WORLDS);
    out
}
