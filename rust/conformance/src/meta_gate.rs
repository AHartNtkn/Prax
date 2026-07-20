//! The meta-gate: parse `conformance/HASKELL_PINS.txt` and assert each of the
//! ~849 labels is accounted for exactly once — either re-expressed (a `// H:`
//! comment on a Rust test) or explicitly killed (a `KILLED.md` row with a
//! category and reason). Enforced as a test once the corpus starts filling.
//!
//! This is the FIRST CUT (S1): the gate is scoped by a per-stage ALLOWLIST of
//! spec-file basenames. It grows as each stage lands — later stages append
//! their spec files to [`ALLOWLIST`]. The accounting is a red/green test, not a
//! claim: a pin that is neither re-expressed nor killed fails the build, and so
//! does a `// H:`/`KILLED.md` entry that names no allowlisted pin (a typo net).

#[cfg(test)]
mod gate {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// Spec-file basenames whose pins this stage must account for. Extend per
    /// stage (S2 adds QuerySpec.hs / CookedSpec.hs, …).
    const ALLOWLIST: &[&str] = &[
        "SymSpec.hs",
        "DbSpec.hs",
        "ELSpec.hs",
        "QuerySpec.hs",
        "CookedSpec.hs",
        "DeriveSpec.hs",
    ];

    fn repo_root() -> PathBuf {
        // rust/conformance -> rust -> repo root
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.pop();
        p.pop();
        p
    }

    fn rust_root() -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.pop();
        p
    }

    fn basename(path: &str) -> String {
        path.rsplit(['/', '\\'])
            .next()
            .unwrap_or(path)
            .trim()
            .to_owned()
    }

    /// The allowlisted `(SpecFile, label)` pins from the committed manifest.
    fn read_pins() -> BTreeSet<(String, String)> {
        let path = repo_root().join("conformance/HASKELL_PINS.txt");
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        let mut pins = BTreeSet::new();
        for line in text.lines() {
            let Some((file, label)) = line.split_once('\t') else {
                continue;
            };
            let base = basename(file);
            if ALLOWLIST.contains(&base.as_str()) {
                pins.insert((base, label.to_owned()));
            }
        }
        pins
    }

    /// Every `// H: <SpecFile> "<label>"` occurrence across `rust/`, as
    /// `(basename, label)`. Only the label's surrounding double quotes delimit
    /// it; S1 labels contain no embedded quotes.
    fn collect_h_comments() -> Vec<(String, String)> {
        let mut out = Vec::new();
        let mut files = Vec::new();
        collect_rs_files(&rust_root(), &mut files);
        for file in files {
            let text = fs::read_to_string(&file)
                .unwrap_or_else(|e| panic!("reading {}: {e}", file.display()));
            for line in text.lines() {
                let Some(rest) = line.split_once("// H:").map(|(_, r)| r) else {
                    continue;
                };
                let Some(open) = rest.find('"') else {
                    continue;
                };
                let spec = rest[..open].trim();
                let after = &rest[open + 1..];
                let Some(close) = after.find('"') else {
                    continue;
                };
                let label = &after[..close];
                out.push((basename(spec), label.to_owned()));
            }
        }
        out
    }

    fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
        let entries =
            fs::read_dir(dir).unwrap_or_else(|e| panic!("reading dir {}: {e}", dir.display()));
        for entry in entries {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            let name = entry.file_name();
            if path.is_dir() {
                // Skip build artifacts; recurse everything else.
                if name == "target" {
                    continue;
                }
                collect_rs_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    /// Every `KILLED.md` table row, as `(basename, label)`. Header and
    /// separator rows are skipped.
    fn read_killed() -> Vec<(String, String)> {
        let path = repo_root().join("conformance/KILLED.md");
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        let mut out = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with('|') {
                continue;
            }
            let cells: Vec<&str> = trimmed.split('|').map(str::trim).collect();
            // "| a | b | c | d |" -> ["", "a", "b", "c", "d", ""]
            if cells.len() < 4 {
                continue;
            }
            let spec = cells[1];
            let label = cells[2];
            if spec == "SpecFile" || spec.starts_with("---") || spec.is_empty() {
                continue;
            }
            out.push((basename(spec), label.to_owned()));
        }
        out
    }

    // The meta-gate FIRST CUT: every allowlisted Haskell pin is accounted for
    // exactly once, and no `// H:`/`KILLED.md` entry in an allowlisted spec file
    // names a pin that does not exist.
    #[test]
    fn every_allowlisted_pin_accounted_for_exactly_once() {
        let pins = read_pins();
        assert!(
            !pins.is_empty(),
            "no allowlisted pins found — manifest path wrong?"
        );

        let h = collect_h_comments();
        let killed = read_killed();

        let mut problems = Vec::new();

        // Each pin appears exactly once across (// H comments) ∪ (KILLED rows).
        for pin in &pins {
            let h_count = h.iter().filter(|e| *e == pin).count();
            let killed_count = killed.iter().filter(|e| *e == pin).count();
            let total = h_count + killed_count;
            if total != 1 {
                problems.push(format!(
                    "pin [{}] {:?}: accounted {total} times ({h_count} // H, {killed_count} KILLED) — must be exactly 1",
                    pin.0, pin.1
                ));
            }
        }

        // No allowlisted // H comment names a non-existent pin (typo net).
        for e in &h {
            if ALLOWLIST.contains(&e.0.as_str()) && !pins.contains(e) {
                problems.push(format!(
                    "// H comment [{}] {:?} names no pin in HASKELL_PINS.txt",
                    e.0, e.1
                ));
            }
        }
        // Same for KILLED rows.
        for e in &killed {
            if ALLOWLIST.contains(&e.0.as_str()) && !pins.contains(e) {
                problems.push(format!(
                    "KILLED.md row [{}] {:?} names no pin in HASKELL_PINS.txt",
                    e.0, e.1
                ));
            }
        }

        assert!(
            problems.is_empty(),
            "meta-gate: {} problem(s):\n{}",
            problems.len(),
            problems.join("\n")
        );
    }
}
