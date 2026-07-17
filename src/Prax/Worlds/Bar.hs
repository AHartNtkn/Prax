-- | The demo storyworld: movement, greeting, and tending bar, wired to the Core
-- Model (v2) and now to Reactions & Norms (v3).
--
-- Cast: @you@ (the player), @ada@ (an NPC bartender) and @bex@ (an NPC patron).
--
-- Social loops (see @docs/WALKTHROUGH.md@):
--
--   * greeting/serving raise a numeric @warmth@ evaluation; once warm enough,
--     the "buy … a drink" affordance appears (a relationship creating a goal);
--   * a greeting spawns a @respondGreet@ reaction — the greeted party can greet
--     back (mutual warmth), rebuff (both cool), or, if they ignore it, be taken
--     to task for the snub;
--   * being served spawns a @settleUp@ obligation — tip (norm respected, warms
--     the bartender) or leave the tab (a norm violation that spawns the
--     bartender's disapproval). Agents given a strong aversion to their own
--     violation will tip rather than stiff;
--   * the engine's period-1 sighting rule ('Prax.Schedule.sightRule') keeps
--     everyone's sense of where everyone else is (or was, recently) current,
--     which is what the planner's belief-relative lookahead needs to ever
--     predict someone else's move.
module Prax.Worlds.Bar
  ( barWorld
  , playerName
  , barDirectorWorld
  , directorName
  ) where

import           Prax.Query
import           Prax.Types
import           Prax.Engine (definePractices, defineFunctions, performOutcome, setCharacters, setSchedule)
import           Prax.Core (coreFns, adjustScore, warmth, scoreAtLeast)
import           Prax.Emotion (feelTowardFor, feelingToward, happy, sad, annoyed, pleased)
import           Prax.Reactions
import           Prax.Deontic
import           Prax.Beliefs
import           Prax.Conversation
import           Prax.Arc
import           Prax.Sight (sightedWithin)
import           Prax.Schedule (sightRule)
import           Prax.Witness (CoPresence)

-- | The character the human player controls.
playerName :: String
playerName = "you"

-- Once your warmth toward someone reaches this, you may buy them a drink.
buyThreshold :: Int
buyThreshold = 15

-- Practices ---------------------------------------------------------------------

-- | Co-presence at the bar: sharing a place.
together :: CoPresence
together = [ Match "practice.world.world.at.Actor!P"
           , Match "practice.world.world.at.Witness!P" ]

-- | The bar's sighting template, over the same movement vocabulary as
-- 'together': whoever shares a place with you is someone you see.
barSighting :: [Condition]
barSighting = [ Match "practice.world.world.at.Seer!Spot"
               , Match "practice.world.world.at.Seen!Spot" ]

-- Locations and movement between connected places.
worldP :: Practice
worldP = practice
  { practiceId = "world"
  , practiceName = "The world exists"
  , roles = ["World"]
  , actions =
      [ action "[Actor]: Go to [Place]"
          [ Match "practice.world.World.at.Actor!OtherPlace"
          , Match "practice.world.World.connected.OtherPlace.Place" ]
          [ Insert "practice.world.World.at.Actor!Place" ]
      , action "[Actor]: Wait a moment"
          [ Match "practice.world.World.at.Actor!Place" ]
          []
      ]
  }

-- Initiating a greeting (and the warmth-gated gift of a drink). The *response*
-- to a greeting lives in `respondGreetP`.
greetP :: Practice
greetP = practice
  { practiceId = "greet"
  , practiceName = "People can greet one another"
  , roles = ["World"]
  , actions =
      [ action "[Actor]: Greet [Other]"
          [ Match "practice.world.world.at.Actor!Place"
          , Match "practice.world.world.at.Other!Place"
          , Neq "Actor" "Other"
          , Not "practice.greet.World.greeted.Actor.Other"
          , Not (beliefSentence "Actor" "resentedBy.Other" "yes")  -- wary of those you think dislike you
            -- if they've greeted you and you owe a response, use that, not a new greeting
          , Not (reactionPath "respondGreet" ["Other", "Actor"]) ]
          [ Insert "practice.greet.World.greeted.Actor.Other"
          , adjustScore "Actor" "Other" warmth 10 "greeting"
          , feelTowardFor 4 "Actor" pleased "Other"
          , spawnReaction "respondGreet" ["Actor", "Other"] ]

      , action "[Actor]: Warn [Hearer] that [Subject] resents them"
          [ Match "practice.world.world.at.Actor!Place"
          , Match "practice.world.world.at.Hearer!Place"
          , Neq "Actor" "Hearer"
          , feelingToward "Actor" annoyed "Subject"     -- you must actually be cross with Subject
          , Neq "Actor" "Subject"
          , Neq "Hearer" "Subject"
          , Not "practice.world.world.at.Subject!Place"    -- behind Subject's back
          , Not (beliefSentence "Hearer" "resentedBy.Subject" "yes") ]
          [ believe "Hearer" "resentedBy.Subject" "yes" ]  -- a possibly-false rumour

      , action "[Actor]: Realize [Subject] doesn't resent you after all"
          [ Match (beliefSentence "Actor" "resentedBy.Subject" "yes")
          , Match "practice.greet.world.greeted.Subject.Actor" ]  -- they greeted you: evidence
          [ forget "Actor" "resentedBy.Subject"
          , feelTowardFor 4 "Actor" pleased "Subject" ]

      , action "[Actor]: Strike up a conversation with [Other]"
          ( [ Match "practice.world.world.at.Actor!Place"
            , Match "practice.world.world.at.Other!Place"
            , Neq "Actor" "Other"
            , Not (beliefSentence "Actor" "resentedBy.Other" "yes")
            , Not (talkPath "Actor" "Other")           -- not already talking (either order)
            , Not (talkPath "Other" "Actor")
            , Not "Actor.chattedWith.Other" ]           -- one conversation per pair
            ++ scoreAtLeast "Actor" "Other" warmth buyThreshold )
          (beginConversation "Actor" "Other" "smallTalk")

      , action "[Actor]: Buy [Other] a drink"
          ( [ Match "practice.world.world.at.Actor!Place"
            , Match "practice.world.world.at.Other!Place"
            , Neq "Actor" "Other"
            , Not "practice.greet.World.grievance.Actor.Other"
            , Not (beliefSentence "Actor" "resentedBy.Other" "yes")
            , Not "practice.greet.World.bought.Actor.Other" ]
            ++ scoreAtLeast "Actor" "Other" warmth buyThreshold )
          [ Insert "practice.greet.World.bought.Actor.Other"
          , adjustScore "Other" "Actor" warmth 15 "boughtMeADrink"
          , adjustScore "Actor" "Other" warmth 5 "feelingGenerous"
          , feelTowardFor 4 "Actor" pleased "Other"
          , feelTowardFor 4 "Other" pleased "Actor" ]
      ]
  }

-- Responding to a greeting: greet back, rebuff, or (for the greeter) take
-- offense that the greeting was ignored. Spawned as
-- practice.respondGreet.<Greeter>.<Greeted>.
respondGreetP :: Practice
respondGreetP = practice
  { practiceId = "respondGreet"
  , practiceName = "[Greeted] can respond to [Greeter]'s greeting"
  , roles = ["Greeter", "Greeted"]
  , actions =
      [ action "[Actor]: Greet [Greeter] back"
          [ Eq "Actor" "Greeted"
          , Not (beliefSentence "Greeted" "resentedBy.Greeter" "yes") ]
          [ Insert "practice.greet.world.greeted.Greeted.Greeter"
          , adjustScore "Greeted" "Greeter" warmth 10 "greetedBack"
          , feelTowardFor 4 "Greeted" pleased "Greeter"
          , endReaction "respondGreet" ["Greeter", "Greeted"] ]

      , action "[Actor]: Rebuff [Greeter]"
          [ Eq "Actor" "Greeted" ]
          [ feelTowardFor 4 "Greeted" annoyed "Greeter"
          , adjustScore "Greeted" "Greeter" warmth (-5) "rebuffed"
          , feelTowardFor 4 "Greeter" sad "Greeted"
          , adjustScore "Greeter" "Greeted" warmth (-10) "rebuffedMe"
          , endReaction "respondGreet" ["Greeter", "Greeted"] ]

      , action "[Actor]: Take offense that [Greeted] ignored your greeting"
          [ Eq "Actor" "Greeter"
          , Not "practice.greet.world.greeted.Greeted.Greeter"
          , Not "practice.greet.world.grievance.Greeter.Greeted" ]
          [ Insert "practice.greet.world.grievance.Greeter.Greeted"
          , feelTowardFor 4 "Greeter" annoyed "Greeted"
          , adjustScore "Greeter" "Greeted" warmth (-15) "snubbedMe"
          , endReaction "respondGreet" ["Greeter", "Greeted"] ]
      ]
  }

-- A patron: exists to hold a per-person drink counter, seeded on spawn.
patronP :: Practice
patronP = practice
  { practiceId = "patron"
  , practiceName = "[Patron] is a patron"
  , roles = ["Patron"]
  , initOutcomes =
      [ Insert "practice.patron.Patron.drinks!0"
      , enterArc "Patron" "hopeful" ]   -- everyone arrives hopeful
  }

-- A character's inner arc: hopeful arrivals either come to feel they belong
-- (if someone stands them a drink) or withdraw, feeling out of place (if someone
-- comes to resent them). These are internal, high-level choices; the stage a
-- character is in changes what they want (see bex's wants).
arcP :: Practice
arcP = practice
  { practiceId = "arc"
  , practiceName = "[Self]'s evening"
  , roles = ["Self"]
  , actions =
      [ -- The rewarding beat an NPC pursues: once you feel genuinely warm toward
        -- someone here, you can decide you belong (bex's +25 want drives this).
        action "[Actor]: settle in, feeling you belong here"
          ( [ Eq "Actor" "Self", arcIs "Actor" "hopeful" ]
            ++ scoreAtLeast "Actor" "Friend" warmth 20 )   -- you've warmed to someone
          [ enterArc "Actor" "belonging"
          , feelTowardFor 4 "Actor" happy "here" ]

        -- A transformation *against* one's desires: sliding into loneliness
        -- forecloses the belonging you crave (+25) and is itself dreaded (-25),
        -- with no way back — so the utility planner never chooses it. In practice
        -- only the player ever does: "true transformation is available only to
        -- the player" (Versu §X).
      , action "[Actor]: give up on the evening, resigning yourself to solitude"
          [ Eq "Actor" "Self"
          , arcIs "Actor" "hopeful" ]
          [ enterArc "Actor" "lonely"
          , feelTowardFor 4 "Actor" sad "here" ]
      ]
  }

-- Tending bar: order, fulfill (which warms the customer and spawns a tip
-- obligation), drink (getting tipsy), and the busy-bar bell.
tendBarP :: Practice
tendBarP = practice
  { practiceId = "tendBar"
  , practiceName = "[Bartender] is tending bar at [Place]"
  , roles = ["Place", "Bartender"]
  , dataFacts =
      [ "beverageType.beer!alcoholic", "beverageType.cider!alcoholic"
      , "beverageType.soda!nonalcoholic", "beverageType.water!nonalcoholic" ]
  , actions =
      [ action "[Actor]: Order [Beverage]"
          [ Neq "Actor" "Bartender"
          , Match "practice.world.world.at.Actor!Place"
          , Not "practice.tendBar.Place.Bartender.customer.Actor!order"
          , Not "practice.tendBar.Place.Bartender.customer.Actor!beverage"
          , Match "practiceData.tendBar.beverageType.Beverage" ]
          [ Insert "practice.tendBar.Place.Bartender.customer.Actor!order!Beverage" ]
      , action "[Actor]: Fulfill [Customer]'s order"
          [ Eq "Actor" "Bartender"
          , Match "practice.tendBar.Place.Bartender.customer.Customer!order!Beverage"
          , Match "practice.world.world.at.Bartender!Place" ]
          [ Delete "practice.tendBar.Place.Bartender.customer.Customer!order"
          , Insert "practice.tendBar.Place.Bartender.customer.Customer!beverage!Beverage"
          , adjustScore "Customer" "Bartender" warmth 8 "servedMeWell"
          , feelTowardFor 4 "Customer" pleased "Bartender"
          , spawnReaction "settleUp" ["Customer", "Bartender"]
            -- being served creates a real obligation to settle (a first-class □)
          , oblige "Customer" "Customer.tipped.Bartender" ]
      , action "[Actor]: Drink the [Beverage]"
          [ Match "practice.tendBar.Place.Bartender.customer.Actor!beverage!Beverage"
          , Match "practiceData.tendBar.beverageType.Beverage!Kind"
          , Match "practice.patron.Actor.drinks!N" ]
          [ Delete "practice.tendBar.Place.Bartender.customer.Actor!beverage"
          , Call "recordDrink" ["Actor", "Kind", "N"] ]
      , action "[Actor]: Ring the bell — busy bar!"
          [ Eq "Actor" "Bartender"
          , Subquery { subSet = "Crowd", subFind = ["C"]
                     , subWhere = [ Match "practice.tendBar.Place.Bartender.customer.C" ] }
          , Count "NumCust" "Crowd"
          , Cmp Gte "NumCust" "2"
          , Not "practice.tendBar.Place.Bartender.rang" ]
          [ Insert "practice.tendBar.Place.Bartender.rang" ]
      ]
  }

-- The bar's drink-counter functions (spec v47: the world registry, registered
-- via 'Prax.Engine.defineFunctions' beside 'coreFns' — not a practice field).
recordDrinkFn, checkTipsyFn, checkSoberFn :: Function
recordDrinkFn = Function "recordDrink" ["P", "Kind", "N"]
  [ FnCase [ Eq "Kind" "alcoholic", Calc "M" Add "N" "1" ]
           [ Insert "practice.patron.P.drinks!M"
           , Call "checkTipsy" ["P", "M"] ]
  , FnCase [] []
  ]
checkTipsyFn = Function "checkTipsy" ["P", "M"]
  [ FnCase [ Cmp Gte "M" "2" ] [ Insert "person.P.tipsy" ] ]
checkSoberFn = Function "checkSober" ["P", "M"]
  [ FnCase [ Cmp Lte "M" "1" ] [ Delete "person.P.tipsy" ] ]

barFns :: [Function]
barFns = coreFns ++ [recordDrinkFn, checkTipsyFn, checkSoberFn]

-- TEST-COMPRESSED cadence (see Prax.Schedule's authoring note; real
-- authoring: ~12 rounds, an hour a drink): each firing metabolizes one drink
-- from every patron who has any, and sobriety returns when the count falls
-- back under checkTipsy's own threshold (its mirror, one home).
metabolism :: ScheduleRule
metabolism = ScheduleRule "metabolism" 2
  [ ( [ Match "practice.patron.P.drinks!N"
      , Cmp Gte "N" "1"
      , Calc "M" Sub "N" "1" ]
    , [ Insert "practice.patron.P.drinks!M"
      , Call "checkSober" ["P", "M"] ] ) ]

-- A ready-made reaction: when spawned as @practice.disapproval.\<Offender\>.\<Onlooker\>@,
-- it offers the onlooker a chance to disapprove of (or forgive) the offender.
-- Spawned by 'settleUpP' below with @'spawnReaction' "disapproval" [patron, bartender]@
-- when a tab goes unpaid.
disapprovalP :: Practice
disapprovalP = practice
  { practiceId   = "disapproval"
  , practiceName = "[Onlooker] saw [Offender] break a norm"
  , roles        = ["Offender", "Onlooker"]
  , actions =
      [ action "[Actor]: Disapprove of [Offender]"
          [ Eq "Actor" "Onlooker" ]
          [ Insert "Onlooker.disapprovedOf.Offender"
          , feelTowardFor 4 "Onlooker" annoyed "Offender"
          , adjustScore "Onlooker" "Offender" warmth (-20) "brokeANorm"
          , endReaction "disapproval" ["Offender", "Onlooker"] ]
      , action "[Actor]: Let [Offender]'s lapse slide"
          [ Eq "Actor" "Onlooker" ]
          [ feelTowardFor 4 "Onlooker" pleased "Offender"
          , endReaction "disapproval" ["Offender", "Onlooker"] ]
      ]
  }

-- Settling up after being served: the obligation "[Patron] should tip
-- [Bartender]" (a first-class deontic □, raised on serve — see 'oblige' above) is
-- discharged by tipping, or breached by leaving the tab unpaid — a norm violation
-- that spawns the bartender's disapproval AND, contrary-to-duty (□□), a reparative
-- obligation to make amends. Spawned as practice.settleUp.<Patron>.<Bartender>.
settleUpP :: Practice
settleUpP = practice
  { practiceId = "settleUp"
  , practiceName = "[Patron] should settle up with [Bartender]"
  , roles = ["Patron", "Bartender"]
  , actions =
      [ action "[Actor]: Tip [Bartender]"
          [ Eq "Actor" "Patron" ]
          [ Insert "Patron.tipped.Bartender"            -- fulfils the duty's content…
          , discharge "Patron" "Patron.tipped.Bartender" -- …so the obligation is met and closed
          , adjustScore "Bartender" "Patron" warmth 8 "aGoodTipper"
          , adjustScore "Patron" "Bartender" warmth 3 "friendlyService"
          , feelTowardFor 4 "Bartender" pleased "Patron"
          , endReaction "settleUp" ["Patron", "Bartender"] ]
      , action "[Actor]: Leave [Bartender]'s tab unpaid"
          [ Eq "Actor" "Patron" ]
          [ breach "Patron" "stiffedTheBartender"         -- the duty is breached (= a violation)
          , discharge "Patron" "Patron.tipped.Bartender"  -- the original duty can no longer be met…
          , obligeReparative "Patron" "make.amends.with.Bartender"  -- …so a reparative □□ duty arises
          , spawnReaction "disapproval" ["Patron", "Bartender"]
          , endReaction "settleUp" ["Patron", "Bartender"] ]
      ]
  }

-- A conversation between two friends: small talk, compliments, or gossip.
-- Quips say a line and shift the core model / plant beliefs. Spawned by "Strike
-- up a conversation"; participants stay on a topic until someone changes it.
converseP :: Practice
converseP = practice
  { practiceId = "converse"
  , practiceName = "[A] and [B] are chatting"
  , roles = ["A", "B"]
  , actions =
      [ quip "smalltalk" "[Actor]: Make small talk with [Partner]" "smallTalk" []
          [ adjustScore "Actor" "Partner" warmth 2 "pleasantChat"
          , adjustScore "Partner" "Actor" warmth 2 "pleasantChat" ]
      , quip "compliment" "[Actor]: Compliment [Partner]" "rapport" []
          [ adjustScore "Partner" "Actor" warmth 8 "kindWords"
          , feelTowardFor 4 "Partner" pleased "Actor" ]
      , quip "gossip" "[Actor]: Confide to [Partner] that [Subject] resents them" "gossip"
          [ feelingToward "Actor" annoyed "Subject"
          , Neq "Actor" "Subject", Neq "Partner" "Subject" ]
          [ believe "Partner" "resentedBy.Subject" "yes" ]
      , changeSubject "[Actor]: Warm the talk toward rapport" "rapport"
      , changeSubject "[Actor]: Lower your voice to gossip" "gossip"
      , changeSubject "[Actor]: Keep it to small talk" "smallTalk"
      , endConversation "[Actor]: Wrap up the chat with [Partner]"
      ]
  }

-- The story manager (Versu's DM): an autonomous agent with *metalevel* desires
-- that shapes the drama without controlling anyone directly. Here it watches for
-- a too-cosy room and injects a falling-out between two friends — which then
-- plays out through the ordinary reaction / gossip machinery.
dmPractice :: Practice
dmPractice = practice
  { practiceId = "dm"
  , practiceName = "the director shapes the evening"
  , roles = ["Director"]
  , actions =
      [ action "[Actor]: turn [X] against [Y] to stir up the evening"
          ( [ Eq "Actor" "Director"
            , Not "dm.stirred" ]                  -- one dramatic beat per evening
            ++ scoreAtLeast "X" "Y" warmth 20     -- bind two who currently like each other…
            ++ [ Neq "X" "Y", Neq "X" "Actor", Neq "Y" "Actor" ] )  -- …then require them distinct
          [ Insert "dm.stirred"
          , feelTowardFor 4 "X" annoyed "Y"
          , adjustScore "X" "Y" warmth (-30) "aSuddenFallingOut"
          , Insert "practice.greet.world.grievance.X.Y" ]
      ]
  }

-- Player-as-DM (Versu §XI): the same metalevel role as the autonomous
-- `director`, but a palette of authorial nudges for a *human* to steer an
-- otherwise autonomous cast — stirring conflict, kindling warmth, or souring the
-- mood — without ever embodying a character. The player is bound to this
-- practice in `barDirectorWorld`, so their menu is these nudges and nothing else.
-- Each nudge is one dramatic beat per participants (it can't be spammed), and it
-- reshapes the story only indirectly: the cast then reacts through the ordinary
-- greeting / conversation / arc machinery.
directP :: Practice
directP = practice
  { practiceId   = "direct"
  , practiceName = "you direct the evening"
  , roles        = ["Director"]
  , actions =
      [ action "[Actor]: stir up a rivalry between [X] and [Y]"
          [ Eq "Actor" "Director"
          , Match "practice.world.world.at.X!Px"
          , Match "practice.world.world.at.Y!Py"
          , Neq "X" "Y", Neq "X" "Director", Neq "Y" "Director"
          , Not "direct.stirred.X.Y" ]
          [ Insert "direct.stirred.X.Y"
          , feelTowardFor 4 "X" annoyed "Y"
          , adjustScore "X" "Y" warmth (-30) "aFallingOut"
          , Insert "practice.greet.world.grievance.X.Y" ]

      , action "[Actor]: kindle warmth between [X] and [Y]"
          [ Eq "Actor" "Director"
          , Match "practice.world.world.at.X!Px"
          , Match "practice.world.world.at.Y!Py"
          , Neq "X" "Y", Neq "X" "Director", Neq "Y" "Director"
          , Not "direct.kindled.X.Y" ]
          [ Insert "direct.kindled.X.Y"
          , adjustScore "X" "Y" warmth 15 "aWarmFeeling"
          , adjustScore "Y" "X" warmth 15 "aWarmFeeling"
          , feelTowardFor 4 "X" pleased "Y"
          , feelTowardFor 4 "Y" pleased "X" ]

      , action "[Actor]: cast a pall over [X]'s evening"
          [ Eq "Actor" "Director"
          , Match "practice.world.world.at.X!Px"
          , Neq "X" "Director"
          , Not "direct.unsettled.X" ]
          [ Insert "direct.unsettled.X"
          , feelTowardFor 4 "X" sad "here" ]
      ]
  }

-- Cast --------------------------------------------------------------------------

you :: Character
you = character playerName   -- the player chooses; no wants drive them

-- The bartender: tends the bar, greets, disapproves of norm-breakers, and takes
-- offense if snubbed.
ada :: Character
ada = (character "ada")
  { charWants =
      [ Want [ Match "practice.tendBar.Place.ada.customer.Customer!order" ] (-5)
      , Want [ Match "practice.world.world.at.ada!bar" ] 1
      , Want [ Match "practice.greet.world.greeted.ada.Other" ] 2
      , Want [ Match "practice.greet.world.grievance.ada.Other" ] 2
      , Want [ Match "ada.disapprovedOf.Offender" ] 3          -- disapproves of tab-skippers
        -- Grudging courtesy (v38): the gate that once withheld greeting (or
        -- chatting with) a cross target is gone; the reluctance is priced
        -- instead. -3 outweighs the +2 ordinary appeal of greeting (this
        -- same want, just above — greeting and greeting-back write the
        -- identical fact); it dominates trivially for striking up a
        -- conversation too, which carries no self-want of its own to
        -- outweigh.
      , Want [ Match "ada.feels.annoyed.toward.T"
             , Or [ [ Match "practice.greet.world.greeted.ada.T" ]
                  , [ Match "ada.chattedWith.T" ] ] ] (-3)
      ] }

-- A patron who wants a beer, greets people, tips (and strongly avoids stiffing
-- the bartender), and — once warm toward ada — stands her a drink.
bex :: Character
bex = (character "bex")
  { charWants =
      [ Want [ Match "practice.tendBar.Place.ada.customer.bex!order!beer" ] 4
      , Want [ Match "practice.tendBar.Place.ada.customer.bex!beverage!beer" ] 9
      , Want [ Match "practice.world.world.at.bex!bar" ] 1
      , Want [ Match "practice.greet.world.greeted.bex.Other" ] 2
      , Want [ Match "practice.greet.world.grievance.bex.Other" ] 2
      , Want [ Match "practice.greet.world.bought.bex.Friend" ] 6
      , Want [ Match "bex.tipped.ada" ] 3                       -- likes to tip
      , Want [ violationOf "bex" "stiffedTheBartender" ] (-40)  -- and hates stiffing
        -- the arc: bex yearns to belong and dreads loneliness, and once settled
        -- behaves accordingly (lingers if it belongs, drifts home if it doesn't)
      , Want [ arcIs "bex" "belonging" ] 25
      , Want [ arcIs "bex" "lonely" ] (-25)
      , Want [ arcIs "bex" "belonging", Match "practice.world.world.at.bex!bar" ] 3
      , Want [ arcIs "bex" "lonely", Match "practice.world.world.at.bex!entrance" ] 8
        -- Grudging courtesy (v38): as ada's, above — -3 outweighs the +2
        -- greeting/greeting-back appeal (this same want, just above), and
        -- dominates trivially for conversation, which has no self-want to
        -- outweigh.
      , Want [ Match "bex.feels.annoyed.toward.T"
             , Or [ [ Match "practice.greet.world.greeted.bex.T" ]
                  , [ Match "bex.chattedWith.T" ] ] ] (-3)
        -- Grudging courtesy, the round (v38): buying T a drink while cross
        -- with T grates strongly enough to outweigh the round's own +6
        -- appeal (the bought.bex.Friend want, just above) — the v38
        -- invariant: the gate is gone, the reluctance is priced.
      , Want [ Match "bex.feels.annoyed.toward.T"
             , Match "practice.greet.world.bought.bex.T" ] (-8)
      ] }

-- The director: no physical presence, only metalevel desires; bound to its own
-- practice, so it never greets or drinks — it only shapes the story.
director :: Character
director = (character "director")
  { charWants   = [ Want [ Match "dm.stirred" ] 20 ]  -- wants the evening to have a spark
  , charBoundTo = Just "dm" }

-- A second patron, so the player-DM has a lively cast to play off (stir bex
-- against cai, kindle either toward ada, …). Wants a cider and, like bex, to
-- belong.
cai :: Character
cai = (character "cai")
  { charWants =
      [ Want [ Match "practice.tendBar.Place.ada.customer.cai!order!cider" ] 4
      , Want [ Match "practice.tendBar.Place.ada.customer.cai!beverage!cider" ] 9
      , Want [ Match "practice.world.world.at.cai!bar" ] 1
      , Want [ Match "practice.greet.world.greeted.cai.Other" ] 2
      , Want [ Match "practice.greet.world.bought.cai.Friend" ] 6
      , Want [ arcIs "cai" "belonging" ] 25
      , Want [ arcIs "cai" "lonely" ] (-25)
        -- Grudging courtesy (v38): as ada's/bex's, above.
      , Want [ Match "cai.feels.annoyed.toward.T"
             , Or [ [ Match "practice.greet.world.greeted.cai.T" ]
                  , [ Match "cai.chattedWith.T" ] ] ] (-3)
        -- Grudging courtesy, the round (v38): as bex's, above.
      , Want [ Match "cai.feels.annoyed.toward.T"
             , Match "practice.greet.world.bought.cai.T" ] (-8)
      ] }

-- | The name of the drama-manager the player controls in 'barDirectorWorld'.
directorName :: String
directorName = "director"

-- The player-controlled DM: bound to 'directP', no wants of its own — the human
-- supplies the intent, choosing nudges from a menu each turn.
directorPlayer :: Character
directorPlayer = (character directorName) { charBoundTo = Just "direct" }

-- Initial world ----------------------------------------------------------------

-- Sort declarations for the type checker (`Prax.TypeCheck`): the clearly
-- monomorphic slots of the bar. Positions like a mood's target are deliberately
-- left unsorted, since they are genuinely polymorphic (you can feel a way toward
-- a person or a place).
barSorts :: [(String, [String])]
barSorts =
  [ ("beverage",     ["beer", "cider", "soda", "water"])
  , ("beverageKind", ["alcoholic", "nonalcoholic"])
  , ("place",        ["bar", "entrance"])
  ]

-- | The fully initialized bar: practices (core-model + reaction libraries)
-- defined, instances spawned, cast placed.
barWorld :: PraxState
barWorld =
  (foldl' (flip performOutcome) withPractices setup)
    -- an epistemic prediction scope: you credit another's predicted move only
    -- if you're with them now, or you sighted them within the last 2 ticks —
    -- one tick per round, and two rounds is roughly a there-and-back trip to
    -- the next room: "you assume people stay put for about as long as it
    -- takes to walk one room away and back."
    { predictionScope = [ Or [ together, sightedWithin 2 ] ] }
  where
    -- The engine owns time now (v44): the schedule fires sight (period 1) and
    -- metabolism (period 2) at each round boundary — no ticker characters.
    withPractices =
      setSchedule [ sightRule barSighting, metabolism ]
        ((setCharacters [you, ada, bex, director]
           (defineFunctions barFns
             (definePractices
                [ disapprovalP
                , worldP, greetP, respondGreetP, patronP, tendBarP, settleUpP, converseP, dmPractice
                , arcP ]
                emptyState)))
           { sorts = barSorts })
    setup =
      [ Insert "practice.world.world.connected.entrance.bar"
      , Insert "practice.world.world.connected.bar.entrance"
      , Insert "practice.world.world.at.you!entrance"
      , Insert "practice.world.world.at.bex!entrance"
      , Insert "practice.world.world.at.ada!bar"
      , Insert "practice.patron.you"
      , Insert "practice.patron.bex"
      , Insert "practice.greet.world"
      , Insert "practice.tendBar.bar.ada"
      , Insert "practice.dm.director"
      , Insert "practice.arc.you"
      , Insert "practice.arc.bex"
      ]

-- | The same bar, but the human is the __drama manager__ (Versu §XI): the player
-- controls 'directorPlayer', steering an autonomous cast (ada, bex, cai) with
-- authorial nudges instead of embodying a character. There is no @you@ — the
-- player is the unseen hand shaping the evening.
barDirectorWorld :: PraxState
barDirectorWorld =
  (foldl' (flip performOutcome) withPractices setup)
    -- same epistemic prediction scope, and the same stated horizon
    -- rationale, as 'barWorld'.
    { predictionScope = [ Or [ together, sightedWithin 2 ] ] }
  where
    withPractices =
      setSchedule [ sightRule barSighting ]
        ((setCharacters [ada, bex, cai, directorPlayer]
           (defineFunctions barFns
             (definePractices
                [ disapprovalP
                , worldP, greetP, respondGreetP, patronP, tendBarP, settleUpP, converseP, directP
                , arcP ]
                emptyState)))
           { sorts = barSorts })
    setup =
      [ Insert "practice.world.world.connected.entrance.bar"
      , Insert "practice.world.world.connected.bar.entrance"
      , Insert "practice.world.world.at.ada!bar"
      , Insert "practice.world.world.at.bex!bar"   -- patrons already in the room
      , Insert "practice.world.world.at.cai!bar"   -- with the bartender, for the DM to play off
      , Insert "practice.patron.bex"
      , Insert "practice.patron.cai"
      , Insert "practice.greet.world"
      , Insert "practice.tendBar.bar.ada"
      , Insert "practice.direct.director"
      , Insert "practice.arc.bex"
      , Insert "practice.arc.cai"
      ]
