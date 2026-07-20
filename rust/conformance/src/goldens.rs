//! The goldens: the LOADER, and the gate that keeps them the only source.
//!
//! `scripts/golden-check.sh` relates `conformance/goldens/*.txt` to the frozen
//! spec literals and to the engine's own walk. It does NOT relate them to the
//! Rust assertion — and a Rust golden test carrying its own inline expected
//! narration is a THIRD copy, editable by whoever is trying to make the suite
//! green ([D-C3(b)]). So:
//!
//! - [`load`] is the one way a Rust test gets a golden sequence, and
//! - [`no_inline_golden_literals`] rejects a multi-line expected-narration
//!   literal in any golden test module.
//!
//! The golden TESTS themselves arrive with their slices (village-21 and
//! intrigue-12 with slices 4 and 2, bar-12 and loop-bar-25 with slice 3) — the
//! worlds they drive do not exist yet. What ships now is the mechanism, in
//! place BEFORE the first test can be written the wrong way.

use std::path::PathBuf;

/// The four goldens, by name (the file stems under `conformance/goldens/`).
pub const GOLDENS: &[&str] = &["village-21", "bar-12", "intrigue-12", "loop-bar-25"];

fn goldens_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/goldens")
}

/// Load a golden decision sequence, one line per element.
///
/// # Panics
/// If the file is missing or empty. A golden test that quietly compares against
/// nothing is worse than one that fails: it reports green while asserting
/// nothing at all.
pub fn load(name: &str) -> Vec<String> {
    let path = goldens_dir().join(format!("{name}.txt"));
    let body = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "golden `{name}` is missing at {}: {e}. Regenerate the goldens from the FROZEN tree \
             with scripts/golden-check.sh --update — never hand-write one.",
            path.display()
        )
    });
    let lines: Vec<String> = body.lines().map(str::to_owned).collect();
    assert!(
        !lines.is_empty(),
        "golden `{name}` is empty; a vacuous comparison is not a golden test"
    );
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_named_golden_loads_and_is_non_trivial() {
        for g in GOLDENS {
            let lines = load(g);
            println!("golden {g}: {} lines, first = {:?}", lines.len(), lines[0]);
            assert!(lines.len() >= 12, "golden {g} is suspiciously short");
        }
    }

    #[test]
    fn the_goldens_hashes_are_committed() {
        // [D-C3(a)] the check's designed successor: the cut-over DELETES the
        // frozen tree, so the hashes must be committed WHILE THE FREEZE LIVES.
        // A guarantee whose expiry is not designed is not a guarantee.
        let sums = goldens_dir().join("SHA256SUMS");
        let body = std::fs::read_to_string(&sums)
            .unwrap_or_else(|e| panic!("{} is missing: {e}", sums.display()));
        for g in GOLDENS {
            assert!(
                body.contains(&format!("{g}.txt")),
                "SHA256SUMS does not cover {g}.txt"
            );
        }
    }

    /// A multi-line expected-narration literal in a golden test is a THIRD copy
    /// of the golden, and the one an implementer can edit to make the suite
    /// green. The gate rejects it by shape: a `let`-bound array of three or more
    /// consecutive string literals in a file that also loads a golden.
    #[test]
    fn no_inline_golden_literals() {
        let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = Vec::new();
        for entry in std::fs::read_dir(&src).expect("the conformance src dir") {
            let path = entry.expect("a dir entry").path();
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            // This file's own doc comment names the pattern it forbids.
            if path.file_name().is_some_and(|f| f == "goldens.rs") {
                continue;
            }
            let body = std::fs::read_to_string(&path).expect("readable");
            if !body.contains("goldens::load") && !body.contains("golden") {
                continue;
            }
            let mut run = 0usize;
            for (i, line) in body.lines().enumerate() {
                let t = line.trim();
                let is_literal_row = t.starts_with('"') && t.ends_with(&['"', ',']);
                run = if is_literal_row { run + 1 } else { 0 };
                if run >= 3 {
                    offenders.push(format!("{}:{}", path.display(), i + 1));
                    break;
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "these golden tests carry an INLINE expected-narration literal: {offenders:?}. \
             A golden test must LOAD conformance/goldens/<name>.txt (goldens::load) — an inline \
             copy is a third source of truth, and the only one editable by whoever is trying to \
             make the suite green [D-C3(b)]."
        );
    }
}
