//! The goldens: the LOADER, and the gate that keeps them the only source.
//!
//! `scripts/golden-check.sh` relates `conformance/goldens/*.txt` to the frozen
//! spec literals and to the engine's own walk. It does NOT relate them to the
//! Rust assertion — and a Rust golden test carrying its own inline expected
//! narration is a THIRD copy, editable by whoever is trying to make the suite
//! green ([D-C3(b)]). So:
//!
//! - [`load`] is the one way a Rust test gets a golden sequence, and
//! - `no_inline_golden_literals` sweeps EVERY `.rs` file under `rust/`,
//!   recursively, and rejects any that re-types three consecutive lines of a
//!   committed golden. It is keyword-free on purpose: the file it has to catch
//!   is precisely the one that does not already say "golden".
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

    /// A source line that is nothing but a string literal, and its contents.
    fn bare_literal(line: &str) -> Option<String> {
        let t = line.trim().strip_prefix('"')?;
        let t = t.strip_suffix(',').unwrap_or(t).strip_suffix('"')?;
        (!t.is_empty()).then(|| t.to_owned())
    }

    /// The line (1-based) at which a source body first re-types a FRAGMENT of a
    /// committed golden: three or more consecutive bare string literals that are
    /// three or more consecutive lines of the same golden, in order.
    ///
    /// That is the discrimination the gate needs, and it is exact rather than
    /// shape-guessed. A workspace full of authored fact lists and label arrays
    /// makes "three string literals in a row" far too coarse — but three
    /// CONSECUTIVE lines of a golden reproduced IN ORDER is not a coincidence
    /// between a world's data and its narration; it is a copy. One matching line
    /// could be an action label that legitimately occurs in both places, which
    /// is why one is not enough.
    fn inline_golden_fragment_at(body: &str, goldens: &[(String, Vec<String>)]) -> Option<usize> {
        let lines: Vec<&str> = body.lines().collect();
        for start in 0..lines.len() {
            let mut run: Vec<String> = Vec::new();
            for line in &lines[start..] {
                match bare_literal(line) {
                    Some(s) => run.push(s),
                    None => break,
                }
            }
            if run.len() < 3 {
                continue;
            }
            for (_, golden) in goldens {
                for w in golden.windows(3) {
                    if run.windows(3).any(|r| r == w) {
                        return Some(start + 1);
                    }
                }
            }
        }
        None
    }

    /// A multi-line expected-narration literal is a THIRD copy of a golden, and
    /// the one an implementer can edit to make the suite green [D-C3(b)]. The
    /// gate is REPO-WIDE and KEYWORD-FREE by design: the file it has to catch is
    /// precisely the one that does not already say "golden" — a LoopSpec-style
    /// 25-line narration inlined in `prax-worlds`' tests, or in a subdirectory,
    /// or in a file that simply never mentions the word. Restricting the sweep
    /// to files that opt in would make it a gate only the compliant pass.
    ///
    /// It rejects by CONTENT: a run of consecutive bare string literals that
    /// reproduces three consecutive lines of a committed golden. The gate is in
    /// place BEFORE the four golden tests arrive in slices 2–4.
    #[test]
    fn no_inline_golden_literals() {
        let goldens: Vec<(String, Vec<String>)> =
            GOLDENS.iter().map(|g| ((*g).to_owned(), load(g))).collect();
        let rust_root = crate::source_sweep::rust_root();
        let sources = crate::source_sweep::every_rust_source(&rust_root);
        crate::source_sweep::assert_reaches_workspace(&sources, &rust_root);
        println!(
            "inline-literal gate: {} .rs files under {}, against {} goldens",
            sources.len(),
            rust_root.display(),
            goldens.len()
        );

        let mut offenders = Vec::new();
        for path in &sources {
            // This file's own doc comment names the pattern it forbids. Matched
            // on the FULL path [slice-2 review M3]: by basename, a second
            // `goldens.rs` in any crate would inherit the exemption and become a
            // blind spot in a gate whose whole job is to have none.
            if path.ends_with("conformance/src/goldens.rs") {
                continue;
            }
            let body = std::fs::read_to_string(path).expect("readable");
            if let Some(line) = inline_golden_fragment_at(&body, &goldens) {
                offenders.push(format!("{}:{line}", path.display()));
            }
        }
        assert!(
            offenders.is_empty(),
            "these files re-type a fragment of a committed golden inline: {offenders:?}. \
             A golden test must LOAD conformance/goldens/<name>.txt (goldens::load) — an inline \
             copy is a third source of truth, and the only one editable by whoever is trying to \
             make the suite green [D-C3(b)]."
        );
    }

    /// The gate's own mutation evidence. The file it exists to catch is the one
    /// that never says "golden" and lives nowhere near `conformance/src` — so
    /// the fixture is exactly that: a LoopSpec-style narration, inlined in a
    /// function with an innocent name, in a body containing the word nowhere.
    #[test]
    fn the_inline_literal_gate_catches_a_narration_that_never_says_golden() {
        let goldens: Vec<(String, Vec<String>)> =
            GOLDENS.iter().map(|g| ((*g).to_owned(), load(g))).collect();
        let narration = &load("loop-bar-25")[..5];
        let rows: String = narration
            .iter()
            .map(|l| format!("        \"{l}\",\n"))
            .collect();
        let body = format!("fn expected_ticks() -> Vec<&'static str> {{\n    vec![\n{rows}    ]\n}}\n");
        assert!(!body.contains("golden"), "the fixture must not opt in");
        assert_eq!(
            inline_golden_fragment_at(&body, &goldens),
            Some(3),
            "the gate must catch a re-typed golden fragment in a file that never says `golden`"
        );

        // …and must NOT fire on the authored data a world legitimately carries:
        // consecutive string literals that are not a golden's consecutive lines.
        let facts = "vec![\n    \"char.vera\",\n    \"char.otto\",\n    \"at.vera.bar\",\n]\n";
        assert_eq!(inline_golden_fragment_at(facts, &goldens), None);

        // Two consecutive golden lines are not enough: a single action label can
        // legitimately appear in a world's own data.
        let two: String = narration[..2]
            .iter()
            .map(|l| format!("    \"{l}\",\n"))
            .collect();
        assert_eq!(inline_golden_fragment_at(&two, &goldens), None);
    }
}
