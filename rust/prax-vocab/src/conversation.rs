//! Conversation: speakers, topics, and quips (Versu paper §X).
//!
//! A conversation is a two-party practice with a SELECTED SPEAKER (turn-taking)
//! and a single current TOPIC. A **quip** is an ordinary action that says a line
//! and applies effects — on the core model and on beliefs — exactly the
//! machinery of [`crate::core_model`] and [`crate::beliefs`]: "a response is just
//! a normal type of action … the same utility-planner." Characters stay on topic
//! (a quip is only available on its topic) until someone deliberately changes
//! the subject.
//!
//! A reusable library over the existing engine (no new machinery): the
//! conversation is `practice.converse.<A>.<B>` with `speaker!`, `listener!`,
//! `topic!` facts. A world supplies the concrete quips via [`quip`] /
//! [`change_subject`] / [`end_conversation`] inside a practice with roles
//! `["A", "B"]`.
//!
//! Frozen reference: `src/Prax/Conversation.hs`. Signature for signature, and
//! the module's own helpers — [`talk_path`], and the private `CONV_P` /
//! `speaker_conds` / `pass_turn` — are ported rather than replaced (§3):
//! `CONV_P` is the frozen `convP`, the WITHIN-practice instance path written
//! over the role variables `A`/`B`, and is deliberately NOT
//! `talk_path("A", "B")` at the call sites, because that is not how the frozen
//! module spells it. Nothing here raises: no `error`, no `pathNames`, no clash
//! guard, in this module or one call deeper.

use prax_core::query::{Condition, matches, not_};
use prax_core::types::{Action, Outcome, delete, insert};

/// Path of a conversation instance between `x` and `y` (in this order).
pub fn talk_path(x: &str, y: &str) -> String {
    format!("practice.converse.{x}.{y}")
}

/// Condition: `x` and `y` are conversing (this ordering).
pub fn talking_with(x: &str, y: &str) -> Condition {
    matches(talk_path(x, y))
}

/// Outcomes that open a conversation: `opener` becomes the first speaker,
/// `other` the listener, starting on `topic`. Records `topic` as visited and
/// marks the pair as having chatted (so a world can prevent re-opening).
pub fn begin_conversation(opener: &str, other: &str, topic: &str) -> Vec<Outcome> {
    let base = talk_path(opener, other);
    vec![
        insert(base.clone()),
        insert(format!("{base}.speaker!{opener}")),
        insert(format!("{base}.listener!{other}")),
        insert(format!("{base}.topic!{topic}")),
        insert(format!("{base}.visited.{topic}")),
        insert(format!("{opener}.chattedWith.{other}")),
        insert(format!("{other}.chattedWith.{opener}")),
    ]
}

/// Within-practice actions reference the instance by its role variables A and B.
const CONV_P: &str = "practice.converse.A.B";

/// The current speaker is the actor; bind the listener as `Partner`.
fn speaker_conds() -> Vec<Condition> {
    vec![
        matches(format!("{CONV_P}.speaker!Actor")),
        matches(format!("{CONV_P}.listener!Partner")),
    ]
}

/// Hand the floor to the other party.
fn pass_turn() -> Vec<Outcome> {
    vec![
        insert(format!("{CONV_P}.speaker!Partner")),
        insert(format!("{CONV_P}.listener!Actor")),
    ]
}

/// A quip: a line the current speaker can say once, on the given `topic`, with
/// `effects` (and any extra `conds`). Saying it passes the turn.
///
/// `key` is a short unique tag recording that this quip has been said (so it is
/// one-shot per speaker); `label` is the displayed line template.
pub fn quip(
    key: &str,
    label: &str,
    topic: &str,
    conds: Vec<Condition>,
    effects: Vec<Outcome>,
) -> Action {
    let mut when = speaker_conds();
    when.push(matches(format!("{CONV_P}.topic!{topic}")));
    when.push(not_(format!("{CONV_P}.said.{key}.Actor")));
    when.extend(conds);

    let mut then = vec![insert(format!("{CONV_P}.said.{key}.Actor"))];
    then.extend(effects);
    then.extend(pass_turn());

    Action::new(label).when(when).then(then)
}

/// Steer the conversation onto `new_topic` (only if not already there and not
/// already covered). Passes the turn.
pub fn change_subject(label: &str, new_topic: &str) -> Action {
    let mut when = speaker_conds();
    when.push(not_(format!("{CONV_P}.topic!{new_topic}")));
    when.push(not_(format!("{CONV_P}.visited.{new_topic}")));

    let mut then = vec![
        insert(format!("{CONV_P}.topic!{new_topic}")),
        insert(format!("{CONV_P}.visited.{new_topic}")),
    ];
    then.extend(pass_turn());

    Action::new(label).when(when).then(then)
}

/// End the conversation (removes the instance).
pub fn end_conversation(label: &str) -> Action {
    Action::new(label)
        .when(speaker_conds())
        .then([delete(CONV_P)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_core::engine::{GroundedAction, State};
    use prax_core::types::Practice;

    // H: ConversationSpec.hs "Prax.Conversation"
    //
    // The frozen `Prax.ConversationSpec`, re-expressed against the Rust engine,
    // over the spec's own minimal `talkP` fixture.

    fn talk_practice() -> Practice {
        Practice::new("converse")
            .name("[A] and [B] are talking")
            .roles(["A", "B"])
            .action(quip(
                "hi",
                "[Actor]: say hi to [Partner]",
                "greetings",
                vec![],
                vec![insert("Actor.saidHiTo.Partner")],
            ))
            .action(quip(
                "weather",
                "[Actor]: remark on the weather to [Partner]",
                "weather",
                vec![],
                vec![insert("Actor.talkedWeatherWith.Partner")],
            ))
            .action(change_subject(
                "[Actor]: change the subject to the weather",
                "weather",
            ))
            .action(end_conversation("[Actor]: wrap up the chat"))
    }

    fn start() -> State {
        let mut st = State::new();
        st.define_practice(talk_practice()).unwrap();
        for o in begin_conversation("ada", "bex", "greetings") {
            st.perform_outcome(&o).expect("opening the conversation");
        }
        st
    }

    fn opts(st: &mut State, actor: &str) -> Vec<String> {
        st.possible_actions(actor)
            .into_iter()
            .map(|ga| ga.label)
            .collect()
    }

    fn find(st: &mut State, actor: &str, needle: &str) -> GroundedAction {
        let had = opts(st, actor);
        st.possible_actions(actor)
            .into_iter()
            .find(|ga| ga.label.contains(needle))
            .unwrap_or_else(|| panic!("no {needle:?} for {actor}; had: {had:?}"))
    }

    fn act(st: &mut State, actor: &str, needle: &str) {
        let ga = find(st, actor, needle);
        st.perform_action(&ga);
    }

    // H: ConversationSpec.hs "beginConversation sets speaker, listener and topic"
    #[test]
    fn begin_conversation_sets_speaker_listener_and_topic() {
        let fs = start().labeled_facts();
        for want in [
            "practice.converse.ada.bex.speaker!ada",
            "practice.converse.ada.bex.listener!bex",
            "practice.converse.ada.bex.topic!greetings",
        ] {
            assert!(fs.contains(&want.to_owned()), "{want} missing from {fs:?}");
        }
    }

    // H: ConversationSpec.hs "only the current speaker may quip, and only on the current topic"
    #[test]
    fn only_the_current_speaker_may_quip_and_only_on_topic() {
        let mut st = start();
        assert!(
            opts(&mut st, "ada").iter().any(|l| l.contains("say hi to bex")),
            "the speaker can say hi"
        );
        let listener = opts(&mut st, "bex");
        for needle in ["say hi", "remark on the weather"] {
            assert!(
                !listener.iter().any(|l| l.contains(needle)),
                "the listener cannot quip ({needle}), had {listener:?}"
            );
        }
        assert!(
            !opts(&mut st, "ada")
                .iter()
                .any(|l| l.contains("remark on the weather")),
            "the off-topic quip is withheld"
        );
    }

    // H: ConversationSpec.hs "a quip applies its effect, is one-shot, and passes the turn"
    #[test]
    fn a_quip_applies_its_effect_is_one_shot_and_passes_the_turn() {
        let mut st = start();
        act(&mut st, "ada", "say hi to bex");
        let fs = st.labeled_facts();
        assert!(
            fs.contains(&"ada.saidHiTo.bex".to_owned()),
            "the effect applied, got {fs:?}"
        );
        assert!(
            fs.contains(&"practice.converse.ada.bex.speaker!bex".to_owned()),
            "the turn passed to bex, got {fs:?}"
        );
        assert!(
            !opts(&mut st, "ada").iter().any(|l| l.contains("say hi")),
            "ada cannot quip out of turn"
        );
    }

    // H: ConversationSpec.hs "changing the subject switches the active topic"
    #[test]
    fn changing_the_subject_switches_the_active_topic() {
        let mut st = start();
        act(&mut st, "ada", "say hi to bex");
        act(&mut st, "bex", "change the subject to the weather");
        assert!(
            st.labeled_facts()
                .contains(&"practice.converse.ada.bex.topic!weather".to_owned()),
            "the topic is now the weather"
        );
        assert!(
            opts(&mut st, "ada")
                .iter()
                .any(|l| l.contains("remark on the weather")),
            "the on-topic quip is offered to the speaker"
        );
    }

    // H: ConversationSpec.hs "ending the conversation removes the instance"
    #[test]
    fn ending_the_conversation_removes_the_instance() {
        let mut st = start();
        act(&mut st, "ada", "wrap up the chat");
        assert!(
            !st.labeled_facts()
                .contains(&"practice.converse.ada.bex".to_owned()),
            "the conversation instance is gone"
        );
    }

    /// [`talking_with`] carries no frozen spec label; the shipped bar negates it
    /// in BOTH orders to gate striking up a chat (`Not (talkPath "Actor"
    /// "Other")`, `Not (talkPath "Other" "Actor")`), which is only correct
    /// because the path is order-bearing. Pinned as the `Match` twin of
    /// [`talk_path`], with the asymmetry asserted against the engine.
    #[test]
    fn talking_with_is_the_ordered_match_over_the_talk_path() {
        assert_eq!(
            talking_with("ada", "bex"),
            Condition::Match(talk_path("ada", "bex"))
        );
        assert_ne!(talk_path("ada", "bex"), talk_path("bex", "ada"));
        let mut st = start();
        assert!(st.db_has(&talk_path("ada", "bex")));
        assert!(
            !st.db_has(&talk_path("bex", "ada")),
            "the reverse ordering is a different instance — which is why the \
             shipped gate has to negate both"
        );
    }
}
