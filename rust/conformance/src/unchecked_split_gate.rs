//! The unchecked-split gate: every `path::segment_names` call site carries a
//! written adjudication against its frozen counterpart.
//!
//! `Prax.Db.pathNames` is `map fst . tokens`, and `tokens` RAISES on a trailing
//! operator. `path::segment_names` is the deliberately UNCHECKED split, which
//! returns a trailing empty segment instead; `path::segment_names_checked` is
//! the raising one. Porting a frozen `pathNames` call site to the unchecked
//! helper therefore converts a loud construction-time rejection into a malformed
//! value that renders plausibly — silently, and identically at every site.
//!
//! That substitution has now been found THREE times (the S7 slice-1 review's
//! [C2], its fix wave, and the slice-2 review's [I2], which found four more call
//! sites one rung deeper). A class that recurs three times is not caught by
//! reading; it is caught by a gate. So: **every call to `segment_names` in the
//! workspace must carry an `UNCHECKED-SPLIT (frozen: …)` marker** in a comment
//! on the call line or in the comment block directly above it, naming the frozen
//! counterpart and why it cannot raise there. A call site that cannot write that
//! sentence honestly is a call site that wants
//! [`prax_core::path::segment_names_checked`].
//!
//! The gate is mechanical about the marker, not about the reasoning: it cannot
//! read Haskell, and a gate that pretended to would be worse than one that
//! forces a human sentence at the exact line where the substitution happens.
//! What it guarantees is that the substitution can never again be made SILENTLY
//! — the diff that adds an unmarked call is red.

/// The marker an unchecked call site must carry, with the frozen counterpart
/// named after it.
pub const MARKER: &str = "UNCHECKED-SPLIT (frozen:";

/// How many lines above a call site the marker may sit. A marker is an
/// adjudication OF a call, so it belongs in the comment block directly above it;
/// the window is the longest such block the workspace's own sites need (six
/// lines) plus room for one more sentence.
const WINDOW: usize = 8;

/// Is this line a call to the UNCHECKED split (and not to the checked one, nor
/// the definition, nor a doc-comment mention)?
pub fn calls_unchecked_split(line: &str) -> bool {
    let code = line.split_once("//").map_or(line, |(before, _)| before);
    let trimmed = code.trim_start();
    if trimmed.starts_with("pub fn segment_names(") || trimmed.starts_with("fn segment_names(") {
        return false;
    }
    let mut rest = code;
    while let Some(at) = rest.find("segment_names(") {
        let after = &rest[at + "segment_names(".len()..];
        let before = &rest[..at];
        // `segment_names_checked(` contains no `segment_names(` occurrence, so a
        // hit here is the unchecked helper; the only remaining false positive is
        // a longer identifier ENDING in it (`foo_segment_names(`).
        if !before.ends_with(|c: char| c.is_alphanumeric() || c == '_') {
            return true;
        }
        rest = after;
    }
    false
}

/// Every unadjudicated call site under `root`, as `path:line`.
pub fn unadjudicated(root: &std::path::Path) -> Vec<String> {
    let sources = crate::source_sweep::every_rust_source(root);
    crate::source_sweep::assert_reaches_workspace(&sources, root);
    let mut offenders = Vec::new();
    for path in &sources {
        // This file states the marker it enforces; its own text is not a call.
        if path.ends_with("conformance/src/unchecked_split_gate.rs") {
            continue;
        }
        let body = std::fs::read_to_string(path).expect("readable");
        let lines: Vec<&str> = body.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if !calls_unchecked_split(line) {
                continue;
            }
            let from = i.saturating_sub(WINDOW);
            if !lines[from..=i].iter().any(|l| l.contains(MARKER)) {
                offenders.push(format!("{}:{}", path.display(), i + 1));
            }
        }
    }
    offenders
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_unchecked_split_call_is_adjudicated_against_its_frozen_counterpart() {
        let root = crate::source_sweep::rust_root();
        let offenders = unadjudicated(&root);
        assert!(
            offenders.is_empty(),
            "these call sites use the UNCHECKED split `path::segment_names` with no adjudication \
             against their frozen counterpart: {offenders:?}\n\
             `Prax.Db.pathNames` is `map fst . tokens` and RAISES on a trailing operator; this \
             helper returns a trailing empty segment instead. If the frozen counterpart is a \
             `pathNames`/`parseNames` call that can raise, use `path::segment_names_checked` and \
             thread the rejection. If it genuinely cannot raise there, say so at the call site in \
             a comment containing `{MARKER} …)` and naming why."
        );
    }

    #[test]
    fn the_gate_recognizes_a_call_and_ignores_the_checked_one() {
        // The recognizer is the whole gate: what it fails to see, nobody sees.
        assert!(calls_unchecked_split("    let n = segment_names(&c.name).len();"));
        assert!(calls_unchecked_split("segment_names(s)"));
        assert!(calls_unchecked_split("        head_names(st).contains(&segment_names(s))"));
        assert!(
            !calls_unchecked_split("    let n = segment_names_checked(&r.name)?.len();"),
            "the CHECKED helper is the remedy, not the offence"
        );
        assert!(
            !calls_unchecked_split("pub fn segment_names(sentence: &str) -> Vec<String> {"),
            "the definition is not a call site"
        );
        assert!(
            !calls_unchecked_split("/// see [`segment_names(`] for the unchecked split"),
            "a doc comment is not a call site"
        );
        assert!(
            !calls_unchecked_split("    let x = my_segment_names(s);"),
            "a longer identifier that merely ends in the name is a different function"
        );
    }

    #[test]
    fn an_unmarked_call_site_is_reported_and_a_marked_one_is_not() {
        // The gate's own mutation evidence, run every time rather than described:
        // the same two files, one with the marker and one without, through the
        // real sweep.
        let dir = std::env::temp_dir().join(format!(
            "prax-unchecked-split-gate-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let src = dir.join("crate-under-test/src");
        std::fs::create_dir_all(&src).expect("the probe tree");
        // 41 filler files: the sweep refuses to judge a tree it has not reached.
        for i in 0..41 {
            std::fs::write(src.join(format!("filler{i}.rs")), "// nothing to see\n")
                .expect("a filler file");
        }
        let marked = format!(
            "fn f(s: &str) -> usize {{\n    // {MARKER} none — invented for the gate's own test)\n    \
             segment_names(s).len()\n}}\n"
        );
        std::fs::write(src.join("marked.rs"), marked).expect("the marked probe");
        assert!(
            unadjudicated(&dir).is_empty(),
            "a marked call site is adjudicated and must not be reported"
        );

        std::fs::write(
            src.join("unmarked.rs"),
            "fn g(s: &str) -> usize {\n    segment_names(s).len()\n}\n",
        )
        .expect("the unmarked probe");
        let offenders = unadjudicated(&dir);
        assert_eq!(offenders.len(), 1, "exactly the unmarked site: {offenders:?}");
        assert!(
            offenders[0].ends_with("unmarked.rs:2"),
            "the gate points at the line, not the file: {offenders:?}"
        );
        std::fs::remove_dir_all(&dir).expect("the probe tree is removed");
    }
}
