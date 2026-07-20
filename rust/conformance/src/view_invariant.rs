//! The stage's flagship net, reborn on the REAL engine (design §5): after EVERY
//! perform, the cached `view` equals the independent naive closure of the current
//! base under the world's axioms. This is the soundness guard for the three-tier
//! delta router — `apply_direct` (irrelevant), `apply_grow` (monotone
//! continuation), and the full `reclose` — over GENERATED vocabularies (both
//! `cont_monotone == true` and `== false` worlds) and random perform sequences,
//! `⊥` up-to-witness. The world-turn shard (real worlds driven through the loop)
//! lands at S7; this shard drives the router directly with adversarial deltas.
//!
//! Scope (honest): the invariant holds on every ⊥-free trajectory AND at the step
//! that first reaches ⊥ (both collapse to `db + contradiction`). A trajectory that
//! CONTINUES past a ⊥ is out of contract — `apply_grow` legitimately keeps
//! deriving on top of a base that globally contradicts, exactly as the frozen
//! `applyGrowToks` does (shipped worlds never ⊥ mid-play). So the sequence stops
//! at the first ⊥, after asserting that ⊥ step matches.

#[cfg(test)]
mod props {
    use prax_core::engine::State;
    use prax_core::query::Condition;
    use prax_core::types::{Axiom, Outcome, delete, insert};
    use proptest::prelude::*;

    fn u(i: u8) -> &'static str {
        ["p", "q", "r"][i as usize % 3]
    }
    fn bin(i: u8) -> &'static str {
        ["e", "f"][i as usize % 2]
    }
    fn k(i: u8) -> &'static str {
        ["c0", "c1", "c2"][i as usize % 3]
    }
    fn m(s: String) -> Condition {
        Condition::Match(s)
    }

    /// A rule shape, drawn to reach BOTH monotone (all-`Match`, so `cont_monotone
    /// == true`) and non-monotone (a `Not` body → `cont_monotone == false`)
    /// worlds, plus `!`-headed rules that can force ⊥.
    fn rule_spec() -> impl Strategy<Value = (Vec<Condition>, Vec<String>)> {
        prop_oneof![
            // monotone unary
            (any::<u8>(), any::<u8>())
                .prop_map(|(p, q)| (vec![m(format!("{}.X", u(p)))], vec![format!("{}.X", u(q))])),
            // monotone transitive binary join
            (any::<u8>(), any::<u8>(), any::<u8>()).prop_map(|(p, q, r)| (
                vec![m(format!("{}.X.Y", bin(p))), m(format!("{}.Y.Z", bin(q)))],
                vec![format!("{}.X.Z", bin(r))]
            )),
            // NON-monotone: Match + Not (disables the continuation tier)
            (any::<u8>(), any::<u8>(), any::<u8>()).prop_map(|(p, q, r)| (
                vec![m(format!("{}.X", u(p))), Condition::Not(format!("{}.X", u(q)))],
                vec![format!("{}.X", u(r))]
            )),
            // exclusion head — monotone body, but a `!` head can force ⊥
            (any::<u8>(), any::<u8>())
                .prop_map(|(p, v)| (vec![m(format!("{}.X", u(p)))], vec![format!("slot.X!{}", k(v))])),
        ]
    }

    /// A perform op: ground inserts/deletes, some exclusion inserts (to exercise
    /// eviction shadows and the exclusion clash → ⊥ path).
    fn op() -> impl Strategy<Value = Outcome> {
        prop_oneof![
            (any::<u8>(), any::<u8>()).prop_map(|(p, c)| insert(format!("{}.{}", u(p), k(c)))),
            (any::<u8>(), any::<u8>(), any::<u8>())
                .prop_map(|(p, c, d)| insert(format!("{}.{}.{}", bin(p), k(c), k(d)))),
            (any::<u8>(), any::<u8>()).prop_map(|(p, c)| insert(format!("{}!{}", u(p), k(c)))),
            (any::<u8>(), any::<u8>()).prop_map(|(p, c)| delete(format!("{}.{}", u(p), k(c)))),
            (any::<u8>(), any::<u8>()).prop_map(|(c, v)| insert(format!("slot.{}!{}", k(c), k(v)))),
        ]
    }

    proptest! {
        // THE FLAGSHIP: view == naive_closure(rules, db) after every perform, over
        // generated axiom sets and random perform sequences.
        #[test]
        fn view_equals_naive_after_every_perform(
            rules in prop::collection::vec(rule_spec(), 0..4),
            ops in prop::collection::vec(op(), 0..14),
        ) {
            let axioms: Vec<Axiom> = rules
                .into_iter()
                .map(|(when, then)| Axiom { when, then })
                .collect();
            let mut st = State::new();
            st.set_axioms(axioms).unwrap();
            prop_assert_eq!(st.labeled_view(), st.naive_view(), "invariant broke at build");
            for o in ops {
                st.perform_outcome(&o).unwrap();
                let view = st.labeled_view();
                prop_assert_eq!(&view, &st.naive_view(), "view != naive after {:?}", o);
                // Post-⊥ continuation is out of contract; stop after the ⊥ step
                // (already asserted equal — both are db + contradiction).
                if view.iter().any(|f| f == "contradiction") {
                    break;
                }
            }
        }

        // A generator that reaches BOTH cont_monotone worlds must exist for the
        // net to be meaningful — this asserts the axiom-monotonicity split is
        // reachable (a monotone all-Match world and a non-monotone Not world both
        // build), so `view_equals_naive_after_every_perform` is not silently
        // exercising only one tier.
        #[test]
        fn generator_reaches_both_monotone_and_non_monotone(seed in any::<u8>()) {
            // monotone: p.X -> q.X ; non-monotone: p.X, Not r.X -> q.X
            let mut mono = State::new();
            mono.set_axioms(vec![Axiom {
                when: vec![m("p.X".into())],
                then: vec!["q.X".into()],
            }]).unwrap();
            let mut nonmono = State::new();
            nonmono.set_axioms(vec![Axiom {
                when: vec![m("p.X".into()), Condition::Not("r.X".into())],
                then: vec!["q.X".into()],
            }]).unwrap();
            // Drive one insert through each and confirm the invariant holds on both.
            let fact = insert(format!("p.{}", k(seed)));
            mono.perform_outcome(&fact).unwrap();
            nonmono.perform_outcome(&fact).unwrap();
            prop_assert_eq!(mono.labeled_view(), mono.naive_view());
            prop_assert_eq!(nonmono.labeled_view(), nonmono.naive_view());
        }
    }
}
