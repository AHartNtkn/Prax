//! Exclusion Logic's lattice (Evans, DEON 2010): `meet` (the greatest lower
//! bound `⊓`; `None` is the paper's `⊥` incompatibility) and `leq` (the
//! information order `≤`, `Excl ≤ Multi`, assertedness pointwise). The formal
//! core the derivation layer's canonical-model construction builds on.
//!
//! Frozen reference: `src/Prax/EL.hs`. `Db` *is* the paper's labeled rooted
//! tree (each node carries an exclusion flag), so the lattice operates on it
//! directly. Both walks are sorted-merge over the id-sorted children.

use crate::db::Db;
use crate::interner::Sym;

/// Greatest lower bound `⊓` (Def 8) — the conjunction of two models. `None` is
/// the paper's `⊥`: at a node exclusive in *either* operand, the two disagree on
/// the child (Def 7 incompatibility). Otherwise children are merged (recursively
/// meeting shared subtrees), a node is exclusive if either operand marks it, and
/// assertedness extends as **disjunction** — asserted in the meet iff asserted
/// in either operand, the choice that keeps the meet a lower bound of an
/// asserted operand.
pub fn meet(a: &Db, b: &Db) -> Option<Db> {
    // Merge b's children into a's (a's are already id-sorted).
    let mut merged: Vec<(Sym, Db)> = a.kids().to_vec();
    for (k, v2) in b.kids() {
        match merged.binary_search_by(|(s, _)| s.id().cmp(&k.id())) {
            Ok(idx) => {
                let met = meet(&merged[idx].1, v2)?;
                merged[idx].1 = met;
            }
            Err(idx) => merged.insert(idx, (*k, v2.clone())),
        }
    }
    let excl = a.is_excl() || b.is_excl();
    let asserted = a.is_asserted() || b.is_asserted();
    if excl && merged.len() > 1 {
        None // an exclusive node forced to two children ⇒ ⊥
    } else {
        Some(Db::from_parts(excl, asserted, merged))
    }
}

/// The information order `≤` (Def 6): `a ≤ b` iff `a` has every edge of `b`,
/// with labels at least as specific (`Excl ≤ Multi`) and the asserted mark at
/// least as strong (an asserted fact entails its unasserted scaffold, not
/// conversely), recursively — i.e. `a` entails `b`.
pub fn leq(a: &Db, b: &Db) -> bool {
    // Mirrors `Prax.EL.leq`: (ea || not eb) && (aa || not ab) && every edge of b
    // present in a with a `leq` child.
    (a.is_excl() || !b.is_excl())
        && (a.is_asserted() || !b.is_asserted())
        && b.kids().iter().all(|(k, b_child)| {
            match a.kids().binary_search_by(|(s, _)| s.id().cmp(&k.id())) {
                Ok(idx) => leq(&a.kids()[idx].1, b_child),
                Err(_) => false,
            }
        })
}

#[cfg(test)]
mod tests {
    // H: ELSpec.hs "Prax.EL (exclusion-logic lattice)"
    //
    // The frozen `Prax.ELSpec`, re-expressed against the Rust lattice.
    use super::*;
    use crate::db::Db;
    use crate::interner::Interner;

    /// A model from several facts (`ELSpec.mk`).
    fn mk(interner: &mut Interner, facts: &[&str]) -> Db {
        let mut db = Db::empty();
        for f in facts {
            db = db.insert_str(interner, f).unwrap();
        }
        db
    }

    /// `dbToSentences <$> meet a b` (`ELSpec.meetS`).
    fn meet_s(interner: &mut Interner, a: &[&str], b: &[&str]) -> Option<Vec<String>> {
        let da = mk(interner, a);
        let db = mk(interner, b);
        meet(&da, &db).map(|m| m.to_sentences(interner))
    }

    // ===== meet ⊓ (Def 8) and incompatibility ⊥ (Def 7) =====
    // H: ELSpec.hs "meet ⊓ (Def 8) and incompatibility ⊥ (Def 7)"

    // H: ELSpec.hs "compatible multi facts conjoin"
    #[test]
    fn compatible_multi_facts_conjoin() {
        let mut i = Interner::new();
        assert_eq!(
            meet_s(&mut i, &["a.b"], &["a.c"]),
            Some(vec!["a.b".to_owned(), "a.c".to_owned()])
        );
    }

    // H: ELSpec.hs "the same exclusive fact is idempotent"
    #[test]
    fn same_exclusive_fact_is_idempotent() {
        let mut i = Interner::new();
        assert_eq!(
            meet_s(&mut i, &["x!a"], &["x!a"]),
            Some(vec!["x.a".to_owned()])
        );
    }

    // H: ELSpec.hs "exclusive slot forced to two values is ⊥"
    #[test]
    fn exclusive_slot_forced_to_two_values_is_bottom() {
        let mut i = Interner::new();
        assert!(meet(&mk(&mut i, &["x!a"]), &mk(&mut i, &["x!b"])).is_none());
    }

    // H: ELSpec.hs "an exclusive claim vs a different multi child is still ⊥ (either side)"
    #[test]
    fn exclusive_vs_different_multi_is_bottom_either_side() {
        let mut i = Interner::new();
        assert!(meet(&mk(&mut i, &["x!a"]), &mk(&mut i, &["x.b"])).is_none());
        assert!(meet(&mk(&mut i, &["x.b"]), &mk(&mut i, &["x!a"])).is_none());
    }

    // H: ELSpec.hs "two multi children never conflict"
    #[test]
    fn two_multi_children_never_conflict() {
        let mut i = Interner::new();
        assert!(meet(&mk(&mut i, &["x.a"]), &mk(&mut i, &["x.b"])).is_some());
    }

    // H: ELSpec.hs "a conflict deep in the tree propagates to ⊥"
    #[test]
    fn deep_conflict_propagates_to_bottom() {
        let mut i = Interner::new();
        assert!(meet(&mk(&mut i, &["p.q.r!a"]), &mk(&mut i, &["p.q.r!b"])).is_none());
    }

    // H: ELSpec.hs "disjoint slots conjoin freely"
    #[test]
    fn disjoint_slots_conjoin_freely() {
        let mut i = Interner::new();
        assert_eq!(
            meet_s(&mut i, &["at!bar"], &["mood!happy"]),
            Some(vec!["at.bar".to_owned(), "mood.happy".to_owned()])
        );
    }

    // ===== information order ≤ (Def 6) =====
    // H: ELSpec.hs "information order ≤ (Def 6): a ≤ b means a entails b"

    // H: ELSpec.hs "more facts entail fewer"
    #[test]
    fn more_facts_entail_fewer() {
        let mut i = Interner::new();
        assert!(leq(&mk(&mut i, &["a.b", "a.c"]), &mk(&mut i, &["a.b"])));
    }

    // H: ELSpec.hs "fewer facts do NOT entail more"
    #[test]
    fn fewer_facts_do_not_entail_more() {
        let mut i = Interner::new();
        assert!(!leq(&mk(&mut i, &["a.b"]), &mk(&mut i, &["a.b", "a.c"])));
    }

    // H: ELSpec.hs "a specific label entails the general (Excl ≤ Multi)"
    #[test]
    fn specific_label_entails_the_general() {
        let mut i = Interner::new();
        assert!(leq(&mk(&mut i, &["x!a"]), &mk(&mut i, &["x.a"])));
    }

    // H: ELSpec.hs "the general does NOT entail the specific (Multi ⋠ Excl)"
    #[test]
    fn general_does_not_entail_the_specific() {
        let mut i = Interner::new();
        assert!(!leq(&mk(&mut i, &["x.a"]), &mk(&mut i, &["x!a"])));
    }

    // H: ELSpec.hs "everything entails the empty model"
    #[test]
    fn everything_entails_the_empty_model() {
        let mut i = Interner::new();
        assert!(leq(&mk(&mut i, &["a.b"]), &Db::empty()));
    }

    // ===== assertedness in the lattice (v39) =====
    // H: ELSpec.hs "assertedness in the lattice (v39): the mark extends pointwise"

    // H: ELSpec.hs "meet preserves an assertion (a ⊓ scaffold ≤ a): forces OR, not AND"
    #[test]
    fn meet_preserves_an_assertion_via_disjunction() {
        let mut i = Interner::new();
        let asserted = mk(&mut i, &["a", "a.b"]);
        let scaffold = mk(&mut i, &["a.b"]);
        let m = meet(&asserted, &scaffold).expect("meet must exist");
        assert!(leq(&m, &asserted), "meet ≤ asserted operand");
        assert!(leq(&m, &scaffold), "meet ≤ scaffold operand");
        assert_eq!(m.to_sentences(&i), ["a", "a.b"]);
    }

    // H: ELSpec.hs "≤ consults the mark: an asserted fact entails its scaffold, not conversely"
    #[test]
    fn leq_consults_the_mark() {
        let mut i = Interner::new();
        assert!(leq(&mk(&mut i, &["a", "a.b"]), &mk(&mut i, &["a.b"])));
        assert!(!leq(&mk(&mut i, &["a.b"]), &mk(&mut i, &["a", "a.b"])));
    }
}

#[cfg(test)]
mod proptest_laws {
    //! EL lattice laws (ARCHITECTURE.md's list): meet commutative / associative
    //! / idempotent, `meet(a,b) ≤ a`, `leq` reflexive / transitive, `⊥`
    //! symmetric.
    use super::*;
    use crate::db::Db;
    use crate::interner::Interner;
    use proptest::prelude::*;

    fn seg() -> impl Strategy<Value = String> {
        prop::sample::select(vec!["a", "b", "c"]).prop_map(String::from)
    }

    /// A random ground path of 1–3 segments with `.`/`!` separators.
    fn path() -> impl Strategy<Value = String> {
        prop::collection::vec((seg(), prop::bool::ANY), 1..4).prop_map(|parts| {
            let mut s = String::new();
            for (idx, (name, bang)) in parts.iter().enumerate() {
                if idx > 0 {
                    s.push(if *bang { '!' } else { '.' });
                }
                s.push_str(name);
            }
            s
        })
    }

    /// A random single, internally-consistent model (built by inserts, so never
    /// self-contradictory).
    fn model() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(path(), 0..5)
    }

    fn build(interner: &mut Interner, facts: &[String]) -> Db {
        let mut db = Db::empty();
        for f in facts {
            db = db.insert_str(interner, f).unwrap();
        }
        db
    }

    proptest! {
        #[test]
        fn meet_idempotent(m in model()) {
            let mut i = Interner::new();
            let a = build(&mut i, &m);
            prop_assert_eq!(meet(&a, &a), Some(a));
        }

        #[test]
        fn meet_commutative(x in model(), y in model()) {
            let mut i = Interner::new();
            let a = build(&mut i, &x);
            let b = build(&mut i, &y);
            prop_assert_eq!(meet(&a, &b), meet(&b, &a));
        }

        #[test]
        fn meet_associative(x in model(), y in model(), z in model()) {
            let mut i = Interner::new();
            let a = build(&mut i, &x);
            let b = build(&mut i, &y);
            let c = build(&mut i, &z);
            let left = meet(&a, &b).and_then(|ab| meet(&ab, &c));
            let right = meet(&b, &c).and_then(|bc| meet(&a, &bc));
            prop_assert_eq!(left, right);
        }

        #[test]
        fn meet_is_a_lower_bound(x in model(), y in model()) {
            let mut i = Interner::new();
            let a = build(&mut i, &x);
            let b = build(&mut i, &y);
            if let Some(m) = meet(&a, &b) {
                prop_assert!(leq(&m, &a), "meet ≤ a");
                prop_assert!(leq(&m, &b), "meet ≤ b");
            }
        }

        #[test]
        fn leq_reflexive(m in model()) {
            let mut i = Interner::new();
            let a = build(&mut i, &m);
            prop_assert!(leq(&a, &a));
        }

        #[test]
        fn leq_transitive(x in model(), y in model(), z in model()) {
            let mut i = Interner::new();
            let a = build(&mut i, &x);
            let b = build(&mut i, &y);
            let c = build(&mut i, &z);
            if leq(&a, &b) && leq(&b, &c) {
                prop_assert!(leq(&a, &c));
            }
        }

        #[test]
        fn bottom_symmetric(x in model(), y in model()) {
            let mut i = Interner::new();
            let a = build(&mut i, &x);
            let b = build(&mut i, &y);
            prop_assert_eq!(meet(&a, &b).is_none(), meet(&b, &a).is_none());
        }
    }
}
