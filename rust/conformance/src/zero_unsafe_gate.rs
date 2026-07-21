//! The zero-unsafe gate: no workspace `.rs` source uses the Rust `unsafe`
//! keyword (S10 design §3, [R3]).
//!
//! The S9 diff was unsafe-free; S10 asserts it TREE-WIDE and RESIDENT, so a
//! future diff that reaches for `unsafe` reddens rather than slipping in. This
//! is the sweep form the design names ("a resident conformance sweep reusing
//! `source_sweep::every_rust_source`"), not a one-shot grep that evaporates.
//!
//! **Why a grammar detector, not a substring match.** The bare text `unsafe`
//! appears legitimately in this workspace and must NOT trip the gate:
//! `unsafePerformIO` (the frozen tree's global intern pool, named in doc
//! comments), the identifier `unsafe_` (an engine test binding), and prose in
//! `//` comments ("unsafe heads stay opaque"). So the gate does not forbid the
//! substring; it forbids the Rust `unsafe` KEYWORD, detected by its grammar:
//! an identifier-bounded `unsafe` token, in code (comment text stripped), whose
//! next non-whitespace begins one of the keyword's grammatical continuations —
//! `{` (block), or `fn`/`impl`/`trait`/`extern` (item). Every real use of the
//! keyword takes one of these forms; prose and identifiers take none of them.
//! The result is exact for the keyword with no false positives on legitimate
//! text, and it survives future edits because it is grounded in the grammar,
//! not in the current file set.

use std::path::Path;

/// The grammatical continuations of the Rust `unsafe` keyword: the block opener
/// and the items it can prefix. A real `unsafe` is always immediately followed
/// (after whitespace) by one of these; prose and identifiers never are.
const CONTINUATIONS: &[&str] = &["{", "fn", "impl", "trait", "extern"];

/// Does `line` use the `unsafe` keyword? Comment text (everything from the
/// first `//`) is stripped first, so a `//`-comment mention never counts. A hit
/// requires an identifier-bounded `unsafe` token whose next non-whitespace
/// begins a [`CONTINUATIONS`] entry.
pub fn uses_unsafe_keyword(line: &str) -> bool {
    let code = line.split_once("//").map_or(line, |(before, _)| before);
    let bytes = code.as_bytes();
    let mut from = 0;
    while let Some(rel) = code[from..].find("unsafe") {
        let at = from + rel;
        let end = at + "unsafe".len();
        // Identifier boundary: `unsafePerformIO` (next byte `P`) and `unsafe_`
        // (next byte `_`) are identifiers, not the keyword; likewise a name
        // ENDING in `unsafe` (`bounds_unsafe`) has an identifier byte before.
        let before_ok = at == 0 || !is_ident_byte(bytes[at - 1]);
        let after_ok = end >= bytes.len() || !is_ident_byte(bytes[end]);
        if before_ok && after_ok {
            let rest = code[end..].trim_start();
            if CONTINUATIONS.iter().any(|c| rest.starts_with(c)) {
                return true;
            }
        }
        from = end;
    }
    false
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Every source line under `root` that uses the `unsafe` keyword, as
/// `path:line`.
///
/// # Panics
/// If the sweep does not reach the workspace (fewer than the floor of source
/// files) — a gate that silently examines nothing always passes.
pub fn unsafe_call_sites(root: &Path) -> Vec<String> {
    let sources = crate::source_sweep::every_rust_source(root);
    crate::source_sweep::assert_reaches_workspace(&sources, root);
    let mut offenders = Vec::new();
    for path in &sources {
        // This file carries keyword-shaped strings as the detector's own test
        // fixtures (`"unsafe fn …"`); its text states the keyword it forbids and
        // is not itself a use of it — the same self-exclusion the unchecked-split
        // gate makes for the marker it enforces.
        if path.ends_with("conformance/src/zero_unsafe_gate.rs") {
            continue;
        }
        let body = std::fs::read_to_string(path).expect("readable source");
        for (i, line) in body.lines().enumerate() {
            if uses_unsafe_keyword(line) {
                offenders.push(format!("{}:{}", path.display(), i + 1));
            }
        }
    }
    offenders
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The keyword detector fires on every grammatical form of `unsafe` and on
    /// none of the legitimate non-keyword texts the workspace actually carries.
    #[test]
    fn detector_matches_the_keyword_and_spares_prose_and_identifiers() {
        // Real uses — each grammatical continuation.
        assert!(uses_unsafe_keyword("        unsafe { ptr::read(p) }"));
        assert!(uses_unsafe_keyword("unsafe fn raw() {}"));
        assert!(uses_unsafe_keyword("pub unsafe fn raw() {}"));
        assert!(uses_unsafe_keyword("unsafe impl Send for X {}"));
        assert!(uses_unsafe_keyword("unsafe trait T {}"));
        assert!(uses_unsafe_keyword("unsafe extern \"C\" {}"));
        // Non-uses — the exact texts this workspace contains.
        assert!(!uses_unsafe_keyword("//! `IORef` pool guarded by `unsafePerformIO`/`NOINLINE`"));
        assert!(!uses_unsafe_keyword("        let unsafe_ = Action::new(\"[Actor]: act\");"));
        assert!(!uses_unsafe_keyword("    fn safe_foreach_binder_bounds_unsafe_head_opaque() {"));
        assert!(!uses_unsafe_keyword("        // ... unsafe heads stay opaque"));
        assert!(!uses_unsafe_keyword("assert!(cond, \"first-position binder is unsafe\");"));
        // `unsafePerformIO` in code position (not a comment) is still not the
        // keyword: the continuation after the identifier is not a block/item.
        assert!(!uses_unsafe_keyword("let x = unsafePerformIO(foo);"));
    }

    /// The standing net: NO workspace source uses the `unsafe` keyword. The
    /// sweep is proven non-vacuous by `assert_reaches_workspace` inside
    /// [`unsafe_call_sites`], and reported here.
    #[test]
    fn no_workspace_source_uses_the_unsafe_keyword() {
        let root = crate::source_sweep::rust_root();
        let n = crate::source_sweep::every_rust_source(&root).len();
        let offenders = unsafe_call_sites(&root);
        println!("zero-unsafe gate scanned {n} workspace .rs files");
        assert!(n > 40, "the sweep reached only {n} files — it is not scanning the workspace");
        assert!(
            offenders.is_empty(),
            "the workspace must be unsafe-free, but the `unsafe` keyword appears at:\n  {}",
            offenders.join("\n  ")
        );
    }
}
