//! `Prax.TypeCheckSpec`, re-expressed against the Rust `type_check`.
//!
//! The frozen suite is 55 cases: the shipped-worlds `type_check == []` standing
//! net, plus SHOULD-flag synthetic fixtures (one+ per constructor) that exercise
//! each verdict, plus the clean counter-cases that prove the checker does not
//! over-flag. Every shipped world is well-formed — that is the standing net —
//! but the CHECKER's job is catching MALFORMED worlds, so most pins are synthetic
//! fixtures that SHOULD flag, asserted against the Rust verdict directly (they are
//! not shipped worlds and never reach the oracle `check` comparator, §1.4).
//!
//! Ported from `test/Prax/TypeCheckSpec.hs`, case for case, in frozen order.

#[cfg(test)]
mod tests {
    // H: TypeCheckSpec.hs "Prax.TypeCheck"
    //
    // The frozen `Prax.TypeCheck` group.
    use prax_core::engine::State;
    use prax_core::query::{CmpOp, Condition, cmp, count, exists, matches, not_, or_, subquery};
    use prax_core::rng::draw;
    use prax_core::typecheck::{TypeError, type_check};
    use prax_core::types::{
        Action, Axiom, Character, Desire, Function, Outcome, Practice, ScheduleRule, Want, call,
        delete, insert, insert_for,
    };
    use prax_vocab::deontic::obliged_close;
    use prax_worlds::audience::audience_world;
    use prax_worlds::bar::{bar_director_world, bar_world};
    use prax_worlds::feud::feud_world;
    use prax_worlds::intrigue::intrigue_world;
    use prax_worlds::play::play_world;
    use prax_worlds::village::village_world;

    fn m(s: &str) -> Condition {
        matches(s)
    }

    /// The frozen `world1 p = definePractices [p] emptyState`.
    fn world1(p: Practice) -> State {
        let mut st = State::new();
        st.define_practices([p]).expect("a well-formed practice registers");
        st
    }

    fn has_unbound(errs: &[TypeError], v: &str) -> bool {
        errs.iter().any(|e| matches!(e, TypeError::UnboundVar { var, .. } if var == v))
    }
    fn has_unbound_at(errs: &[TypeError], site: &str, v: &str) -> bool {
        errs.iter()
            .any(|e| matches!(e, TypeError::UnboundVar { where_, var, .. } if where_ == site && var == v))
    }
    fn has_undefined(errs: &[TypeError], name: &str) -> bool {
        errs.iter().any(|e| matches!(e, TypeError::UndefinedRef { name: n, .. } if n == name))
    }
    fn has_reserved(errs: &[TypeError], fam: &str, sent: &str) -> bool {
        errs.iter().any(
            |e| matches!(e, TypeError::ReservedFamily { family, sentence, .. } if family == fam && sentence == sent),
        )
    }
    fn has_seedless(errs: &[TypeError]) -> bool {
        errs.iter().any(|e| matches!(e, TypeError::SeedlessDraw))
    }
    fn has_dead(errs: &[TypeError]) -> bool {
        errs.iter().any(|e| matches!(e, TypeError::DeadCondition { .. }))
    }
    fn has_sort_conflict(errs: &[TypeError]) -> bool {
        errs.iter().any(|e| matches!(e, TypeError::SortConflict { .. }))
    }

    // H: TypeCheckSpec.hs "every shipped world is well-formed"
    #[test]
    fn every_shipped_world_is_well_formed() {
        // The standing net: the shipped worlds' `type_check == []`. village's
        // drawn-to-market desire reads marketDay.square (schedule-only), which
        // `producible_atoms` folds in; feud's kin axiom remainder is out of the
        // dead-condition lint's scope.
        assert_eq!(type_check(&bar_world()), []);
        assert_eq!(type_check(&bar_director_world()), []);
        assert_eq!(type_check(&intrigue_world()), []);
        assert_eq!(type_check(&play_world()), []);
        assert_eq!(type_check(&feud_world()), []);
        assert_eq!(type_check(&village_world()), []);
        assert_eq!(type_check(&audience_world()), []);
    }

    // H: TypeCheckSpec.hs "an outcome variable bound by nothing is caught"
    #[test]
    fn an_outcome_variable_bound_by_nothing_is_caught() {
        let p = Practice::new("bug")
            .roles(["R"])
            .action(Action::new("[Actor]: x").then([insert("foo.Ghost")]));
        assert!(has_unbound(&type_check(&world1(p)), "Ghost"));
    }

    // H: TypeCheckSpec.hs "an axiom head variable absent from the body is caught"
    #[test]
    fn an_axiom_head_variable_absent_from_the_body_is_caught() {
        let mut st = State::new();
        st.set_axioms(vec![Axiom::new(vec![m("p.X")], ["q.X.Y"])]).unwrap();
        assert!(has_unbound_at(&type_check(&st), "axiom", "Y"));
    }

    // H: TypeCheckSpec.hs "a relation used as both ! and . is caught"
    #[test]
    fn a_relation_used_as_both_excl_and_dot_is_caught() {
        let p = Practice::new("c")
            .action(Action::new("[Actor]: a").then([insert("a.mood!happy")]))
            .action(Action::new("[Actor]: b").then([insert("a.mood.sad")]));
        assert!(type_check(&world1(p)).iter().any(
            |e| matches!(e, TypeError::CardinalityClash { slot } if slot == "a.mood")
        ));
    }

    // H: TypeCheckSpec.hs "a Call to an undefined function is caught"
    #[test]
    fn a_call_to_an_undefined_function_is_caught() {
        let p = Practice::new("d")
            .roles(["R"])
            .action(Action::new("[Actor]: y").then([call("nope", vec!["R".into()])]));
        assert!(has_undefined(&type_check(&world1(p)), "nope"));
    }

    // H: TypeCheckSpec.hs "spawning an undefined practice is caught"
    #[test]
    fn spawning_an_undefined_practice_is_caught() {
        let p = Practice::new("e")
            .roles(["R"])
            .action(Action::new("[Actor]: z").then([insert("practice.ghost.R")]));
        assert!(has_undefined(&type_check(&world1(p)), "practice.ghost"));
    }

    // H: TypeCheckSpec.hs "an unbound variable in a registered function's case is caught, sited at fn <name>"
    #[test]
    fn an_unbound_variable_in_a_registered_functions_case_is_caught() {
        let f = Function::new("grant", ["P"]).case([], [insert("gift.Ghost")]);
        let mut st = State::new();
        st.define_functions([f]).unwrap();
        assert!(has_unbound_at(&type_check(&st), "fn grant", "Ghost"));
    }

    // H: TypeCheckSpec.hs "a correct little practice is well-formed"
    #[test]
    fn a_correct_little_practice_is_well_formed() {
        let p = Practice::new("ok")
            .roles(["R"])
            .init([insert("here.someone")])
            .action(
                Action::new("[Actor]: greet [R]")
                    .when([m("here.Actor"), m("here.R")])
                    .then([insert("greeted.Actor.R")]),
            );
        assert_eq!(type_check(&world1(p)), []);
    }

    // H: TypeCheckSpec.hs "no sort declarations ⇒ no sort errors"
    #[test]
    fn no_sort_declarations_no_sort_errors() {
        let p = Practice::new("z")
            .action(Action::new("[Actor]: a").then([insert("cup.beer"), insert("cup.bar")]));
        assert_eq!(type_check(&world1(p)), []);
    }

    // H: TypeCheckSpec.hs "a position given values of two sorts is caught"
    #[test]
    fn a_position_given_values_of_two_sorts_is_caught() {
        let p = Practice::new("menu")
            .action(Action::new("[Actor]: pour a beer").then([insert("cup.beer")]))
            .action(Action::new("[Actor]: pour a bar!").then([insert("cup.bar")]));
        let mut st = world1(p);
        st.set_sorts(vec![
            ("beverage".into(), vec!["beer".into()]),
            ("place".into(), vec!["bar".into()]),
        ])
        .unwrap();
        assert!(type_check(&st).iter().any(
            |e| matches!(e, TypeError::SortConflict { where_, .. } if where_ == "cup")
        ));
    }

    // H: TypeCheckSpec.hs "a variable used in two different sorts is caught"
    #[test]
    fn a_variable_used_in_two_different_sorts_is_caught() {
        let p = Practice::new("v")
            .roles(["X"])
            .action(Action::new("[Actor]: mix").then([insert("cup.X"), insert("spot.X")]));
        let mut st = world1(p);
        st.set_sorts(vec![
            ("beverage".into(), vec!["beer".into()]),
            ("place".into(), vec!["bar".into()]),
        ])
        .unwrap();
        st.perform_outcome(&insert("cup.beer")).unwrap();
        st.perform_outcome(&insert("spot.bar")).unwrap();
        assert!(has_sort_conflict(&type_check(&st)));
    }

    // H: TypeCheckSpec.hs "a constant declared in two sorts is caught"
    #[test]
    fn a_constant_declared_in_two_sorts_is_caught() {
        let mut st = State::new();
        st.set_sorts(vec![
            ("agent".into(), vec!["x".into()]),
            ("beverage".into(), vec!["x".into()]),
        ])
        .unwrap();
        assert!(type_check(&st).iter().any(|e| matches!(
            e, TypeError::SortConflict { where_, detail }
                if where_ == "x" && detail.contains("agent") && detail.contains("beverage")
        )));
    }

    // H: TypeCheckSpec.hs "a variable bound by ForEach conditions is not unbound"
    #[test]
    fn a_variable_bound_by_foreach_conditions_is_not_unbound() {
        let p = Practice::new("w")
            .roles(["R"])
            .init([insert("member.someone")])
            .action(Action::new("[Actor]: broadcast").then([Outcome::ForEach(
                vec![m("member.X")],
                vec![insert("told.X")],
            )]));
        assert_eq!(type_check(&world1(p)), []);
    }

    // H: TypeCheckSpec.hs "a genuinely unbound variable inside ForEach is flagged"
    #[test]
    fn a_genuinely_unbound_variable_inside_foreach_is_flagged() {
        let p = Practice::new("w")
            .roles(["R"])
            .action(Action::new("[Actor]: broadcast").then([Outcome::ForEach(
                vec![m("member.X")],
                vec![insert("told.Ghost")],
            )]));
        assert!(has_unbound(&type_check(&world1(p)), "Ghost"));
    }

    // H: TypeCheckSpec.hs "ForEach sub-inserts join the cardinality corpus"
    #[test]
    fn foreach_sub_inserts_join_the_cardinality_corpus() {
        let p = Practice::new("w")
            .roles(["R"])
            .action(Action::new("[Actor]: a").then([insert("mark.R!x")]))
            .action(Action::new("[Actor]: b").then([Outcome::ForEach(
                vec![m("member.X")],
                vec![insert("mark.X.y")],
            )]));
        assert!(type_check(&world1(p)).iter().any(|e| matches!(e, TypeError::CardinalityClash { .. })));
    }

    // H: TypeCheckSpec.hs "a dangling Call or spawn inside ForEach is caught"
    #[test]
    fn a_dangling_call_or_spawn_inside_foreach_is_caught() {
        let p = Practice::new("w").roles(["R"]).action(
            Action::new("[Actor]: broadcast").then([Outcome::ForEach(
                vec![m("member.X")],
                vec![call("nope", vec!["X".into()]), insert("practice.ghost.X")],
            )]),
        );
        let errs = type_check(&world1(p));
        assert!(has_undefined(&errs, "nope"));
        assert!(has_undefined(&errs, "practice.ghost"));
    }

    // H: TypeCheckSpec.hs "a dead action conjunct (typo'd predicate) is caught"
    #[test]
    fn a_dead_action_conjunct_is_caught() {
        let p = Practice::new("hunt")
            .init([insert("treasure.spot")])
            .action(Action::new("[Actor]: dig").when([m("tresure.spot")]).then([insert("dug.Actor")]));
        assert_eq!(
            type_check(&world1(p)),
            [dead("hunt / [Actor]: dig", "tresure.spot")]
        );
    }

    // H: TypeCheckSpec.hs "the corrected twin of the typo is well-formed"
    #[test]
    fn the_corrected_twin_of_the_typo_is_well_formed() {
        let p = Practice::new("hunt")
            .init([insert("treasure.spot")])
            .action(Action::new("[Actor]: dig").when([m("treasure.spot")]).then([insert("dug.Actor")]));
        assert_eq!(type_check(&world1(p)), []);
    }

    // H: TypeCheckSpec.hs "a dead positive inside Exists is caught"
    #[test]
    fn a_dead_positive_inside_exists_is_caught() {
        let p = Practice::new("hunt")
            .init([insert("treasure.spot")])
            .action(
                Action::new("[Actor]: dig")
                    .when([exists(vec![m("tresure.spot")])])
                    .then([insert("dug.Actor")]),
            );
        assert_eq!(
            type_check(&world1(p)),
            [dead("hunt / [Actor]: dig", "tresure.spot")]
        );
    }

    // H: TypeCheckSpec.hs "a dead ForEach guard is caught, sited as an effect guard"
    #[test]
    fn a_dead_foreach_guard_is_caught_sited_as_an_effect_guard() {
        let p = Practice::new("hunt").init([insert("treasure.spot")]).action(
            Action::new("[Actor]: search").then([Outcome::ForEach(
                vec![m("tresure.spot")],
                vec![insert("found")],
            )]),
        );
        assert_eq!(
            type_check(&world1(p)),
            [dead("hunt / [Actor]: search (effect guard)", "tresure.spot")]
        );
    }

    // H: TypeCheckSpec.hs "a dead desire and a dead character want are each caught"
    #[test]
    fn a_dead_desire_and_a_dead_character_want_are_each_caught() {
        let desire_w = Desire::new("wantGold", Want::new(vec![m("ghost.family")], 5));
        let vic = Character::new("vic").want(Want::new(vec![m("ghost.spirit")], 3));
        let mut st = State::new();
        st.set_desires(vec![desire_w]).unwrap();
        st.set_characters(vec![vic]).unwrap();
        assert_eq!(
            type_check(&st),
            [
                dead("desire wantGold", "ghost.family"),
                dead("want of vic", "ghost.spirit"),
            ]
        );
    }

    // H: TypeCheckSpec.hs "a negation over a never-produced family is not flagged"
    #[test]
    fn a_negation_over_a_never_produced_family_is_not_flagged() {
        let p = Practice::new("spookless").action(
            Action::new("[Actor]: peek").when([not_("ghost.Actor")]).then([insert("peeked.Actor")]),
        );
        assert_eq!(type_check(&world1(p)), []);
    }

    // H: TypeCheckSpec.hs "a half-dead Or clause is not flagged"
    #[test]
    fn a_half_dead_or_clause_is_not_flagged() {
        let p = Practice::new("hunt").init([insert("treasure.spot")]).action(
            Action::new("[Actor]: dig")
                .when([or_(vec![vec![m("tresure.spot")], vec![m("treasure.spot")]])])
                .then([insert("dug.Actor")]),
        );
        assert_eq!(type_check(&world1(p)), []);
    }

    // H: TypeCheckSpec.hs "a dead pattern inside a Subquery interior is not flagged"
    #[test]
    fn a_dead_pattern_inside_a_subquery_interior_is_not_flagged() {
        let p = Practice::new("hunt").action(
            Action::new("[Actor]: check")
                .when([
                    subquery("S", vec![], vec![m("tresure.spot")]),
                    count("N", "S"),
                    cmp(CmpOp::Lte, "N", "0"),
                ])
                .then([insert("checked.Actor")]),
        );
        assert_eq!(type_check(&world1(p)), []);
    }

    // H: TypeCheckSpec.hs "a fully unanchored pattern (every segment a variable) is not flagged"
    #[test]
    fn a_fully_unanchored_pattern_is_not_flagged() {
        let p = Practice::new("hunt")
            .roles(["X", "Y"])
            .action(Action::new("[Actor]: link").when([m("X.Y")]).then([insert("linked.X.Y")]));
        assert_eq!(type_check(&world1(p)), []);
    }

    // H: TypeCheckSpec.hs "a wild world (undefined Call) silences the lint, not the ref check"
    #[test]
    fn a_wild_world_silences_the_lint_not_the_ref_check() {
        let p = Practice::new("hunt").action(
            Action::new("[Actor]: dig").when([m("tresure.spot")]).then([call("missingFn", vec![])]),
        );
        let errs = type_check(&world1(p));
        assert!(has_undefined(&errs, "missingFn"), "UndefinedRef missingFn fires");
        assert!(!has_dead(&errs), "no DeadCondition (the wild pool silences the lint)");
    }

    // H: TypeCheckSpec.hs "a dead axiom body is not flagged (axiom bodies are out of scope)"
    #[test]
    fn a_dead_axiom_body_is_not_flagged() {
        let mut st = State::new();
        st.set_axioms(vec![Axiom::new(vec![m("parent.P.C")], ["kin.P.C"])]).unwrap();
        assert_eq!(type_check(&st), []);
    }

    // H: TypeCheckSpec.hs "an authored write to the engine clock is flagged"
    #[test]
    fn an_authored_write_to_the_engine_clock_is_flagged() {
        let p = Practice::new("clocksmith")
            .action(Action::new("[Actor]: forge time").then([insert("turn!99")]));
        assert!(has_reserved(&type_check(&world1(p)), "turn", "turn!99"));
    }

    // H: TypeCheckSpec.hs "an axiom head deriving the clock family is flagged"
    #[test]
    fn an_axiom_head_deriving_the_clock_family_is_flagged() {
        let mut st = State::new();
        st.set_axioms(vec![Axiom::new(vec![m("ping.X")], ["turn!5"])]).unwrap();
        assert!(type_check(&st).iter().any(
            |e| matches!(e, TypeError::ReservedFamily { family, where_, sentence }
                if family == "turn" && where_ == "axiom" && sentence == "turn!5")
        ));
    }

    // H: TypeCheckSpec.hs "a performOutcome clock-jump is NOT flagged (typeCheck sees no authored write)"
    #[test]
    fn a_perform_outcome_clock_jump_is_not_flagged() {
        let ok = Practice::new("ok").action(Action::new("[Actor]: wait").then([]));
        let mut st = world1(ok);
        st.perform_outcome(&insert("turn!42")).unwrap();
        assert_eq!(type_check(&st), []);
    }

    // H: TypeCheckSpec.hs "an authored Delete of turn is flagged (the strengthening)"
    #[test]
    fn an_authored_delete_of_turn_is_flagged() {
        let p = Practice::new("clocksmith2")
            .action(Action::new("[Actor]: erase time").then([delete("turn")]));
        assert!(has_reserved(&type_check(&world1(p)), "turn", "turn"));
    }

    // H: TypeCheckSpec.hs "SeedlessDraw: an unseeded world with a draw is flagged"
    #[test]
    fn seedless_draw_an_unseeded_world_with_a_draw_is_flagged() {
        let p = Practice::new("gambler").action(
            Action::new("[Actor]: roll").then(draw(1, 2, vec![], vec![insert("hit.Actor")]).unwrap()),
        );
        assert!(has_seedless(&type_check(&world1(p))));
    }

    // H: TypeCheckSpec.hs "SeedlessDraw: seedDie clears it"
    #[test]
    fn seedless_draw_seed_die_clears_it() {
        let p = Practice::new("gambler").action(
            Action::new("[Actor]: roll").then(draw(1, 2, vec![], vec![insert("hit.Actor")]).unwrap()),
        );
        let mut st = world1(p);
        st.seed_die(7).unwrap();
        assert!(!has_seedless(&type_check(&st)));
    }

    // H: TypeCheckSpec.hs "SeedlessDraw: a draw nested under a ForEach is still found"
    #[test]
    fn seedless_draw_a_draw_nested_under_a_foreach_is_still_found() {
        let p = Practice::new("gambler2").action(Action::new("[Actor]: roll all").then([
            Outcome::ForEach(vec![m("here.X")], draw(1, 2, vec![], vec![insert("hit.X")]).unwrap()),
        ]));
        assert!(has_seedless(&type_check(&world1(p))));
    }

    // H: TypeCheckSpec.hs "SeedlessDraw: a draw in a schedule rule body is found (v50 T1 review M2)"
    #[test]
    fn seedless_draw_a_draw_in_a_schedule_rule_body_is_found() {
        let r = ScheduleRule::new("storms", 3)
            .clause(vec![], draw(1, 2, vec![], vec![insert("storm.here")]).unwrap());
        let mut st = world1(Practice::new("p"));
        st.set_schedule(vec![r]).unwrap();
        assert!(has_seedless(&type_check(&st)));
    }

    // H: TypeCheckSpec.hs "an authored Insert of contradiction is flagged"
    #[test]
    fn an_authored_insert_of_contradiction_is_flagged() {
        let p = Practice::new("sophist")
            .action(Action::new("[Actor]: break logic").then([insert("contradiction")]));
        assert!(has_reserved(&type_check(&world1(p)), "contradiction", "contradiction"));
    }

    // H: TypeCheckSpec.hs "an authored Match on contradiction is clean (reads free)"
    #[test]
    fn an_authored_match_on_contradiction_is_clean() {
        let p = Practice::new("sophist2")
            .action(Action::new("[Actor]: check logic").when([m("contradiction")]).then([]));
        assert_eq!(type_check(&world1(p)), []);
    }

    // H: TypeCheckSpec.hs "an authored practice-action Insert of scenePatience is flagged"
    #[test]
    fn an_authored_practice_action_insert_of_scene_patience_is_flagged() {
        let p = Practice::new("meddler")
            .action(Action::new("[Actor]: forge patience").then([insert("scenePatience.x.y")]));
        assert!(has_reserved(&type_check(&world1(p)), "scenePatience", "scenePatience.x.y"));
    }

    // H: TypeCheckSpec.hs "a reserved write NESTED under a ForEach is still flagged (v53 final review M-2)"
    #[test]
    fn a_reserved_write_nested_under_a_foreach_is_still_flagged() {
        let p = Practice::new("meddler9").init([insert("mark.here")]).action(
            Action::new("[Actor]: forge patience for everyone").then([Outcome::ForEach(
                vec![m("mark.M")],
                vec![insert("scenePatience.n.M")],
            )]),
        );
        assert!(has_reserved(&type_check(&world1(p)), "scenePatience", "scenePatience.n.M"));
    }

    // H: TypeCheckSpec.hs "an authored InsertFor of scenePatience in a practice init is flagged"
    #[test]
    fn an_authored_insert_for_of_scene_patience_in_a_practice_init_is_flagged() {
        let p = Practice::new("meddler2").init([insert_for(3, "scenePatience.a.b")]);
        assert!(has_reserved(&type_check(&world1(p)), "scenePatience", "scenePatience.a.b"));
    }

    // H: TypeCheckSpec.hs "an authored function-case write of scenePatience is flagged"
    #[test]
    fn an_authored_function_case_write_of_scene_patience_is_flagged() {
        let f = Function::new("meddle", Vec::<String>::new())
            .case([], [insert("scenePatience.f.g")]);
        let mut st = State::new();
        st.define_functions([f]).unwrap();
        assert!(has_reserved(&type_check(&st), "scenePatience", "scenePatience.f.g"));
    }

    // H: TypeCheckSpec.hs "an authored setSchedule rule body writing scenePatience is flagged"
    #[test]
    fn an_authored_set_schedule_rule_body_writing_scene_patience_is_flagged() {
        let r = ScheduleRule::new("meddle", 2).clause(vec![], vec![insert("scenePatience.s.t")]);
        let mut st = State::new();
        st.set_schedule(vec![r]).unwrap();
        assert!(has_reserved(&type_check(&st), "scenePatience", "scenePatience.s.t"));
    }

    // H: TypeCheckSpec.hs "an axiom head deriving scenePatience is flagged"
    #[test]
    fn an_axiom_head_deriving_scene_patience_is_flagged() {
        let mut st = State::new();
        st.set_axioms(vec![Axiom::new(vec![m("trigger.X")], ["scenePatience.X.j"])]).unwrap();
        assert!(type_check(&st).iter().any(
            |e| matches!(e, TypeError::ReservedFamily { family, where_, sentence }
                if family == "scenePatience" && where_ == "axiom" && sentence == "scenePatience.X.j")
        ));
    }

    // H: TypeCheckSpec.hs "an authored Delete of scenePatience is flagged (a delete is a write)"
    #[test]
    fn an_authored_delete_of_scene_patience_is_flagged() {
        let p = Practice::new("wrecker")
            .action(Action::new("[Actor]: cancel patience").then([delete("scenePatience.x.y")]));
        assert!(has_reserved(&type_check(&world1(p)), "scenePatience", "scenePatience.x.y"));
    }

    // H: TypeCheckSpec.hs "an authored write to the currentScene family is flagged"
    #[test]
    fn an_authored_write_to_the_current_scene_family_is_flagged() {
        let p = Practice::new("director")
            .action(Action::new("[Actor]: seize the stage").then([insert("currentScene!banquet")]));
        assert!(has_reserved(&type_check(&world1(p)), "currentScene", "currentScene!banquet"));
    }

    // H: TypeCheckSpec.hs "a practice injected onto compiled Audience writing its patience marker flags loudly (the v50 door)"
    #[test]
    fn a_practice_injected_onto_compiled_audience_writing_its_patience_marker_flags() {
        let meddler = Practice::new("meddler").action(
            Action::new("[Actor]: reset the clock").then([insert("scenePatience.audience.dismissed")]),
        );
        let mut st = audience_world();
        st.define_practices([meddler]).unwrap();
        assert!(has_reserved(&type_check(&st), "scenePatience", "scenePatience.audience.dismissed"));
    }

    // H: TypeCheckSpec.hs "a sightedWithin-shaped authored condition is clean (turn reads free)"
    #[test]
    fn a_sighted_within_shaped_authored_condition_is_clean() {
        let p = Practice::new("watcher")
            .init([insert("carol.believes.atSince.bob!3")])
            .action(
                Action::new("[Actor]: recall sighting")
                    .when([m("Actor.believes.atSince.Witness!Since"), m("turn!Now")])
                    .then([]),
            );
        assert_eq!(type_check(&world1(p)), []);
    }

    // H: TypeCheckSpec.hs "a world that can produce obliged.* but omits its □-closure is flagged, naming the axiom"
    #[test]
    fn a_world_that_can_produce_obliged_but_omits_closure_is_flagged() {
        let mut st = State::new();
        st.define_practices([oblige_producer()]).unwrap();
        st.set_axioms(vec![Axiom::new(vec![m("a.X")], ["b.X"])]).unwrap();
        assert_eq!(type_check(&st), [TypeError::DeonticUnclosed { sentence: "b.X".into() }]);
    }

    // H: TypeCheckSpec.hs "the same world declared via obligedClose is well-formed"
    #[test]
    fn the_same_world_declared_via_obliged_close_is_well_formed() {
        let mut st = State::new();
        st.define_practices([oblige_producer()]).unwrap();
        st.set_axioms(obliged_close(&[Axiom::new(vec![m("a.X")], ["b.X"])])).unwrap();
        assert_eq!(type_check(&st), []);
    }

    // H: TypeCheckSpec.hs "an axiomless world that can invoke obligation is clean (nothing to close)"
    #[test]
    fn an_axiomless_world_that_can_invoke_obligation_is_clean() {
        let mut st = State::new();
        st.define_practices([oblige_producer()]).unwrap();
        assert_eq!(type_check(&st), []);
    }

    // H: TypeCheckSpec.hs "a world with liftable axioms that CANNOT invoke obligation is clean (the Feud shape)"
    #[test]
    fn a_world_with_liftable_axioms_that_cannot_invoke_obligation_is_clean() {
        let mut st = State::new();
        st.set_axioms(vec![Axiom::new(vec![m("a.X")], ["b.X"])]).unwrap();
        assert_eq!(type_check(&st), []);
    }

    // H: TypeCheckSpec.hs "partial closure flags each missing twin individually (v51 final review M2)"
    #[test]
    fn partial_closure_flags_each_missing_twin_individually() {
        let a1 = Axiom::new(vec![m("a.X")], ["b.X"]);
        let a2 = Axiom::new(vec![m("c.X")], ["d.X"]);
        let mut axioms = obliged_close(&[a1]);
        axioms.push(a2);
        let mut st = State::new();
        st.define_practices([oblige_producer()]).unwrap();
        st.set_axioms(axioms).unwrap();
        assert_eq!(type_check(&st), [TypeError::DeonticUnclosed { sentence: "d.X".into() }]);
    }

    // H: TypeCheckSpec.hs "a threaten-shaped deposit for an UNREGISTERED punitive name flags CoercionUnmotivated, naming the name (v54)"
    #[test]
    fn a_threaten_shaped_deposit_for_an_unregistered_punitive_name_flags() {
        assert_eq!(
            type_check(&world1(haunt())),
            [TypeError::CoercionUnmotivated { name: "punishes-ghost".into() }]
        );
    }

    // H: TypeCheckSpec.hs "registering the punitive desire clears it — genuine (held) and bluff (not-held) are the SAME clean result (v54)"
    #[test]
    fn registering_the_punitive_desire_clears_it() {
        let ghost_desire = Desire::new("punishes-ghost", Want::new(vec![m("barn.PraxD")], 5));
        let mut st = world1(haunt());
        st.set_desires(vec![ghost_desire]).unwrap();
        assert_eq!(type_check(&st), []);
    }

    // H: TypeCheckSpec.hs "a coercion-free world names no punitive belief and is clean (v54)"
    #[test]
    fn a_coercion_free_world_names_no_punitive_belief_and_is_clean() {
        let p = Practice::new("quiet")
            .roles(["R"])
            .action(Action::new("[Actor]: greet [R]").then([insert("greeted.Actor.R")]));
        assert_eq!(type_check(&world1(p)), []);
    }

    // ---- shared fixtures (the frozen `where` clause) ------------------------

    fn dead(where_: &str, sentence: &str) -> TypeError {
        TypeError::DeadCondition { where_: where_.into(), sentence: sentence.into() }
    }

    /// A practice whose action produces an obliged.* fact (a census-true world).
    fn oblige_producer() -> Practice {
        Practice::new("oblige")
            .roles(["R"])
            .action(Action::new("[Actor]: swear a duty").then([insert("obliged.Actor.duty")]))
    }

    /// A practice that DEPOSITS a coercion motive belief for punishes-ghost (the
    /// threaten shape). Its init seeds a barn so neither the deposit action's
    /// guard nor a registered kernel want is dead.
    fn haunt() -> Practice {
        Practice::new("haunt")
            .roles(["V"])
            .init([insert("barn.here")])
            .action(
                Action::new("[Actor]: menace [V]")
                    .when([m("barn.V")])
                    .then([insert("V.believes.desires.Actor.punishes-ghost.heard.Actor")]),
            )
    }
}
