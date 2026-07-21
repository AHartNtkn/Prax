//! # prax-oracle — the differential comparator
//!
//! The Rust side of the differential net (PLAN.md, Verification; S7 design §1).
//! It drives the frozen Haskell oracle and the Rust engine over the same walk,
//! compares them record by record, and on a divergence LOCALIZES it: the record
//! pair, the full field diff, and a triage class that says where to look.
//!
//! ```text
//! prax-oracle worldshape <world>
//! prax-oracle compare <world> --mode trace|randtrace [--turns N|--cap N]
//!                             [--seed S] [--die-seed S] [--depth D] [--emit view]
//! prax-oracle explain <world> --mode M …          (compare with everything emitted)
//! prax-oracle matrix [--worlds a,b] --seeds 0..99 --cap 50 [--jobs N]
//!                    [--min-records N] [--format report]
//! ```
//!
//! The pieces: [`drive_frozen`] (subprocess + freeze-check + the freeze-rev-keyed
//! cache), [`drive_rust`] (in process), [`record`] (the ONE record builder),
//! [`walk`] (the randtrace generator and stop rules), [`worldshape`] (the
//! world-fidelity gate), [`diff`] (the three-bucket path diff), [`classify`]
//! (the ladder), [`register`] (the adjudicated-divergence register), [`compare`]
//! (the run), [`matrix`] (one line per world×seed).

mod classify;
mod compare;
mod diff;
mod drive_frozen;
mod drive_rust;
mod matrix;
mod probe;
mod record;
mod register;
mod stress;
mod walk;
mod worldshape;
mod worlds;

use classify::{Ctx, Shape, ViewWitness, Walk};
use record::{Emit, Mode};
use register::Register;
use serde_json::Value;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();
    let r = match args.first().map(String::as_str) {
        Some("worldshape") => cmd_worldshape(&rest),
        Some("check") => cmd_check(&rest),
        Some("compare") => cmd_compare(&rest),
        Some("explain") => cmd_explain(&rest),
        Some("matrix") => matrix::run(&rest),
        _ => Err(usage()),
    };
    match r {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("prax-oracle: {e}");
            ExitCode::FAILURE
        }
    }
}

fn usage() -> String {
    format!(
        "usage:\n\
         \x20 prax-oracle worldshape <world>\n\
         \x20 prax-oracle check <world>\n\
         \x20 prax-oracle compare <world> --mode trace|randtrace [--turns N] [--cap N] [--seed S]\n\
         \x20                             [--die-seed S] [--depth D] [--emit decisions|state|view]\n\
         \x20 prax-oracle explain <world> --mode M [same flags as compare]\n\
         \x20 prax-oracle matrix [--worlds a,b,c] --seeds A..B --cap N [--jobs N]\n\
         \x20                    [--min-records N] [--format report]\n\
         \n\
         ported worlds: {}",
        worlds::ported().join(" ")
    )
}

// ---- flag helpers (loud, never a silent default for a required number) ------

fn flag<'a>(args: &[&'a str], name: &str) -> Option<&'a str> {
    args.windows(2).find(|w| w[0] == name).map(|w| w[1])
}

fn has(args: &[&str], name: &str) -> bool {
    args.contains(&name)
}

fn int_flag(args: &[&str], name: &str, default: Option<i64>) -> Result<i64, String> {
    match flag(args, name) {
        Some(s) => s
            .parse()
            .map_err(|_| format!("{name} expects an integer, got `{s}`")),
        None => default.ok_or_else(|| format!("missing required flag {name}")),
    }
}

// ---- worldshape -------------------------------------------------------------

fn cmd_worldshape(args: &[&str]) -> Result<bool, String> {
    let world = *args.first().ok_or("worldshape needs a world")?;
    let (green, lines) = shape_compare(world)?;
    for l in lines {
        println!("{l}");
    }
    Ok(green)
}

/// The Rust `check` document: `sort(map describe (type_check world))` as a JSON
/// array — byte-comparable, Value-vs-Value, against the frozen `check`
/// (`TraceMain.hs:runCheckCmd`). Shared by the CLI's `prax check` and this
/// differential.
pub fn check_json(world: &str) -> Result<Value, String> {
    let st = worlds::build(world)?;
    let mut lines: Vec<String> = prax_core::typecheck::type_check(&st)
        .iter()
        .map(prax_core::typecheck::describe)
        .collect();
    lines.sort();
    Ok(serde_json::json!(lines))
}

/// The `check` differential: the frozen `sort(map describe (typeCheck))` array
/// vs the Rust one, Value-vs-Value.
///
/// **This is a shipped-worlds `[] == []` AGREEMENT channel, not verdict
/// coverage.** Every shipped world is well-formed, so both sides are the empty
/// array; what this proves is that the shipped worlds AGREE clean, cross-engine.
/// The checker's DISCRIMINATION — one SHOULD-flag fixture per constructor, and
/// the byte-exact `describe` rendering — lives in the native pins
/// (`conformance::typecheck_spec` and `typecheck::describe_renders_…`), which are
/// not shipped worlds and never reach this comparator. Do not read a green
/// `check` differential as evidence the verdict logic is exercised.
pub fn check_compare(world: &str) -> Result<(bool, Vec<String>), String> {
    let frozen = drive_frozen::run_json(&["check".to_owned(), world.to_owned()])?;
    let rust = check_json(world)?;
    let d = diff::diff_records(&frozen, &rust);
    let rev = drive_frozen::freeze_rev()?;
    if d.is_empty() {
        Ok((true, vec![format!("check {world}: GREEN (freeze rev {rev})")]))
    } else {
        let mut out = vec![format!("check {world}: DIVERGENT (freeze rev {rev})")];
        for fd in &d.fields {
            out.extend(diff::render_field(fd));
        }
        Ok((false, out))
    }
}

fn cmd_check(args: &[&str]) -> Result<bool, String> {
    let world = *args.first().ok_or("check needs a world")?;
    let (green, lines) = check_compare(world)?;
    for l in lines {
        println!("{l}");
    }
    Ok(green)
}

/// Compare the two `worldshape` documents. Returns whether they agree and the
/// rendered report.
///
/// # Errors
/// If either side cannot produce a document (a world not yet ported, a dirty
/// frozen tree, a world whose setup draws from the die).
pub fn shape_compare(world: &str) -> Result<(bool, Vec<String>), String> {
    let frozen = drive_frozen::run_json(&["worldshape".to_owned(), world.to_owned()])?;
    let mut st = worlds::build(world)?;
    let rust = worldshape::worldshape(world, &mut st)?;
    let rev = drive_frozen::freeze_rev()?;
    let mut out = Vec::new();
    let shape_d = diff::diff_records(
        frozen.get("shape").unwrap_or(&Value::Null),
        rust.get("shape").unwrap_or(&Value::Null),
    );
    let bodies_d = diff::diff_records(
        frozen.get("bodies").unwrap_or(&Value::Null),
        rust.get("bodies").unwrap_or(&Value::Null),
    );
    if shape_d.is_empty() && bodies_d.is_empty() {
        out.push(format!("worldshape {world}: GREEN (freeze rev {rev})"));
        return Ok((true, out));
    }
    out.push(format!(
        "worldshape {world}: SHAPE-DIVERGENT (freeze rev {rev}) — this is a WORLD-PORT error \
         until proven otherwise; no trace runs behind it (§2, [S-I2])"
    ));
    if !shape_d.is_empty() {
        out.push("== shape ==".to_owned());
        for fd in &shape_d.fields {
            out.extend(diff::render_field(fd));
        }
    }
    if !bodies_d.is_empty() {
        out.push("== bodies ==".to_owned());
        for fd in &bodies_d.fields {
            out.extend(diff::render_field(fd));
        }
    }
    Ok((false, out))
}

// ---- compare / explain ------------------------------------------------------

/// Everything one comparison run needs.
#[derive(Clone)]
pub struct RunSpec {
    /// The world name.
    pub world: String,
    /// Which walk.
    pub walk: Walk,
    /// `--turns` (trace) or `--cap` (randtrace).
    pub steps: i64,
    /// The walk seed (randtrace).
    pub seed: Option<i64>,
    /// The engine die seed override (randtrace) [D-I6].
    pub die_seed: Option<i64>,
    /// The planner depth — PINNED at 2 across the design [D-I7], and compared in
    /// the header so a drift is SHAPE-DIVERGENT, never a DECISION storm [M1].
    pub depth: i32,
    /// The idler (trace).
    pub idle: Option<String>,
    /// The comparison depth.
    pub mode: Mode,
    /// Which localization fields to emit. `--candidates` is always on [S-I4].
    pub emit: Emit,
}

impl RunSpec {
    /// The frozen side's subcommand name.
    pub fn sub(&self) -> &'static str {
        match self.walk {
            Walk::Trace => "trace",
            Walk::Randtrace => "randtrace",
        }
    }

    /// The frozen oracle's command line for this run. `--mode` and the
    /// localization flags ride every invocation; `--depth`/`--idle` ride the
    /// trace walk and `--seed`/`--cap`/`--die-seed` the randtrace one.
    pub fn frozen_args(&self, mode: Mode) -> Vec<String> {
        let mut v = vec![self.sub().to_owned(), self.world.clone()];
        match self.walk {
            Walk::Trace => {
                v.push("--turns".into());
                v.push(self.steps.to_string());
                if let Some(i) = &self.idle {
                    v.push("--idle".into());
                    v.push(i.clone());
                }
                v.push("--depth".into());
                v.push(self.depth.to_string());
            }
            Walk::Randtrace => {
                v.push("--seed".into());
                v.push(self.seed.unwrap_or(0).to_string());
                v.push("--cap".into());
                v.push(self.steps.to_string());
                if let Some(ds) = self.die_seed {
                    v.push("--die-seed".into());
                    v.push(ds.to_string());
                }
            }
        }
        v.push("--mode".into());
        v.push(mode.as_str().to_owned());
        v.extend(self.emit.args());
        v
    }
}

fn parse_run(args: &[&str]) -> Result<RunSpec, String> {
    let world = (*args.first().ok_or("compare needs a world")?).to_owned();
    let walk = match flag(args, "--mode") {
        Some("trace") => Walk::Trace,
        Some("randtrace") => Walk::Randtrace,
        other => return Err(format!("--mode expects trace|randtrace, got {other:?}")),
    };
    let mode = match flag(args, "--emit") {
        None => Mode::State,
        Some(m) => Mode::parse(m)
            .ok_or_else(|| format!("--emit expects decisions|state|view, got `{m}`"))?,
    };
    let steps = match walk {
        Walk::Trace => int_flag(args, "--turns", Some(24))?,
        Walk::Randtrace => int_flag(args, "--cap", Some(50))?,
    };
    let die_seed = match flag(args, "--die-seed") {
        None => None,
        Some(_) => Some(int_flag(args, "--die-seed", None)?),
    };
    Ok(RunSpec {
        world: world.clone(),
        walk,
        steps,
        seed: match walk {
            Walk::Trace => None,
            Walk::Randtrace => Some(int_flag(args, "--seed", Some(0))?),
        },
        die_seed,
        depth: i32::try_from(int_flag(args, "--depth", Some(2))?)
            .map_err(|_| "--depth out of range".to_owned())?,
        idle: flag(args, "--idle")
            .map(str::to_owned)
            .or_else(|| worlds::idler(&world).map(str::to_owned)),
        mode,
        emit: if has(args, "--localize") {
            Emit::all()
        } else {
            Emit::matrix()
        },
    })
}

/// What one comparison run produced: the verdict, the `worldshape` verdict it
/// ran behind, and the WALK the two engines actually walked [I1].
///
/// The walk identity rides along because a matrix cell's contribution to
/// COVERAGE is not its record count — 375 intrigue seeds replay four walks — and
/// a number the report cannot distinguish from repetition is the number nobody
/// should size a budget with.
pub struct Run {
    /// The verdict.
    pub outcome: compare::Outcome,
    /// The `worldshape` verdict this ran behind.
    pub shape: Shape,
    /// The frozen stream's [`compare::walk_identity`]; empty when no walk ran
    /// (a shape divergence).
    pub walk: String,
}

/// Run one comparison, gating on `worldshape` FIRST (§1.6: worlds are
/// shape-checked before any seed runs, and [S-I2] makes ENUMERATION reportable
/// only behind a green one).
///
/// # Errors
/// If either side cannot be driven, or the classifier refuses.
pub fn run_one(spec: &RunSpec, reg: &Register) -> Result<Run, String> {
    let (green, shape_lines) = shape_compare(&spec.world)?;
    let rev = drive_frozen::freeze_rev()?;
    if !green {
        return Ok(Run {
            outcome: compare::Outcome::ShapeDivergent {
                fields: vec!["worldshape".to_owned()],
                detail: shape_lines,
            },
            shape: Shape::NotChecked,
            walk: String::new(),
        });
    }
    run_one_behind(spec, reg, &Shape::Green(rev))
}

/// One comparison run BEHIND an already-established `worldshape` verdict.
///
/// The shape check is per WORLD, not per (world, seed): it drives the frozen
/// oracle and two `git` subprocesses, and at 100 seeds × 4 worlds re-running it
/// per cell is ~400 redundant frozen invocations that say the same thing every
/// time. `matrix` shape-checks each world once up front (§1.6) and passes the
/// verdict here.
///
/// # Errors
/// If either side cannot be driven, or the classifier refuses.
pub fn run_one_behind(spec: &RunSpec, reg: &Register, shape: &Shape) -> Result<Run, String> {
    let shape = shape.clone();
    let mut frozen = drive_frozen::run_jsonl(&spec.frozen_args(spec.mode))?;
    // The walk identity is taken from the COARSE frozen stream, before any
    // localization rerun truncates it — the walk a seed names is the whole walk,
    // not the prefix the comparator stopped at.
    let walk = compare::walk_identity(&frozen);
    let mut rust = rust_stream(spec)?;
    let mut ctx = Ctx {
        walk: spec.walk,
        shape: shape.clone(),
        view_differs_earlier: None,
    };
    let mut outcome =
        compare::compare_streams(&frozen, &rust, &ctx, reg, &spec.world, spec.sub(), spec.seed)?;

    // [§1.4] THE LOCALIZATION RERUN. `compare` and `matrix` run the matrix
    // emission (candidates only [S-I4]), under which RNG and SCHEDULE cannot
    // reach their own pointers: `CRoll` advances the stream unconditionally, so
    // taken-vs-not leaves `rng` EQUAL, and an expiry firing on the wrong subtree
    // leaves `expiries` equal — both would report STATE and point at four
    // innocent subsystems [S-C5]. So on ANY divergence both sides are rerun with
    // the FULL emission (the draw log, the boundary log, the score table, the
    // action identity), truncated to the divergent ordinal, and RE-CLASSIFIED.
    // The richer emission can only reveal the divergence at or before the
    // ordinal the coarse pass found; the truncation stops it from wandering past
    // it, which is what makes the rerun a localization and not a second run.
    if let Some(ordinal) = divergent_ordinal(&outcome)
        && spec.emit != Emit::all()
    {
        let (f, r) = localization_streams(spec, ordinal)?;
        outcome =
            compare::compare_streams(&f, &r, &ctx, reg, &spec.world, spec.sub(), spec.seed)?;
        frozen = f;
        rust = r;
        // The full emission is a SUPERSET of the coarse one, so the rerun must
        // still find the divergence. If it does not, the two emissions disagree
        // about the same walk — a comparator bug, and it says so rather than
        // reporting the run clean.
        if divergent_ordinal(&outcome).is_none() {
            return Err(format!(
                "the localization rerun at ordinal {ordinal} found NO divergence under the full \
                 emission, while the matrix emission found one. The two emissions describe the \
                 same walk, so this is a bug in prax-oracle, not in either engine."
            ));
        }
    }

    // [§1.3(a)] THE VIEW-MODE RECLASSIFICATION — the DIV-1 shape and the single
    // most valuable rule in the classifier. A view-only divergence is invisible
    // in `state` mode and surfaces a turn later as TURN/ENUMERATION/DECISION/
    // STATE, so on ANY divergence both sides are rerun in `--mode view`; if the
    // views differ at an earlier record while the base dbs there agree, the class
    // becomes STATE(view) and the classifier says so ABOVE the whole ladder.
    if let Some(ordinal) = divergent_ordinal(&outcome)
        && spec.mode != Mode::View
        && let Some(witness) = view_divergence_before(spec, ordinal)?
    {
        ctx.view_differs_earlier = Some(witness);
        outcome = compare::compare_streams(
            &frozen,
            &rust,
            &ctx,
            reg,
            &spec.world,
            spec.sub(),
            spec.seed,
        )?;
    }
    Ok(Run {
        outcome,
        shape,
        walk,
    })
}

/// Both sides of the run re-driven with the FULL localization emission and
/// truncated to `ordinal` — the [§1.4] rerun's two properties in one place.
///
/// # Errors
/// If either side cannot be re-driven.
pub fn localization_streams(
    spec: &RunSpec,
    ordinal: usize,
) -> Result<(Vec<Value>, Vec<Value>), String> {
    let loc = RunSpec {
        emit: Emit::all(),
        ..spec.clone()
    };
    let frozen = truncate(drive_frozen::run_jsonl(&loc.frozen_args(loc.mode))?, ordinal);
    let rust = truncate(rust_stream(&loc)?, ordinal);
    Ok((frozen, rust))
}

/// Keep the header and records `1..=ordinal` — the localization rerun's
/// truncation to the divergent record.
fn truncate(mut stream: Vec<Value>, ordinal: usize) -> Vec<Value> {
    stream.truncate(ordinal + 1);
    stream
}

/// The ordinal a divergent outcome localized to.
fn divergent_ordinal(o: &compare::Outcome) -> Option<usize> {
    match o {
        compare::Outcome::Divergent(d) => Some(d.ordinal),
        _ => None,
    }
}

/// The FIRST record strictly before `ordinal` at which the closed VIEWs differ
/// while the base dbs still agree — the [§1.3(a)] window.
///
/// The window is scanned, not indexed at `ordinal - 1`. Two units are in play:
/// the view divergence happens at some walk step, the state-mode divergence
/// surfaces at some record ordinal, and [M2] records that `t` does not advance
/// on an idle pass — so an idle pass between them desynchronises "t−1" from
/// "ordinal−1" and a fixed one-record lookback returns `false` for exactly the
/// derivation bugs the rule exists to catch. Scanning forward from the first
/// record also reports the EARLIEST such record, which is where the reader
/// should start.
///
/// `None` when the view rerun cannot reach the ordinal at all.
fn view_divergence_before(
    spec: &RunSpec,
    ordinal: usize,
) -> Result<Option<ViewWitness>, String> {
    if ordinal < 2 {
        return Ok(None);
    }
    let view_spec = RunSpec {
        mode: Mode::View,
        ..spec.clone()
    };
    let frozen = drive_frozen::run_jsonl(&view_spec.frozen_args(Mode::View))?;
    let rust = rust_stream(&view_spec)?;
    Ok(first_view_divergence(&frozen, &rust, ordinal))
}

/// The scan itself, over two already-driven `--mode view` streams — separated
/// from the driving so the window rule has a resident net that does not need an
/// engine bug to exercise it.
pub fn first_view_divergence(
    frozen: &[Value],
    rust: &[Value],
    ordinal: usize,
) -> Option<ViewWitness> {
    for i in 1..ordinal {
        let (Some(a), Some(b)) = (frozen.get(i), rust.get(i)) else {
            return None;
        };
        let d = diff::diff_records(a, b);
        if d.has("facts") {
            // The base dbs already disagree here: whatever else differs, this is
            // not a view-only divergence and the ladder below must judge it.
            return None;
        }
        if let Some(fd) = d.get("view") {
            return Some(ViewWitness {
                ordinal: i,
                diff: diff::render_field(fd),
            });
        }
    }
    None
}

/// Build the Rust side's record stream for a run.
///
/// # Errors
/// If the world is not ported, or the die seed is out of the stream's domain.
pub fn rust_stream(spec: &RunSpec) -> Result<Vec<Value>, String> {
    let mut st = worlds::build(&spec.world)?;
    if let Some(ds) = spec.die_seed {
        st.seed_die(ds).map_err(|e| format!("--die-seed: {e}"))?;
    }
    Ok(match spec.walk {
        Walk::Trace => {
            let mut v = vec![drive_rust::trace_header(
                &spec.world,
                spec.steps,
                spec.idle.as_deref(),
                spec.depth,
                spec.mode,
                spec.emit,
            )];
            v.extend(drive_rust::trace_walk(
                &mut st,
                spec.emit,
                spec.depth,
                spec.steps,
                spec.idle.as_deref(),
                spec.mode,
            ));
            v
        }
        Walk::Randtrace => {
            let mut v = vec![drive_rust::rand_header(
                &spec.world,
                spec.seed.unwrap_or(0),
                spec.steps,
                spec.mode,
                spec.die_seed,
                spec.emit,
            )];
            v.extend(drive_rust::rand_walk(
                &mut st,
                spec.emit,
                spec.mode,
                spec.steps,
                spec.seed.unwrap_or(0) as u64,
            ));
            v
        }
    })
}

/// The repository root.
///
/// # Errors
/// If git cannot report it.
pub fn repo_root() -> Result<std::path::PathBuf, String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("git: {e}"))?;
    Ok(std::path::PathBuf::from(
        String::from_utf8_lossy(&out.stdout).trim().to_owned(),
    ))
}

/// The adjudicated-divergence register, loaded from the repository.
///
/// # Errors
/// If the file is missing or malformed — an unreadable register is never
/// treated as an empty one.
pub fn load_register() -> Result<Register, String> {
    Register::load(&repo_root()?.join("conformance/ADJUDICATED.json"))
}

fn cmd_compare(args: &[&str]) -> Result<bool, String> {
    let spec = parse_run(args)?;
    let reg = load_register()?;
    let Run { outcome, shape, .. } = run_one(&spec, &reg)?;
    report(&spec, &outcome, &shape);
    Ok(!outcome.is_failure())
}

/// `explain` is `compare` with the FULL localization emission on: candidates in
/// native order, the score table at depths 0..D, the action identity, the draw
/// log and the boundary log.
fn cmd_explain(args: &[&str]) -> Result<bool, String> {
    let mut spec = parse_run(args)?;
    spec.emit = Emit::all();
    let reg = load_register()?;
    let Run { outcome, shape, .. } = run_one(&spec, &reg)?;
    report(&spec, &outcome, &shape);
    Ok(!outcome.is_failure())
}

fn report(spec: &RunSpec, outcome: &compare::Outcome, shape: &Shape) {
    match outcome {
        // The COMPARED record count, never the requested one [I3]. A world that
        // dead-ends compares a fraction of its cap, and §4 normalizes the slice
        // budget on effective turns — so the request is the one number that
        // cannot be used for it.
        compare::Outcome::Clean { records } => println!(
            "{} {}: clean ({records} records compared, cap {})",
            spec.world,
            spec.sub(),
            spec.steps
        ),
        compare::Outcome::CleanModAdjudicated { ids, records } => println!(
            "{} {}: clean-mod-adjudicated {ids:?} ({records} records compared, cap {})",
            spec.world,
            spec.sub(),
            spec.steps
        ),
        compare::Outcome::ShapeDivergent { detail, .. } => {
            for l in detail {
                println!("{l}");
            }
        }
        compare::Outcome::Divergent(d) => {
            for l in compare::render(d, shape) {
                println!("{l}");
            }
        }
    }
}

#[cfg(test)]
mod classifier_selftest;
#[cfg(test)]
mod harness_tests;
