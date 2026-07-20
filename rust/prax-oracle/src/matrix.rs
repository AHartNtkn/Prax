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
//! retyped is a number that can drift from the run that produced it. The block
//! carries RECORDS COMPARED per world alongside the cell counts, because §4's
//! budget is denominated in effective turns and a seed count cannot be converted
//! to one without knowing how far each walk got.
//!
//! `--min-records N` makes that floor the parameter instead of a stated seed
//! count [I2]. Worlds differ by 3x in records per seed — feud dead-ends at 8
//! (alice makes amends and the feud dissolves), bigfeud runs 24 — so one seed
//! range gives two worlds wildly different coverage, and "≥100 seeds" clears
//! §4's 3,000-record floor for neither. Under `--min-records` each world's seed
//! range is EXTENDED, in batches sized from the records per seed that world has
//! actually produced, until it clears the floor. Nobody retypes a per-world seed
//! count, and the floor is checked rather than assumed.
//!
//! **But records are not coverage** [I1]. The slice-2 review measured intrigue's
//! 375-seed sweep replaying exactly FOUR distinct walks ~94 times each: the
//! record count rose with every seed while the distinct work stayed at 34
//! records, so a floor denominated in records certified duplication. Slice 1's
//! own diagnosis had already said so — *branching factor, not seeds or cap, sets
//! coverage* — and the amendment then gated on the metric that responds to
//! replaying one walk. So every cell now carries its WALK IDENTITY
//! ([`crate::compare::walk_identity`]), the report prints DISTINCT WALKS beside
//! records, and extension stops at whichever comes first: the record floor, or
//! SATURATION — no new distinct walk in [`saturation_run`] consecutive seeds.
//! The report says which stopped it, so "375 seeds, 3,025 records, 4 distinct
//! walks" cannot be read as coverage.
//!
//! **And no reported quantity may overstate its own basis** [I1, third
//! recurrence]. Slice 1 printed the requested step count as the compared count;
//! slice 2 let a record count certify duplication as coverage; slice 3 printed
//! `min-records 3000 cleared` for a sweep in which the extension loop never ran —
//! the declared range already exceeded the floor, so the floor stopped nothing
//! and the number read as derived when it was chosen. Two structures answer the
//! class rather than the instance: [`Provenance`]/[`Reported`], which make a
//! number unrenderable without saying where it came from, and
//! [`provenance_violations`], which re-checks the finished block before it is
//! printed. The block also LEADS WITH ITS OWN INVOCATION, because a block whose
//! seed range is invisible cannot be reproduced by the reader §1.8 wrote the
//! no-hand-typed-numbers rule for.

use crate::classify::{Shape, Walk};
use crate::compare::Outcome;
use crate::record::{Emit, Mode};
use crate::register::Register;
use crate::{Run, RunSpec, drive_frozen, load_register, run_one_behind, shape_compare, worlds};

/// One (world, seed) result. `seed` is `None` for the world's single trace cell.
struct Cell {
    world: String,
    seed: Option<i64>,
    outcome: Outcome,
    /// The walk this cell walked ([`crate::compare::walk_identity`]) — the unit
    /// the distinct-walk count is taken over [I1].
    walk: String,
}

/// Where a number in a report block came from.
///
/// **Why this is a type and not a convention.** Three slices running, a reported
/// quantity was sourced from the operator's request and printed as though the run
/// had derived it: slice-1 [I3] printed the REQUESTED step count as the compared
/// count; slice-2 [I1] let a record count certify duplication as happily as
/// coverage; slice-3 [I1] named the record floor as the stop reason when the
/// extension loop never ran and the requested range is what ended the sweep.
/// Three recurrences is a structural defect, not three lapses of attention — so
/// provenance rides the value. [`Reported`] is the only way a number reaches a
/// block and it cannot render without its tag, and
/// [`provenance_violations`] re-checks the finished block before it is
/// printed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Provenance {
    /// Counted from cells that actually ran.
    Measured,
    /// NOT counted: it came from the invocation, or from a constant stated in
    /// this file. A chosen number says nothing about what the run found.
    Chosen,
}

impl Provenance {
    /// The tag that must appear beside every number of this provenance.
    fn tag(self) -> &'static str {
        match self {
            Provenance::Measured => "measured",
            Provenance::Chosen => "chosen",
        }
    }

    /// The two tags, for the block scanner.
    const BOTH: [Provenance; 2] = [Provenance::Measured, Provenance::Chosen];
}

/// A number on its way into a report block, carrying where it came from.
///
/// The `Display` impl is the enforcement: there is no way to render the value
/// without the tag, so a free-text report cell cannot name a number and leave the
/// reader to guess whether the run measured it.
struct Reported<T: std::fmt::Display>(T, Provenance);

impl<T: std::fmt::Display> std::fmt::Display for Reported<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.0, self.1.tag())
    }
}

/// What stopped a world's seed extension — printed in the report block, because
/// "375 seeds" means one thing when the record floor stopped it and another when
/// saturation did.
enum BudgetStop {
    /// No `--min-records`: exactly the seeds that were asked for.
    SeedsAsRequested,
    /// The seed range was EXTENDED, and the extension stopped when the record
    /// floor cleared.
    MinRecords(usize),
    /// The requested seed range already cleared the floor, so the extension loop
    /// never ran and the REQUEST is what ended the sweep [I1]. Carries the floor,
    /// the seed at which the running record count first met it, and how many
    /// further seeds the operator asked for beyond that point.
    RequestedExceedsFloor {
        /// The `--min-records` floor.
        floor: usize,
        /// The seed at which the running record count first met the floor.
        cleared_at_seed: i64,
        /// Seeds run after the floor was already met — operator-requested
        /// surplus, not budget the harness derived.
        surplus_seeds: usize,
    },
    /// No new distinct walk in this many consecutive seeds.
    Saturated(usize),
    /// A divergence — the finding, not the budget, ended the sweep.
    Divergent,
}

impl BudgetStop {
    fn describe(&self) -> String {
        match self {
            BudgetStop::SeedsAsRequested => {
                "--seeds as requested; no record floor was asked for".to_owned()
            }
            BudgetStop::MinRecords(n) => format!(
                "the seed range was EXTENDED until min-records {} cleared",
                Reported(n, Provenance::Chosen)
            ),
            BudgetStop::RequestedExceedsFloor {
                floor,
                cleared_at_seed,
                surplus_seeds: 0,
            } => format!(
                "the REQUESTED range ended the sweep, not the floor: min-records {} cleared at \
                 seed {}, the last seed asked for — no extension ran",
                Reported(floor, Provenance::Chosen),
                Reported(cleared_at_seed, Provenance::Measured)
            ),
            BudgetStop::RequestedExceedsFloor {
                floor,
                cleared_at_seed,
                surplus_seeds,
            } => format!(
                "the REQUESTED range ended the sweep, not the floor: min-records {} cleared at \
                 seed {}, and the further {} seeds are operator-requested surplus — no extension \
                 ran",
                Reported(floor, Provenance::Chosen),
                Reported(cleared_at_seed, Provenance::Measured),
                Reported(surplus_seeds, Provenance::Measured)
            ),
            BudgetStop::Saturated(n) => format!(
                "saturated: no new walk in {} consecutive seeds",
                Reported(n, Provenance::Chosen)
            ),
            BudgetStop::Divergent => {
                "DIVERGENT — the finding, not the budget, ended the sweep".to_owned()
            }
        }
    }
}

/// A walk reachable from at least this share of seeds is one a sweep must not
/// miss. Stated, not tuned: 1% of a several-hundred-seed sweep is the coarsest
/// branch anybody would call rare, and the criterion below is derived from it
/// rather than from a round number of seeds.
const WALK_MISS_PROB: f64 = 0.01;
/// The confidence that no such walk was missed.
const WALK_CONFIDENCE: f64 = 0.95;

/// How many CONSECUTIVE seeds must add no new distinct walk before a world is
/// called saturated.
///
/// Treating a seed as an independent draw from the world's walk distribution, a
/// walk of probability `p` survives `n` draws unseen with probability
/// `(1 - p)^n`; requiring that to be at most `1 - c` gives
/// `n = ceil(ln(1 - c) / ln(1 - p))` = **299** at `p = 1%`, `c = 95%`. That is
/// the whole justification: no round number is chosen, and both inputs are named
/// above so a reader can disagree with the criterion rather than with a
/// constant. `--walk-saturation N` overrides it for an operator who wants a
/// different claim.
pub fn saturation_run() -> usize {
    let n = (1.0 - WALK_CONFIDENCE).ln() / (1.0 - WALK_MISS_PROB).ln();
    n.ceil() as usize
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
    // [I2] The §4 budget is denominated in effective RECORDS, not seeds. Worlds
    // differ by 3x in records per seed (feud dead-ends at 8; bigfeud runs 24), so
    // one seed range cannot give two worlds the same coverage and a stated seed
    // floor is not the operative gate. `--min-records N` makes the floor the
    // parameter: each world's seed range is EXTENDED until it clears N.
    let min_records = match crate::flag(args, "--min-records") {
        Some(_) => Some(usize::try_from(crate::int_flag(args, "--min-records", None)?)
            .map_err(|_| "--min-records expects a non-negative integer".to_owned())?),
        None => None,
    };
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

    let sat = match crate::flag(args, "--walk-saturation") {
        Some(_) => usize::try_from(crate::int_flag(args, "--walk-saturation", None)?)
            .map_err(|_| "--walk-saturation expects a non-negative integer".to_owned())?,
        None => saturation_run(),
    };

    let mut cells = Vec::new();
    let mut stops: Vec<(String, BudgetStop)> = Vec::new();
    for (w, green) in &shapes {
        if !green {
            cells.push(Cell {
                world: w.clone(),
                seed: None,
                outcome: Outcome::ShapeDivergent {
                    fields: vec!["worldshape".to_owned()],
                    detail: Vec::new(),
                },
                walk: String::new(),
            });
            stops.push((w.clone(), BudgetStop::Divergent));
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
        let r = run_one_behind(&trace, &reg, &shape)?;
        println!("{}", line(w, None, "trace", &r.outcome, false));
        cells.push(Cell {
            world: w.clone(),
            seed: None,
            outcome: r.outcome,
            walk: r.walk,
        });
        // The declared seeds first, then — under `--min-records` — as many more
        // as the floor needs. Each extension is sized from the records per seed
        // this world has ACTUALLY produced, so a world that dead-ends early gets
        // proportionally more seeds and nobody retypes a per-world count.
        let mut batch: Vec<i64> = seeds.clone();
        let mut next_seed = seeds.last().map_or(0, |s| s + 1);
        let (mut seeds_run, mut rand_records) = (0usize, 0usize);
        let mut failed = false;
        // [I1] The coverage accounting: the distinct walks seen so far, and how
        // many consecutive seeds have added none. `since_new` counts SEEDS in
        // seed order, which is the unit the criterion is stated in.
        let mut walks: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut since_new = 0usize;
        // [I1] Whether any batch beyond the DECLARED seed range ever ran, and the
        // (seed, seeds-run-so-far) at which the running record count first met the
        // floor. Without these the loop cannot tell "the floor stopped the sweep"
        // from "the request already exceeded the floor and the floor stopped
        // nothing" — and it printed the first when it meant the second.
        let mut extended = false;
        let mut cleared: Option<(i64, usize)> = None;
        let stop = loop {
            let specs: Vec<RunSpec> = batch
                .iter()
                .map(|s| RunSpec {
                    walk: Walk::Randtrace,
                    steps: cap,
                    seed: Some(*s),
                    ..trace.clone()
                })
                .collect();
            for (spec, r) in specs.iter().zip(run_seeds(&specs, &reg, &shape, jobs)?) {
                let fresh = !r.walk.is_empty() && walks.insert(r.walk.clone());
                since_new = if fresh { 0 } else { since_new + 1 };
                println!("{}", line(w, spec.seed, "randtrace", &r.outcome, fresh));
                seeds_run += 1;
                rand_records += r.outcome.records_compared();
                if let Some(floor) = min_records
                    && cleared.is_none()
                    && rand_records >= floor
                {
                    cleared = Some((spec.seed.unwrap_or(0), seeds_run));
                }
                failed |= r.outcome.is_failure();
                cells.push(Cell {
                    world: w.clone(),
                    seed: spec.seed,
                    outcome: r.outcome,
                    walk: r.walk,
                });
            }
            let Some(floor) = min_records else { break BudgetStop::SeedsAsRequested };
            if failed {
                // A divergence is the finding; grinding out more seeds behind one
                // would bury it.
                break BudgetStop::Divergent;
            }
            // Saturation is checked BEFORE the record floor: past it every
            // further seed is a replay, so a floor cleared by replays is not a
            // floor anybody should read as coverage.
            if since_new >= sat {
                break BudgetStop::Saturated(sat);
            }
            let Some(needed) = seeds_still_needed(rand_records, seeds_run, floor)
                .map_err(|e| format!("matrix {w}: {e}"))?
            else {
                // [I1] The floor is met — but WHAT MET IT decides what the column
                // may say. If no extension ever ran, the declared range is what
                // ended the sweep and the floor stopped nothing; naming the floor
                // there prints a stop that did not happen.
                break match cleared {
                    Some((seed, at)) if !extended => BudgetStop::RequestedExceedsFloor {
                        floor,
                        cleared_at_seed: seed,
                        surplus_seeds: seeds_run - at,
                    },
                    _ => BudgetStop::MinRecords(floor),
                };
            };
            // Never extend past the point where saturation would fire mid-batch:
            // the criterion counts consecutive seeds, and a batch that runs
            // hundreds of seeds beyond it would report a stop that already
            // happened.
            let needed = needed.min(sat - since_new);
            let needed = i64::try_from(needed).map_err(|_| {
                format!("matrix --min-records {floor}: {w} needs more seeds than fit in i64")
            })?;
            batch = (next_seed..next_seed + needed).collect();
            next_seed += needed;
            extended = true;
        };
        stops.push((w.clone(), stop));
    }

    if report_format {
        println!();
        let block = report_block(&cells, &stops, args);
        // The class-level guard, live rather than only in the test suite: a block
        // that names a number without saying where it came from is a comparator
        // bug, and it says so instead of being embedded in a stage report [I1].
        let bad = provenance_violations(&block);
        if !bad.is_empty() {
            return Err(format!(
                "the report block names a quantity without its provenance — a comparator bug, not \
                 a run result (three slices running, a report overstated its own basis):\n  {}",
                bad.join("\n  ")
            ));
        }
        for l in block {
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
) -> Result<Vec<Run>, String> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let done: std::sync::Mutex<Vec<(usize, Result<Run, String>)>> =
        std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..jobs.min(specs.len()) {
            let (next, done) = (&next, &done);
            scope.spawn(move || {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some(spec) = specs.get(i) else { return };
                    let r = run_one_behind(spec, reg, shape);
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

/// How many FURTHER seeds a world needs to clear `floor` records, sized from the
/// records per seed it has already produced. `Ok(None)` when the floor is met.
///
/// The estimate rounds records-per-seed DOWN (a seed that produced fewer records
/// than the running mean must not make the next batch too small) and the seed
/// count UP, and the caller loops — so an underestimate costs another batch, not
/// a silently-short run.
///
/// # Errors
/// If seeds were run and produced no records at all. Every walk emits at least
/// its terminal stop record, so this cannot happen; if it does, no number of
/// further seeds could reach the floor and the run says so instead of looping.
fn seeds_still_needed(records: usize, seeds_run: usize, floor: usize) -> Result<Option<usize>, String> {
    if records >= floor {
        return Ok(None);
    }
    let per_seed = records / seeds_run.max(1);
    if per_seed == 0 {
        return Err(format!(
            "--min-records {floor}: {seeds_run} seed(s) produced {records} record(s) — fewer than \
             one per seed, so no number of further seeds can reach the floor. Every walk emits a \
             terminal stop record, so this is a comparator bug."
        ));
    }
    Ok(Some((floor - records).div_ceil(per_seed)))
}

fn line(world: &str, seed: Option<i64>, walk: &str, o: &Outcome, fresh: bool) -> String {
    let seed = seed.map_or_else(|| "-".to_owned(), |s| s.to_string());
    let fresh = if fresh { "  NEW-WALK" } else { "" };
    let detail = match o {
        Outcome::Divergent(d) => format!(
            "  {} @ord {} (t {:?}/{:?}) fields {:?}",
            d.verdict.class.as_str(),
            d.ordinal,
            d.t.0,
            d.t.1,
            d.diff.field_names()
        ),
        Outcome::CleanModAdjudicated { ids, .. } => format!("  {ids:?}"),
        _ => String::new(),
    };
    format!(
        "{world:<10} {walk:<10} seed {seed:>4}  {:<22}{:>5} rec{fresh}{detail}",
        o.cell(),
        o.records_compared()
    )
}

/// The numeric columns of the per-world block, in order, each with the
/// provenance of every value that can appear under it. The header is BUILT from
/// this list, so a column cannot be added without stating where its numbers come
/// from, and [`provenance_violations`] checks the rendered header against it.
const COLUMNS: [(&str, Provenance); 8] = [
    ("randtrace seeds", Provenance::Measured),
    ("cells", Provenance::Measured),
    ("clean", Provenance::Measured),
    ("clean-mod-adjudicated", Provenance::Measured),
    ("DIVERGENT", Provenance::Measured),
    ("SHAPE-DIVERGENT", Provenance::Measured),
    ("records compared", Provenance::Measured),
    ("distinct walks", Provenance::Measured),
];

/// The prefix of the invocation line. The line carries the request VERBATIM, so
/// it is wholly chosen and labels itself as such.
const INVOCATION_PREFIX: &str = "invocation (chosen, verbatim): prax-oracle matrix";

/// One argument, safe to paste back into a shell.
fn shell_quote(arg: &str) -> String {
    let plain = |c: char| c.is_ascii_alphanumeric() || "_,.=:/+@-".contains(c);
    if !arg.is_empty() && arg.chars().all(plain) {
        arg.to_owned()
    } else {
        format!("'{}'", arg.replace('\'', r"'\''"))
    }
}

/// The per-world block a stage report embeds VERBATIM.
///
/// `distinct walks` is counted over the RANDTRACE cells only — the one trace
/// cell is a different walk kind, and the column exists to say what the seed
/// sweep bought. `budget stop` names what ended the extension, because "375
/// seeds" means one thing when the record floor stopped it and another when
/// saturation did [I1].
///
/// `cells` is the block's own BASIS, and it is why the outcome columns can be
/// read at all [slice-4 review M3]: `clean`/`clean-mod-adjudicated`/`DIVERGENT`/
/// `SHAPE-DIVERGENT` count CELLS — the trace walk plus every randtrace seed —
/// while `randtrace seeds` counts seeds. Without a cell count in the row, a
/// clean sweep prints one more `clean` than it has seeds, and a row whose count
/// exceeds its own visible basis is precisely the misreading §13(3) exists to
/// prevent.
///
/// The block LEADS WITH THE INVOCATION THAT PRODUCED IT [I1]. §1.8's rule — no
/// hand-typed matrix numbers, ever — exists so a reader can reproduce the block;
/// a block whose seed range, cap and floor are invisible in it cannot be
/// reproduced, and the slice-3 block could not be: it was read as the documented
/// 0..99 sweep and was in fact a 0..299 one.
fn report_block(cells: &[Cell], stops: &[(String, BudgetStop)], argv: &[&str]) -> Vec<String> {
    let mut invocation = INVOCATION_PREFIX.to_owned();
    for a in argv {
        invocation.push(' ');
        invocation.push_str(&shell_quote(a));
    }
    let header: String = std::iter::once("| world".to_owned())
        .chain(COLUMNS.iter().map(|(n, p)| format!(" | {n} ({})", p.tag())))
        .chain(std::iter::once(" | budget stop |".to_owned()))
        .collect();
    let mut out = vec![
        invocation,
        String::new(),
        header,
        "|---|---|---|---|---|---|---|---|---|---|".to_owned(),
    ];
    let mut worlds: Vec<&String> = cells.iter().map(|c| &c.world).collect();
    worlds.sort();
    worlds.dedup();
    for w in worlds {
        let mine: Vec<&Cell> = cells.iter().filter(|c| &c.world == w).collect();
        let count = |f: fn(&Outcome) -> bool| mine.iter().filter(|c| f(&c.outcome)).count();
        let distinct: std::collections::BTreeSet<&String> = mine
            .iter()
            .filter(|c| c.seed.is_some() && !c.walk.is_empty())
            .map(|c| &c.walk)
            .collect();
        let stop = stops
            .iter()
            .find(|(sw, _)| sw == w)
            .map_or_else(|| "-".to_owned(), |(_, s)| s.describe());
        out.push(format!(
            "| {w} | {} | {} | {} | {} | {} | {} | {} | {} | {stop} |",
            mine.iter().filter(|c| c.seed.is_some()).count(),
            mine.len(),
            count(|o| matches!(o, Outcome::Clean { .. })),
            count(|o| matches!(o, Outcome::CleanModAdjudicated { .. })),
            count(|o| matches!(o, Outcome::Divergent(_))),
            count(|o| matches!(o, Outcome::ShapeDivergent { .. })),
            mine.iter().map(|c| c.outcome.records_compared()).sum::<usize>(),
            distinct.len(),
        ));
    }
    out
}

/// THE CLASS-LEVEL GUARD [I1]: every quantity a report block prints must be
/// attributable, from inside the block, to either the run or the request.
///
/// The rule, stated once so it can be checked rather than remembered:
///
/// 1. The **invocation line** is the request verbatim and says so in its prefix.
/// 2. Every **numeric column** carries its provenance in the HEADER — built from
///    [`COLUMNS`], so a new column must declare one — and its body cells are bare
///    integers covered by that tag.
/// 3. The **`budget stop`** cell is free text, so every number in it must be
///    tagged WHERE IT STANDS: each run of digits is immediately followed by
///    `(measured)` or `(chosen)`. [`Reported`] is the only renderer that produces
///    that shape, which is what makes the rule cheap to obey.
///
/// A number that satisfies none of these is unattributed, and an unattributed
/// number is exactly what let three consecutive slices report a basis they did
/// not have. Returns one message per violation; empty means the block is sound.
fn provenance_violations(block: &[String]) -> Vec<String> {
    let mut bad = Vec::new();
    for (i, line) in block.iter().enumerate() {
        let at = |m: String| format!("line {i}: {m} — in `{line}`");
        if line.is_empty() || line.starts_with(INVOCATION_PREFIX) || line.starts_with("|---") {
            continue;
        }
        let Some(cells) = split_row(line) else {
            bad.push(at("not a table row, an invocation line or a separator".to_owned()));
            continue;
        };
        if cells.len() != COLUMNS.len() + 2 {
            bad.push(at(format!(
                "{} cells for {} columns",
                cells.len(),
                COLUMNS.len() + 2
            )));
            continue;
        }
        let is_header = cells[0] == "world";
        for (cell, (name, prov)) in cells[1..=COLUMNS.len()].iter().zip(COLUMNS) {
            if is_header {
                if *cell != format!("{name} ({})", prov.tag()) {
                    bad.push(at(format!(
                        "column header `{cell}` does not declare its provenance as \
                         `{name} ({})`",
                        prov.tag()
                    )));
                }
            } else if cell.parse::<u64>().is_err() {
                bad.push(at(format!(
                    "column `{name}` is declared {} but its cell `{cell}` is not a bare count",
                    prov.tag()
                )));
            }
        }
        // The free-text stop cell: every number tagged where it stands.
        let stop = cells[COLUMNS.len() + 1];
        if is_header {
            if stop != "budget stop" {
                bad.push(at(format!("last column is `{stop}`, not `budget stop`")));
            }
            continue;
        }
        for n in untagged_numbers(stop) {
            bad.push(at(format!(
                "the budget-stop cell names `{n}` without saying whether it was measured or \
                 chosen"
            )));
        }
    }
    bad
}

/// A markdown row's cells, trimmed. `None` if it is not a `|`-delimited row.
fn split_row(line: &str) -> Option<Vec<&str>> {
    let inner = line.strip_prefix('|')?.strip_suffix('|')?;
    Some(inner.split('|').map(str::trim).collect())
}

/// Every run of digits in `s` NOT immediately followed by a provenance tag.
fn untagged_numbers(s: &str) -> Vec<&str> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if !b[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        let tagged = Provenance::BOTH
            .iter()
            .any(|p| s[i..].starts_with(&format!(" ({})", p.tag())));
        if !tagged {
            out.push(&s[start..i]);
        }
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
    fn the_seed_extension_is_sized_from_the_records_per_seed_actually_observed() {
        // [I2]. The two shipped slice-1 worlds differ by 3x in records per seed:
        // feud dead-ends at 8 (alice makes amends), bigfeud runs 24. One seed
        // range therefore cannot give both §4's 3,000-record floor, which is why
        // the floor is the parameter and the seeds are derived from it.
        assert_eq!(
            seeds_still_needed(800, 100, 3000).expect("8 records/seed"),
            Some(275),
            "feud: 800 records over 100 seeds is 8/seed, and 2,200 more needs 275 seeds"
        );
        assert_eq!(
            seeds_still_needed(2400, 100, 3000).expect("24 records/seed"),
            Some(25),
            "bigfeud: 2,400 over 100 seeds is 24/seed, and 600 more needs 25 seeds"
        );
        assert_eq!(
            seeds_still_needed(3000, 375, 3000).expect("met"),
            None,
            "a met floor asks for no further seeds"
        );
        // A ragged mean rounds the per-seed rate DOWN, so the next batch is never
        // too small; an overshoot is one extra seed, an undershoot is a short run.
        assert_eq!(seeds_still_needed(23, 3, 30).expect("7/seed"), Some(1));
        assert!(
            seeds_still_needed(0, 4, 10).is_err(),
            "zero records over four seeds can never reach a positive floor"
        );
    }

    fn cell(seed: Option<i64>, outcome: Outcome, walk: &str) -> Cell {
        Cell {
            world: "probe".into(),
            seed,
            outcome,
            walk: walk.to_owned(),
        }
    }

    /// The block's first data row. Line 0 is the invocation, 1 is blank, 2 the
    /// header, 3 the separator.
    const ROW: usize = 4;

    #[test]
    fn the_report_block_counts_every_cell_exactly_once() {
        let cells = vec![
            cell(None, Outcome::Clean { records: 8 }, "trace-walk"),
            cell(Some(0), Outcome::Clean { records: 8 }, "A|B|end"),
            cell(
                Some(1),
                Outcome::CleanModAdjudicated {
                    ids: vec!["DIV-9".into()],
                    records: 8,
                },
                "A|B|end",
            ),
        ];
        let stops = vec![("probe".to_owned(), BudgetStop::MinRecords(16))];
        let b = report_block(&cells, &stops, &[]);
        assert_eq!(
            b[ROW],
            "| probe | 2 | 3 | 2 | 1 | 0 | 0 | 24 | 1 | the seed range was EXTENDED until min-records \
             16 (chosen) cleared |"
        );
    }

    #[test]
    fn the_block_carries_the_invocation_that_produced_it() {
        // [I1] The slice-3 block was read as the documented `0..99` sweep and was
        // in fact a `0..299` one; nothing in the block said which. Now it does,
        // verbatim and shell-pasteable.
        let b = report_block(&[], &[], &[
            "--worlds",
            "bar,dm",
            "--seeds",
            "0..299",
            "--cap",
            "50",
            "--min-records",
            "3000",
            "--format",
            "report",
        ]);
        assert_eq!(
            b[0],
            "invocation (chosen, verbatim): prax-oracle matrix --worlds bar,dm --seeds 0..299 \
             --cap 50 --min-records 3000 --format report"
        );
        assert_eq!(shell_quote("a b"), "'a b'", "an argument with a space is quoted");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }

    #[test]
    fn a_sweep_whose_requested_range_already_cleared_the_floor_says_the_floor_stopped_nothing() {
        // [I1] THE INSTANCE. `--seeds 0..299 --min-records 3000` on bar: the floor
        // is met around seed 45 and the extension loop never runs, so naming the
        // floor as the stop names a stop that did not happen.
        let stop = BudgetStop::RequestedExceedsFloor {
            floor: 3000,
            cleared_at_seed: 45,
            surplus_seeds: 254,
        };
        assert_eq!(
            stop.describe(),
            "the REQUESTED range ended the sweep, not the floor: min-records 3000 (chosen) \
             cleared at seed 45 (measured), and the further 254 (measured) seeds are \
             operator-requested surplus — no extension ran"
        );
        // The boundary: the floor clears exactly at the last requested seed, so
        // there is no surplus — and it is still the request that ended the sweep.
        let exact = BudgetStop::RequestedExceedsFloor {
            floor: 3000,
            cleared_at_seed: 299,
            surplus_seeds: 0,
        };
        assert!(
            exact.describe().contains("the last seed asked for — no extension ran"),
            "{}",
            exact.describe()
        );
        // And the extended case still says the floor stopped it, because there it
        // did.
        assert!(
            BudgetStop::MinRecords(3000)
                .describe()
                .contains("the seed range was EXTENDED until min-records 3000 (chosen) cleared")
        );
    }

    #[test]
    fn every_quantity_a_report_block_prints_is_attributable_to_the_run_or_the_request() {
        // THE CLASS-LEVEL GUARD [I1], over EVERY `BudgetStop` variant — so a new
        // stop reason cannot reach a stage report naming an untagged number.
        let cells = vec![
            cell(None, Outcome::Clean { records: 8 }, "trace-walk"),
            cell(Some(0), Outcome::Clean { records: 8 }, "A|B|end"),
        ];
        for stop in [
            BudgetStop::SeedsAsRequested,
            BudgetStop::MinRecords(3000),
            BudgetStop::RequestedExceedsFloor {
                floor: 3000,
                cleared_at_seed: 45,
                surplus_seeds: 254,
            },
            BudgetStop::RequestedExceedsFloor {
                floor: 3000,
                cleared_at_seed: 299,
                surplus_seeds: 0,
            },
            BudgetStop::Saturated(299),
            BudgetStop::Divergent,
        ] {
            let rendered = stop.describe();
            let b = report_block(&cells, &[("probe".to_owned(), stop)], &[
                "--seeds", "0..299",
            ]);
            assert!(
                provenance_violations(&b).is_empty(),
                "unattributed quantity under `{rendered}`: {:?}",
                provenance_violations(&b)
            );
        }
        // The header declares a provenance for every numeric column.
        let b = report_block(&cells, &[], &[]);
        assert_eq!(
            b[2],
            "| world | randtrace seeds (measured) | cells (measured) | \
             clean (measured) | clean-mod-adjudicated (measured) | \
             DIVERGENT (measured) | SHAPE-DIVERGENT (measured) | \
             records compared (measured) | distinct walks (measured) | budget stop |"
        );
    }

    #[test]
    fn the_guard_catches_the_three_ways_a_block_has_actually_overstated_its_basis() {
        // The guard's own RED set — a guard nobody has seen fail is a guard nobody
        // knows works. Each string below is the shape of a real recurrence.
        let header = report_block(&[], &[], &[])[2].clone();
        let sep = "|---|---|---|---|---|---|---|---|---|---|".to_owned();
        let sound = "| probe | 2 | 3 | 2 | 0 | 0 | 0 | 24 | 1 | saturated: no new walk in \
                     299 (chosen) consecutive seeds |"
            .to_owned();
        assert!(provenance_violations(&[header.clone(), sep.clone(), sound]).is_empty());

        // 1. Slice-3's shape: a free-text stop naming a number with no source.
        let untagged = "| probe | 2 | 3 | 2 | 0 | 0 | 0 | 24 | 1 | min-records 3000 cleared |";
        let v = provenance_violations(&[header.clone(), sep.clone(), untagged.to_owned()]);
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(v[0].contains("names `3000` without saying whether it was measured or chosen"));

        // 2. A column header that drops its tag — the way a new column would
        //    arrive undeclared.
        let stripped = header.replace("distinct walks (measured)", "distinct walks");
        let v = provenance_violations(&[stripped, sep.clone()]);
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(v[0].contains("does not declare its provenance"));

        // 3. A numeric column carrying prose instead of a count — the way a
        //    requested value sneaks in dressed as a measurement.
        let prose = "| probe | 2 | 3 | 2 | 0 | 0 | 0 | as requested | 1 | \
                     --seeds as requested; no record floor was asked for |";
        let v = provenance_violations(&[header, sep, prose.to_owned()]);
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(v[0].contains("is not a bare count"));
    }

    #[test]
    fn the_distinct_walk_column_counts_walks_and_not_seeds() {
        // [I1] The finding this column exists for, in miniature: three seeds,
        // 24 records, ONE walk. The records column is the same number a sweep
        // with three genuinely different walks would print.
        let cells = vec![
            cell(None, Outcome::Clean { records: 8 }, "trace-walk"),
            cell(Some(0), Outcome::Clean { records: 8 }, "A|B|end"),
            cell(Some(1), Outcome::Clean { records: 8 }, "A|B|end"),
            cell(Some(2), Outcome::Clean { records: 8 }, "A|B|end"),
        ];
        let stops = vec![("probe".to_owned(), BudgetStop::Saturated(299))];
        let b = report_block(&cells, &stops, &[]);
        assert_eq!(
            b[ROW],
            "| probe | 3 | 4 | 4 | 0 | 0 | 0 | 32 | 1 | saturated: no new walk in 299 (chosen) \
             consecutive seeds |",
            "three seeds replaying one walk must report ONE distinct walk"
        );
        // The trace cell is not a randtrace walk and is not counted as one, but
        // its records are compared records and do count.
        let varied = vec![
            cell(None, Outcome::Clean { records: 8 }, "trace-walk"),
            cell(Some(0), Outcome::Clean { records: 8 }, "A|B|end"),
            cell(Some(1), Outcome::Clean { records: 8 }, "A|C|end"),
            cell(Some(2), Outcome::Clean { records: 8 }, "B|end"),
        ];
        let b = report_block(&varied, &stops, &[]);
        assert!(
            b[ROW].contains("| 32 | 3 |"),
            "three different walks over the same record count: {}",
            b[ROW]
        );
    }

    #[test]
    fn the_saturation_criterion_is_derived_from_its_two_stated_inputs() {
        // n = ceil(ln(1 - 0.95) / ln(1 - 0.01)): a walk reachable from 1% of
        // seeds survives n draws unseen with probability at most 5%.
        assert_eq!(saturation_run(), 299);
        let n = f64::from(u32::try_from(saturation_run()).expect("the run length fits"));
        assert!(
            (1.0 - WALK_MISS_PROB).powf(n) <= 1.0 - WALK_CONFIDENCE,
            "the run length must actually deliver the confidence it claims"
        );
    }

    #[test]
    fn the_walk_identity_separates_walks_that_the_record_count_cannot() {
        use serde_json::json;
        let header = json!({"world": "probe"});
        let walk = |actions: &[&str], reason: &str| {
            let mut v = vec![header.clone()];
            for a in actions {
                v.push(json!({"t": 0, "action": a}));
            }
            v.push(json!({"end": true, "reason": reason, "ending": null}));
            crate::compare::walk_identity(&v)
        };
        // Same length, different actions: two walks.
        assert_ne!(walk(&["a: bide", "b: bide"], "cap"), walk(&["a: bide", "b: act"], "cap"));
        // Same actions, different stop rule: still two walks — "both stopped" is
        // not agreement about how.
        assert_ne!(walk(&["a: bide"], "cap"), walk(&["a: bide"], "ending"));
        // The same walk from two seeds is ONE walk, which is the whole point.
        assert_eq!(walk(&["a: bide"], "cap"), walk(&["a: bide"], "cap"));
        // The header is not part of the walk.
        let mut with_other_header = vec![json!({"world": "other"})];
        with_other_header.push(json!({"end": true, "reason": "cap", "ending": null}));
        assert_eq!(
            crate::compare::walk_identity(&with_other_header),
            walk(&[], "cap")
        );
    }
}
