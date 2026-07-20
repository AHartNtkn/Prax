//! The royal audience: flatter the king then petition, before the moment (or the
//! Duke) passes — a short play-script exercising two Prompter compilation
//! features in one story.
//!
//! A **character sketch**: the ambitious Duke, whose *concern* for royal favour
//! compiles to a desire and makes him flatter the king unbidden. And a **timed
//! junction**: the audience ends of its own accord if you dawdle.
//!
//! You are the envoy. Flatter the king to win a little favour, then present your
//! petition while you still have it — do it and the petition is `granted`;
//! dither and the king's patience runs out (`dismissed`).
//!
//! This is the FIRST shipped world whose state carries a non-empty expiry map at
//! t=0: `timeout "dismissed" 5` arms a patience marker during the compile-time
//! setup fold, so `worldshape audience`'s `state.expiries` is where the timing
//! contract is compared, not `setup_db` (which sees the marker FACT but not its
//! due).
//!
//! Frozen reference: `src/Prax/Worlds/Audience.hs`.

use prax_core::engine::State;
use prax_core::query::{matches, not_};
use prax_core::types::insert;
use prax_script::compile::compile;
use prax_script::script::{
    Scene, Script, concerned_with, ending, member, player, quip, scene, timeout, with_traits,
};
use prax_vocab::core_model::{adjust_score, score_at_least};

/// You are the envoy.
pub const PLAYER_NAME: &str = "envoy";

/// The relationship dimension the court trades in.
const FAVOR: &str = "favor";

pub fn audience_script() -> Script {
    Script::new("audience")
        .cast([
            player(PLAYER_NAME),
            // the sketch: the Duke is defined by what he is concerned with
            // (standing at court) and who he is (ambitious); the concern
            // compiles to a desire, the trait to a queryable fact.
            with_traits(
                concerned_with(member("duke"), [(FAVOR, 10)]),
                ["ambitious"],
            ),
            member("king"),
        ])
        .scenes([audience()])
}

fn audience() -> Scene {
    scene("audience")
        .opening(
            "The throne room. You hold the king's ear — but not for long, and the Duke is circling.",
        )
        .setup([insert("atCourt")])
        .beats([
            quip(
                "envoy",
                "[Actor]: flatter the king",
                vec![not_("petitioned")],
                vec![adjust_score("king", "envoy", FAVOR, 5, "flattery")],
            ),
            quip(
                "envoy",
                "[Actor]: present your petition",
                {
                    let mut w = score_at_least("king", "envoy", FAVOR, 5);
                    w.push(not_("petitioned"));
                    w
                },
                vec![insert("petitioned")],
            ),
            // the Duke needs no wants of his own here — his *concern* for favour
            // drives him to court the king unbidden (one telling gesture)
            quip(
                "duke",
                "[Actor]: flatter the king",
                vec![not_("dukeSpoke"), not_("petitioned")],
                vec![
                    insert("dukeSpoke"),
                    adjust_score("king", "duke", FAVOR, 5, "flattery"),
                ],
            ),
        ])
        .junctions([
            // you pressed your case in time
            ending("granted", vec![matches("petitioned")]),
            // …or the king's patience ran out
            timeout("dismissed", 5),
        ])
}

/// The compiled, ready-to-run audience.
///
/// # Panics
/// If a compile guard rejects the script above — a bug in this file, not a
/// condition a world can handle.
pub fn audience_world() -> State {
    compile(&audience_script()).expect("the audience script compiles")
}
