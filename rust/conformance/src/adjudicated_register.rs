//! The adjudicated-divergence register's ANTI-DRIFT GATE (S7 design §1.7 law 3).
//!
//! Two registers record the same facts from two directions:
//! `conformance/ADJUDICATED.json` tells the comparator what to suppress, and
//! `docs/rewrite/DIVERGENCES.md` tells a human WHY the Rust is right and the
//! Haskell is not. Either one alone can rot: a suppression with no written
//! adjudication is a divergence quietly swept under the matrix, and a written
//! adjudication with a stale suppression drowns fresh signal at a coordinate
//! nobody is watching any more.
//!
//! So the two are held in BIJECTION, mechanically: every ADJUDICATED `id` must
//! be a `## DIV-n` heading in DIVERGENCES.md that DECLARES a suppression, and
//! every such heading must have exactly one register entry. Neither register
//! grows without the other.
//!
//! The register ships EMPTY, so this test currently asserts the empty bijection
//! — which is the point: it is in place BEFORE the first entry, not written
//! after one has already drifted.

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        // The conformance crate lives at <root>/rust/conformance.
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("the crate is two levels under the repo root")
            .to_path_buf()
    }

    /// Every `id` in the register.
    fn register_ids() -> BTreeSet<String> {
        let path = repo_root().join("conformance/ADJUDICATED.json");
        let body = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "the adjudicated-divergence register is missing at {}: {e}. It ships EMPTY \
                 (`{{\"format\": 1, \"entries\": []}}`) — an absent file is not the same as an empty \
                 one, because the comparator would then have nothing to fail on.",
                path.display()
            )
        });
        let v: serde_json::Value =
            serde_json::from_str(&body).unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()));
        assert_eq!(
            v.get("format").and_then(serde_json::Value::as_i64),
            Some(1),
            "ADJUDICATED.json must declare format 1"
        );
        v.get("entries")
            .and_then(serde_json::Value::as_array)
            .expect("ADJUDICATED.json has an `entries` array")
            .iter()
            .map(|e| {
                e.get("id")
                    .and_then(serde_json::Value::as_str)
                    .expect("every entry has a string `id`")
                    .to_owned()
            })
            .collect()
    }

    /// Every `## DIV-n` heading in DIVERGENCES.md whose section DECLARES a
    /// suppression. The declaration is the line `**Comparator posture**:` followed
    /// by anything other than "no suppression" — the two shipped entries (DIV-1,
    /// DIV-2) both say "no suppression", which is exactly why the register is empty.
    fn declared_suppressions() -> BTreeSet<String> {
        let path = repo_root().join("docs/rewrite/DIVERGENCES.md");
        let body = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let mut out = BTreeSet::new();
        let mut current: Option<String> = None;
        for line in body.lines() {
            if let Some(rest) = line.strip_prefix("## ") {
                current = rest
                    .split_whitespace()
                    .next()
                    .filter(|w| w.starts_with("DIV-"))
                    .map(|w| w.trim_end_matches(':').to_owned());
            }
            if let Some(id) = &current
                && let Some(posture) = line.strip_prefix("**Comparator posture**:")
                && !posture.to_lowercase().contains("no suppression")
            {
                out.insert(id.clone());
            }
        }
        out
    }

    #[test]
    fn every_suppression_is_declared_in_both_registers() {
        let reg = register_ids();
        let docs = declared_suppressions();
        let only_reg: Vec<&String> = reg.difference(&docs).collect();
        let only_docs: Vec<&String> = docs.difference(&reg).collect();
        assert!(
            only_reg.is_empty(),
            "ADJUDICATED.json suppresses {only_reg:?}, but DIVERGENCES.md declares no matching \
             `## DIV-n` section with a suppressing **Comparator posture**. A suppression with no \
             written adjudication is a divergence swept under the matrix."
        );
        assert!(
            only_docs.is_empty(),
            "DIVERGENCES.md declares suppressions for {only_docs:?}, but ADJUDICATED.json has no \
             matching entry. A declared suppression the comparator does not implement means the \
             matrix is reporting DIVERGENT for something already adjudicated — or, worse, that the \
             entry was deleted and nobody noticed."
        );
    }

    #[test]
    fn the_register_ships_empty_and_the_shipped_divergences_declare_no_suppression() {
        // The prediction §1.7 makes, held as a red/green test rather than prose: the
        // two adjudicated divergences (DIV-1's incomplete semi-naive closure, DIV-2's
        // separator-bearing character name) change what the engines ACCEPT, not what
        // any shipped trace CONTAINS — so no suppression is needed and no-op entries
        // would lie about the mechanism. When the first real suppression lands, this
        // test is the one that has to be deliberately updated.
        assert!(
            register_ids().is_empty(),
            "the register is no longer empty — update this test alongside the entry, and make sure \
             the mutation evidence in prax-oracle's register.rs still covers the new shape"
        );
        assert!(declared_suppressions().is_empty());
    }
}
