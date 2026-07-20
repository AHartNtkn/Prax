//! The village: the proving ground for the sandbox arc (spec
//! `docs/specs/2026-07-10-v19-witnessing-design.md`).
//!
//! v19 seeds it with the witnessing keystone: bob steals a loaf in the square;
//! whoever is THERE comes to believe it and can act on the belief — whoever isn't,
//! doesn't and can't. v20 makes the news travel: carol tells; hearsay licenses
//! suspicion, not confrontation. v21 completes the arc: evidence settles into
//! derived standing, notoriety tips the thief into atonement, and forgiveness
//! follows; nothing is ever forgotten. v22 adds the adversarial layer: bob keeps
//! his secret by planning (waiting for an empty square), and eve's whispered
//! fabrication cascades through the same rumor/reputation machinery as truth. v23
//! wires sight in. v24 completes the moral arc: deterrence plus opportunity yields
//! industry (bob earns the loaf he cannot safely steal).
//!
//! v25 gives temperament teeth: eve and gale carry the same named spite, but gale
//! bears the honest trait — a conscience valuing her own lie-marks — so eve
//! whispers and gale never does, and anyone told of both spites predicts the
//! difference.
//!
//! Frozen reference: `src/Prax/Worlds/Village.hs`. This is the MECHANISM-DENSEST
//! world in the program: it is the only one that seeds the die and draws from it,
//! the only one carrying a v37 gathering, an `endeavor` part-set, a coercion, a
//! confession/absolution pair and the sight window all at once. Construction ORDER
//! is part of the port — practices, functions, cast, schedule, the setup outcomes
//! then the persona facts, the axioms, the desires, the die seed, the prediction
//! scope — exactly as the frozen world nests them; `worldshape village` compares
//! the whole post-setup state ([S-C6]).

use prax_core::engine::State;
use prax_core::query::{CmpOp, Condition, absent, cmp, exists, matches, neq, not_, or_};
use prax_core::rng::draw;
use prax_core::schedule::{gathering, sight_rule};
use prax_core::sight::sighted_within;
use prax_core::types::{
    Action, Character, Desire, Outcome, Practice, ScheduleRule, Want, delete, insert,
};
use prax_vocab::blackmail::shakedown;
use prax_vocab::confession::{absolve, confess, incorrigible};
use prax_vocab::core_model::{adjust_score, core_fns};
use prax_vocab::debt::owes;
use prax_vocab::deceit::{conceal, lie};
use prax_vocab::deontic::obliged_close;
use prax_vocab::emotion::{ANGRY, feel_toward_for, feeling_toward, unfeel_toward};
use prax_vocab::persona::{Trait, cast, persona_vocabulary, transparent};
use prax_vocab::project::{Part, endeavor};
use prax_vocab::repute::{notoriety, regarded_as, standing_unless};
use prax_vocab::rumor::{gossip, heard};
use prax_vocab::witness::{CoPresence, observable, saw, witnessed};

use prax_core::types::Axiom;

/// You are a villager — one agent among many.
pub const PLAYER_NAME: &str = "you";

/// Co-presence in the village: sharing a place.
///
/// `pub` for the same reason `bar::together` is: `conformance::witness_templates`
/// runs the [S-I6] `as_role` equality against THIS template rather than a copy.
pub fn together() -> CoPresence {
    vec![
        matches("practice.world.world.at.Actor!P"),
        matches("practice.world.world.at.Witness!P"),
    ]
}

/// The village's sighting template, over the same movement vocabulary as
/// [`together`]: whoever shares a place with you is someone you see.
fn village_sighting() -> Vec<Condition> {
    vec![
        matches("practice.world.world.at.Seer!Spot"),
        matches("practice.world.world.at.Seen!Spot"),
    ]
}

/// Places and movement, in the bar's idiom.
fn world_practice() -> Practice {
    Practice::new("world")
        .name("The village exists")
        .roles(["World"])
        .action(
            Action::new("[Actor]: Go to [Place]")
                .when([
                    matches("practice.world.World.at.Actor!OtherPlace"),
                    matches("practice.world.World.connected.OtherPlace.Place"),
                ])
                .then([insert("practice.world.World.at.Actor!Place")]),
        )
        .action(
            Action::new("[Actor]: Wait a moment")
                .when([matches("practice.world.World.at.Actor!Place")]),
        )
}

/// Honest work: the lawful path to bread. Progress itself satisfies (+3 a part —
/// real, but no substitute for bread in hand); the culminating part earns the loaf
/// bob's +10 want has stared at since v19. Sweeping is public — honest work is done
/// in the open — so watching bob work teaches the village his purpose (the
/// inference axiom below). The three parts form a chain (fetch after sweep, bake
/// after fetch): these two edges ARE the transcript-identity claim — omitting one
/// would make the parts parallel.
fn earn_bread() -> (Action, Practice, Desire) {
    endeavor(
        "earnBread",
        3,
        "[Actor]: take up honest work at the stall",
        vec![matches("practice.world.world.at.Actor!square")],
        &[
            Part::new(
                "sweep",
                "[Actor]: sweep the square",
                Vec::new(),
                vec![matches("practice.world.world.at.Actor!square")],
                vec![witnessed(&together(), "swept.Actor")],
            ),
            Part::new(
                "fetch",
                "[Actor]: fetch flour from the mill",
                vec!["sweep".to_owned()],
                vec![matches("practice.world.world.at.Actor!mill")],
                vec![insert("carrying.Actor.flour")],
            ),
            Part::new(
                "bake",
                "[Actor]: bake and earn the loaf",
                vec!["fetch".to_owned()],
                vec![
                    matches("practice.world.world.at.Actor!square"),
                    matches("carrying.Actor.flour"),
                ],
                vec![
                    delete("carrying.Actor.flour"),
                    insert("holding.Actor.loaf"),
                ],
            ),
        ],
    )
    .expect("the village's earnBread endeavor")
}

/// Hunger, the build-up cargo (v36): an episodic engine-schedule rule that closes
/// bob's bread economy into a cycle instead of a one-shot want. TEST-COMPRESSED
/// cadence: hunger every three rounds keeps the cycle inside short test drives;
/// real authoring is ~72 rounds. The engine re-arms the rule a period FROM its
/// firing, so a fed bob stays fed for a full period regardless of when he ate.
fn hunger_pulse() -> ScheduleRule {
    ScheduleRule::new("hunger", 3).clause(
        [matches("appetite.X"), not_("hungry.X")],
        [insert("hungry.X")],
    )
}

/// The drama die's seed (`prax_core::rng`): an authored world parameter that
/// selects THIS playthrough's fate — every draw below reads off this one stream,
/// and the goldens pin its consequences. Picked as a nod to Park & Miller's own
/// 1988 publication. At this seed, the golden's own dramatic beat — dana's shun of
/// carol — draws a hit on both arms (computed: `lehmerNext(1988) mod 4 == 0`, so
/// the base 1-in-4 arm alone already lands; `lehmerNext(lehmerNext(1988)) mod 4 ==
/// 1 < 2`, so the trait arm would have too).
const VILLAGE_SEED: i64 = 1988;

/// Hunger, when it arrives, outranks pride and larder both: eating spends the +10
/// loaf AND forfeits the finished endeavor's +9 part credit (3 parts × 3, torn down
/// by the eat), so the relief must beat 19 — at -22, a hungry bob eats (net +3) and
/// the emptied hand re-opens the earn cycle.
fn suffers_hunger() -> Desire {
    Desire::new(
        "suffers-hunger",
        Want::new(vec![matches("hungry.Owner")], -22),
    )
}

/// Market day (v37): a recurring convening on the sight clock. The market
/// practice: an instance the calendar spawns and tears down; its presence IS the
/// event (no market-only affordances — the draw is the valuation, the payoff is
/// co-presence density feeding sight/witnessing).
fn market_practice() -> Practice {
    Practice::new("market").roles(["Fair"])
}

/// Market day: TEST-COMPRESSED cadence. Every sixth round, lasting one — most days
/// are quiet. A [`gathering`]: one open rule whose inserts each carry `lasts 1`, so
/// the engine's expiry queue tears the market down a round later (the v37 close
/// rule is subsumed by expiry — one mechanism).
fn market_gathering() -> ScheduleRule {
    gathering(
        "market",
        6,
        1,
        vec![insert("practice.market.fair"), insert("marketDay.square")],
    )
    .expect("the village's market gathering")
}

/// Everyone likes a market day: +3 for being at the square while it's on — above
/// the +1 loitering anchors, below the +5 event wants and every conduct stake
/// (drama outranks festivity).
fn drawn_to_market() -> Desire {
    Desire::new(
        "drawn-to-market",
        Want::new(
            vec![
                matches("marketDay.square"),
                matches("practice.world.world.at.Owner!square"),
            ],
            3,
        ),
    )
}

/// Temperament: honesty as conduct, not prohibition. The weight is authored
/// meaning: a lie costs gale 6 per hearer per event — more than the +4 a deceived
/// head's contempt for carol is worth to her spite (so at these stakes it never
/// pays), yet nothing forbids it.
///
/// The confessed form is priced identically (v32's own "0 or a mild authored
/// residue" — this trait authors the FULL residue): a truly honest person's
/// conscience is about having deceived at all, not about being caught or having
/// covered it up, so confessing doesn't relieve it. Without this second desire,
/// `confess` opens a cheap-grace loophole this trait's own design forbids — found
/// by measurement (a VillageSpec regression), not by inspection.
fn honest() -> Trait {
    Trait::new(
        "honest",
        vec![
            Desire::new(
                "clean-conscience",
                Want::new(vec![matches("Owner.lied.H.stole.C.loaf")], -6),
            ),
            Desire::new(
                "conscience-remembers",
                Want::new(vec![matches("Owner.confessed.H.stole.C.loaf")], -6),
            ),
        ],
    )
}

/// Pricing the smoulder (v38): anger as discomfort, driving its own discharge. -8
/// outweighs carol's own +5 event wants so she acts to relieve it when she can.
/// Bound to a real target ([`feeling_toward`] with a fresh variable, not the bare
/// subtree `feeling` Match) for its PER-TARGET semantics: two grudges smoulder
/// twice as hot (-8 each), and confront's discharge lifts exactly the vented
/// grudge's price.
fn smoulders() -> Desire {
    Desire::new(
        "smoulders",
        Want::new(vec![feeling_toward("Owner", ANGRY, "T")], -8),
    )
}

/// Malice with a name: wanting carol ill-regarded, per head. Naming it makes it
/// believable (a told-about spite enters prediction) but it stays unheralded —
/// nothing professes or derives it, so until someone is told it is exactly as
/// unreadable as the unnamed want it replaces (v22's eve).
fn spites_carol() -> Desire {
    Desire::new(
        "spites-carol",
        Want::new(vec![matches("regards.W.carol.thief")], 4),
    )
}

/// The shakedown: carol, who already holds eyewitness evidence eve whispers,
/// presses her for a favor. Threshold fear (v30 §3, bob's own idiom): notoriety is
/// nonlinear, so this serves both masters a per-head cost couldn't — free below the
/// brink, catastrophic at it.
fn whisper_shakedown() -> (Desire, Vec<Action>) {
    shakedown("whisper", &together(), "whispered.V.H", "favor", 6)
        .expect("the village's whisper shakedown")
}

/// Fabrication: assert a theft you have no evidence for, binding the scapegoat from
/// the village roster. The deceived hold real hearsay — the whole rumor/reputation
/// stack cascades on the falsehood, and nobody in the village can tell it from
/// truth. Re-offending forfeits any amends already made for the SLANDER (v21's
/// re-steal idiom): a fresh whisper deletes the standing defeater, snapping
/// notoriety back from the beliefs nobody lost, before a now-less-patient audience.
fn whisper_act() -> Action {
    let mut raw_lie = lie(
        &together(),
        vec![absent(vec![
            matches("Actor.relationship.Hearer.trust.score!TrustScore"),
            cmp(CmpOp::Lt, "TrustScore", "0"),
        ])],
        vec![matches("practice.world.world.at.Culprit!AnywhereQ")],
        "stole.Culprit.loaf",
        "[Actor]: whisper to [Hearer] that [Culprit] stole the loaf",
    )
    .expect("the village's whisper");
    raw_lie.then.push(delete("recanted.Actor"));
    observable(&together(), "whispered.Actor.Hearer", raw_lie)
}

/// Confession & absolution over the whisper (v32): eve's conscience-mark from
/// [`whisper_act`] is CONTENT-shaped (`stole.C.loaf` — who she framed), but her
/// slanderer standing derives from the ACT (`whispered.V.H`). One pattern cannot
/// serve both what the mark IS and what confessing it REVEALS: the MARK pattern
/// matches her content-shaped conscience; the DEPOSIT pattern is the act-shaped
/// truth confessing it reveals, grounded straight from the mark's own bindings
/// (`Actor`, `H`) — not a re-assertion of the fabricated content.
fn confess_whisper() -> Action {
    confess(
        "lied",
        "confessed",
        &together(),
        "stole.C.loaf",
        "whispered.Actor.H",
        "[Actor]: confess to [Hearer] about framing [C]",
    )
    .expect("the village's confession")
}

fn absolve_whisper() -> Action {
    absolve(
        "recanted",
        "whispered.V.H",
        "incorrigible",
        "[Actor]: absolve [V] of slander",
    )
    .expect("the village's absolution")
}

/// Village life: the theft (observable) and the belief-gated confrontation.
///
/// The role is named `Scene` (a v30 rename): the singleton instance key silently
/// pre-binds any action-local variable of the same name before that action's own
/// conditions are ever evaluated, so a role named `V` collided with the shakedown
/// evidence-pattern convention.
fn village_practice() -> Practice {
    let (_, shakedown_acts) = whisper_shakedown();
    let [threaten, comply, defy, expose] = <[Action; 4]>::try_from(shakedown_acts)
        .expect("whisperShakedown: expected 4 actions");
    Practice::new("village")
        .name("Village life")
        .roles(["Scene"])
        // Anyone at the stall can steal — bob is merely the one who wants to.
        .action(observable(
            &together(),
            "stole.Actor.loaf",
            Action::new("[Actor]: steal the loaf from the stall")
                .when([
                    matches("practice.world.world.at.Actor!square"),
                    matches("stall.loaf"),
                ])
                .then([
                    delete("stall.loaf"),
                    insert("holding.Actor.loaf"),
                    // stealing again forfeits any amends you'd made: standing (and
                    // notoriety) re-derive instantly from the beliefs nobody lost
                    delete("atoned.Actor"),
                ]),
        ))
        // Only someone who SAW the theft can call it out; it cools them toward the
        // thief. dana, who was elsewhere, never gets this affordance. Discharge
        // (v38): confronting vents any anger held toward the very person
        // confronted — the smoulder's own outlet.
        .action(
            Action::new("[Actor]: confront [Thief] about the theft")
                .when([
                    saw("Actor", "stole.Thief.loaf"),
                    matches("practice.world.world.at.Actor!P"),
                    matches("practice.world.world.at.Thief!P"),
                    not_("confronted.Actor.Thief"),
                ])
                .then([
                    insert("confronted.Actor.Thief"),
                    adjust_score("Actor", "Thief", "trust", -10, "sawTheft"),
                    unfeel_toward("Actor", ANGRY, "Thief"),
                ]),
        )
        // Word travels: anyone with evidence can pass it on. Never told: bob (the
        // subject), an eyewitness (no news value), or the same hearer twice. The
        // village's own gate: you don't gossip with someone you distrust.
        .action(
            gossip(
                &together(),
                vec![absent(vec![
                    matches("Actor.relationship.Hearer.trust.score!TrustScore"),
                    cmp(CmpOp::Lt, "TrustScore", "0"),
                ])],
                "stole.Culprit.loaf",
                "[Actor]: tell [Hearer] that [Culprit] stole the loaf",
            )
            .expect("the village's gossip"),
        )
        // Hearsay licenses suspicion, not confrontation — and an eyewitness
        // confronts instead (seen suppresses the milder act).
        .action(
            Action::new("[Actor]: eye [Thief] with suspicion")
                .when([
                    matches("practice.world.world.at.Actor!P"),
                    matches("practice.world.world.at.Thief!P"),
                    neq("Thief", "Actor"),
                    heard("Actor", "stole.Thief.loaf"),
                    absent(vec![matches("Actor.believes.stole.Thief.loaf.seen")]),
                    not_("eyed.Actor.Thief"),
                ])
                .then([
                    insert("eyed.Actor.Thief"),
                    adjust_score("Actor", "Thief", "trust", -5, "heardOfTheft"),
                ]),
        )
        // Standing has teeth: anyone who has come to regard [T] a thief may shun
        // them — reputation (a derived fact) gating behaviour. Being shunned stings
        // (v38): anyone might flare (1 in 4 — the direct, victim-present
        // provocation); a short temper flares on most slights too (a further 2 in
        // 4, so 3 in 4 overall for the short-tempered — each arm's odds an authored
        // sentence; the trait makes the feeling LIKELIER, never longer). Two draws,
        // two stream steps, by design; a double hit's insert is idempotent. The
        // anger carries a lifetime (v44): each onset lives 4 round boundaries from
        // when it flared — TEST-COMPRESSED, replacing v38's synchronized fade sweep
        // with a per-onset span.
        .action(Action::new("[Actor]: shun [T]").when([
            regarded_as("Actor", "T", "thief"),
            neq("T", "Actor"),
            not_("shunned.Actor.T"),
        ]).then(shun_outcomes()))
        // Atonement, not amnesia: returning the loaf defeats the standing — every
        // regard dissolves on the next read — while every belief (the memory of the
        // deed) persists untouched.
        .action(
            Action::new("[Actor]: return the loaf with apologies")
                .when([
                    matches("holding.Actor.loaf"),
                    exists(vec![matches("regards.W.Actor.thief")]),
                ])
                .then([
                    delete("holding.Actor.loaf"),
                    insert("stall.loaf"),
                    insert("atoned.Actor"),
                ]),
        )
        // Forgiveness: you don't keep shunning someone you no longer condemn.
        .action(
            Action::new("[Actor]: relent toward [T]")
                .when([
                    matches("shunned.Actor.T"),
                    absent(vec![matches("regards.Actor.T.thief")]),
                ])
                .then([delete("shunned.Actor.T")]),
        )
        // Fabrication, and its road back: confessing self-incriminates through the
        // ordinary hearsay channel; absolution is a refusable second-party grant.
        .action(whisper_act())
        .action(confess_whisper())
        .action(absolve_whisper())
        // The lawful alternative to the stall's temptation.
        .action(earn_bread().0)
        // Closing the loop: hunger is a physical precondition of eating (the
        // ordinary practice sense, like holding the loaf), so the
        // emotions-never-gate discipline does not apply here.
        .action(
            Action::new("[Actor]: eat the loaf")
                .when([matches("hungry.Actor"), matches("holding.Actor.loaf")])
                .then([
                    delete("hungry.Actor"),
                    delete("holding.Actor.loaf"),
                    // eating ends the finished bread-project: the endeavor instance
                    // is torn down, so undertake's own Not-gate re-opens and the
                    // work can begin again. Delete on the instance path reaps the
                    // whole ledger subtree (every `did.<part>` fact) in one stroke.
                    // A no-op for eaters who never baked.
                    delete("practice.earnBread.Actor"),
                ]),
        )
        .action(threaten)
        .action(comply)
        .action(defy)
        .action(expose)
}

/// The shun's outcome list: the marker plus the TWO draws off the one die stream.
fn shun_outcomes() -> Vec<Outcome> {
    let mut outs = vec![insert("shunned.Actor.T")];
    outs.extend(
        draw(
            1,
            4,
            Vec::new(),
            vec![feel_toward_for(4, "T", ANGRY, "Actor")],
        )
        .expect("the village's base flare draw"),
    );
    outs.extend(
        draw(
            2,
            4,
            vec![matches("shortTempered.T")],
            vec![feel_toward_for(4, "T", ANGRY, "Actor")],
        )
        .expect("the village's short-temper flare draw"),
    );
    outs
}

/// Reputation: evidence settles into standing (defeated by atonement, not by
/// forgetting), and three regarders — the whole village save the thief — make it
/// common knowledge.
fn village_axioms() -> Vec<Axiom> {
    vec![
        standing_unless("stole.Culprit.loaf", "atoned.Culprit", "thief")
            .expect("the thief standing"),
        notoriety("thief", 3),
        // Watching him work teaches you his purpose: a witnessed sweep is enough to
        // presume the pursuit (v21's inference pattern, aimed at a mind).
        Axiom::new(
            vec![matches("Regarder.believes.swept.bob")],
            ["Regarder.believes.desires.bob.pursues-earnBread.presumed"],
        ),
        // temperament is worn on the sleeve: the whole village presumes a bearer's
        // conduct-valuations from t=0 (v25)
        transparent(),
        // Threshold fear, bob's own idiom (v30 §3): standing derives per believer
        // of the whispering ACT (content stays secret); "recanted" names the
        // never-exercised defeater, kept for symmetry with `stole.Culprit.loaf`'s
        // own standingUnless.
        standing_unless("whispered.V.H", "recanted.V", "slanderer")
            .expect("the slanderer standing"),
        notoriety("slanderer", 3),
        // Fed-up-ness is knowledge, not bookkeeping (v32 spec point 4): an
        // absolver's patience is spent once she personally believes (witnessed or
        // told, provenance doesn't matter) 2 DISTINCT whispered-lie instances by the
        // same person — a "two strikes" threshold. Permanent by memory and per
        // absolver.
        incorrigible("whispered.V.H", 2, "incorrigible").expect("the incorrigibility axiom"),
    ]
}

/// The village roster and the persona facts `transparent` reads.
fn village_cast() -> (Vec<Character>, Vec<Outcome>) {
    cast(
        &[honest()],
        vec![
            (Character::new("you"), Vec::new()),
            (
                Character::new("bob")
                    .want(Want::new(vec![matches("holding.bob.loaf")], 10))
                    // loiters near the stall (the bar's anchoring idiom: an idle
                    // character needs a place it wants to be, or it drifts on
                    // tie-break)
                    .want(Want::new(
                        vec![matches("practice.world.world.at.bob!square")],
                        1,
                    ))
                    // bob can live with individuals' contempt; being the village's
                    // NOTORIOUS thief outweighs the bread
                    .want(Want::new(vec![matches("notorious.bob.thief")], -15))
                    // and better still that no one ever knows: the bread is worth
                    // +10, the secret is worth more (unnamed wants are inherently
                    // unreadable in prediction — this is how bob's concealment
                    // stays secret)
                    .want(conceal("stole.bob.loaf", 12).expect("bob's concealment"))
                    // his disposition to honest work: dormant until he takes it up
                    .holds("pursues-earnBread")
                    .holds("suffers-hunger")
                    .holds("drawn-to-market"),
                Vec::new(),
            ),
            (
                Character::new("carol")
                    .want(Want::new(vec![matches("confronted.carol.T")], 5))
                    // keeps to the square unless something needs doing (genuinely
                    // needed once bob conceals — with no early theft her first turns
                    // are zero-utility ties, and unanchored she wanders off)
                    .want(Want::new(
                        vec![matches("practice.world.world.at.carol!square")],
                        1,
                    ))
                    // carol wants others to hear WHAT SHE KNOWS from her — an
                    // unconditioned "believe it from me" would be satisfiable by
                    // fabrication once `lie` exists
                    .want(Want::new(
                        vec![
                            matches("carol.believes.stole.bob.loaf"),
                            matches("Other.believes.stole.bob.loaf.heard.carol"),
                        ],
                        5,
                    ))
                    .want(Want::new(
                        vec![
                            matches("shunned.carol.T"),
                            matches("regards.carol.T.thief"),
                        ],
                        5,
                    ))
                    .want(Want::new(
                        vec![
                            matches("shunned.carol.T"),
                            absent(vec![matches("regards.carol.T.thief")]),
                        ],
                        -5,
                    ))
                    // the shakedown's price: carol wants the favor eve's
                    // silence-money buys her (small — the punitive desire is what
                    // motivates the threat; this just makes the payoff concrete)
                    .want(Want::new(
                        vec![owes("carol", "eve", "favor").expect("carol's favor")],
                        4,
                    ))
                    .holds("punishes-whisper")
                    .holds("drawn-to-market")
                    .holds("smoulders"),
                Vec::new(),
            ),
            (
                Character::new("dana")
                    .want(Want::new(vec![matches("confronted.dana.T")], 5))
                    .want(Want::new(vec![matches("eyed.dana.T")], 5))
                    .want(Want::new(
                        vec![matches("shunned.dana.T"), matches("regards.dana.T.thief")],
                        5,
                    ))
                    .want(Want::new(
                        vec![
                            matches("shunned.dana.T"),
                            absent(vec![matches("regards.dana.T.thief")]),
                        ],
                        -5,
                    ))
                    // loiters near the mill: she keeps to her own place rather than
                    // drifting on the wander/wait tie-break
                    .want(Want::new(
                        vec![matches("practice.world.world.at.dana!mill")],
                        1,
                    ))
                    .holds("drawn-to-market"),
                Vec::new(),
            ),
            // eve's authored malice, named vocabulary since v25: the same +4/head
            // spite gale carries, and — unheralded — exactly as unreadable as the
            // unnamed want it replaces. Her own threshold fear (v30 §3) mirrors
            // bob's notorious -15 exactly.
            (
                Character::new("eve")
                    .want(Want::new(vec![matches("notorious.eve.slanderer")], -15))
                    .holds("spites-carol")
                    .holds("drawn-to-market"),
                Vec::new(),
            ),
            // gale: eve's contrast pair. The same spite, plus a temperament — her
            // conscience (-6/lie) outprices what any single whisper buys (+4/head),
            // so eve whispers and gale never does.
            (
                Character::new("gale")
                    .holds("spites-carol")
                    .holds("drawn-to-market"),
                vec![honest()],
            ),
        ],
    )
    .expect("the village roster")
}

/// The village's setup facts, in declaration order.
fn village_setup() -> Vec<Outcome> {
    vec![
        insert("practice.village.here"),
        insert("practice.world.world.connected.square.mill"),
        insert("practice.world.world.connected.mill.square"),
        insert("practice.world.world.at.you!square"),
        insert("practice.world.world.at.bob!square"),
        insert("practice.world.world.at.carol!square"),
        insert("practice.world.world.at.dana!mill"),
        insert("practice.world.world.at.eve!mill"),
        insert("practice.world.world.at.gale!mill"),
        insert("stall.loaf"),
        insert("appetite.bob"),
        // Temperament, the round's stochastic cargo (v38): a plain disposition
        // fact, not a Trait bundle (it gates a draw's odds, not a conduct-desire) —
        // like every disposition it never fades, unlike the episodic feeling it
        // primes.
        insert("shortTempered.carol"),
    ]
}

/// An epistemic prediction scope: you credit another's predicted move only if
/// you're with them now, or you sighted them within the last 2 ticks — one tick per
/// round, and two rounds is roughly a square↔mill round trip: "you assume people
/// stay put for about as long as it takes to walk there and back."
fn prediction_scope() -> Vec<Condition> {
    vec![or_(vec![together(), sighted_within(2)])]
}

/// The fully initialized village.
///
/// # Panics
/// If a construction guard rejects the authored content — a bug in this file, not
/// a condition a world can handle.
pub fn village_world() -> State {
    let (_, earn_bread_practice, earn_bread_pursuit) = earn_bread();
    let (roster, persona_facts) = village_cast();
    let mut st = State::new();
    st.define_practices([
        world_practice(),
        village_practice(),
        earn_bread_practice,
        market_practice(),
    ])
    .expect("village practices");
    st.define_functions(core_fns())
        .expect("the village's functions");
    st.set_characters(roster).expect("village cast");
    // The engine owns time now (v44): the schedule fires sight (period 1), hunger
    // (period 3), and the market gathering (period 6) at each round boundary, in
    // declaration order — no ticker characters in the roster.
    st.set_schedule(vec![
        sight_rule(village_sighting()),
        hunger_pulse(),
        market_gathering(),
    ])
    .expect("village schedule");
    for o in village_setup().iter().chain(persona_facts.iter()) {
        st.perform_outcome(o).expect("village setup");
    }
    st.set_axioms(obliged_close(&village_axioms()))
        .expect("village axioms");
    let mut desires = vec![
        earn_bread_pursuit,
        spites_carol(),
        whisper_shakedown().0,
        suffers_hunger(),
        drawn_to_market(),
        smoulders(),
    ];
    desires.extend(persona_vocabulary(&[honest()]).expect("the village's persona vocabulary"));
    st.set_desires(desires).expect("village desires");
    st.seed_die(VILLAGE_SEED).expect("the village's die seed");
    st.set_prediction_scope(prediction_scope())
        .expect("village prediction scope");
    st
}
