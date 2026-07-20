//! The static well-formedness checker: unbound variables, exclusion-cardinality
//! clashes, dangling references, dead conditions, reserved-family touches, an
//! unseeded die, unclosed obligations, unmotivated coercions. Sound and
//! declaration-free (no false positives), so the report is trustworthy.
//!
//! The checker itself is S9. What lives here at S8 is the one thing S8's script
//! layer forces a decision about: the RESERVED-FAMILY LIST.
//!
//! Frozen reference: `Prax.TypeCheck.reservedFamilies`, a top-level constant
//! `[turnPath, "contradiction", scenePatienceFamily, currentScenePath]` consulted
//! for EVERY state, script-compiled or not. Two of its four members are declared
//! in `Prax.Script` and imported here; the Rust crate graph forbids
//! `prax-core → prax-script` (the cycle argument that split `obliged_close` at
//! S4), so the two script constants live HERE, at the checker, and `prax-script`
//! reads them from here. One home each, and the list stays a WORLD-INDEPENDENT
//! constant exactly as the frozen one is.
//!
//! The alternative the S8 design first proposed — a per-`State` list grown
//! through an engine door when a script compiles — was rejected by the design
//! panel [D-C2] and is not implemented: it would make membership a property of
//! how a world was BUILT, so a NON-script world authoring `scenePatience` would
//! be flagged by the frozen checker and not by the Rust one. That is a semantic
//! divergence, and the already-booked owed:S9 row on `ScheduleRuleSpec`'s
//! provenance verdict pins exactly the case it would break (`emptyState`, no
//! prax-script anywhere). With the list static there is nothing left for a
//! `register_reserved_families` door to do, so no such door exists.

/// The engine clock's fact family (`Prax.Types.turnPath`).
pub const TURN_PATH: &str = "turn";

/// The detected-contradiction marker's family (the frozen `reservedFamilies`
/// spells this one as a literal too).
pub const CONTRADICTION_PATH: &str = "contradiction";

/// The timed-junction patience-marker family (`Prax.Script.scenePatienceFamily`,
/// spec v50): a timed junction `j` of scene `sid` carries the fact
/// `scenePatience.<sid>.<j>`, armed with lifetime `n` on scene entry and
/// retracted `n` boundaries later by the v44 expiry schedule. Compiler
/// machinery, not fiction — produced only by the script compiler's scene-entry
/// fold, read only by the compiled `story` rule, and closed to authors.
pub const SCENE_PATIENCE_FAMILY: &str = "scenePatience";

/// The current-scene fact family (`Prax.Script.currentScenePath`, spec v46): the
/// single-slot fact `currentScene!<id>` names the active scene. Compiler-emitted
/// and literal-tailed with exactly one legitimate writer, so it is reserved: no
/// authored surface may write it.
pub const CURRENT_SCENE_PATH: &str = "currentScene";

/// The families no AUTHORED rule, action or desire may write — the checker's
/// world-independent constant (`Prax.TypeCheck.reservedFamilies`). Rules
/// installed through the compiler-level door
/// ([`crate::engine::door::register_engine_rules`]) are exempt by PROVENANCE
/// (v53), which is what [`crate::engine::State::engine_rule_names`] records; the
/// family list itself never varies by world.
pub const RESERVED_FAMILIES: [&str; 4] = [
    TURN_PATH,
    CONTRADICTION_PATH,
    SCENE_PATIENCE_FAMILY,
    CURRENT_SCENE_PATH,
];

/// The patience marker for timed junction `jname` of scene `sid`
/// (`Prax.Script.scenePatiencePath`). Lives beside the family constant it is
/// built from, so the two cannot desync; the script compiler is its only writer.
pub fn scene_patience_path(sid: &str, jname: &str) -> String {
    format!("{SCENE_PATIENCE_FAMILY}.{sid}.{jname}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// NATIVE PIN — no frozen label. The frozen suite never asserts the CONTENT
    /// of `reservedFamilies`; it asserts consequences of it through `typeCheck`,
    /// which has no Rust twin until S9. Until then this list is the whole
    /// contract, and the S9 checker will be written against it, so it is pinned
    /// here by content and by ORDER (the order a diagnostic would enumerate).
    ///
    /// REDDENS UNDER (both verified): replacing `SCENE_PATIENCE_FAMILY` with a
    /// look-alike — the [D-C2] failure mode, a checker that no longer refuses an
    /// authored patience write — and reordering the four. DROPPING a member does
    /// not reach this assertion at all: the fixed-length `[&str; 4]` makes it a
    /// compile error, which is the stronger outcome and is why the type is an
    /// array rather than a slice.
    #[test]
    fn the_reserved_family_list_is_the_frozen_four_in_order() {
        assert_eq!(
            RESERVED_FAMILIES,
            ["turn", "contradiction", "scenePatience", "currentScene"],
            "Prax.TypeCheck.reservedFamilies is a fixed list of four, consulted \
             for every state -- script-compiled or not"
        );
    }

    /// NATIVE PIN — no frozen label (`scenePatiencePath` is private to the
    /// frozen `Prax.Script`, so nothing frozen names it). The compiled `story`
    /// rule's `Not` guard and the scene-entry `InsertFor` must agree on this
    /// spelling or a timed junction never fires; both build it from here.
    ///
    /// REDDENS UNDER: swapping the two interpolations (`sid`/`jname`).
    #[test]
    fn the_patience_marker_path_keys_scene_then_junction() {
        assert_eq!(
            scene_patience_path("audience", "dismissed"),
            "scenePatience.audience.dismissed"
        );
    }
}
