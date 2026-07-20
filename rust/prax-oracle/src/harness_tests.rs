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
    let o = run_one(&spec(Walk::Trace, 16, None), &reg).expect("the run completes").outcome;
    println!("trace: {}", o.cell());
    assert!(
        matches!(o, Outcome::Clean { .. }),
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
    let o = run_one(&s, &reg).expect("the run completes").outcome;
    println!("trace --localize: {}", o.cell());
    assert!(
        matches!(o, Outcome::Clean { .. }),
        "the localization emission diverged: {:?}",
        render(&o)
    );
}

#[test]
fn the_randtrace_walk_agrees_over_a_seed_sweep() {
    let reg = load_register().expect("the register loads");
    for seed in 0..8 {
        let o = run_one(&spec(Walk::Randtrace, 25, Some(seed)), &reg)
            .expect("the run completes")
            .outcome;
        println!("randtrace seed {seed}: {}", o.cell());
        assert!(
            matches!(o, Outcome::Clean { .. }),
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
    let o = run_one(&s, &reg).expect("the run completes").outcome;
    println!("randtrace --emit view: {}", o.cell());
    assert!(matches!(o, Outcome::Clean { .. }), "{:?}", render(&o));
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
        let o = run_one(&s, &reg).expect("the run completes").outcome;
        println!("randtrace --die-seed {die}: {}", o.cell());
        assert!(matches!(o, Outcome::Clean { .. }), "{:?}", render(&o));
    }
}

#[test]
fn the_zero_setup_rolls_assertion_is_transitive_through_call_on_both_sides() {
    // [M1]. The assertion licenses the setup db's SET comparison [D-I5], and it
    // claimed to stop any world whose setup draws. Both sides stopped at the
    // `Call` boundary, so an init outcome calling a rolling function passed a
    // gate that said it could not. The fixture is that world, transcribed on
    // both sides, and BOTH must refuse it.
    let mut mutated = crate::probe::probe_world_with_a_drawing_setup();
    let rust = crate::worldshape::worldshape("probe-drawing-setup", &mut mutated)
        .expect_err("the Rust gate must refuse a setup that draws through a Call");
    println!("rust  : {rust}");
    assert!(rust.contains("gamble") && rust.contains("Call"), "{rust}");

    let frozen = crate::drive_frozen::run_json(&[
        "worldshape".to_owned(),
        "probe-drawing-setup".to_owned(),
    ])
    .expect_err("the frozen gate must refuse it too");
    println!("frozen: {frozen}");
    assert!(frozen.contains("gamble") && frozen.contains("Call"), "{frozen}");

    // …and the assertion still passes the world that only draws in an ACTION.
    let mut clean = crate::probe::probe_world();
    crate::worldshape::worldshape("probe", &mut clean).expect("the probe world still passes");
}

#[test]
fn a_hoisted_worldshape_is_not_re_run_per_cell() {
    // [I4]. `worldshape` is a property of a WORLD at a freeze rev; matrix mode
    // establishes it once up front (§1.6) and every cell runs behind that
    // verdict. Re-checking per (world, seed) was ~400 redundant frozen asks and
    // ~800 `git` subprocesses at the specified 100 seeds × 4 worlds. The net:
    // three seeds behind a hoisted verdict must cost exactly three frozen asks,
    // one walk each.
    let reg = load_register().expect("the register loads");
    let shape = crate::classify::Shape::Green(
        crate::drive_frozen::freeze_rev().expect("the freeze rev resolves"),
    );
    let before = crate::drive_frozen::frozen_calls();
    for seed in 0..3 {
        let o = crate::run_one_behind(&spec(Walk::Randtrace, 20, Some(seed)), &reg, &shape)
            .expect("the run completes").outcome;
        assert!(matches!(o, Outcome::Clean { .. }), "seed {seed}: {:?}", render(&o));
    }
    let asked = crate::drive_frozen::frozen_calls() - before;
    println!("3 seeds behind a hoisted worldshape: {asked} frozen invocations");
    assert_eq!(
        asked, 3,
        "three seeds cost {asked} frozen asks, not 3 — the worldshape gate is being re-run per \
         cell instead of once per world"
    );
}

#[test]
fn matrix_jobs_do_not_change_the_result_or_its_order() {
    // [I4]'s other half (§1.8). `--jobs N` parallelizes over seeds; the outcomes
    // and their ORDER must not depend on N, because reports embed the matrix
    // output verbatim and a scheduling-dependent line order cannot be diffed
    // between runs.
    let reg = load_register().expect("the register loads");
    let shape = crate::classify::Shape::Green(
        crate::drive_frozen::freeze_rev().expect("the freeze rev resolves"),
    );
    let specs: Vec<crate::RunSpec> = (0..6)
        .map(|s| spec(Walk::Randtrace, 20, Some(s)))
        .collect();
    let serial = crate::matrix::run_seeds(&specs, &reg, &shape, 1).expect("serial");
    let parallel = crate::matrix::run_seeds(&specs, &reg, &shape, 4).expect("parallel");
    let cells = |v: &[crate::Run]| v.iter().map(|r| r.outcome.cell()).collect::<Vec<_>>();
    let walks = |v: &[crate::Run]| v.iter().map(|r| r.walk.clone()).collect::<Vec<_>>();
    println!("jobs 1: {:?}\njobs 4: {:?}", cells(&serial), cells(&parallel));
    assert_eq!(serial.len(), 6, "every seed produced a cell");
    assert_eq!(cells(&serial), cells(&parallel));
    // The walk identities ride the same seed order [I1]: a sweep whose coverage
    // accounting depended on `--jobs` would report a different distinct-walk
    // count on every run.
    assert_eq!(walks(&serial), walks(&parallel));
    assert!(serial.iter().all(|r| matches!(r.outcome, Outcome::Clean { .. })));
}

#[test]
fn the_localization_rerun_carries_the_full_emission_truncated_to_the_ordinal() {
    // [C1]'s two properties, over both engines. `compare`/`matrix` run the
    // matrix emission (candidates only), under which the draw log and the
    // boundary log do not exist — so RNG and SCHEDULE cannot reach their own
    // pointers [S-C5] and an RNG bug arrives labelled STATE. `run_one` reruns
    // through this function on ANY divergence; what it must produce is BOTH
    // sides at the full emission, truncated to the divergent record.
    let ordinal = 6;
    let (frozen, rust) =
        crate::localization_streams(&spec(Walk::Randtrace, 25, Some(3)), ordinal)
            .expect("both sides re-drive");
    assert_eq!(frozen.len(), ordinal + 1, "the frozen side is truncated");
    assert_eq!(rust.len(), ordinal + 1, "the rust side is truncated");
    for (side, stream) in [("frozen", &frozen), ("rust", &rust)] {
        let turn = stream[ordinal].as_object().expect("a record object");
        println!("{side} @ord {ordinal}: {:?}", turn.keys().collect::<Vec<_>>());
        for field in ["candidates", "draws", "identity"] {
            assert!(
                turn.contains_key(field),
                "the {side} localization record is missing `{field}` — the rerun did not turn \
                 the full emission on, and RNG/SCHEDULE cannot reach their pointers without it"
            );
        }
    }
}

#[test]
fn rung_field_sets_cover_every_emitted_key() {
    // [C2] THE LADDER'S TOTALITY, asserted against the EMISSION rather than
    // against a list someone maintained by hand. A field that is emitted and
    // claimed by no rung classifies as UNCLASSIFIED — which tells an implementer
    // to go fix `classify.rs` for what is a genuine engine divergence. That is
    // [S-C1]'s defect, and it came back once already when this slice added
    // `scores`/`intention_before`/`intention_after`/`passes` to the emission.
    // So the coverage is checked by DRIVING the frozen oracle with every
    // emission flag on, over both walks and the richest mode, and taking the
    // union of the keys it actually produces.
    use crate::classify::RUNGS;
    use std::collections::BTreeSet;

    let claimed: BTreeSet<&str> = RUNGS.iter().flat_map(|(_, fs)| fs.iter().copied()).collect();
    let runs: Vec<Vec<String>> = vec![
        crate::RunSpec {
            mode: Mode::View,
            emit: Emit::all(),
            ..spec(Walk::Trace, 16, None)
        }
        .frozen_args(Mode::View),
        crate::RunSpec {
            mode: Mode::View,
            emit: Emit::all(),
            ..spec(Walk::Randtrace, 25, Some(3))
        }
        .frozen_args(Mode::View),
    ];

    let mut emitted: BTreeSet<String> = BTreeSet::new();
    for args in &runs {
        let stream = crate::drive_frozen::run_jsonl(args).expect("the frozen oracle runs");
        for rec in &stream {
            let obj = rec.as_object().expect("every record is an object");
            // The header is compared on its own and reported as SHAPE-DIVERGENT
            // [M1]; it never reaches the classifier.
            if obj.contains_key("format") {
                continue;
            }
            emitted.extend(obj.keys().cloned());
        }
    }
    assert!(
        !emitted.is_empty(),
        "no records were collected — the coverage assertion would be vacuous"
    );
    println!("emitted keys ({}): {emitted:?}", emitted.len());

    let unclaimed: Vec<&String> = emitted.iter().filter(|k| !claimed.contains(k.as_str())).collect();
    assert!(
        unclaimed.is_empty(),
        "the oracle emits {unclaimed:?}, which NO rung of the ladder claims. A record pair \
         differing only in one of them classifies as UNCLASSIFIED and points the reader at \
         classify.rs instead of at the engine. Put each field on the rung whose evidence it is \
         (see classify::RUNGS)."
    );

    // The converse, so the rung sets cannot rot into fiction: every claimed
    // field must be one the oracle can actually emit.
    let phantom: Vec<&&str> = claimed.iter().filter(|k| !emitted.contains(**k)).collect();
    assert!(
        phantom.is_empty(),
        "the ladder claims {phantom:?}, which the oracle never emits over either walk at the \
         full emission — a rung field that describes nothing"
    );
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

// ---- the slice-1 worlds: feud and bigfeud ----------------------------------
//
// The standing net for the FIRST ported content (S7 slice 1). The matrix is run
// by hand at slice close over the full seed budget; these three tests are its
// resident core, so a later change under `prax_core` that breaks the feud port
// fails `cargo test` instead of waiting for the next matrix run. They are kept
// small on purpose — the budget lives in the matrix, the regression net lives
// here.

fn feud_spec(world: &str, walk: Walk, steps: i64, seed: Option<i64>) -> RunSpec {
    RunSpec {
        world: world.to_owned(),
        walk,
        steps,
        seed,
        die_seed: None,
        depth: 2,
        idle: worlds::idler(world).map(str::to_owned),
        mode: Mode::State,
        emit: Emit::matrix(),
    }
}

#[test]
fn worldshape_agrees_on_both_slice_1_worlds() {
    for world in ["feud", "bigfeud"] {
        let (green, lines) = shape_compare(world).expect("both sides emit a worldshape");
        for l in &lines {
            println!("{l}");
        }
        assert!(green, "{world}: the two transcriptions of the world disagree");
    }
}

#[test]
fn the_feud_trace_agrees_record_for_record() {
    let reg = load_register().expect("the register loads");
    for world in ["feud", "bigfeud"] {
        let o = run_one(&feud_spec(world, Walk::Trace, 24, None), &reg)
            .expect("the run completes").outcome;
        println!("{world} trace: {}", o.cell());
        assert!(
            matches!(o, Outcome::Clean { .. }),
            "{world} trace diverged: {:?}",
            render(&o)
        );
    }
}

#[test]
fn the_feud_randtrace_agrees_over_a_seed_sweep() {
    let reg = load_register().expect("the register loads");
    for world in ["feud", "bigfeud"] {
        for seed in 0..4 {
            let o = run_one(&feud_spec(world, Walk::Randtrace, 50, Some(seed)), &reg)
                .expect("the run completes").outcome;
            println!("{world} randtrace seed {seed}: {}", o.cell());
            assert!(
                matches!(o, Outcome::Clean { .. }),
                "{world} seed {seed} diverged: {:?}",
                render(&o)
            );
        }
    }
}

fn render(o: &Outcome) -> Vec<String> {
    match o {
        Outcome::Divergent(d) => crate::compare::render(d, &crate::classify::Shape::NotChecked),
        Outcome::ShapeDivergent { detail, .. } => detail.clone(),
        _ => vec![o.cell().to_owned()],
    }
}

// The standing net for S7 slice 2 (intrigue) — the resident core of the slice's
// matrix, on the same terms as slice 1's: small, fast enough to live in
// `cargo test`, and enough that a later `prax_core` change that breaks the
// intrigue port fails the suite instead of waiting for a hand-run matrix.
//
// Slice 2 is the first slice whose walks can END: `ending.E` freezes the drama
// and stops the randtrace. That stop is the reason the seed sweep here also
// asserts the terminal record's REASON rather than only that the streams agree —
// two walks that both stopped on `cap` would agree about a rule neither ran.

fn intrigue_spec(walk: Walk, steps: i64, seed: Option<i64>) -> RunSpec {
    RunSpec {
        world: "intrigue".to_owned(),
        walk,
        steps,
        seed,
        die_seed: None,
        depth: 2,
        idle: worlds::idler("intrigue").map(str::to_owned),
        mode: Mode::State,
        emit: Emit::matrix(),
    }
}

#[test]
fn worldshape_agrees_on_the_intrigue_world() {
    let (green, lines) = shape_compare("intrigue").expect("both sides emit a worldshape");
    for l in &lines {
        println!("{l}");
    }
    assert!(
        green,
        "intrigue: the two transcriptions of the world disagree"
    );
}

#[test]
fn the_intrigue_trace_agrees_record_for_record() {
    let reg = load_register().expect("the register loads");
    let o = run_one(&intrigue_spec(Walk::Trace, 24, None), &reg).expect("the run completes").outcome;
    println!("intrigue trace: {}", o.cell());
    assert!(
        matches!(o, Outcome::Clean { .. }),
        "intrigue trace diverged: {:?}",
        render(&o)
    );
}

#[test]
fn the_intrigue_randtrace_agrees_over_a_seed_sweep() {
    let reg = load_register().expect("the register loads");
    for seed in 0..4 {
        let o = run_one(&intrigue_spec(Walk::Randtrace, 50, Some(seed)), &reg)
            .expect("the run completes").outcome;
        println!("intrigue randtrace seed {seed}: {}", o.cell());
        assert!(
            matches!(o, Outcome::Clean { .. }),
            "intrigue seed {seed} diverged: {:?}",
            render(&o)
        );
    }
}

/// THE ENDING STOP, observed rather than assumed. `ending.E` is slice 2's new
/// terminal rule; a differential that only checks the two streams for agreement
/// cannot tell "both walks stopped at the ending" from "neither walk ever
/// reached one". So this reads the terminal record on BOTH sides and asserts the
/// reason IS the ending — and, since `stop_record` carries which ending, that
/// they name the same one.
#[test]
fn the_randtrace_walk_stops_at_the_ending_on_both_sides() {
    for seed in 0..4 {
        let spec = intrigue_spec(Walk::Randtrace, 50, Some(seed));
        let frozen = crate::drive_frozen::run_jsonl(&spec.frozen_args(spec.mode))
            .expect("the frozen randtrace");
        let rust = crate::rust_stream(&spec).expect("the rust randtrace");
        let f_end = frozen.last().expect("a terminal record");
        let r_end = rust.last().expect("a terminal record");
        println!("intrigue seed {seed}: frozen {f_end}\n                rust   {r_end}");
        assert_eq!(
            f_end["reason"], "ending",
            "seed {seed}: the frozen walk must stop because an ending was reached, not on cap \
             — otherwise this slice's new stop rule is never exercised"
        );
        assert_eq!(f_end, r_end, "seed {seed}: the terminal records must agree");
        assert!(
            f_end["ending"].is_string(),
            "seed {seed}: the stop record names the ending reached"
        );
    }
}
