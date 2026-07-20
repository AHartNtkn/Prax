//! `probe` — the harness's own self-test world.
//!
//! Slice 0 ships the differential harness BEFORE any Rust world exists. Without
//! a world both engines can build, every load-bearing piece of it —
//! [`crate::drive_frozen`], [`crate::drive_rust`], [`crate::record`]'s builder,
//! [`crate::walk`]'s transcription of `randWalk`, [`crate::worldshape`]'s
//! encoder and the classifier's behaviour on REAL streams — would ship
//! unexercised, verified only against synthetic pairs the comparator itself
//! made up.
//!
//! `probe` is a HARNESS FIXTURE, not content. It makes no coverage claim about
//! the engine (the four slices' worlds do that), it is excluded from the shipped
//! world list on both sides, and it is not a "synthetic world corpus" in the
//! sense S7 design [D-I6] rejects — nothing here stands in for a pin. Its one
//! job is to be transcribed IDENTICALLY on both sides, so it names every
//! [`Condition`] and every [`Outcome`] constructor at least once: exactly what
//! the canonical encoder and the record builder have to agree on.
//!
//! The mirror of `oracle/TraceMain.hs`'s `probeWorld`. If the two ever drift,
//! `prax-oracle worldshape probe` says so in one structural line — which is the
//! same net every ported world gets in slices 1-4.

use prax_core::engine::State;
use prax_core::query::{
    CalcOp, CmpOp, absent, calc, cmp, count, eq, exists, matches, neq, not_, or_, subquery,
};
use prax_core::rng::draw;
use prax_core::types::{
    Action, Axiom, Character, Desire, Function, Outcome, Practice, ScheduleRule, Want, call, delete,
    for_each, insert, insert_for,
};

/// Build the probe world (the mirror of the oracle's `probeWorld`).
///
/// # Panics
/// If any construction guard rejects it — the world is a fixture, so a guard
/// failure is a bug in this file, not a condition to handle.
pub fn probe_world() -> State {
    let mut st = State::new();
    st.define_practices([greet_practice(), vigil_practice()])
        .expect("probe practices");
    st.define_functions(vec![note_fn()]).expect("probe functions");
    st.set_axioms(probe_axioms()).expect("probe axioms");
    st.set_characters(vec![
        Character::new("vera")
            .want(Want::new(vec![matches("greeted.vera.otto")], 5))
            .holds("curious"),
        Character::new("otto")
            .want(Want::new(vec![matches("bragged.otto")], 3))
            .want(Want::new(vec![matches("impressed.otto.vera")], -2)),
        Character::new("quill").bound_to("vigil"),
    ])
    .expect("probe characters");
    st.set_desires(vec![Desire::new(
        "curious",
        Want::new(vec![matches("tallied.Owner")], 4),
    )])
    .expect("probe desires");
    st.set_schedule(vec![
        ScheduleRule::new("dust", 2).clause(vec![not_("dusty.here")], vec![insert("dusty.here")]),
    ])
    .expect("probe schedule");
    st.set_sorts(vec![(
        "place".to_owned(),
        vec!["here".to_owned(), "there".to_owned()],
    )])
    .expect("probe sorts");
    st.set_prediction_scope(vec![matches("char.Actor"), matches("char.Witness")])
        .expect("probe prediction scope");
    st.seed_die(7).expect("probe die seed");
    for s in [
        "char.vera",
        "char.otto",
        "char.quill",
        "dusty.here",
        "practice.greet.world",
        "practice.vigil.tower",
    ] {
        st.perform_outcome(&insert(s)).expect("probe setup fact");
    }
    st
}

/// The probe world's player name — `quill`, who is bound to a practice whose one
/// action they can never satisfy, so `trace --idle quill` and the walk's idle
/// pass are both exercised.
pub const PROBE_IDLER: &str = "quill";

fn probe_axioms() -> Vec<Axiom> {
    vec![
        Axiom::new(vec![matches("greeted.X.Y")], ["acquainted.X.Y"]),
        Axiom::new(vec![matches("acquainted.X.Y")], ["acquainted.Y.X"]),
    ]
}

fn note_fn() -> Function {
    Function::new("note", ["Who"])
        .case(vec![matches("char.Who")], vec![insert("noted.Who")])
        .case(vec![], vec![insert("noted.nobody")])
}

fn greet_practice() -> Practice {
    Practice::new("greet")
        .name("[G] greets")
        .roles(["G"])
        .data_facts(["tone.warm", "tone.curt!rude"])
        .init([insert("practice.greet.G.open")])
        .action(
            Action::new("[Actor]: greet [Other]")
                .when([
                    matches("char.Other"),
                    neq("Actor", "Other"),
                    not_("greeted.Actor.Other"),
                ])
                .then([
                    insert("greeted.Actor.Other"),
                    insert_for(2, "fresh.Actor.Other"),
                ]),
        )
        .action(
            Action::new("[Actor]: brag")
                .when([not_("bragged.Actor")])
                .then(brag_outcomes()),
        )
        .action(
            Action::new("[Actor]: tally the room")
                .when([
                    subquery("Cs", vec!["C".to_owned()], vec![matches("char.C")]),
                    count("N", "Cs"),
                    cmp(CmpOp::Gte, "N", "2"),
                    calc("M", CalcOp::Add, "N", "1"),
                    not_("tallied.Actor"),
                ])
                .then([
                    insert("tallied.Actor!M"),
                    call("note", vec!["Actor".to_owned()]),
                ]),
        )
        .action(
            Action::new("[Actor]: sweep")
                .when([
                    or_(vec![vec![matches("dusty.here")], vec![matches("muddy.here")]]),
                    absent(vec![matches("swept.Actor")]),
                ])
                .then([
                    for_each(vec![matches("char.C")], vec![insert("saw.C.sweep")]),
                    insert("swept.Actor"),
                    delete("dusty.here"),
                ]),
        )
        .action(Action::new("[Actor]: wait about"))
}

/// `bragged` then a 1-in-2 draw over everyone else — the one place the probe
/// touches the die (an ACTION, never setup: `worldshape`'s `setup_rolls_zero`
/// assertion covers the other case).
fn brag_outcomes() -> Vec<Outcome> {
    let mut outs = vec![insert("bragged.Actor")];
    outs.extend(
        draw(
            1,
            2,
            vec![matches("char.Other"), neq("Other", "Actor")],
            vec![insert("impressed.Other.Actor")],
        )
        .expect("probe draw odds"),
    );
    outs
}

fn vigil_practice() -> Practice {
    Practice::new("vigil")
        .name("[V] keeps vigil")
        .roles(["V"])
        .action(
            Action::new("[Actor]: keep watch")
                .when([
                    eq("Actor", "vera"),
                    exists(vec![matches("char.C")]),
                    not_("watching.Actor"),
                ])
                .then([insert("watching.Actor")]),
        )
}

/// The probe world with ONE authored label mis-transcribed — the mutation the
/// `worldshape` gate exists to catch. A swapped action label is invisible in the
/// world's own tests and presents at trace time as an ENUMERATION or DECISION
/// divergence; here it must present as a one-line structural diff instead.
#[cfg(test)]
pub(crate) fn probe_world_with_a_mistranscribed_label() -> State {
    let mut st = State::new();
    let mut greet = greet_practice();
    greet.actions[1].name = "[Actor]: boast".to_owned(); // was "[Actor]: brag"
    st.define_practices([greet, vigil_practice()])
        .expect("probe practices");
    st
}

/// Every condition constructor the probe names, for the encoder's own coverage
/// assertion (a constructor the fixture never names is a constructor the
/// canonical encoder ships unexercised).
#[cfg(test)]
pub(crate) fn condition_constructor_names(c: &prax_core::query::Condition) -> Vec<&'static str> {
    let mut out = Vec::new();
    use prax_core::query::Condition;
    fn go(c: &Condition, out: &mut Vec<&'static str>) {
        let (tag, kids): (&'static str, Vec<&Condition>) = match c {
            Condition::Match(_) => ("Match", vec![]),
            Condition::Not(_) => ("Not", vec![]),
            Condition::Eq(_, _) => ("Eq", vec![]),
            Condition::Neq(_, _) => ("Neq", vec![]),
            Condition::Cmp(_, _, _) => ("Cmp", vec![]),
            Condition::Calc(_, _, _, _) => ("Calc", vec![]),
            Condition::Count(_, _) => ("Count", vec![]),
            Condition::Subquery { where_, .. } => ("Subquery", where_.iter().collect()),
            Condition::Or(cls) => ("Or", cls.iter().flatten().collect()),
            Condition::Absent(cs) => ("Absent", cs.iter().collect()),
            Condition::Exists(cs) => ("Exists", cs.iter().collect()),
        };
        out.push(tag);
        for k in kids {
            go(k, out);
        }
    }
    go(c, &mut out);
    out
}
