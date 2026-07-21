//! The SURVIVING self-consistency net ([P2a]/§4): the `--baseline` loader path,
//! exercised over the committed corpus with NO frozen oracle.
//!
//! `harness_tests` drives the frozen Haskell oracle as a subprocess and DIES at
//! the frozen tree's deletion. This module does not — it loads the committed
//! `conformance/oracle-baselines/` corpus and compares it against the live
//! in-process Rust walk through the SAME [`crate::compare::compare_streams`] the
//! frozen path uses. So `cargo test --workspace` keeps covering the
//! compare_streams / baseline-loader pipeline after the frozen is gone, which is
//! exactly the window [P2] closes: the loader is new code written for the moment
//! its counterparty is destroyed, and it must be exercised by a resident test
//! BEFORE that, not first run by a manual operator with no oracle to adjudicate.

use crate::classify::{Ctx, Shape, Walk};
use crate::compare::Outcome;
use crate::record::{Emit, Mode};
use crate::{Reference, RunSpec, load_register, run_one, worlds};

/// The committed corpus root: the repo-root `conformance/oracle-baselines/` data
/// dir (§5c), not the `rust/conformance/` crate dir.
fn baseline_dir() -> std::path::PathBuf {
    crate::repo_root()
        .expect("the repository root resolves")
        .join("conformance/oracle-baselines")
}

/// A `village` randtrace spec at the corpus's captured parameters — cap 50,
/// `Emit::matrix()`, `Mode::State`, depth 2 — so the live walk is byte-for-byte
/// the emission the corpus holds.
fn village_randtrace_spec(seed: i64) -> RunSpec {
    RunSpec {
        world: "village".to_owned(),
        walk: Walk::Randtrace,
        steps: 50,
        seed: Some(seed),
        die_seed: None,
        depth: 2,
        idle: worlds::idler("village").map(str::to_owned),
        mode: Mode::State,
        emit: Emit::matrix(),
    }
}

#[test]
fn the_baseline_loader_agrees_with_the_live_walk_over_committed_village_cells() {
    // The surviving net, GREEN direction: load committed village cells, run the
    // live in-process walk, and compare through the SAME compare_streams the
    // frozen path uses — Rust-now vs Rust-at-capture. No cabal, no frozen
    // subprocess, so this survives the frozen tree's deletion and keeps the
    // baseline pipeline under `cargo test`.
    let reg = load_register().expect("the register loads");
    let reference = Reference::Baseline(baseline_dir());

    for seed in [0, 1, 7, 42, 123, 499] {
        let o = run_one(&village_randtrace_spec(seed), &reg, &reference)
            .expect("the run completes")
            .outcome;
        println!("village randtrace seed {seed}: {}", o.cell());
        assert!(
            matches!(o, Outcome::Clean { .. }),
            "the live village walk diverged from the committed baseline at seed {seed}: {o:?}"
        );
    }

    // The trace cell too — the other cell kind the corpus commits (turns 24,
    // idler-driven). A randtrace-only test would leave the trace loader unproven.
    let trace = RunSpec {
        walk: Walk::Trace,
        steps: 24,
        seed: None,
        ..village_randtrace_spec(0)
    };
    let o = run_one(&trace, &reg, &reference)
        .expect("the run completes")
        .outcome;
    println!("village trace: {}", o.cell());
    assert!(
        matches!(o, Outcome::Clean { .. }),
        "the live village trace walk diverged from the committed baseline: {o:?}"
    );
}

#[test]
fn a_missing_baseline_cell_fails_loud_and_is_never_a_silent_skip() {
    // A missing cell is a BROKEN NET, not a skip (§4): the loader must fail loudly
    // rather than let the tripwire pass with nothing to compare against. Against
    // the REAL committed corpus dir (so the corpus identity resolves), request a
    // seed whose cell does not exist — isolating the loader's missing-cell path.
    let reg = load_register().expect("the register loads");
    let reference = Reference::Baseline(baseline_dir());
    let e = match run_one(&village_randtrace_spec(99_999), &reg, &reference) {
        Ok(_) => panic!("a missing baseline cell must be a loud error, never a silent pass"),
        Err(e) => e,
    };
    println!("{e}");
    assert!(
        e.contains("BROKEN NET") && e.contains("randtrace-99999.jsonl"),
        "the error must name the missing cell and refuse to skip it: {e}"
    );
}

#[test]
fn a_corrupted_baseline_record_reddens_the_surviving_net() {
    // MUTATION-VERIFY the net BITES. The same loader + compare_streams that runs
    // clean above must REDDEN when a single loaded baseline record is perturbed —
    // otherwise the green above could be a both-sides no-op and the tripwire would
    // be disarmed. The committed file is never touched: the pristine stream is
    // loaded, cloned, and one record of the CLONE is corrupted in memory.
    let reg = load_register().expect("the register loads");
    let spec = village_randtrace_spec(0);
    let dir = baseline_dir();
    let ctx = Ctx {
        walk: spec.walk,
        shape: Shape::Green("baseline-mutation-test".to_owned()),
        view_differs_earlier: None,
    };

    // GREEN: the pristine committed cell agrees with the live walk.
    let pristine = crate::load_baseline_cell(&dir, &spec).expect("the committed cell loads");
    let rust = crate::rust_stream(&spec).expect("the live walk");
    let clean = crate::compare::compare_streams(
        &pristine, &rust, &ctx, &reg, &spec.world, spec.sub(), spec.seed,
    )
    .expect("the comparison completes");
    assert!(
        matches!(clean, Outcome::Clean { .. }),
        "the pristine committed cell must agree with the live walk: {clean:?}"
    );

    // RED: corrupt one non-header record's `action` and the net must catch it.
    let mut corrupted = pristine.clone();
    corrupted
        .get_mut(1)
        .expect("a walked record after the header")
        .as_object_mut()
        .expect("a record object")
        .insert(
            "action".to_owned(),
            serde_json::json!("CORRUPTED: not the captured action"),
        );
    let red = crate::compare::compare_streams(
        &corrupted, &rust, &ctx, &reg, &spec.world, spec.sub(), spec.seed,
    )
    .expect("the comparison completes");
    assert!(
        red.is_failure(),
        "a corrupted baseline record must REDDEN the surviving net, got {red:?} — the tripwire \
         is disarmed"
    );
    println!(
        "mutation-verify: pristine -> {}, corrupted `action` -> {}",
        clean.cell(),
        red.cell()
    );

    // RESTORE / non-destructiveness: the on-disk corpus is untouched (we mutated a
    // clone), so re-loading yields the identical pristine stream.
    let reloaded = crate::load_baseline_cell(&dir, &spec).expect("the committed cell re-loads");
    assert_eq!(
        reloaded, pristine,
        "the committed corpus file must be untouched by the in-memory mutation"
    );
}
