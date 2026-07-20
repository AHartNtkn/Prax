//! Byte-for-byte replay of `conformance/fixtures/planner.json` against the Rust
//! planner — the S6 summit's differential channel.
//!
//! Each synthetic world is reconstructed with the Rust builder API (world
//! construction is transcribed on both sides; only the OUTPUT comes from the
//! FROZEN engine) and every planner observable is asserted equal to the frozen
//! one:
//!
//! - `scoreActions` tables at depths 0/1/2, in NATIVE result order [D-C1], with
//!   each score compared as `u64 == f64::to_bits` [D-I1] — there is NO decimal
//!   round-trip anywhere in this channel;
//! - `pickAction`, `candidateActions`, `predictMove` per (predictor, mover)
//!   pair, `motiveSignature`;
//! - the relevance tables (`improvables` / `liveness` / `caresAbout` /
//!   `moverReadAnchors`), rendered by name.
//!
//! The corpus carries the FOLD-ORDER CANARY as a world: the depth-2 score of
//! `alice: raise the mark` is `12 + (3.5 + 0.9*0.9)`, whose two associations
//! land exactly one ULP apart — re-associating [`prax_core`]'s fold reddens this
//! replay, not merely the native unit canary.

#[cfg(test)]
mod replay {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    use prax_core::engine::State;
    use prax_core::query::{
        CalcOp, CmpOp, Condition, calc, cmp, count, eq, for_all, implies, matches, neq,
        not_, subquery,
    };
    use prax_core::types::{
        Action, Axiom, Character, Desire, Outcome, Practice, ScheduleRule, Want, call, delete,
        insert,
    };
    use serde_json::Value;

    fn load() -> Value {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../../conformance/fixtures/planner.json");
        let text = fs::read_to_string(&p).unwrap_or_else(|e| panic!("reading planner.json: {e}"));
        serde_json::from_str(&text).expect("parsing planner.json")
    }

    fn strs(v: &Value) -> Vec<String> {
        v.as_array()
            .expect("array")
            .iter()
            .map(|s| s.as_str().expect("string").to_owned())
            .collect()
    }

    fn m(s: &str) -> Condition {
        Condition::Match(s.to_owned())
    }

    // ---- the worlds ---------------------------------------------------------

    /// The tendBar practice — the corpus's multi-instance shape.
    fn tend_bar() -> Practice {
        Practice::new("tendBar")
            .name("[Bartender] is tending bar")
            .roles(["Bartender"])
            .data_facts([
                "beverageType.beer!alcoholic",
                "beverageType.cider!alcoholic",
                "beverageType.soda!nonalcoholic",
            ])
            .action(
                Action::new("[Actor]: Walk up to bar")
                    .when([
                        neq("Actor", "Bartender"),
                        not_("practice.tendBar.Bartender.customer.Actor"),
                    ])
                    .then([insert("practice.tendBar.Bartender.customer.Actor")]),
            )
            .action(
                Action::new("[Actor]: Order [Beverage]")
                    .when([
                        matches("practice.tendBar.Bartender.customer.Actor"),
                        not_("practice.tendBar.Bartender.customer.Actor!beverage"),
                        matches("practiceData.tendBar.beverageType.Beverage"),
                    ])
                    .then([insert(
                        "practice.tendBar.Bartender.customer.Actor!order!Beverage",
                    )]),
            )
    }

    fn heist() -> Practice {
        Practice::new("heist")
            .roles(["R"])
            .action(
                Action::new("[Actor]: grab the relic")
                    .when([
                        m("gate.open"),
                        not_("grabbed.inge"),
                        eq("Actor", "inge"),
                    ])
                    .then([insert("grabbed.inge")]),
            )
            .action(
                Action::new("[Actor]: open the gate")
                    .when([eq("Actor", "olaf"), not_("gate.open")])
                    .then([insert("gate.open")]),
            )
            .action(Action::new("[Actor]: Wait about"))
    }

    fn heist_base() -> State {
        let mut st = State::new();
        st.define_practices([heist()]).unwrap();
        st.set_characters(vec![
            Character::new("olaf").want(Want::new(vec![m("grabbed.inge")], 6)),
            Character::new("inge").holds("covet-relic"),
        ])
        .unwrap();
        st.set_desires(vec![Desire::new(
            "covet-relic",
            Want::new(vec![m("grabbed.Owner")], 10),
        )])
        .unwrap();
        st.set_prediction_scope(vec![m("at.Actor!Room"), m("at.Witness!Room")])
            .unwrap();
        st
    }

    fn dead_now_base() -> State {
        let town = Practice::new("town")
            .roles(["R"])
            .action(
                Action::new("[Actor]: confess")
                    .when([m("lied.Actor")])
                    .then([delete("lied.Actor")]),
            )
            .action(
                Action::new("[Actor]: sell at the market")
                    .when([m("marketDay"), not_("sold.Actor")])
                    .then([insert("sold.Actor")]),
            )
            .action(
                Action::new("[Actor]: greet a neighbour")
                    .when([not_("neighbour.Actor")])
                    .then([insert("neighbour.Actor")]),
            )
            .action(Action::new("[Actor]: Wait about"));
        let mut st = State::new();
        st.define_practices([town]).unwrap();
        st.set_characters(vec![
            Character::new("priya").want(Want::new(vec![m("sold.beth")], 5)),
            Character::new("beth")
                .holds("hates-lying")
                .holds("wants-market")
                .holds("counts-neighbours"),
        ])
        .unwrap();
        st.set_desires(vec![
            Desire::new("hates-lying", Want::new(vec![m("lied.Owner")], -5)),
            Desire::new(
                "wants-market",
                Want::new(vec![m("marketDay"), m("sold.Owner")], 5),
            ),
            Desire::new(
                "counts-neighbours",
                Want::new(
                    vec![
                        subquery("Ns", vec!["N".to_owned()], vec![m("neighbour.N")]),
                        count("K", "Ns"),
                        cmp(CmpOp::Gte, "K", "1"),
                    ],
                    5,
                ),
            ),
        ])
        .unwrap();
        // The schedule is not a mover: the fact it moves stays an environment gate.
        st.set_schedule(vec![
            ScheduleRule::new("market", 2).clause([not_("marketDay")], [insert("marketDay")]),
        ])
        .unwrap();
        st
    }

    fn dead_now_shut() -> State {
        let mut st = dead_now_base();
        for o in [
            insert("practice.town.here"),
            insert("priya.believes.desires.beth.hates-lying.heard.gossip"),
            insert("priya.believes.desires.beth.wants-market.heard.gossip"),
        ] {
            st.perform_outcome(&o).unwrap();
        }
        st
    }

    fn canary_world() -> State {
        let stage = Practice::new("stage")
            .roles(["R"])
            .action(
                Action::new("[Actor]: raise the mark")
                    .when([eq("Actor", "alice"), not_("raised.Actor")])
                    .then([insert("raised.Actor"), insert("mark.p")]),
            )
            .action(
                Action::new("[Actor]: reach for the shelf")
                    .when([eq("Actor", "alice"), not_("reached.Actor")])
                    .then([insert("reached.Actor")]),
            )
            .action(
                Action::new("[Actor]: take the small mark")
                    .when([
                        eq("Actor", "alice"),
                        m("reached.Actor"),
                        not_("mark.s"),
                    ])
                    .then([insert("mark.s")]),
            )
            .action(
                Action::new("[Actor]: swap the marks")
                    .when([eq("Actor", "bob"), m("mark.p")])
                    .then([delete("mark.p"), insert("mark.q")]),
            )
            .action(
                Action::new("[Actor]: tidy the marks")
                    .when([eq("Actor", "cara"), m("mark.q")])
                    .then([delete("mark.q")]),
            )
            .action(
                Action::new("[Actor]: take a seat")
                    .when([eq("Actor", "dan"), not_("chair.Actor")])
                    .then([insert("chair.Actor")]),
            )
            .action(
                Action::new("[Actor]: polish the small mark")
                    .when([eq("Actor", "eve"), not_("mark.s")])
                    .then([insert("mark.s")]),
            );
        let mut st = State::new();
        st.define_practices([stage]).unwrap();
        st.set_characters(vec![
            Character::new("alice")
                .want(Want::new(vec![m("mark.p")], 12))
                .want(Want::new(vec![m("mark.q")], 7))
                .want(Want::new(vec![m("mark.s")], 1)),
            Character::new("bob"),
            Character::new("cara"),
            Character::new("dan"),
            Character::new("eve"),
        ])
        .unwrap();
        st.set_desires(vec![
            Desire::new("swap-marks", Want::new(vec![m("mark.q")], 5)),
            Desire::new("tidy-marks", Want::new(vec![not_("mark.q")], 5)),
            Desire::new("take-a-seat", Want::new(vec![m("chair.Owner")], 5)),
        ])
        .unwrap();
        for o in [
            insert("practice.stage.here"),
            insert("alice.believes.desires.bob.swap-marks.heard.gossip"),
            insert("alice.believes.desires.cara.tidy-marks.heard.gossip"),
            insert("alice.believes.desires.dan.take-a-seat.heard.gossip"),
        ] {
            st.perform_outcome(&o).unwrap();
        }
        st
    }

    fn perform_all(st: &mut State, outs: impl IntoIterator<Item = Outcome>) {
        for o in outs {
            st.perform_outcome(&o).unwrap();
        }
    }

    /// The Rust twin of the oracle's `plannerWorlds`, by name.
    fn world(name: &str) -> State {
        match name {
            "tendBar: two instances, two customers" => {
                let mut st = State::new();
                st.define_practices([tend_bar()]).unwrap();
                st.set_characters(vec![
                    Character::new("beth").want(Want::new(
                        vec![m("practice.tendBar.Bartender.customer.beth!order!cider")],
                        10,
                    )),
                    Character::new("dana").want(Want::new(
                        vec![m("practice.tendBar.Bartender.customer.dana!order!soda")],
                        8,
                    )),
                    Character::new("ada"),
                    Character::new("cleo"),
                ])
                .unwrap();
                perform_all(
                    &mut st,
                    [
                        insert("practice.tendBar.ada"),
                        insert("practice.tendBar.cleo"),
                        insert("practice.tendBar.ada.customer.beth"),
                    ],
                );
                st
            }
            "forall-host: a universal desire and a vacuous implication" => {
                let serve = Practice::new("serve")
                    .name("[Host] hosts")
                    .roles(["Host"])
                    .action(
                        Action::new("[Actor]: pour a drink for [Guest]")
                            .when([m("guest.Guest"), not_("hasDrink.Guest")])
                            .then([insert("hasDrink.Guest")]),
                    )
                    .action(Action::new("[Actor]: rest"));
                let mut st = State::new();
                st.define_practices([serve]).unwrap();
                st.set_characters(vec![
                    Character::new("host")
                        .want(Want::new(
                            vec![for_all(vec![m("guest.G")], vec![m("hasDrink.G")])],
                            10,
                        ))
                        .want(Want::new(
                            vec![implies(vec![m("raining")], vec![m("wet")])],
                            4,
                        )),
                    Character::new("b"),
                ])
                .unwrap();
                perform_all(
                    &mut st,
                    [
                        insert("guest.a"),
                        insert("guest.b"),
                        insert("hasDrink.a"),
                        insert("practice.serve.host"),
                    ],
                );
                st
            }
            "models: gossiped, seen, and false believed minds" => {
                let mut st = State::new();
                st.define_practices([tend_bar()]).unwrap();
                st.set_characters(vec![
                    Character::new("ada"),
                    Character::new("beth").holds("cider-craving"),
                    Character::new("cleo"),
                    Character::new("dana").holds("cider-craving"),
                ])
                .unwrap();
                st.set_desires(vec![Desire::new(
                    "cider-craving",
                    Want::new(
                        vec![m("practice.tendBar.Bartender.customer.Owner!order!cider")],
                        10,
                    ),
                )])
                .unwrap();
                perform_all(
                    &mut st,
                    [
                        insert("practice.tendBar.ada"),
                        insert("practice.tendBar.ada.customer.beth"),
                        insert("practice.tendBar.ada.customer.cleo"),
                        insert("practice.tendBar.ada.customer.dana"),
                        insert("ada.believes.desires.beth.cider-craving.heard.gossip"),
                        insert("ada.believes.desires.dana.cider-craving.seen"),
                        insert("ada.believes.desires.cleo.cider-craving.presumed"),
                    ],
                );
                st
            }
            "scope: the pair apart" => {
                let mut st = heist_base();
                perform_all(
                    &mut st,
                    [
                        insert("practice.heist.here"),
                        insert("olaf.believes.desires.inge.covet-relic.heard.inge"),
                        insert("at.olaf!gatehouse"),
                        insert("at.inge!vault"),
                    ],
                );
                st
            }
            "scope: the pair together" => {
                let mut st = heist_base();
                perform_all(
                    &mut st,
                    [
                        insert("practice.heist.here"),
                        insert("olaf.believes.desires.inge.covet-relic.heard.inge"),
                        insert("at.olaf!vault"),
                        insert("at.inge!vault"),
                    ],
                );
                st
            }
            "deadNow: floor shut, gate shut, subquery always live" => dead_now_shut(),
            "deadNow: floor marked, gate open" => {
                let mut st = dead_now_shut();
                perform_all(&mut st, [insert("lied.beth"), insert("marketDay")]);
                st
            }
            "reuse: the cone-mediated read (a derived head only)" => {
                let court = Practice::new("court")
                    .roles(["R"])
                    .action(
                        Action::new("[Actor]: denounce beth")
                            .when([neq("Actor", "beth")])
                            .then([insert("Actor.believes.beth.thief")]),
                    )
                    .action(
                        Action::new("[Actor]: make amends")
                            .when([m("regards.V.Actor.thief")])
                            .then([insert("recanted.Actor"), insert("apology.Actor")]),
                    )
                    .action(Action::new("[Actor]: bide time"));
                let mut st = State::new();
                st.define_practices([court]).unwrap();
                st.set_characters(vec![
                    Character::new("priya").want(Want::new(vec![m("apology.beth")], 10)),
                    Character::new("beth"),
                ])
                .unwrap();
                st.set_desires(vec![Desire::new(
                    "hates-infamy",
                    Want::new(vec![m("regards.V.Owner.thief")], -8),
                )])
                .unwrap();
                st.set_axioms(vec![Axiom::new(
                    vec![m("W.believes.C.thief"), not_("recanted.C")],
                    ["regards.W.C.thief"],
                )])
                .unwrap();
                perform_all(
                    &mut st,
                    [
                        insert("practice.court.here"),
                        insert("priya.believes.desires.beth.hates-infamy.heard.gossip"),
                    ],
                );
                st
            }
            "reuse: the eviction shadow (an exclusion displaces the read)" => {
                let parlour = Practice::new("parlour")
                    .roles(["R"])
                    .action(
                        Action::new("[Actor]: mope")
                            .when([
                                eq("Actor", "beth"),
                                m("mood.Actor!sad"),
                                not_("moped.Actor"),
                            ])
                            .then([insert("moped.Actor")]),
                    )
                    .action(
                        // Guarded on the ACTOR alone: see the oracle's note —
                        // any mood-family guard would itself be a read anchor
                        // and the eviction shadow would never be load-bearing.
                        Action::new("[Actor]: console beth")
                            .when([eq("Actor", "alice")])
                            .then([insert("mood.beth!happy")]),
                    )
                    .action(Action::new("[Actor]: Wait about"));
                let mut st = State::new();
                st.define_practices([parlour]).unwrap();
                st.set_characters(vec![
                    Character::new("alice").want(Want::new(vec![m("moped.beth")], 6)),
                    Character::new("beth"),
                ])
                .unwrap();
                st.set_desires(vec![Desire::new(
                    "wants-to-mope",
                    Want::new(vec![m("moped.Owner")], 5),
                )])
                .unwrap();
                perform_all(
                    &mut st,
                    [
                        insert("practice.parlour.here"),
                        insert("mood.beth!sad"),
                        insert("alice.believes.desires.beth.wants-to-mope.heard.gossip"),
                    ],
                );
                st
            }
            "collision: a Calc-minted constant colliding with a scope literal" => {
                let signal = Practice::new("signal")
                    .roles(["R"])
                    .action(
                        // Guarded on `computed.Actor`, never on `gate.2`: a
                        // `gate.2` guard would itself anchor the gate family for
                        // every mover, making the scope's read-anchor
                        // contribution a duplicate and the fixture inert.
                        Action::new("[Actor]: compute the gate")
                            .when([
                                eq("Actor", "alice"),
                                not_("computed.Actor"),
                                calc("Sum", CalcOp::Add, "1", "1"),
                            ])
                            .then([insert("computed.Actor"), insert("gate.Sum")]),
                    )
                    .action(
                        Action::new("[Actor]: cheer")
                            .when([eq("Actor", "bob"), not_("cheer.Actor")])
                            .then([insert("cheer.Actor")]),
                    )
                    .action(Action::new("[Actor]: Wait about"));
                let mut st = State::new();
                st.define_practices([signal]).unwrap();
                st.set_characters(vec![
                    Character::new("alice").want(Want::new(vec![m("cheer.bob")], 4)),
                    Character::new("bob"),
                ])
                .unwrap();
                st.set_desires(vec![Desire::new(
                    "wants-cheer",
                    Want::new(vec![m("cheer.Owner")], 5),
                )])
                .unwrap();
                st.set_prediction_scope(vec![m("gate.2")]).unwrap();
                perform_all(
                    &mut st,
                    [
                        insert("practice.signal.here"),
                        insert("alice.believes.desires.bob.wants-cheer.heard.gossip"),
                    ],
                );
                st
            }
            "wild Call: cares_about bears on everyone" => {
                let rumour = Practice::new("rumour")
                    .roles(["R"])
                    .action(
                        Action::new("[Actor]: rouse the room")
                            .then([call("noSuchFunction", vec![])]),
                    )
                    .action(
                        Action::new("[Actor]: hush")
                            .when([not_("quiet.Actor")])
                            .then([insert("quiet.Actor")]),
                    )
                    .action(Action::new("[Actor]: Wait about"));
                let mut st = State::new();
                st.define_practices([rumour]).unwrap();
                st.set_characters(vec![
                    Character::new("alice").want(Want::new(vec![m("quiet.alice")], 3)),
                    Character::new("beth").holds("wants-quiet"),
                ])
                .unwrap();
                st.set_desires(vec![Desire::new(
                    "wants-quiet",
                    Want::new(vec![m("quiet.Owner")], 5),
                )])
                .unwrap();
                perform_all(&mut st, [insert("practice.rumour.here")]);
                st
            }
            "the fold-order canary" => canary_world(),
            other => panic!("planner.json world has no Rust twin: {other:?}"),
        }
    }

    // ---- the checks ---------------------------------------------------------

    fn check_world(st: &mut State, w: &Value) {
        let name = w["name"].as_str().expect("world name");

        // improvables, in table order.
        assert_eq!(
            st.improvables().to_vec(),
            strs(&w["improvables"]),
            "improvables @ {name}"
        );

        // liveness: tag + rendered gates.
        let mut want_liveness: BTreeMap<String, (String, Vec<Vec<String>>)> = BTreeMap::new();
        for (k, v) in w["liveness"].as_object().expect("liveness object") {
            let entry = match v {
                Value::String(tag) => (tag.clone(), Vec::new()),
                Value::Object(o) => {
                    let gates = o["GateCheck"]
                        .as_array()
                        .expect("GateCheck array")
                        .iter()
                        .map(strs)
                        .collect();
                    ("GateCheck".to_owned(), gates)
                }
                other => panic!("liveness entry {other:?} @ {name}"),
            };
            want_liveness.insert(k.clone(), entry);
        }
        assert_eq!(st.liveness_rendered(), want_liveness, "liveness @ {name}");

        // caresAbout.
        let want_cares: BTreeMap<String, Vec<String>> = w["caresAbout"]
            .as_object()
            .expect("caresAbout object")
            .iter()
            .map(|(k, v)| (k.clone(), strs(v)))
            .collect();
        assert_eq!(
            st.cares_about_table().clone(),
            want_cares,
            "caresAbout @ {name}"
        );

        // moverReadAnchors, per ordered pair, in native walk order.
        for row in w["readAnchors"].as_array().expect("readAnchors array") {
            let actor = row["actor"].as_str().expect("actor");
            let mover = row["mover"].as_str().expect("mover");
            assert_eq!(
                st.mover_read_anchor_names(actor, mover),
                strs(&row["anchors"]),
                "moverReadAnchors {actor}->{mover} @ {name}"
            );
        }

        let cast: Vec<Character> = st.characters().to_vec();
        let by_name = |n: &str| -> Character {
            cast.iter()
                .find(|c| c.name == n)
                .unwrap_or_else(|| panic!("no cast member {n} @ {name}"))
                .clone()
        };

        // predictMove, per ordered pair.
        for row in w["predict"].as_array().expect("predict array") {
            let p = by_name(row["predictor"].as_str().expect("predictor"));
            let mv = by_name(row["mover"].as_str().expect("mover"));
            let got = st.predict_move(&p, &mv).map(|g| g.label);
            let want = row["action"].as_str().map(str::to_owned);
            assert_eq!(got, want, "predictMove {}->{} @ {name}", p.name, mv.name);
        }

        // motiveSignature, field for field.
        for row in w["signatures"].as_array().expect("signatures array") {
            let c = by_name(row["character"].as_str().expect("character"));
            let sig = st.motive_signature(&c);
            let want = &row["signature"];
            assert_eq!(sig.bearing, strs(&want["bearing"]), "bearing {} @ {name}", c.name);
            let sat: Vec<usize> = want["satisfaction"]
                .as_array()
                .expect("satisfaction array")
                .iter()
                .map(|n| usize::try_from(n.as_u64().expect("count")).expect("count fits"))
                .collect();
            assert_eq!(sig.satisfaction, sat, "satisfaction {} @ {name}", c.name);
            assert_eq!(
                sig.live_desires,
                strs(&want["liveDesires"]),
                "liveDesires {} @ {name}",
                c.name
            );
            let known: Vec<(String, String)> = want["knownMotives"]
                .as_array()
                .expect("knownMotives array")
                .iter()
                .map(|pair| {
                    let p = strs(pair);
                    (p[0].clone(), p[1].clone())
                })
                .collect();
            assert_eq!(sig.known_motives, known, "knownMotives {} @ {name}", c.name);
        }

        // candidateActions, in enumeration order.
        for row in w["candidates"].as_array().expect("candidates array") {
            let c = by_name(row["character"].as_str().expect("character"));
            let got: Vec<String> = st
                .candidate_actions(&c)
                .into_iter()
                .map(|g| g.label)
                .collect();
            assert_eq!(got, strs(&row["actions"]), "candidates {} @ {name}", c.name);
        }

        // scoreActions: NATIVE result order [D-C1], scores as RAW BITS [D-I1].
        for row in w["scored"].as_array().expect("scored array") {
            let c = by_name(row["actor"].as_str().expect("actor"));
            let depth = i32::try_from(row["depth"].as_i64().expect("depth")).expect("depth fits");
            let got = st.score_actions(depth, &c);
            let want = row["rows"].as_array().expect("rows array");
            assert_eq!(
                got.len(),
                want.len(),
                "scored row count for {} at depth {depth} @ {name}",
                c.name
            );
            for (i, (ga, score)) in got.iter().enumerate() {
                assert_eq!(
                    ga.label,
                    want[i]["label"].as_str().expect("label"),
                    "scored label #{i} for {} at depth {depth} @ {name}",
                    c.name
                );
                let want_bits = want[i]["bits"].as_u64().expect("raw score bits");
                assert_eq!(
                    score.to_bits(),
                    want_bits,
                    "SCORE BITS #{i} ({}) for {} at depth {depth} @ {name}: \
                     got {score:?} (bits {}), frozen bits {want_bits}",
                    ga.label,
                    c.name,
                    score.to_bits()
                );
            }
        }

        // pickAction.
        for row in w["pick"].as_array().expect("pick array") {
            let c = by_name(row["actor"].as_str().expect("actor"));
            let depth = i32::try_from(row["depth"].as_i64().expect("depth")).expect("depth fits");
            let got = st.pick_action(depth, &c).map(|g| g.label);
            assert_eq!(
                got,
                row["action"].as_str().map(str::to_owned),
                "pickAction {} at depth {depth} @ {name}",
                c.name
            );
        }
    }

    // FIXTURE REPLAY: planner.json — every synthetic world's full planner
    // observable set, asserted against the frozen engine's dump. Scores compare
    // as raw u64 bit patterns; the scored tables compare in native order.
    #[test]
    fn planner_corpus_replays_bit_for_bit() {
        let data = load();
        let worlds = data["worlds"].as_array().expect("worlds array");
        assert!(!worlds.is_empty(), "planner.json has no worlds");
        for w in worlds {
            let name = w["name"].as_str().expect("world name");
            let mut st = world(name);
            check_world(&mut st, w);
        }
    }

    // The canary world's discriminating row, called out by name so a failure
    // reads as what it is: the fold association moved. `12 + (3.5 + 0.9*0.9)`
    // and `(12 + 3.5) + 0.9*0.9` differ by exactly one ULP.
    #[test]
    fn the_fold_order_canary_world_is_present_and_discriminating() {
        let data = load();
        let w = data["worlds"]
            .as_array()
            .expect("worlds")
            .iter()
            .find(|w| w["name"] == "the fold-order canary")
            .expect("the canary world must be in the corpus");
        let row = w["scored"]
            .as_array()
            .expect("scored")
            .iter()
            .find(|r| r["actor"] == "alice" && r["depth"] == 2)
            .expect("alice at depth 2")["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .find(|r| r["label"] == "alice: raise the mark")
            .expect("the raise row")
            .clone();
        let bits = row["bits"].as_u64().expect("bits");
        assert_eq!(
            bits, 4625284074552279696,
            "the corpus canary must carry the RIGHT association's bits"
        );
        let wrong = ((12.0_f64 + 3.5) + 0.9 * 0.9).to_bits();
        assert_ne!(
            bits, wrong,
            "the corpus canary is inert — it must separate the two associations"
        );
        assert_eq!(wrong, 4625284074552279695);
    }
}
