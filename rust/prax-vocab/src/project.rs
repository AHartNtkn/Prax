//! Endeavors: long-horizon behavior as part-sets.
//!
//! A project TYPE is authored vocabulary (like practices, deeds, and desires); a
//! project INSTANCE emerges when a disposed character's planner chooses the
//! undertake action. The pursuit is a named [`Desire`] counting the owner's
//! completed parts — DORMANT without an instance (zero bindings, zero utility),
//! which is how disposed characters carry it permanently and undertaking switches
//! it on: conditioned wants ARE injectable wants.
//!
//! Progress itself is rewarding (+w per completed part — the authored weight is
//! how invested this character is in this kind of work), so horizon length is
//! irrelevant: every next part is locally visible to the ordinary planner.
//!
//! A plan is a SET of parts, not a linear chain: parts complete independently and
//! in parallel, and NOT ALL are required for success. Topology is authored two
//! ways: [`Part::after`] names sibling parts that must finish first (a validated,
//! privately-compiled dependency edge), and [`Part::needs`] carries world
//! resources and genuine threshold gates (raw conditions — `Subquery`/`Count`/
//! `Cmp` over the ledger family expresses "fire when any 3 of 5 are done").
//!
//! Completing a part writes the per-part completion ledger
//! (`practice.<pid>.Owner.did.<partName>`) — JUSTIFIED infrastructure, not
//! world-visible fiction. The ledger entry doubles as the once-guard, so each part
//! fires ONCE per instance.
//!
//! Frozen reference: `src/Prax/Project.hs`. The generated LABEL ORDER and GUARD
//! ORDER are golden-visible (S7 design §3.3): [`endeavor`] is transcribed
//! literally, and `worldshape village` compares the whole generated practice
//! structurally against the frozen one.

use prax_core::error::WorldError;
use prax_core::query::{Condition, eq, matches, not_};
use prax_core::types::{Action, Desire, Outcome, Practice, Want, authored_var_clash, insert};

/// One moving part of the work: its ledger key, action label, the sibling parts
/// it depends on, what the world must provide, and what completing it does to the
/// world (beyond writing the ledger).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// Single path segment; the ledger key.
    pub name: String,
    /// Action label.
    pub label: String,
    /// Dependency edges: sibling part names.
    pub after: Vec<String>,
    /// World resources; threshold gates.
    pub needs: Vec<Condition>,
    pub yields: Vec<Outcome>,
}

impl Part {
    /// A part, field for field in the frozen `Part`'s declaration order.
    pub fn new(
        name: impl Into<String>,
        label: impl Into<String>,
        after: Vec<String>,
        needs: Vec<Condition>,
        yields: Vec<Outcome>,
    ) -> Part {
        Part {
            name: name.into(),
            label: label.into(),
            after,
            needs,
            yields,
        }
    }
}

/// An authored endeavor: the undertake action (for the world to slot into one of
/// its own practices), the part-set practice, and the named pursuit desire (for
/// the world's vocabulary). One instance per owner; a finished instance persists
/// as the record of the work.
///
/// The undertake and the parts live in DIFFERENT practices, so an endeavor owner
/// must not be `bound_to` a single practice — a bound character could undertake
/// but never complete a part (or vice versa).
///
/// Every [`Part::after`] name is validated against the actual part set — a
/// dangling or misspelled edge is a LOUD construction error, as is a name that
/// forms or feeds a cycle (the whole endeavor would be silently unreachable). The
/// fact-path convention is PRIVATE: the compiler builds the ledger conditions
/// itself, so a typo'd edge cannot fail silently as a never-available part.
///
/// # Errors
/// In the frozen guard order: no parts; a punctuated project id; a punctuated part
/// name; a duplicate part name; a dangling dependency edge; a `Prax`-namespace
/// variable in a part's needs or yields; an unreachable part (its dependency edges
/// form or feed a cycle).
pub fn endeavor(
    pid: &str,
    w: i32,
    ulabel: &str,
    gate: Vec<Condition>,
    parts: &[Part],
) -> Result<(Action, Practice, Desire), WorldError> {
    if parts.is_empty() {
        return Err(WorldError::PatternVariables {
            context: "Project.endeavor".to_owned(),
            pattern: pid.to_owned(),
            needs: "have parts (an endeavor is work)".to_owned(),
        });
    }
    if pid.contains(['.', '!']) {
        return Err(WorldError::NotASinglePathSegment {
            context: "Project.endeavor: id".to_owned(),
            name: pid.to_owned(),
        });
    }
    let names: Vec<&str> = parts.iter().map(|p| p.name.as_str()).collect();
    if let Some(n) = names.iter().find(|n| n.contains(['.', '!'])) {
        return Err(WorldError::NotASinglePathSegment {
            context: format!("Project.endeavor {pid:?}: part name (it keys the ledger)"),
            name: (*n).to_owned(),
        });
    }
    let mut seen: Vec<&str> = Vec::new();
    for n in &names {
        if seen.contains(n) {
            return Err(WorldError::NotASinglePathSegment {
                context: format!("Project.endeavor {pid:?}: duplicate part name"),
                name: (*n).to_owned(),
            });
        }
        seen.push(n);
    }
    if let Some((p, e)) = parts
        .iter()
        .flat_map(|p| p.after.iter().map(move |e| (p, e)))
        .find(|(_, e)| !names.contains(&e.as_str()))
    {
        return Err(WorldError::PatternVariables {
            context: format!("Project.endeavor {pid:?}: part {:?}", p.name),
            pattern: e.clone(),
            needs: "name an actual part (a dangling dependency edge)".to_owned(),
        });
    }
    for p in parts {
        if let Some(v) = authored_var_clash(&[], &p.needs, &p.yields)?.first() {
            return Err(WorldError::ReservedVarClash {
                context: format!("Project.endeavor {pid:?}: part {:?}", p.name),
                var: v.clone(),
                extra: String::new(),
            });
        }
    }
    // Transitive reachability from the edge-free roots (fixpoint). A part is
    // reachable iff all its edges point to reachable parts; a graph with no
    // edge-free node contains a cycle, and every cycle participant or dependent
    // is unreachable — so reachability from the roots IS complete cycle
    // detection.
    let mut reachable: Vec<&str> = parts
        .iter()
        .filter(|p| p.after.is_empty())
        .map(|p| p.name.as_str())
        .collect();
    loop {
        let mut next = reachable.clone();
        for p in parts {
            if p.after.iter().all(|e| next.contains(&e.as_str()))
                && !next.contains(&p.name.as_str())
            {
                next.push(p.name.as_str());
            }
        }
        if next.len() == reachable.len() {
            break;
        }
        reachable = next;
    }
    if let Some(n) = names.iter().find(|n| !reachable.contains(n)) {
        return Err(WorldError::PatternVariables {
            context: format!("Project.endeavor {pid:?}: part {n:?}"),
            pattern: (*n).to_owned(),
            needs: "be reachable (its dependency edges form or feed a cycle) -- unreachable"
                .to_owned(),
        });
    }

    let inst = |suffix: &str| format!("practice.{pid}{suffix}");
    let ledger = |n: &str| inst(&format!(".Owner.did.{n}"));

    let mut undertake_when = gate;
    undertake_when.push(not_(inst(".Actor")));
    let undertake = Action::new(ulabel)
        .when(undertake_when)
        .then([insert(inst(".Actor"))]);

    // No instance-fact Match and no init seed: instance existence and Owner's
    // binding ride the practice-instance ENUMERATION (the undertake fact's trie
    // node — `possible_actions` unifies the practice's instance names against
    // it), so a part's gate needs only ownership, its once-guard, its edges, and
    // its needs.
    let part_action = |p: &Part| {
        let mut when = vec![eq("Actor", "Owner"), not_(ledger(&p.name))];
        when.extend(p.after.iter().map(|d| matches(ledger(d))));
        when.extend(p.needs.iter().cloned());
        let mut then = vec![insert(ledger(&p.name))];
        then.extend(p.yields.iter().cloned());
        Action::new(&p.label).when(when).then(then)
    };
    let mut proj = Practice::new(pid)
        .name(format!("[Owner] pursues {pid}"))
        .roles(["Owner"]);
    for p in parts {
        proj = proj.action(part_action(p));
    }

    let pursuit = Desire::new(
        format!("pursues-{pid}"),
        Want::new(vec![matches(inst(".Owner.did.P"))], w),
    );
    Ok((undertake, proj, pursuit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_core::engine::State;
    use prax_core::query::{CmpOp, cmp, count, subquery};
    use prax_core::types::{Character, delete};

    // H: ProjectSpec.hs "Prax.Project"
    //
    // The frozen `Prax.ProjectSpec`, re-expressed against the Rust engine.
    //
    // Fixtures: endeavors as part-SETS (no cursor). Each names its parts, the
    // ledger keys; topology is authored two ways — `after` (validated sibling
    // edges) and `needs` (world resources + threshold gates).

    fn seg(ss: &[&str]) -> Vec<String> {
        ss.iter().map(|s| (*s).to_owned()).collect()
    }

    /// A 4-part CHAIN: mia builds an oven, part after part; pat looks on. The
    /// edges (fetch after sweep, shape after fetch, fire after shape) make it a
    /// chain again, so it re-pins v24's horizon theorem on the new machinery.
    fn chain_parts() -> Vec<Part> {
        vec![
            Part::new(
                "sweep",
                "[Actor]: sweep the hearth",
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
            Part::new(
                "fetch",
                "[Actor]: fetch the clay",
                seg(&["sweep"]),
                vec![matches("clay.available")],
                vec![insert("carrying.Owner.clay")],
            ),
            Part::new(
                "shape",
                "[Actor]: shape the oven",
                seg(&["fetch"]),
                vec![matches("carrying.Owner.clay")],
                vec![delete("carrying.Owner.clay")],
            ),
            Part::new(
                "fire",
                "[Actor]: fire it",
                seg(&["shape"]),
                Vec::new(),
                vec![insert("oven.standing")],
            ),
        ]
    }

    fn oven_end() -> (Action, Practice, Desire) {
        endeavor(
            "oven",
            3,
            "[Actor]: resolve to build an oven",
            Vec::new(),
            &chain_parts(),
        )
        .expect("the oven endeavor")
    }

    /// Two edge-free parts: nothing chains them, so both are live at once.
    fn chores_end() -> (Action, Practice, Desire) {
        endeavor(
            "chores",
            3,
            "[Actor]: set to the chores",
            Vec::new(),
            &[
                Part::new(
                    "dishes",
                    "[Actor]: wash the dishes",
                    Vec::new(),
                    Vec::new(),
                    vec![insert("washed.Owner")],
                ),
                Part::new(
                    "mop",
                    "[Actor]: mop the floor",
                    Vec::new(),
                    Vec::new(),
                    vec![insert("mopped.Owner")],
                ),
            ],
        )
        .expect("the chores endeavor")
    }

    /// A culmination (`finish`) that requires only `gather`; `flourish` hangs off
    /// the side — optional, never blocking, +w when taken.
    fn feast_end() -> (Action, Practice, Desire) {
        endeavor(
            "feast",
            3,
            "[Actor]: throw a feast",
            Vec::new(),
            &[
                Part::new(
                    "gather",
                    "[Actor]: gather the harvest",
                    Vec::new(),
                    Vec::new(),
                    vec![insert("gathered.Owner")],
                ),
                Part::new(
                    "flourish",
                    "[Actor]: garnish the platter",
                    Vec::new(),
                    Vec::new(),
                    vec![insert("garnished.Owner")],
                ),
                Part::new(
                    "finish",
                    "[Actor]: serve the feast",
                    seg(&["gather"]),
                    Vec::new(),
                    vec![insert("served.Owner")],
                ),
            ],
        )
        .expect("the feast endeavor")
    }

    /// Five parts; the culmination (`close`) carries a THRESHOLD gate — a Count
    /// over the ledger family, fired at 3 of the workers done (not 2).
    fn quota_end() -> (Action, Practice, Desire) {
        let mut parts: Vec<Part> = ["a", "b", "c", "d"]
            .into_iter()
            .map(|n| {
                Part::new(
                    n,
                    format!("[Actor]: file report {n}"),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                )
            })
            .collect();
        parts.push(Part::new(
            "close",
            "[Actor]: close the quota",
            Vec::new(),
            vec![
                subquery(
                    "Done",
                    vec!["P".to_owned()],
                    vec![matches("practice.quota.Owner.did.P")],
                ),
                count("N", "Done"),
                cmp(CmpOp::Gte, "N", "3"),
            ],
            vec![insert("quota.met.Owner")],
        ));
        endeavor("quota", 3, "[Actor]: take on the quota", Vec::new(), &parts)
            .expect("the quota endeavor")
    }

    /// The world scaffold: mia carries the pursuit; a bare "yard" practice hosts
    /// the undertake action and an idle. pat is a bystander who owns no instance.
    fn build_world(
        e: (Action, Practice, Desire),
        chars: Vec<Character>,
        extra: Vec<Outcome>,
    ) -> State {
        let (take_, prac, pursuit) = e;
        let yard = Practice::new("yard")
            .roles(["R"])
            .action(take_)
            .action(Action::new("[Actor]: Idle about"));
        let mut st = State::new();
        st.define_practices([prac, yard]).unwrap();
        st.set_characters(chars).unwrap();
        st.set_desires(vec![pursuit]).unwrap();
        st.perform_outcome(&insert("practice.yard.here")).unwrap();
        for o in &extra {
            st.perform_outcome(o).unwrap();
        }
        st
    }

    fn mia_for(pid: &str) -> Character {
        Character::new("mia").holds(format!("pursues-{pid}"))
    }

    fn oven_world() -> State {
        build_world(
            oven_end(),
            vec![mia_for("oven"), Character::new("pat")],
            vec![insert("clay.available")],
        )
    }

    fn do_act(who: &str, needle: &str, st: &mut State) {
        let found = st
            .possible_actions(who)
            .into_iter()
            .find(|ga| ga.label.contains(needle));
        match found {
            Some(ga) => st.perform_action(&ga),
            None => panic!(
                "no action for {who} matching {needle:?}; had: {:?}",
                st.possible_actions(who)
                    .into_iter()
                    .map(|ga| ga.label)
                    .collect::<Vec<_>>()
            ),
        }
    }

    fn offered(who: &str, needle: &str, st: &mut State) -> bool {
        st.possible_actions(who)
            .iter()
            .any(|ga| ga.label.contains(needle))
    }

    // Property 1: local reward carries the horizon (v24's theorem, on edges).
    // H: ProjectSpec.hs "the horizon regression: a four-part chain pursued to completion at depth 2"
    #[test]
    fn the_horizon_regression_a_four_part_chain_pursued_to_completion() {
        let mut st = oven_world();
        let mia = mia_for("oven");
        for _ in 0..5 {
            // undertake + 4 parts
            let ga = st.pick_action(2, &mia).expect("mia has no move");
            st.perform_action(&ga);
        }
        assert!(
            st.db_has("practice.oven.mia.did.fire"),
            "the chain completed to its culminating part"
        );
    }

    // Property 2: parallel parts are genuinely parallel.
    // H: ProjectSpec.hs "parallel parts: both edge-free parts are offered; the un-chosen one survives"
    #[test]
    fn parallel_parts_are_genuinely_parallel() {
        let mut st = build_world(chores_end(), vec![mia_for("chores")], Vec::new());
        do_act("mia", "set to the chores", &mut st);
        assert!(
            offered("mia", "wash the dishes", &mut st) && offered("mia", "mop the floor", &mut st),
            "both edge-free parts offered at once"
        );
        do_act("mia", "wash the dishes", &mut st);
        assert!(
            !offered("mia", "wash the dishes", &mut st),
            "the completed part is not re-offered"
        );
        assert!(
            offered("mia", "mop the floor", &mut st),
            "the un-chosen sibling is STILL offered (parts do not suppress siblings)"
        );
    }

    // Property 3: optional parts are optional.
    // H: ProjectSpec.hs "an optional part: the culmination fires with it undone, and it still pays +w"
    #[test]
    fn an_optional_part_never_blocks_and_still_pays() {
        let mut st = build_world(feast_end(), vec![mia_for("feast")], Vec::new());
        do_act("mia", "throw a feast", &mut st);
        do_act("mia", "gather the harvest", &mut st);
        assert!(
            offered("mia", "serve the feast", &mut st)
                && !st.db_has("practice.feast.mia.did.flourish"),
            "the culmination is offered though the optional part is undone"
        );
        do_act("mia", "serve the feast", &mut st);
        assert!(
            st.db_has("practice.feast.mia.did.finish"),
            "the culmination fired"
        );
        // the optional still hangs off the side, and performing it pays +w
        assert!(
            offered("mia", "garnish the platter", &mut st),
            "the optional part is still offered after the culmination"
        );
        let want = Want::new(vec![matches("practice.feast.Owner.did.P")], 3);
        let before = st.evaluate_wants(std::slice::from_ref(&want)).unwrap();
        do_act("mia", "garnish the platter", &mut st);
        let after = st.evaluate_wants(std::slice::from_ref(&want)).unwrap();
        assert_eq!(after, before + 3);
    }

    // Property 4: threshold success is authorable.
    // H: ProjectSpec.hs "a threshold culmination fires at three of five, not two"
    #[test]
    fn a_threshold_culmination_fires_at_three_of_five() {
        let mut st = build_world(quota_end(), vec![mia_for("quota")], Vec::new());
        do_act("mia", "take on the quota", &mut st);
        do_act("mia", "file report a", &mut st);
        do_act("mia", "file report b", &mut st);
        assert!(
            !offered("mia", "close the quota", &mut st),
            "two done: the threshold gate blocks the culmination"
        );
        do_act("mia", "file report c", &mut st);
        assert!(
            offered("mia", "close the quota", &mut st),
            "three done: the threshold gate opens the culmination"
        );
    }

    // Property 5: dependencies gate loudly and correctly.
    // H: ProjectSpec.hs "dependency gating: an unmet edge blocks; the met edge offers"
    #[test]
    fn dependency_gating() {
        let mut st = oven_world();
        do_act("mia", "resolve to build an oven", &mut st);
        assert!(
            offered("mia", "sweep the hearth", &mut st),
            "the edge-free root is offered"
        );
        assert!(
            !offered("mia", "fetch the clay", &mut st),
            "the dependent part is blocked while its edge is unmet"
        );
        do_act("mia", "sweep the hearth", &mut st);
        assert!(
            offered("mia", "fetch the clay", &mut st),
            "the dependent part is offered once its edge is met"
        );
    }

    // Property 6: each part once; teardown re-opens the whole endeavor.
    // H: ProjectSpec.hs "each part fires once; the subtree delete re-opens the endeavor"
    #[test]
    fn each_part_fires_once_and_the_subtree_delete_re_opens_the_endeavor() {
        let mut st = oven_world();
        do_act("mia", "resolve to build an oven", &mut st);
        do_act("mia", "sweep the hearth", &mut st);
        assert!(
            !offered("mia", "sweep the hearth", &mut st),
            "the completed part is not re-offered within the instance"
        );
        assert!(
            !offered("mia", "resolve to build an oven", &mut st),
            "and undertaking is not re-offered while the instance stands"
        );
        // teardown: a subtree delete on the instance path reaps the ledger and
        // re-opens undertake (the eat-cycle contract, in miniature)
        st.perform_outcome(&delete("practice.oven.mia")).unwrap();
        assert!(
            !st.db_has("practice.oven.mia.did.sweep"),
            "the ledger subtree is gone"
        );
        assert!(
            offered("mia", "resolve to build an oven", &mut st),
            "undertaking is offered again"
        );
    }

    // Property 7: dormancy and theory-of-mind survive.
    // H: ProjectSpec.hs "the pursuit: exact shape, dormant (zero utility, no prediction), live once undertaken"
    #[test]
    fn the_pursuit_is_dormant_until_undertaken() {
        let (_, _, pursuit) = oven_end();
        assert_eq!(
            pursuit,
            Desire::new(
                "pursues-oven",
                Want::new(vec![matches("practice.oven.Owner.did.P")], 3)
            )
        );
        let mut st = oven_world();
        // instanceless: no ledger facts, so the pursuit scores zero
        assert_eq!(
            st.evaluate_wants(std::slice::from_ref(&pursuit.want))
                .unwrap(),
            0
        );
        // dormant: pat believes mia pursues it, but with no instance the believed
        // model gains nothing from any move — no prediction.
        let mia = mia_for("oven");
        let pat = Character::new("pat");
        st.perform_outcome(&insert("pat.believes.desires.mia.pursues-oven.heard.mia"))
            .unwrap();
        assert_eq!(st.predict_move(&pat, &mia), None);
        // undertaken: the same belief now predicts the next available part.
        do_act("mia", "resolve to build an oven", &mut st);
        assert_eq!(
            st.predict_move(&pat, &mia).map(|ga| ga.label),
            Some("mia: sweep the hearth".to_owned())
        );
    }

    // H: ProjectSpec.hs "a believed pursuit predicts among PARALLEL parts (v52 T1 review M1)"
    #[test]
    fn a_believed_pursuit_predicts_among_parallel_parts() {
        // both edge-free parts are live predictions; scoring ties (+3 each), so
        // the deterministic label tiebreak picks the alphabetically-first label
        // ("mop" < "wash") — the point is that prediction needs no chain:
        // whichever part the planner would take is the prediction, and BOTH are
        // candidates.
        let mia = mia_for("chores");
        let pat = Character::new("pat");
        let mut st = build_world(
            chores_end(),
            vec![mia.clone(), Character::new("pat")],
            Vec::new(),
        );
        st.perform_outcome(&insert("pat.believes.desires.mia.pursues-chores.heard.mia"))
            .unwrap();
        do_act("mia", "set to the chores", &mut st);
        assert_eq!(
            st.predict_move(&pat, &mia).map(|ga| ga.label),
            Some("mia: mop the floor".to_owned())
        );
        // and once the tiebreak winner is done, the prediction moves to the OTHER
        // parallel part — the un-chosen sibling was a real candidate.
        do_act("mia", "mop the floor", &mut st);
        assert_eq!(
            st.predict_move(&pat, &mia).map(|ga| ga.label),
            Some("mia: wash the dishes".to_owned())
        );
    }

    // Property 8: loud construction guards, one pin each.
    // H: ProjectSpec.hs "an endeavor with no parts errors loudly"
    #[test]
    fn an_endeavor_with_no_parts_errors_loudly() {
        assert!(
            endeavor("idle", 1, "[Actor]: do nothing much", Vec::new(), &[]).is_err(),
            "an endeavor is work"
        );
    }

    // H: ProjectSpec.hs "a dotted project id errors loudly"
    #[test]
    fn a_dotted_project_id_errors_loudly() {
        assert!(
            endeavor("my.oven", 1, "[Actor]: x", Vec::new(), &chain_parts()).is_err(),
            "id must be a single path segment"
        );
    }

    // H: ProjectSpec.hs "a dotted part name errors loudly"
    #[test]
    fn a_dotted_part_name_errors_loudly() {
        assert!(
            endeavor(
                "oven",
                1,
                "[Actor]: x",
                Vec::new(),
                &[Part::new(
                    "sw.eep",
                    "[Actor]: sweep",
                    Vec::new(),
                    Vec::new(),
                    Vec::new()
                )]
            )
            .is_err(),
            "part name must be a single path segment"
        );
    }

    // H: ProjectSpec.hs "duplicate part names error loudly"
    #[test]
    fn duplicate_part_names_error_loudly() {
        assert!(
            endeavor(
                "oven",
                1,
                "[Actor]: x",
                Vec::new(),
                &[
                    Part::new("dup", "[Actor]: one", Vec::new(), Vec::new(), Vec::new()),
                    Part::new("dup", "[Actor]: two", Vec::new(), Vec::new(), Vec::new()),
                ]
            )
            .is_err(),
            "part names must be distinct"
        );
    }

    // H: ProjectSpec.hs "a dangling dependency edge errors loudly"
    #[test]
    fn a_dangling_dependency_edge_errors_loudly() {
        assert!(
            endeavor(
                "oven",
                1,
                "[Actor]: x",
                Vec::new(),
                &[Part::new(
                    "real",
                    "[Actor]: real",
                    seg(&["ghost"]),
                    Vec::new(),
                    Vec::new()
                )]
            )
            .is_err(),
            "an edge must name an actual part"
        );
    }

    // H: ProjectSpec.hs "a self-edge errors loudly (unreachable by the reachability fixpoint)"
    #[test]
    fn a_self_edge_errors_loudly() {
        let e = endeavor(
            "oven",
            1,
            "[Actor]: x",
            Vec::new(),
            &[Part::new(
                "a",
                "[Actor]: a",
                seg(&["a"]),
                Vec::new(),
                Vec::new(),
            )],
        );
        assert!(
            e.unwrap_err().to_string().contains("unreachable"),
            "a self-dependent part is unreachable"
        );
    }

    // H: ProjectSpec.hs "a two-cycle errors loudly (both participants unreachable)"
    #[test]
    fn a_two_cycle_errors_loudly() {
        let e = endeavor(
            "oven",
            1,
            "[Actor]: x",
            Vec::new(),
            &[
                Part::new("a", "[Actor]: a", seg(&["b"]), Vec::new(), Vec::new()),
                Part::new("b", "[Actor]: b", seg(&["a"]), Vec::new(), Vec::new()),
            ],
        );
        assert!(
            e.unwrap_err().to_string().contains("unreachable"),
            "a two-cycle has no edge-free root, so both are unreachable"
        );
    }

    // H: ProjectSpec.hs "a Prax-namespace variable in partNeeds errors loudly"
    #[test]
    fn a_prax_namespace_variable_in_part_needs_errors_loudly() {
        assert!(
            endeavor(
                "oven",
                1,
                "[Actor]: x",
                Vec::new(),
                &[Part::new(
                    "x",
                    "[Actor]: x",
                    Vec::new(),
                    vec![matches("PraxFoo.bar")],
                    Vec::new()
                )]
            )
            .is_err(),
            "the Prax namespace is reserved"
        );
    }
}
