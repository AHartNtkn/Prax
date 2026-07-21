//! `Prax.Stress` — the stress/coverage aggregator over the random walk.
//!
//! [R7]/[P8]: `walk.rs` owns the walk's generator (`lcg`/`pick`), stop names
//! (`Stop`) and — since S9 — the go-loop's stop DECISION (`pre_advance_stop`).
//! This module adds the two things the frozen `Prax.Stress` is: [`run_random`],
//! which plays ONE seeded game to a [`RunResult`], and [`stress_test`], which
//! folds `run_random` over the deterministic `seedFor` sweep into a
//! [`StressReport`]. The go-loop's four stop exits come from the SAME
//! `pre_advance_stop` the randtrace driver uses, so the two walks cannot drift
//! (verified behaviour-preserving by re-running the randtrace differential).
//!
//! Coverage over a single-valued fact family (`<family>.<id>` / `<family>!<id>`,
//! e.g. a Script world's `currentScene`) is an OPTIONAL declared parameter — no
//! family is privileged by this module (only the CLI's callers name one).
//!
//! STATED LIMIT of the dead-end detector (v46): the idle-pass counter tolerates
//! exactly ONE round boundary of move-less progression; a scene advancing only
//! via the schedule across TWO+ boundaries reports a spurious dead end. No
//! shipped world has that shape.

use std::collections::{BTreeMap, BTreeSet};

use prax_core::engine::State;
use prax_core::turn::advance;

use crate::walk::{Stop, pick, pre_advance_stop};

/// The result of one random play (`Prax.Stress.RunResult`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RunResult {
    /// The ending reached, if any.
    pub ending: Option<String>,
    /// Ids of actions performed.
    pub actions: BTreeSet<String>,
    /// Coverage-family members visited (empty if no family named, or the world
    /// populates none of it).
    pub visited: BTreeSet<String>,
    /// A living character had no available action for a full rotation + wrap.
    pub dead_end: bool,
    /// Turns actually spent (idle passes excluded).
    pub turns: i64,
}

/// Aggregated report over many runs (`Prax.Stress.StressReport`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StressReport {
    pub runs: i64,
    /// ending → how many runs reached it.
    pub endings: BTreeMap<String, i64>,
    /// every action id that fired in any run.
    pub coverage: BTreeSet<String>,
    /// coverage-family member → how many runs visited it.
    pub visited: BTreeMap<String, i64>,
    /// runs that hit a dead end.
    pub dead_ends: i64,
    /// runs that hit the turn cap with no ending.
    pub no_ending: i64,
}

/// The ending reached, if any — an `ending.<key>` fact in the BASE db (the
/// frozen `endingReached`, first child key).
fn ending_reached(st: &mut State) -> Option<String> {
    st.db_child_keys("ending").into_iter().next()
}

/// The current member of the coverage `family`, if any (`familyReached`): the
/// first child key of the family path. A `!`- or `.`-separated single-valued
/// family both present as one child key, so either spelling is tracked.
fn family_reached(st: &mut State, family: &str) -> Option<String> {
    st.db_child_keys(family).into_iter().next()
}

/// Record the visited state's current member of the tracked family, if any.
fn record_visit(st: &mut State, visited: &mut BTreeSet<String>, family: Option<&str>) {
    if let Some(f) = family
        && let Some(s) = family_reached(st, f)
    {
        visited.insert(s);
    }
}

/// Play one game for up to `cap` turns: each turn the next living character
/// performs a uniformly-random available action, stopping at an ending, the cap,
/// extinction, or a true dead end. A character with no action passes (idle
/// passes do not spend a turn). `family` is the optional coverage family.
pub fn run_random(st0: &State, cap: i64, seed: u64, family: Option<&str>) -> RunResult {
    let mut st = st0.clone();
    let mut actions: BTreeSet<String> = BTreeSet::new();
    let mut visited: BTreeSet<String> = BTreeSet::new();
    record_visit(&mut st, &mut visited, family);
    let mut k = cap;
    let mut s = seed;
    let mut passes: i64 = 0;
    let mut turns: i64 = 0;
    loop {
        let ending = ending_reached(&mut st);
        let living = st.living_characters().len();
        if let Some(stop) = pre_advance_stop(k, ending.clone(), living, passes) {
            // Cap reports the current ending if one exists; Ending reports its
            // key; Extinct/DeadEnd report no ending. dead_end iff DeadEnd.
            let ending = match &stop {
                Stop::Cap => ending,
                Stop::Ending(e) => Some(e.clone()),
                _ => None,
            };
            return RunResult {
                ending,
                actions,
                visited,
                dead_end: matches!(stop, Stop::DeadEnd),
                turns,
            };
        }
        let actor = advance(&mut st);
        let acts = st.possible_actions(&actor.name);
        if acts.is_empty() {
            record_visit(&mut st, &mut visited, family); // idle: pass, no turn
            passes += 1;
            continue;
        }
        let (i, s2) = pick(acts.len(), s);
        let ga = acts[i].clone();
        actions.insert(ga.action_id.clone());
        st.perform_action(&ga);
        record_visit(&mut st, &mut visited, family);
        turns += 1;
        passes = 0;
        k -= 1;
        s = s2;
    }
}

/// The frozen `seedFor i = fromIntegral i * 2654435761` (Word64), spreading the
/// seeds apart.
fn seed_for(i: i64) -> u64 {
    (i as u64).wrapping_mul(2_654_435_761)
}

/// Run `runs` seeded random games of up to `cap` turns and aggregate the report.
/// `family` is the optional coverage family (the CLI passes `currentScene`; this
/// module privileges nothing).
pub fn stress_test(runs: i64, cap: i64, family: Option<&str>, st0: &State) -> StressReport {
    let mut report = StressReport {
        runs,
        endings: BTreeMap::new(),
        coverage: BTreeSet::new(),
        visited: BTreeMap::new(),
        dead_ends: 0,
        no_ending: 0,
    };
    for i in 1..=runs {
        let res = run_random(st0, cap, seed_for(i), family);
        if let Some(e) = &res.ending {
            *report.endings.entry(e.clone()).or_insert(0) += 1;
        }
        report.coverage.extend(res.actions.iter().cloned());
        for s in &res.visited {
            *report.visited.entry(s.clone()).or_insert(0) += 1;
        }
        if res.dead_end {
            report.dead_ends += 1;
        }
        if res.ending.is_none() && !res.dead_end {
            report.no_ending += 1;
        }
    }
    report
}

#[cfg(test)]
mod tests {
    // H: StressSpec.hs "Prax.Stress"
    //
    // The frozen `Prax.Stress` spec, over the aggregator.
    use super::*;
    use prax_script::compile::compile;
    use prax_script::script::{Script, beat, goto, member, player, scene};
    use prax_worlds::bar::bar_world;
    use prax_worlds::intrigue::intrigue_world;
    use prax_worlds::play::play_world;
    use prax_worlds::village::village_world;

    /// The v46 finding-4 repro: scene "s" offers its cast no beat, only a pending
    /// unconditional transition to "s2" — the only way forward is the engine's
    /// round-boundary story rule. The dead-end detector must cross it.
    fn dead_end_regression_world() -> State {
        let scr = Script::new("s")
            .cast([player("p"), member("q")])
            .scenes([
                scene("s").junctions([goto("go", "s2", vec![])]),
                scene("s2").beats([beat("linger", vec![], vec![])]),
            ]);
        compile(&scr).expect("the regression script compiles")
    }

    // H: StressSpec.hs "a move-less scene with a pending transition is not a stress-harness false positive: the dead-end detector must give the round boundary's wrap its turn before declaring deadlock (v46 review finding 4)"
    #[test]
    fn a_move_less_scene_with_a_pending_transition_is_not_a_false_positive() {
        let r = stress_test(50, 40, Some("currentScene"), &dead_end_regression_world());
        assert_eq!(r.dead_ends, 0);
        assert!(r.visited.contains_key("s2"), "s2 reached");
    }

    // H: StressSpec.hs "random play of the episode: no dead ends, both active branches reached"
    #[test]
    fn random_play_of_the_episode_no_dead_ends_both_branches() {
        let r = stress_test(60, 40, Some("currentScene"), &intrigue_world());
        assert_eq!(r.dead_ends, 0, "no dead ends");
        assert_eq!(r.no_ending, 0, "no run stuck at the cap");
        assert!(r.endings.contains_key("loyalty"), "loyalty reached");
        assert!(r.endings.contains_key("complicity"), "complicity reached");
    }

    // H: StressSpec.hs "the bar survives random play with no dead ends and broad coverage"
    #[test]
    fn the_bar_survives_random_play() {
        let r = stress_test(20, 30, Some("currentScene"), &bar_world());
        assert_eq!(r.dead_ends, 0);
        assert!(r.coverage.len() >= 10, "many distinct actions exercised: {}", r.coverage.len());
    }

    // H: StressSpec.hs "scene coverage: random play reaches both scenes and every ending"
    #[test]
    fn scene_coverage_reaches_both_scenes_and_every_ending() {
        let r = stress_test(200, 50, Some("currentScene"), &play_world());
        assert!(r.visited.contains_key("confidence"), "confidence visited");
        assert!(r.visited.contains_key("banquet"), "banquet visited");
        for e in ["betrayal", "loyalty", "complicity"] {
            assert!(r.endings.contains_key(e), "{e} reached");
        }
        assert_eq!(r.dead_ends, 0, "no dead ends");
    }

    // H: StressSpec.hs "coverage family generalizes past Script's currentScene: the village's marketDay family is tracked when named, proving the second application"
    #[test]
    fn coverage_family_generalizes_past_current_scene() {
        let r = stress_test(80, 60, Some("marketDay"), &village_world());
        assert!(r.visited.contains_key("square"), "the market was observed open at least once");
    }
}
