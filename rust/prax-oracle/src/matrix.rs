//! Matrix mode: one line per (world, seed) — the report's only source of
//! numbers.
//!
//! Worlds are shape-checked (§2) ONCE, BEFORE any seed runs, so a world-port
//! error costs one line rather than a hundred localizations — and the check does
//! not repeat per cell, where at 100 seeds × 4 worlds it would be ~400 redundant
//! frozen invocations plus ~800 `git` subprocesses saying the same thing. Every
//! run carries `--candidates` [S-I4]: without it ENUMERATION can never fire and
//! every enumeration bug reports as DECISION.
//!
//! `--jobs N` parallelizes over the seeds of one world (§1.8, mandatory at 100+
//! seeds × 4 worlds): the frozen subprocess is the bottleneck and the cache is
//! keyed by freeze rev and argv, so distinct seeds never contend. The OUTPUT is
//! in seed order regardless of `N` — a matrix whose line order depended on
//! scheduling could not be diffed between runs, and reports embed it verbatim.
//!
//! `--format report` emits the per-world block that stage reports embed
//! VERBATIM. Reports never carry hand-typed matrix numbers — a number a human
//! retyped is a number that can drift from the run that produced it.

use crate::classify::{Shape, Walk};
use crate::compare::Outcome;
use crate::record::{Emit, Mode};
use crate::register::Register;
use crate::{RunSpec, drive_frozen, load_register, run_one_behind, shape_compare, worlds};

/// One (world, seed) result.
struct Cell {
    world: String,
    outcome: Outcome,
}

/// Parse `A..B` (inclusive).
fn parse_seeds(s: &str) -> Result<Vec<i64>, String> {
    let (a, b) = s
        .split_once("..")
        .ok_or_else(|| format!("--seeds expects A..B, got `{s}`"))?;
    let a: i64 = a.parse().map_err(|_| format!("bad seed start `{a}`"))?;
    let b: i64 = b.parse().map_err(|_| format!("bad seed end `{b}`"))?;
    Ok((a..=b).collect())
}

/// Run the matrix. Returns whether every cell passed.
///
/// # Errors
/// If a world cannot be driven or the classifier refuses.
pub fn run(args: &[&str]) -> Result<bool, String> {
    let worlds_arg: Vec<String> = match crate::flag(args, "--worlds") {
        Some(s) => s.split(',').map(str::to_owned).collect(),
        None => worlds::ported().into_iter().map(str::to_owned).collect(),
    };
    let seeds = parse_seeds(crate::flag(args, "--seeds").unwrap_or("0..0"))?;
    let cap = crate::int_flag(args, "--cap", Some(50))?;
    let turns = crate::int_flag(args, "--turns", Some(24))?;
    let report_format = crate::flag(args, "--format") == Some("report");
    let jobs = usize::try_from(crate::int_flag(args, "--jobs", Some(1))?)
        .ok()
        .filter(|j| *j >= 1)
        .ok_or("--jobs expects a positive integer")?;
    let reg = load_register()?;

    // Shape-check every world FIRST (§1.6), ONCE. A shape divergence is one line,
    // not a hundred seeds of noise — and the check is a property of the world at
    // a freeze rev, so re-running it per (world, seed) would be ~400 frozen
    // invocations that can only ever say the same thing.
    let rev = drive_frozen::freeze_rev()?;
    let mut shapes = Vec::new();
    for w in &worlds_arg {
        let (green, lines) = shape_compare(w)?;
        shapes.push((w.clone(), green));
        if !green {
            for l in lines {
                println!("{l}");
            }
        }
    }

    let mut cells = Vec::new();
    for (w, green) in &shapes {
        if !green {
            cells.push(Cell {
                world: w.clone(),
                outcome: Outcome::ShapeDivergent {
                    fields: vec!["worldshape".to_owned()],
                    detail: Vec::new(),
                },
            });
            continue;
        }
        let shape = Shape::Green(rev.clone());
        // The trace walk once per world, then the randtrace walk per seed.
        let trace = RunSpec {
            world: w.clone(),
            walk: Walk::Trace,
            steps: turns,
            seed: None,
            die_seed: None,
            depth: 2,
            idle: worlds::idler(w).map(str::to_owned),
            mode: Mode::State,
            emit: Emit::matrix(),
        };
        let (o, _) = run_one_behind(&trace, &reg, &shape)?;
        println!("{}", line(w, None, "trace", &o));
        cells.push(Cell {
            world: w.clone(),
            outcome: o,
        });
        let specs: Vec<RunSpec> = seeds
            .iter()
            .map(|s| RunSpec {
                walk: Walk::Randtrace,
                steps: cap,
                seed: Some(*s),
                ..trace.clone()
            })
            .collect();
        for (spec, o) in specs.iter().zip(run_seeds(&specs, &reg, &shape, jobs)?) {
            println!("{}", line(w, spec.seed, "randtrace", &o));
            cells.push(Cell {
                world: w.clone(),
                outcome: o,
            });
        }
    }

    if report_format {
        println!();
        for l in report_block(&cells) {
            println!("{l}");
        }
    }
    Ok(!cells.iter().any(|c| c.outcome.is_failure()))
}

/// Run one world's seeds across `jobs` threads, returning the outcomes IN SEED
/// ORDER (§1.8).
///
/// The frozen invocations are the bottleneck and each is an independent
/// subprocess; the oracle cache is keyed by freeze rev and argv, and distinct
/// seeds produce distinct argv, so two workers never contend for one entry. The
/// results are collected and printed in order rather than as they land — a
/// matrix whose line order depended on scheduling could not be diffed between
/// runs, and stage reports embed its output verbatim.
///
/// # Errors
/// The FIRST error in seed order, so `--jobs` cannot change which failure a run
/// reports.
pub fn run_seeds(
    specs: &[RunSpec],
    reg: &Register,
    shape: &Shape,
    jobs: usize,
) -> Result<Vec<Outcome>, String> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let done: std::sync::Mutex<Vec<(usize, Result<Outcome, String>)>> =
        std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..jobs.min(specs.len()) {
            let (next, done) = (&next, &done);
            scope.spawn(move || {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some(spec) = specs.get(i) else { return };
                    let r = run_one_behind(spec, reg, shape).map(|(o, _)| o);
                    done.lock().expect("the results lock").push((i, r));
                }
            });
        }
    });
    let mut landed = done.into_inner().expect("the results lock");
    landed.sort_by_key(|(i, _)| *i);
    assert_eq!(
        landed.len(),
        specs.len(),
        "a worker dropped a seed: {} results for {} seeds",
        landed.len(),
        specs.len()
    );
    landed.into_iter().map(|(_, r)| r).collect()
}

fn line(world: &str, seed: Option<i64>, walk: &str, o: &Outcome) -> String {
    let seed = seed.map_or_else(|| "-".to_owned(), |s| s.to_string());
    let detail = match o {
        Outcome::Divergent(d) => format!(
            "  {} @ord {} (t {:?}/{:?}) fields {:?}",
            d.verdict.class.as_str(),
            d.ordinal,
            d.t.0,
            d.t.1,
            d.diff.field_names()
        ),
        Outcome::CleanModAdjudicated { ids } => format!("  {ids:?}"),
        _ => String::new(),
    };
    format!("{world:<10} {walk:<10} seed {seed:>4}  {:<22}{detail}", o.cell())
}

/// The per-world block a stage report embeds VERBATIM.
fn report_block(cells: &[Cell]) -> Vec<String> {
    let mut out = vec![
        "| world | clean | clean-mod-adjudicated | DIVERGENT | SHAPE-DIVERGENT |".to_owned(),
        "|---|---|---|---|---|".to_owned(),
    ];
    let mut worlds: Vec<&String> = cells.iter().map(|c| &c.world).collect();
    worlds.sort();
    worlds.dedup();
    for w in worlds {
        let mine: Vec<&Cell> = cells.iter().filter(|c| &c.world == w).collect();
        let count = |f: fn(&Outcome) -> bool| mine.iter().filter(|c| f(&c.outcome)).count();
        out.push(format!(
            "| {w} | {} | {} | {} | {} |",
            count(|o| matches!(o, Outcome::Clean)),
            count(|o| matches!(o, Outcome::CleanModAdjudicated { .. })),
            count(|o| matches!(o, Outcome::Divergent(_))),
            count(|o| matches!(o, Outcome::ShapeDivergent { .. })),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_ranges_are_inclusive() {
        assert_eq!(parse_seeds("0..3").expect("parses"), vec![0, 1, 2, 3]);
        assert!(parse_seeds("0-3").is_err());
    }

    #[test]
    fn the_report_block_counts_every_cell_exactly_once() {
        let cells = vec![
            Cell {
                world: "probe".into(),
                outcome: Outcome::Clean,
            },
            Cell {
                world: "probe".into(),
                outcome: Outcome::Clean,
            },
            Cell {
                world: "probe".into(),
                outcome: Outcome::CleanModAdjudicated {
                    ids: vec!["DIV-9".into()],
                },
            },
        ];
        let b = report_block(&cells);
        assert_eq!(b[2], "| probe | 2 | 1 | 0 | 0 |");
    }
}
