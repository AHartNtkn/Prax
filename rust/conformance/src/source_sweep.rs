//! The workspace source sweep the repo-wide source gates share.
//!
//! Two gates walk every `.rs` file under `rust/` and judge its lines: the
//! inline-golden gate ([`crate::goldens`]) and the unchecked-split gate
//! ([`crate::unchecked_split_gate`]). They walk the same tree for the same
//! reason — a rule that holds only inside the crate that states it is a rule
//! whoever adds the next crate never sees — so the walk lives once, here.

use std::path::{Path, PathBuf};

/// The `rust/` workspace root, canonicalized (this crate's manifest dir is
/// `rust/conformance`).
///
/// # Panics
/// If the workspace root does not exist, which would mean the sweep is pointed
/// somewhere other than the repository.
pub fn rust_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("the rust/ workspace root")
}

/// Every `.rs` file under `dir`, recursively, in a deterministic order.
/// `target/` is build output, not source.
///
/// # Panics
/// If a directory cannot be read — an unreadable source tree is a broken gate,
/// not a gate that passes.
pub fn every_rust_source(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(dir, &mut out);
    out.sort();
    out
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("a dir entry").path();
        if path.is_dir() {
            if path.file_name().is_some_and(|f| f == "target") {
                continue;
            }
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Assert the sweep actually reached the workspace. A gate that silently
/// examines nothing is a gate that always passes.
///
/// # Panics
/// If fewer than 40 source files were found.
pub fn assert_reaches_workspace(sources: &[PathBuf], root: &Path) {
    assert!(
        sources.len() > 40,
        "the sweep found only {} files under {} — it is not reaching the workspace",
        sources.len(),
        root.display()
    );
}
