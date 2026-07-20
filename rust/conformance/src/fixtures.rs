//! Replay of the committed unit-fixture corpora
//! (`conformance/fixtures/{db,el,query,derive}.json`) against the Rust engine:
//! each recorded input is recomputed and checked byte-for-byte against the
//! frozen Haskell's recorded output. Populated as S1–S3 land the engine paths.
//!
//! S1 replays `db.json` (insert/retract mutations, unify, ground) and `el.json`
//! (meet, leq). The corpora are DIFFERENTIAL LEVERAGE, not spec coverage
//! (conformance/README.md) — the meta-gate enforces pin coverage separately.

#[cfg(test)]
mod replay {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    use prax_core::db::{Bindings, Db, Val, ground, val_to_string};
    use prax_core::el::{leq, meet};
    use prax_core::interner::Interner;
    use prax_core::path::tokenize;
    use serde_json::Value;

    fn fixture_path(name: &str) -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../../conformance/fixtures");
        p.push(name);
        p
    }

    fn load(name: &str) -> Value {
        let text = fs::read_to_string(fixture_path(name))
            .unwrap_or_else(|e| panic!("reading fixture {name}: {e}"));
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("parsing fixture {name}: {e}"))
    }

    fn strs(v: &Value) -> Vec<String> {
        v.as_array()
            .expect("expected a JSON array")
            .iter()
            .map(|s| s.as_str().expect("expected a string").to_owned())
            .collect()
    }

    fn build(interner: &mut Interner, facts: &[String]) -> Db {
        let mut db = Db::empty();
        for f in facts {
            db = db.insert_str(interner, f).unwrap();
        }
        db
    }

    fn binding_map(interner: &Interner, b: &Bindings) -> BTreeMap<String, String> {
        b.iter()
            .map(|(sym, val)| {
                (
                    interner.resolve(sym).to_owned(),
                    val_to_string(interner, val),
                )
            })
            .collect()
    }

    fn json_binding(v: &Value) -> BTreeMap<String, String> {
        v.as_object()
            .expect("binding is an object")
            .iter()
            .map(|(k, val)| {
                (
                    k.clone(),
                    val.as_str().expect("binding value string").to_owned(),
                )
            })
            .collect()
    }

    // FIXTURE REPLAY: db.json mutations — apply the op sequence, then match the
    // rendered sentences, labeled sentences, and existence probes byte-for-byte.
    #[test]
    fn db_mutations_replay() {
        let data = load("db.json");
        for case in data["mutations"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let mut i = Interner::new();
            let mut db = Db::empty();
            for op in case["ops"].as_array().unwrap() {
                let arg = op["arg"].as_str().unwrap();
                match op["op"].as_str().unwrap() {
                    "insert" => db = db.insert_str(&mut i, arg).unwrap(),
                    "retract" => db = db.retract_str(&mut i, arg).unwrap(),
                    other => panic!("unknown op {other:?} in '{name}'"),
                }
            }
            assert_eq!(
                db.to_sentences(&i),
                strs(&case["sentences"]),
                "sentences mismatch in '{name}'"
            );
            assert_eq!(
                db.to_labeled_sentences(&i),
                strs(&case["labeled"]),
                "labeled mismatch in '{name}'"
            );
            for (path, expected) in case["exists"].as_object().unwrap() {
                assert_eq!(
                    db.exists_str(&mut i, path).unwrap(),
                    expected.as_bool().unwrap(),
                    "exists({path}) mismatch in '{name}'"
                );
            }
        }
    }

    // FIXTURE REPLAY: db.json unify — build the model, unify the patterns
    // conjunctively, match the binding rows in order (name-order branching).
    #[test]
    fn db_unify_replay() {
        let data = load("db.json");
        for case in data["unify"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let mut i = Interner::new();
            let db = build(&mut i, &strs(&case["facts"]));
            let patterns = strs(&case["patterns"]);

            let mut bss = vec![Bindings::new()];
            for pat in &patterns {
                let path = tokenize(&mut i, pat).unwrap();
                let mut next = Vec::new();
                for b in bss {
                    next.extend(db.unify(&mut i, &path.segs, b));
                }
                bss = next;
            }

            let got: Vec<BTreeMap<String, String>> =
                bss.iter().map(|b| binding_map(&i, b)).collect();
            let want: Vec<BTreeMap<String, String>> = case["bindings"]
                .as_array()
                .unwrap()
                .iter()
                .map(json_binding)
                .collect();
            assert_eq!(got, want, "unify bindings mismatch in '{name}'");
        }
    }

    // FIXTURE REPLAY: db.json ground — substitute the bindings, match the
    // rendered sentence.
    #[test]
    fn db_ground_replay() {
        let data = load("db.json");
        for case in data["ground"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let mut i = Interner::new();
            let pattern = case["pattern"].as_str().unwrap();
            let path = tokenize(&mut i, pattern).unwrap();
            let mut b = Bindings::new();
            for (var, val) in case["binding"].as_object().unwrap() {
                let vs = i.intern(var);
                let raw = val.as_str().unwrap();
                // Bind by the value's REAL kind, not everything-as-Sym: the
                // set-marker case must exercise Val::Set's rendering (the
                // "<Set(n)>" opaque form), not trivially intern the marker
                // text (S1 review M3).
                let vv = if let Some(n) = raw
                    .strip_prefix("<Set(")
                    .and_then(|r| r.strip_suffix(")>"))
                    .and_then(|n| n.parse::<usize>().ok())
                {
                    Val::Set(vec![Vec::new(); n])
                } else if let Ok(num) = raw.parse::<i64>() {
                    Val::Num(num)
                } else {
                    Val::Sym(i.intern(raw))
                };
                b.insert(vs, vv);
            }
            assert_eq!(
                ground(&i, &path, &b),
                case["result"].as_str().unwrap(),
                "ground mismatch in '{name}'"
            );
        }
    }

    // FIXTURE REPLAY: el.json meet — `null` is ⊥; otherwise match the conjoined
    // model's sentences.
    #[test]
    fn el_meet_replay() {
        let data = load("el.json");
        for case in data["meet"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let mut i = Interner::new();
            let a = build(&mut i, &strs(&case["a"]));
            let b = build(&mut i, &strs(&case["b"]));
            match meet(&a, &b) {
                None => assert!(
                    case["result"].is_null(),
                    "expected a defined meet in '{name}'"
                ),
                Some(m) => {
                    assert!(!case["result"].is_null(), "expected ⊥ in '{name}'");
                    assert_eq!(
                        m.to_sentences(&i),
                        strs(&case["result"]),
                        "meet mismatch in '{name}'"
                    );
                }
            }
        }
    }

    // FIXTURE REPLAY: el.json leq — match the entailment verdict.
    #[test]
    fn el_leq_replay() {
        let data = load("el.json");
        for case in data["leq"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let mut i = Interner::new();
            let a = build(&mut i, &strs(&case["a"]));
            let b = build(&mut i, &strs(&case["b"]));
            assert_eq!(
                leq(&a, &b),
                case["result"].as_bool().unwrap(),
                "leq mismatch in '{name}'"
            );
        }
    }
}
