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

    // FIXTURE REPLAY: query.json — parse each Haskell-`show` condition string,
    // compile it, evaluate over the built db from the initial binding, and match
    // the recorded result rows byte-for-byte (name-order branching and the
    // fold's left-to-right threading are both under test).
    #[test]
    fn query_replay() {
        use prax_core::query::{Cond, compile_condition, query};

        let data = load("query.json");
        for case in data["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let mut i = Interner::new();
            let db = build(&mut i, &strs(&case["facts"]));

            // The seed binding: initial values are symbols (the corpus carries no
            // numeric/set initials).
            let mut seed = Bindings::new();
            for (var, val) in case["initial"].as_object().unwrap() {
                let key = i.intern(var);
                seed.insert(key, Val::Sym(i.intern(val.as_str().unwrap())));
            }

            let conds: Vec<Cond> = strs(&case["conds"])
                .iter()
                .map(|s| {
                    let authored = parse_condition(s)
                        .unwrap_or_else(|| panic!("parsing condition {s:?} in '{name}'"));
                    compile_condition(&mut i, &authored)
                        .unwrap_or_else(|e| panic!("compiling {s:?} in '{name}': {e}"))
                })
                .collect();

            let got: Vec<BTreeMap<String, String>> = query(&mut i, &db, &conds, &seed)
                .iter()
                .map(|b| binding_map(&i, b))
                .collect();
            let want: Vec<BTreeMap<String, String>> = case["results"]
                .as_array()
                .unwrap()
                .iter()
                .map(json_binding)
                .collect();
            assert_eq!(got, want, "query results mismatch in '{name}'");
        }
    }

    // ---- Haskell-`show` condition parser (fixture-consumption only) ----------
    //
    // The query fixtures serialize `Prax.Query.Condition` via Haskell `show`
    // (e.g. `Subquery {subSet = "Dancers", subFind = ["Dancer"], subWhere =
    // [...]}`). A small recursive-descent reader turns each back into the Rust
    // authoring `Condition`. Operand strings never contain embedded quotes, so a
    // literal is `"` … `"` with no escape handling.
    use prax_core::query::{CalcOp, CmpOp, Condition};

    struct Reader<'a> {
        bytes: &'a [u8],
        pos: usize,
    }

    impl<'a> Reader<'a> {
        fn new(s: &'a str) -> Reader<'a> {
            Reader {
                bytes: s.as_bytes(),
                pos: 0,
            }
        }

        fn skip_ws(&mut self) {
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
                self.pos += 1;
            }
        }

        fn peek(&mut self) -> Option<u8> {
            self.skip_ws();
            self.bytes.get(self.pos).copied()
        }

        fn eat(&mut self, tok: &str) -> bool {
            self.skip_ws();
            if self.bytes[self.pos..].starts_with(tok.as_bytes()) {
                self.pos += tok.len();
                true
            } else {
                false
            }
        }

        fn expect(&mut self, tok: &str) -> Option<()> {
            if self.eat(tok) { Some(()) } else { None }
        }

        /// A double-quoted operand string (no escapes in the corpus).
        fn string(&mut self) -> Option<String> {
            self.skip_ws();
            if self.bytes.get(self.pos)? != &b'"' {
                return None;
            }
            self.pos += 1;
            let start = self.pos;
            while self.pos < self.bytes.len() && self.bytes[self.pos] != b'"' {
                self.pos += 1;
            }
            let s = std::str::from_utf8(&self.bytes[start..self.pos]).ok()?.to_owned();
            self.pos += 1; // closing quote
            Some(s)
        }

        /// A bare identifier (constructor / operator name).
        fn ident(&mut self) -> Option<String> {
            self.skip_ws();
            let start = self.pos;
            while self.pos < self.bytes.len()
                && (self.bytes[self.pos].is_ascii_alphanumeric() || self.bytes[self.pos] == b'_')
            {
                self.pos += 1;
            }
            if self.pos == start {
                None
            } else {
                Some(String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned())
            }
        }

        /// A `["a", "b", ...]` list of quoted strings.
        fn string_list(&mut self) -> Option<Vec<String>> {
            self.expect("[")?;
            let mut out = Vec::new();
            if self.peek() == Some(b']') {
                self.expect("]")?;
                return Some(out);
            }
            loop {
                out.push(self.string()?);
                if self.eat(",") {
                    continue;
                }
                break;
            }
            self.expect("]")?;
            Some(out)
        }

        /// A `[cond, cond, ...]` list of conditions.
        fn cond_list(&mut self) -> Option<Vec<Condition>> {
            self.expect("[")?;
            let mut out = Vec::new();
            if self.peek() == Some(b']') {
                self.expect("]")?;
                return Some(out);
            }
            loop {
                out.push(self.condition()?);
                if self.eat(",") {
                    continue;
                }
                break;
            }
            self.expect("]")?;
            Some(out)
        }

        fn cmp_op(&mut self) -> Option<CmpOp> {
            match self.ident()?.as_str() {
                "Lt" => Some(CmpOp::Lt),
                "Lte" => Some(CmpOp::Lte),
                "Gt" => Some(CmpOp::Gt),
                "Gte" => Some(CmpOp::Gte),
                _ => None,
            }
        }

        fn calc_op(&mut self) -> Option<CalcOp> {
            match self.ident()?.as_str() {
                "Add" => Some(CalcOp::Add),
                "Sub" => Some(CalcOp::Sub),
                "Mul" => Some(CalcOp::Mul),
                "Mod" => Some(CalcOp::Mod),
                _ => None,
            }
        }

        fn condition(&mut self) -> Option<Condition> {
            self.skip_ws();
            let ctor = self.ident()?;
            match ctor.as_str() {
                "Match" => Some(Condition::Match(self.string()?)),
                "Not" => Some(Condition::Not(self.string()?)),
                "Eq" => Some(Condition::Eq(self.string()?, self.string()?)),
                "Neq" => Some(Condition::Neq(self.string()?, self.string()?)),
                "Cmp" => {
                    let op = self.cmp_op()?;
                    Some(Condition::Cmp(op, self.string()?, self.string()?))
                }
                "Calc" => {
                    let r = self.string()?;
                    let op = self.calc_op()?;
                    Some(Condition::Calc(r, op, self.string()?, self.string()?))
                }
                "Count" => Some(Condition::Count(self.string()?, self.string()?)),
                "Subquery" => {
                    self.expect("{")?;
                    self.expect("subSet")?;
                    self.expect("=")?;
                    let set = self.string()?;
                    self.expect(",")?;
                    self.expect("subFind")?;
                    self.expect("=")?;
                    let find = self.string_list()?;
                    self.expect(",")?;
                    self.expect("subWhere")?;
                    self.expect("=")?;
                    let where_ = self.cond_list()?;
                    self.expect("}")?;
                    Some(Condition::Subquery { set, find, where_ })
                }
                "Or" => {
                    // Or [[..],[..]] — a bracketed list of condition lists.
                    self.expect("[")?;
                    let mut clauses = Vec::new();
                    if self.peek() == Some(b']') {
                        self.expect("]")?;
                        return Some(Condition::Or(clauses));
                    }
                    loop {
                        clauses.push(self.cond_list()?);
                        if self.eat(",") {
                            continue;
                        }
                        break;
                    }
                    self.expect("]")?;
                    Some(Condition::Or(clauses))
                }
                "Absent" => Some(Condition::Absent(self.cond_list()?)),
                "Exists" => Some(Condition::Exists(self.cond_list()?)),
                _ => None,
            }
        }
    }

    fn parse_condition(s: &str) -> Option<Condition> {
        let mut r = Reader::new(s);
        let c = r.condition()?;
        r.skip_ws();
        // The whole string must be consumed.
        if r.pos == r.bytes.len() { Some(c) } else { None }
    }
}
