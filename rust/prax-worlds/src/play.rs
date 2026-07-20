//! The conspiracy as a play-script: the same drama as [`crate::intrigue`],
//! authored through [`prax_script`]'s scene/beat/junction surface instead of
//! hand-coded practices, and split across two scenes so it exercises a scene
//! TRANSITION and the auto flow-chart.
//!
//! Act I, *the confidence*: Cassia confides her plot to Marcus (the player).
//! Once she has, the story flows to Act II, *the banquet*, where — unless Marcus
//! warns Artus or strikes first himself — Cassia poisons the patron. Three
//! endings: betrayal (Cassia's poison), loyalty (Marcus warns), complicity
//! (Marcus's hand); Marcus may also warm to Cassia along the way, a romance
//! orthogonal to the killing.
//!
//! The compiled `story` schedule rule advances the scene and fires the ending
//! silently at a round boundary, the moment a junction's condition holds. There
//! is no story manager and no actor for it.
//!
//! Frozen reference: `src/Prax/Worlds/Play.hs`. `examples/play.json` is this
//! script's own JSON re-emission, byte for byte.

use prax_core::engine::State;
use prax_core::query::{absent, matches, not_};
use prax_core::types::{Want, dead_sentence, insert};
use prax_script::compile::compile;
use prax_script::script::{Scene, Script, ending, goto, member, player, quip, scene, wanting};
use prax_vocab::core_model::{WARMTH, adjust_score, set_bond};
use prax_vocab::emotion::{PLEASED, feel_toward};

/// The player is Marcus, the poet. A plain literal, as the frozen `playerName`
/// is: `prax_script::script::script_player` exists for the S9 CLI's file-loading
/// arm, not for a world that already knows its own cast.
pub const PLAYER_NAME: &str = "marcus";

/// The whole episode as a play-script.
pub fn play_script() -> Script {
    Script::new("confidence")
        .cast([
            player(PLAYER_NAME),
            member("artus"),
            wanting(
                member("cassia"),
                [Want::new(vec![matches(dead_sentence("artus"))], 100)],
            ),
        ])
        .scenes([confidence(), banquet()])
}

/// Act I — Cassia lets Marcus in on the plot; that opens the way to the banquet.
fn confidence() -> Scene {
    scene("confidence")
        .opening("A quiet portico. Cassia draws Marcus aside.")
        .beats([quip(
            "cassia",
            "[Actor]: confide the plot against artus to marcus",
            vec![not_("confided")],
            vec![
                insert("confided"),
                insert("marcusKnows"),
                adjust_score("marcus", "cassia", WARMTH, 5, "sharedASecret"),
            ],
        )])
        .junctions([goto("toBanquet", "banquet", vec![matches("confided")])])
}

/// Act II — the poisoning plays out; whoever acts first fixes the ending.
fn banquet() -> Scene {
    scene("banquet")
        .opening("The banquet hall. Artus reclines, oblivious, wine in hand.")
        .beats([
            quip(
                "cassia",
                "[Actor]: slip poison into artus's cup",
                vec![not_(dead_sentence("artus")), absent(vec![matches("foiled")])],
                vec![
                    insert("poisoned.artus.byCassia"),
                    insert(dead_sentence("artus")),
                ],
            ),
            quip(
                "marcus",
                "[Actor]: warn artus that cassia means to kill him",
                vec![
                    matches("marcusKnows"),
                    not_(dead_sentence("artus")),
                    absent(vec![matches("foiled")]),
                ],
                vec![
                    insert("foiled"),
                    adjust_score("artus", "marcus", WARMTH, 30, "savedMyLife"),
                    feel_toward("artus", PLEASED, "marcus"),
                ],
            ),
            quip(
                "marcus",
                "[Actor]: poison artus with your own hand",
                vec![
                    matches("marcusKnows"),
                    not_(dead_sentence("artus")),
                    absent(vec![matches("foiled")]),
                ],
                vec![
                    insert("poisoned.artus.byMarcus"),
                    insert(dead_sentence("artus")),
                ],
            ),
            // Romance: warm to the conspirator you now share a secret with —
            // orthogonal to the killing; it neither foils nor causes it.
            quip(
                "marcus",
                "[Actor]: warm to cassia's charms",
                vec![matches("marcusKnows"), not_("bond.marcus.cassia!lovers")],
                vec![
                    set_bond("marcus", "cassia", "lovers"),
                    adjust_score("marcus", "cassia", WARMTH, 15, "sweptUp"),
                    feel_toward("marcus", PLEASED, "cassia"),
                ],
            ),
        ])
        .junctions([
            ending("betrayal", vec![matches("poisoned.artus.byCassia")]),
            ending("loyalty", vec![matches("foiled")]),
            ending("complicity", vec![matches("poisoned.artus.byMarcus")]),
        ])
}

/// The compiled, ready-to-run world.
///
/// # Panics
/// If a compile guard rejects the script above — a bug in this file, not a
/// condition a world can handle.
pub fn play_world() -> State {
    compile(&play_script()).expect("the play script compiles")
}
