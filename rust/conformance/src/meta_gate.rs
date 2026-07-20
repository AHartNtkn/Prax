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
        // S4:
        "EngineSpec.hs",
        "RngSpec.hs",
        "GateSpec.hs",
        // S5:
        "LoopSpec.hs",
        "ScheduleSpec.hs",
        "ScheduleRuleSpec.hs",
        // S6:
        "PlannerSpec.hs",
        "MindsSpec.hs",
        "RelevanceSpec.hs",
        "SightSpec.hs",
        // S7 slice 1 (feud):
        "FactionSpec.hs",
        "KinSpec.hs",
        "FeudSpec.hs",
        // S7 slice 2 (intrigue):
        "CoreSpec.hs",
        "EmotionSpec.hs",
        "BeliefsSpec.hs",
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

    /// A stage tag is `S` followed by ≥1 ASCII digit (`S0`..`S10`).
    fn is_stage_tag(s: &str) -> bool {
        s.starts_with('S') && s.len() > 1 && s[1..].chars().all(|c| c.is_ascii_digit())
    }

    /// PROGRAM.md status-table rows as `(stage, state)` for every `S<N>` row. Pure
    /// over the file text so the gate's loudness is unit-testable.
    fn parse_stage_states(text: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with('|') {
                continue;
            }
            // "| S3 | scope | IN PROGRESS | report |" -> 6 cells.
            let cells: Vec<&str> = trimmed.split('|').map(str::trim).collect();
            if cells.len() < 5 {
                continue;
            }
            if is_stage_tag(cells[1]) {
                out.push((cells[1].to_owned(), cells[3].to_owned()));
            }
        }
        out
    }

    /// [`parse_stage_states`] with the mandatory loudness: an empty parse (the
    /// board's format drifted out from under the parser) PANICS rather than
    /// silently disabling the owed gate. Split from the file read so the panic is
    /// exercised by a synthetic board in tests.
    fn stage_states_or_die(text: &str, source: &str) -> Vec<(String, String)> {
        let states = parse_stage_states(text);
        assert!(
            !states.is_empty(),
            "meta-gate: parsed NO S<N> stage rows from {source} — the status board \
             format changed and the deferral-owed gate is now dead. Fix parse_stage_states."
        );
        states
    }

    fn read_program_stage_states() -> Vec<(String, String)> {
        let path = repo_root().join("docs/rewrite/PROGRAM.md");
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        stage_states_or_die(&text, "PROGRAM.md status board")
    }

    /// The owed stage a KILLED `Owed` cell names (`owed: S<N>`), or `None` for
    /// `—`/empty. Loud on a cell that starts with `owed:` but is malformed — a
    /// typo there would otherwise silently drop a deferral obligation.
    fn parse_owed(cell: &str) -> Option<String> {
        let rest = cell.trim().strip_prefix("owed:")?.trim();
        assert!(
            is_stage_tag(rest),
            "KILLED.md Owed cell {cell:?} starts with `owed:` but names no S<N> stage — \
             a typo that silently drops a deferral obligation."
        );
        Some(rest.to_owned())
    }

    /// KILLED rows carrying an owed obligation, as `(spec, label, owed-stage)`.
    /// The `Owed` cell sits between `Category` and `Reason` (cell index 4), before
    /// the free-text reason, so a stray `|` in a reason cannot shift it.
    fn read_killed_owed() -> Vec<(String, String, String)> {
        let path = repo_root().join("conformance/KILLED.md");
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        let mut out = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with('|') {
                continue;
            }
            // "| spec | label | cat | owed | reason |" -> 7 cells.
            let cells: Vec<&str> = trimmed.split('|').map(str::trim).collect();
            if cells.len() < 6 {
                continue;
            }
            let spec = cells[1];
            if spec == "SpecFile" || spec.starts_with("---") || spec.is_empty() {
                continue;
            }
            if let Some(stage) = parse_owed(cells[4]) {
                out.push((basename(spec), cells[2].to_owned(), stage));
            }
        }
        out
    }

    /// The deferral-owed check, pure over parsed inputs: an owed row whose owing
    /// stage is DONE — or is absent from the board — is a problem. A pending
    /// (not-yet-DONE) owing stage is fine: the obligation is still live.
    fn owed_problems(
        states: &[(String, String)],
        owed: &[(String, String, String)],
    ) -> Vec<String> {
        let mut problems = Vec::new();
        for (spec, label, stage) in owed {
            match states.iter().find(|(s, _)| s == stage).map(|(_, st)| st.as_str()) {
                None => problems.push(format!(
                    "KILLED row [{spec}] {label:?} owes {stage}, but the PROGRAM.md board has \
                     no such stage — stale owed tag or renamed stage."
                )),
                Some("DONE") => problems.push(format!(
                    "KILLED row [{spec}] {label:?} owes {stage}, which is DONE — the deferral was \
                     never re-expressed. Re-expression REMOVES this row (the exactly-once rule); a \
                     standing owed row at DONE means the pin silently never landed."
                )),
                Some(_) => {}
            }
        }
        problems
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

    // The deferral-owed gate (S3 review I2): no KILLED row may still owe a
    // re-expression to a stage the PROGRAM.md board marks DONE. This complements
    // the exactly-once accounting above — that catches a re-expressed pin whose
    // row still stands (double-counted); this catches the converse hole, a pin
    // that silently never landed while its row rode along to a DONE stage.
    #[test]
    fn no_owed_deferral_survives_its_owing_stage_being_done() {
        let states = read_program_stage_states();
        let owed = read_killed_owed();
        assert!(
            !owed.is_empty(),
            "no owed deferral rows found in KILLED.md — the Owed column format changed \
             and the deferral gate is now dead."
        );
        let problems = owed_problems(&states, &owed);
        assert!(
            problems.is_empty(),
            "meta-gate deferral-owed: {} problem(s):\n{}",
            problems.len(),
            problems.join("\n")
        );
    }

    // The RED this gate exists to produce, on a synthetic board: an owing stage
    // marked DONE with its row still standing must fire a problem.
    #[test]
    fn owed_gate_fires_when_the_owing_stage_is_done() {
        let states = vec![("S4".to_owned(), "DONE".to_owned())];
        let owed = vec![(
            "DeriveSpec.hs".to_owned(),
            "obligedClose: a domain rule (written once) also closes under obligation".to_owned(),
            "S4".to_owned(),
        )];
        let problems = owed_problems(&states, &owed);
        assert_eq!(problems.len(), 1, "a DONE owing stage with a standing row must fire");
        assert!(problems[0].contains("DONE"), "problem names the DONE stage: {}", problems[0]);
    }

    // While the owing stage is still pending (not DONE), the live obligation is
    // silent — the gate does not false-fire on an in-progress or untouched stage.
    #[test]
    fn owed_gate_silent_while_the_owing_stage_is_pending() {
        let states = vec![("S4".to_owned(), "—".to_owned())];
        let owed = vec![("X.hs".to_owned(), "lbl".to_owned(), "S4".to_owned())];
        assert!(owed_problems(&states, &owed).is_empty());
    }

    // An owed tag naming a stage the board does not list is itself a problem
    // (stale tag / renamed stage), never silently ignored.
    #[test]
    fn owed_gate_fires_on_a_stage_absent_from_the_board() {
        let states = vec![("S4".to_owned(), "—".to_owned())];
        let owed = vec![("X.hs".to_owned(), "lbl".to_owned(), "S9".to_owned())];
        let problems = owed_problems(&states, &owed);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("no such stage"), "{}", problems[0]);
    }

    // Loudness: a board the parser cannot read (no S<N> rows) PANICS rather than
    // returning empty and silently disabling the owed gate.
    #[test]
    #[should_panic(expected = "deferral-owed gate is now dead")]
    fn stage_board_gate_is_loud_when_no_stage_rows_parse() {
        stage_states_or_die("# PROGRAM\n\nno pipe table here\n", "synthetic board");
    }

    // The parser does read the real board (guards the loudness test against a
    // parser that only ever returns empty).
    #[test]
    fn parser_sees_the_real_program_board() {
        let states = read_program_stage_states();
        assert!(
            states.iter().any(|(s, _)| s == "S3"),
            "expected an S3 row on the board, got {states:?}"
        );
    }
}
