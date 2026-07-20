//! Confession & absolution (spec `docs/specs/2026-07-12-v32-confession.md`).
//! Marks convert, never delete; confession self-incriminates through the ordinary
//! hearsay channel; absolution is a refusable second-party grant; an absolver's
//! patience is what they KNOW (per-regarder, permanent by memory, fed by gossip
//! and confession alike).
//!
//! Frozen reference: `src/Prax/Confession.hs`. [`incorrigible`] is an AXIOM
//! builder whose head name is derived by STRING SURGERY over the deed pattern's
//! own variables (S7 design §3.4) — the `<name>0` witness-dummy convention is
//! what keeps `Count` counting instances rather than re-checking the one already
//! bound, and a one-character drift there renders plausibly while making the
//! threshold unfireable.

use prax_core::error::WorldError;
use prax_core::interner::is_variable_name;
use prax_core::path::{segment_names_checked, segment_tokens_checked, tokens_to_sentence};
use prax_core::query::{CmpOp, cmp, count, matches, neq, not_, subquery};
use prax_core::types::{Action, Axiom, authored_pat_clash, delete, insert};

use crate::beliefs::belief_about;
use crate::witness::{CoPresence, as_role};

/// The frozen `segOk`: a nonempty name carrying no separator.
fn seg_ok(n: &str) -> bool {
    !n.is_empty() && !n.contains(['.', '!'])
}

fn not_a_segment(context: &str, name: &str) -> WorldError {
    WorldError::NotASinglePathSegment {
        context: context.to_owned(),
        name: name.to_owned(),
    }
}

/// Confess ONE deed (one mark binding) to a co-present hearer. The lied-mark is
/// the precondition and it CONVERTS — a deed can be confessed once; further
/// hearers learn by gossip. `H` is the mark's own original-hearer slot and is
/// reserved in the MARK pattern. The discharge `verb` names what the mark becomes
/// (shipped worlds pass `"confessed"`).
///
/// The MARK pattern and the DEPOSIT pattern are deliberately separate: a
/// content-shaped mark (e.g. `"stole.C.loaf"`, priceable by conscience) and an
/// act-shaped standing (e.g. `"whispered.V.H"`, what confessing a LIE actually
/// reveals) cannot share one pattern. The deposit is grounded from the mark's own
/// bindings only (`Actor`, `H`, and the mark pattern's own variables) — any other
/// deposit variable would insert a variable-bearing fact, so it is a loud error.
///
/// # Errors
/// In the frozen guard order: [`WorldError::NotASinglePathSegment`] for a
/// punctuated `kind` or `verb`; [`WorldError::ReservedVarClash`] for a mark
/// pattern naming `H`/`Hearer`/`Actor` or a deposit pattern naming `Hearer` (or
/// either naming the `Prax` namespace); [`WorldError::PatternVariables`] for a
/// deposit variable the mark does not ground; [`WorldError::TrailingOperator`] on
/// a malformed pattern.
pub fn confess(
    kind: &str,
    verb: &str,
    copresence: &CoPresence,
    mark_pat: &str,
    deposit_pat: &str,
    label: &str,
) -> Result<Action, WorldError> {
    if !seg_ok(kind) {
        return Err(not_a_segment("Confession.confess: mark kind", kind));
    }
    if !seg_ok(verb) {
        return Err(not_a_segment("Confession.confess: discharge verb", verb));
    }
    // UNCHECKED-SPLIT is not taken: the frozen guards split both patterns with
    // `pathNames` (S7 design §12).
    let mark_names = segment_names_checked(mark_pat)?;
    if let Some(v) = authored_pat_clash(
        &["H".to_owned(), "Hearer".to_owned(), "Actor".to_owned()],
        &mark_names,
    )
    .first()
    {
        return Err(WorldError::ReservedVarClash {
            context: "Confession.confess: mark pattern".to_owned(),
            var: v.clone(),
            extra: format!(
                " -- {mark_pat:?} reserves it (the mark's hearer slot / the action's own roles, \
                 or the Prax namespace)"
            ),
        });
    }
    let deposit_names = segment_names_checked(deposit_pat)?;
    if let Some(v) = authored_pat_clash(&["Hearer".to_owned()], &deposit_names).first() {
        return Err(WorldError::ReservedVarClash {
            context: "Confession.confess: deposit pattern".to_owned(),
            var: v.clone(),
            extra: format!(
                " -- {deposit_pat:?} reserves it (the confession's own audience role, or the \
                 Prax namespace)"
            ),
        });
    }
    let mut grounded_vars = vec!["Actor".to_owned(), "H".to_owned()];
    grounded_vars.extend(mark_names.iter().filter(|n| is_variable_name(n)).cloned());
    if let Some(v) = deposit_names
        .iter()
        .filter(|n| is_variable_name(n))
        .find(|n| !grounded_vars.contains(n))
    {
        return Err(WorldError::PatternVariables {
            context: "Confession.confess: deposit pattern".to_owned(),
            pattern: deposit_pat.to_owned(),
            needs: format!(
                "ground {v:?} from the mark (only Actor, H, and {mark_pat:?}'s own variables are \
                 available) -- an ungroundable deposit would insert a variable-bearing fact"
            ),
        });
    }

    let lied_path = format!("Actor.{kind}.H.{mark_pat}");
    let confessed_path = format!("Actor.{verb}.H.{mark_pat}");
    let mut conds = vec![matches(&lied_path)];
    conds.extend(as_role("Hearer", copresence));
    conds.push(neq("Hearer", "Actor"));
    Ok(Action::new(label).when(conds).then([
        delete(&lied_path),
        insert(confessed_path),
        insert(format!(
            "{}.heard.Actor",
            belief_about("Hearer", deposit_pat)
        )),
    ]))
}

/// Grant absolution: insert the world's defeater for a deed confessed TO YOU (the
/// belief must be heard from its own doer — gossip does not qualify), unless your
/// patience is spent (the incorrigibility regard).
///
/// # Errors
/// [`WorldError::NotASinglePathSegment`] for a punctuated defeater or label;
/// [`WorldError::ReservedVarClash`] for an event pattern naming `Actor` (or the
/// `Prax` namespace); [`WorldError::PatternVariables`] if the pattern names no one
/// (the FIRST variable is the confessor); [`WorldError::TrailingOperator`] on a
/// malformed pattern.
pub fn absolve(
    defeater: &str,
    pat: &str,
    inc_label: &str,
    label: &str,
) -> Result<Action, WorldError> {
    if !seg_ok(defeater) || !seg_ok(inc_label) {
        return Err(not_a_segment(
            "Confession.absolve: defeater/label",
            if seg_ok(defeater) {
                inc_label
            } else {
                defeater
            },
        ));
    }
    let names = segment_names_checked(pat)?;
    if let Some(v) = authored_pat_clash(&["Actor".to_owned()], &names).first() {
        return Err(WorldError::ReservedVarClash {
            context: "Confession.absolve: event pattern".to_owned(),
            var: v.clone(),
            extra: format!(" -- {pat:?} reserves it"),
        });
    }
    let confessor =
        names
            .iter()
            .find(|n| is_variable_name(n))
            .ok_or_else(|| WorldError::PatternVariables {
                context: "Confession.absolve: event pattern".to_owned(),
                pattern: pat.to_owned(),
                needs: "name someone (the FIRST variable is the confessor)".to_owned(),
            })?;
    Ok(Action::new(label)
        .when([
            matches(format!("{}.heard.{confessor}", belief_about("Actor", pat))),
            neq("Actor", confessor.as_str()),
            not_(format!("regards.Actor.{confessor}.{inc_label}")),
            not_(format!("{defeater}.{confessor}")),
        ])
        .then([insert(format!("{defeater}.{confessor}"))]))
}

/// Patience as knowledge: the believer regards the offender `label` once they
/// believe at least `k` distinct instances of the deed — however they learned
/// them.
///
/// Mirrors [`crate::repute::notoriety`]'s `Count` idiom pointed inward, with one
/// correction the mirroring forces: `notoriety` keeps its outer existence check
/// and its counting `Subquery` on DIFFERENT names for the counted role
/// (`W0` vs `W`). A literal transliteration that instead points the pattern's own
/// deed variables (`rest`) at both the outer `Match` and the inner `Subquery`
/// verbatim binds `rest` before the subquery ever runs, so the subquery re-checks
/// the single already-known instance and `Count` is always 1 — the threshold could
/// never fire for any `k > 1`. The outer `Match` here instead witnesses existence
/// over DUMMY names for `rest` (each suffixed `0`) so they stay free entering the
/// `Subquery`, which alone does the real per-instance counting under their true
/// names. The axiom's own believer/subquery-set/count variables live in the `Prax`
/// namespace (v40).
///
/// # Errors
/// [`WorldError::NotASinglePathSegment`] for a punctuated label;
/// [`WorldError::ReservedVarClash`] for a pattern naming the `Prax` namespace;
/// [`WorldError::PatternVariables`] for a pattern naming no offender, a
/// single-variable pattern (no deed variables to count), or a deed variable
/// colliding with the outer witness-naming convention;
/// [`WorldError::TrailingOperator`] on a malformed pattern.
pub fn incorrigible(pat: &str, k: i32, label: &str) -> Result<Axiom, WorldError> {
    if !seg_ok(label) {
        return Err(not_a_segment("Confession.incorrigible: label", label));
    }
    let tokens = segment_tokens_checked(pat)?;
    let names: Vec<String> = tokens.iter().map(|(n, _)| n.clone()).collect();
    if let Some(v) = authored_pat_clash(&[], &names).first() {
        return Err(WorldError::ReservedVarClash {
            context: "Confession.incorrigible: pattern".to_owned(),
            var: v.clone(),
            extra: format!(
                " -- {pat:?} reserves it; the Prax namespace is reserved for the axiom's own \
                 machinery"
            ),
        });
    }
    let vars: Vec<String> = names
        .iter()
        .filter(|n| is_variable_name(n))
        .cloned()
        .collect();
    let (offender, rest) = match vars.split_first() {
        None => {
            return Err(WorldError::PatternVariables {
                context: "Confession.incorrigible".to_owned(),
                pattern: pat.to_owned(),
                needs: "name an offender".to_owned(),
            });
        }
        Some((_, [])) => {
            return Err(WorldError::PatternVariables {
                context: "Confession.incorrigible".to_owned(),
                pattern: pat.to_owned(),
                needs: "have deed variables to count (a single-variable pattern admits only one \
                        possible instance; a k > 1 threshold could never fire, and k <= 1 needs \
                        no threshold at all)"
                    .to_owned(),
            });
        }
        Some((offender, rest)) => (offender.clone(), rest.to_vec()),
    };
    let witness_names: Vec<String> = rest.iter().map(|r| format!("{r}0")).collect();
    if let Some(v) = witness_names
        .iter()
        .find(|d| **d == offender || rest.contains(d))
    {
        return Err(WorldError::PatternVariables {
            context: "Confession.incorrigible".to_owned(),
            pattern: pat.to_owned(),
            needs: format!(
                "not name {v:?}: it collides with the outer witness-naming convention (a deed \
                 variable named <name>0 shadowing another variable named <name>)"
            ),
        });
    }
    let witness_pat = tokens_to_sentence(
        &tokens
            .iter()
            .map(|(name, op)| {
                (
                    if rest.contains(name) {
                        format!("{name}0")
                    } else {
                        name.clone()
                    },
                    *op,
                )
            })
            .collect::<Vec<_>>(),
    );
    Ok(Axiom::new(
        vec![
            matches(format!("PraxW.believes.{witness_pat}")),
            subquery(
                "PraxDs",
                rest,
                vec![matches(format!("PraxW.believes.{pat}"))],
            ),
            count("PraxN", "PraxDs"),
            cmp(CmpOp::Gte, "PraxN", k.to_string()),
        ],
        [format!("regards.PraxW.{offender}.{label}")],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_core::engine::{GroundedAction, State};
    use prax_core::types::{Action, Character, Desire, Practice, Want};

    use crate::blackmail::shakedown;
    use crate::deceit::conceal;
    use crate::persona::{Trait, cast, persona_vocabulary};

    // H: ConfessionSpec.hs "Prax.Confession"
    //
    // The frozen `Prax.ConfessionSpec`, re-expressed against the Rust engine.

    fn together() -> CoPresence {
        vec![matches("at.Actor!P"), matches("at.Witness!P")]
    }

    /// Shared deed vocabulary across confess/absolve/incorrigible: the FIRST
    /// variable (Doer) is both the deed's subject and its confessor — confession
    /// is self-incriminating by design (spec point 2).
    const PAT: &str = "wronged.Doer.Victim";

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

    fn offered(who: &str, needle: &str, st: &mut State) -> bool {
        st.possible_actions(who)
            .iter()
            .any(|ga| ga.label.contains(needle))
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

    // ---- Group 1: conversion mechanics + trait-priced relief (v25) ----------

    /// A trait pricing your OWN "lied" marks at -6 (guilt) and their "confessed"
    /// form at 0 (a clean conscience, not a bonus) — the residue the conversion is
    /// FOR.
    fn conscience() -> Trait {
        Trait::new(
            "conscience",
            vec![
                Desire::new(
                    "guilt",
                    Want::new(vec![matches("Owner.lied.H.wronged.Owner.Victim")], -6),
                ),
                Desire::new(
                    "clearConscience",
                    Want::new(vec![matches("Owner.confessed.H.wronged.Owner.Victim")], 0),
                ),
            ],
        )
    }

    fn confess_act() -> Action {
        confess(
            "lied",
            "confessed",
            &together(),
            PAT,
            PAT,
            "[Actor]: confess to [Hearer] about wronging [Victim]",
        )
        .expect("the shipped confess fixture")
    }

    fn conv_world() -> State {
        let (roster, persona_facts) = cast(
            &[conscience()],
            vec![
                (Character::new("bob"), vec![conscience()]),
                (Character::new("fay"), Vec::new()),
            ],
        )
        .expect("the conversion roster");
        let mut st = State::new();
        st.define_practices([Practice::new("confessional")
            .roles(["R"])
            .action(confess_act())])
            .unwrap();
        st.set_characters(roster).unwrap();
        st.set_desires(persona_vocabulary(&[conscience()]).unwrap())
            .unwrap();
        for o in persona_facts.iter().chain(
            [
                insert("practice.confessional.here"),
                insert("at.bob!yard"),
                insert("at.fay!yard"),
                // two DISTINCT deeds, same original hearer "edda": bob wronged
                // carol, and separately wronged dana. Confessing one must not
                // touch the other.
                insert("bob.lied.edda.wronged.bob.carol"),
                insert("bob.lied.edda.wronged.bob.dana"),
            ]
            .iter(),
        ) {
            st.perform_outcome(o).expect("conversion setup");
        }
        st
    }

    // H: ConfessionSpec.hs "conversion mechanics"
    // H: ConfessionSpec.hs "confessing converts exactly the confessed deed's mark; a second lied-mark survives"
    #[test]
    fn confessing_converts_exactly_the_confessed_deeds_mark() {
        let mut st = conv_world();
        do_act("bob", "confess to fay about wronging carol", &mut st);
        assert!(
            st.db_has("bob.confessed.edda.wronged.bob.carol"),
            "the carol mark converted to confessed"
        );
        assert!(
            !st.db_has("bob.lied.edda.wronged.bob.carol"),
            "the carol lied-mark is gone"
        );
        assert!(
            st.db_has("bob.lied.edda.wronged.bob.dana"),
            "the dana lied-mark survives untouched"
        );
    }

    // H: ConfessionSpec.hs "confession deposits the hearer's sourced belief (the ordinary hearsay channel)"
    #[test]
    fn confession_deposits_the_hearers_sourced_belief() {
        let mut st = conv_world();
        do_act("bob", "confess to fay about wronging carol", &mut st);
        assert!(
            st.db_has("fay.believes.wronged.bob.carol.heard.bob"),
            "fay heard it from bob"
        );
    }

    // H: ConfessionSpec.hs "the converted deed is not re-offered; the surviving one still is"
    #[test]
    fn the_converted_deed_is_not_re_offered() {
        let mut st = conv_world();
        do_act("bob", "confess to fay about wronging carol", &mut st);
        assert!(
            !offered("bob", "wronging carol", &mut st),
            "no more confessing about carol (the mark is gone)"
        );
        assert!(
            offered("bob", "wronging dana", &mut st),
            "confessing about dana remains offered"
        );
    }

    // H: ConfessionSpec.hs "a trait pricing lied at -6 and confessed at 0: the relief in evaluate/selfWants"
    #[test]
    fn the_relief_in_evaluate_over_self_wants() {
        let mut st = conv_world();
        let bob = member(&st, "bob");
        // measured directly (v25's own idiom): two lied marks cost -6 each.
        assert_eq!(st.evaluate_self_wants(&bob), -12);
        do_act("bob", "confess to fay about wronging carol", &mut st);
        // one converted (0 residue) + one still lied (-6): -6, not -12.
        assert_eq!(st.evaluate_self_wants(&bob), -6);
    }

    // ---- Group 2: absolution ------------------------------------------------

    fn absolve_act() -> Action {
        absolve("recanted", PAT, "fedUp", "[Actor]: absolve [Doer]")
            .expect("the shipped absolve fixture")
    }

    fn absolve_world() -> State {
        let mut st = State::new();
        st.define_practices([Practice::new("confessional")
            .roles(["R"])
            .action(absolve_act())])
            .unwrap();
        for o in [
            insert("practice.confessional.here"),
            // gwen was confessed to directly (sourced from cody, the doer).
            insert("gwen.believes.wronged.cody.carol.heard.cody"),
            // hank was ALSO confessed to, but his patience is already spent.
            insert("hank.believes.wronged.cody.carol.heard.cody"),
            insert("regards.hank.cody.fedUp"),
            // ivy only heard it from a third party (gossip) — never confessed TO
            // her, so the doer-sourced gate must block her.
            insert("ivy.believes.wronged.cody.carol.heard.jill"),
        ] {
            st.perform_outcome(&o).expect("absolution setup");
        }
        st.set_characters(vec![
            Character::new("gwen"), // fresh: will grant
            Character::new("hank"), // fed up: refuses
            Character::new("ivy"),  // only gossip-sourced: never qualifies
            Character::new("cody"), // the confessor
        ])
        .unwrap();
        st
    }

    // H: ConfessionSpec.hs "absolution: grant, refusal, gossip gate, double-grant"
    // H: ConfessionSpec.hs "granting inserts the world's defeater"
    #[test]
    fn granting_inserts_the_worlds_defeater() {
        let mut st = absolve_world();
        do_act("gwen", "absolve cody", &mut st);
        assert!(st.db_has("recanted.cody"), "the defeater");
    }

    // H: ConfessionSpec.hs "a planted incorrigibility regard blocks the affordance (refusal)"
    #[test]
    fn a_planted_incorrigibility_regard_blocks_the_affordance() {
        let mut st = absolve_world();
        assert!(
            !offered("hank", "absolve cody", &mut st),
            "hank, fed up, has no absolve action against cody"
        );
    }

    // H: ConfessionSpec.hs "gossip-sourced belief does not qualify (heard from a non-doer)"
    #[test]
    fn gossip_sourced_belief_does_not_qualify() {
        let mut st = absolve_world();
        assert!(
            !offered("ivy", "absolve cody", &mut st),
            "ivy never confessed to, has no absolve action"
        );
    }

    // H: ConfessionSpec.hs "double-absolution is blocked while the defeater stands"
    #[test]
    fn double_absolution_is_blocked() {
        let mut st = absolve_world();
        do_act("gwen", "absolve cody", &mut st);
        assert!(
            !offered("gwen", "absolve cody", &mut st),
            "gwen cannot grant a second time"
        );
    }

    // ---- Group 3: incorrigibility -------------------------------------------

    fn incorrig_world() -> State {
        let mut st = State::new();
        st.set_characters(
            ["mave", "hank", "cody"]
                .into_iter()
                .map(Character::new)
                .collect(),
        )
        .unwrap();
        for o in [
            // mave believes exactly ONE distinct instance of cody's wrongdoing
            // (k-1 of k=2): must derive nothing.
            insert("mave.believes.wronged.cody.carol.seen"),
            // hank believes TWO distinct instances — one seen, one gossiped —
            // reaching k=2: must derive the regard.
            insert("hank.believes.wronged.cody.carol.seen"),
            insert("hank.believes.wronged.cody.dana.heard.jill"),
        ] {
            st.perform_outcome(&o).expect("incorrigibility setup");
        }
        st.set_axioms(vec![incorrigible(PAT, 2, "fedUp").unwrap()])
            .unwrap();
        st
    }

    // H: ConfessionSpec.hs "incorrigibility: threshold, gossip, independence, permanence"
    // H: ConfessionSpec.hs "k-1 believed deeds derive nothing"
    #[test]
    fn k_minus_one_believed_deeds_derive_nothing() {
        let mut st = incorrig_world();
        assert!(
            !st.view_has("regards.mave.cody.fedUp"),
            "mave (one instance) does not regard cody as fed-up"
        );
    }

    // H: ConfessionSpec.hs "k believed deeds -- one via gossip -- derive the regard"
    #[test]
    fn k_believed_deeds_one_via_gossip_derive_the_regard() {
        let mut st = incorrig_world();
        assert!(
            st.view_has("regards.hank.cody.fedUp"),
            "hank (two instances, one gossiped) regards cody as fed-up"
        );
    }

    // H: ConfessionSpec.hs "per-absolver independence: one fed up, one fresh, from the same facts"
    #[test]
    fn per_absolver_independence() {
        let mut st = incorrig_world();
        assert!(st.view_has("regards.hank.cody.fedUp"), "hank: fed up");
        assert!(!st.view_has("regards.mave.cody.fedUp"), "mave: fresh");
    }

    // H: ConfessionSpec.hs "permanence: an absolution elsewhere does not retract the regard"
    #[test]
    fn permanence_an_absolution_elsewhere_does_not_retract_the_regard() {
        let mut st = incorrig_world();
        st.perform_outcome(&insert("recanted.cody")).unwrap();
        assert!(
            st.view_has("regards.hank.cody.fedUp"),
            "hank's fed-up regard survives an unrelated absolution"
        );
    }

    // ---- Group 4: the probed arithmetic -------------------------------------

    fn spont_world(stake: i32) -> State {
        let (roster, persona_facts) = cast(
            &[conscience()],
            vec![
                (
                    Character::new("wade").want(conceal("wronged.wade.carol", stake).unwrap()),
                    vec![conscience()],
                ),
                (Character::new("priya"), Vec::new()),
            ],
        )
        .expect("the spontaneous-confession roster");
        let mut st = State::new();
        st.define_practices([Practice::new("confessional")
            .roles(["R"])
            .action(confess_act())
            .action(Action::new("[Actor]: hold your tongue").when([matches("at.Actor!P")]))])
            .unwrap();
        st.set_characters(roster).unwrap();
        st.set_desires(persona_vocabulary(&[conscience()]).unwrap())
            .unwrap();
        for o in persona_facts.iter().chain(
            [
                insert("practice.confessional.here"),
                insert("at.wade!yard"),
                insert("at.priya!yard"),
                insert("wade.lied.edda.wronged.wade.carol"),
            ]
            .iter(),
        ) {
            st.perform_outcome(o).expect("spontaneous setup");
        }
        st
    }

    const BLACKMAIL_PAT: &str = "stole.Vic.loaf";

    fn blackmail_world(price: i32, fear: i32) -> State {
        let (_, acts) = shakedown("theft", &together(), BLACKMAIL_PAT, "favor", 2)
            .expect("the confession fixture's shakedown");
        let mut yard = Practice::new("yard").roles(["R"]);
        for a in acts {
            yard = yard.action(a);
        }
        yard = yard.action(
            confess(
                "lied",
                "confessed",
                &together(),
                BLACKMAIL_PAT,
                BLACKMAIL_PAT,
                "[Actor]: confess to [Hearer] about the loaf",
            )
            .expect("the blackmail-defense confess"),
        );
        yard = yard.action(Action::new("[Actor]: wait").when([matches("at.Actor!P")]));
        let mut st = State::new();
        st.define_practices([yard]).unwrap();
        st.set_characters(vec![
            Character::new("mel").holds("punishes-theft"),
            Character::new("vic")
                .want(Want::new(vec![matches("debt.mel.vic.favor")], -price))
                .holds("fears-scandal")
                .holds("guilt")
                .holds("clearConscience"),
            Character::new("cora"),
        ])
        .unwrap();
        for o in [
            insert("practice.yard.here"),
            insert("at.mel!court"),
            insert("at.vic!court"),
            insert("at.cora!court"),
            insert("mel.believes.stole.vic.loaf.seen"),
            insert("vic.lied.some.stole.vic.loaf"),
        ] {
            st.perform_outcome(&o).expect("blackmail-defense setup");
        }
        st.set_desires(vec![
            shakedown("theft", &together(), BLACKMAIL_PAT, "favor", 2)
                .unwrap()
                .0,
            Desire::new(
                "fears-scandal",
                Want::new(vec![matches("W.believes.stole.Owner.loaf")], -fear),
            ),
            // The v25 conscience shape keyed to THIS fixture's own deed pattern:
            // -6 while the "lied" mark stands, 0 once confessed — the residue
            // that makes confessing a merit-based move rather than a label-order
            // tie-break against defy/wait.
            Desire::new(
                "guilt",
                Want::new(vec![matches("Owner.lied.H.stole.Owner.loaf")], -6),
            ),
            Desire::new(
                "clearConscience",
                Want::new(vec![matches("Owner.confessed.H.stole.Owner.loaf")], 0),
            ),
        ])
        .unwrap();
        st
    }

    // H: ConfessionSpec.hs "probed arithmetic: spontaneous confession, blackmail defense"
    // H: ConfessionSpec.hs "spontaneous confession: a mild stake makes conscience relief worth it"
    #[test]
    fn a_mild_stake_makes_conscience_relief_worth_it() {
        let mut st = spont_world(4);
        let wade = member(&st, "wade");
        let scores = st.score_actions(2, &wade);
        assert_eq!(
            score_of(&scores, "confess to priya about wronging carol"),
            0.0
        );
        assert_eq!(score_of(&scores, "hold your tongue"), -2.0);
        assert_eq!(
            st.pick_action(2, &wade).map(|ga| ga.label),
            Some("wade: confess to priya about wronging carol".to_owned())
        );
    }

    // H: ConfessionSpec.hs "spontaneous confession: an expensive stake makes it NOT worth it"
    #[test]
    fn an_expensive_stake_makes_it_not_worth_it() {
        let mut st = spont_world(20);
        let wade = member(&st, "wade");
        let scores = st.score_actions(2, &wade);
        assert_eq!(score_of(&scores, "hold your tongue"), 37.94);
        assert_eq!(
            score_of(&scores, "confess to priya about wronging carol"),
            0.0
        );
        assert_eq!(
            st.pick_action(2, &wade).map(|ga| ga.label),
            Some("wade: hold your tongue".to_owned())
        );
    }

    // H: ConfessionSpec.hs "blackmail defense: a high price against a mild secret -- the victim confesses (property 4, ordering)"
    #[test]
    fn a_high_price_against_a_mild_secret_makes_the_victim_confess() {
        let mut st = blackmail_world(30, 3);
        do_act("mel", "threaten vic", &mut st);
        let vic = member(&st, "vic");
        let scores = st.score_actions(2, &vic);
        let confess_ = score_of(&scores, "confess to cora about the loaf");
        let defy = score_of(&scores, "defy mel");
        let wait_ = score_of(&scores, "wait");
        let comply = score_of(&scores, "buy mel's silence");
        // The v49 contract is the ORDERING: confess STRICTLY dominates the
        // defy/wait tie (conscience relief, not an alphabetical tie-break), and
        // paying the steep price is dead last.
        assert!(
            confess_ > defy && defy == wait_ && wait_ > comply,
            "confess must strictly dominate; had confess={confess_} defy={defy} \
             wait={wait_} comply={comply}"
        );
        assert_eq!(
            st.pick_action(2, &vic).map(|ga| ga.label),
            Some("vic: confess to cora about the loaf".to_owned())
        );
    }

    // H: ConfessionSpec.hs "blackmail defense: after confessing, mel's expose deposits nothing new (dead)"
    #[test]
    fn after_confessing_mels_expose_is_dead() {
        let mut st = blackmail_world(30, 3);
        do_act("mel", "threaten vic", &mut st);
        do_act("vic", "confess to cora about the loaf", &mut st);
        assert!(
            st.db_has("cora.believes.stole.vic.loaf.heard.vic"),
            "cora now believes, sourced from vic himself"
        );
        assert!(
            !offered("mel", "expose", &mut st),
            "mel has no expose action left (every co-present hearer already believes)"
        );
        assert_eq!(
            st.possible_actions("mel")
                .into_iter()
                .map(|ga| ga.label)
                .collect::<Vec<_>>(),
            vec!["mel: wait".to_owned()]
        );
    }

    // H: ConfessionSpec.hs "blackmail defense: the converse -- a cheap price against a severe fear -- complies (property 4, ordering)"
    #[test]
    fn a_cheap_price_against_a_severe_fear_makes_the_victim_comply() {
        let mut st = blackmail_world(1, 30);
        do_act("mel", "threaten vic", &mut st);
        let vic = member(&st, "vic");
        let scores = st.score_actions(2, &vic);
        let comply = score_of(&scores, "buy mel's silence");
        let confess_ = score_of(&scores, "confess to cora about the loaf");
        let defy = score_of(&scores, "defy mel");
        let wait_ = score_of(&scores, "wait");
        assert!(
            comply > confess_ && confess_ > defy && defy == wait_,
            "comply must strictly dominate; had comply={comply} confess={confess_} \
             defy={defy} wait={wait_}"
        );
        assert_eq!(
            st.pick_action(2, &vic).map(|ga| ga.label),
            Some("vic: buy mel's silence".to_owned())
        );
    }

    // ---- Group 5: re-offense snaps the defeater back ------------------------

    // H: ConfessionSpec.hs "re-offense deletes the defeater (v21 idiom)"
    // H: ConfessionSpec.hs "absolved, then re-offending snaps the defeater away"
    #[test]
    fn absolved_then_re_offending_snaps_the_defeater_away() {
        // A fixed second victim (dana) keeps the action's own precondition free
        // of any query variable beyond Actor.
        let reoffend = Action::new("[Actor]: wrong dana again")
            .when([matches("at.Actor!P")])
            .then([
                insert("Actor.lied.edda.wronged.Actor.dana"),
                delete("recanted.Actor"),
            ]);
        let mut st = State::new();
        st.define_practices([Practice::new("confessional")
            .roles(["R"])
            .action(absolve_act())
            .action(reoffend)])
            .unwrap();
        for o in [
            insert("practice.confessional.here"),
            insert("at.cody!yard"),
            insert("gwen.believes.wronged.cody.carol.heard.cody"),
        ] {
            st.perform_outcome(&o).expect("re-offense setup");
        }
        st.set_characters(vec![Character::new("cody"), Character::new("gwen")])
            .unwrap();
        do_act("gwen", "absolve cody", &mut st);
        assert!(st.db_has("recanted.cody"), "recanted stands");
        do_act("cody", "wrong dana again", &mut st);
        assert!(
            !st.db_has("recanted.cody"),
            "recanted is gone: standing snaps back before a less patient audience"
        );
        assert!(
            st.db_has("cody.lied.edda.wronged.cody.dana"),
            "the new lied mark is planted"
        );
    }

    // ---- Group 7: the decoupled deposit -------------------------------------

    // H: ConfessionSpec.hs "the deposit is decoupled from the mark's own content"
    // H: ConfessionSpec.hs "confessing converts the content-mark and deposits the ACT, not the content"
    #[test]
    fn confessing_deposits_the_act_not_the_content() {
        // eve's mark is content-shaped ("stole.C.loaf" — who she framed);
        // confessing it must NOT re-assert that content to the new hearer — it
        // reveals the ACT of having whispered the lie, and to WHOM (H, the mark's
        // own original hearer), grounded straight from the mark's own bindings.
        let decoupled = confess(
            "lied",
            "confessed",
            &together(),
            "stole.C.loaf",
            "whispered.Actor.H",
            "[Actor]: confess to [Hearer] about framing [C]",
        )
        .expect("the decoupled confess");
        let mut st = State::new();
        st.define_practices([Practice::new("confessional").roles(["R"]).action(decoupled)])
            .unwrap();
        for o in [
            insert("practice.confessional.here"),
            insert("at.eve!yard"),
            insert("at.fay!yard"),
            // eve once lied to gale, framing carol for the theft.
            insert("eve.lied.gale.stole.carol.loaf"),
        ] {
            st.perform_outcome(&o).expect("decoupled setup");
        }
        st.set_characters(vec![Character::new("eve"), Character::new("fay")])
            .unwrap();
        do_act("eve", "confess to fay about framing carol", &mut st);
        assert!(
            st.db_has("eve.confessed.gale.stole.carol.loaf"),
            "the content-mark converted"
        );
        assert!(
            !st.db_has("eve.lied.gale.stole.carol.loaf"),
            "the content-mark's lied form is gone"
        );
        assert!(
            st.db_has("fay.believes.whispered.eve.gale.heard.eve"),
            "fay learns the ACT (whispered.eve.gale), sourced from eve"
        );
        assert!(
            !st.db_has("fay.believes.stole.carol.loaf"),
            "fay does NOT learn a re-assertion of the framed content"
        );
    }

    // ---- Group 6: guards forced ---------------------------------------------

    // H: ConfessionSpec.hs "guards"
    // H: ConfessionSpec.hs "confess rejects a dotted mark kind"
    #[test]
    fn confess_rejects_a_dotted_mark_kind() {
        assert!(
            confess(
                "li.ed",
                "confessed",
                &together(),
                PAT,
                PAT,
                "[Actor]: confess"
            )
            .is_err(),
            "a dotted kind is an error"
        );
    }

    // H: ConfessionSpec.hs "confess rejects a dotted discharge verb"
    #[test]
    fn confess_rejects_a_dotted_discharge_verb() {
        assert!(
            confess(
                "lied",
                "con.fessed",
                &together(),
                PAT,
                PAT,
                "[Actor]: confess"
            )
            .is_err(),
            "a dotted verb is an error"
        );
    }

    // H: ConfessionSpec.hs "confess rejects a mark pattern reserving H/Hearer/Actor"
    #[test]
    fn confess_rejects_a_mark_pattern_reserving_h() {
        assert!(
            confess(
                "lied",
                "confessed",
                &together(),
                "wronged.H.Victim",
                "wronged.H.Victim",
                "[Actor]: confess"
            )
            .is_err(),
            "H is reserved (the mark's own hearer slot)"
        );
    }

    // H: ConfessionSpec.hs "confess rejects a deposit pattern reserving Hearer"
    #[test]
    fn confess_rejects_a_deposit_pattern_reserving_hearer() {
        assert!(
            confess(
                "lied",
                "confessed",
                &together(),
                "stole.C.loaf",
                "told.Actor.Hearer",
                "[Actor]: confess"
            )
            .is_err(),
            "Hearer is reserved (the confession's own audience role)"
        );
    }

    // H: ConfessionSpec.hs "confess rejects an ungroundable deposit variable"
    #[test]
    fn confess_rejects_an_ungroundable_deposit_variable() {
        // "Someone" is neither Actor, H, nor one of the mark's own variables (the
        // mark is "stole.C.loaf", whose only variable is C) — grounding it would
        // insert a variable-bearing fact.
        assert!(
            confess(
                "lied",
                "confessed",
                &together(),
                "stole.C.loaf",
                "whispered.Actor.Someone",
                "[Actor]: confess"
            )
            .is_err(),
            "an ungroundable deposit variable is an error"
        );
    }

    // H: ConfessionSpec.hs "absolve rejects non-single-segment defeater/label"
    #[test]
    fn absolve_rejects_a_dotted_defeater() {
        assert!(
            absolve("re.canted", PAT, "fedUp", "[Actor]: absolve").is_err(),
            "a dotted defeater is an error"
        );
    }

    // H: ConfessionSpec.hs "absolve rejects an event pattern reserving Actor"
    #[test]
    fn absolve_rejects_an_event_pattern_reserving_actor() {
        assert!(
            absolve(
                "recanted",
                "wronged.Actor.Victim",
                "fedUp",
                "[Actor]: absolve"
            )
            .is_err(),
            "Actor is reserved"
        );
    }

    // H: ConfessionSpec.hs "absolve rejects a subject-less event pattern"
    #[test]
    fn absolve_rejects_a_subject_less_event_pattern() {
        assert!(
            absolve("recanted", "somethinghappened", "fedUp", "[Actor]: absolve").is_err(),
            "an absolution needs a confessor"
        );
    }

    // H: ConfessionSpec.hs "incorrigible rejects a dotted label"
    #[test]
    fn incorrigible_rejects_a_dotted_label() {
        assert!(
            incorrigible(PAT, 2, "fed.up").is_err(),
            "a dotted label is an error"
        );
    }

    // H: ConfessionSpec.hs "incorrigible rejects a pattern authoring the Prax namespace"
    #[test]
    fn incorrigible_rejects_a_prax_namespace_pattern() {
        assert!(
            incorrigible("wronged.Doer.PraxDs", 2, "fedUp").is_err(),
            "PraxDs is reserved (the axiom's own subquery set variable)"
        );
    }

    // H: ConfessionSpec.hs "incorrigible: the usability win -- W/Ds/N are ordinary variables now (v40 moved the axiom's own machinery to the Prax namespace)"
    #[test]
    fn incorrigible_accepts_w_ds_and_n_as_ordinary_variables() {
        assert!(
            incorrigible("wronged.Doer.Ds.W.N", 2, "fedUp").is_ok(),
            "W, Ds, and N no longer collide with anything"
        );
    }

    // H: ConfessionSpec.hs "incorrigible rejects a pattern naming no offender"
    #[test]
    fn incorrigible_rejects_a_pattern_naming_no_offender() {
        assert!(
            incorrigible("somethinghappened", 2, "fedUp").is_err(),
            "a threshold needs an offender"
        );
    }

    // H: ConfessionSpec.hs "incorrigible rejects a single-variable pattern (no deed variables to count)"
    #[test]
    fn incorrigible_rejects_a_single_variable_pattern() {
        assert!(
            incorrigible("confessed.Doer", 2, "fedUp").is_err(),
            "one variable can name only one instance -- a k>1 threshold could never fire"
        );
    }

    // H: ConfessionSpec.hs "incorrigible rejects a deed variable colliding with the witness-naming convention"
    #[test]
    fn incorrigible_rejects_a_witness_naming_collision() {
        assert!(
            incorrigible("wronged.Doer.Victim.Victim0", 2, "fedUp").is_err(),
            "Victim0 would collide with Victim's own outer witness dummy"
        );
    }
}
