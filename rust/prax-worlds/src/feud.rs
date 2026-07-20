//! An __emergent sandbox__ demo (the direction the engine is heading — a social
//! substrate for other games, not authored IF). It shows the derivation layer
//! ([`prax_core::derive`]) doing the work: from *one* authored fact and a handful
//! of domain rules, a whole feud emerges — people who never met come to resent
//! someone purely through the alliance network — and it is __defeasible__: make
//! amends and the enmity dissolves on its own.
//!
//! Setup (v31): Alice wronged Bob; Bob, Carol, and Dave are house kestrel
//! ([`prax_vocab::faction`] membership — [`prax_vocab::faction::comrades`] derives
//! their alliances); Esme is house wren, marriageable across the lines
//! ([`prax_vocab::kin`]). The rules say alliances are mutual, the wronged resent
//! the wrongdoer, and "the enemy of my ally is my enemy". Forward-chaining (the
//! read view) then derives `resents.bob.alice`, `resents.carol.alice`,
//! `resents.dave.alice` — Alice is shunned by Bob's entire network though she
//! only ever wronged Bob. Retract the wrong (Alice "makes amends") and every
//! derived `resents` — and the shunning — vanishes.
//!
//! Frozen reference: `src/Prax/Worlds/Feud.hs`. Construction ORDER is part of the
//! port: the axioms are set AFTER the setup outcomes, exactly as the frozen
//! world folds them, and `worldshape feud` compares the whole post-setup state.

use prax_core::engine::State;
use prax_core::query::{matches, neq, not_};
use prax_core::types::{Action, Axiom, Character, Practice, Want, delete, insert};
use prax_vocab::faction::{comrades, joins};
use prax_vocab::kin::kin_axioms;

/// You are Alice — the one who gave offence.
pub const PLAYER_NAME: &str = "alice";

/// Domain knowledge, authored once. Reads see the forward-chained closure.
pub fn feud_axioms() -> Vec<Axiom> {
    vec![
        // alliances are mutual
        Axiom::new(vec![matches("allied.X.Y")], ["allied.Y.X"]),
        // the wronged resent the wrongdoer
        Axiom::new(vec![matches("wronged.X.Y")], ["resents.Y.X"]),
        // the enemy of my ally is my enemy
        Axiom::new(
            vec![matches("resents.A.B"), matches("allied.A.C")],
            ["resents.C.B"],
        ),
        // shared membership derives allied (prax_vocab::faction)
        comrades(),
    ]
}

/// The one place everyone shares. Affordances key off *derived* enmity.
fn society_practice() -> Practice {
    Practice::new("society")
        .name("the company keeps its grudges")
        .roles(["Room"])
        // available only to someone who resents the target — and `resents` is
        // almost always a derived fact, not an authored one
        .action(
            Action::new("[Actor]: shun [Target]")
                .when([
                    matches("resents.Actor.Target"),
                    neq("Actor", "Target"),
                    not_("shunned.Actor.Target"),
                ])
                .then([insert("shunned.Actor.Target")]),
        )
        // the wrongdoer can make amends: retracting the base wrong dissolves
        // every resentment derived from it (defeasibility as a game move)
        .action(
            Action::new("[Actor]: make amends with [Target]")
                .when([matches("wronged.Actor.Target")])
                .then([delete("wronged.Actor.Target")]),
        )
}

/// Someone who acts on their grudges: wants to shun anyone they (come to)
/// resent.
fn grudge_bearer(n: &str) -> Character {
    Character::new(n).want(Want::new(vec![matches(format!("shunned.{n}.Target"))], 5))
}

/// Alice — dislikes being shunned.
fn alice() -> Character {
    Character::new(PLAYER_NAME).want(Want::new(vec![matches("shunned.Other.alice")], -10))
}

/// The set-up sandbox: three domain rules + one act of offence, and a feud
/// assembles itself. Bob, carol and dave share one house (kestrel);
/// [`comrades`] derives the `allied` facts the old world used to assert
/// directly. Esme starts in a house of her own (wren) — inert to the feud until
/// a wedding moves her in. All of [`kin_axioms`] is wired in wholesale here —
/// the families are namespace-disjoint, so inclusion is free and the wedding
/// needs only the marriage rules; an authored choice for this world, not a spec
/// mandate — harmless to the unmodified feud tests since no `parent.*`/
/// `married.*` base fact exists until a wedding inserts one.
///
/// # Panics
/// If a construction guard rejects the authored content — a bug in this file,
/// not a condition a world can handle.
pub fn feud_world() -> State {
    let mut st = State::new();
    st.define_practices([society_practice()])
        .expect("feud practices");
    st.set_characters(vec![
        alice(),
        grudge_bearer("bob"),
        grudge_bearer("carol"),
        grudge_bearer("dave"),
        grudge_bearer("esme"),
    ])
    .expect("feud cast");
    let setup = [
        insert("practice.society.here"),
        insert("wronged.alice.bob"), // the single authored grievance
        joins("bob", "kestrel").expect("bob's house"),
        joins("carol", "kestrel").expect("carol's house"),
        joins("dave", "kestrel").expect("dave's house"),
        joins("esme", "wren").expect("esme's house"),
    ];
    for o in &setup {
        st.perform_outcome(o).expect("feud setup");
    }
    let mut axioms = feud_axioms();
    axioms.extend(kin_axioms());
    st.set_axioms(axioms).expect("feud axioms");
    st
}

/// A scaled feud (for scale demos / benchmarks): `n` grudge-bearers in an
/// alliance chain, all turned against Alice by the one original wrong — so the
/// closure derives `O(n)` enmities and the planner has `n+1` movers per node.
/// UNCHANGED by the faction refactor: the pairwise `allied` chain is the
/// benchmark's own design, not the demo world's — base `allied.*` facts remain
/// legal vocabulary (the spec's ontology note: not every alliance is a
/// membership).
///
/// # Panics
/// As [`feud_world`].
pub fn big_feud(n: usize) -> State {
    let names: Vec<String> = (1..=n).map(|i| format!("a{i}")).collect();
    let mut st = State::new();
    st.define_practices([society_practice()])
        .expect("bigFeud practices");
    let mut cast = vec![alice()];
    cast.extend(names.iter().map(|n| grudge_bearer(n)));
    st.set_characters(cast).expect("bigFeud cast");
    let mut setup = vec![insert("practice.society.here"), insert("wronged.alice.a1")];
    setup.extend((1..n).map(|i| insert(format!("allied.a{i}.a{}", i + 1))));
    for o in &setup {
        st.perform_outcome(o).expect("bigFeud setup");
    }
    st.set_axioms(feud_axioms()).expect("bigFeud axioms");
    st
}

#[cfg(test)]
mod tests {
    // H: FeudSpec.hs "Prax.Worlds.Feud (emergent sandbox)"
    //
    // The frozen `Prax.FeudSpec`, re-expressed against the Rust engine: derived
    // enmity that no one authored, a derived fact gating an affordance, and
    // defeasibility — all through the engine's normal read path.
    use super::*;
    use prax_core::engine::{GroundedAction, State};
    use prax_core::turn::{advance, npc_act};
    use prax_vocab::kin::wed;

    /// Run with `idle` (the player, Alice) never acting and everyone else
    /// planner-driven.
    fn run_with_passive(idle: &str, k: i32, st: &mut State) {
        for _ in 0..k {
            let actor = advance(st);
            if actor.name != idle {
                npc_act(st, 2, &actor);
            }
        }
    }

    /// The targets `who` can shun right now.
    fn can_shun(st: &mut State, who: &str) -> Vec<String> {
        let prefix = format!("{who}: shun ");
        st.possible_actions(who)
            .into_iter()
            .filter_map(|ga| ga.label.strip_prefix(&prefix).map(str::to_owned))
            .collect()
    }

    fn find(st: &mut State, who: &str, needle: &str) -> GroundedAction {
        st.possible_actions(who)
            .into_iter()
            .find(|ga| ga.label.contains(needle))
            .unwrap_or_else(|| panic!("no action {needle:?} for {who}"))
    }

    /// Esme, wed into the feud's kestrel house (the bride moves — authored
    /// direction).
    fn wedded_world() -> State {
        let mut st = feud_world();
        for o in wed("esme", "kestrel", "dave").expect("esme's wedding") {
            st.perform_outcome(&o).expect("the wedding");
        }
        st
    }

    // H: FeudSpec.hs "enmity is DERIVED, not authored: Alice's wrong spreads through the network"
    #[test]
    fn enmity_is_derived_not_authored() {
        let st = feud_world();
        // authored: only that Alice wronged Bob
        assert!(
            st.labeled_facts().contains(&"wronged.alice.bob".to_owned()),
            "the one authored grievance"
        );
        // derived: Bob and — though they never met Alice — Carol and Dave resent her
        let view = st.labeled_view();
        for (s, why) in [
            ("resents.bob.alice", "bob resents alice (wronged)"),
            ("resents.carol.alice", "carol resents alice (ally)"),
            ("resents.dave.alice", "dave resents alice (ally's ally)"),
        ] {
            assert!(view.contains(&s.to_owned()), "{why}");
        }
        // none of these are in the base — they exist only in the closure
        assert!(
            !st.labeled_facts().iter().any(|s| s.contains("resents.")),
            "not authored in the base"
        );
    }

    // H: FeudSpec.hs "a derived fact GATES an affordance: Carol may shun Alice, Alice may shun no one"
    #[test]
    fn a_derived_fact_gates_an_affordance() {
        let mut st = feud_world();
        assert!(
            can_shun(&mut st, "carol").contains(&"alice".to_owned()),
            "carol can shun alice (via derived enmity)"
        );
        assert!(
            can_shun(&mut st, "alice").is_empty(),
            "alice resents no one, so shuns no one"
        );
    }

    // H: FeudSpec.hs "with Alice passive, the network shuns her on its own (emergent behaviour)"
    #[test]
    fn a_passive_alice_is_shunned_by_the_network() {
        // (an *active* Alice would rationally make amends first — see the last case)
        let mut st = feud_world();
        run_with_passive("alice", 12, &mut st);
        let facts = st.labeled_facts();
        for who in ["bob", "carol", "dave"] {
            assert!(
                facts.contains(&format!("shunned.{who}.alice")),
                "{who} shunned alice"
            );
        }
    }

    // H: FeudSpec.hs "the feud scales: bigFeud turns every ally in the chain against Alice"
    #[test]
    fn the_feud_scales() {
        // guards semi-naive closure correctness at scale (no derivation dropped)
        let n = 20;
        let view = big_feud(n).labeled_view();
        for i in 1..=n {
            assert!(
                view.contains(&format!("resents.a{i}.alice")),
                "every one of the chain's members (transitively) resents alice: a{i}"
            );
        }
    }

    // H: FeudSpec.hs "DEFEASIBLE: making amends retracts the wrong and dissolves the whole feud"
    #[test]
    fn making_amends_dissolves_the_feud() {
        let mut st = feud_world();
        let ga = find(&mut st, "alice", "make amends with bob");
        st.perform_action(&ga);
        assert!(
            !st.labeled_facts().contains(&"wronged.alice.bob".to_owned()),
            "the wrong is gone"
        );
        let view = st.labeled_view();
        assert!(
            !view.contains(&"resents.carol.alice".to_owned()),
            "carol no longer resents"
        );
        assert!(
            !view.contains(&"resents.dave.alice".to_owned()),
            "dave no longer resents"
        );
        assert!(
            can_shun(&mut st, "carol").is_empty(),
            "and can no longer shun her"
        );
    }

    // H: FeudSpec.hs "pre-wedding: esme is inert to the feud — her own house, no resentment, no kestrel ties"
    #[test]
    fn esme_starts_inert_to_the_feud() {
        // wren has no other member, so comrades derives nothing for esme —
        // inertness is structural, not accidental
        let mut st = feud_world();
        assert!(
            st.db_has("member.esme!wren"),
            "esme starts in her own house (wren)"
        );
        let view = st.labeled_view();
        assert!(
            !view.iter().any(|s| s.starts_with("resents.esme.")),
            "esme resents no one yet"
        );
        for s in ["allied.esme.bob", "allied.esme.carol", "allied.esme.dave"] {
            assert!(
                !view.contains(&s.to_owned()),
                "esme is not yet allied with the kestrel house: {s}"
            );
        }
    }

    // H: FeudSpec.hs "the wedding: wed moves esme's membership; the derived world flips"
    #[test]
    fn the_wedding_flips_the_derived_world() {
        let mut st = wedded_world();
        assert!(
            st.db_has("member.esme!kestrel"),
            "esme's membership moved to kestrel"
        );
        assert!(
            !st.db_has("member.esme!wren"),
            "esme's old wren membership is gone"
        );
        let view = st.labeled_view();
        for s in ["allied.esme.bob", "allied.esme.carol", "allied.esme.dave"] {
            assert!(
                view.contains(&s.to_owned()),
                "esme is now allied with the whole kestrel house (comrades): {s}"
            );
        }
        assert!(
            view.contains(&"resents.esme.alice".to_owned()),
            "esme inherits her in-laws' grudge: resents.esme.alice is derived"
        );
        assert!(
            view.contains(&"married.dave.esme".to_owned()),
            "married.dave.esme is derived (marriage symmetry, kinAxioms)"
        );
    }

    // H: FeudSpec.hs "the driven beat: after the wedding, esme (a grudgeBearer) shuns alice unprompted"
    #[test]
    fn the_wedded_esme_shuns_alice_unprompted() {
        let mut st = wedded_world();
        run_with_passive("alice", 12, &mut st);
        assert!(
            st.labeled_facts()
                .contains(&"shunned.esme.alice".to_owned()),
            "esme shunned alice"
        );
    }
}
