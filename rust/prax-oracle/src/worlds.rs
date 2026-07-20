//! The Rust world registry.
//!
//! One entry per world the Rust side can BUILD. The S7 slices filled it in the
//! design's risk order (feud → intrigue → bar+dm → village); S8 adds the two
//! SCRIPT-compiled worlds. An unknown name is still a LOUD error — a comparator
//! that quietly compares nothing is worse than one that refuses.

use prax_core::engine::State;

/// The worlds S7 drives, in slice order. `probe` is not among them: it is the
/// harness's own fixture (see [`crate::probe`]). `bigfeud` is the scale variant
/// of the feud that S7 design [D-I8] pulls into slice 1 — the frozen
/// `Prax.Worlds.Feud.bigFeud` at the size `Prax.FeudSpec` pins (20), which is
/// what the frozen oracle's own `bigfeud` entry builds.
pub const S7_WORLDS: &[&str] = &["feud", "bigfeud", "intrigue", "bar", "dm", "village"];

/// The S8 worlds: the two `prax_script`-compiled ones. The frozen registry
/// carries both under these names with these idlers (`oracle/TraceMain.hs`'s
/// `worldNamed`), and both are in the frozen `allWorldNames`, so they join the
/// matrix's default sweep exactly as the S7 worlds did.
pub const S8_WORLDS: &[&str] = &["play", "audience"];

/// The CG-1 fixture world's name.
///
/// It is a FIXTURE, not shipped content — like `probe` and `bigfeud` it is
/// absent from the frozen `allWorldNames` — so it is listed separately and
/// [`ported`] adds it explicitly. See [`cg1_world`] for what it demonstrates.
pub const CG1_WORLD: &str = "cg1";

/// The CG-1 script, embedded from the ONE committed file the frozen oracle also
/// reads (`oracle/TraceMain.hs`'s `cg1ScriptPath`) and the conformance fixture
/// also reads. Embedded rather than read at runtime so the world builds without
/// a working-directory assumption.
const CG1_SCRIPT_JSON: &str = include_str!("../../../conformance/fixtures/cg1_supersession.json");

/// The CG-1 fixture world: a play-script, authored entirely through JSON, whose
/// scene setup arms its OWN timer (`insertFor`) and whose one beat bare-inserts
/// the same path.
///
/// This is the counterexample to the S8 design's claim that a bare insert onto a
/// live script timer is "inexpressible in a script". `InsertFor` is in the
/// authored `Outcome` surface, both `setup` and `effects` accept it, and the
/// JSON door spells it directly — so a script can arm a timer and then cancel
/// it. Left alone, the lantern goes out at boundary 3 and the story ends
/// (`darkness`); shielded, the v44 supersession law cancels the pending expiry
/// and the ending never comes. `conformance::script_supersession` pins both
/// halves; this entry is what puts them through the DIFFERENTIAL.
///
/// # Panics
/// If the committed fixture no longer decodes or compiles — which is the loud
/// failure that fixture edit deserves.
pub fn cg1_world() -> State {
    let scr = prax_script::json::decode_script(CG1_SCRIPT_JSON.as_bytes())
        .expect("the committed CG-1 script decodes");
    prax_script::compile::compile(&scr).expect("the committed CG-1 script compiles")
}

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
        "play" => Ok(prax_worlds::play::play_world()),
        "audience" => Ok(prax_worlds::audience::audience_world()),
        CG1_WORLD => Ok(cg1_world()),
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
        // The frozen `worldNamed` pairs each script world with its own player:
        // `play` idles marcus, `audience` idles the envoy. A different idler
        // would make the trace differential a different walk.
        "play" => Some(prax_worlds::play::PLAYER_NAME),
        "audience" => Some(prax_worlds::audience::PLAYER_NAME),
        // the CG-1 fixture's player; `q` is the one who can shield the lantern
        CG1_WORLD => Some("p"),
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
    out.extend(S8_WORLDS);
    out.push(CG1_WORLD);
    out
}
