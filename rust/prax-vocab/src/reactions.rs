//! Reactions and norms (Versu paper §X and §VIII-D).
//!
//! Reactivity is an ordinary social practice: performing an action SPAWNS a
//! reaction practice keyed on its participants, offering the affected characters
//! responses; a response is itself an action, which can spawn further reactions,
//! and consumes the instance by deleting it. Norms use the same machinery — a
//! norm-violating action marks a violation and spawns a reaction (disapproval)
//! offering bystanders a response.
//!
//! Like [`crate::core_model`] this is a reusable library over the existing
//! engine; it adds NO machinery. A world registers its own reaction practices
//! (e.g. `prax_worlds::bar`'s `disapproval`) with
//! [`prax_core::engine::State::define_practices`].
//!
//! Frozen reference: `src/Prax/Reactions.hs`. Signature for signature: one Rust
//! fn per exported Haskell fn, same parameter order and arity, `&str` params.
//! The module's own path helpers ([`reaction_path`], and the private
//! `violation_path`) are ported, never replaced (S7 design §3) — the frozen
//! module builds them with `Data.List.intercalate` over the same segments, and
//! neither raises: no `error`, no `pathNames`, no clash guard anywhere in the
//! frozen module or one call deeper (`intercalate` is total).

use prax_core::query::{Condition, matches};
use prax_core::types::{Outcome, delete, insert};

/// The DB path of a reaction instance: `practice.<id>.<part…>`. Parts may be
/// constants or action variables (grounded when the outcome runs).
pub fn reaction_path(pid: &str, parts: &[&str]) -> String {
    let mut segs: Vec<&str> = Vec::with_capacity(parts.len() + 2);
    segs.push("practice");
    segs.push(pid);
    segs.extend_from_slice(parts);
    segs.join(".")
}

/// Spawn a reaction practice instance (offer its responses to the participants).
pub fn spawn_reaction(pid: &str, parts: &[&str]) -> Outcome {
    insert(reaction_path(pid, parts))
}

/// Consume a reaction instance (a response has been taken).
pub fn end_reaction(pid: &str, parts: &[&str]) -> Outcome {
    delete(reaction_path(pid, parts))
}

/// Condition: this reaction instance is currently pending.
pub fn reaction_active(pid: &str, parts: &[&str]) -> Condition {
    matches(reaction_path(pid, parts))
}

/// Where a norm violation by `who` is recorded. Private, exactly as the frozen
/// `violationPath` is: the exported surface is [`mark_violation`]/
/// [`violation_of`], and porting the helper rather than inlining a `format!` at
/// the two call sites is what keeps them moving together (§3).
fn violation_path(who: &str, norm: &str) -> String {
    ["violated", who, norm].join(".")
}

/// Record that `who` violated the named `norm`. Agents are given strong negative
/// wants on their own violations, so the planner avoids causing them.
pub fn mark_violation(who: &str, norm: &str) -> Outcome {
    insert(violation_path(who, norm))
}

/// Condition matching a recorded violation of `norm` by `who`.
pub fn violation_of(who: &str, norm: &str) -> Condition {
    matches(violation_path(who, norm))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_model::{WARMTH, adjust_score, core_fns};
    use crate::emotion::{ANNOYED, PLEASED, feel_toward_for};
    use prax_core::engine::{GroundedAction, State};
    use prax_core::query::eq;
    use prax_core::types::{Action, Character, Practice, Want};

    // H: ReactionsSpec.hs "Prax.Reactions"
    //
    // The frozen `Prax.ReactionsSpec`, re-expressed against the Rust engine.
    // The fixture practices below are the spec's own (`disapprovalFixture`,
    // `chainerP`) — deliberately independent of any shipped world's content, so
    // the mechanism is pinned on its own.

    /// The spec's local disapproval fixture, atom for atom.
    fn disapproval_fixture() -> Practice {
        Practice::new("disapproval")
            .name("[Onlooker] saw [Offender] break a norm")
            .roles(["Offender", "Onlooker"])
            .action(
                Action::new("[Actor]: Disapprove of [Offender]")
                    .when([eq("Actor", "Onlooker")])
                    .then([
                        insert("Onlooker.disapprovedOf.Offender"),
                        feel_toward_for(4, "Onlooker", ANNOYED, "Offender"),
                        adjust_score("Onlooker", "Offender", WARMTH, -20, "brokeANorm"),
                        end_reaction("disapproval", &["Offender", "Onlooker"]),
                    ]),
            )
            .action(
                Action::new("[Actor]: Let [Offender]'s lapse slide")
                    .when([eq("Actor", "Onlooker")])
                    .then([
                        feel_toward_for(4, "Onlooker", PLEASED, "Offender"),
                        end_reaction("disapproval", &["Offender", "Onlooker"]),
                    ]),
            )
    }

    /// A reaction whose response spawns a further (disapproval) reaction.
    fn chainer_practice() -> Practice {
        Practice::new("chainer")
            .name("[A] provoked [B]")
            .roles(["A", "B"])
            .action(
                Action::new("[Actor]: React to [A]")
                    .when([eq("Actor", "B")])
                    .then([
                        spawn_reaction("disapproval", &["A", "B"]),
                        end_reaction("chainer", &["A", "B"]),
                    ]),
            )
    }

    fn base() -> State {
        let mut st = State::new();
        st.define_practices([disapproval_fixture()]).unwrap();
        st.define_functions(core_fns()).unwrap();
        st
    }

    fn labels(st: &mut State, actor: &str) -> Vec<String> {
        st.possible_actions(actor)
            .into_iter()
            .map(|ga| ga.label)
            .collect()
    }

    fn find(st: &mut State, actor: &str, needle: &str) -> GroundedAction {
        let had = labels(st, actor);
        st.possible_actions(actor)
            .into_iter()
            .find(|ga| ga.label.contains(needle))
            .unwrap_or_else(|| panic!("no action matching {needle:?} for {actor}; had: {had:?}"))
    }

    fn act(st: &mut State, actor: &str, needle: &str) {
        let ga = find(st, actor, needle);
        st.perform_action(&ga);
    }

    // H: ReactionsSpec.hs "path helpers"
    // H: ReactionsSpec.hs "reactionPath builds practice.<id>.<parts>"
    #[test]
    fn reaction_path_builds_practice_id_parts() {
        assert_eq!(
            reaction_path("settleUp", &["bex", "ada"]),
            "practice.settleUp.bex.ada"
        );
    }

    // H: ReactionsSpec.hs "violationOf builds the expected match"
    #[test]
    fn violation_of_builds_the_expected_match() {
        assert_eq!(
            violation_of("bex", "tipping"),
            Condition::Match("violated.bex.tipping".to_owned())
        );
    }

    // H: ReactionsSpec.hs "disapproval reaction"
    // H: ReactionsSpec.hs "spawning offers the response only to the onlooker"
    #[test]
    fn spawning_offers_the_response_only_to_the_onlooker() {
        let mut st = base();
        st.perform_outcome(&spawn_reaction("disapproval", &["bex", "ada"]))
            .unwrap();
        assert!(
            labels(&mut st, "ada")
                .iter()
                .any(|l| l.contains("Disapprove of bex")),
            "the onlooker can disapprove"
        );
        assert!(
            !labels(&mut st, "bex")
                .iter()
                .any(|l| l.contains("Disapprove")),
            "the offender cannot"
        );
    }

    // H: ReactionsSpec.hs "disapproving cools the relationship and consumes the reaction"
    #[test]
    fn disapproving_cools_the_relationship_and_consumes_the_reaction() {
        let mut st = base();
        st.perform_outcome(&spawn_reaction("disapproval", &["bex", "ada"]))
            .unwrap();
        act(&mut st, "ada", "Disapprove of bex");
        let fs = st.labeled_facts();
        assert!(
            fs.contains(&"ada.feels.annoyed.toward.bex".to_owned()),
            "ada is annoyed at bex, got {fs:?}"
        );
        assert!(
            fs.contains(&"ada.relationship.bex.warmth.score!-20".to_owned()),
            "warmth cooled, got {fs:?}"
        );
        assert!(
            !fs.contains(&"practice.disapproval.bex.ada".to_owned()),
            "the reaction instance is consumed"
        );
        assert!(
            !labels(&mut st, "ada")
                .iter()
                .any(|l| l.contains("Disapprove")),
            "no disapproval option remains"
        );
    }

    // H: ReactionsSpec.hs "forgiving also consumes the reaction (no cooling)"
    #[test]
    fn forgiving_also_consumes_the_reaction() {
        let mut st = base();
        st.perform_outcome(&spawn_reaction("disapproval", &["bex", "ada"]))
            .unwrap();
        act(&mut st, "ada", "Let bex's lapse slide");
        let fs = st.labeled_facts();
        assert!(
            !fs.contains(&"practice.disapproval.bex.ada".to_owned()),
            "the reaction instance is consumed"
        );
        assert!(
            !fs.iter()
                .any(|s| s.contains("ada.relationship.bex.warmth.score!-")),
            "no warmth penalty, got {fs:?}"
        );
    }

    // H: ReactionsSpec.hs "norm violations"
    // H: ReactionsSpec.hs "markViolation records the fact"
    #[test]
    fn mark_violation_records_the_fact() {
        let mut st = base();
        st.perform_outcome(&mark_violation("bex", "tipping"))
            .unwrap();
        assert!(
            st.labeled_facts()
                .contains(&"violated.bex.tipping".to_owned())
        );
    }

    // H: ReactionsSpec.hs "an agent avoids an action that violates a norm it wants to respect"
    #[test]
    fn an_agent_avoids_an_action_that_violates_a_norm_it_wants_to_respect() {
        let conduct = Practice::new("conduct")
            .name("conduct")
            .roles(["X"])
            .action(Action::new("[Actor]: Behave"))
            .action(Action::new("[Actor]: Misbehave").then([mark_violation("Actor", "tipping")]));
        let bex = Character::new("bex").want(Want::new(vec![violation_of("bex", "tipping")], -50));
        let mut st = State::new();
        st.define_practices([conduct]).unwrap();
        st.define_functions(core_fns()).unwrap();
        st.set_characters(vec![bex.clone()]).unwrap();
        st.perform_outcome(&spawn_reaction("conduct", &["bex"]))
            .unwrap();
        let ls = labels(&mut st, "bex");
        assert!(ls.iter().any(|l| l.contains("Behave")), "can behave");
        assert!(ls.iter().any(|l| l.contains("Misbehave")), "can misbehave");
        assert_eq!(
            st.pick_action(1, &bex).map(|g| g.label),
            Some("bex: Behave".to_owned()),
            "the violation future scores -50, so the planner complies"
        );
    }

    // H: ReactionsSpec.hs "chaining"
    // H: ReactionsSpec.hs "a response can spawn a further reaction"
    #[test]
    fn a_response_can_spawn_a_further_reaction() {
        let mut st = State::new();
        st.define_practices([disapproval_fixture(), chainer_practice()])
            .unwrap();
        st.define_functions(core_fns()).unwrap();
        st.perform_outcome(&spawn_reaction("chainer", &["bex", "ada"]))
            .unwrap();
        act(&mut st, "ada", "React to bex");
        let fs = st.labeled_facts();
        assert!(
            !fs.contains(&"practice.chainer.bex.ada".to_owned()),
            "the original reaction is consumed"
        );
        assert!(
            fs.contains(&"practice.disapproval.bex.ada".to_owned()),
            "the follow-up disapproval is spawned, got {fs:?}"
        );
    }

    /// [`reaction_active`] carries no frozen spec label of its own, but it IS
    /// exported and the shipped bar negates exactly it (`Not (reactionPath
    /// "respondGreet" …)` gates a fresh greeting while a response is owed).
    /// Pinned here as the `Match` twin of [`reaction_path`]: an unpinned export
    /// is one a rename can silently desync from its own path helper.
    #[test]
    fn reaction_active_is_the_match_over_the_same_path() {
        assert_eq!(
            reaction_active("respondGreet", &["you", "ada"]),
            Condition::Match(reaction_path("respondGreet", &["you", "ada"]))
        );
        // …and the spawn/active pair round-trips through the engine, so this is
        // an agreement about the DB, not merely about two strings.
        let mut st = base();
        assert!(!st.db_has("practice.disapproval.bex.ada"));
        st.perform_outcome(&spawn_reaction("disapproval", &["bex", "ada"]))
            .unwrap();
        assert!(st.db_has("practice.disapproval.bex.ada"));
    }
}
