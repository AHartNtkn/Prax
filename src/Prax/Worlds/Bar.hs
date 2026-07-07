-- | A small demo storyworld: a bar with movement, greeting, and tending bar,
-- now wired to the Core Model so interaction changes feelings and feelings
-- change behaviour. Adapted from Praxish's @demos/pwim@ into the eDSL.
--
-- Cast: @you@ (the player), @ada@ (an NPC bartender) and @bex@ (an NPC patron).
--
-- The social feedback loop (see @docs/WALKTHROUGH.md@): greeting and being
-- served raise a numeric @warmth@ evaluation between characters; once warm
-- enough toward someone, the "buy … a drink" affordance appears (a relationship
-- *creating* a new goal) and characters with the matching want pursue it. A
-- greeting that goes unreciprocated lets the snubbed party take offense —
-- setting an @annoyed@ mood and cooling the relationship, which then withholds
-- the friendly buy-a-drink action. Because warmth is directional, one character
-- can end up warmer than the other (asymmetry).
module Prax.Worlds.Bar
  ( barWorld
  , playerName
  ) where

import           Prax.Query
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome)
import           Prax.Core

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

-- Greeting, taking offense at a snub, and the warmth-gated gift of a drink.
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
          , Not "Actor.mood!annoyed.toward!Other" ]     -- won't greet someone you're cross with
          [ Insert "practice.greet.World.greeted.Actor.Other"
          , adjustScore "Actor" "Other" warmth 10 "greeting"
          , setMood "Actor" pleased "Other" "greeting" ]

      , action "[Actor]: Take offense at [Other] ignoring your greeting"
          [ Match "practice.greet.World.greeted.Actor.Other"
          , Not "practice.greet.World.greeted.Other.Actor"
          , Not "practice.greet.World.grievance.Actor.Other" ]
          [ Insert "practice.greet.World.grievance.Actor.Other"
          , setMood "Actor" annoyed "Other" "ignoredMyGreeting"
          , adjustScore "Actor" "Other" warmth (-15) "snubbedMe" ]

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

-- A patron: exists to hold a per-person drink counter, seeded on spawn.
patronP :: Practice
patronP = practice
  { practiceId = "patron"
  , practiceName = "[Patron] is a patron"
  , roles = ["Patron"]
  , initOutcomes = [ Insert "practice.patron.Patron.drinks!0" ]
  }

-- Tending bar: order, fulfill (which warms the customer to the bartender),
-- drink (getting tipsy), and the busy-bar bell.
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
          , setMood "Customer" pleased "Bartender" "gotMyDrink" ]
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

-- Cast --------------------------------------------------------------------------

you :: Character
you = character playerName   -- the player chooses; no wants drive them

-- The bartender: tends the bar, greets patrons, and takes offense if snubbed.
ada :: Character
ada = (character "ada")
  { charWants =
      [ Want [ Match "practice.tendBar.Place.ada.customer.Customer!order" ] (-5)
      , Want [ Match "practice.world.world.at.ada!bar" ] 1
      , Want [ Match "practice.greet.world.greeted.ada.Other" ] 2     -- likes greeting people
      , Want [ Match "practice.greet.world.grievance.ada.Other" ] 2   -- takes justified offense
      ] }

-- A patron who wants a beer, greets people, and — once warm toward the
-- bartender — likes to stand her a drink in return.
bex :: Character
bex = (character "bex")
  { charWants =
      [ Want [ Match "practice.tendBar.Place.ada.customer.bex!order!beer" ] 4
      , Want [ Match "practice.tendBar.Place.ada.customer.bex!beverage!beer" ] 9
      , Want [ Match "practice.world.world.at.bex!bar" ] 1
      , Want [ Match "practice.greet.world.greeted.bex.Other" ] 2
      , Want [ Match "practice.greet.world.grievance.bex.Other" ] 2
      , Want [ Match "practice.greet.world.bought.bex.Friend" ] 6     -- stands a warm friend a drink
      ] }

-- Initial world ----------------------------------------------------------------

-- | The fully initialized bar: practices (incl. the core-model library) defined,
-- instances spawned, cast placed.
barWorld :: PraxState
barWorld =
  foldl' (flip performOutcome) withPractices setup
  where
    withPractices =
      (definePractices [coreLib, worldP, greetP, patronP, tendBarP] emptyState)
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
