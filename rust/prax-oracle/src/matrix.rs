//! Matrix mode: one line per (world, seed) — the report's only source of
//! numbers.
//!
//! Worlds are shape-checked (§2) BEFORE any seed runs, so a world-port error
//! costs one line rather than a hundred localizations. Every run carries
//! `--candidates` [S-I4]: without it ENUMERATION can never fire and every
//! enumeration bug reports as DECISION.
//!
//! `--format report` emits the per-world block that stage reports embed
//! VERBATIM. Reports never carry hand-typed matrix numbers — a number a human
//! retyped is a number that can drift from the run that produced it.

use crate::classify::Walk;
use crate::compare::Outcome;
use crate::record::{Emit, Mode};
use crate::{RunSpec, load_register, run_one, shape_compare, worlds};

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
    let reg = load_register()?;

    // Shape-check every world FIRST (§1.6). A shape divergence is one line, not
    // a hundred seeds of noise.
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
        let (o, _) = run_one(&trace, &reg)?;
        println!("{}", line(w, None, "trace", &o));
        cells.push(Cell {
            world: w.clone(),
            outcome: o,
        });
        for s in &seeds {
            let spec = RunSpec {
                walk: Walk::Randtrace,
                steps: cap,
                seed: Some(*s),
                ..trace.clone()
            };
            let (o, _) = run_one(&spec, &reg)?;
            println!("{}", line(w, Some(*s), "randtrace", &o));
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
