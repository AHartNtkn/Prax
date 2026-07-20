//! Blackmail: the exposure instance of coercion (spec
//! `docs/specs/2026-07-17-v49-coercion.md`).
//!
//! **Blackmail is one [`crate::coerce::Coercion`], composed not wrapped.**
//! [`shakedown`] keeps its v30 signature but builds a [`Coercion`] record and
//! returns [`coerce`] of it. The four-action protocol
//! (threaten/comply/defy/expose), the motive-belief deposit, the extorted mark,
//! the standing-threat punish gate, and the punitive [`Desire`] are all the
//! primitive's; blackmail supplies only the CONTENT:
//!
//! * **trigger** — the extorter holds evidence (`Actor.believes.<pat>`) and the
//!   victim is co-present (`pat`'s first variable, retargeted through the world's
//!   co-presence template).
//! * **demand** — a debt ([`crate::debt::owe`]): the victim buys silence with a
//!   favour. Blackmail the instance is the fixed PERMANENT fiction (spec v54): it
//!   passes `compliance_lasts = None` and `threat_lasts = None`, and a world
//!   wanting expiry or serial extraction authors its own [`Coercion`].
//! * **punish** — expose: plant the evidence in a co-present non-believer
//!   (Rumor's sourced-hearsay shape).
//! * **kernel** — believers of the evidence. Authored with the PLAIN believer
//!   name `Believer` and `pat`'s own plain victim variable; [`coerce`]
//!   alpha-renames them into the `Prax` namespace (`Believer` → `PraxW`, the
//!   victim → `PraxD`). The believer is named for its role rather than `W` so that
//!   a same-named secondary evidence variable cannot merge with it under the
//!   rename.
//!
//! **The threat is credible because the extorter genuinely holds the punitive
//! desire it professes.** The returned [`Desire`] (`punishes-<id>`) must be
//! world-registered and carried; see [`crate::coerce`]'s registration contract.
//!
//! Frozen reference: `src/Prax/Blackmail.hs`.

use prax_core::error::WorldError;
use prax_core::interner::is_variable_name;
use prax_core::path::segment_names_checked;
use prax_core::query::{matches, neq, not_};
use prax_core::types::{Action, Desire, authored_pat_clash, insert};

use crate::beliefs::belief_about;
use crate::coerce::{Coercion, coerce};
use crate::debt::owe;
use crate::witness::{CoPresence, as_role};

/// `shakedown(id, copresence, pat, price, w)` builds the exposure [`Coercion`] and
/// returns [`coerce`] of it: the threaten/comply/defy/expose protocol and the
/// punitive [`Desire`] it professes.
///
/// * `id` — a single path segment scoping this shakedown's facts.
/// * `copresence` — the world's co-presence template ([`crate::witness`]).
/// * `pat` — the evidence pattern, e.g. `"stole.V.loaf"`; its FIRST variable is
///   the victim.
/// * `price` — the debt content the victim pays to buy silence.
/// * `w` — the extorter's punitive weight.
///
/// # Errors
/// [`WorldError::ReservedVarClash`] if `pat` names a SECONDARY variable called
/// `Owner` (the punitive desire's extorter, which the kernel rename exempts) or
/// `Hearer` (expose's own audience) — the two the primitive cannot catch for
/// `pat`; [`WorldError::PatternVariables`] if `pat` names no one; plus every
/// rejection [`coerce`] itself raises (which, because `pat` FLOWS INTO the kernel,
/// covers a secondary `Actor`/`E`/`Prax*`).
pub fn shakedown(
    sid: &str,
    copresence: &CoPresence,
    pat: &str,
    price: &str,
    w: i32,
) -> Result<(Desire, Vec<Action>), WorldError> {
    // UNCHECKED-SPLIT is not taken: the frozen guard splits `pat` with
    // `pathNames` (S7 design §12).
    let names = segment_names_checked(pat)?;
    // The victim is pat's first variable (loud error if pat names no one).
    let victim = names
        .iter()
        .find(|n| is_variable_name(n))
        .ok_or_else(|| WorldError::PatternVariables {
            context: "Blackmail.shakedown".to_owned(),
            pattern: pat.to_owned(),
            needs: "name someone (a threat needs a victim)".to_owned(),
        })?
        .clone();
    // The instance guard, reasoned against the primitive's own field guards. `pat`
    // is spliced into THREE frames: threaten's trigger (Actor = extorter),
    // expose's punish (Actor = extorter, Hearer = audience), and the punitive
    // kernel (Owner = extorter, the victim renamed to PraxD). The primitive
    // already forbids the `Prax` namespace on every field and — because the kernel
    // carries `pat` — its kernel guard forbids `Actor` and `E` there. What the
    // primitive CANNOT catch for `pat` is `Owner` (the kernel rename deliberately
    // exempts the mechanism interface name) and `Hearer` (expose's frame
    // legitimately binds it as the audience). Those two the instance adds.
    let secondaries: Vec<String> = names.iter().filter(|n| **n != victim).cloned().collect();
    if let Some(bad) =
        authored_pat_clash(&["Owner".to_owned(), "Hearer".to_owned()], &secondaries).first()
    {
        return Err(WorldError::ReservedVarClash {
            context: "Blackmail.shakedown: evidence pattern".to_owned(),
            var: bad.clone(),
            extra: format!(
                " -- {pat:?} names a secondary variable that would silently merge with a frame \
                 role Owner (the punitive desire's extorter) or Hearer (expose's own audience); \
                 pick a different name for anyone besides the victim"
            ),
        });
    }

    let mut trigger = vec![matches(belief_about("Actor", pat))];
    trigger.extend(as_role(&victim, copresence));

    // expose to a co-present hearer who doesn't already believe; the primitive
    // prepends the standing-threat-or-defiance availability core
    let mut punish_when = vec![matches(belief_about("Actor", pat))];
    punish_when.extend(as_role("Hearer", copresence));
    punish_when.extend([
        neq("Hearer", "Actor"),
        neq("Hearer", victim.as_str()),
        not_(belief_about("Hearer", pat)),
    ]);

    let blackmail = Coercion {
        id: sid.to_owned(),
        victim: victim.clone(),
        // the extorter holds evidence, the victim is co-present
        trigger,
        threaten_label: format!("[Actor]: threaten [{victim}] with what you know"),
        demand_label: "[Actor]: buy [E]'s silence".to_owned(),
        demand: owe("E", "Actor", price)?,
        punish_label: format!("[Actor]: expose [{victim}] to [Hearer]"),
        punish_when,
        // Rumor's sourced-hearsay plant
        punish_outs: vec![insert(format!(
            "{}.heard.Actor",
            belief_about("Hearer", pat)
        ))],
        // believers of the evidence, authored plain; `coerce` lifts Believer ->
        // PraxW and the victim -> PraxD. LOAD-BEARING for the instance guard (v49
        // review M1): `pat` flowing into the kernel is what makes the primitive's
        // kernel guard reject a secondary evidence variable named Actor or E — the
        // instance list above adds only Owner/Hearer. If `pat` ever stops flowing
        // here, the Actor/E coverage goes with it.
        kernel: vec![matches(belief_about("Believer", pat))],
        weight: w,
        // Blackmail the INSTANCE is a fixed fiction: permanent threats, permanent
        // purchase (spec v54). A world wanting an expiring or serial blackmail
        // authors its own `Coercion` directly — the primitive, not the instance,
        // is the expressiveness home.
        threat_lasts: None,
        compliance_lasts: None,
    };
    coerce(&blackmail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_core::engine::{GroundedAction, State};
    use prax_core::types::{Action, Character, Practice, Want};
    use prax_core::vocab_consts::obligation_path;

    use crate::debt::owes;
    use crate::deceit::conceal;
    use crate::minds::conventional;
    use crate::persona::{Trait, bearing, cast, persona_vocabulary, transparent};

    // H: BlackmailSpec.hs "Prax.Blackmail"
    //
    // The frozen `Prax.BlackmailSpec`, re-expressed against the Rust engine.
    //
    // The tale ported from the session probe: mel (extortionist, holds evidence)
    // threatens vic (thief, fears exposure) with what mel saw. wit (and,
    // optionally, wit2) are onlookers at court. The audience arity is the whole
    // mechanic: two heads of exposure risk make compliance rational, one makes
    // defiance rational — pinned exactly at the probe's own numbers (never tuned).

    fn together() -> CoPresence {
        vec![matches("at.Actor!P"), matches("at.Witness!P")]
    }

    fn shakedown_parts() -> (Desire, Vec<Action>) {
        shakedown("defiance", &together(), "took.V.gem", "favor", 6).expect("the shipped shakedown")
    }

    fn wait_action() -> Action {
        Action::new("[Actor]: wait").when([matches("at.Actor!P")])
    }

    fn yard_practice() -> Practice {
        let (_, acts) = shakedown_parts();
        let mut p = Practice::new("yard").roles(["R"]);
        for a in acts {
            p = p.action(a);
        }
        p.action(wait_action())
    }

    fn fears_scandal() -> Desire {
        Desire::new(
            "fears-scandal",
            Want::new(vec![matches("W.believes.took.Owner.gem")], -10),
        )
    }

    /// `two_onlookers` toggles whether wit2 shares mel and vic's court (versus off
    /// at the mill) — the sole variable between the two pinned scenarios.
    fn mk_world(two_onlookers: bool) -> State {
        let mut st = State::new();
        st.define_practices([yard_practice()]).unwrap();
        st.set_characters(vec![
            Character::new("mel")
                .want(Want::new(vec![owes("mel", "vic", "favor").unwrap()], 8))
                .holds("punishes-defiance"),
            Character::new("vic")
                .want(conceal("took.vic.gem", 12).unwrap())
                .want(Want::new(vec![matches("debt.C.vic.favor")], -4))
                .holds("fears-scandal"),
            Character::new("wit"),
            Character::new("wit2"),
        ])
        .unwrap();
        for o in [
            insert("practice.yard.here"),
            insert("at.mel!court"),
            insert("at.vic!court"),
            insert("at.wit!court"),
            insert(format!(
                "at.wit2!{}",
                if two_onlookers { "court" } else { "mill" }
            )),
            insert("mel.believes.took.vic.gem.seen"),
        ] {
            st.perform_outcome(&o).expect("blackmail setup");
        }
        st.set_axioms(vec![conventional()]).unwrap();
        st.set_desires(vec![shakedown_parts().0, fears_scandal()])
            .unwrap();
        st
    }

    fn member(st: &State, n: &str) -> Character {
        st.characters()
            .iter()
            .find(|c| c.name == n)
            .unwrap_or_else(|| panic!("no such character: {n}"))
            .clone()
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

    fn score_of(scores: &[(GroundedAction, f64)], needle: &str) -> f64 {
        scores
            .iter()
            .find(|(ga, _)| ga.label.contains(needle))
            .map(|(_, s)| *s)
            .unwrap_or_else(|| {
                panic!(
                    "no scored action matching {needle:?}; had: {:?}",
                    scores
                        .iter()
                        .map(|(ga, _)| ga.label.clone())
                        .collect::<Vec<_>>()
                )
            })
    }

    /// Trait-priced deterrence fixture (v25 composition): hal and rex share the
    /// identical extortion motive; hal is scrupulous (his own extorted marks cost
    /// him), rex is his unprincipled twin.
    fn scrupulous() -> Trait {
        // more than the discounted future value of the debt this deters, less
        // than everything: a real cost, not a prohibition. Priced on the SUBTREE
        // mark `Owner.extorted.vic` (v49).
        Trait::new(
            "scrupulous",
            vec![Desire::new(
                "qualms",
                Want::new(vec![matches("Owner.extorted.vic")], -20),
            )],
        )
    }

    fn deterrence_world() -> State {
        let (roster, persona_facts) = cast(
            &[scrupulous()],
            vec![
                (
                    Character::new("hal")
                        .want(Want::new(vec![owes("hal", "vic", "favor").unwrap()], 8))
                        .holds("punishes-defiance"),
                    vec![scrupulous()],
                ),
                (
                    Character::new("rex")
                        .want(Want::new(vec![owes("rex", "vic", "favor").unwrap()], 8))
                        .holds("punishes-defiance"),
                    Vec::new(),
                ),
                (Character::new("vic"), Vec::new()),
                (Character::new("wit"), Vec::new()),
            ],
        )
        .expect("the deterrence roster");
        let mut st = State::new();
        st.define_practices([yard_practice()]).unwrap();
        st.set_characters(roster).unwrap();
        st.set_axioms(vec![conventional(), transparent()]).unwrap();
        for o in persona_facts.iter().chain(
            [
                insert("practice.yard.here"),
                insert("at.hal!court"),
                insert("at.rex!court"),
                insert("at.vic!court"),
                insert("at.wit!court"),
                insert("hal.believes.took.vic.gem.seen"),
                insert("rex.believes.took.vic.gem.seen"),
            ]
            .iter(),
        ) {
            st.perform_outcome(o).expect("deterrence setup");
        }
        let mut desires = vec![shakedown_parts().0];
        desires.extend(persona_vocabulary(&[scrupulous()]).unwrap());
        st.set_desires(desires).unwrap();
        st
    }

    // H: BlackmailSpec.hs "guards"
    // H: BlackmailSpec.hs "a dotted id errors loudly"
    #[test]
    fn a_dotted_id_errors_loudly() {
        assert!(
            shakedown("de.f", &together(), "took.V.gem", "favor", 6).is_err(),
            "a dotted id is an error"
        );
    }

    // H: BlackmailSpec.hs "an evidence pattern naming no one errors loudly"
    #[test]
    fn an_evidence_pattern_naming_no_one_errors_loudly() {
        assert!(
            shakedown("defiance", &together(), "somethinghappened", "favor", 6).is_err(),
            "a threat must be about someone"
        );
    }

    // H: BlackmailSpec.hs "the usability win: a secondary evidence variable named D or W no longer collides (v40 moved the punitive desire's own machinery to the Prax namespace)"
    #[test]
    fn a_secondary_variable_named_d_or_w_no_longer_collides() {
        assert!(
            shakedown("x", &Vec::new(), "took.V.from.D", "favor", 1).is_ok(),
            "D is an unremarkable secondary variable post-v40"
        );
        assert!(
            shakedown("x", &Vec::new(), "took.V.by.W", "favor", 1).is_ok(),
            "W is an unremarkable secondary variable post-v40"
        );
    }

    // H: BlackmailSpec.hs "a secondary evidence variable authoring the Prax namespace collides with the punitive desire's own machinery"
    #[test]
    fn a_secondary_variable_in_the_prax_namespace_collides() {
        assert!(
            shakedown("x", &Vec::new(), "took.V.from.PraxD", "favor", 1).is_err(),
            "PraxD is reserved for the punitive desire's own victim variable"
        );
        assert!(
            shakedown("x", &Vec::new(), "took.V.by.PraxW", "favor", 1).is_err(),
            "PraxW is reserved for the punitive desire's own believer variable"
        );
    }

    // H: BlackmailSpec.hs "a secondary evidence variable named Actor or E is rejected (via the kernel flow — v49 review M2)"
    #[test]
    fn a_secondary_variable_named_actor_or_e_is_rejected_via_the_kernel_flow() {
        assert!(
            shakedown("x", &Vec::new(), "took.V.by.E", "favor", 1).is_err(),
            "E is the comply/defy frame's extorter; a secondary named E merges"
        );
        assert!(
            shakedown("x", &Vec::new(), "took.V.by.Actor", "favor", 1).is_err(),
            "Actor is every generated frame's actor; a secondary named Actor merges"
        );
    }

    // H: BlackmailSpec.hs "a secondary evidence variable named Owner collides with the desire's Owner"
    #[test]
    fn a_secondary_variable_named_owner_collides() {
        assert!(
            shakedown("x", &Vec::new(), "took.V.for.Owner", "favor", 1).is_err(),
            "Owner is reserved for the desire's own Owner-templated variable"
        );
    }

    // H: BlackmailSpec.hs "a secondary evidence variable named Hearer collides with expose's own hearer"
    #[test]
    fn a_secondary_variable_named_hearer_collides() {
        assert!(
            shakedown("x", &Vec::new(), "took.V.before.Hearer", "favor", 1).is_err(),
            "Hearer is reserved for expose's (and gossip's) own hearer variable"
        );
    }

    // H: BlackmailSpec.hs "a secondary evidence variable named Actor collides with the generated actions' own actor"
    #[test]
    fn a_secondary_variable_named_actor_collides() {
        assert!(
            shakedown("x", &Vec::new(), "took.V.with.Actor", "favor", 1).is_err(),
            "Actor is reserved for the generated actions' own actor variable"
        );
    }

    // H: BlackmailSpec.hs "threaten: the extorter is motivated to threaten, and the threat deposits"
    // H: BlackmailSpec.hs "the extorter threatens at depth 2, holding the punitive desire via charDesires"
    #[test]
    fn the_extorter_threatens_at_depth_two() {
        let mut st = mk_world(true);
        let mel = member(&st, "mel");
        assert_eq!(
            st.pick_action(2, &mel).map(|ga| ga.label),
            Some("mel: threaten vic with what you know".to_owned())
        );
    }

    // H: BlackmailSpec.hs "threatening deposits the ordinary fact, the motive-belief, and the extorted mark"
    #[test]
    fn threatening_deposits_the_fact_the_motive_belief_and_the_mark() {
        let mut st = mk_world(true);
        do_act("mel", "threaten vic", &mut st);
        assert!(
            st.db_has("threatened.defiance.mel.vic"),
            "the threatened fact"
        );
        assert!(
            st.db_has("vic.believes.desires.mel.punishes-defiance.heard.mel"),
            "the motive-belief deposit: vic hears mel's professed punitive intent"
        );
        assert!(
            st.db_has("mel.extorted.vic.defiance"),
            "the extorted mark: mel's own memory of having extorted vic, tailed \
             by the coercion id (v49)"
        );
    }

    // H: BlackmailSpec.hs "two onlookers: the victim complies"
    // H: BlackmailSpec.hs "with two heads of exposure risk, compliance dominates waiting and defiance (property 2, ordering)"
    #[test]
    fn with_two_onlookers_compliance_dominates() {
        let mut st = mk_world(true);
        do_act("mel", "threaten vic", &mut st);
        let vic = member(&st, "vic");
        let scores = st.score_actions(2, &vic);
        let comply = score_of(&scores, "vic: buy mel's silence");
        let wait_ = score_of(&scores, "vic: wait");
        let defy = score_of(&scores, "vic: defy mel");
        // The v49 contract is the ORDERING, not the decimals.
        assert!(
            comply > wait_ && wait_ > defy,
            "comply must dominate the ordering; had comply={comply} wait={wait_} defy={defy}"
        );
        assert_eq!(
            st.pick_action(2, &vic).map(|ga| ga.label),
            Some("vic: buy mel's silence".to_owned())
        );
    }

    // H: BlackmailSpec.hs "complying leaves a debt and its obligation, and the threat is gone"
    #[test]
    fn complying_leaves_a_debt_and_its_obligation() {
        let mut st = mk_world(true);
        do_act("mel", "threaten vic", &mut st);
        do_act("vic", "buy mel's silence", &mut st);
        assert!(st.db_has("debt.mel.vic.favor"), "the debt fact");
        assert!(
            st.db_has(&obligation_path("vic", "favor")),
            "the obligation Debt composes it from"
        );
        assert!(
            !st.db_has("threatened.defiance.mel.vic"),
            "the threat is gone"
        );
        assert!(
            !st.possible_actions("mel")
                .iter()
                .any(|ga| ga.label.contains("expose vic")),
            "expose is no longer offered against vic (no standing threat, no defiance)"
        );
    }

    // H: BlackmailSpec.hs "a renewed threat after compliance extracts nothing (property 3 at the instance — v49 review M3)"
    #[test]
    fn a_renewed_threat_after_compliance_extracts_nothing() {
        // Property 3's primitive pin lives in CoerceSpec; this is the same law
        // observed through blackmail's own shapes.
        let mut st = mk_world(true);
        do_act("mel", "threaten vic", &mut st);
        do_act("vic", "buy mel's silence", &mut st);
        do_act("mel", "threaten vic", &mut st);
        assert!(
            st.db_has("complied.defiance.mel.vic"),
            "the complied marker stands"
        );
        assert!(
            !st.possible_actions("vic")
                .iter()
                .any(|ga| ga.label.contains("buy mel's silence")),
            "buy is not offered again under the renewed threat"
        );
    }

    // H: BlackmailSpec.hs "one onlooker: the victim rationally defies (both sides of the arithmetic)"
    // H: BlackmailSpec.hs "with a single head of exposure risk, defiance ties waiting and both beat compliance (property 2, ordering)"
    #[test]
    fn with_one_onlooker_defiance_ties_waiting_and_both_beat_compliance() {
        let mut st = mk_world(false);
        do_act("mel", "threaten vic", &mut st);
        let vic = member(&st, "vic");
        let scores = st.score_actions(2, &vic);
        let comply = score_of(&scores, "vic: buy mel's silence");
        let wait_ = score_of(&scores, "vic: wait");
        let defy = score_of(&scores, "vic: defy mel");
        assert_eq!(defy, wait_);
        assert!(
            defy > comply,
            "defy/wait must dominate comply; had defy={defy} comply={comply}"
        );
        assert_eq!(
            st.pick_action(2, &vic).map(|ga| ga.label),
            Some("vic: defy mel".to_owned())
        );
    }

    // H: BlackmailSpec.hs "the stall-tie: scoreActions gives wait and defy the SAME score under a standing threat"
    #[test]
    fn the_stall_tie() {
        let mut st = mk_world(false);
        do_act("mel", "threaten vic", &mut st);
        let vic = member(&st, "vic");
        let scores = st.score_actions(2, &vic);
        assert_eq!(
            score_of(&scores, "vic: wait"),
            score_of(&scores, "vic: defy mel")
        );
    }

    // H: BlackmailSpec.hs "defiance leaves the defied fact and clears the threat"
    #[test]
    fn defiance_leaves_the_defied_fact_and_clears_the_threat() {
        let mut st = mk_world(false);
        do_act("mel", "threaten vic", &mut st);
        do_act("vic", "defy mel", &mut st);
        assert!(st.db_has("defied.defiance.vic.mel"), "the defied fact");
        assert!(
            !st.db_has("threatened.defiance.mel.vic"),
            "the threat is gone"
        );
    }

    // H: BlackmailSpec.hs "the victim's model predicts mel's exposure after defiance"
    #[test]
    fn the_victims_model_predicts_the_exposure_after_defiance() {
        let mut st = mk_world(false);
        do_act("mel", "threaten vic", &mut st);
        do_act("vic", "defy mel", &mut st);
        let vic = member(&st, "vic");
        let mel = member(&st, "mel");
        assert_eq!(
            st.predict_move(&vic, &mel).map(|ga| ga.label),
            Some("mel: expose vic to wit".to_owned())
        );
    }

    // H: BlackmailSpec.hs "trait-priced deterrence (v25 composition)"
    // H: BlackmailSpec.hs "a scrupulous extorter declines; an unprincipled twin with the same motive still threatens"
    #[test]
    fn a_scrupulous_extorter_declines_and_the_twin_threatens() {
        let mut st = deterrence_world();
        let hal = member(&st, "hal");
        let rex = member(&st, "rex");
        assert_eq!(
            st.pick_action(2, &hal).map(|ga| ga.label),
            Some("hal: wait".to_owned())
        );
        assert_eq!(
            st.pick_action(2, &rex).map(|ga| ga.label),
            Some("rex: threaten vic with what you know".to_owned())
        );
    }

    // H: BlackmailSpec.hs "bearing endows the extorter with the qualm by name"
    #[test]
    fn bearing_endows_the_extorter_with_the_qualm_by_name() {
        assert_eq!(
            bearing(&scrupulous(), Character::new("zed")).desires,
            vec!["qualms".to_owned()]
        );
    }
}
