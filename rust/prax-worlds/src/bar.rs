//! The demo storyworld: movement, greeting, and tending bar, wired to the core
//! model and to reactions & norms.
//!
//! Cast: `you` (the player), `ada` (an NPC bartender) and `bex` (an NPC patron).
//!
//! Social loops:
//!
//! * greeting/serving raise a numeric `warmth` evaluation; once warm enough, the
//!   "buy … a drink" affordance appears (a relationship creating a goal);
//! * a greeting spawns a `respondGreet` reaction — the greeted party can greet
//!   back (mutual warmth), rebuff (both cool), or, if they ignore it, be taken
//!   to task for the snub;
//! * being served spawns a `settleUp` obligation — tip (norm respected, warms
//!   the bartender) or leave the tab (a norm violation that spawns the
//!   bartender's disapproval). An agent given a strong aversion to its own
//!   violation tips rather than stiffs;
//! * the engine's period-1 sighting rule ([`prax_core::schedule::sight_rule`])
//!   keeps everyone's sense of where everyone else is (or was, recently)
//!   current, which is what the planner's belief-relative lookahead needs to
//!   ever predict someone else's move.
//!
//! Frozen reference: `src/Prax/Worlds/Bar.hs` — TWO worlds from one module.
//! Construction ORDER is part of the port: practices, then functions, then cast,
//! then sorts, then the schedule, then the setup outcomes, then the prediction
//! scope, exactly as the frozen world nests them; `worldshape bar` and
//! `worldshape dm` compare the whole post-setup state ([S-C6]).
//!
//! This is the widest integration in the program: schedule rules (metabolism and
//! `sightRule`) with expiries and the v44 supersession, reaction SPAWN,
//! `Subquery`/`Count`/`Cmp`/`Calc` in anger, first-class obligations and
//! violations, and the Script-less DIRECTOR practice (metalevel affordances, a
//! `bound_to` cast member).

use prax_core::engine::State;
use prax_core::query::{
    CalcOp, CmpOp, Condition, calc, cmp, count, eq, matches, neq, not_, or_, subquery,
};
use prax_core::schedule::sight_rule;
use prax_core::sight::sighted_within;
use prax_core::types::{
    Action, Character, Function, Practice, ScheduleRule, Want, call, delete, insert,
};
use prax_vocab::arc::{arc_is, enter_arc};
use prax_vocab::beliefs::{belief_sentence, believe, forget};
use prax_vocab::conversation::{
    begin_conversation, change_subject, end_conversation, quip, talk_path,
};
use prax_vocab::core_model::{WARMTH, adjust_score, core_fns, score_at_least};
use prax_vocab::deontic::{breach, discharge, oblige, oblige_reparative};
use prax_vocab::emotion::{ANNOYED, HAPPY, PLEASED, SAD, feel_toward_for, feeling_toward};
use prax_vocab::reactions::{end_reaction, reaction_path, spawn_reaction, violation_of};
use prax_vocab::witness::CoPresence;

/// The character the human player controls.
pub const PLAYER_NAME: &str = "you";

/// The name of the drama-manager the player controls in [`bar_director_world`].
pub const DIRECTOR_NAME: &str = "director";

/// Once your warmth toward someone reaches this, you may buy them a drink.
const BUY_THRESHOLD: i32 = 15;

// Practices -------------------------------------------------------------------

/// Co-presence at the bar: sharing a place.
fn together() -> CoPresence {
    vec![
        matches("practice.world.world.at.Actor!P"),
        matches("practice.world.world.at.Witness!P"),
    ]
}

/// The bar's sighting template, over the same movement vocabulary as
/// [`together`]: whoever shares a place with you is someone you see.
fn bar_sighting() -> Vec<Condition> {
    vec![
        matches("practice.world.world.at.Seer!Spot"),
        matches("practice.world.world.at.Seen!Spot"),
    ]
}

/// Locations and movement between connected places.
fn world_practice() -> Practice {
    Practice::new("world")
        .name("The world exists")
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

/// Initiating a greeting (and the warmth-gated gift of a drink). The RESPONSE to
/// a greeting lives in [`respond_greet_practice`].
fn greet_practice() -> Practice {
    let mut strike_up = vec![
        matches("practice.world.world.at.Actor!Place"),
        matches("practice.world.world.at.Other!Place"),
        neq("Actor", "Other"),
        not_(belief_sentence("Actor", "resentedBy.Other", "yes")),
        // not already talking (either order)
        not_(talk_path("Actor", "Other")),
        not_(talk_path("Other", "Actor")),
        // one conversation per pair
        not_("Actor.chattedWith.Other"),
    ];
    strike_up.extend(score_at_least("Actor", "Other", WARMTH, BUY_THRESHOLD));

    let mut buy = vec![
        matches("practice.world.world.at.Actor!Place"),
        matches("practice.world.world.at.Other!Place"),
        neq("Actor", "Other"),
        not_("practice.greet.World.grievance.Actor.Other"),
        not_(belief_sentence("Actor", "resentedBy.Other", "yes")),
        not_("practice.greet.World.bought.Actor.Other"),
    ];
    buy.extend(score_at_least("Actor", "Other", WARMTH, BUY_THRESHOLD));

    Practice::new("greet")
        .name("People can greet one another")
        .roles(["World"])
        .action(
            Action::new("[Actor]: Greet [Other]")
                .when([
                    matches("practice.world.world.at.Actor!Place"),
                    matches("practice.world.world.at.Other!Place"),
                    neq("Actor", "Other"),
                    not_("practice.greet.World.greeted.Actor.Other"),
                    // wary of those you think dislike you
                    not_(belief_sentence("Actor", "resentedBy.Other", "yes")),
                    // if they've greeted you and you owe a response, use that,
                    // not a new greeting
                    not_(reaction_path("respondGreet", &["Other", "Actor"])),
                ])
                .then([
                    insert("practice.greet.World.greeted.Actor.Other"),
                    adjust_score("Actor", "Other", WARMTH, 10, "greeting"),
                    feel_toward_for(4, "Actor", PLEASED, "Other"),
                    spawn_reaction("respondGreet", &["Actor", "Other"]),
                ]),
        )
        .action(
            Action::new("[Actor]: Warn [Hearer] that [Subject] resents them")
                .when([
                    matches("practice.world.world.at.Actor!Place"),
                    matches("practice.world.world.at.Hearer!Place"),
                    neq("Actor", "Hearer"),
                    // you must actually be cross with Subject
                    feeling_toward("Actor", ANNOYED, "Subject"),
                    neq("Actor", "Subject"),
                    neq("Hearer", "Subject"),
                    // behind Subject's back
                    not_("practice.world.world.at.Subject!Place"),
                    not_(belief_sentence("Hearer", "resentedBy.Subject", "yes")),
                ])
                // a possibly-false rumour
                .then([believe("Hearer", "resentedBy.Subject", "yes")]),
        )
        .action(
            Action::new("[Actor]: Realize [Subject] doesn't resent you after all")
                .when([
                    matches(belief_sentence("Actor", "resentedBy.Subject", "yes")),
                    // they greeted you: evidence
                    matches("practice.greet.world.greeted.Subject.Actor"),
                ])
                .then([
                    forget("Actor", "resentedBy.Subject"),
                    feel_toward_for(4, "Actor", PLEASED, "Subject"),
                ]),
        )
        .action(
            Action::new("[Actor]: Strike up a conversation with [Other]")
                .when(strike_up)
                .then(begin_conversation("Actor", "Other", "smallTalk")),
        )
        .action(
            Action::new("[Actor]: Buy [Other] a drink")
                .when(buy)
                .then([
                    insert("practice.greet.World.bought.Actor.Other"),
                    adjust_score("Other", "Actor", WARMTH, 15, "boughtMeADrink"),
                    adjust_score("Actor", "Other", WARMTH, 5, "feelingGenerous"),
                    feel_toward_for(4, "Actor", PLEASED, "Other"),
                    feel_toward_for(4, "Other", PLEASED, "Actor"),
                ]),
        )
}

/// Responding to a greeting: greet back, rebuff, or (for the greeter) take
/// offense that the greeting was ignored. Spawned as
/// `practice.respondGreet.<Greeter>.<Greeted>`.
fn respond_greet_practice() -> Practice {
    Practice::new("respondGreet")
        .name("[Greeted] can respond to [Greeter]'s greeting")
        .roles(["Greeter", "Greeted"])
        .action(
            Action::new("[Actor]: Greet [Greeter] back")
                .when([
                    eq("Actor", "Greeted"),
                    not_(belief_sentence("Greeted", "resentedBy.Greeter", "yes")),
                ])
                .then([
                    insert("practice.greet.world.greeted.Greeted.Greeter"),
                    adjust_score("Greeted", "Greeter", WARMTH, 10, "greetedBack"),
                    feel_toward_for(4, "Greeted", PLEASED, "Greeter"),
                    end_reaction("respondGreet", &["Greeter", "Greeted"]),
                ]),
        )
        .action(
            Action::new("[Actor]: Rebuff [Greeter]")
                .when([eq("Actor", "Greeted")])
                .then([
                    feel_toward_for(4, "Greeted", ANNOYED, "Greeter"),
                    adjust_score("Greeted", "Greeter", WARMTH, -5, "rebuffed"),
                    feel_toward_for(4, "Greeter", SAD, "Greeted"),
                    adjust_score("Greeter", "Greeted", WARMTH, -10, "rebuffedMe"),
                    end_reaction("respondGreet", &["Greeter", "Greeted"]),
                ]),
        )
        .action(
            Action::new("[Actor]: Take offense that [Greeted] ignored your greeting")
                .when([
                    eq("Actor", "Greeter"),
                    not_("practice.greet.world.greeted.Greeted.Greeter"),
                    not_("practice.greet.world.grievance.Greeter.Greeted"),
                ])
                .then([
                    insert("practice.greet.world.grievance.Greeter.Greeted"),
                    feel_toward_for(4, "Greeter", ANNOYED, "Greeted"),
                    adjust_score("Greeter", "Greeted", WARMTH, -15, "snubbedMe"),
                    end_reaction("respondGreet", &["Greeter", "Greeted"]),
                ]),
        )
}

/// A patron: exists to hold a per-person drink counter, seeded on spawn.
fn patron_practice() -> Practice {
    Practice::new("patron")
        .name("[Patron] is a patron")
        .roles(["Patron"])
        .init([
            insert("practice.patron.Patron.drinks!0"),
            // everyone arrives hopeful
            enter_arc("Patron", "hopeful"),
        ])
}

/// A character's inner arc: hopeful arrivals either come to feel they belong (if
/// someone stands them a drink) or withdraw, feeling out of place (if someone
/// comes to resent them). These are internal, high-level choices; the stage a
/// character is in changes what they want (see bex's wants).
fn arc_practice() -> Practice {
    let mut settle = vec![eq("Actor", "Self"), arc_is("Actor", "hopeful")];
    // you've warmed to someone
    settle.extend(score_at_least("Actor", "Friend", WARMTH, 20));

    Practice::new("arc")
        .name("[Self]'s evening")
        .roles(["Self"])
        // The rewarding beat an NPC pursues: once you feel genuinely warm toward
        // someone here, you can decide you belong (bex's +25 want drives this).
        .action(
            Action::new("[Actor]: settle in, feeling you belong here")
                .when(settle)
                .then([
                    enter_arc("Actor", "belonging"),
                    feel_toward_for(4, "Actor", HAPPY, "here"),
                ]),
        )
        // A transformation AGAINST one's desires: sliding into loneliness
        // forecloses the belonging you crave (+25) and is itself dreaded (-25),
        // with no way back — so the utility planner never chooses it. In practice
        // only the player ever does: "true transformation is available only to
        // the player".
        .action(
            Action::new("[Actor]: give up on the evening, resigning yourself to solitude")
                .when([eq("Actor", "Self"), arc_is("Actor", "hopeful")])
                .then([
                    enter_arc("Actor", "lonely"),
                    feel_toward_for(4, "Actor", SAD, "here"),
                ]),
        )
}

/// Tending bar: order, fulfill (which warms the customer and spawns a tip
/// obligation), drink (getting tipsy), and the busy-bar bell.
fn tend_bar_practice() -> Practice {
    Practice::new("tendBar")
        .name("[Bartender] is tending bar at [Place]")
        .roles(["Place", "Bartender"])
        .data_facts([
            "beverageType.beer!alcoholic",
            "beverageType.cider!alcoholic",
            "beverageType.soda!nonalcoholic",
            "beverageType.water!nonalcoholic",
        ])
        .action(
            Action::new("[Actor]: Order [Beverage]")
                .when([
                    neq("Actor", "Bartender"),
                    matches("practice.world.world.at.Actor!Place"),
                    not_("practice.tendBar.Place.Bartender.customer.Actor!order"),
                    not_("practice.tendBar.Place.Bartender.customer.Actor!beverage"),
                    matches("practiceData.tendBar.beverageType.Beverage"),
                ])
                .then([insert(
                    "practice.tendBar.Place.Bartender.customer.Actor!order!Beverage",
                )]),
        )
        .action(
            Action::new("[Actor]: Fulfill [Customer]'s order")
                .when([
                    eq("Actor", "Bartender"),
                    matches("practice.tendBar.Place.Bartender.customer.Customer!order!Beverage"),
                    matches("practice.world.world.at.Bartender!Place"),
                ])
                .then([
                    delete("practice.tendBar.Place.Bartender.customer.Customer!order"),
                    insert("practice.tendBar.Place.Bartender.customer.Customer!beverage!Beverage"),
                    adjust_score("Customer", "Bartender", WARMTH, 8, "servedMeWell"),
                    feel_toward_for(4, "Customer", PLEASED, "Bartender"),
                    spawn_reaction("settleUp", &["Customer", "Bartender"]),
                    // being served creates a real obligation to settle (a
                    // first-class □)
                    oblige("Customer", "Customer.tipped.Bartender"),
                ]),
        )
        .action(
            Action::new("[Actor]: Drink the [Beverage]")
                .when([
                    matches("practice.tendBar.Place.Bartender.customer.Actor!beverage!Beverage"),
                    matches("practiceData.tendBar.beverageType.Beverage!Kind"),
                    matches("practice.patron.Actor.drinks!N"),
                ])
                .then([
                    delete("practice.tendBar.Place.Bartender.customer.Actor!beverage"),
                    call(
                        "recordDrink",
                        vec!["Actor".to_owned(), "Kind".to_owned(), "N".to_owned()],
                    ),
                ]),
        )
        .action(
            Action::new("[Actor]: Ring the bell — busy bar!")
                .when([
                    eq("Actor", "Bartender"),
                    subquery(
                        "Crowd",
                        vec!["C".to_owned()],
                        vec![matches("practice.tendBar.Place.Bartender.customer.C")],
                    ),
                    count("NumCust", "Crowd"),
                    cmp(CmpOp::Gte, "NumCust", "2"),
                    not_("practice.tendBar.Place.Bartender.rang"),
                ])
                .then([insert("practice.tendBar.Place.Bartender.rang")]),
        )
}

/// The bar's drink-counter functions (the world registry, registered via
/// [`State::define_functions`] beside `core_fns` — not a practice field).
fn record_drink_fn() -> Function {
    Function::new("recordDrink", ["P", "Kind", "N"])
        .case(
            [
                eq("Kind", "alcoholic"),
                calc("M", CalcOp::Add, "N", "1"),
            ],
            [
                insert("practice.patron.P.drinks!M"),
                call("checkTipsy", vec!["P".to_owned(), "M".to_owned()]),
            ],
        )
        .case([], [])
}

fn check_tipsy_fn() -> Function {
    Function::new("checkTipsy", ["P", "M"]).case(
        [cmp(CmpOp::Gte, "M", "2")],
        [insert("person.P.tipsy")],
    )
}

fn check_sober_fn() -> Function {
    Function::new("checkSober", ["P", "M"]).case(
        [cmp(CmpOp::Lte, "M", "1")],
        [delete("person.P.tipsy")],
    )
}

fn bar_fns() -> Vec<Function> {
    let mut fns = core_fns();
    fns.extend([record_drink_fn(), check_tipsy_fn(), check_sober_fn()]);
    fns
}

/// TEST-COMPRESSED cadence (real authoring: ~12 rounds, an hour a drink): each
/// firing metabolizes one drink from every patron who has any, and sobriety
/// returns when the count falls back under `checkTipsy`'s own threshold (its
/// mirror, one home).
fn metabolism() -> ScheduleRule {
    ScheduleRule::new("metabolism", 2).clause(
        [
            matches("practice.patron.P.drinks!N"),
            cmp(CmpOp::Gte, "N", "1"),
            calc("M", CalcOp::Sub, "N", "1"),
        ],
        [
            insert("practice.patron.P.drinks!M"),
            call("checkSober", vec!["P".to_owned(), "M".to_owned()]),
        ],
    )
}

/// A ready-made reaction: when spawned as
/// `practice.disapproval.<Offender>.<Onlooker>`, it offers the onlooker a chance
/// to disapprove of (or forgive) the offender. Spawned by
/// [`settle_up_practice`] when a tab goes unpaid.
fn disapproval_practice() -> Practice {
    Practice::new("disapproval")
        .name("[Onlooker] saw [Offender] break a norm")
        .roles(["Offender", "Onlooker"])
        .action(
            Action::new("[Actor]: Disapprove of [Offender]")
                .when([eq("Actor", "Onlooker")])
                .then([
                    insert("Onlooker.disapprovedOf.Offender"),
                    feel_toward_for(4, "Onlooker", ANNOYED, "Offender"),
                    adjust_score("Onlooker", "Offender", WARMTH, -20, "brokeANorm"),
                    end_reaction("disapproval", &["Offender", "Onlooker"]),
                ]),
        )
        .action(
            Action::new("[Actor]: Let [Offender]'s lapse slide")
                .when([eq("Actor", "Onlooker")])
                .then([
                    feel_toward_for(4, "Onlooker", PLEASED, "Offender"),
                    end_reaction("disapproval", &["Offender", "Onlooker"]),
                ]),
        )
}

/// Settling up after being served: the obligation "[Patron] should tip
/// [Bartender]" (a first-class deontic □, raised on serve) is discharged by
/// tipping, or breached by leaving the tab unpaid — a norm violation that spawns
/// the bartender's disapproval AND, contrary-to-duty (□□), a reparative
/// obligation to make amends. Spawned as
/// `practice.settleUp.<Patron>.<Bartender>`.
fn settle_up_practice() -> Practice {
    Practice::new("settleUp")
        .name("[Patron] should settle up with [Bartender]")
        .roles(["Patron", "Bartender"])
        .action(
            Action::new("[Actor]: Tip [Bartender]")
                .when([eq("Actor", "Patron")])
                .then([
                    // fulfils the duty's content…
                    insert("Patron.tipped.Bartender"),
                    // …so the obligation is met and closed
                    discharge("Patron", "Patron.tipped.Bartender"),
                    adjust_score("Bartender", "Patron", WARMTH, 8, "aGoodTipper"),
                    adjust_score("Patron", "Bartender", WARMTH, 3, "friendlyService"),
                    feel_toward_for(4, "Bartender", PLEASED, "Patron"),
                    end_reaction("settleUp", &["Patron", "Bartender"]),
                ]),
        )
        .action(
            Action::new("[Actor]: Leave [Bartender]'s tab unpaid")
                .when([eq("Actor", "Patron")])
                .then([
                    // the duty is breached (= a violation)
                    breach("Patron", "stiffedTheBartender"),
                    // the original duty can no longer be met…
                    discharge("Patron", "Patron.tipped.Bartender"),
                    // …so a reparative □□ duty arises
                    oblige_reparative("Patron", "make.amends.with.Bartender"),
                    spawn_reaction("disapproval", &["Patron", "Bartender"]),
                    end_reaction("settleUp", &["Patron", "Bartender"]),
                ]),
        )
}

/// A conversation between two friends: small talk, compliments, or gossip. Quips
/// say a line and shift the core model / plant beliefs. Spawned by "Strike up a
/// conversation"; participants stay on a topic until someone changes it.
fn converse_practice() -> Practice {
    Practice::new("converse")
        .name("[A] and [B] are chatting")
        .roles(["A", "B"])
        .action(quip(
            "smalltalk",
            "[Actor]: Make small talk with [Partner]",
            "smallTalk",
            vec![],
            vec![
                adjust_score("Actor", "Partner", WARMTH, 2, "pleasantChat"),
                adjust_score("Partner", "Actor", WARMTH, 2, "pleasantChat"),
            ],
        ))
        .action(quip(
            "compliment",
            "[Actor]: Compliment [Partner]",
            "rapport",
            vec![],
            vec![
                adjust_score("Partner", "Actor", WARMTH, 8, "kindWords"),
                feel_toward_for(4, "Partner", PLEASED, "Actor"),
            ],
        ))
        .action(quip(
            "gossip",
            "[Actor]: Confide to [Partner] that [Subject] resents them",
            "gossip",
            vec![
                feeling_toward("Actor", ANNOYED, "Subject"),
                neq("Actor", "Subject"),
                neq("Partner", "Subject"),
            ],
            vec![believe("Partner", "resentedBy.Subject", "yes")],
        ))
        .action(change_subject(
            "[Actor]: Warm the talk toward rapport",
            "rapport",
        ))
        .action(change_subject(
            "[Actor]: Lower your voice to gossip",
            "gossip",
        ))
        .action(change_subject(
            "[Actor]: Keep it to small talk",
            "smallTalk",
        ))
        .action(end_conversation("[Actor]: Wrap up the chat with [Partner]"))
}

/// The story manager (Versu's DM): an autonomous agent with METALEVEL desires
/// that shapes the drama without controlling anyone directly. Here it watches for
/// a too-cosy room and injects a falling-out between two friends — which then
/// plays out through the ordinary reaction / gossip machinery.
fn dm_practice() -> Practice {
    let mut stir = vec![
        eq("Actor", "Director"),
        // one dramatic beat per evening
        not_("dm.stirred"),
    ];
    // bind two who currently like each other…
    stir.extend(score_at_least("X", "Y", WARMTH, 20));
    // …then require them distinct
    stir.extend([neq("X", "Y"), neq("X", "Actor"), neq("Y", "Actor")]);

    Practice::new("dm")
        .name("the director shapes the evening")
        .roles(["Director"])
        .action(
            Action::new("[Actor]: turn [X] against [Y] to stir up the evening")
                .when(stir)
                .then([
                    insert("dm.stirred"),
                    feel_toward_for(4, "X", ANNOYED, "Y"),
                    adjust_score("X", "Y", WARMTH, -30, "aSuddenFallingOut"),
                    insert("practice.greet.world.grievance.X.Y"),
                ]),
        )
}

/// Player-as-DM (Versu §XI): the same metalevel role as the autonomous
/// `director`, but a palette of authorial nudges for a HUMAN to steer an
/// otherwise autonomous cast — stirring conflict, kindling warmth, or souring
/// the mood — without ever embodying a character. The player is bound to this
/// practice in [`bar_director_world`], so their menu is these nudges and nothing
/// else. Each nudge is one dramatic beat per participants (it can't be spammed),
/// and it reshapes the story only indirectly: the cast then reacts through the
/// ordinary greeting / conversation / arc machinery.
fn direct_practice() -> Practice {
    Practice::new("direct")
        .name("you direct the evening")
        .roles(["Director"])
        .action(
            Action::new("[Actor]: stir up a rivalry between [X] and [Y]")
                .when([
                    eq("Actor", "Director"),
                    matches("practice.world.world.at.X!Px"),
                    matches("practice.world.world.at.Y!Py"),
                    neq("X", "Y"),
                    neq("X", "Director"),
                    neq("Y", "Director"),
                    not_("direct.stirred.X.Y"),
                ])
                .then([
                    insert("direct.stirred.X.Y"),
                    feel_toward_for(4, "X", ANNOYED, "Y"),
                    adjust_score("X", "Y", WARMTH, -30, "aFallingOut"),
                    insert("practice.greet.world.grievance.X.Y"),
                ]),
        )
        .action(
            Action::new("[Actor]: kindle warmth between [X] and [Y]")
                .when([
                    eq("Actor", "Director"),
                    matches("practice.world.world.at.X!Px"),
                    matches("practice.world.world.at.Y!Py"),
                    neq("X", "Y"),
                    neq("X", "Director"),
                    neq("Y", "Director"),
                    not_("direct.kindled.X.Y"),
                ])
                .then([
                    insert("direct.kindled.X.Y"),
                    adjust_score("X", "Y", WARMTH, 15, "aWarmFeeling"),
                    adjust_score("Y", "X", WARMTH, 15, "aWarmFeeling"),
                    feel_toward_for(4, "X", PLEASED, "Y"),
                    feel_toward_for(4, "Y", PLEASED, "X"),
                ]),
        )
        .action(
            Action::new("[Actor]: cast a pall over [X]'s evening")
                .when([
                    eq("Actor", "Director"),
                    matches("practice.world.world.at.X!Px"),
                    neq("X", "Director"),
                    not_("direct.unsettled.X"),
                ])
                .then([
                    insert("direct.unsettled.X"),
                    feel_toward_for(4, "X", SAD, "here"),
                ]),
        )
}

// Cast ------------------------------------------------------------------------

/// The player: they choose; no wants drive them.
fn you() -> Character {
    Character::new(PLAYER_NAME)
}

/// The bartender: tends the bar, greets, disapproves of norm-breakers, and takes
/// offense if snubbed.
fn ada() -> Character {
    let mut c = Character::new("ada");
    c.wants = vec![
        Want::new(
            vec![matches("practice.tendBar.Place.ada.customer.Customer!order")],
            -5,
        ),
        Want::new(vec![matches("practice.world.world.at.ada!bar")], 1),
        Want::new(vec![matches("practice.greet.world.greeted.ada.Other")], 2),
        Want::new(vec![matches("practice.greet.world.grievance.ada.Other")], 2),
        // disapproves of tab-skippers
        Want::new(vec![matches("ada.disapprovedOf.Offender")], 3),
        // Grudging courtesy: the gate that once withheld greeting (or chatting
        // with) a cross target is gone; the reluctance is priced instead. -3
        // outweighs the +2 ordinary appeal of greeting (this same want, just
        // above — greeting and greeting-back write the identical fact); it
        // dominates trivially for striking up a conversation too, which carries
        // no self-want of its own to outweigh.
        Want::new(
            vec![
                matches("ada.feels.annoyed.toward.T"),
                or_(vec![
                    vec![matches("practice.greet.world.greeted.ada.T")],
                    vec![matches("ada.chattedWith.T")],
                ]),
            ],
            -3,
        ),
    ];
    c
}

/// A patron who wants a beer, greets people, tips (and strongly avoids stiffing
/// the bartender), and — once warm toward ada — stands her a drink.
fn bex() -> Character {
    let mut c = Character::new("bex");
    c.wants = vec![
        Want::new(
            vec![matches(
                "practice.tendBar.Place.ada.customer.bex!order!beer",
            )],
            4,
        ),
        Want::new(
            vec![matches(
                "practice.tendBar.Place.ada.customer.bex!beverage!beer",
            )],
            9,
        ),
        Want::new(vec![matches("practice.world.world.at.bex!bar")], 1),
        Want::new(vec![matches("practice.greet.world.greeted.bex.Other")], 2),
        Want::new(vec![matches("practice.greet.world.grievance.bex.Other")], 2),
        Want::new(vec![matches("practice.greet.world.bought.bex.Friend")], 6),
        // likes to tip
        Want::new(vec![matches("bex.tipped.ada")], 3),
        // and hates stiffing
        Want::new(vec![violation_of("bex", "stiffedTheBartender")], -40),
        // the arc: bex yearns to belong and dreads loneliness, and once settled
        // behaves accordingly (lingers if it belongs, drifts home if it doesn't)
        Want::new(vec![arc_is("bex", "belonging")], 25),
        Want::new(vec![arc_is("bex", "lonely")], -25),
        Want::new(
            vec![
                arc_is("bex", "belonging"),
                matches("practice.world.world.at.bex!bar"),
            ],
            3,
        ),
        Want::new(
            vec![
                arc_is("bex", "lonely"),
                matches("practice.world.world.at.bex!entrance"),
            ],
            8,
        ),
        // Grudging courtesy: as ada's, above — -3 outweighs the +2
        // greeting/greeting-back appeal (this same want, just above), and
        // dominates trivially for conversation, which has no self-want to
        // outweigh.
        Want::new(
            vec![
                matches("bex.feels.annoyed.toward.T"),
                or_(vec![
                    vec![matches("practice.greet.world.greeted.bex.T")],
                    vec![matches("bex.chattedWith.T")],
                ]),
            ],
            -3,
        ),
        // Grudging courtesy, the round: buying T a drink while cross with T
        // grates strongly enough to outweigh the round's own +6 appeal (the
        // bought.bex.Friend want, just above) — the invariant: the gate is gone,
        // the reluctance is priced.
        Want::new(
            vec![
                matches("bex.feels.annoyed.toward.T"),
                matches("practice.greet.world.bought.bex.T"),
            ],
            -8,
        ),
    ];
    c
}

/// The director: no physical presence, only metalevel desires; bound to its own
/// practice, so it never greets or drinks — it only shapes the story.
fn director() -> Character {
    // wants the evening to have a spark
    Character::new("director")
        .want(Want::new(vec![matches("dm.stirred")], 20))
        .bound_to("dm")
}

/// A second patron, so the player-DM has a lively cast to play off (stir bex
/// against cai, kindle either toward ada, …). Wants a cider and, like bex, to
/// belong.
fn cai() -> Character {
    let mut c = Character::new("cai");
    c.wants = vec![
        Want::new(
            vec![matches(
                "practice.tendBar.Place.ada.customer.cai!order!cider",
            )],
            4,
        ),
        Want::new(
            vec![matches(
                "practice.tendBar.Place.ada.customer.cai!beverage!cider",
            )],
            9,
        ),
        Want::new(vec![matches("practice.world.world.at.cai!bar")], 1),
        Want::new(vec![matches("practice.greet.world.greeted.cai.Other")], 2),
        Want::new(vec![matches("practice.greet.world.bought.cai.Friend")], 6),
        Want::new(vec![arc_is("cai", "belonging")], 25),
        Want::new(vec![arc_is("cai", "lonely")], -25),
        // Grudging courtesy: as ada's/bex's, above.
        Want::new(
            vec![
                matches("cai.feels.annoyed.toward.T"),
                or_(vec![
                    vec![matches("practice.greet.world.greeted.cai.T")],
                    vec![matches("cai.chattedWith.T")],
                ]),
            ],
            -3,
        ),
        // Grudging courtesy, the round: as bex's, above.
        Want::new(
            vec![
                matches("cai.feels.annoyed.toward.T"),
                matches("practice.greet.world.bought.cai.T"),
            ],
            -8,
        ),
    ];
    c
}

/// The player-controlled DM: bound to [`direct_practice`], no wants of its own —
/// the human supplies the intent, choosing nudges from a menu each turn.
fn director_player() -> Character {
    Character::new(DIRECTOR_NAME).bound_to("direct")
}

// Initial world ---------------------------------------------------------------

/// Sort declarations for the type checker: the clearly monomorphic slots of the
/// bar. Positions like a mood's target are deliberately left unsorted, since they
/// are genuinely polymorphic (you can feel a way toward a person or a place).
fn bar_sorts() -> Vec<(String, Vec<String>)> {
    [
        ("beverage", ["beer", "cider", "soda", "water"].as_slice()),
        ("beverageKind", ["alcoholic", "nonalcoholic"].as_slice()),
        ("place", ["bar", "entrance"].as_slice()),
    ]
    .into_iter()
    .map(|(k, vs)| {
        (
            k.to_owned(),
            vs.iter().map(|v| (*v).to_owned()).collect::<Vec<String>>(),
        )
    })
    .collect()
}

/// An epistemic prediction scope shared by both worlds: you credit another's
/// predicted move only if you're with them now, or you sighted them within the
/// last 2 ticks — one tick per round, and two rounds is roughly a there-and-back
/// trip to the next room: "you assume people stay put for about as long as it
/// takes to walk one room away and back."
fn prediction_scope() -> Vec<Condition> {
    vec![or_(vec![together(), sighted_within(2)])]
}

/// The fully initialized bar: practices (core-model + reaction libraries)
/// defined, instances spawned, cast placed.
///
/// # Panics
/// If a construction guard rejects the authored content — a bug in this file,
/// not a condition a world can handle.
pub fn bar_world() -> State {
    let mut st = State::new();
    st.define_practices([
        disapproval_practice(),
        world_practice(),
        greet_practice(),
        respond_greet_practice(),
        patron_practice(),
        tend_bar_practice(),
        settle_up_practice(),
        converse_practice(),
        dm_practice(),
        arc_practice(),
    ])
    .expect("bar practices");
    st.define_functions(bar_fns()).expect("the bar's functions");
    st.set_characters(vec![you(), ada(), bex(), director()])
        .expect("bar cast");
    st.set_sorts(bar_sorts()).expect("bar sorts");
    // The engine owns time (v44): the schedule fires sight (period 1) and
    // metabolism (period 2) at each round boundary — no ticker characters.
    st.set_schedule(vec![sight_rule(bar_sighting()), metabolism()])
        .expect("bar schedule");
    for o in [
        insert("practice.world.world.connected.entrance.bar"),
        insert("practice.world.world.connected.bar.entrance"),
        insert("practice.world.world.at.you!entrance"),
        insert("practice.world.world.at.bex!entrance"),
        insert("practice.world.world.at.ada!bar"),
        insert("practice.patron.you"),
        insert("practice.patron.bex"),
        insert("practice.greet.world"),
        insert("practice.tendBar.bar.ada"),
        insert("practice.dm.director"),
        insert("practice.arc.you"),
        insert("practice.arc.bex"),
    ] {
        st.perform_outcome(&o).expect("bar setup");
    }
    st.set_prediction_scope(prediction_scope())
        .expect("bar prediction scope");
    st
}

/// The same bar, but the human is the DRAMA MANAGER (Versu §XI): the player
/// controls [`director_player`], steering an autonomous cast (ada, bex, cai) with
/// authorial nudges instead of embodying a character. There is no `you` — the
/// player is the unseen hand shaping the evening.
///
/// Note the two differences from [`bar_world`] beyond the cast and setup: the
/// `dm` practice is replaced by `direct`, and the schedule carries `sightRule`
/// ALONE (no metabolism).
///
/// # Panics
/// If a construction guard rejects the authored content.
pub fn bar_director_world() -> State {
    let mut st = State::new();
    st.define_practices([
        disapproval_practice(),
        world_practice(),
        greet_practice(),
        respond_greet_practice(),
        patron_practice(),
        tend_bar_practice(),
        settle_up_practice(),
        converse_practice(),
        direct_practice(),
        arc_practice(),
    ])
    .expect("dm practices");
    st.define_functions(bar_fns()).expect("the bar's functions");
    st.set_characters(vec![ada(), bex(), cai(), director_player()])
        .expect("dm cast");
    st.set_sorts(bar_sorts()).expect("dm sorts");
    st.set_schedule(vec![sight_rule(bar_sighting())])
        .expect("dm schedule");
    for o in [
        insert("practice.world.world.connected.entrance.bar"),
        insert("practice.world.world.connected.bar.entrance"),
        insert("practice.world.world.at.ada!bar"),
        // patrons already in the room…
        insert("practice.world.world.at.bex!bar"),
        // …with the bartender, for the DM to play off
        insert("practice.world.world.at.cai!bar"),
        insert("practice.patron.bex"),
        insert("practice.patron.cai"),
        insert("practice.greet.world"),
        insert("practice.tendBar.bar.ada"),
        insert("practice.direct.director"),
        insert("practice.arc.bex"),
        insert("practice.arc.cai"),
    ] {
        st.perform_outcome(&o).expect("dm setup");
    }
    st.set_prediction_scope(prediction_scope())
        .expect("dm prediction scope");
    st
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_core::engine::GroundedAction;
    use prax_vocab::emotion::{feel_toward, unfeel_toward};

    // H: BarSpec.hs "Prax.Worlds.Bar (feature integration)"
    //
    // The frozen `Prax.BarSpec`, re-expressed against the Rust engine: the
    // drink counter (init/Call/Calc/Cmp), the metabolism schedule rule and its
    // re-arming due, the bell's Subquery/Count/Cmp, the reaction spawn/consume
    // cycle, the first-class □ obligation and its contrary-to-duty repair, the
    // conversation, the arc, and the autonomous director.

    /// One round boundary: the engine advances the clock and fires every due
    /// schedule rule (sight every boundary; metabolism every 2). The bar's
    /// schedule seeds metabolism's first due one period out (turn 2), so
    /// `boundaries(st, 2)` from a fresh (turn-0) state reaches it.
    fn boundaries(st: &mut State, k: usize) {
        for _ in 0..k {
            st.round_boundary();
        }
    }

    fn labels(st: &mut State, actor: &str) -> Vec<String> {
        st.possible_actions(actor)
            .into_iter()
            .map(|ga| ga.label)
            .collect()
    }

    fn find(st: &mut State, actor: &str, needle: &str) -> GroundedAction {
        let had = labels(st, actor);
        st.possible_actions(actor)
            .into_iter()
            .find(|ga| ga.label.contains(needle))
            .unwrap_or_else(|| {
                panic!("no action matching {needle:?} for {actor}; available: {had:?}")
            })
    }

    /// The frozen `runSteps`: perform, in order, the first action whose label
    /// contains each needle.
    fn run_steps(st: &mut State, steps: &[(&str, &str)]) {
        for (actor, needle) in steps {
            let ga = find(st, actor, needle);
            st.perform_action(&ga);
        }
    }

    fn character_named(st: &State, name: &str) -> Character {
        st.characters()
            .iter()
            .find(|c| c.name == name)
            .cloned()
            .unwrap_or_else(|| panic!("{name} not found in the given state"))
    }

    fn has(st: &State, fact: &str) -> bool {
        st.labeled_facts().contains(&fact.to_owned())
    }

    // H: BarSpec.hs "drinking two beers makes you tipsy (init/Call/Calc/Cmp)"
    #[test]
    fn drinking_two_beers_makes_you_tipsy() {
        let mut st = bar_world();
        run_steps(
            &mut st,
            &[
                ("you", "Go to bar"),
                ("you", "Order beer"),
                ("ada", "Fulfill you"),
                ("you", "Drink the beer"),
            ],
        );
        assert!(has(&st, "practice.patron.you.drinks!1"), "counter at 1");
        assert!(!has(&st, "person.you.tipsy"), "not tipsy yet");

        run_steps(
            &mut st,
            &[
                ("you", "Order beer"),
                ("ada", "Fulfill you"),
                ("you", "Drink the beer"),
            ],
        );
        assert!(has(&st, "practice.patron.you.drinks!2"), "counter at 2");
        assert!(has(&st, "person.you.tipsy"), "now tipsy");
    }

    /// Two beers drunk, the state every metabolism pin starts from.
    fn two_drinks() -> State {
        let mut st = bar_world();
        run_steps(
            &mut st,
            &[
                ("you", "Go to bar"),
                ("you", "Order beer"),
                ("ada", "Fulfill you"),
                ("you", "Drink the beer"),
                ("you", "Order beer"),
                ("ada", "Fulfill you"),
                ("you", "Drink the beer"),
            ],
        );
        st
    }

    // H: BarSpec.hs "a patron at 2 drinks is tipsy; one dry metabolism firing (2 -> 1) clears it"
    #[test]
    fn one_dry_metabolism_firing_clears_the_tipsiness() {
        let mut st = two_drinks();
        assert!(has(&st, "practice.patron.you.drinks!2"), "counter at 2");
        assert!(has(&st, "person.you.tipsy"), "tipsy before the firing");
        // metabolism's period is 2: the due (seeded at turn 2) is reached after
        // two round boundaries.
        boundaries(&mut st, 2);
        assert!(
            has(&st, "practice.patron.you.drinks!1"),
            "counter decremented to 1"
        );
        assert!(
            !has(&st, "practice.patron.you.drinks!2"),
            "the 2-drinks fact is gone"
        );
        assert!(
            !has(&st, "person.you.tipsy"),
            "tipsy cleared once under the threshold"
        );
    }

    // H: BarSpec.hs "drinking again before the firing (3 -> 2) keeps you tipsy through it"
    #[test]
    fn drinking_again_before_the_firing_keeps_you_tipsy() {
        let mut st = two_drinks();
        run_steps(
            &mut st,
            &[
                ("you", "Order beer"),
                ("ada", "Fulfill you"),
                ("you", "Drink the beer"),
            ],
        );
        assert!(has(&st, "practice.patron.you.drinks!3"), "counter at 3");
        assert!(has(&st, "person.you.tipsy"), "tipsy before the firing");
        boundaries(&mut st, 2);
        assert!(
            has(&st, "practice.patron.you.drinks!2"),
            "decremented to 2, still at the threshold"
        );
        assert!(
            has(&st, "person.you.tipsy"),
            "still tipsy: 2 is still >= checkTipsy's own threshold"
        );
    }

    // H: BarSpec.hs "a firing at 0 drinks leaves 0 (the Gte 1 guard never goes negative)"
    #[test]
    fn a_firing_at_zero_drinks_leaves_zero() {
        let mut st = bar_world();
        boundaries(&mut st, 2);
        assert!(
            has(&st, "practice.patron.you.drinks!0"),
            "you never drank: still at 0"
        );
        assert!(
            has(&st, "practice.patron.bex.drinks!0"),
            "bex never drank: still at 0"
        );
        assert!(
            !st.labeled_facts()
                .iter()
                .any(|f| f.contains("practice.patron.you.drinks!-")),
            "no negative counter appears"
        );
    }

    // H: BarSpec.hs "the metabolism due re-arms: the second firing is only a full period later"
    #[test]
    fn the_metabolism_due_re_arms() {
        let mut st = two_drinks();
        // first firing at turn 2: 2 -> 1, due re-arms to 2 + 2 = 4.
        boundaries(&mut st, 2);
        assert!(
            has(&st, "practice.patron.you.drinks!1"),
            "the first firing landed"
        );
        // one more boundary (turn 3): below the re-armed due (4) — no metabolism.
        st.round_boundary();
        assert!(
            has(&st, "practice.patron.you.drinks!1"),
            "not due yet: the counter is unchanged at 1"
        );
        // the due-reaching boundary (turn 4): the second firing lands.
        st.round_boundary();
        assert!(
            has(&st, "practice.patron.you.drinks!0"),
            "the second firing landed a full period (2 rounds) after the first"
        );
    }

    // H: BarSpec.hs "the bell requires two customers (Subquery/Count/Cmp)"
    #[test]
    fn the_bell_requires_two_customers() {
        let mut st = bar_world();
        run_steps(&mut st, &[("bex", "Go to bar"), ("bex", "Order beer")]);
        assert!(
            !labels(&mut st, "ada")
                .iter()
                .any(|l| l.contains("Ring the bell")),
            "no bell with one customer"
        );
        run_steps(&mut st, &[("you", "Go to bar"), ("you", "Order cider")]);
        assert!(
            labels(&mut st, "ada")
                .iter()
                .any(|l| l.contains("Ring the bell")),
            "the bell becomes available with two customers"
        );
    }

    // H: BarSpec.hs "the buy-a-drink affordance is gated on relationship warmth"
    #[test]
    fn the_buy_a_drink_affordance_is_gated_on_warmth() {
        let bex_can_buy = |st: &mut State| {
            st.possible_actions("bex")
                .iter()
                .any(|ga| ga.label.contains("Buy ada a drink"))
        };
        let at_bar = || {
            let mut st = bar_world();
            st.perform_outcome(&insert("practice.world.world.at.bex!bar"))
                .expect("co-locating bex");
            st
        };
        let mut cool = at_bar();
        cool.perform_outcome(&adjust_score("bex", "ada", WARMTH, 10, "acquaintance"))
            .expect("warming a little");
        assert!(
            !bex_can_buy(&mut cool),
            "no buy option when only mildly warm"
        );
        let mut warm = at_bar();
        warm.perform_outcome(&adjust_score("bex", "ada", WARMTH, 20, "fondness"))
            .expect("warming past the threshold");
        assert!(
            bex_can_buy(&mut warm),
            "the gift appears once warm enough — a relationship creating a goal"
        );
    }

    // H: BarSpec.hs "cross bartenders may pour, and won't: the gate is gone, the reluctance is priced (both halves)"
    #[test]
    fn the_gate_is_gone_the_reluctance_is_priced() {
        // Warm bex past the buy threshold and let her settle in (so buying, not
        // the arc's "settle in" beat, is her best remaining move — this isolates
        // the buy decision from the unrelated +25 arc want).
        let mut st = bar_world();
        st.perform_outcome(&insert("practice.world.world.at.bex!bar"))
            .expect("co-locating bex");
        st.perform_outcome(&adjust_score("bex", "ada", WARMTH, 20, "fondness"))
            .expect("warming bex");
        let bex_char = character_named(&st, "bex");
        let settle = st
            .candidate_actions(&bex_char)
            .into_iter()
            .find(|ga| ga.label.contains("settle in"))
            .expect("a settle-in action is offered to warm bex");
        st.perform_action(&settle);

        let can_buy = |st: &mut State| {
            let b = character_named(st, "bex");
            st.candidate_actions(&b)
                .iter()
                .any(|ga| ga.label.contains("Buy ada a drink"))
        };
        let picks_buy = |st: &mut State| {
            let b = character_named(st, "bex");
            st.pick_action(2, &b)
                .is_some_and(|ga| ga.label.contains("Buy ada a drink"))
        };
        assert!(
            can_buy(&mut st) && picks_buy(&mut st),
            "warm, untroubled bex both can and does buy"
        );

        // Annoyed at ada: the buy grounding is STILL offered — THE INVARIANT: a
        // feeling gates no decision (availability half).
        st.perform_outcome(&feel_toward("bex", ANNOYED, "ada"))
            .expect("souring bex");
        assert!(
            can_buy(&mut st),
            "annoyed bex can still buy (no availability gate)"
        );
        // …but the planner prices the grudge out (pricing half — the
        // grudging-round want, -8, outweighs the round's +6 ordinary appeal).
        assert!(!picks_buy(&mut st), "annoyed bex does not pick buying");

        // Unfeel it: she buys again — the third state the pin asserts.
        st.perform_outcome(&unfeel_toward("bex", ANNOYED, "ada"))
            .expect("mollifying bex");
        assert!(can_buy(&mut st), "un-annoyed again, she can buy");
        assert!(picks_buy(&mut st), "un-annoyed again, she picks it");
    }

    // H: BarSpec.hs "an onlooker's disapproval-annoyance fades on its own lifetime"
    #[test]
    fn an_onlookers_disapproval_annoyance_fades_on_its_own_lifetime() {
        let mut st = bar_world();
        run_steps(
            &mut st,
            &[
                ("bex", "Go to bar"),
                ("bex", "Order beer"),
                ("ada", "Fulfill bex"),
                ("bex", "Leave ada"),
                ("ada", "Disapprove of bex"),
            ],
        );
        assert!(
            has(&st, "ada.feels.annoyed.toward.bex"),
            "ada is annoyed at bex right after disapproving"
        );
        // disapproval's annoyance carries a lifetime of 4 (test-compressed): the
        // onset fired at turn 0 (run_steps crosses no boundary), so the engine's
        // expiry queue retracts it at boundary 4.
        boundaries(&mut st, 4);
        assert!(
            !has(&st, "ada.feels.annoyed.toward.bex"),
            "the annoyance has faded on its lifetime, got {:?}",
            st.labeled_facts()
        );
    }

    // H: BarSpec.hs "greeting spawns a response reaction the greeted party can take"
    #[test]
    fn greeting_spawns_a_response_reaction() {
        let mut st = bar_world();
        run_steps(&mut st, &[("you", "Go to bar"), ("you", "Greet ada")]);
        assert!(
            has(&st, "practice.respondGreet.you.ada"),
            "the reaction is spawned"
        );
        assert!(
            labels(&mut st, "ada")
                .iter()
                .any(|l| l.contains("Greet you back")),
            "ada can greet back — a response that only exists now"
        );
        run_steps(&mut st, &[("ada", "Greet you back")]);
        assert!(
            has(&st, "practice.greet.world.greeted.ada.you"),
            "ada greeted you back"
        );
        assert!(
            !has(&st, "practice.respondGreet.you.ada"),
            "the reaction is consumed"
        );
    }

    /// bex ordered and was served: the state both settle-up pins start from.
    fn served() -> State {
        let mut st = bar_world();
        run_steps(
            &mut st,
            &[
                ("bex", "Go to bar"),
                ("bex", "Order beer"),
                ("ada", "Fulfill bex"),
            ],
        );
        st
    }

    // H: BarSpec.hs "being served spawns a tip obligation; tipping respects the norm"
    #[test]
    fn being_served_spawns_a_tip_obligation() {
        let mut st = served();
        assert!(
            has(&st, "practice.settleUp.bex.ada"),
            "the settle-up reaction is spawned"
        );
        assert!(
            has(&st, "obliged.bex.bex.tipped.ada"),
            "a first-class tip obligation (a real □) arose on serve"
        );
        run_steps(&mut st, &[("bex", "Tip ada")]);
        assert!(has(&st, "bex.tipped.ada"), "bex tipped ada");
        assert!(
            !has(&st, "violated.bex.stiffedTheBartender"),
            "no violation"
        );
        assert!(
            !has(&st, "practice.settleUp.bex.ada"),
            "the reaction is cleared"
        );
        assert!(
            !has(&st, "obliged.bex.bex.tipped.ada"),
            "the □ obligation is discharged once tipped"
        );
    }

    // H: BarSpec.hs "leaving the tab is a violation that draws the bartender's disapproval"
    #[test]
    fn leaving_the_tab_draws_disapproval() {
        let mut st = served();
        run_steps(&mut st, &[("bex", "Leave ada")]);
        assert!(
            has(&st, "violated.bex.stiffedTheBartender"),
            "the violation is marked"
        );
        assert!(
            has(&st, "obliged.bex.obliged.bex.make.amends.with.ada"),
            "a reparative □□ obligation arises after the breach (contrary-to-duty)"
        );
        assert!(
            has(&st, "practice.disapproval.bex.ada"),
            "the disapproval reaction is spawned for ada"
        );
        run_steps(&mut st, &[("ada", "Disapprove of bex")]);
        assert!(
            has(&st, "ada.relationship.bex.warmth.score!-20"),
            "ada cooled toward bex, got {:?}",
            st.labeled_facts()
        );
    }

    // H: BarSpec.hs "a believed grudge suppresses friendliness (a false belief drives behaviour)"
    #[test]
    fn a_believed_grudge_suppresses_friendliness() {
        let mut st = bar_world();
        st.perform_outcome(&insert("practice.world.world.at.bex!bar"))
            .expect("co-locating bex");
        st.perform_outcome(&adjust_score("bex", "ada", WARMTH, 20, "fond"))
            .expect("warming bex");
        let can = |st: &mut State, needle: &str| {
            st.possible_actions("bex")
                .iter()
                .any(|ga| ga.label.contains(needle))
        };
        assert!(can(&mut st, "Buy ada a drink"), "warm bex would buy");
        assert!(can(&mut st, "Greet ada"), "warm bex would greet");
        // bex comes to believe ada resents them (even though she's actually warm)
        st.perform_outcome(&believe("bex", "resentedBy.ada", "yes"))
            .expect("planting the false belief");
        assert!(
            !can(&mut st, "Buy ada a drink"),
            "the belief blocks the gift"
        );
        assert!(!can(&mut st, "Greet ada"), "the belief blocks greeting");
    }

    // H: BarSpec.hs "a grudge lets you plant a (possibly-false) rumour"
    #[test]
    fn a_grudge_lets_you_plant_a_rumour() {
        let mut st = bar_world();
        for o in [
            // ada steps out
            insert("practice.world.world.at.ada!entrance"),
            insert("practice.world.world.at.you!bar"),
            insert("practice.world.world.at.bex!bar"),
            // you're cross with ada
            feel_toward("you", ANNOYED, "ada"),
        ] {
            st.perform_outcome(&o).expect("staging the rumour");
        }
        assert!(
            labels(&mut st, "you")
                .iter()
                .any(|l| l.contains("Warn bex that ada resents")),
            "the rumour is available behind ada's back"
        );
        run_steps(&mut st, &[("you", "Warn bex that ada resents")]);
        assert!(
            has(&st, "bex.believes.resentedBy.ada!yes"),
            "bex now believes ada resents them, got {:?}",
            st.labeled_facts()
        );
    }

    // H: BarSpec.hs "evidence of warmth dispels a false belief"
    #[test]
    fn evidence_of_warmth_dispels_a_false_belief() {
        let mut st = bar_world();
        for o in [
            insert("practice.world.world.at.bex!bar"),
            believe("bex", "resentedBy.ada", "yes"),
            // ada actually greeted bex
            insert("practice.greet.world.greeted.ada.bex"),
        ] {
            st.perform_outcome(&o).expect("staging the evidence");
        }
        assert!(
            labels(&mut st, "bex")
                .iter()
                .any(|l| l.contains("Realize ada doesn't resent you")),
            "bex can reconsider"
        );
        run_steps(&mut st, &[("bex", "Realize ada doesn't resent you")]);
        assert!(
            !st.labeled_facts()
                .iter()
                .any(|f| f.contains("bex.believes.resentedBy.ada")),
            "the false belief is dropped"
        );
    }

    // H: BarSpec.hs "friends can strike up a chat; quips stay on topic and shift feeling"
    #[test]
    fn friends_can_strike_up_a_chat_and_quips_stay_on_topic() {
        let mut st = bar_world();
        for o in [
            insert("practice.world.world.at.bex!bar"),
            adjust_score("bex", "ada", WARMTH, 20, "fond"),
        ] {
            st.perform_outcome(&o).expect("warming bex");
        }
        assert!(
            labels(&mut st, "bex")
                .iter()
                .any(|l| l.contains("Strike up a conversation with ada")),
            "warm bex can strike up a conversation"
        );
        run_steps(&mut st, &[("bex", "Strike up a conversation with ada")]);
        // opens on small talk: the compliment quip (rapport) is off-topic
        assert!(
            labels(&mut st, "bex")
                .iter()
                .any(|l| l.contains("Make small talk with ada")),
            "small talk is on topic"
        );
        assert!(
            !labels(&mut st, "bex")
                .iter()
                .any(|l| l.contains("Compliment ada")),
            "the compliment is off topic (withheld)"
        );
        run_steps(
            &mut st,
            &[
                ("bex", "Make small talk with ada"),
                ("ada", "Warm the talk toward rapport"),
                ("bex", "Compliment ada"),
            ],
        );
        assert!(
            has(&st, "ada.relationship.bex.warmth.score!10"),
            "the compliment warmed ada toward bex, got {:?}",
            st.labeled_facts()
        );
    }

    // H: BarSpec.hs "a gossip quip transmits a (possibly-false) belief in conversation"
    #[test]
    fn a_gossip_quip_transmits_a_belief() {
        // bex, cross with you, is chatting with ada on the gossip topic.
        let mut st = bar_world();
        st.perform_outcome(&feel_toward("bex", ANNOYED, "you"))
            .expect("souring bex");
        for o in begin_conversation("bex", "ada", "gossip") {
            st.perform_outcome(&o).expect("opening the gossip");
        }
        assert!(
            labels(&mut st, "bex")
                .iter()
                .any(|l| l.contains("Confide to ada that you resents them")),
            "the gossip quip is available to the speaker"
        );
        run_steps(&mut st, &[("bex", "Confide to ada that you resents them")]);
        assert!(
            has(&st, "ada.believes.resentedBy.you!yes"),
            "ada now believes you resent her, got {:?}",
            st.labeled_facts()
        );
    }

    // H: BarSpec.hs "the director (story manager) has nothing to do in a placid room"
    #[test]
    fn the_director_has_nothing_to_do_in_a_placid_room() {
        let mut st = bar_world();
        assert!(
            st.possible_actions("director").is_empty(),
            "the director is idle when nothing is warm"
        );
    }

    // H: BarSpec.hs "the director injects a rivalry between two warm friends"
    #[test]
    fn the_director_injects_a_rivalry_between_two_warm_friends() {
        let mut st = bar_world();
        for o in [
            adjust_score("ada", "bex", WARMTH, 25, "friends"),
            adjust_score("bex", "ada", WARMTH, 25, "friends"),
        ] {
            st.perform_outcome(&o).expect("making them fond");
        }
        let dir_acts = st.possible_actions("director");
        assert!(!dir_acts.is_empty(), "the director can now act");
        assert!(
            dir_acts.iter().all(|ga| ga.practice_id == "dm"),
            "the director only acts through its own (dm) practice, got {:?}",
            dir_acts
                .iter()
                .map(|g| (&g.practice_id, &g.label))
                .collect::<Vec<_>>()
        );
        run_steps(&mut st, &[("director", "turn ada against bex")]);
        assert!(has(&st, "dm.stirred"), "the beat is marked done");
        assert!(
            has(&st, "practice.greet.world.grievance.ada.bex"),
            "ada now bears a grievance against bex"
        );
        assert!(
            has(&st, "ada.relationship.bex.warmth.score!-5"),
            "and their warmth has soured, got {:?}",
            st.labeled_facts()
        );
    }

    // H: BarSpec.hs "a character's arc advances to belonging once it feels at home"
    #[test]
    fn a_characters_arc_advances_to_belonging() {
        let mut st = bar_world();
        st.perform_outcome(&adjust_score("bex", "ada", WARMTH, 20, "fond"))
            .expect("warming bex");
        assert!(
            labels(&mut st, "bex")
                .iter()
                .any(|l| l.contains("settle in, feeling you belong")),
            "the belonging beat is available"
        );
        run_steps(&mut st, &[("bex", "settle in, feeling you belong")]);
        assert!(has(&st, "bex.arc!belonging"), "bex now belongs");
    }

    // H: BarSpec.hs "the against-desires transformation is offered but the planner refuses it"
    #[test]
    fn the_against_desires_transformation_is_offered_but_refused() {
        let mut st = bar_world();
        // Every hopeful patron CAN resign themselves to loneliness…
        assert!(
            labels(&mut st, "bex")
                .iter()
                .any(|l| l.contains("give up on the evening")),
            "the transformation is on the table"
        );
        // …but an NPC never chooses it: sliding into loneliness only costs
        // utility. True transformation is, in practice, a player-only act.
        let bex_char = character_named(&st, "bex");
        assert!(
            st.pick_action(2, &bex_char)
                .is_none_or(|ga| !ga.label.contains("give up")),
            "bex never resigns to solitude on its own"
        );
    }

    // ---- the player-as-DM world --------------------------------------------

    // H: DirectorSpec.hs "Prax.Worlds.Bar (player-as-DM)"
    //
    // The frozen `Prax.DirectorSpec`, re-expressed: the metalevel-only palette,
    // a nudge that reshapes the world without the DM embodying anyone, and a
    // nudge that opens an affordance for an autonomous character.

    fn director_char(st: &State) -> Character {
        character_named(st, DIRECTOR_NAME)
    }

    /// Perform the director's affordance whose label contains `needle`.
    fn direct(st: &mut State, needle: &str) {
        let dir = director_char(st);
        let acts = st.candidate_actions(&dir);
        let ga = acts
            .iter()
            .find(|ga| ga.label.contains(needle))
            .unwrap_or_else(|| {
                panic!(
                    "no director nudge matching {needle:?}; had: {:?}",
                    acts.iter().map(|g| &g.label).collect::<Vec<_>>()
                )
            })
            .clone();
        st.perform_action(&ga);
    }

    /// bex's currently available action labels, in the given state.
    fn bex_can(st: &mut State) -> Vec<String> {
        let bex = character_named(st, "bex");
        st.candidate_actions(&bex)
            .into_iter()
            .map(|ga| ga.label)
            .collect()
    }

    // H: DirectorSpec.hs "the drama manager is offered only metalevel affordances"
    #[test]
    fn the_drama_manager_is_offered_only_metalevel_affordances() {
        let mut st = bar_director_world();
        let dir = director_char(&st);
        let acts = st.candidate_actions(&dir);
        assert!(!acts.is_empty(), "the DM has nudges to make");
        assert!(
            acts.iter().all(|ga| ga.practice_id == "direct"),
            "every DM affordance is a metalevel `direct` move (never embodied), got {:?}",
            acts.iter()
                .map(|g| (&g.practice_id, &g.label))
                .collect::<Vec<_>>()
        );
        let all = acts
            .iter()
            .map(|ga| ga.label.clone())
            .collect::<Vec<_>>()
            .join(" ");
        for needle in ["stir up a rivalry", "kindle warmth", "cast a pall"] {
            assert!(all.contains(needle), "{needle} offered, got {all}");
        }
    }

    // H: DirectorSpec.hs "a DM nudge reshapes the world without the DM embodying anyone"
    #[test]
    fn a_dm_nudge_reshapes_the_world_without_embodying_anyone() {
        let mut st = bar_director_world();
        direct(&mut st, "stir up a rivalry between bex and cai");
        // directP writes through the emotion layer, into the coexisting feels.*
        // family — not a single-slot mood.
        assert!(
            has(&st, "bex.feels.annoyed.toward.cai"),
            "bex is now annoyed at cai, got {:?}",
            st.labeled_facts()
        );
        assert!(
            has(&st, "practice.greet.world.grievance.bex.cai"),
            "a grievance is recorded"
        );
        // …and it is one dramatic beat: the same nudge is not offered again
        let dir = director_char(&st);
        assert!(
            !st.candidate_actions(&dir)
                .iter()
                .any(|ga| ga.label.contains("rivalry between bex and cai")),
            "the stir is one-shot for that pair"
        );
    }

    /// `bound_to` ISOLATED, in both worlds. Neither frozen spec pin actually
    /// tests it: both assert that every director affordance is metalevel, and
    /// both directors are unplaced in the setup, so every embodied action is
    /// already withheld by its own location condition. Removing `charBoundTo`
    /// therefore reddens no frozen-derived pin at all — only `worldshape`.
    ///
    /// So this pin PLACES the director in the room and asserts the menu is
    /// unchanged. Without the binding a located director is offered "Wait a
    /// moment", "Order beer", greetings — the whole embodied palette; with it,
    /// the metalevel practice and nothing else. That makes the binding the
    /// CAUSE, which is more than either frozen spec can express.
    #[test]
    fn the_binding_is_what_keeps_a_director_disembodied() {
        for (mut st, who, pid) in [
            (bar_world(), "director", "dm"),
            (bar_director_world(), DIRECTOR_NAME, "direct"),
        ] {
            let dir = character_named(&st, who);
            assert_eq!(
                dir.bound_to.as_deref(),
                Some(pid),
                "{who} is bound to its metalevel practice"
            );
            // Put them in the room, where every embodied affordance grounds.
            st.perform_outcome(&insert(format!(
                "practice.world.world.at.{who}!bar"
            )))
            .expect("placing the director");
            let acts = st.candidate_actions(&dir);
            assert!(
                acts.iter().all(|ga| ga.practice_id == pid),
                "a PLACED {who} is still offered only {pid} moves, got {:?}",
                acts.iter()
                    .map(|g| (&g.practice_id, &g.label))
                    .collect::<Vec<_>>()
            );

            // The other half: the same world, the same placement, the binding
            // dropped — the embodied palette appears. Without this the assertion
            // above would pass in a world where nothing was ever offered.
            let mut unbound = st;
            let mut cast = unbound.characters().to_vec();
            for c in &mut cast {
                if c.name == who {
                    c.bound_to = None;
                }
            }
            unbound.set_characters(cast).expect("the unbound cast");
            let bare = Character::new(who);
            let loose = unbound.candidate_actions(&bare);
            assert!(
                loose.iter().any(|ga| ga.practice_id != pid),
                "an UNBOUND placed {who} picks up embodied affordances — otherwise \
                 the binding is not what the pin above is measuring; got {:?}",
                loose
                    .iter()
                    .map(|g| (&g.practice_id, &g.label))
                    .collect::<Vec<_>>()
            );
        }
    }

    // H: DirectorSpec.hs "a DM nudge opens a new affordance for an autonomous character"
    #[test]
    fn a_dm_nudge_opens_a_new_affordance_for_an_autonomous_character() {
        let mut st = bar_director_world();
        // baseline: cold, bex cannot yet stand ada a drink (needs warmth >= 15)
        assert!(
            !bex_can(&mut st).iter().any(|l| l.contains("Buy ada a drink")),
            "bex can't buy ada a drink while cold"
        );
        direct(&mut st, "kindle warmth between bex and ada");
        assert!(
            bex_can(&mut st).iter().any(|l| l.contains("Buy ada a drink")),
            "after the DM kindles warmth, bex may buy ada a drink"
        );
    }
}
