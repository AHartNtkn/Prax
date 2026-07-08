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
--     violation will tip rather than stiff.
module Prax.Worlds.Bar
  ( barWorld
  , playerName
  , barDirectorWorld
  , directorName
  ) where

import           Prax.Query
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome)
import           Prax.Core
import           Prax.Reactions
import           Prax.Beliefs
import           Prax.Conversation
import           Prax.Arc

-- | The character the human player controls.
playerName :: String
playerName = "you"

-- Once your warmth toward someone reaches this, you may buy them a drink.
buyThreshold :: Int
buyThreshold = 15

-- Practices ---------------------------------------------------------------------

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
          , Not "Actor.mood!annoyed.toward!Other"
          , Not (beliefSentence "Actor" "resentedBy.Other" "yes")  -- wary of those you think dislike you
            -- if they've greeted you and you owe a response, use that, not a new greeting
          , Not (reactionPath "respondGreet" ["Other", "Actor"]) ]
          [ Insert "practice.greet.World.greeted.Actor.Other"
          , adjustScore "Actor" "Other" warmth 10 "greeting"
          , setMood "Actor" pleased "Other" "greeting"
          , spawnReaction "respondGreet" ["Actor", "Other"] ]

      , action "[Actor]: Warn [Hearer] that [Subject] resents them"
          [ Match "practice.world.world.at.Actor!Place"
          , Match "practice.world.world.at.Hearer!Place"
          , Neq "Actor" "Hearer"
          , Match "Actor.mood!annoyed.toward!Subject"     -- you must actually be cross with Subject
          , Neq "Actor" "Subject"
          , Neq "Hearer" "Subject"
          , Not "practice.world.world.at.Subject!Place"    -- behind Subject's back
          , Not (beliefSentence "Hearer" "resentedBy.Subject" "yes") ]
          [ believe "Hearer" "resentedBy.Subject" "yes" ]  -- a possibly-false rumour

      , action "[Actor]: Realize [Subject] doesn't resent you after all"
          [ Match (beliefSentence "Actor" "resentedBy.Subject" "yes")
          , Match "practice.greet.world.greeted.Subject.Actor" ]  -- they greeted you: evidence
          [ forget "Actor" "resentedBy.Subject"
          , setMood "Actor" pleased "Subject" "reassured" ]

      , action "[Actor]: Strike up a conversation with [Other]"
          ( [ Match "practice.world.world.at.Actor!Place"
            , Match "practice.world.world.at.Other!Place"
            , Neq "Actor" "Other"
            , Not "Actor.mood!annoyed.toward!Other"
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
            , Not "Actor.mood!annoyed.toward!Other"
            , Not (beliefSentence "Actor" "resentedBy.Other" "yes")
            , Not "practice.greet.World.bought.Actor.Other" ]
            ++ scoreAtLeast "Actor" "Other" warmth buyThreshold )
          [ Insert "practice.greet.World.bought.Actor.Other"
          , adjustScore "Other" "Actor" warmth 15 "boughtMeADrink"
          , adjustScore "Actor" "Other" warmth 5 "feelingGenerous"
          , setMood "Actor" pleased "Other" "generosity"
          , setMood "Other" pleased "Actor" "aFreeDrink" ]
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
          , Not "Greeted.mood!annoyed.toward!Greeter"
          , Not (beliefSentence "Greeted" "resentedBy.Greeter" "yes") ]
          [ Insert "practice.greet.world.greeted.Greeted.Greeter"
          , adjustScore "Greeted" "Greeter" warmth 10 "greetedBack"
          , setMood "Greeted" pleased "Greeter" "greetedBack"
          , endReaction "respondGreet" ["Greeter", "Greeted"] ]

      , action "[Actor]: Rebuff [Greeter]"
          [ Eq "Actor" "Greeted" ]
          [ setMood "Greeted" annoyed "Greeter" "notInTheMood"
          , adjustScore "Greeted" "Greeter" warmth (-5) "rebuffed"
          , setMood "Greeter" sad "Greeted" "wasRebuffed"
          , adjustScore "Greeter" "Greeted" warmth (-10) "rebuffedMe"
          , endReaction "respondGreet" ["Greeter", "Greeted"] ]

      , action "[Actor]: Take offense that [Greeted] ignored your greeting"
          [ Eq "Actor" "Greeter"
          , Not "practice.greet.world.greeted.Greeted.Greeter"
          , Not "practice.greet.world.grievance.Greeter.Greeted" ]
          [ Insert "practice.greet.world.grievance.Greeter.Greeted"
          , setMood "Greeter" annoyed "Greeted" "ignoredMyGreeting"
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
          , setMood "Actor" happy "here" "foundMyPeople" ]

        -- A transformation *against* one's desires: sliding into loneliness
        -- forecloses the belonging you crave (+25) and is itself dreaded (-25),
        -- with no way back — so the utility planner never chooses it. In practice
        -- only the player ever does: "true transformation is available only to
        -- the player" (Versu §X).
      , action "[Actor]: give up on the evening, resigning yourself to solitude"
          [ Eq "Actor" "Self"
          , arcIs "Actor" "hopeful" ]
          [ enterArc "Actor" "lonely"
          , setMood "Actor" sad "here" "whatsThePoint" ]
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
          , setMood "Customer" pleased "Bartender" "gotMyDrink"
          , spawnReaction "settleUp" ["Customer", "Bartender"] ]
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
  , functions =
      [ Function "recordDrink" ["P", "Kind", "N"]
          [ FnCase [ Eq "Kind" "alcoholic", Calc "M" Add "N" "1" ]
                   [ Insert "practice.patron.P.drinks!M"
                   , Call "checkTipsy" ["P", "M"] ]
          , FnCase [] []
          ]
      , Function "checkTipsy" ["P", "M"]
          [ FnCase [ Cmp Gte "M" "2" ] [ Insert "person.P.tipsy" ] ]
      ]
  }

-- Settling up after being served: tip (norm respected) or leave the tab (a
-- violation that spawns the bartender's disapproval). Spawned as
-- practice.settleUp.<Patron>.<Bartender>.
settleUpP :: Practice
settleUpP = practice
  { practiceId = "settleUp"
  , practiceName = "[Patron] should settle up with [Bartender]"
  , roles = ["Patron", "Bartender"]
  , actions =
      [ action "[Actor]: Tip [Bartender]"
          [ Eq "Actor" "Patron" ]
          [ Insert "Patron.tipped.Bartender"
          , adjustScore "Bartender" "Patron" warmth 8 "aGoodTipper"
          , adjustScore "Patron" "Bartender" warmth 3 "friendlyService"
          , setMood "Bartender" pleased "Patron" "aGoodTip"
          , endReaction "settleUp" ["Patron", "Bartender"] ]
      , action "[Actor]: Leave [Bartender]'s tab unpaid"
          [ Eq "Actor" "Patron" ]
          [ markViolation "Patron" "stiffedTheBartender"
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
          , setMood "Partner" pleased "Actor" "flattered" ]
      , quip "gossip" "[Actor]: Confide to [Partner] that [Subject] resents them" "gossip"
          [ Match "Actor.mood!annoyed.toward!Subject"
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
          , setMood "X" annoyed "Y" "aBitterMisunderstanding"
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
          , setMood "X" annoyed "Y" "aSuddenCoolness"
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
          , setMood "X" pleased "Y" "aSuddenFondness"
          , setMood "Y" pleased "X" "aSuddenFondness" ]

      , action "[Actor]: cast a pall over [X]'s evening"
          [ Eq "Actor" "Director"
          , Match "practice.world.world.at.X!Px"
          , Neq "X" "Director"
          , Not "direct.unsettled.X" ]
          [ Insert "direct.unsettled.X"
          , setMood "X" sad "here" "aCreepingUnease" ]
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
      ] }

-- | The name of the drama-manager the player controls in 'barDirectorWorld'.
directorName :: String
directorName = "director"

-- The player-controlled DM: bound to 'directP', no wants of its own — the human
-- supplies the intent, choosing nudges from a menu each turn.
directorPlayer :: Character
directorPlayer = (character directorName) { charBoundTo = Just "direct" }

-- Initial world ----------------------------------------------------------------

-- | The fully initialized bar: practices (core-model + reaction libraries)
-- defined, instances spawned, cast placed.
barWorld :: PraxState
barWorld =
  foldl' (flip performOutcome) withPractices setup
  where
    withPractices =
      (definePractices
         [ coreLib, disapprovalP
         , worldP, greetP, respondGreetP, patronP, tendBarP, settleUpP, converseP, dmPractice
         , arcP ]
         emptyState)
        { characters = [you, ada, bex, director] }
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
  foldl' (flip performOutcome) withPractices setup
  where
    withPractices =
      (definePractices
         [ coreLib, disapprovalP
         , worldP, greetP, respondGreetP, patronP, tendBarP, settleUpP, converseP, directP
         , arcP ]
         emptyState)
        { characters = [ada, bex, cai, directorPlayer] }
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
