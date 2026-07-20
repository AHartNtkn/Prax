//! The harness's END-TO-END net, over the `probe` world.
//!
//! Everything above this file is checked against synthetic pairs. These tests
//! are the other half: both engines actually run, over a world both sides build,
//! and the comparator's own plumbing — freeze-check, the cache, the record
//! builder, the walk transcription, the canonical encoder, the register — is
//! exercised on real streams before any shipped world is ported.
//!
//! They shell out to `cabal run prax-oracle`, so they are slower than the rest
//! of the suite; they are not optional for that. A harness whose only evidence
//! is its own synthetic fixtures has never been differentially tested.

use crate::classify::Walk;
use crate::compare::Outcome;
use crate::record::{Emit, Mode};
use crate::{RunSpec, load_register, run_one, shape_compare, worlds};

fn spec(walk: Walk, steps: i64, seed: Option<i64>) -> RunSpec {
    RunSpec {
        world: "probe".to_owned(),
        walk,
        steps,
        seed,
        die_seed: None,
        depth: 2,
        idle: worlds::idler("probe").map(str::to_owned),
        mode: Mode::State,
        emit: Emit::matrix(),
    }
}

#[test]
fn worldshape_agrees_on_the_probe_world() {
    let (green, lines) = shape_compare("probe").expect("both sides emit a worldshape");
    for l in &lines {
        println!("{l}");
    }
    assert!(green, "the probe world's two transcriptions disagree");
}

#[test]
fn a_mistranscribed_action_label_is_caught_by_worldshape_as_a_one_line_diff() {
    // THE GATE'S MUTATION EVIDENCE (§2). A swapped label is exactly what a world
    // port gets wrong, it is invisible in the world's own tests, and at trace
    // time it presents as an ENUMERATION or DECISION divergence pointing at
    // innocent engine code. Behind the gate it is one structural line.
    let frozen = crate::drive_frozen::run_json(&["worldshape".to_owned(), "probe".to_owned()])
        .expect("the frozen side emits a worldshape");
    let mut mutated = crate::probe::probe_world_with_a_mistranscribed_label();
    let rust = crate::worldshape::worldshape("probe", &mut mutated).expect("emits");
    let d = crate::diff::diff_records(
        frozen.get("shape").expect("shape"),
        rust.get("shape").expect("shape"),
    );
    for l in d.fields.iter().flat_map(crate::diff::render_field) {
        println!("{l}");
    }
    assert!(
        d.has("practices"),
        "the gate must name the practice table: {:?}",
        d.field_names()
    );
    let rendered = format!("{:?}", d.get("practices").expect("practices"));
    assert!(
        rendered.contains("boast") && rendered.contains("brag"),
        "the diff must show both labels: {rendered}"
    );
}

#[test]
fn the_trace_walk_agrees_record_for_record() {
    let reg = load_register().expect("the register loads");
    let (o, _) = run_one(&spec(Walk::Trace, 16, None), &reg).expect("the run completes");
    println!("trace: {}", o.cell());
    assert!(
        matches!(o, Outcome::Clean),
        "the probe world's trace walk diverged: {:?}",
        render(&o)
    );
}

#[test]
fn the_trace_walk_agrees_with_the_full_localization_emission() {
    // The localization fields are the ones a divergence is read through, so they
    // are differentially checked too: candidates in NATIVE order, the score
    // table's raw bits at depths 0..2, the action identity, the draw log and the
    // boundary log.
    let reg = load_register().expect("the register loads");
    let mut s = spec(Walk::Trace, 16, None);
    s.emit = Emit::all();
    let (o, _) = run_one(&s, &reg).expect("the run completes");
    println!("trace --localize: {}", o.cell());
    assert!(
        matches!(o, Outcome::Clean),
        "the localization emission diverged: {:?}",
        render(&o)
    );
}

#[test]
fn the_randtrace_walk_agrees_over_a_seed_sweep() {
    let reg = load_register().expect("the register loads");
    for seed in 0..8 {
        let (o, _) =
            run_one(&spec(Walk::Randtrace, 25, Some(seed)), &reg).expect("the run completes");
        println!("randtrace seed {seed}: {}", o.cell());
        assert!(
            matches!(o, Outcome::Clean),
            "seed {seed} diverged: {:?}",
            render(&o)
        );
    }
}

#[test]
fn the_randtrace_walk_agrees_in_view_mode() {
    // [S-C4]: `randtrace --mode` exists so the view-mode reclassification is
    // available on the walk that carries the bulk of every slice's budget.
    let reg = load_register().expect("the register loads");
    let mut s = spec(Walk::Randtrace, 20, Some(3));
    s.mode = Mode::View;
    let (o, _) = run_one(&s, &reg).expect("the run completes");
    println!("randtrace --emit view: {}", o.cell());
    assert!(matches!(o, Outcome::Clean), "{:?}", render(&o));
}

#[test]
fn the_die_seed_override_agrees() {
    // [D-I6]: the walk seed is not the die seed. Overriding it moves the engine
    // stream under a fixed walk, which is the integration coverage the sweep buys.
    let reg = load_register().expect("the register loads");
    for die in [1, 12345, 2_147_483_646] {
        let mut s = spec(Walk::Randtrace, 20, Some(2));
        s.die_seed = Some(die);
        s.emit = Emit::all();
        let (o, _) = run_one(&s, &reg).expect("the run completes");
        println!("randtrace --die-seed {die}: {}", o.cell());
        assert!(matches!(o, Outcome::Clean), "{:?}", render(&o));
    }
}

#[test]
fn a_world_that_is_not_ported_yet_is_a_loud_error_naming_its_slice() {
    let e = worlds::build("village").expect_err("village is not ported in slice 0");
    println!("{e}");
    assert!(e.contains("slice 4"), "{e}");
}

#[test]
fn the_freeze_check_gates_every_frozen_invocation() {
    // The comparator refuses to produce a record against an edited reference.
    // Here the tree is clean, so the gate passes — the negative direction is
    // covered by `scripts/freeze-check.sh`'s own exit code, which this calls.
    crate::drive_frozen::freeze_check().expect("the frozen tree is clean");
    let rev = crate::drive_frozen::freeze_rev().expect("the freeze rev resolves");
    println!("freeze rev: {rev}");
    assert!(
        rev.contains('-'),
        "the rev must key on BOTH the tag and the oracle source: {rev}"
    );
}

#[test]
fn every_condition_constructor_is_exercised_by_the_probe_world() {
    // The canonical encoder is only checked where the fixture names a
    // constructor. A constructor the probe never names is one the encoder ships
    // unexercised — so the coverage is asserted, not assumed.
    use prax_core::query::Condition;
    use std::collections::BTreeSet;
    let st = crate::probe::probe_world();
    let mut seen: BTreeSet<&'static str> = BTreeSet::new();
    let mut visit = |cs: &[Condition]| {
        for c in cs {
            seen.extend(crate::probe::condition_constructor_names(c));
        }
    };
    for p in st.practice_defs().values() {
        for a in &p.actions {
            visit(&a.when);
        }
    }
    for f in st.functions_src() {
        for c in &f.cases {
            visit(&c.conditions);
        }
    }
    for a in st.axioms_src() {
        visit(&a.when);
    }
    for d in st.desires_src() {
        visit(&d.want.when);
    }
    for c in st.characters() {
        for w in &c.wants {
            visit(&w.when);
        }
    }
    visit(st.prediction_scope());
    let all = [
        "Match", "Not", "Eq", "Neq", "Cmp", "Calc", "Count", "Subquery", "Or", "Absent", "Exists",
    ];
    let missing: Vec<&str> = all.iter().filter(|c| !seen.contains(*c)).copied().collect();
    assert!(
        missing.is_empty(),
        "the probe world never names {missing:?}, so the canonical encoder ships those \
         constructors unexercised"
    );
}

#[test]
fn every_outcome_constructor_is_exercised_by_the_probe_world() {
    use prax_core::types::Outcome as O;
    use std::collections::BTreeSet;
    let st = crate::probe::probe_world();
    let mut seen: BTreeSet<&'static str> = BTreeSet::new();
    fn go(o: &O, seen: &mut BTreeSet<&'static str>) {
        match o {
            O::Insert(_) => {
                seen.insert("Insert");
            }
            O::Delete(_) => {
                seen.insert("Delete");
            }
            O::InsertFor(_, _) => {
                seen.insert("InsertFor");
            }
            O::Call(_, _) => {
                seen.insert("Call");
            }
            O::ForEach(_, os) => {
                seen.insert("ForEach");
                for x in os {
                    go(x, seen);
                }
            }
            O::Roll(_, _, _, os) => {
                seen.insert("Roll");
                for x in os {
                    go(x, seen);
                }
            }
        }
    }
    for p in st.practice_defs().values() {
        for a in &p.actions {
            for o in &a.then {
                go(o, &mut seen);
            }
        }
        for o in &p.init_outcomes {
            go(o, &mut seen);
        }
    }
    for f in st.functions_src() {
        for c in &f.cases {
            for o in &c.outcomes {
                go(o, &mut seen);
            }
        }
    }
    for r in st.schedule_src() {
        for (_, os) in &r.body {
            for o in os {
                go(o, &mut seen);
            }
        }
    }
    let all = ["Insert", "Delete", "InsertFor", "Call", "ForEach", "Roll"];
    let missing: Vec<&str> = all.iter().filter(|c| !seen.contains(*c)).copied().collect();
    assert!(missing.is_empty(), "the probe world never names {missing:?}");
}

fn render(o: &Outcome) -> Vec<String> {
    match o {
        Outcome::Divergent(d) => crate::compare::render(d, &crate::classify::Shape::NotChecked),
        Outcome::ShapeDivergent { detail, .. } => detail.clone(),
        _ => vec![o.cell().to_owned()],
    }
}
