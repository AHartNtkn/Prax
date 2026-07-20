//! Driving the FROZEN Haskell oracle.
//!
//! `cabal run -v0 prax-oracle -- <subcmd>` (overridable by `$PRAX_ORACLE_CMD`),
//! streaming JSONL. Three rules, all load-bearing (S7 design §1.1):
//!
//! 1. **`scripts/freeze-check.sh` runs FIRST**, once per process, before any
//!    record can be produced. A dirty frozen tree aborts the whole run — a
//!    comparison against an edited reference is not a comparison.
//! 2. **The cache is keyed by the freeze rev**, so a stale entry cannot lie. The
//!    rev is the `haskell-freeze` tag's commit AND the hash of `oracle/`: the
//!    frozen library cannot change, but the additive oracle can, and its output
//!    is what is being cached.
//! 3. **A nonzero exit or an unparseable line is FATAL**, never an empty stream.

use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

/// Where the repository root is (every path here is absolute).
fn repo_root() -> Result<PathBuf, String> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("git rev-parse --show-toplevel failed to start: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git rev-parse --show-toplevel failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(PathBuf::from(
        String::from_utf8_lossy(&out.stdout).trim().to_owned(),
    ))
}

/// Run `scripts/freeze-check.sh` — ONCE per process, before the first frozen
/// invocation. The result is memoized because the tree cannot change under a
/// running comparison without the comparison already being void.
pub fn freeze_check() -> Result<(), String> {
    static CHECKED: OnceLock<Result<(), String>> = OnceLock::new();
    CHECKED.get_or_init(run_freeze_check).clone()
}

fn run_freeze_check() -> Result<(), String> {
    let root = repo_root()?;
    let out = Command::new(root.join("scripts/freeze-check.sh"))
        .current_dir(&root)
        .output()
        .map_err(|e| format!("scripts/freeze-check.sh failed to start: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "FREEZE CHECK FAILED — refusing to produce a record against an edited reference.\n{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ))
    }
}

/// The freeze revision: the `haskell-freeze` tag's commit plus the hash of the
/// additive oracle source. Both halves matter — the frozen library is pinned by
/// the tag, and `oracle/TraceMain.hs` is what actually emits the records.
pub fn freeze_rev() -> Result<String, String> {
    static REV: OnceLock<Result<String, String>> = OnceLock::new();
    REV.get_or_init(compute_freeze_rev).clone()
}

fn compute_freeze_rev() -> Result<String, String> {
    let root = repo_root()?;
    let tag = git(&root, &["rev-parse", "haskell-freeze"])?;
    let oracle = git(&root, &["hash-object", "oracle/TraceMain.hs"])?;
    Ok(format!("{}-{}", &tag[..12.min(tag.len())], &oracle[..12.min(oracle.len())]))
}

fn git(root: &PathBuf, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("git {args:?} failed to start: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// The oracle invocation, overridable by `$PRAX_ORACLE_CMD` (whitespace-split).
fn oracle_cmd() -> Vec<String> {
    match std::env::var("PRAX_ORACLE_CMD") {
        Ok(s) if !s.trim().is_empty() => s.split_whitespace().map(str::to_owned).collect(),
        _ => ["cabal", "run", "-v0", "prax-oracle", "--"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
    }
}

/// A cache filename for an invocation: the arguments, made filesystem-safe.
/// Readable on purpose — a cache you cannot inspect is a cache you cannot trust.
fn cache_name(args: &[String]) -> String {
    let mut s: String = args
        .join("_")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '.' })
        .collect();
    s.push_str(".jsonl");
    s
}

/// Run the frozen oracle and return its stdout, memoized under
/// `target/oracle-cache/<freeze-rev>/`.
fn run_raw(args: &[String]) -> Result<String, String> {
    freeze_check()?;
    let root = repo_root()?;
    let dir = root.join("rust/target/oracle-cache").join(freeze_rev()?);
    let path = dir.join(cache_name(args));
    if let Ok(s) = std::fs::read_to_string(&path) {
        return Ok(s);
    }
    let cmd = oracle_cmd();
    let (prog, head) = cmd.split_first().expect("PRAX_ORACLE_CMD is non-empty");
    let out = Command::new(prog)
        .args(head)
        .args(args)
        .current_dir(&root)
        .output()
        .map_err(|e| format!("the frozen oracle ({}) failed to start: {e}", cmd.join(" ")))?;
    if !out.status.success() {
        return Err(format!(
            "the frozen oracle exited {} on `{} {}`:\n{}",
            out.status,
            cmd.join(" "),
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let body = String::from_utf8(out.stdout)
        .map_err(|e| format!("the frozen oracle emitted non-UTF-8 bytes: {e}"))?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("cannot create the oracle cache dir {}: {e}", dir.display()))?;
    std::fs::write(&path, &body)
        .map_err(|e| format!("cannot write the oracle cache file {}: {e}", path.display()))?;
    Ok(body)
}

/// Run a frozen subcommand returning JSONL, parsed record by record. An
/// unparseable line is fatal: a dropped record would silently shorten the
/// stream, which the comparator would then report as a TERMINATION divergence
/// against a stream that was never short.
pub fn run_jsonl(args: &[String]) -> Result<Vec<Value>, String> {
    let body = run_raw(args)?;
    let mut out = Vec::new();
    for (i, line) in body.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        out.push(
            serde_json::from_str(line)
                .map_err(|e| format!("frozen record {} is not JSON: {e}\n  {line}", i + 1))?,
        );
    }
    if out.is_empty() {
        return Err(format!(
            "the frozen oracle produced NO records for `{}` — an empty stream is never a valid \
             comparison base",
            args.join(" ")
        ));
    }
    Ok(out)
}

/// Run a frozen subcommand returning ONE JSON value (`worldshape`, `check`,
/// `fixtures`).
pub fn run_json(args: &[String]) -> Result<Value, String> {
    let body = run_raw(args)?;
    serde_json::from_str(body.trim())
        .map_err(|e| format!("frozen `{}` did not emit one JSON value: {e}", args.join(" ")))
}
