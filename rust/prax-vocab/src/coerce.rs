//! Coercion: leverage as a content-agnostic primitive.
//!
//! **A threat is conditional intent made credible by a desire.** [`coerce`]
//! generates the four-action protocol — threaten/comply/defy/punish — and the
//! punitive [`Desire`] that professes the intent, exactly as [`crate::blackmail`]
//! does for its exposure instance, but with the CONTENT (what makes the threat
//! available, what compliance costs, what punishment does, and what the extorter
//! VALUES about the punished state) lifted out into the [`Coercion`] record.
//!
//! **Evidence is optional.** The primitive owns none of it: the trigger
//! (including how the victim is reached — co-presence for an in-person threat, a
//! letter in absentia) is author content, and the punitive kernel is whatever the
//! instance names.
//!
//! **The kernel variable law.** The author writes the kernel with NO
//! `Prax`-namespaced variables (the v40 splice guards forbid them on every
//! authored field). [`coerce`] then alpha-renames the kernel INTO the `Prax`
//! namespace, op-preservingly ([`namespace_kernel`]): the victim → `PraxD`, and
//! every other author-introduced free variable that is not the mechanism interface
//! name `Owner` → `PraxW`, `PraxW2`, … in FIRST-APPEARANCE order. Only the
//! post-rename OUTPUT is `Prax`-namespaced; no author ever writes one.
//!
//! **Credibility is three world states (spec v54 §2).** `threaten` itself plants
//! the victim's fear (a mechanism-owned `believes.desires.<E>.punishes-<id>`
//! deposit); what distinguishes GENUINE (registered AND held), BLUFF (registered,
//! not held) and THE ACCIDENT (unregistered — S9's `CoercionUnmotivated` lint) is
//! authored world state, not a flag.
//!
//! Frozen reference: `src/Prax/Coerce.hs`. [`namespace_kernel`] is the module's
//! one non-mechanical part (S7 design §3.1): the numbering runs over an
//! ORDER-PRESERVING dedup of the kernel's free variables in the order
//! [`prax_core::query::condition_vars`] yields them — a sorted set would rename
//! differently and still render plausibly.

use std::collections::BTreeMap;

use prax_core::error::WorldError;
use prax_core::interner::is_variable_name;
use prax_core::query::{Condition, flat_condition_vars, matches, neq, not_, or_};
use prax_core::types::{
    Action, Desire, Outcome, Want, authored_var_clash, delete, insert, insert_for, is_prax_var,
    rename_vars,
};
use prax_core::vocab_consts::PUNITIVE_PREFIX;

use crate::beliefs::belief_about;

/// A coercion: the leverage skeleton with its content in named fields (the frozen
/// `Coercion`, field for field in declaration order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coercion {
    /// Single path segment; scopes this coercion's facts.
    pub id: String,
    /// The victim role variable (see the reserved set).
    pub victim: String,
    /// What makes threatening available, INCLUDING how the victim is reached
    /// (co-presence, a letter, …). Must BIND the victim.
    pub trigger: Vec<Condition>,
    /// Display template for the threaten action.
    pub threaten_label: String,
    /// Display template for the comply action.
    pub demand_label: String,
    /// What compliance does.
    pub demand: Vec<Outcome>,
    /// Display template for the punish action.
    pub punish_label: String,
    /// EXTRA punish availability (the core gate is mechanism-owned: a standing
    /// threat or a defiance).
    pub punish_when: Vec<Condition>,
    /// What punishment does.
    pub punish_outs: Vec<Outcome>,
    /// What the extorter VALUES about the punished state, authored with plain
    /// variable names.
    pub kernel: Vec<Condition>,
    /// The extorter's punitive weight.
    pub weight: i32,
    /// `None` = a standing threat (permanent marker); `Some(n)` = the threat
    /// marker retracts n boundaries after threaten (the DEFIED arm is untouched).
    pub threat_lasts: Option<i64>,
    /// `None` = bought silence stays bought (permanent `complied` marker);
    /// `Some(n)` = the marker expires and the racket cycles — one purchase per
    /// bought period.
    pub compliance_lasts: Option<i64>,
}

/// The marker-insert selector shared by BOTH markers (the threat and the
/// `complied` mark): `None` compiles to a permanent `Insert` (byte-identical to
/// the shipped default), `Some(n)` to v44's boundary-exact `InsertFor`.
fn lasting(n: Option<i64>, s: String) -> Outcome {
    match n {
        None => insert(s),
        Some(n) => insert_for(n, s),
    }
}

/// `coerce(coercion)` generates the threaten/comply/defy/punish protocol and the
/// punitive [`Desire`] it professes. The [`Desire`] must be registered and held
/// (the module header's registration contract).
///
/// # Errors
/// In the frozen guard order: [`WorldError::NotASinglePathSegment`] for a
/// punctuated id; [`WorldError::ReservedVarClash`] for a reserved victim name, and
/// for a trigger/demand/punish/kernel field naming a name that would CAPTURE in
/// that field's own generated query — which is FRAME-RELATIVE, not uniform (the
/// trigger's own frame already binds `Actor` to the extorter, so the trigger MAY
/// name `Actor`; the `Prax` namespace is reserved on every field regardless of
/// frame). [`WorldError::TrailingOperator`] from any malformed authored sentence.
pub fn coerce(co: &Coercion) -> Result<(Desire, Vec<Action>), WorldError> {
    let sid = &co.id;
    let victim = &co.victim;
    if sid.contains(['.', '!']) {
        return Err(WorldError::NotASinglePathSegment {
            context: "Coerce.coerce: id".to_owned(),
            name: sid.clone(),
        });
    }
    if ["Actor", "E", "Owner", "Hearer"].contains(&victim.as_str()) || is_prax_var(victim) {
        return Err(WorldError::ReservedVarClash {
            context: "Coerce.coerce: victim variable".to_owned(),
            var: victim.clone(),
            extra: " -- Actor and E (the generated actions' own extorter and victim roles), \
                    Owner (the punitive desire's extorter), Hearer, and the Prax namespace are \
                    all mechanism-owned; pick another name for the victim"
                .to_owned(),
        });
    }
    // Each authored field forbids exactly the mechanism names that would CAPTURE
    // (silently unify with a name the author didn't intend) in its OWN generated
    // query. The trigger is the one field where Actor is already the author's own
    // frame variable (the extorter performing threaten) rather than something the
    // mechanism binds out from under them, so naming it is a legitimate frame
    // reference, not a capture; E never appears in the threaten query at all, so
    // forbidding it would be inert.
    /// One field's hygiene check: the field name, the names IT forbids beyond the
    /// `Prax` namespace, its conditions, its outcomes, and the error tail
    /// explaining that field's own frame.
    type FieldCheck<'a> = (
        &'a str,
        Vec<String>,
        &'a [Condition],
        &'a [Outcome],
        &'a str,
    );
    let checks: [FieldCheck<'_>; 4] = [
        (
            "trigger",
            Vec::new(),
            co.trigger.as_slice(),
            &[],
            " -- the Prax namespace is reserved for the mechanism's own post-rename output",
        ),
        (
            "demand",
            vec![victim.clone()],
            &[],
            co.demand.as_slice(),
            " -- in the comply query the victim is Actor and the extorter is E; refer to the victim as Actor",
        ),
        (
            "punish",
            vec!["E".to_owned()],
            co.punish_when.as_slice(),
            co.punish_outs.as_slice(),
            " -- E is the victim's frame (comply/defy); the punish query's extorter is Actor",
        ),
        (
            "kernel",
            vec!["Actor".to_owned(), "E".to_owned()],
            co.kernel.as_slice(),
            &[],
            " -- the kernel's frame is the punitive desire: its extorter is Owner and its victim is renamed to PraxD",
        ),
    ];
    for (field, forbidden, conds, outs, extra) in checks {
        if let Some(bad) = authored_var_clash(&forbidden, conds, outs)?.first() {
            return Err(WorldError::ReservedVarClash {
                context: format!("Coerce.coerce: {field}"),
                var: bad.clone(),
                extra: extra.to_owned(),
            });
        }
    }

    // Fact conventions, id-scoped so multiple coercions coexist.
    let threat_path = |extorter: &str, v: &str| format!("threatened.{sid}.{extorter}.{v}");
    let defied_path = |v: &str, extorter: &str| format!("defied.{sid}.{v}.{extorter}");
    let complied_path = |extorter: &str, v: &str| format!("complied.{sid}.{extorter}.{v}");
    let punitive_name = format!("{PUNITIVE_PREFIX}{sid}");

    // Actor is the extorter. The threat IS the communication of conditional
    // intent: the marker (lifetime = `threat_lasts`), the motive-belief deposit
    // (over the same channel confiding and lying ride), and the extorter's own
    // mark. The deposit and the extorted mark stay PERMANENT: they record the
    // attempt, not the threat's currency.
    let mut threaten_when = co.trigger.clone();
    threaten_when.push(neq(victim.as_str(), "Actor"));
    threaten_when.push(not_(threat_path("Actor", victim)));
    let threaten = Action::new(&co.threaten_label).when(threaten_when).then([
        lasting(co.threat_lasts, threat_path("Actor", victim)),
        insert(format!(
            "{}.heard.Actor",
            belief_about(victim, &format!("desires.Actor.{punitive_name}"))
        )),
        insert(format!("Actor.extorted.{victim}.{sid}")),
    ]);

    // The victim buys off the threat: they are Actor, the extorter is E.
    let mut comply_then = co.demand.clone();
    comply_then.push(lasting(co.compliance_lasts, complied_path("E", "Actor")));
    comply_then.push(delete(threat_path("E", "Actor")));
    let comply = Action::new(&co.demand_label)
        .when([
            matches(threat_path("E", "Actor")),
            not_(complied_path("E", "Actor")),
        ])
        .then(comply_then);

    let defy = Action::new("[Actor]: defy [E]")
        .when([matches(threat_path("E", "Actor"))])
        .then([
            insert(defied_path("Actor", "E")),
            delete(threat_path("E", "Actor")),
        ]);

    // The extorter punishes a STANDING threat or a defiance — gating on defiance
    // alone would make stalling safe forever. Actor is the extorter.
    let mut punish_when = vec![or_(vec![
        vec![matches(threat_path("Actor", victim))],
        vec![matches(defied_path(victim, "Actor"))],
    ])];
    punish_when.extend(co.punish_when.iter().cloned());
    let punish = Action::new(&co.punish_label)
        .when(punish_when)
        .then(co.punish_outs.clone());

    // The punitive desire pays `weight` per (victim, valued-state) pair: the Or
    // clause (victim = PraxD, extorter = Owner) joins the renamed kernel on PraxD.
    // Owner-templated, instantiated per holder (`Prax.Minds.wantFor`).
    let mut punitive_when = vec![or_(vec![
        vec![matches(defied_path("PraxD", "Owner"))],
        vec![matches(threat_path("Owner", "PraxD"))],
    ])];
    punitive_when.extend(namespace_kernel(victim, &co.kernel)?);
    let punitive = Desire::new(punitive_name, Want::new(punitive_when, co.weight));

    Ok((punitive, vec![threaten, comply, defy, punish]))
}

/// Alpha-rename an author-written kernel into the `Prax` namespace,
/// op-preservingly. The `victim` variable → `PraxD`; every other free variable, in
/// FIRST-APPEARANCE order and excluding the mechanism interface name `Owner`, →
/// `PraxW`, `PraxW2`, … (note there is NO `PraxW1`). Renaming is by NAME, applied
/// uniformly through every [`Condition`] constructor — so a binder (`Subquery`'s
/// set/find variables) and its interior uses move together, and `Match`/`Not`
/// pattern segments round-trip so each segment's following `.`/`!` operator is
/// preserved ([`prax_core::types::rename_vars`]).
///
/// The frozen `nub` is an ORDER-PRESERVING dedup over `concatMap conditionVars`.
/// A set-based collection would sort the names and hand out different `PraxW`
/// numbers — value-different, and legible enough to pass a reading.
///
/// # Errors
/// [`WorldError::TrailingOperator`] from [`prax_core::query::condition_vars`] on a
/// malformed kernel sentence.
pub fn namespace_kernel(victim: &str, conds: &[Condition]) -> Result<Vec<Condition>, WorldError> {
    let mut free: Vec<String> = Vec::new();
    for v in flat_condition_vars(conds)? {
        if is_variable_name(&v) && !free.contains(&v) {
            free.push(v);
        }
    }
    let mut subst: BTreeMap<String, String> =
        BTreeMap::from([(victim.to_owned(), "PraxD".to_owned())]);
    let mut n = 1usize;
    for v in free {
        if v == victim || v == "Owner" {
            continue;
        }
        subst.insert(
            v,
            if n == 1 {
                "PraxW".to_owned()
            } else {
                format!("PraxW{n}")
            },
        );
        n += 1;
    }
    Ok(rename_vars(&subst, conds))
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_core::engine::{GroundedAction, State};
    use prax_core::query::CmpOp;
    use prax_core::types::{Character, Practice};

    use crate::debt::{owe, owes};
    use crate::witness::{CoPresence, as_role};

    // H: CoerceSpec.hs "Prax.Coerce"
    //
    // The frozen `Prax.CoerceSpec`, re-expressed against the Rust engine.
    //
    // A protection racket, the SECOND instance of the leverage skeleton
    // (blackmail is the first): mob threatens to burn a barn-owner's barn unless
    // they do a favor. It is EVIDENCE-FREE — the trigger is merely owning a barn
    // — and its punitive kernel is VENGEANCE, not exposure. The kernel is
    // authored with the plain victim name `V`; `coerce` lifts it to `PraxD`.

    fn racket() -> Coercion {
        Coercion {
            id: "racket".to_owned(),
            victim: "V".to_owned(),
            trigger: vec![matches("barn.V")],
            threaten_label: "[Actor]: threaten [V]".to_owned(),
            demand_label: "[Actor]: do [E]'s favor".to_owned(),
            demand: owe("E", "Actor", "favor").unwrap(),
            punish_label: "[Actor]: burn [V]'s barn".to_owned(),
            punish_when: vec![matches("barn.V"), not_("burned.barn.V")],
            punish_outs: vec![insert("burned.barn.V")],
            kernel: vec![matches("burned.barn.V")],
            weight: 9,
            threat_lasts: None,
            compliance_lasts: None,
        }
    }

    /// The do-nothing alternative. Its label sorts BEFORE "threaten" so that when
    /// the punitive want is ABSENT and threatening ties it at zero, the tie-break
    /// by label picks bide.
    fn bide_act() -> Action {
        Action::new("[Actor]: bide")
    }

    fn acts_of(co: &Coercion) -> Vec<Action> {
        coerce(co).expect("a legal coercion").1
    }

    /// `holds_want` toggles whether mob actually HOLDS the punitive desire — the
    /// sole variable behind the vengeance self-motivation pin.
    fn mk_world(holds_want: bool) -> State {
        mk_turf_world_with(&racket(), holds_want)
    }

    /// A turf world installing a given racket's actions; mob holds the named
    /// vengeance want iff `holds_want`, vic fears the burned barn and the debt.
    fn mk_turf_world_with(co: &Coercion, holds_want: bool) -> State {
        let (pun, acts) = coerce(co).expect("a legal coercion");
        let mut turf = Practice::new("turf").roles(["R"]);
        for a in acts {
            turf = turf.action(a);
        }
        turf = turf.action(bide_act());
        let mut mob = Character::new("mob");
        if holds_want {
            mob = mob.holds(pun.name.clone());
        }
        let mut st = State::new();
        st.define_practices([turf]).unwrap();
        st.set_characters(vec![
            mob,
            Character::new("vic")
                .want(Want::new(vec![matches("burned.barn.vic")], -12))
                .want(Want::new(vec![owes("mob", "vic", "favor").unwrap()], -4)),
        ])
        .unwrap();
        for o in [insert("practice.turf.here"), insert("barn.vic")] {
            st.perform_outcome(&o).expect("turf setup");
        }
        st.set_desires(vec![pun]).unwrap();
        st
    }

    fn mk_turf_world(co: &Coercion) -> State {
        mk_turf_world_with(co, true)
    }

    /// v54: a SERIAL-extortion racket. The complied marker expires after 2
    /// boundaries, so the racket cycles.
    fn cycling_racket() -> Coercion {
        Coercion {
            compliance_lasts: Some(2),
            ..racket()
        }
    }

    /// v54: an EXPIRING threat. The DEFIED arm is untouched.
    fn expiring_racket() -> Coercion {
        Coercion {
            threat_lasts: Some(2),
            ..racket()
        }
    }

    /// Drive k round boundaries (the ScheduleSpec idiom — `InsertFor` retracts
    /// only at a boundary, which the racket harness never fires on its own).
    fn boundaries(k: usize, st: &mut State) {
        for _ in 0..k {
            st.round_boundary();
        }
    }

    fn is_insert_for(o: &Outcome) -> bool {
        matches!(o, Outcome::InsertFor(_, _))
    }

    /// v54: counter-coercion is composition. vic's own Coercion whose trigger
    /// reads the mark mob's extortion deposits — the extorter slot `M` is a
    /// VARIABLE, so the trigger BINDS the counter's victim to whoever extorted
    /// vic.
    fn counter() -> Coercion {
        Coercion {
            id: "counter".to_owned(),
            victim: "M".to_owned(),
            trigger: vec![matches("M.extorted.vic.racket")],
            threaten_label: "[Actor]: threaten to expose [M]".to_owned(),
            demand_label: "[Actor]: take [E]'s protection".to_owned(),
            demand: owe("E", "Actor", "protection").unwrap(),
            punish_label: "[Actor]: expose [M] as an extorter".to_owned(),
            punish_when: vec![not_("exposed.M")],
            punish_outs: vec![insert("exposed.M")],
            kernel: vec![matches("exposed.M")],
            weight: 9,
            threat_lasts: None,
            compliance_lasts: None,
        }
    }

    fn counter_world() -> State {
        let mut turf = Practice::new("turf").roles(["R"]);
        for a in acts_of(&racket()) {
            turf = turf.action(a);
        }
        turf = turf.action(bide_act());
        let mut countyard = Practice::new("countyard").roles(["R"]);
        for a in acts_of(&counter()) {
            countyard = countyard.action(a);
        }
        countyard = countyard.action(bide_act());
        let mut st = State::new();
        st.define_practices([turf, countyard]).unwrap();
        st.set_characters(vec![
            Character::new("mob").holds("punishes-racket"),
            Character::new("vic").holds("punishes-counter"),
        ])
        .unwrap();
        for o in [
            insert("practice.turf.here"),
            insert("practice.countyard.here"),
            insert("barn.vic"),
        ] {
            st.perform_outcome(&o).expect("counter setup");
        }
        st.set_desires(vec![
            coerce(&racket()).unwrap().0,
            coerce(&counter()).unwrap().0,
        ])
        .unwrap();
        st
    }

    /// Regression fixture (v49 Task 1 fix wave): a BLACKMAIL-SHAPED coercion
    /// built straight through `coerce` — an evidence trigger naming `Actor` (the
    /// extorter's own frame variable), a debt-shaped demand, and an expose-shaped
    /// punish. This is the shape that exposed the Critical finding: the
    /// extorter's evidence-holding is a legitimate frame reference in threaten's
    /// own query, not a capture.
    fn court() -> CoPresence {
        vec![matches("at.Actor!P"), matches("at.Witness!P")]
    }

    fn blackmail_shaped() -> Coercion {
        let mut trigger = vec![matches("Actor.believes.stole.V.loaf")];
        trigger.extend(as_role("V", &court()));
        let mut punish_when = vec![matches("Actor.believes.stole.V.loaf"), neq("Hearer", "V")];
        punish_when.extend(as_role("Hearer", &court()));
        Coercion {
            id: "leverage".to_owned(),
            victim: "V".to_owned(),
            trigger,
            threaten_label: "[Actor]: threaten [V] with what you know".to_owned(),
            demand_label: "[Actor]: buy [E]'s silence".to_owned(),
            demand: owe("E", "Actor", "silence").unwrap(),
            punish_label: "[Actor]: expose [V] to [Hearer]".to_owned(),
            punish_when,
            punish_outs: vec![insert("Hearer.believes.stole.V.loaf")],
            kernel: vec![matches("W.believes.stole.V.loaf")],
            weight: 6,
            threat_lasts: None,
            compliance_lasts: None,
        }
    }

    fn leverage_world() -> State {
        let mut court_p = Practice::new("court").roles(["R"]);
        for a in acts_of(&blackmail_shaped()) {
            court_p = court_p.action(a);
        }
        let mut st = State::new();
        st.define_practices([court_p]).unwrap();
        st.set_characters(vec![
            Character::new("mel").holds("punishes-leverage"),
            Character::new("vic"),
        ])
        .unwrap();
        for o in [
            insert("practice.court.here"),
            insert("mel.believes.stole.vic.loaf"),
            insert("at.mel.court"),
            insert("at.vic.court"),
        ] {
            st.perform_outcome(&o).expect("leverage setup");
        }
        st.set_desires(vec![coerce(&blackmail_shaped()).unwrap().0])
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

    fn offers(who: &str, needle: &str, st: &mut State) -> bool {
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

    // H: CoerceSpec.hs "guards"
    // H: CoerceSpec.hs "a dotted id errors loudly"
    #[test]
    fn a_dotted_id_errors_loudly() {
        let co = Coercion {
            id: "rac.ket".to_owned(),
            ..racket()
        };
        assert!(coerce(&co).is_err(), "a dotted id is an error");
    }

    // H: CoerceSpec.hs "the reserved set: victim Actor errors (the previously-latent hole)"
    #[test]
    fn the_reserved_set_victim_actor_errors() {
        let co = Coercion {
            victim: "Actor".to_owned(),
            ..racket()
        };
        assert!(coerce(&co).is_err(), "victim Actor is reserved");
    }

    // H: CoerceSpec.hs "the reserved set: victim PraxV errors"
    #[test]
    fn the_reserved_set_victim_praxv_errors() {
        let co = Coercion {
            victim: "PraxV".to_owned(),
            ..racket()
        };
        assert!(coerce(&co).is_err(), "a Prax-namespaced victim is reserved");
    }

    // H: CoerceSpec.hs "the reserved set: a legal victim passes"
    #[test]
    fn the_reserved_set_a_legal_victim_passes() {
        assert!(
            coerce(&racket()).is_ok(),
            "an ordinary victim variable is fine"
        );
    }

    // H: CoerceSpec.hs "a trigger naming a Prax-namespaced variable errors loudly (the v40 law, frame-independent)"
    #[test]
    fn a_trigger_naming_a_prax_variable_errors_loudly() {
        let co = Coercion {
            trigger: vec![matches("spy.PraxW")],
            ..racket()
        };
        assert!(
            coerce(&co).is_err(),
            "a Prax var in the trigger is rejected"
        );
    }

    // H: CoerceSpec.hs "regression: the trigger guard must not forbid Actor (blackmail-shaped evidence, v49 Task 1 fix wave)"
    // H: CoerceSpec.hs "a blackmail-shaped Coercion whose trigger names Actor (the extorter's own evidence) constructs without error"
    #[test]
    fn a_blackmail_shaped_coercion_naming_actor_constructs() {
        assert!(
            coerce(&blackmail_shaped()).is_ok(),
            "Actor is bound in threaten's own frame, not a capture"
        );
    }

    // H: CoerceSpec.hs "threaten is offered and fires, depositing the threat marker"
    #[test]
    fn threaten_is_offered_and_fires() {
        let mut st = leverage_world();
        assert!(offers("mel", "threaten", &mut st), "threaten offered");
        do_act("mel", "threaten", &mut st);
        assert!(
            st.db_has("threatened.leverage.mel.vic"),
            "the threatened fact"
        );
    }

    // H: CoerceSpec.hs "the authored threaten label surfaces exactly (BlackmailSpec:171's pinned shape, proven producible)"
    #[test]
    fn the_authored_threaten_label_surfaces_exactly() {
        let mut st = leverage_world();
        let label = st
            .possible_actions("mel")
            .into_iter()
            .find(|ga| ga.label.contains("threaten"))
            .expect("no threaten action offered")
            .label;
        assert_eq!(label, "mel: threaten vic with what you know");
    }

    // H: CoerceSpec.hs "the rename law"
    // H: CoerceSpec.hs "an authored plain-var kernel lifts the victim to PraxD"
    #[test]
    fn an_authored_plain_var_kernel_lifts_the_victim_to_praxd() {
        let (pun, _) = coerce(&racket()).unwrap();
        assert!(
            pun.want.when.contains(&matches("burned.barn.PraxD")),
            "kernel V became PraxD; had {:?}",
            pun.want.when
        );
    }

    // H: CoerceSpec.hs "two fresh quantifiers get distinct names (PraxW, PraxW2), first-appearance order"
    #[test]
    fn two_fresh_quantifiers_get_distinct_names_in_first_appearance_order() {
        // A kernel with TWO fresh quantifiers beyond the victim.
        let co = Coercion {
            kernel: vec![matches("burned.barn.V"), matches("ally.A.of.B")],
            ..racket()
        };
        let (pun, _) = coerce(&co).unwrap();
        assert!(
            pun.want.when.contains(&matches("ally.PraxW.of.PraxW2")),
            "A -> PraxW and B -> PraxW2; had {:?}",
            pun.want.when
        );
    }

    // H: CoerceSpec.hs "an authored kernel naming a Prax variable errors loudly (no author writes one)"
    #[test]
    fn an_authored_kernel_naming_a_prax_variable_errors_loudly() {
        let co = Coercion {
            kernel: vec![matches("burned.barn.PraxW")],
            ..racket()
        };
        assert!(coerce(&co).is_err(), "a Prax var in the kernel is rejected");
    }

    // H: CoerceSpec.hs "threaten deposits and the punish gate"
    // H: CoerceSpec.hs "threatening deposits the marker, the motive-belief, and the extorted mark"
    #[test]
    fn threatening_deposits_the_marker_the_motive_belief_and_the_mark() {
        let mut st = mk_world(true);
        do_act("mob", "threaten", &mut st);
        assert!(
            st.db_has("threatened.racket.mob.vic"),
            "the threatened fact"
        );
        assert!(
            st.db_has("vic.believes.desires.mob.punishes-racket.heard.mob"),
            "the motive-belief deposit: vic hears mob's professed punitive intent"
        );
        assert!(
            st.db_has("mob.extorted.vic.racket"),
            "the extorted mark, tailed by the coercion id"
        );
    }

    // H: CoerceSpec.hs "punish fires against a STANDING threat, with no defiance"
    #[test]
    fn punish_fires_against_a_standing_threat() {
        let mut st = mk_world(true);
        do_act("mob", "threaten", &mut st);
        assert!(!st.db_has("defied.racket.vic.mob"), "vic has not defied");
        assert!(
            offers("mob", "burn", &mut st),
            "burning is offered against the standing threat alone"
        );
        do_act("mob", "burn", &mut st);
        assert!(st.db_has("burned.barn.vic"), "the barn is burned");
    }

    // H: CoerceSpec.hs "property 1: stalling never dominates"
    // H: CoerceSpec.hs "under a standing threat, bide never strictly beats both comply and defy"
    #[test]
    fn stalling_never_dominates() {
        let mut st = mk_world(true);
        do_act("mob", "threaten", &mut st);
        let vic = member(&st, "vic");
        let scores = st.score_actions(2, &vic);
        let bide = score_of(&scores, "vic: bide");
        let comply = score_of(&scores, "favor");
        let defy = score_of(&scores, "vic: defy");
        assert!(
            !(bide > comply && bide > defy),
            "bide must not dominate; had bide={bide} comply={comply} defy={defy}"
        );
    }

    // H: CoerceSpec.hs "property 3: repeat extraction is impossible"
    // H: CoerceSpec.hs "after compliance, a renewed threat offers no re-buy (the permanent complied marker)"
    #[test]
    fn repeat_extraction_is_impossible() {
        let mut st = mk_world(true);
        do_act("mob", "threaten", &mut st);
        do_act("vic", "favor", &mut st);
        do_act("mob", "threaten", &mut st);
        assert!(
            st.db_has("complied.racket.mob.vic"),
            "the permanent complied marker survives"
        );
        assert!(
            st.db_has("threatened.racket.mob.vic"),
            "the renewed threat is standing"
        );
        assert!(
            !offers("vic", "favor", &mut st),
            "comply is not offered a second time"
        );
        let vic = member(&st, "vic");
        assert!(
            !st.pick_action(2, &vic)
                .is_some_and(|ga| ga.label.contains("favor")),
            "the victim's chosen response is not a re-buy"
        );
    }

    // H: CoerceSpec.hs "property 5: credibility is self-motivation (the vengeance, base-less kernel)"
    // H: CoerceSpec.hs "the extorter CHOOSES to threaten at depth 2, holding the punitive want"
    #[test]
    fn the_extorter_chooses_to_threaten_holding_the_punitive_want() {
        let mut st = mk_world(true);
        let mob = member(&st, "mob");
        assert_eq!(
            st.pick_action(2, &mob).map(|ga| ga.label),
            Some("mob: threaten vic".to_owned())
        );
    }

    // H: CoerceSpec.hs "without the punitive want, the same choice collapses to doing nothing"
    #[test]
    fn without_the_punitive_want_the_choice_collapses() {
        let mut st = mk_world(false);
        let mob = member(&st, "mob");
        assert_eq!(
            st.pick_action(2, &mob).map(|ga| ga.label),
            Some("mob: bide".to_owned())
        );
    }

    // H: CoerceSpec.hs "v54 property 1: Nothing is today (the default markers compile to permanent Inserts)"
    // H: CoerceSpec.hs "the racket's compiled threaten and comply carry Insert for both markers, no InsertFor"
    #[test]
    fn nothing_is_today_the_default_markers_are_permanent_inserts() {
        let acts = acts_of(&racket());
        let (threaten, comply) = (&acts[0], &acts[1]);
        assert!(
            threaten.then.contains(&insert("threatened.racket.Actor.V")),
            "threaten's marker is a permanent Insert"
        );
        assert!(
            comply.then.contains(&insert("complied.racket.E.Actor")),
            "comply's complied marker is a permanent Insert"
        );
        assert!(
            !threaten
                .then
                .iter()
                .chain(comply.then.iter())
                .any(is_insert_for),
            "neither marker is an InsertFor (byte-identical to the shipped compilation)"
        );
    }

    // H: CoerceSpec.hs "v54 property 2: the racket cycles under an expiring complied marker"
    // H: CoerceSpec.hs "one purchase per bought period; the marker expires and the extorter extracts again"
    #[test]
    fn the_racket_cycles_under_an_expiring_complied_marker() {
        let mut st = mk_turf_world(&cycling_racket());
        do_act("mob", "threaten", &mut st);
        do_act("vic", "favor", &mut st);
        assert!(
            st.db_has("debt.mob.vic.favor"),
            "the demand fact lands (first extraction)"
        );
        assert!(
            st.db_has("complied.racket.mob.vic"),
            "the complied marker holds after compliance"
        );
        do_act("mob", "threaten", &mut st);
        assert!(
            st.db_has("threatened.racket.mob.vic"),
            "the re-threat stands"
        );
        assert!(
            !offers("vic", "favor", &mut st),
            "comply is BLOCKED within the bought period (the complied marker gates it)"
        );
        boundaries(2, &mut st);
        assert!(
            !st.db_has("complied.racket.mob.vic"),
            "the complied marker expired after 2 boundaries"
        );
        assert!(
            offers("vic", "favor", &mut st),
            "comply is available AGAIN under the still-standing threat"
        );
        do_act("vic", "favor", &mut st);
        assert!(
            !st.db_has("threatened.racket.mob.vic"),
            "the second extraction completes: the threat is bought off again"
        );
        assert!(
            st.db_has("complied.racket.mob.vic"),
            "the complied marker is re-armed for the next bought period"
        );
    }

    // H: CoerceSpec.hs "v54: the deal economics, as observed (the no-fiat ruling — no marker enforces the deal)"
    // H: CoerceSpec.hs "during the bought period the extorter's punish-vs-bide goes where the vengeance utilities point"
    #[test]
    fn the_deal_economics_as_observed() {
        let mut st = mk_turf_world(&cycling_racket());
        do_act("mob", "threaten", &mut st);
        do_act("vic", "favor", &mut st);
        do_act("mob", "threaten", &mut st);
        let mob = member(&st, "mob");
        let scores = st.score_actions(2, &mob);
        let burn = score_of(&scores, "burn");
        let bide = score_of(&scores, "mob: bide");
        let chose = st.pick_action(2, &mob).map(|ga| ga.label);
        // OBSERVED (not guaranteed): burning the barn satisfies the kernel NOW;
        // biding gains nothing and the purse is already taken, so the extorter
        // BETRAYS. Paying bought the purse for the period, never the person.
        assert!(
            burn > bide,
            "observed betrayal: burn={burn} bide={bide} chose={chose:?}"
        );
        assert_eq!(chose, Some("mob: burn vic's barn".to_owned()));
    }

    // H: CoerceSpec.hs "v54 property 3: a stale threat is spent, but a defied threat's punish survives"
    // H: CoerceSpec.hs "after n boundaries an unanswered threat stops offering punish and stops pressuring comply"
    #[test]
    fn a_stale_threat_is_spent() {
        let mut st = mk_turf_world(&expiring_racket());
        do_act("mob", "threaten", &mut st);
        assert!(
            offers("mob", "burn", &mut st),
            "punish (burn) offered while the threat stands"
        );
        boundaries(2, &mut st);
        assert!(
            !st.db_has("threatened.racket.mob.vic"),
            "the threat marker expired after 2 boundaries"
        );
        assert!(
            !offers("mob", "burn", &mut st),
            "punish's standing arm is gone (no burn offered)"
        );
        assert!(
            !offers("vic", "favor", &mut st),
            "comply is no longer pressured (not offered — no standing threat)"
        );
    }

    // H: CoerceSpec.hs "a DEFIED threat's punish survives expiry (the defied marker is permanent)"
    #[test]
    fn a_defied_threats_punish_survives_expiry() {
        let mut st = mk_turf_world(&expiring_racket());
        do_act("mob", "threaten", &mut st);
        do_act("vic", "defy", &mut st);
        boundaries(3, &mut st);
        assert!(
            st.db_has("defied.racket.vic.mob"),
            "the defied marker stands permanently past the threat's expiry"
        );
        assert!(
            offers("mob", "burn", &mut st),
            "burn is still offered via the defied arm, past expiry"
        );
    }

    // H: CoerceSpec.hs "v54 property 5: the bluff pair — registered-not-held vs registered-and-held"
    // H: CoerceSpec.hs "the victim's comply/defy decision is IDENTICAL (the fear resolves on the registered vocabulary)"
    #[test]
    fn the_bluff_pairs_victim_decision_is_identical() {
        let mut g = mk_world(true); // genuine: registered AND held
        let mut b = mk_world(false); // bluff:   registered, NOT held
        do_act("mob", "threaten", &mut g);
        do_act("mob", "threaten", &mut b);
        let gvic = member(&g, "vic");
        let bvic = member(&b, "vic");
        let gs = g.score_actions(2, &gvic);
        let bs = b.score_actions(2, &bvic);
        for needle in ["favor", "vic: defy", "vic: bide"] {
            assert_eq!(score_of(&gs, needle), score_of(&bs, needle), "{needle}");
        }
        assert_eq!(
            g.pick_action(2, &gvic).map(|ga| ga.label),
            b.pick_action(2, &bvic).map(|ga| ga.label)
        );
    }

    // H: CoerceSpec.hs "defied, the genuine extorter picks punish; the bluffer (holding no want) does not"
    #[test]
    fn defied_the_genuine_extorter_punishes_and_the_bluffer_does_not() {
        let mut g = mk_world(true);
        let mut b = mk_world(false);
        for st in [&mut g, &mut b] {
            do_act("mob", "threaten", st);
            do_act("vic", "defy", st);
        }
        let gmob = member(&g, "mob");
        let bmob = member(&b, "mob");
        assert!(
            g.pick_action(2, &gmob)
                .is_some_and(|ga| ga.label.contains("burn")),
            "genuine mob punishes after defiance"
        );
        assert!(
            !b.pick_action(2, &bmob)
                .is_some_and(|ga| ga.label.contains("burn")),
            "the bluffing mob declines punish (its choice is bide, not burn)"
        );
    }

    // H: CoerceSpec.hs "v54 property 7: the table turns — counter-coercion is composition"
    // H: CoerceSpec.hs "once extorted, the victim reaches a standing counter-threat from pure content over the shipped surface"
    #[test]
    fn counter_coercion_is_composition() {
        let mut st = counter_world();
        do_act("mob", "threaten", &mut st);
        assert!(
            st.db_has("mob.extorted.vic.racket"),
            "mob's threaten deposited the extorted mark the counter reads"
        );
        assert!(
            offers("vic", "threaten to expose", &mut st),
            "vic can now counter-threaten mob (the variable extorter slot bound M to mob)"
        );
        let vic = member(&st, "vic");
        assert_eq!(
            st.pick_action(2, &vic).map(|ga| ga.label),
            Some("vic: threaten to expose mob".to_owned())
        );
        do_act("vic", "threaten to expose", &mut st);
        assert!(
            st.db_has("threatened.racket.mob.vic"),
            "the racket threat stands"
        );
        assert!(
            st.db_has("threatened.counter.vic.mob"),
            "the counter threat stands — both threats live at once"
        );
    }

    /// The rename kernel's own law, asserted structurally rather than through a
    /// rendered substring: FIRST-APPEARANCE order, `Owner` exempt, and NO
    /// `PraxW1`. A set-based free-variable collection would sort `A`/`B`/`Z` and
    /// rename differently while still reading plausibly (S7 design §3.1) — this
    /// is the pin that catches it, and it carries no frozen label because the
    /// frozen spec asserts the same law only through `isInfixOf` on one kernel.
    #[test]
    fn namespace_kernel_numbers_in_first_appearance_order_never_praxw1() {
        // `Z` appears first, then `A` — sorted order would swap them.
        let kernel = vec![
            matches("burned.barn.Z"),
            matches("ally.A.of.V"),
            matches("knows.Owner.A"),
        ];
        assert_eq!(
            namespace_kernel("V", &kernel).unwrap(),
            vec![
                matches("burned.barn.PraxW"),
                matches("ally.PraxW2.of.PraxD"),
                matches("knows.Owner.PraxW2"),
            ],
            "first appearance order (Z before A), Owner exempt, victim to PraxD, no PraxW1"
        );
    }

    /// The rename is op-PRESERVING and moves a `Subquery` binder with its
    /// interior uses — the two properties `rename_vars` exists for.
    #[test]
    fn namespace_kernel_preserves_operators_and_moves_binders() {
        use prax_core::query::{cmp, count, subquery};
        let kernel = vec![
            matches("hoard.V!W"),
            subquery("S", vec!["W".to_owned()], vec![matches("saw.W.V")]),
            count("N", "S"),
            cmp(CmpOp::Gte, "N", "2"),
        ];
        assert_eq!(
            namespace_kernel("V", &kernel).unwrap(),
            vec![
                matches("hoard.PraxD!PraxW"),
                subquery(
                    "PraxW2",
                    vec!["PraxW".to_owned()],
                    vec![matches("saw.PraxW.PraxD")]
                ),
                count("PraxW3", "PraxW2"),
                cmp(CmpOp::Gte, "PraxW3", "2"),
            ]
        );
    }
}
