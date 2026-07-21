//! `Prax.ScheduleRuleSpec` re-expressed: schedule-rule firing (due arithmetic,
//! re-arm, late fire, per-binding coverage), the `set_schedule` construction
//! guards, the `gathering` combinator, and the two-door rule-name table with its
//! v53 provenance. All through the PUBLIC State API plus `engine::door`.
//!
//! Three labels are killed (`conformance/KILLED.md`): the two that need
//! `typeCheck` (well-formedness; the reserved-family provenance VERDICT) owe S9,
//! and the compiled-script-world collision (which needs `playWorld`) owes S8.

#[cfg(test)]
mod tests {
    use prax_core::engine::{State, door};
    use prax_core::error::WorldError;
    use prax_core::query::Condition;
    use prax_core::schedule::gathering;
    use prax_core::types::{Outcome, ScheduleRule, insert};

    fn m(s: &str) -> Condition {
        Condition::Match(s.to_owned())
    }
    fn due_of(st: &State, name: &str) -> Option<i64> {
        st.schedule_dues().get(name).copied()
    }
    /// "mark" flags every flagged thing every 2 boundaries (the retired DriftSpec
    /// markR, now an engine ScheduleRule the boundary fires).
    fn mark_r() -> ScheduleRule {
        ScheduleRule::new("mark", 2).clause([m("flag.X")], [insert("marked.X")])
    }
    /// Install a schedule on a fresh clocked world, then seed the flag facts;
    /// `set_schedule` start-sates each rule one period out.
    fn scheduled(rules: Vec<ScheduleRule>, outs: &[Outcome]) -> State {
        let mut st = State::new();
        st.set_schedule(rules).unwrap();
        for o in outs {
            st.perform_outcome(o).unwrap();
        }
        st
    }

    // ===== well-formedness (owed:S9 discharged) =====

    // A schedule world is well-formed: the v42 dead-condition lint sees the
    // rule's guard (`flag.X`) fed by a producible fact (`flag.a`, here in the
    // db), so `type_check` finds nothing — empty ⇒ well-formed.
    // H: ScheduleRuleSpec.hs "a schedule world is well-formed"
    #[test]
    fn a_schedule_world_is_well_formed() {
        let st = scheduled(vec![mark_r()], &[insert("flag.a")]);
        assert!(
            prax_core::typecheck::type_check(&st).is_empty(),
            "a schedule world whose rule guard is fed by a producible fact is \
             well-formed, got {:?}",
            prax_core::typecheck::type_check(&st)
        );
    }

    // ===== firing / dues =====

    // H: ScheduleRuleSpec.hs "Prax.ScheduleRule"
    // H: ScheduleRuleSpec.hs "a rule does not fire before its due"
    #[test]
    fn a_rule_does_not_fire_before_its_due() {
        let mut st = scheduled(vec![mark_r()], &[insert("flag.a")]);
        st.round_boundary(); // turn 1; due at 2
        assert!(
            !st.db_has("marked.a"),
            "marked.a absent (due seeded at 2, now 1)"
        );
    }

    // H: ScheduleRuleSpec.hs "a rule fires at its due, covering every binding"
    #[test]
    fn a_rule_fires_at_its_due_covering_every_binding() {
        let mut st = scheduled(vec![mark_r()], &[insert("flag.a"), insert("flag.b")]);
        st.round_boundary();
        st.round_boundary(); // turn 2 = due
        assert!(st.db_has("marked.a"), "marked.a inserted");
        assert!(st.db_has("marked.b"), "marked.b inserted");
    }

    // H: ScheduleRuleSpec.hs "the due re-arms period boundaries from now"
    #[test]
    fn the_due_re_arms_period_boundaries_from_now() {
        let mut st = scheduled(vec![mark_r()], &[insert("flag.a")]);
        st.round_boundary();
        st.round_boundary(); // turn 2: fires, re-arms
        assert_eq!(due_of(&st, "mark"), Some(4));
        st.round_boundary(); // turn 3: not yet due at 4
        assert_eq!(due_of(&st, "mark"), Some(4));
        assert!(
            st.db_has("marked.a"),
            "no re-fire before the new due (mark still stands from turn 2)"
        );
    }

    // H: ScheduleRuleSpec.hs "two rules with different periods fire on their own schedules"
    #[test]
    fn two_rules_with_different_periods_fire_on_their_own_schedules() {
        let p2 = ScheduleRule::new("p2", 2).clause([m("flagA.X")], [insert("markedA.X")]);
        let p3 = ScheduleRule::new("p3", 3).clause([m("flagB.X")], [insert("markedB.X")]);
        let mut st = scheduled(vec![p2, p3], &[insert("flagA.a"), insert("flagB.a")]);

        st.round_boundary(); // turn 1: neither due (seeded at 2 / 3)
        assert_eq!(due_of(&st, "p2"), Some(2));
        assert_eq!(due_of(&st, "p3"), Some(3));
        st.round_boundary(); // turn 2: p2 fires, re-arms to 4; p3 not yet
        assert_eq!(due_of(&st, "p2"), Some(4));
        assert_eq!(due_of(&st, "p3"), Some(3));
        st.round_boundary(); // turn 3: p3 fires, re-arms to 6; p2 not yet
        assert_eq!(due_of(&st, "p2"), Some(4));
        assert_eq!(due_of(&st, "p3"), Some(6));
        st.round_boundary(); // turn 4: p2 fires, re-arms to 6
        assert_eq!(due_of(&st, "p2"), Some(6));
        assert_eq!(due_of(&st, "p3"), Some(6));
        st.round_boundary(); // turn 5: neither due
        assert_eq!(due_of(&st, "p2"), Some(6));
        assert_eq!(due_of(&st, "p3"), Some(6));
        st.round_boundary(); // turn 6: both fire, re-arm to 8 and 9
        assert_eq!(due_of(&st, "p2"), Some(8));
        assert_eq!(due_of(&st, "p3"), Some(9));
        assert!(st.db_has("markedA.a"), "markedA.a present");
        assert!(st.db_has("markedB.a"), "markedB.a present");
    }

    // H: ScheduleRuleSpec.hs "a late fire (clock jumped past the due) re-arms FROM the boundary it fires at"
    #[test]
    fn a_late_fire_re_arms_from_the_boundary_it_fires_at() {
        // The clock-jump idiom: jump the clock forward (the only sanctioned
        // author-free clock write) so the rule is overdue, then one boundary fires
        // it late and re-arms a period from NOW, not the stale due.
        let mut st = scheduled(vec![mark_r()], &[insert("flag.a")]);
        st.perform_outcome(&insert("turn!10")).unwrap(); // overdue (due 2, clock 10)
        st.round_boundary(); // now 11: fires late
        assert!(st.db_has("marked.a"), "fired late");
        assert_eq!(due_of(&st, "mark"), Some(13)); // 11 + period, not 2 + period
    }

    // ===== construction guards =====

    // H: ScheduleRuleSpec.hs "duplicate rule names are a loud construction-time error (one due key each)"
    #[test]
    fn duplicate_rule_names_are_a_loud_construction_time_error() {
        assert!(matches!(
            State::new().set_schedule(vec![ScheduleRule::new("same", 2), ScheduleRule::new("same", 3)]),
            Err(WorldError::DuplicateScheduleRuleName { .. })
        ));
    }

    // H: ScheduleRuleSpec.hs "a multi-segment rule name is a loud construction-time error"
    #[test]
    fn a_multi_segment_rule_name_is_a_loud_construction_time_error() {
        assert!(matches!(
            State::new().set_schedule(vec![ScheduleRule::new("a.b", 2)]),
            Err(WorldError::MultiSegmentRuleName { .. })
        ));
    }

    #[test]
    fn a_trailing_operator_in_a_rule_name_dies_as_the_frozen_tokens_error() {
        // No frozen spec label: the frozen guard is
        // `filter ((/= 1) . length . pathNames . srName)` (Engine.hs:291) and
        // `pathNames` RAISES before the length is taken. Observed on the frozen
        // engine at this input:
        //   setSchedule name "tick." -> ERROR ->
        //   Prax.Db.tokens: trailing operator '.' in "tick." -- a sentence ends
        //   in a name, not an operator
        // so the malformed name is a MALFORMED SENTENCE, not a multi-segment one.
        assert_eq!(
            State::new().set_schedule(vec![ScheduleRule::new("tick.", 2)]),
            Err(WorldError::TrailingOperator {
                sentence: "tick.".to_owned(),
                op: '.',
            })
        );
    }

    #[test]
    fn a_trailing_operator_in_a_rule_body_dies_before_the_hygiene_verdict() {
        // Frozen, same probe:
        //   setSchedule body Match "a.b." -> ERROR ->
        //   Prax.Db.tokens: trailing operator '.' in "a.b." ...
        // The v40 splice check walks the body through `conditionVars`, which is
        // `pathNames` on every Match/Not — so a malformed sentence dies at the
        // door rather than being split into `["a", "b", ""]` and waved through.
        // This is an END-TO-END statement of the error identity, not an isolation
        // of the walk: with the walk unguarded the SAME error still arrives one
        // step later from `path::tokenize` when the clause is compiled. The pin
        // that isolates the walk is
        // `types::tests::authored_var_clash_flags_prax_and_forbidden_only`, whose
        // last assertion reddens under exactly that mutation.
        assert_eq!(
            State::new().set_schedule(vec![
                ScheduleRule::new("x", 2).clause([m("a.b.")], Vec::<Outcome>::new())
            ]),
            Err(WorldError::TrailingOperator {
                sentence: "a.b.".to_owned(),
                op: '.',
            })
        );
    }

    // H: ScheduleRuleSpec.hs "a body authoring the Prax namespace is a loud error"
    #[test]
    fn a_body_authoring_the_prax_namespace_is_a_loud_error() {
        assert!(matches!(
            State::new().set_schedule(vec![
                ScheduleRule::new("x", 2).clause([m("flag.PraxNow")], Vec::<Outcome>::new())
            ]),
            Err(WorldError::ReservedVarClash { .. })
        ));
    }

    // H: ScheduleRuleSpec.hs "a body authoring Actor is a loud error (a schedule rule has no actor)"
    #[test]
    fn a_body_authoring_actor_is_a_loud_error() {
        assert!(matches!(
            State::new().set_schedule(vec![
                ScheduleRule::new("x", 2).clause([m("flag.Actor")], Vec::<Outcome>::new())
            ]),
            Err(WorldError::ReservedVarClash { .. })
        ));
    }

    // H: ScheduleRuleSpec.hs "the usability win: D/D2/Now are ordinary variables, not reserved"
    #[test]
    fn the_usability_win_d_d2_now_are_ordinary_variables() {
        let mut st = State::new();
        assert!(
            st.set_schedule(vec![
                ScheduleRule::new("x", 2).clause([m("flag.Now")], [insert("marked.D")])
            ])
            .is_ok(),
            "D and Now are unremarkable author variables"
        );
    }

    // H: ScheduleRuleSpec.hs "a zero period is a loud error"
    #[test]
    fn a_zero_period_is_a_loud_error() {
        assert!(matches!(
            State::new().set_schedule(vec![
                ScheduleRule::new("x", 0).clause([m("flag.a")], [insert("marked.a")])
            ]),
            Err(WorldError::NonPositivePeriod { .. })
        ));
    }

    // ===== gathering (open fires; the fact expires -- no close rule) =====

    // H: ScheduleRuleSpec.hs "gathering (open fires; the fact expires -- no close rule)"
    // H: ScheduleRuleSpec.hs "opens at period, not before"
    #[test]
    fn gathering_opens_at_period_not_before() {
        let mut st = scheduled(
            vec![gathering("fair", 3, 1, vec![insert("marketDay.now")]).unwrap()],
            &[],
        );
        st.round_boundary();
        st.round_boundary(); // turn 2
        assert!(
            !st.db_has("marketDay.now"),
            "not open before period"
        );
        st.round_boundary(); // turn 3 == period
        assert!(st.db_has("marketDay.now"), "opens exactly at period");
    }

    // H: ScheduleRuleSpec.hs "closes at period + duration (the expiry queue tears it down)"
    #[test]
    fn gathering_closes_at_period_plus_duration() {
        let mut st = scheduled(
            vec![gathering("fair", 3, 1, vec![insert("marketDay.now")]).unwrap()],
            &[],
        );
        for _ in 0..3 {
            st.round_boundary(); // turn 3: open
        }
        assert!(st.db_has("marketDay.now"), "still open at period");
        st.round_boundary(); // turn 4 == period + duration
        assert!(
            !st.db_has("marketDay.now"),
            "closed exactly at period + duration"
        );
    }

    // H: ScheduleRuleSpec.hs "recurs: opens again a full period later"
    #[test]
    fn gathering_recurs_a_full_period_later() {
        let mut st = scheduled(
            vec![gathering("fair", 3, 1, vec![insert("marketDay.now")]).unwrap()],
            &[],
        );
        for _ in 0..6 {
            st.round_boundary(); // second open at 2*period
        }
        assert!(
            st.db_has("marketDay.now"),
            "cycle 2 opens at 2 x period"
        );
        assert_eq!(due_of(&st, "fair"), Some(9)); // re-armed to 6 + period
    }

    // H: ScheduleRuleSpec.hs "duration == period is a loud construction-time error"
    #[test]
    fn gathering_duration_equals_period_is_a_loud_error() {
        assert!(matches!(
            gathering("fair", 3, 3, vec![insert("x")]),
            Err(WorldError::GatheringDuration { .. })
        ));
    }

    // H: ScheduleRuleSpec.hs "duration == 0 is a loud construction-time error"
    #[test]
    fn gathering_duration_zero_is_a_loud_error() {
        assert!(matches!(
            gathering("fair", 3, 0, vec![insert("x")]),
            Err(WorldError::GatheringDuration { .. })
        ));
    }

    // ===== the two-door rule-name table (v46) + provenance (v53) =====

    // H: ScheduleRuleSpec.hs "the compiler-level door shares the global rule-name table (v46)"
    // H: ScheduleRuleSpec.hs "registerEngineRules seeds a due exactly like setSchedule (one period out)"
    #[test]
    fn register_engine_rules_seeds_a_due_like_set_schedule() {
        // emptyState clocks turn!0, so a period-1 engine rule is due at 1.
        let mut st = State::new();
        door::register_engine_rules(&mut st, vec![ScheduleRule::new("story", 1)]).unwrap();
        assert_eq!(due_of(&st, "story"), Some(1));
    }

    // H: ScheduleRuleSpec.hs "an authored 'story' rule blocks the engine door, and vice versa"
    #[test]
    fn an_authored_story_rule_blocks_the_engine_door_and_vice_versa() {
        // Direction 1: authored first, engine door second.
        let mut authored = State::new();
        authored.set_schedule(vec![ScheduleRule::new("story", 2)]).unwrap();
        assert!(
            matches!(
                door::register_engine_rules(&mut authored, vec![ScheduleRule::new("story", 1)]),
                Err(WorldError::DuplicateScheduleRuleName { .. })
            ),
            "engine door rejected: 'story' already authored"
        );
        // Direction 2: engine door first, authored second.
        let mut engine_first = State::new();
        door::register_engine_rules(&mut engine_first, vec![ScheduleRule::new("story", 1)]).unwrap();
        assert!(
            matches!(
                engine_first.set_schedule(vec![ScheduleRule::new("story", 2)]),
                Err(WorldError::DuplicateScheduleRuleName { .. })
            ),
            "authoring door rejected: 'story' already registered by the engine"
        );
    }

    // H: ScheduleRuleSpec.hs "engine-rule provenance exempts the reserved-family scan (v53)"
    // H: ScheduleRuleSpec.hs "registerEngineRules records the rule name; setSchedule does not"
    #[test]
    fn register_engine_rules_records_the_rule_name_set_schedule_does_not() {
        let mut a = State::new();
        door::register_engine_rules(&mut a, vec![ScheduleRule::new("story", 1)]).unwrap();
        assert_eq!(a.engine_rule_names(), ["story"]);
        let mut b = State::new();
        b.set_schedule(vec![ScheduleRule::new("auth", 1)]).unwrap();
        assert!(b.engine_rule_names().is_empty());
    }

    // H: ScheduleRuleSpec.hs "adding an authored 'story' rule to a compiled script world is a loud collision"
    //
    // The owed:S8 discharge. The two doors write ONE globally-keyed rule table
    // (the dues map is keyed by name), so a compiled SCRIPT world — which
    // pre-registers `story` through the compiler door — refuses an authored rule
    // of the same name. Only this direction is reachable for a compiled world;
    // the converse and the bare-state case are pinned above at S5.
    //
    // The pin's HOME stays here, at the frozen label's own spec file, rather
    // than moving to the script suite: a `// H:` label is a claim about
    // provenance, and this label is `ScheduleRuleSpec`'s.
    #[test]
    fn adding_an_authored_story_rule_to_a_compiled_script_world_is_a_loud_collision() {
        let mut play = prax_worlds::play::play_world();
        assert_eq!(
            play.engine_rule_names(),
            ["story"],
            "precondition: the compiled world already holds the engine `story` rule"
        );
        assert!(matches!(
            play.set_schedule(vec![ScheduleRule::new("story", 2)]),
            Err(WorldError::DuplicateScheduleRuleName { name }) if name == "story"
        ));
    }

    // H: ScheduleRuleSpec.hs "a duplicate name through the engine door alone still errors loudly (the record-update forces the guard)"
    #[test]
    fn a_duplicate_name_through_the_engine_door_alone_still_errors_loudly() {
        // The Rust records names only AFTER add_schedule_rules' guard passes (see
        // `door::register_engine_rules`), so a duplicate is never silently exempted:
        // the loud error fires before any name is recorded (the Haskell laziness
        // concern is structurally absent).
        let mut st = State::new();
        assert!(matches!(
            door::register_engine_rules(
                &mut st,
                vec![ScheduleRule::new("dup", 1), ScheduleRule::new("dup", 1)]
            ),
            Err(WorldError::DuplicateScheduleRuleName { .. })
        ));
    }
}
