//! The oracle's `stress` differential surface over `prax_core::stress`.
//!
//! The aggregator itself (`run_random`/`stress_test`/`RunResult`/`StressReport`)
//! and the go-loop's stop DECISION live in `prax_core::stress` (the frozen
//! `Prax.Stress` is an engine-library module, reachable by both the CLI and this
//! differential; [P8]). This module adds only the oracle's canonical-JSON
//! rendering and the fixed differential parameters, plus the frozen StressSpec
//! re-expressed over the shared aggregator.

use prax_core::stress::StressReport;

/// The report as CANONICAL JSON for the oracle `stress` differential — the
/// `BTreeMap`/`BTreeSet` fields serialize sorted, so the document is
/// Value-vs-Value comparable against the frozen `oracle stress`. Field spellings
/// mirror `StressReport`'s frozen record (`srEndings`/…) in camelCase.
pub fn report_json(r: &StressReport) -> serde_json::Value {
    serde_json::json!({
        "runs": r.runs,
        "endings": r.endings,
        "coverage": r.coverage,
        "visited": r.visited,
        "deadEnds": r.dead_ends,
        "noEnding": r.no_ending,
    })
}

/// The fixed parameters the oracle `stress` differential drives with, on BOTH
/// engines: enough runs to exercise the many-seed coverage/dead-end tracking the
/// single-seed randtrace channel never reaches, tracking the `currentScene`
/// family the CLI's own callers name.
pub const DIFF_RUNS: i64 = 50;
pub const DIFF_CAP: i64 = 40;
pub const DIFF_FAMILY: &str = "currentScene";

#[cfg(test)]
mod tests {
    // H: StressSpec.hs "Prax.Stress"
    //
    // The frozen `Prax.Stress` spec, over the shared aggregator.
    use prax_core::stress::stress_test;
    use prax_script::compile::compile;
    use prax_script::script::{Script, beat, goto, member, player, scene};
    use prax_core::engine::State;
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
