//! Utility-based action selection (Versu §IX) with the v23 believed-mind
//! lookahead and the v34 prediction-reuse memo — the fidelity summit.
//!
//! Frozen reference: `src/Prax/Planner.hs`. This is the fidelity-critical file:
//! every differential golden and every world trace is downstream of
//! [`pick_action`]. The scoring arithmetic is transcribed op-for-op — the frozen
//! expression tree IS the spec, and its association order (`base + (acc + 0.9v)`,
//! never `(base + acc) + 0.9v`) is the contract. Rust f64 is strict IEEE with no
//! fast-math and no automatic FMA contraction (`a + b*c` never fuses without an
//! explicit `mul_add`), exactly as GHC x86_64 SSE2 — so the bits match.
//!
//! The single planner [`Interner`] threads through every fork (forks clone only
//! the cheap [`Runtime`]), so every name lives in one id space and the frozen
//! "cross-lineage" reuse hazard [S-C1] is structurally absent: the reuse gate's
//! delta-vs-read [`may_unify_syms`] is always a same-lineage id compare.

use rustc_hash::FxHashMap;
use smallvec::SmallVec;

use crate::db::{Bindings, Db, Val};
use crate::engine::{
    Defs, GroundedAction, Runtime, grounded_delta_anchors, perform_action_on, possible_actions_impl,
};
use crate::interner::{Interner, Sym};
use crate::minds::{believed_desires, cooked_desires_for, cooked_self_wants};
use crate::path::tokenize;
use crate::query::{Cond, ground_cond, query};
use crate::relevance::{Liveness, may_unify_syms, mover_read_anchors};
use crate::types::Character;

type Names = SmallVec<[Sym; 6]>;

/// One imagined path's accumulated effect on the pick's root state, as anchor
/// families with the derived-fact cone folded in (`Prax.Planner.PathDelta`).
/// `None` is the opaque path: some applied outcome could not be bounded, so
/// nothing at or below it may reuse.
type PathDelta = Option<Vec<Names>>;

/// A cooked scoring model: each want's conditions paired with its utility — what
/// [`evaluate_compiled`] sums over.
type Model = [(Vec<Cond>, i32)];

/// The v35 motive signature (`Prax.Types.MotiveSignature`): what I can do that I
/// care about, how I am doing (per-want satisfaction COUNTS), what is driving me
/// (own live desires), and what motives I know of. Compared for equality at the
/// character's own turn against their last deliberation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotiveSignature {
    pub bearing: Vec<String>,
    pub satisfaction: Vec<usize>,
    pub live_desires: Vec<String>,
    pub known_motives: Vec<(String, String)>,
}

/// A standing intention (`Prax.Types.Intention`): the action chosen at the last
/// deliberation (or the choice to do nothing) and the motive signature it was
/// based on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intention {
    pub act: Option<GroundedAction>,
    pub basis: MotiveSignature,
}

/// The v34 reuse net's hit counter (test builds only): every time the reuse gate
/// fires, this increments AFTER the reused step is `debug_assert`-checked equal to
/// the live one. The flagship reuse==live proptest asserts it nonzero, proving the
/// generator actually reaches reuse [S-I4].
#[cfg(debug_assertions)]
pub(crate) static REUSE_HITS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Total integer utility of a world to a cooked model: `Σ utility × #satisfying`
/// (count = query bindings, duplicates included) — `Prax.Planner.evaluateCooked`,
/// i64 accumulation, the INTEGER core (no FP here; the one f64 lift is in
/// [`value_after`]).
pub(crate) fn evaluate_compiled(interner: &mut Interner, view: &Db, model: &Model) -> i64 {
    let seed = Bindings::new();
    let mut total: i64 = 0;
    for (cs, u) in model {
        let n = query(interner, view, cs, &seed).len() as i64;
        total += i64::from(*u) * n;
    }
    total
}

/// The actions a character may actually take (`Prax.Planner.candidateActions`).
/// The dead act in no one's plans: a character marked dead in the BASE db has no
/// candidates. Otherwise, practice-bound filtering.
pub(crate) fn candidate_actions(
    interner: &mut Interner,
    defs: &Defs,
    rt: &Runtime,
    c: &Character,
) -> Vec<GroundedAction> {
    let dead = tokenize(interner, &format!("dead.{}", c.name)).expect("dead path");
    if rt.db().exists(interner, &dead.segs) {
        return Vec::new();
    }
    let acts = possible_actions_impl(interner, defs, rt.view(), &c.name);
    match &c.bound_to {
        None => acts,
        Some(pid) => acts.into_iter().filter(|a| &a.practice_id == pid).collect(),
    }
}

/// Is the mover within the actor's prediction scope (`Prax.Planner.inScope`)? The
/// scope template (over `Actor`/`Witness`) is grounded to the pair and queried
/// against the view; the empty template means everyone (an empty conjunction
/// yields the seed binding, non-null). [S-C1] The template is the already-compiled
/// `Compiled.scope`, never re-cooked.
fn in_scope(
    interner: &mut Interner,
    defs: &Defs,
    rt: &Runtime,
    actor: &Character,
    m: &Character,
) -> bool {
    let actor_key = interner.intern("Actor");
    let witness_key = interner.intern("Witness");
    let actor_sym = interner.intern(&actor.name);
    let m_sym = interner.intern(&m.name);
    let mut b = Bindings::new();
    b.insert(actor_key, Val::Sym(actor_sym));
    b.insert(witness_key, Val::Sym(m_sym));
    let grounded: Vec<Cond> = defs
        .compiled()
        .scope
        .iter()
        .map(|c| ground_cond(interner, &b, c))
        .collect();
    !query(interner, rt.view(), &grounded, &Bindings::new()).is_empty()
}

/// Is a believed desire dead RIGHT NOW (`Prax.Planner.deadNow`): at its floor (a
/// negative want-kind's own conditions have zero bindings) or gated shut (a
/// positive want-kind's environment gate has zero bindings)? The `Owner` binding
/// is passed to `query` as a SEED (so it grounds `Owner` at every occurrence),
/// mirroring the frozen mechanism. A desire with no recipe or `AlwaysLive` is
/// never dead-now.
fn dead_now(interner: &mut Interner, defs: &Defs, rt: &Runtime, m_name: &str, desire: &str) -> bool {
    let owner = owner_binding(interner, m_name);
    match defs.compiled().liveness.get(desire) {
        Some(Liveness::FloorCheck) => {
            let conds = defs
                .compiled()
                .desires
                .get(desire)
                .cloned()
                .unwrap_or_default();
            query(interner, rt.view(), &conds, &owner).is_empty()
        }
        Some(Liveness::GateCheck(gates)) => gates
            .iter()
            .any(|g| query(interner, rt.view(), g, &owner).is_empty()),
        _ => false,
    }
}

fn owner_binding(interner: &mut Interner, name: &str) -> Bindings {
    let mut b = Bindings::new();
    b.insert(interner.intern("Owner"), Val::Sym(interner.intern(name)));
    b
}

/// The predictor's guess at the mover's next move (`Prax.Planner.predictMove`):
/// the mover's best candidate under the predictor's believed model of them — and
/// only if it strictly improves that model over doing nothing. `None` when the
/// mind is unreadable or unmotivated. The internal sort is over INTEGERS (no FP in
/// prediction).
pub(crate) fn predict_move(
    interner: &mut Interner,
    defs: &Defs,
    rt: &Runtime,
    p: &Character,
    m: &Character,
) -> Option<GroundedAction> {
    let ds = believed_desires(interner, rt.view(), defs.desires(), &p.name, &m.name);
    if ds.is_empty() {
        return None;
    }
    let improvables = defs.compiled().improvables.clone();
    // Every believed desire DEAD (statically un-improvable OR dead-now): no
    // candidate can strictly beat standing still, so don't ground or evaluate any.
    let all_dead = ds
        .iter()
        .all(|d| !improvables.contains(&d.name) || dead_now(interner, defs, rt, &m.name, &d.name));
    if all_dead {
        return None;
    }
    let model = cooked_desires_for(interner, &defs.compiled().desires, &m.name, &ds);
    let still = evaluate_compiled(interner, rt.view(), &model);
    let cands = candidate_actions(interner, defs, rt, m);
    let mut scored: Vec<(GroundedAction, i64)> = Vec::with_capacity(cands.len());
    for a in cands {
        let mut fork = rt.clone();
        perform_action_on(interner, defs, &mut fork, &a);
        let s = evaluate_compiled(interner, fork.view(), &model);
        scored.push((a, s));
    }
    // sortOn (Down s, gaLabel), stable.
    scored.sort_by(|x, y| y.1.cmp(&x.1).then_with(|| x.0.label.cmp(&y.0.label)));
    match scored.first() {
        Some((a, s)) if *s > still => Some(a.clone()),
        _ => None,
    }
}

/// The living characters not the actor, one full cycle in cast order starting
/// after the actor (`Prax.Planner.othersAfter`).
fn others_after(
    interner: &mut Interner,
    defs: &Defs,
    rt: &Runtime,
    actor: &Character,
) -> Vec<Character> {
    let cs = living_characters(interner, defs, rt);
    let i = cs
        .iter()
        .position(|c| c.name == actor.name)
        .map_or(cs.len().saturating_sub(1), |k| k);
    let start = i + 1;
    cs.iter()
        .skip(start)
        .chain(cs.iter().take(start))
        .filter(|c| c.name != actor.name)
        .cloned()
        .collect()
}

/// The cast members with no `dead.<name>` in the BASE db (`Prax.Types.livingCharacters`).
fn living_characters(interner: &mut Interner, defs: &Defs, rt: &Runtime) -> Vec<Character> {
    defs.characters()
        .iter()
        .filter(|c| {
            let path = tokenize(interner, &format!("dead.{}", c.name)).expect("dead path");
            !rt.db().exists(interner, &path.segs)
        })
        .cloned()
        .collect()
}

/// Extend a path delta with one move's grounded anchors (`Prax.Planner.extendDelta`),
/// folding in the derived-fact cone: the moment any extension feeds an axiom
/// (footprint), every fireable head joins — and stays. Heads dedup against OLD
/// only. `None` propagates opacity.
fn extend_delta(defs: &Defs, delta: &PathDelta, new: Option<Vec<Names>>) -> PathDelta {
    match (delta, new) {
        (Some(old), Some(new)) => {
            let footprint = &defs.compiled().footprint;
            let feeds = new
                .iter()
                .any(|n| footprint.iter().any(|f| may_unify_syms(n, f)));
            let mut result = old.clone();
            result.extend(new);
            if feeds {
                for h in &defs.compiled().axiom_heads {
                    if !old.contains(h) {
                        result.push(h.clone());
                    }
                }
            }
            Some(result)
        }
        _ => None,
    }
}

/// The root memo (`Prax.Planner.scoreActions`'s `rootStep`/`rootReads`), filled on
/// demand against the retained root fork. `cohort` (eager) is the movers the root
/// enumerated; `steps` fills on first REUSE per mover, `reads` on first GATE CHECK.
struct PickMemo {
    cohort: Vec<String>,
    steps: FxHashMap<String, Option<GroundedAction>>,
    reads: FxHashMap<String, Vec<Names>>,
}

/// A mover's step decision at a state: scope gate + prediction
/// (`Prax.Planner.stepPredict`).
fn step_predict(
    interner: &mut Interner,
    defs: &Defs,
    s: &Runtime,
    actor: &Character,
    m: &Character,
) -> Option<GroundedAction> {
    if in_scope(interner, defs, s, actor, m) {
        predict_move(interner, defs, s, actor, m)
    } else {
        None
    }
}

/// Reuse the root's decision when sound; live otherwise (`Prax.Planner.predictAt`).
/// Reuse iff the path is transparent (`delta` is `Some`) AND the mover is in the
/// root cohort AND no delta anchor may-unify the mover's root read anchors.
#[allow(clippy::too_many_arguments)]
fn predict_at(
    interner: &mut Interner,
    defs: &Defs,
    memo: &mut PickMemo,
    st0: &Runtime,
    actor: &Character,
    delta: &PathDelta,
    s: &Runtime,
    m: &Character,
) -> Option<GroundedAction> {
    if let Some(d) = delta
        && memo.cohort.contains(&m.name)
    {
        if !memo.reads.contains_key(&m.name) {
            let c = defs.compiled();
            let anchors = mover_read_anchors(
                interner, &c.scope, &c.practices, &c.fns, &c.desires, &actor.name, &m.name,
            );
            memo.reads.insert(m.name.clone(), anchors);
        }
        let intersects = {
            let rs = &memo.reads[&m.name];
            d.iter().any(|dd| rs.iter().any(|r| may_unify_syms(dd, r)))
        };
        if !intersects {
            if !memo.steps.contains_key(&m.name) {
                let sp = step_predict(interner, defs, st0, actor, m);
                memo.steps.insert(m.name.clone(), sp);
            }
            let reused = memo.steps[&m.name].clone();
            // [S-I4] The reuse-site net: in test builds, the reused step must
            // equal the live one; and the hit is counted so the proptest can
            // prove the generator reaches reuse.
            #[cfg(debug_assertions)]
            {
                let live = step_predict(interner, defs, s, actor, m);
                debug_assert_eq!(
                    reused, live,
                    "v34 prediction reuse must equal the live step for mover {}",
                    m.name
                );
                REUSE_HITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            return reused;
        }
    }
    step_predict(interner, defs, s, actor, m)
}

/// Score each candidate by the imagined round it opens, best first
/// (`Prax.Planner.scoreActions.go`). Ties broken by label (stable → candidate
/// enumeration order for full ties). Native result order [D-C1].
#[allow(clippy::too_many_arguments)]
fn go(
    interner: &mut Interner,
    defs: &Defs,
    memo: &mut PickMemo,
    st0: &Runtime,
    actor: &Character,
    d: i32,
    delta: &PathDelta,
    st: &Runtime,
    self_wants: &Model,
) -> Vec<(GroundedAction, f64)> {
    let mut scored: Vec<(GroundedAction, f64)> = Vec::new();
    for a in candidate_actions(interner, defs, st, actor) {
        let new = grounded_delta_anchors(interner, defs, &a);
        let delta1 = extend_delta(defs, delta, new);
        let mut st1 = st.clone();
        perform_action_on(interner, defs, &mut st1, &a);
        let v = value_after(interner, defs, memo, st0, actor, d, &delta1, &st1, self_wants);
        scored.push((a, v));
    }
    // y.total_cmp(x) for descending score; then ascending label. Stable.
    scored.sort_by(|x, y| y.1.total_cmp(&x.1).then_with(|| x.0.label.cmp(&y.0.label)));
    scored
}

/// The one f64 lift and the discounted round-walk (`Prax.Planner.valueAfter`).
/// The association is the contract: `base + (othersScore + selfNext)`, the fold's
/// adds and `0.9 * v` left as written.
#[allow(clippy::too_many_arguments)]
fn value_after(
    interner: &mut Interner,
    defs: &Defs,
    memo: &mut PickMemo,
    st0: &Runtime,
    actor: &Character,
    d: i32,
    delta: &PathDelta,
    st1: &Runtime,
    self_wants: &Model,
) -> f64 {
    let base = evaluate_compiled(interner, st1.view(), self_wants) as f64;
    if d <= 0 {
        return base;
    }
    // The imagined round: each other in st1's LIVING cast, rotation order.
    let others = others_after(interner, defs, st1, actor);
    let mut acc = 0.0_f64;
    let mut s = st1.clone();
    let mut dlt = delta.clone();
    for m in &others {
        match predict_at(interner, defs, memo, st0, actor, &dlt, &s, m) {
            None => {} // skip = no term
            Some(ga) => {
                let new = grounded_delta_anchors(interner, defs, &ga);
                perform_action_on(interner, defs, &mut s, &ga);
                dlt = extend_delta(defs, &dlt, new);
                // cookedSelfWants s' actor is invariant across states (the §1
                // hoist): the same `self_wants`, evaluated at s'.
                acc += 0.5 * (evaluate_compiled(interner, s.view(), self_wants) as f64);
            }
        }
    }
    let self_next = match go(interner, defs, memo, st0, actor, d - 1, &dlt, &s, self_wants).first() {
        Some((_, v)) => 0.9 * v,
        None => 0.0,
    };
    base + (acc + self_next)
}

/// Score every candidate for the actor at the pick's root (`Prax.Planner.scoreActions`).
pub(crate) fn score_actions(
    interner: &mut Interner,
    defs: &Defs,
    rt: &Runtime,
    depth: i32,
    actor: &Character,
) -> Vec<(GroundedAction, f64)> {
    let cohort: Vec<String> = others_after(interner, defs, rt, actor)
        .iter()
        .map(|c| c.name.clone())
        .collect();
    let mut memo = PickMemo {
        cohort,
        steps: FxHashMap::default(),
        reads: FxHashMap::default(),
    };
    // The §1 permitted hoist: cookedSelfWants actor, once per pick.
    let c = defs.compiled();
    let self_wants = cooked_self_wants(interner, &c.wants, &c.desires, defs.desires(), actor);
    let root_delta: PathDelta = Some(Vec::new());
    go(interner, defs, &mut memo, rt, actor, depth, &root_delta, rt, &self_wants)
}

/// The actor's best action, if any (`Prax.Planner.pickAction`).
pub(crate) fn pick_action(
    interner: &mut Interner,
    defs: &Defs,
    rt: &Runtime,
    depth: i32,
    actor: &Character,
) -> Option<GroundedAction> {
    score_actions(interner, defs, rt, depth, actor)
        .into_iter()
        .next()
        .map(|(a, _)| a)
}

/// The character's current motive signature — grounding without scoring, four
/// count/existence walks against the view, no lookahead (`Prax.Planner.motiveSignature`).
pub(crate) fn motive_signature(
    interner: &mut Interner,
    defs: &Defs,
    rt: &Runtime,
    c: &Character,
) -> MotiveSignature {
    let bearing_set = defs
        .compiled()
        .cares_about
        .get(&c.name)
        .cloned()
        .unwrap_or_default();
    let cands = candidate_actions(interner, defs, rt, c);
    let mut bearing: Vec<String> = cands
        .iter()
        .map(|ga| ga.action_id.clone())
        .filter(|aid| bearing_set.contains(aid))
        .collect();
    bearing.sort();
    bearing.dedup();

    let comp = defs.compiled();
    let self_wants = cooked_self_wants(interner, &comp.wants, &comp.desires, defs.desires(), c);
    let seed = Bindings::new();
    let satisfaction: Vec<usize> = self_wants
        .iter()
        .map(|(cs, _)| query(interner, rt.view(), cs, &seed).len())
        .collect();

    let improvables = defs.compiled().improvables.clone();
    let mut live_desires: Vec<String> = Vec::new();
    for d in defs.desires() {
        if c.desires.contains(&d.name)
            && improvables.contains(&d.name)
            && !dead_now(interner, defs, rt, &c.name, &d.name)
        {
            live_desires.push(d.name.clone());
        }
    }

    let known_motives = known_motives_of(interner, rt.view(), &c.name);
    MotiveSignature {
        bearing,
        satisfaction,
        live_desires,
        known_motives,
    }
}

/// The two-level believed-motive walk (`msKnownMotives`): every mover the
/// character has a believed model of, paired with each believed desire name.
fn known_motives_of(interner: &mut Interner, view: &Db, name: &str) -> Vec<(String, String)> {
    let prefix = format!("{name}.believes.desires");
    let ppath = tokenize(interner, &prefix).expect("believes path");
    let movers = view.child_keys(interner, &ppath.segs);
    let mut out = Vec::new();
    for mv in movers {
        let mpath = tokenize(interner, &format!("{prefix}.{mv}")).expect("believes path");
        for d in view.child_keys(interner, &mpath.segs) {
            out.push((mv.clone(), d));
        }
    }
    out
}

#[cfg(test)]
mod smoke {
    // An end-to-end port of PlannerSpec's discriminating fixtures — the scoring
    // core, the believed-mind round-walk, dead-now, scope, and v34 reuse all
    // validated against the frozen expected values.
    use crate::engine::State;
    use crate::query::{Condition, eq, matches, neq, not_};
    use crate::types::{Action, Axiom, Character, Desire, Practice, Want, insert};

    fn m(s: &str) -> Condition {
        Condition::Match(s.into())
    }
    fn want1(cond: &str, u: i32) -> Want {
        Want::new(vec![m(cond)], u)
    }
    fn desire(name: &str, cond: &str, u: i32) -> Desire {
        Desire::new(name, want1(cond, u))
    }

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

    fn beth() -> Character {
        Character::new("beth").want(Want::new(
            vec![Condition::Match(
                "practice.tendBar.Bartender.customer.beth!order!cider".into(),
            )],
            10,
        ))
    }

    fn bar_state() -> State {
        let mut st = State::new();
        st.define_practices([tend_bar()]).unwrap();
        st.set_characters(vec![beth()]).unwrap();
        st.perform_outcome(&insert("practice.tendBar.ada")).unwrap();
        st
    }

    #[test]
    fn tendbar_pick_and_scores() {
        let mut walked = bar_state();
        walked
            .perform_outcome(&insert("practice.tendBar.ada.customer.beth"))
            .unwrap();
        let beth = beth();

        // depth 0: only cider satisfies beth's want.
        assert_eq!(
            walked.pick_action(0, &beth).map(|g| g.label),
            Some("beth: Order cider".to_owned())
        );
        let scored = walked.score_actions(0, &beth);
        assert_eq!(scored[0].0.label, "beth: Order cider");
        assert_eq!(scored[0].1, 10.0);
        for (ga, s) in &scored {
            if ga.label != "beth: Order cider" {
                assert_eq!(*s, 0.0, "non-cider action {} scored {}", ga.label, s);
            }
        }

        // depth 1: walking up is worthless now but worth 0.9 * 10 = 9.0 ahead.
        let mut bar = bar_state();
        let scored1 = bar.score_actions(1, &beth);
        let walk = scored1
            .iter()
            .find(|(g, _)| g.label == "beth: Walk up to bar")
            .expect("walk offered");
        assert_eq!(walk.1, 9.0, "walk-up at depth 1");
        assert_eq!(
            bar.pick_action(1, &beth).map(|g| g.label),
            Some("beth: Walk up to bar".to_owned())
        );
    }

    // ---- believed-mind prediction ------------------------------------------

    fn cider_vocab() -> Vec<Desire> {
        vec![desire(
            "cider-craving",
            "practice.tendBar.Bartender.customer.Owner!order!cider",
            10,
        )]
    }

    fn walked_up_with_belief() -> State {
        // walkedUp + the cider vocabulary + ada/beth, ada gossiped beth's craving.
        let mut st = bar_state();
        st.perform_outcome(&insert("practice.tendBar.ada.customer.beth"))
            .unwrap();
        st.set_desires(cider_vocab()).unwrap();
        st.set_characters(vec![
            Character::new("beth").holds("cider-craving"),
            Character::new("ada"),
        ])
        .unwrap();
        st
    }

    #[test]
    fn predict_move_is_belief_relative_and_motivated() {
        let ada = Character::new("ada");
        let beth = Character::new("beth").holds("cider-craving");
        // No belief: no prediction.
        let mut plain = bar_state();
        plain
            .perform_outcome(&insert("practice.tendBar.ada.customer.beth"))
            .unwrap();
        plain.set_desires(cider_vocab()).unwrap();
        plain
            .set_characters(vec![beth.clone(), Character::new("ada")])
            .unwrap();
        assert_eq!(plain.predict_move(&ada, &beth), None);

        // A believed motive → the mover's motivated best.
        let mut st = walked_up_with_belief();
        st.perform_outcome(&insert(
            "ada.believes.desires.beth.cider-craving.heard.gossip",
        ))
        .unwrap();
        assert_eq!(
            st.predict_move(&ada, &beth).map(|g| g.label),
            Some("beth: Order cider".to_owned())
        );
        // Motivated-only: once satisfied, predict still.
        st.perform_outcome(&insert(
            "practice.tendBar.ada.customer.beth!order!cider",
        ))
        .unwrap();
        assert_eq!(st.predict_move(&ada, &beth), None);
    }

    #[test]
    fn dead_are_predicted_to_do_nothing() {
        let ada = Character::new("ada");
        let beth = Character::new("beth").holds("cider-craving");
        let mut st = walked_up_with_belief();
        st.perform_outcome(&insert(
            "ada.believes.desires.beth.cider-craving.heard.gossip",
        ))
        .unwrap();
        assert_eq!(
            st.predict_move(&ada, &beth).map(|g| g.label),
            Some("beth: Order cider".to_owned())
        );
        st.perform_outcome(&insert("dead.beth")).unwrap();
        assert_eq!(st.predict_move(&ada, &beth), None);
    }

    // ---- the believed round-walk -------------------------------------------

    fn heist_state(scope: Option<Vec<Condition>>) -> State {
        let grab = Practice::new("heist")
            .roles(["R"])
            .action(
                Action::new("[Actor]: grab the relic")
                    .when([m("gate.open"), not_("grabbed.inge"), eq("Actor", "inge")])
                    .then([insert("grabbed.inge")]),
            )
            .action(
                Action::new("[Actor]: open the gate")
                    .when([eq("Actor", "olaf"), not_("gate.open")])
                    .then([insert("gate.open")]),
            )
            .action(Action::new("[Actor]: Wait about"));
        let mut st = State::new();
        st.define_practices([grab]).unwrap();
        st.set_characters(vec![
            Character::new("olaf").want(want1("grabbed.inge", 6)),
            Character::new("inge").holds("covet-relic"),
        ])
        .unwrap();
        st.set_desires(vec![desire("covet-relic", "grabbed.Owner", 10)])
            .unwrap();
        if let Some(sc) = scope {
            st.set_prediction_scope(sc).unwrap();
        }
        st.perform_outcome(&insert("practice.heist.here")).unwrap();
        st
    }

    #[test]
    fn round_walk_credits_a_predicted_enabling_world() {
        let olaf = Character::new("olaf").want(want1("grabbed.inge", 6));
        let mut told = heist_state(None);
        told.perform_outcome(&insert(
            "olaf.believes.desires.inge.covet-relic.heard.inge",
        ))
        .unwrap();
        assert_eq!(
            told.pick_action(1, &olaf).map(|g| g.label),
            Some("olaf: open the gate".to_owned())
        );
        // Not in on it: opening the gate gains him nothing.
        let mut cold = heist_state(None);
        assert_ne!(
            cold.pick_action(1, &olaf).map(|g| g.label),
            Some("olaf: open the gate".to_owned())
        );
    }

    #[test]
    fn scope_gates_participation() {
        let olaf = Character::new("olaf").want(want1("grabbed.inge", 6));
        let scope = vec![m("at.Actor!Room"), m("at.Witness!Room")];
        let mut apart = heist_state(Some(scope.clone()));
        apart
            .perform_outcome(&insert(
                "olaf.believes.desires.inge.covet-relic.heard.inge",
            ))
            .unwrap();
        apart.perform_outcome(&insert("at.olaf!gatehouse")).unwrap();
        apart.perform_outcome(&insert("at.inge!vault")).unwrap();
        assert_ne!(
            apart.pick_action(1, &olaf).map(|g| g.label),
            Some("olaf: open the gate".to_owned())
        );
        let mut together = heist_state(Some(scope));
        together
            .perform_outcome(&insert(
                "olaf.believes.desires.inge.covet-relic.heard.inge",
            ))
            .unwrap();
        together.perform_outcome(&insert("at.olaf!vault")).unwrap();
        together.perform_outcome(&insert("at.inge!vault")).unwrap();
        assert_eq!(
            together.pick_action(1, &olaf).map(|g| g.label),
            Some("olaf: open the gate".to_owned())
        );
    }

    #[test]
    fn sequential_chain_credits_signal_at_14() {
        let chain = Practice::new("chain")
            .roles(["R"])
            .action(
                Action::new("[Actor]: signal")
                    .when([eq("Actor", "alice"), not_("signaled")])
                    .then([insert("signaled")]),
            )
            .action(
                Action::new("[Actor]: relay")
                    .when([eq("Actor", "bob"), m("signaled"), not_("relayed")])
                    .then([insert("relayed")]),
            )
            .action(
                Action::new("[Actor]: deliver")
                    .when([eq("Actor", "carol"), m("relayed"), not_("delivered")])
                    .then([insert("delivered")]),
            )
            .action(Action::new("[Actor]: Wait about"));
        let alice = Character::new("alice").want(want1("delivered", 10));
        let mut st = State::new();
        st.define_practices([chain]).unwrap();
        st.set_characters(vec![alice.clone(), Character::new("bob"), Character::new("carol")])
            .unwrap();
        st.set_desires(vec![
            desire("relay-desire", "relayed", 10),
            desire("deliver-desire", "delivered", 10),
        ])
        .unwrap();
        st.perform_outcome(&insert("practice.chain.here")).unwrap();
        let mut both = st.clone();
        both.perform_outcome(&insert(
            "alice.believes.desires.bob.relay-desire.heard.gossip",
        ))
        .unwrap();
        both.perform_outcome(&insert(
            "alice.believes.desires.carol.deliver-desire.heard.gossip",
        ))
        .unwrap();
        let scored = both.score_actions(1, &alice);
        let sig = scored
            .iter()
            .find(|(g, _)| g.label == "alice: signal")
            .expect("signal offered");
        assert_eq!(sig.1, 14.0);
        assert_eq!(
            both.pick_action(1, &alice).map(|g| g.label),
            Some("alice: signal".to_owned())
        );
        // Neither belief: signal earns nothing.
        let none = st.score_actions(1, &alice);
        let s0 = none.iter().find(|(g, _)| g.label == "alice: signal").unwrap();
        assert_eq!(s0.1, 0.0);
    }

    #[test]
    fn mid_round_death_silences_the_rest_of_the_round() {
        let duel = Practice::new("duel")
            .roles(["R"])
            .action(
                Action::new("[Actor]: signal")
                    .when([eq("Actor", "alice"), not_("signaled")])
                    .then([insert("signaled")]),
            )
            .action(
                Action::new("[Actor]: kill carol")
                    .when([eq("Actor", "bob"), m("signaled"), not_("dead.carol")])
                    .then([insert("dead.carol")]),
            )
            .action(
                Action::new("[Actor]: deliver")
                    .when([eq("Actor", "carol"), m("signaled"), not_("delivered")])
                    .then([insert("delivered")]),
            )
            .action(Action::new("[Actor]: Wait about"));
        let alice = Character::new("alice").want(want1("delivered", 10));
        let mut st = State::new();
        st.define_practices([duel]).unwrap();
        st.set_characters(vec![alice.clone(), Character::new("bob"), Character::new("carol")])
            .unwrap();
        st.set_desires(vec![
            desire("kill-desire", "dead.carol", 10),
            desire("deliver-desire", "delivered", 10),
        ])
        .unwrap();
        st.perform_outcome(&insert("practice.duel.here")).unwrap();
        st.perform_outcome(&insert(
            "alice.believes.desires.bob.kill-desire.heard.gossip",
        ))
        .unwrap();
        st.perform_outcome(&insert(
            "alice.believes.desires.carol.deliver-desire.heard.gossip",
        ))
        .unwrap();
        let scored = st.score_actions(1, &alice);
        let sig = scored.iter().find(|(g, _)| g.label == "alice: signal").unwrap();
        assert_eq!(sig.1, 0.0, "a corpse must not be credited");
    }

    // ---- dead-now -----------------------------------------------------------

    #[test]
    fn dead_now_floor_and_gate() {
        // Floor: a markless conscience skips; one lied-mark goes live.
        let confess = Practice::new("confess")
            .roles(["R"])
            .action(
                Action::new("[Actor]: confess")
                    .when([m("lied.Actor")])
                    .then([crate::types::delete("lied.Actor")]),
            )
            .action(Action::new("[Actor]: Wait about"));
        let priya = Character::new("priya");
        let beth = Character::new("beth");
        let mut st = State::new();
        st.define_practices([confess]).unwrap();
        st.set_characters(vec![priya.clone(), beth.clone()]).unwrap();
        st.set_desires(vec![desire("hates-lying", "lied.Owner", -5)]).unwrap();
        st.perform_outcome(&insert("practice.confess.here")).unwrap();
        st.perform_outcome(&insert(
            "priya.believes.desires.beth.hates-lying.heard.gossip",
        ))
        .unwrap();
        assert_eq!(st.predict_move(&priya, &beth), None, "markless: at the floor");
        st.perform_outcome(&insert("lied.beth")).unwrap();
        assert_eq!(
            st.predict_move(&priya, &beth).map(|g| g.label),
            Some("beth: confess".to_owned())
        );
    }

    #[test]
    fn dead_now_conservative_axiom_derivable_never_skips() {
        let toil = Practice::new("toil")
            .roles(["R"])
            .action(Action::new("[Actor]: toil").then([insert("starving.Actor")]))
            .action(Action::new("[Actor]: Wait about"));
        let priya = Character::new("priya");
        let beth = Character::new("beth");
        let mut st = State::new();
        st.define_practices([toil]).unwrap();
        st.set_characters(vec![priya.clone(), beth.clone()]).unwrap();
        st.set_axioms(vec![Axiom::new(vec![m("starving.Owner")], ["hungry.Owner"])])
            .unwrap();
        st.set_desires(vec![desire("craves-hunger", "hungry.Owner", 5)]).unwrap();
        st.perform_outcome(&insert("practice.toil.here")).unwrap();
        st.perform_outcome(&insert(
            "priya.believes.desires.beth.craves-hunger.heard.gossip",
        ))
        .unwrap();
        assert_eq!(
            st.predict_move(&priya, &beth).map(|g| g.label),
            Some("beth: toil".to_owned())
        );
    }

    // ---- v34 reuse ----------------------------------------------------------

    #[test]
    fn reuse_base_fact_delta_recomputed_not_reused() {
        let p = Practice::new("mess")
            .roles(["R"])
            .action(
                Action::new("[Actor]: taunt beth")
                    .when([neq("Actor", "beth")])
                    .then([insert("hungry.beth")]),
            )
            .action(
                Action::new("[Actor]: eat lunch")
                    .when([m("hungry.Actor")])
                    .then([insert("meal.Actor")]),
            )
            .action(Action::new("[Actor]: idle about"));
        let priya = Character::new("priya").want(want1("meal.beth", 10));
        let beth = Character::new("beth");
        let mut st = State::new();
        st.define_practices([p]).unwrap();
        st.set_characters(vec![priya.clone(), beth.clone()]).unwrap();
        st.set_desires(vec![Desire::new(
            "wants-food",
            Want::new(vec![m("hungry.Owner"), m("meal.Owner")], 5),
        )])
        .unwrap();
        st.perform_outcome(&insert("practice.mess.here")).unwrap();
        st.perform_outcome(&insert(
            "priya.believes.desires.beth.wants-food.heard.gossip",
        ))
        .unwrap();
        assert_eq!(st.predict_move(&priya, &beth), None, "root: beth unmotivated");
        assert_eq!(
            st.pick_action(2, &priya).map(|g| g.label),
            Some("priya: taunt beth".to_owned()),
            "the pick must see through the taunt (no stale reuse)"
        );
    }

    #[test]
    fn reuse_derived_cone_flip_recomputed_not_reused() {
        let p = Practice::new("court")
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
        let priya = Character::new("priya").want(want1("apology.beth", 10));
        let beth = Character::new("beth");
        let mut st = State::new();
        st.define_practices([p]).unwrap();
        st.set_axioms(vec![Axiom::new(
            vec![m("W.believes.C.thief"), not_("recanted.C")],
            ["regards.W.C.thief"],
        )])
        .unwrap();
        st.set_characters(vec![priya.clone(), beth.clone()]).unwrap();
        st.set_desires(vec![desire("hates-infamy", "regards.V.Owner.thief", -8)])
            .unwrap();
        st.perform_outcome(&insert("practice.court.here")).unwrap();
        st.perform_outcome(&insert(
            "priya.believes.desires.beth.hates-infamy.heard.gossip",
        ))
        .unwrap();
        assert_eq!(st.predict_move(&priya, &beth), None);
        assert_eq!(
            st.pick_action(2, &priya).map(|g| g.label),
            Some("priya: denounce beth".to_owned()),
            "the cone must be recomputed, not reused"
        );
    }

    // The v34 reuse gate must actually be REACHED (both arms) [S-I4]: a
    // transparent empty-delta path (the actor's Wait candidate) makes the gate
    // fire for every cohort mover; in test builds every fire is debug_assert'd
    // reused==live, and the hit counter increments — asserted nonzero here.
    #[cfg(debug_assertions)]
    #[test]
    fn reuse_gate_is_reached_and_verified() {
        use std::sync::atomic::Ordering;
        let before = crate::planner::REUSE_HITS.load(Ordering::Relaxed);
        let olaf = Character::new("olaf").want(want1("grabbed.inge", 6));
        let mut told = heist_state(None);
        told.perform_outcome(&insert(
            "olaf.believes.desires.inge.covet-relic.heard.inge",
        ))
        .unwrap();
        let _ = told.pick_action(2, &olaf);
        let after = crate::planner::REUSE_HITS.load(Ordering::Relaxed);
        assert!(after > before, "the v34 reuse gate was never reached");
    }
}
