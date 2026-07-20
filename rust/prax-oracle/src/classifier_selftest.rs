//! THE CLASSIFIER'S OWN CORRECTNESS NET.
//!
//! The classifier is the stage's real design work, and it is the one component
//! that cannot be checked by running the engines: if it mislabels, the run still
//! goes green and an implementer is sent to innocent code. So it is netted the
//! only way it can be — against SYNTHETIC record pairs built to sit on exactly
//! one rung each, including the rungs that only fire when something is wrong
//! with the comparator itself.
//!
//! Every rung of the ladder appears here, plus the four hazards the panels
//! named: the walkSeed-length rule, the mode parameterisation [D-C2], the
//! view-mode reclassification, and the [S-I2] shape precedence. `cargo test -p
//! prax-oracle -- --nocapture` prints each verdict.

use crate::classify::{Class, Ctx, Shape, ViewWitness, Walk, classify, render};
use crate::diff::diff_records;
use serde_json::{Value, json};

/// The localizer's [§1.3(a)] finding, as the classifier receives it: an earlier
/// record whose closed views differ while the base dbs there agree.
fn witness(ordinal: usize) -> ViewWitness {
    ViewWitness {
        ordinal,
        diff: vec!["field `view`:".to_owned(), "  only_frozen: resents.dave.alice".to_owned()],
    }
}

fn ctx(walk: Walk) -> Ctx {
    Ctx {
        walk,
        shape: Shape::Green("selftest".to_owned()),
        view_differs_earlier: None,
    }
}

/// Classify a pair and print the verdict (so the self-test's output IS the
/// evidence), returning the class.
fn verdict(name: &str, c: &Ctx, a: &Value, b: &Value) -> Class {
    let d = diff_records(a, b);
    assert!(!d.is_empty(), "{name}: the synthetic pair does not differ");
    let v = classify(c, &d, a, b).unwrap_or_else(|e| panic!("{name}: classify refused: {e}"));
    println!("[{name}]");
    for l in render(&v) {
        println!("  {l}");
    }
    v.class
}

/// A minimal agreeing record; each test perturbs exactly one rung's fields.
fn base() -> Value {
    json!({
        "t": 3, "boundary": false, "actor": "vera", "action": "vera: brag", "idle": false,
        "cursor": 0, "rng": 16807, "dues": {"dust": 4}, "expiries": {},
        "candidates": ["vera: brag", "vera: wait about"],
        "facts": ["bragged.vera", "char.vera"]
    })
}

fn with(v: Value, k: &str, x: Value) -> Value {
    let mut m = v.as_object().expect("object").clone();
    m.insert(k.to_owned(), x);
    Value::Object(m)
}

#[test]
fn termination_one_stream_ended_and_the_other_did_not() {
    let a = json!({"end": true, "reason": "deadend", "ending": null, "passes": 4, "records": 12});
    let b = base();
    assert_eq!(
        verdict("TERMINATION / short stream", &ctx(Walk::Randtrace), &a, &b),
        Class::Termination
    );
}

#[test]
fn termination_the_two_stop_rules_disagree() {
    let a = json!({"end": true, "reason": "deadend", "ending": null, "passes": 4, "records": 12});
    let b = json!({"end": true, "reason": "cap", "ending": null, "passes": 4, "records": 12});
    assert_eq!(
        verdict("TERMINATION / stop rule", &ctx(Walk::Randtrace), &a, &b),
        Class::Termination
    );
}

#[test]
fn turn_advance_disagreed_about_whose_turn_it_is() {
    // [S-C1]: actor/cursor/idle/t were EMITTED and UNCLASSIFIED, while `advance`
    // (cursor arithmetic, the `i <= cursor` wrap, aliveness, post-boundary
    // re-selection) is a distinct bug site.
    let a = base();
    let b = with(with(base(), "actor", json!("otto")), "cursor", json!(1));
    assert_eq!(
        verdict("TURN", &ctx(Walk::Trace), &a, &b),
        Class::Turn
    );
}

#[test]
fn enumeration_same_set_different_order() {
    // [D-C1]/[S-C2]: this is the pair the removed `sort` made invisible. The
    // sets are equal; only the ORDER differs.
    let a = base();
    let b = with(
        base(),
        "candidates",
        json!(["vera: wait about", "vera: brag"]),
    );
    assert_eq!(
        verdict("ENUMERATION / order only", &ctx(Walk::Randtrace), &a, &b),
        Class::Enumeration
    );
}

#[test]
fn enumeration_beats_rng_when_the_candidate_lengths_differ() {
    // The walkSeed hazard [§1.3(b)]: the pick index depends on len(acts), so a
    // differing walkSeed with a differing LENGTH is a symptom, never RNG.
    let a = with(base(), "walkSeed", json!(111));
    let b = with(
        with(base(), "walkSeed", json!(222)),
        "candidates",
        json!(["vera: brag"]),
    );
    let d = diff_records(&a, &b);
    let v = classify(&ctx(Walk::Randtrace), &d, &a, &b).expect("classifies");
    println!("[ENUMERATION beats RNG on differing lengths]");
    for l in render(&v) {
        println!("  {l}");
    }
    assert_eq!(v.class, Class::Enumeration);
    assert!(
        v.pointer.contains("LENGTHS differ"),
        "the report must name the length rule: {}",
        v.pointer
    );
    assert!(
        v.other_fields.contains(&"walkSeed".to_owned()),
        "walkSeed must still be reported as also-differing (the class is triage)"
    );
}

#[test]
fn enumeration_is_not_reportable_without_a_green_worldshape() {
    // [S-I2]: the precedence is a RULE, not a parenthetical. A world-port error
    // presents exactly like an enumeration bug.
    let a = base();
    let b = with(base(), "candidates", json!(["vera: brag"]));
    let d = diff_records(&a, &b);
    let c = Ctx {
        walk: Walk::Randtrace,
        shape: Shape::NotChecked,
        view_differs_earlier: None,
    };
    let e = classify(&c, &d, &a, &b).expect_err("must refuse");
    println!("[S-I2 shape precedence] refused: {e}");
    assert!(e.contains("worldshape"));
}

#[test]
fn decision_in_trace_mode_points_at_the_planner() {
    let a = base();
    let b = with(base(), "action", json!("vera: wait about"));
    let d = diff_records(&a, &b);
    let v = classify(&ctx(Walk::Trace), &d, &a, &b).expect("classifies");
    println!("[DECISION / trace]");
    for l in render(&v) {
        println!("  {l}");
    }
    assert_eq!(v.class, Class::Decision);
    assert!(v.pointer.contains("planner"), "{}", v.pointer);
}

#[test]
fn decision_in_randtrace_mode_points_at_enumeration_order_and_pick() {
    // [D-C2]: `randWalk` never touches Prax.Planner. Pointing a reader at fold
    // association here would send them to machinery that never ran.
    let a = base();
    let b = with(base(), "action", json!("vera: wait about"));
    let d = diff_records(&a, &b);
    let v = classify(&ctx(Walk::Randtrace), &d, &a, &b).expect("classifies");
    println!("[DECISION / randtrace]");
    for l in render(&v) {
        println!("  {l}");
    }
    assert_eq!(v.class, Class::Decision);
    assert!(!v.pointer.contains("fold association"), "{}", v.pointer);
    assert!(v.pointer.contains("pick"), "{}", v.pointer);
}

#[test]
fn rng_the_engine_stream_moved_differently() {
    let a = base();
    let b = with(base(), "rng", json!(282475249));
    assert_eq!(verdict("RNG / engine die", &ctx(Walk::Trace), &a, &b), Class::Rng);
}

#[test]
fn rng_the_draw_log_reaches_taken_versus_not() {
    // [S-C5]: `CRoll` advances the stream UNCONDITIONALLY, so `rng` is EQUAL
    // here and only the log can see that one side took the draw.
    let taken = json!([{"i": 1, "odds": [[1, 2]], "before": 7, "after": 117649,
                        "values": [117649], "changed": true}]);
    let missed = json!([{"i": 1, "odds": [[1, 2]], "before": 7, "after": 117649,
                         "values": [117649], "changed": false}]);
    let a = with(base(), "draws", taken);
    let b = with(base(), "draws", missed);
    let d = diff_records(&a, &b);
    assert!(!d.has("rng"), "the stream position must be EQUAL in this pair");
    let v = classify(&ctx(Walk::Trace), &d, &a, &b).expect("classifies");
    println!("[RNG / draw log, equal rng]");
    for l in render(&v) {
        println!("  {l}");
    }
    assert_eq!(v.class, Class::Rng);
    assert!(v.pointer.contains("taken"), "{}", v.pointer);
}

#[test]
fn rng_the_walk_generator_is_named_separately_from_the_die() {
    let a = with(base(), "walkSeed", json!(111));
    let b = with(base(), "walkSeed", json!(222));
    let d = diff_records(&a, &b);
    let v = classify(&ctx(Walk::Randtrace), &d, &a, &b).expect("classifies");
    println!("[RNG / walk generator, equal candidates]");
    for l in render(&v) {
        println!("  {l}");
    }
    assert_eq!(v.class, Class::Rng);
    assert!(v.pointer.contains("MMIX"), "{}", v.pointer);
}

#[test]
fn schedule_the_boundary_bookkeeping_differs() {
    let a = base();
    let b = with(base(), "dues", json!({"dust": 6}));
    assert_eq!(
        verdict("SCHEDULE", &ctx(Walk::Trace), &a, &b),
        Class::Schedule
    );
}

#[test]
fn schedule_the_boundary_log_reaches_an_expiry_that_fired_wrong() {
    // [S-C5]: an expiry firing on the wrong subtree leaves the `expiries` MAP
    // equal — the pointer has to come from what FIRED, not from what remains.
    let fired = json!({"now": 4, "due_rules": ["dust"],
        "due_expiries": [{"path": "fresh.vera.otto", "due": 4,
                          "existed_before": true, "present_after": false}]});
    let missed = json!({"now": 4, "due_rules": ["dust"],
        "due_expiries": [{"path": "fresh.vera.otto", "due": 4,
                          "existed_before": true, "present_after": true}]});
    let a = with(base(), "boundary_log", fired);
    let b = with(base(), "boundary_log", missed);
    let d = diff_records(&a, &b);
    assert!(!d.has("expiries"), "the expiry MAP must be equal in this pair");
    assert_eq!(
        verdict("SCHEDULE / boundary log, equal maps", &ctx(Walk::Trace), &a, &b),
        Class::Schedule
    );
}

#[test]
fn state_the_facts_differ_and_nothing_above_does() {
    let a = base();
    let b = with(base(), "facts", json!(["char.vera"]));
    assert_eq!(verdict("STATE", &ctx(Walk::Trace), &a, &b), Class::State);
}

#[test]
fn state_view_reclassifies_when_the_view_diverged_at_an_earlier_record() {
    // [§1.3(a)] the DIV-1 shape: a view-only divergence is invisible in `state`
    // mode and surfaces a turn later. The localizer's `--mode view` rerun is
    // what sets this flag; the classifier's job is to honour it — and to name
    // the record the reader should open.
    let a = base();
    let b = with(base(), "facts", json!(["char.vera"]));
    let d = diff_records(&a, &b);
    let c = Ctx {
        walk: Walk::Trace,
        shape: Shape::Green("selftest".to_owned()),
        view_differs_earlier: Some(witness(2)),
    };
    let v = classify(&c, &d, &a, &b).expect("classifies");
    println!("[STATE(view) / reclassified]");
    for l in render(&v) {
        println!("  {l}");
    }
    assert_eq!(v.class, Class::StateView);
    assert!(v.pointer.contains("ordinal 2"), "{}", v.pointer);
}

#[test]
fn idle_is_a_decision_field_not_a_turn_field() {
    // [C1](a). `idle` is `acted.is_none()` — the turn produced no move. That is
    // downstream of enumeration and planning, and it is the same observation
    // `action` already carries. Claimed by TURN, a closure bug that removes the
    // actor's only candidate reports as an `advance` bug and sends the reader to
    // cursor arithmetic. Here candidates are EQUAL, so the pair is a clean
    // DECISION.
    let a = with(base(), "idle", json!(false));
    let b = with(with(base(), "idle", json!(true)), "action", json!("-"));
    let v = verdict("DECISION / idle", &ctx(Walk::Trace), &a, &b);
    assert_eq!(
        v,
        Class::Decision,
        "`idle` is not an `advance` output and must never carry a pair to TURN"
    );
}

#[test]
fn the_view_reclassification_outranks_turn_enumeration_and_decision() {
    // [C1](b). §1.3(a)'s stated hazard is that a view divergence surfaces a turn
    // later AS ENUMERATION/DECISION — and in randtrace, where `t` does not
    // advance on an idle pass [M2], as TURN. All three outrank STATE, so a rule
    // consulted from inside the STATE rung is inoperative for every class it was
    // written to correct. This pair perturbs all three at once.
    let a = base();
    let mut b = base();
    for (k, x) in [
        ("t", json!(4)),
        ("candidates", json!(["vera: brag"])),
        ("action", json!("vera: wait about")),
        ("idle", json!(true)),
    ] {
        b = with(b, k, x);
    }
    let d = diff_records(&a, &b);
    let c = Ctx {
        walk: Walk::Randtrace,
        shape: Shape::Green("selftest".to_owned()),
        view_differs_earlier: Some(witness(1)),
    };
    let v = classify(&c, &d, &a, &b).expect("classifies");
    println!("[STATE(view) / above the ladder]");
    for l in render(&v) {
        println!("  {l}");
    }
    assert_eq!(
        v.class,
        Class::StateView,
        "an earlier view-only divergence outranks TURN, ENUMERATION and DECISION"
    );
    assert!(v.pointer.contains("ordinal 1"), "{}", v.pointer);
}

#[test]
fn termination_still_outranks_the_view_reclassification() {
    // The view rule sits BELOW TERMINATION: comparing a terminal record against a
    // turn record makes every other field difference an artifact of comparing
    // unlike records, whatever the derivation did earlier.
    let a = json!({"end": true, "reason": "deadend", "ending": null, "passes": 4, "records": 12});
    let b = base();
    let d = diff_records(&a, &b);
    let c = Ctx {
        walk: Walk::Randtrace,
        shape: Shape::Green("selftest".to_owned()),
        view_differs_earlier: Some(witness(1)),
    };
    let v = classify(&c, &d, &a, &b).expect("classifies");
    println!("[TERMINATION above STATE(view)]");
    for l in render(&v) {
        println!("  {l}");
    }
    assert_eq!(v.class, Class::Termination);
}

#[test]
fn decision_a_scoring_bug_that_does_not_move_the_argmax() {
    // [C2]. `scores` is the planner's OWN evidence, and a scoring bug that leaves
    // the argmax where it was has an EQUAL `action`. Before this rung claimed
    // `scores`, this pair — the exact one `explain` is run to produce — reported
    // UNCLASSIFIED and sent the reader to classify.rs.
    let row = |bits: u64| json!([{"depth": 0, "rows": [{"label": "vera: brag", "bits": bits}]}]);
    let a = with(base(), "scores", row(4_607_182_418_800_017_408));
    let b = with(base(), "scores", row(4_611_686_018_427_387_904));
    let v = verdict("DECISION / scores, action equal", &ctx(Walk::Trace), &a, &b);
    assert_eq!(v, Class::Decision);
}

#[test]
fn decision_the_intention_differs_while_the_action_agrees() {
    // [M4]'s tell, made classifiable: the DECISION pointer advertises "if the
    // score tables are identical and the action still differs, it is the
    // INTENTION" — so the intention has to sit on the rung that says so.
    let a = with(base(), "intention_after", json!({"label": "vera: brag", "left": 2}));
    let b = with(base(), "intention_after", json!({"label": "vera: brag", "left": 1}));
    assert_eq!(
        verdict("DECISION / intention_after", &ctx(Walk::Trace), &a, &b),
        Class::Decision
    );
}

#[test]
fn termination_two_records_differing_only_in_passes() {
    // [C2]. `passes` is the counter the `passes > living` dead-end rule reads.
    // It rides every randtrace TURN record as well as the terminal one, so the
    // rung cannot be gated on the record being terminal.
    let a = json!({"end": true, "reason": "cap", "ending": null, "passes": 4, "records": 12});
    let b = json!({"end": true, "reason": "cap", "ending": null, "passes": 3, "records": 12});
    assert_eq!(
        verdict("TERMINATION / passes, terminal", &ctx(Walk::Randtrace), &a, &b),
        Class::Termination
    );
    let c = with(base(), "passes", json!(2));
    let e = with(base(), "passes", json!(1));
    assert_eq!(
        verdict("TERMINATION / passes, turn record", &ctx(Walk::Randtrace), &c, &e),
        Class::Termination
    );
}

#[test]
fn state_view_fires_at_its_own_record_without_the_previous_turn_flag() {
    // [I1]. `view` differing while `facts` agrees is a DERIVATION divergence
    // HERE — axiom heads, defeater names, closure completeness. Requiring the
    // t−1 flag handled the DIV-1 shape one record late and pointed the reader at
    // perform semantics, spawn and the ForEach snapshot for a closure bug.
    let a = with(base(), "view", json!(["char.vera", "trusts.vera.otto"]));
    let b = with(base(), "view", json!(["char.vera"]));
    let d = diff_records(&a, &b);
    let v = classify(&ctx(Walk::Trace), &d, &a, &b).expect("classifies");
    println!("[STATE(view) / at its own record]");
    for l in render(&v) {
        println!("  {l}");
    }
    assert_eq!(v.class, Class::StateView);
    assert!(v.pointer.contains("base dbs AGREE"), "{}", v.pointer);
    // …and `facts` ALSO differing is a base-db divergence, not a view one.
    let b2 = with(b, "facts", json!(["char.vera"]));
    assert_eq!(
        verdict("STATE / facts and view both", &ctx(Walk::Trace), &a, &b2),
        Class::State
    );
}

#[test]
fn unclassified_fails_loud_rather_than_mislabelling() {
    // [S-C1]'s terminal class. A field added to the emission without being added
    // to the ladder must report as a COMPARATOR bug — not be folded into STATE,
    // where it would send an implementer to innocent perform semantics.
    let a = with(base(), "someNewField", json!(1));
    let b = with(base(), "someNewField", json!(2));
    let d = diff_records(&a, &b);
    let v = classify(&ctx(Walk::Trace), &d, &a, &b).expect("classifies");
    println!("[UNCLASSIFIED]");
    for l in render(&v) {
        println!("  {l}");
    }
    assert_eq!(v.class, Class::Unclassified);
    assert!(
        v.pointer.contains("THE COMPARATOR ITSELF"),
        "{}",
        v.pointer
    );
}

#[test]
fn the_ladder_precedence_holds_when_several_rungs_fire_at_once() {
    // Every rung's fields perturbed at once: the HIGHEST must win, and every
    // other differing field must still be reported (the class is triage).
    let a = base();
    let mut b = base();
    for (k, v) in [
        ("actor", json!("otto")),
        ("candidates", json!(["x"])),
        ("action", json!("otto: wait about")),
        ("rng", json!(9)),
        ("dues", json!({"dust": 9})),
        ("facts", json!([])),
    ] {
        b = with(b, k, v);
    }
    let d = diff_records(&a, &b);
    let v = classify(&ctx(Walk::Trace), &d, &a, &b).expect("classifies");
    println!("[ladder precedence / all rungs at once]");
    for l in render(&v) {
        println!("  {l}");
    }
    assert_eq!(v.class, Class::Turn, "TURN outranks everything below it");
    for f in ["candidates", "action", "rng", "dues", "facts"] {
        assert!(
            v.other_fields.contains(&f.to_owned()),
            "the report must still name `{f}` as also-differing"
        );
    }
}

#[test]
fn the_view_window_scans_back_past_an_idle_pass_instead_of_indexing_t_minus_one() {
    // [C1](c). The two units are NOT the same window. A derivation divergence
    // shows in the closed view at record 1; the base dbs stay equal through an
    // IDLE pass at record 2 (where the acting character's view carries none of
    // the affected facts, so `view` agrees there); the state-mode divergence
    // finally surfaces at record 3. Indexing `ordinal - 1` looks only at record
    // 2, finds nothing, and the reclassification never fires — which is exactly
    // what suppressed it on the reviewer's injected closure bug.
    let rec = |view: Value| {
        json!({"t": 1, "actor": "vera", "facts": ["char.vera"], "view": view})
    };
    let agree = || rec(json!(["char.vera"]));
    let frozen = vec![
        json!({"header": true}),
        rec(json!(["char.vera", "resents.dave.alice"])),
        agree(),
        agree(),
    ];
    let rust = vec![json!({"header": true}), agree(), agree(), agree()];

    let w = crate::first_view_divergence(&frozen, &rust, 3)
        .expect("the scan must reach back past the idle pass to record 1");
    assert_eq!(w.ordinal, 1, "the EARLIEST view divergence is the anchor");
    assert!(
        w.diff.iter().any(|l| l.contains("resents.dave.alice")),
        "the witness must NAME the derived fact that went missing: {:?}",
        w.diff
    );

    // The one-record window this replaces: record 2 alone shows nothing.
    assert!(
        crate::first_view_divergence(&frozen, &rust, 2).is_some(),
        "record 1 is still in the window when the divergence is at 2"
    );
    assert!(
        crate::first_view_divergence(&frozen, &rust, 1).is_none(),
        "there is no record before the first one"
    );
}

#[test]
fn a_base_db_divergence_before_the_ordinal_defeats_the_view_window() {
    // The rule's precondition is that the base dbs AGREE where the views differ.
    // If `facts` already disagreed earlier, this is not a derivation divergence
    // and the ladder below must judge it on its own evidence.
    let frozen = vec![
        json!({"header": true}),
        json!({"facts": ["char.vera", "bragged.vera"], "view": ["char.vera"]}),
        json!({"facts": ["char.vera"], "view": ["char.vera", "resents.dave.alice"]}),
    ];
    let rust = vec![
        json!({"header": true}),
        json!({"facts": ["char.vera"], "view": ["char.vera"]}),
        json!({"facts": ["char.vera"], "view": ["char.vera"]}),
    ];
    assert!(
        crate::first_view_divergence(&frozen, &rust, 3).is_none(),
        "a base-db divergence at record 1 is not a view-only divergence"
    );
}

#[test]
fn classifying_an_agreement_is_a_caller_bug() {
    let a = base();
    let d = diff_records(&a, &a);
    assert!(classify(&ctx(Walk::Trace), &d, &a, &a).is_err());
}
