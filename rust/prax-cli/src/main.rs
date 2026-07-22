//! # prax — the command-line player (`app/Main.hs`'s heir)
//!
//! Round-robin turns, an autonomous cast, a numbered action menu on the player's
//! turn, and save/resume — plus the non-interactive subcommands `stress`,
//! `check`, `flow`, and `dump-play`. World selection mirrors the frozen CLI (bar
//! default; intrigue/play/dm/feud/audience/village by name).
//!
//! **What S9 pins vs what S10 inherits ([R11]).** The non-interactive
//! subcommands are fully realized here with committed stdout equality
//! (`check`/`stress`/`flow`/`dump-play`, each also carrying a structural durable
//! net: the oracle differential for check/stress, the S8 native pin for flow,
//! `examples/play.json` byte-identity for dump-play). The TESTABLE pieces of
//! `play` are pinned natively: [`player_actions`] (no-op-hidden + bound-player
//! restriction) and the resume path (Persist round-trip replays the save-point
//! advance). The interactive menu's stdin loop and [`render_scene`]'s cosmetic
//! assembly are DEMO-verified, human-in-the-loop, and belong to the S10 demo —
//! they are not conformance-pinned (and not silently dropped: they are the
//! demo's surface, implemented here for real, just not pinned).

use std::io::{Write, stdin, stdout};

use prax_core::engine::{GroundedAction, State};
use prax_core::persist::deserialize_state;
use prax_core::stress::{StressReport, stress_test};
use prax_core::turn::{advance, npc_act};
use prax_core::typecheck::{CURRENT_SCENE_PATH, describe, type_check};
use prax_core::types::Character;
use prax_script::compile::{compile, flow_chart};
use prax_script::json::{decode_script, encode_script};
use prax_script::script::script_player;
use prax_worlds::{audience, bar, feud, intrigue, play, vampire, village};

/// Where an in-game save is written / resumed from.
const SAVE_FILE: &str = "prax.save";
/// The NPC lookahead depth the player-facing loop drives the cast at.
const LOOKAHEAD_DEPTH: i32 = 2;

/// The world a CLI arg selects: display title, the built world, and the player's
/// name (the frozen `worldNamed`).
fn world_named(args: &[String]) -> (&'static str, State, &'static str) {
    match args.first().map(String::as_str) {
        Some("intrigue") => ("Intrigue (Rome)", intrigue::intrigue_world(), intrigue::PLAYER_NAME),
        Some("play") => ("the conspiracy (a play)", play::play_world(), play::PLAYER_NAME),
        Some("dm") => ("the bar, and you direct it", bar::bar_director_world(), bar::DIRECTOR_NAME),
        Some("feud") => ("the feud (emergent sandbox)", feud::feud_world(), feud::PLAYER_NAME),
        Some("audience") => ("the royal audience", audience::audience_world(), audience::PLAYER_NAME),
        Some("village") => ("the village", village::village_world(), village::PLAYER_NAME),
        Some("vampire") => ("the vampire village", vampire::vampire_world(), vampire::PLAYER_NAME),
        _ => ("a night at the bar", bar::bar_world(), bar::PLAYER_NAME),
    }
}

// ---- non-interactive subcommands -------------------------------------------

/// `prax check [world]` — the static well-formedness report. Byte-identical to
/// the frozen `runCheck` (well-formed line, or a numbered problem list rendered
/// by the shared byte-exact [`describe`]).
fn render_check(name: &str, world: &State) -> String {
    let es = type_check(world);
    if es.is_empty() {
        format!("well-formed: {name}\n")
    } else {
        let mut out = format!("{name} — {} problem(s):\n", es.len());
        for e in &es {
            out.push_str(&format!("  - {}\n", describe(e)));
        }
        out
    }
}

/// Render a `[(String, Int)]` association list exactly as Haskell's `show` on
/// `Map.toList` does: `[("k1",v1),("k2",v2)]`, sorted by key.
fn show_assoc(m: &std::collections::BTreeMap<String, i64>) -> String {
    let inner: Vec<String> = m.iter().map(|(k, v)| format!("(\"{k}\",{v})")).collect();
    format!("[{}]", inner.join(","))
}

/// `prax stress [world]` — 200 random runs, cap 50, tracking `currentScene`.
/// Byte-identical to the frozen `runStress`.
fn render_stress(name: &str, world: &State) -> String {
    let r: StressReport = stress_test(200, 50, Some(CURRENT_SCENE_PATH), world);
    let mut out = format!("stress-testing {name} — 200 random runs, cap 50 turns\n");
    out.push_str(&format!("  endings:   {}\n", show_assoc(&r.endings)));
    out.push_str(&format!("  coverage:  {} distinct actions fired\n", r.coverage.len()));
    if !r.visited.is_empty() {
        out.push_str(&format!("  scenes:    {} (runs visiting each)\n", show_assoc(&r.visited)));
    }
    out.push_str(&format!("  dead ends: {}\n", r.dead_ends));
    out.push_str(&format!("  no ending: {} / {} runs\n", r.no_ending, r.runs));
    out
}

// ---- the player's affordances (a native-pinned library surface) ------------

/// The actions the player is OFFERED (`app/Main.hs`'s `playerActions`):
/// `candidate_actions` minus pure no-ops (empty-outcome actions). The player has
/// `m` to pass, so a no-op is menu noise; NPCs keep it. Uses `candidate_actions`,
/// so a practice-bound player (the drama manager) is offered only its bound
/// practice's affordances.
pub fn player_actions(st: &mut State, actor: &Character) -> Vec<GroundedAction> {
    let cands = st.candidate_actions(actor);
    let defs = st.practice_defs();
    let is_no_op = |ga: &GroundedAction| {
        defs.get(&ga.practice_id).is_some_and(|def| {
            def.actions
                .iter()
                .find(|a| a.name == ga.action_id)
                .is_some_and(|a| a.then.is_empty())
        })
    };
    cands.into_iter().filter(|ga| !is_no_op(ga)).collect()
}

/// The first reached ending, if any (an `ending.<key>` fact).
fn ending_of(st: &mut State) -> Option<String> {
    st.db_child_keys("ending").into_iter().next()
}

// ---- interactive play (S10 demo surface; setup + resume pinned here) --------

/// A cosmetic scene sketch — DEMO-verified, not conformance-pinned ([R11]).
fn render_scene(st: &mut State) -> String {
    match prax_script::compile::current_scene_of(st) {
        Some(scene) => format!("  (scene: {scene})\n"),
        None => String::new(),
    }
}

enum Choice {
    Quit,
    Wait,
    Save,
    Pick(usize),
}

fn prompt() -> Choice {
    print!("> ");
    let _ = stdout().flush();
    let mut line = String::new();
    if stdin().read_line(&mut line).unwrap_or(0) == 0 {
        return Choice::Quit;
    }
    match line.trim() {
        "q" => Choice::Quit,
        "m" => Choice::Wait,
        "s" => Choice::Save,
        other => match other.strip_prefix("Pick ").unwrap_or(other).parse::<usize>() {
            Ok(n) => Choice::Pick(n),
            Err(_) => Choice::Pick(0),
        },
    }
}

fn banner(title: &str) {
    let bar: String = "=".repeat(title.len() + 4);
    println!("{bar}");
    println!("  {title}");
    println!("{bar}");
}

/// The round-robin loop: advance, act (NPC) or hand to the player.
fn game_loop(player: &str, mut st: State) {
    loop {
        if let Some(e) = ending_of(&mut st) {
            println!("\n==============================");
            println!("  THE END — {e}");
            println!("==============================");
            return;
        }
        let save_point = st.clone(); // the state BEFORE advancing to the player
        let actor = advance(&mut st);
        if actor.name == player {
            if !player_turn(player, &save_point, &mut st, &actor) {
                return;
            }
        } else if let Some(ga) = npc_act(&mut st, LOOKAHEAD_DEPTH, &actor) {
            println!("  {}", ga.label);
        }
    }
}

/// One player turn. Returns `false` to end the game. `save_point` is the state
/// BEFORE the advance, so resuming from it replays the advance and lands here.
fn player_turn(player: &str, save_point: &State, st: &mut State, actor: &Character) -> bool {
    loop {
        println!("\n-------------------- scene --------------------");
        print!("{}", render_scene(st));
        let acts = player_actions(st, actor);
        println!("Your move ({}):", actor.name);
        for (i, a) in acts.iter().enumerate() {
            println!("  {}) {}", i + 1, a.label);
        }
        println!("  m) wait and let others act");
        println!("  s) save    q) quit");
        match prompt() {
            Choice::Quit => {
                println!("Bye.");
                return false;
            }
            Choice::Wait => {
                game_loop(player, st.clone());
                return false;
            }
            Choice::Save => {
                match std::fs::write(SAVE_FILE, prax_core::persist::serialize_state(save_point)) {
                    Ok(()) => println!("(saved to {SAVE_FILE})"),
                    Err(e) => println!("(save failed: {e})"),
                }
            }
            Choice::Pick(i) if i >= 1 && i <= acts.len() => {
                let ga = acts[i - 1].clone();
                println!("> {}", ga.label);
                st.perform_action(&ga);
                game_loop(player, st.clone());
                return false;
            }
            Choice::Pick(_) => println!("No such option."),
        }
    }
}

fn play(args: &[String]) {
    let (title, world, player) = world_named(args);
    banner(&format!("prax — {title}"));
    let world = if args.iter().any(|a| a == "resume") {
        println!("(resumed from {SAVE_FILE})");
        let fresh = world_named(args).1;
        match std::fs::read_to_string(SAVE_FILE)
            .map_err(|e| e.to_string())
            .and_then(|t| deserialize_state(&t, fresh).map_err(|e| e.to_string()))
        {
            Ok(st) => st,
            Err(e) => {
                eprintln!("could not resume: {e}");
                return;
            }
        }
    } else {
        world
    };
    game_loop(player, world);
}

fn play_file(file: &str) {
    let bytes = match std::fs::read(file) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("could not load {file}: {e}");
            std::process::exit(1);
        }
    };
    let scr = match decode_script(&bytes) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("could not load {file}: {e}");
            std::process::exit(1);
        }
    };
    banner(&format!("prax — {file}"));
    let world = compile(&scr).expect("the loaded script compiles");
    let player = script_player(&scr).expect("the script has a player").to_owned();
    game_loop(&player, world);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("stress") => {
            let (name, world, _) = world_named(&args[1..]);
            print!("{}", render_stress(name, &world));
        }
        Some("check") => {
            let (name, world, _) = world_named(&args[1..]);
            print!("{}", render_check(name, &world));
        }
        // `putStrLn` adds the trailing newline that makes the stdout byte-identical
        // to the shipped `examples/play.json` (2122 bytes, S8 [R7]).
        Some("dump-play") => println!("{}", encode_script(&play::play_script())),
        Some("flow") => {
            let script = match args.iter().find(|a| a.ends_with(".json")) {
                Some(f) => match std::fs::read(f).map_err(|e| e.to_string()).and_then(|b| decode_script(&b)) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("could not load {f}: {e}");
                        std::process::exit(1);
                    }
                },
                None => play::play_script(),
            };
            print!("{}", flow_chart(&script));
        }
        Some("play") if args.get(1).is_some_and(|f| f.ends_with(".json")) => play_file(&args[1]),
        _ => play(&args),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_core::types::{Action, Practice, insert};

    // dump-play: the whole criterion is byte-identity to the shipped file (S8).
    #[test]
    fn dump_play_is_byte_identical_to_the_shipped_example() {
        // `prax dump-play` emits `encode_script(...)` + a trailing newline
        // (the frozen `putStrLn`), which IS the shipped file byte-for-byte.
        let shipped = include_str!("../../../examples/play.json");
        assert_eq!(format!("{}\n", encode_script(&play::play_script())), shipped);
    }

    // check: the well-formed line per world (deterministic golden).
    #[test]
    fn check_reports_every_shipped_world_well_formed() {
        for (args, name) in [
            (vec![], "a night at the bar"),
            (vec!["intrigue".to_owned()], "Intrigue (Rome)"),
            (vec!["play".to_owned()], "the conspiracy (a play)"),
            (vec!["village".to_owned()], "the village"),
        ] {
            let (n, world, _) = world_named(&args);
            assert_eq!(n, name);
            assert_eq!(render_check(n, &world), format!("well-formed: {name}\n"));
        }
    }

    #[test]
    fn vampire_world_is_selectable() {
        let (title, world, player) = world_named(&["vampire".to_owned()]);
        assert_eq!(title, "the vampire village");
        assert_eq!(player, "bram");
        assert!(
            world.labeled_facts().iter().any(|f| f == "character.bram"),
            "world_named(\"vampire\") returns the vampire village"
        );
    }

    // flow ([P7]): byte-exact stdout golden. `prax flow` (no `.json` arg) prints
    // `flow_chart(&play::play_script())`; the committed golden IS those exact
    // bytes, captured while the frozen still certifies them. A durable resident
    // net — a mid-output format drift reddens here, not only against the dying
    // oracle differential. Golden lives beside the CLI goldens under
    // `conformance/goldens/cli/`, held apart from the frozen-EXTRACTED
    // decision-sequence goldens that `golden-check.sh` derives.
    #[test]
    fn flow_renders_the_default_play_script() {
        let golden = include_str!("../../../conformance/goldens/cli/flow-play.txt");
        assert_eq!(flow_chart(&play::play_script()), golden);
    }

    // stress ([P7]): byte-exact stdout golden. `prax stress play` prints
    // `render_stress("the conspiracy (a play)", &play::play_world())` — a
    // deterministic 200-run report (fixed RNG). The committed golden IS those
    // exact bytes; a drift in any of the endings/coverage/scenes/dead-ends lines
    // reddens against a resident net, not the dying differential.
    #[test]
    fn stress_renders_a_deterministic_report() {
        let golden = include_str!("../../../conformance/goldens/cli/stress-play.txt");
        let (name, world, _) = world_named(&["play".to_owned()]);
        assert_eq!(render_stress(name, &world), golden);
    }

    /// A no-op action (empty outcomes) is HIDDEN from the player but kept as an
    /// NPC candidate ([R11]). REDDENS UNDER: dropping the no-op filter.
    #[test]
    fn player_actions_hide_no_ops_but_candidates_keep_them() {
        let p = Practice::new("idle")
            .roles(["R"])
            .action(Action::new("[Actor]: do a thing").then([insert("did.Actor")]))
            .action(Action::new("[Actor]: wait a moment").then([]));
        let mut st = State::new();
        st.define_practices([p]).unwrap();
        st.perform_outcome(&insert("practice.idle.here")).unwrap();
        st.set_characters(vec![Character::new("you")]).unwrap();
        let you = st.characters().iter().find(|c| c.name == "you").unwrap().clone();
        let cands = st.candidate_actions(&you);
        let player = player_actions(&mut st, &you);
        assert!(
            cands.iter().any(|g| g.label.contains("wait a moment")),
            "the no-op stays an NPC candidate"
        );
        assert!(
            !player.iter().any(|g| g.label.contains("wait a moment")),
            "the no-op is hidden from the player"
        );
        assert!(
            player.iter().any(|g| g.label.contains("do a thing")),
            "the real action is still offered"
        );
    }

    /// A practice-bound player (the drama manager) is offered ONLY its bound
    /// practice's affordances — `candidate_actions`' bound-player restriction.
    #[test]
    fn a_bound_player_sees_only_its_practices_affordances() {
        let mut st = bar::bar_director_world();
        let director = st
            .characters()
            .iter()
            .find(|c| c.name == bar::DIRECTOR_NAME)
            .unwrap()
            .clone();
        assert!(director.bound_to.is_some(), "the director is practice-bound");
        let bound = director.bound_to.clone().unwrap();
        for ga in player_actions(&mut st, &director) {
            assert_eq!(
                ga.practice_id, bound,
                "a bound player is offered only its bound practice's affordances"
            );
        }
    }

    /// `save_point` is the state BEFORE advancing to the player; resuming from it
    /// (Persist round-trip) and advancing lands back on the SAME actor — the
    /// frozen save-point subtlety. REDDENS UNDER: saving the post-advance state.
    #[test]
    fn resume_from_the_save_point_replays_the_advance() {
        use prax_core::persist::serialize_state;
        let (_, world, player) = world_named(&["intrigue".to_owned()]);
        let mut st = world.clone();
        let mut save_point;
        loop {
            save_point = st.clone();
            let actor = advance(&mut st);
            if actor.name == player {
                break;
            }
        }
        let fresh = world_named(&["intrigue".to_owned()]).1;
        let mut resumed = deserialize_state(&serialize_state(&save_point), fresh).unwrap();
        let re_actor = advance(&mut resumed);
        assert_eq!(
            re_actor.name, player,
            "resuming the save-point and re-advancing lands back on the player's turn"
        );
    }
}
