//! `Prax.Stress` — stress-testing and coverage over a random walk.
//!
//! The frozen `Prax.Stress` is an engine-LIBRARY module (its own tiny LCG, its
//! own go-loop), so its Rust home is here in `prax-core` — reachable by BOTH the
//! `prax-cli` player (`prax stress`) and the `prax-oracle` differential, which a
//! binary-crate home could not be. [P8]: the go-loop's stop DECISION
//! ([`pre_advance_stop`]) is single-sourced here and consumed by both the
//! randtrace driver and [`run_random`], so the two walks cannot drift; the
//! generator ([`lcg`]/[`pick`], Knuth's MMIX constants) is likewise single-sourced.
//!
//! `stress_test` plays many seeded games in which every character takes a
//! uniformly-random available action each turn, reporting which endings were
//! reached, which action ids ever fired (coverage), which coverage-family
//! members were visited, and how many runs hit a dead end. Pure and
//! deterministic given the seeds.
//!
//! STATED LIMIT of the dead-end detector (v46): the idle-pass counter tolerates
//! exactly ONE round boundary of move-less progression; a scene advancing only
//! via the schedule across TWO+ boundaries reports a spurious dead end. No
//! shipped world has that shape.

use std::collections::{BTreeMap, BTreeSet};

use crate::engine::State;
use crate::turn::advance;

/// One MMIX linear-congruential step (Knuth's constants), wrapping `u64`
/// arithmetic — the frozen `Prax.Stress.lcg`. NOT the engine's MINSTD die.
pub fn lcg(x: u64) -> u64 {
    6_364_136_223_846_793_005_u64
        .wrapping_mul(x)
        .wrapping_add(1_442_695_040_888_963_407)
}

/// A uniform index in `[0, n)` and the next seed — the frozen `pick`.
///
/// # Panics
/// If `n == 0` (the walk only picks from a non-empty candidate list).
pub fn pick(n: usize, s: u64) -> (usize, u64) {
    assert!(n > 0, "pick from an empty candidate list");
    let s2 = lcg(s);
    ((s2 % n as u64) as usize, s2)
}

/// Why a random walk stopped, at the point BEFORE the round's `advance`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RunStop {
    /// The step cap was reached (`k == 0`).
    Cap,
    /// An `ending.<key>` fact appeared.
    Ending(String),
    /// The whole cast is dead.
    Extinct,
    /// Every living character passed for a full rotation + wrap — a dead end.
    DeadEnd,
}

/// The go-loop's terminal stop decision, taken BEFORE the round's `advance` —
/// the frozen `runRandom`'s four exits IN ORDER: cap, ending, extinction,
/// dead-end. Single-sourced ([P8]) so the randtrace driver and [`run_random`]
/// never drift. `None` ⇒ take a turn.
///
/// `passes > living` is the v46 dead-end rule: only past a FULL rotation of idle
/// passes has the round-boundary wrap had its say (the boundary fires on the
/// wrap call, and `cursor` starts one below every valid index), so the streak
/// crossed a boundary and still changed nothing.
pub fn pre_advance_stop(
    k: i64,
    ending: Option<String>,
    living: usize,
    passes: i64,
) -> Option<RunStop> {
    if k == 0 {
        return Some(RunStop::Cap);
    }
    if let Some(e) = ending {
        return Some(RunStop::Ending(e));
    }
    if living == 0 {
        return Some(RunStop::Extinct);
    }
    if passes > living as i64 {
        return Some(RunStop::DeadEnd);
    }
    None
}

/// The result of one random play (`Prax.Stress.RunResult`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RunResult {
    /// The ending reached, if any.
    pub ending: Option<String>,
    /// Ids of actions performed.
    pub actions: BTreeSet<String>,
    /// Coverage-family members visited.
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

/// The ending reached, if any — an `ending.<key>` fact (the frozen
/// `endingReached`, first child key).
fn ending_reached(st: &mut State) -> Option<String> {
    st.db_child_keys("ending").into_iter().next()
}

/// The current member of the coverage `family`, if any (`familyReached`): the
/// first child key of the family path. A `!`- or `.`-separated single-valued
/// family both present as one child key.
fn family_reached(st: &mut State, family: &str) -> Option<String> {
    st.db_child_keys(family).into_iter().next()
}

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
            let ending = match &stop {
                RunStop::Cap => ending,
                RunStop::Ending(e) => Some(e.clone()),
                _ => None,
            };
            return RunResult {
                ending,
                actions,
                visited,
                dead_end: matches!(stop, RunStop::DeadEnd),
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

/// The frozen `seedFor i = fromIntegral i * 2654435761` (Word64).
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
    use super::*;

    // The generator is a transcription, pinned by its own arithmetic.
    #[test]
    fn lcg_step_is_the_mmix_recurrence() {
        assert_eq!(lcg(0), 1_442_695_040_888_963_407);
        assert_eq!(
            lcg(1),
            6_364_136_223_846_793_005_u64.wrapping_add(1_442_695_040_888_963_407)
        );
    }

    #[test]
    fn pick_indexes_the_advanced_seed_not_the_current_one() {
        let (i, s) = pick(7, 0);
        assert_eq!(s, lcg(0));
        assert_eq!(i, (lcg(0) % 7) as usize);
    }

    #[test]
    fn pre_advance_stop_orders_cap_before_ending() {
        // k == 0 is Cap even if an ending is present (frozen order).
        assert_eq!(pre_advance_stop(0, Some("x".into()), 3, 0), Some(RunStop::Cap));
        assert_eq!(pre_advance_stop(5, Some("x".into()), 3, 0), Some(RunStop::Ending("x".into())));
        assert_eq!(pre_advance_stop(5, None, 0, 0), Some(RunStop::Extinct));
        assert_eq!(pre_advance_stop(5, None, 2, 3), Some(RunStop::DeadEnd));
        assert_eq!(pre_advance_stop(5, None, 2, 2), None);
    }
}
