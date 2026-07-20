//! The S3 derivation property suite — the flagship `naive == production` law and
//! its supporting corners (`docs/rewrite/stages/S03-design.md` §8, the panel's
//! DIV-1/C1/C2 charges).
//!
//! The generator MUST cover the shapes the delta-join's correctness turns on
//! (`S03-design.md` §8, soundness C1, design C2): multi-`Match` bodies, the
//! shipped 4-condition aggregate (`Match`+`Subquery`+`Count`+`Cmp`), `Exists`/
//! `Not` bodies reading DERIVED predicates (the frozen bug's fragment — where the
//! Rust must beat the frozen engine), no-`Match` bodies, multi-head and recursive
//! (Kin-shaped) rules. A `Match`-only generator would pass while blind to exactly
//! the class of bug the net exists to catch, so the shapes below wire reads to
//! other rules' heads on purpose.
//!
//! `naive == production` is a true theorem across ALL these shapes (not only the
//! monotone ones): within a single closure the model only accumulates, and
//! semi-naive derives each fact in the same round the naive full-query first
//! could, so the per-round models — hence the fixpoint, and any ⊥ witness —
//! coincide. The continuation law (`close_from`) is the one that needs the
//! monotone gate (design C1); it has its own restricted generator here.

#[cfg(test)]
mod props {
    use prax_core::db::Db;
    use prax_core::derive::{CompiledRule, Contradiction, close, close_from, naive_closure};
    use prax_core::interner::Interner;
    use prax_core::path::{CompiledPath, tokenize};
    use prax_core::query::{CmpOp, Condition};
    use proptest::prelude::*;

    const UNARY: &[&str] = &["ua", "ub", "uc", "ud"];
    const BIN: &[&str] = &["ba", "bb", "bc"];
    const CONST: &[&str] = &["c0", "c1", "c2"];

    fn u(i: u8) -> &'static str {
        UNARY[i as usize % UNARY.len()]
    }
    fn b(i: u8) -> &'static str {
        BIN[i as usize % BIN.len()]
    }
    fn k(i: u8) -> &'static str {
        CONST[i as usize % CONST.len()]
    }

    fn mtch(s: String) -> Condition {
        Condition::Match(s)
    }

    /// One authored rule: its body conditions and its head sentence templates.
    type RuleSpec = (Vec<Condition>, Vec<String>);

    /// A rule shape drawn to cover every constructor the delta-join split turns
    /// on. Predicate indices are independent draws, so an `Exists`/`Not`/
    /// `Subquery` frequently reads a predicate some other rule's head writes and
    /// disjoint from this rule's `Match` — the DIV-1 fragment.
    fn rule_spec() -> impl Strategy<Value = RuleSpec> {
        let idx = || any::<u8>();
        prop_oneof![
            // 1. single-Match unary → unary (optionally two heads: multi-head).
            (idx(), idx(), idx(), any::<bool>()).prop_map(|(p, q, r, two)| {
                let mut heads = vec![format!("{}.X", u(q))];
                if two {
                    heads.push(format!("{}.X", u(r)));
                }
                (vec![mtch(format!("{}.X", u(p)))], heads)
            }),
            // 2. two-Match binary join — recursive / grandparent / Kin transitive.
            (idx(), idx(), idx()).prop_map(|(p, q, r)| {
                (
                    vec![
                        mtch(format!("{}.X.Y", b(p))),
                        mtch(format!("{}.Y.Z", b(q))),
                    ],
                    vec![format!("{}.X.Z", b(r))],
                )
            }),
            // 3. Match + Not (defeasible; Not reads a possibly-derived predicate).
            (idx(), idx(), idx()).prop_map(|(p, q, r)| {
                (
                    vec![
                        mtch(format!("{}.X", u(p))),
                        Condition::Not(format!("{}.X", u(q))),
                    ],
                    vec![format!("{}.X", u(r))],
                )
            }),
            // 4. THE BUG FRAGMENT: Match + Exists over a disjoint, possibly-derived
            //    predicate. Frozen drops the head once the Match leaves the delta;
            //    the correct closure (full-eval) keeps it.
            (idx(), idx(), idx()).prop_map(|(p, q, r)| {
                (
                    vec![
                        mtch(format!("{}.X", u(p))),
                        Condition::Exists(vec![mtch(format!("{}.Y", u(q)))]),
                    ],
                    vec![format!("{}.X", u(r))],
                )
            }),
            // 5. Match + Neq guard on a binary join.
            (idx(), idx()).prop_map(|(p, q)| {
                (
                    vec![
                        mtch(format!("{}.X.Y", b(p))),
                        Condition::Neq("X".into(), "Y".into()),
                    ],
                    vec![format!("{}.X", u(q))],
                )
            }),
            // 6. the shipped 4-condition aggregate: Match + Subquery + Count + Cmp.
            //    The subquery reads a (possibly other) binary predicate; when it
            //    differs from the Match predicate this is the dangerous aggregate.
            (idx(), idx(), idx(), 1u8..=3).prop_map(|(p, q, r, thr)| {
                (
                    vec![
                        mtch(format!("{}.W0.T", b(p))),
                        Condition::Subquery {
                            set: "S".into(),
                            find: vec!["W".into()],
                            where_: vec![mtch(format!("{}.W.T", b(q)))],
                        },
                        Condition::Count("N".into(), "S".into()),
                        Condition::Cmp(CmpOp::Gte, "N".into(), thr.to_string()),
                    ],
                    vec![format!("{}.T", u(r))],
                )
            }),
            // 7. no-Match body (Absent / Exists at top level → full-eval, fired on
            //    the model every round).
            (idx(), idx(), idx()).prop_map(|(p, q, c)| {
                (
                    vec![Condition::Absent(vec![mtch(format!("{}.{}", u(p), k(c)))])],
                    vec![format!("{}.{}", u(q), k(c))],
                )
            }),
            // 8. Or body (disjunction reading two possibly-derived predicates).
            (idx(), idx(), idx()).prop_map(|(p, q, r)| {
                (
                    vec![Condition::Or(vec![
                        vec![mtch(format!("{}.X", u(p)))],
                        vec![mtch(format!("{}.X", u(q)))],
                    ])],
                    vec![format!("{}.X", u(r))],
                )
            }),
        ]
    }

    /// A ground base fact — unary or binary over the constant pool.
    fn base_fact() -> impl Strategy<Value = String> {
        prop_oneof![
            (any::<u8>(), any::<u8>()).prop_map(|(p, c)| format!("{}.{}", u(p), k(c))),
            (any::<u8>(), any::<u8>(), any::<u8>())
                .prop_map(|(p, c, d)| format!("{}.{}.{}", b(p), k(c), k(d))),
        ]
    }

    fn build_db(interner: &mut Interner, facts: &[String]) -> Db {
        let mut db = Db::empty();
        for f in facts {
            db = db.insert_str(interner, f).unwrap();
        }
        db
    }

    fn compile_rules(interner: &mut Interner, specs: &[RuleSpec]) -> Vec<CompiledRule> {
        specs
            .iter()
            .map(|(body, heads)| {
                let head_refs: Vec<&str> = heads.iter().map(String::as_str).collect();
                CompiledRule::compile(interner, body, &head_refs).unwrap()
            })
            .collect()
    }

    /// Compare two closure results up to the ⊥ witness (design I4): both `Ok`
    /// with identical labeled sentences, or both `Err` (the witness itself is
    /// pinned by the deterministic-name-order proptest, not compared here — an
    /// `Err`/`Ok` split is the real divergence).
    fn agree(interner: &Interner, a: &Result<Db, Contradiction>, b: &Result<Db, Contradiction>) -> bool {
        match (a, b) {
            (Ok(x), Ok(y)) => x.to_labeled_sentences(interner) == y.to_labeled_sentences(interner),
            (Err(_), Err(_)) => true,
            _ => false,
        }
    }

    proptest! {
        // THE FLAGSHIP LAW: the semi-naive production closure equals the naive
        // full-query oracle on every generated rule/base set — including the
        // frozen bug's fragment (shape 4), where a faithful port would diverge.
        #[test]
        fn naive_equals_production(
            specs in prop::collection::vec(rule_spec(), 0..4),
            facts in prop::collection::vec(base_fact(), 0..6),
        ) {
            let mut i = Interner::new();
            let rules = compile_rules(&mut i, &specs);
            let base = build_db(&mut i, &facts);
            let prod = close(&mut i, &rules, &base);
            let naive = naive_closure(&mut i, &rules, &base);
            // Compare witnesses too when both err (they share the sort+fold, so
            // must match exactly), else labeled sentences.
            match (&prod, &naive) {
                (Ok(p), Ok(n)) => prop_assert_eq!(
                    p.to_labeled_sentences(&i), n.to_labeled_sentences(&i),
                    "naive != production facts"),
                (Err(p), Err(n)) => prop_assert_eq!(p, n, "naive != production ⊥ witness"),
                _ => prop_assert!(false, "naive/production Ok-vs-Err split: prod={:?} naive={:?}", prod.is_ok(), naive.is_ok()),
            }
        }

        // THE DIV-1 BUG FRAGMENT, reliably constructed (the broad generator hits
        // it only rarely): a Match + an INDEPENDENT Exists over a derived, disjoint
        // predicate. The correct closure derives `r.C`; a frozen-style delta-seed
        // (delta-seeding the Match position of a full-eval body) drops it from
        // production only — so this proptest is RED under that mutation while
        // naive stays right. This is where the Rust must beat the frozen engine.
        #[test]
        fn div1_bug_fragment_naive_equals_production(c in any::<u8>()) {
            let mut i = Interner::new();
            let cc = k(c);
            let specs: Vec<RuleSpec> = vec![
                // A: Match p.X + Exists over the DERIVED, disjoint predicate q → r.X
                (vec![mtch("p.X".into()), Condition::Exists(vec![mtch("q.Y".into())])],
                 vec!["r.X".into()]),
                // B: derives q (disjoint from p), fired by a base trigger t
                (vec![mtch("t.Z".into())], vec!["q.thing".into()]),
            ];
            let rules = compile_rules(&mut i, &specs);
            let base = build_db(&mut i, &[format!("p.{cc}"), format!("t.{cc}")]);
            let prod = close(&mut i, &rules, &base);
            let naive = naive_closure(&mut i, &rules, &base);
            prop_assert_eq!(&prod, &naive, "DIV-1 fragment: naive != production");
            prop_assert!(
                prod.unwrap().to_labeled_sentences(&i).contains(&format!("r.{cc}")),
                "DIV-1 fragment: r.{} must be derived (the Rust beats frozen here)", cc);
        }

        // Idempotence: closing an already-closed model is the closed model itself
        // (a fixpoint). Skipped when the base already contradicts.
        #[test]
        fn closure_is_idempotent(
            specs in prop::collection::vec(rule_spec(), 0..4),
            facts in prop::collection::vec(base_fact(), 0..6),
        ) {
            let mut i = Interner::new();
            let rules = compile_rules(&mut i, &specs);
            let base = build_db(&mut i, &facts);
            if let Ok(closed) = close(&mut i, &rules, &base) {
                let twice = close(&mut i, &rules, &closed);
                prop_assert!(twice.is_ok(), "re-closing a closed model contradicted");
                prop_assert_eq!(
                    twice.unwrap().to_labeled_sentences(&i),
                    closed.to_labeled_sentences(&i),
                    "closure is not idempotent");
            }
        }

        // ⊥ is join-order independent (design I4, up-to-Err): reversing the rule
        // order agrees — both err, or both ok with the same facts.
        #[test]
        fn bottom_and_facts_are_rule_order_independent(
            specs in prop::collection::vec(rule_spec(), 0..4),
            facts in prop::collection::vec(base_fact(), 0..6),
        ) {
            let mut i = Interner::new();
            let rules = compile_rules(&mut i, &specs);
            let mut rev = rules.clone();
            rev.reverse();
            let base = build_db(&mut i, &facts);
            let a = close(&mut i, &rules, &base);
            let b = close(&mut i, &rev, &base);
            prop_assert!(agree(&i, &a, &b), "rule order changed the closure result");
        }

        // The ⊥ witness is DETERMINISTIC and name-ordered: production and naive
        // (which share the sort-by-name + fold) report the identical witness, so
        // a broken sort would surface as a mismatch. (Covered inside
        // naive_equals_production's Err arm; restated here as a focused corner
        // over `!` heads that force conflicts.)
        #[test]
        fn conflicting_bang_heads_agree_on_witness(
            p in any::<u8>(), q in any::<u8>(), c in any::<u8>(),
        ) {
            let mut i = Interner::new();
            // Two rules writing the SAME exclusive slot to two different values,
            // both fired by a base trigger — a guaranteed ⊥ when the values differ.
            let specs: Vec<RuleSpec> = vec![
                (vec![mtch("trig".into())], vec![format!("slot!{}", k(p))]),
                (vec![mtch("trig".into())], vec![format!("slot!{}", k(q))]),
            ];
            let rules = compile_rules(&mut i, &specs);
            let base = build_db(&mut i, &[format!("anchor.{}", k(c)), "trig".into()]);
            let prod = close(&mut i, &rules, &base);
            let naive = naive_closure(&mut i, &rules, &base);
            prop_assert_eq!(prod, naive, "production and naive disagree on the ⊥ witness/result");
        }
    }

    // ---- the continuation law (close_from), GATED to monotone shapes (C1) ----

    /// A MONOTONE rule: only `Match`/`Neq`/`Count`/`Cmp`-Gte-literal, `!`-free
    /// heads — adding base facts can only ADD derived facts (`monotoneAxioms`'s
    /// shape). No `Not`/`Absent`/`Exists`/`Or`, no exclusive heads.
    fn monotone_rule_spec() -> impl Strategy<Value = RuleSpec> {
        let idx = || any::<u8>();
        prop_oneof![
            (idx(), idx()).prop_map(|(p, q)| (
                vec![mtch(format!("{}.X", u(p)))],
                vec![format!("{}.X", u(q))]
            )),
            (idx(), idx(), idx()).prop_map(|(p, q, r)| (
                vec![
                    mtch(format!("{}.X.Y", b(p))),
                    mtch(format!("{}.Y.Z", b(q))),
                ],
                vec![format!("{}.X.Z", b(r))]
            )),
            (idx(), idx()).prop_map(|(p, q)| (
                vec![
                    mtch(format!("{}.X.Y", b(p))),
                    Condition::Neq("X".into(), "Y".into()),
                ],
                vec![format!("{}.X", u(q))]
            )),
        ]
    }

    /// A monotone (`!`-free) base fact.
    fn monotone_fact() -> impl Strategy<Value = String> {
        base_fact() // base_fact never emits `!`
    }

    proptest! {
        // close_from(close(base), monotone-delta) == close(base ∪ delta), on
        // monotone axioms + monotone delta only (design C1). Outside this gate
        // close_from legitimately over-derives (it never retracts), so the law is
        // stated exactly where it holds — the router's precondition.
        #[test]
        fn close_from_continues_monotone_deltas(
            specs in prop::collection::vec(monotone_rule_spec(), 0..4),
            facts in prop::collection::vec(monotone_fact(), 0..5),
            delta in prop::collection::vec(monotone_fact(), 0..3),
        ) {
            let mut i = Interner::new();
            let rules = compile_rules(&mut i, &specs);
            let base = build_db(&mut i, &facts);

            // The from-scratch closure of base ∪ delta.
            let mut both = facts.clone();
            both.extend(delta.iter().cloned());
            let base_plus = build_db(&mut i, &both);
            let scratch = close(&mut i, &rules, &base_plus);

            // The continuation from the closed base.
            let closed = close(&mut i, &rules, &base);
            let delta_paths: Vec<CompiledPath> =
                delta.iter().map(|f| tokenize(&mut i, f).unwrap()).collect();

            // If either side contradicts (an exclusive base fact can still clash
            // even with monotone `!`-free-head rules), the law is vacuous for this
            // input.
            if let (Ok(c), Ok(s)) = (closed, scratch) {
                let cont = close_from(&mut i, &rules, &c, &delta_paths).unwrap();
                prop_assert_eq!(
                    cont.to_labeled_sentences(&i),
                    s.to_labeled_sentences(&i),
                    "close_from diverged from a from-scratch monotone closure");
            }
        }
    }
}
