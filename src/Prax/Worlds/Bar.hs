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
  ) where

import           Prax.Query
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome)
import           Prax.Core
import           Prax.Reactions

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
            -- if they've greeted you and you owe a response, use that, not a new greeting
          , Not (reactionPath "respondGreet" ["Other", "Actor"]) ]
          [ Insert "practice.greet.World.greeted.Actor.Other"
          , adjustScore "Actor" "Other" warmth 10 "greeting"
          , setMood "Actor" pleased "Other" "greeting"
          , spawnReaction "respondGreet" ["Actor", "Other"] ]

      , action "[Actor]: Buy [Other] a drink"
          ( [ Match "practice.world.world.at.Actor!Place"
            , Match "practice.world.world.at.Other!Place"
            , Neq "Actor" "Other"
            , Not "practice.greet.World.grievance.Actor.Other"
            , Not "Actor.mood!annoyed.toward!Other"
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
          , Not "Greeted.mood!annoyed.toward!Greeter" ]
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
  , initOutcomes = [ Insert "practice.patron.Patron.drinks!0" ]
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
      ] }

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
         , worldP, greetP, respondGreetP, patronP, tendBarP, settleUpP ]
         emptyState)
        { characters = [you, ada, bex] }
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
      ]
