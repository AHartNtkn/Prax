-- | A small demo storyworld: a bar with a movement/location practice, greeting,
-- and tending bar (order → fulfill → drink). Adapted from Praxish's @demos/pwim@
-- domain into the eDSL, then extended so a single playthrough exercises every
-- v1 engine feature.
--
-- Cast: @you@ (the player), @ada@ (an NPC bartender who dislikes leaving orders
-- outstanding and likes staying at the bar), and @bex@ (an NPC patron who wants
-- a beer). With lookahead, bex walks to the bar, orders, and ada fulfills.
--
-- Feature coverage (see @docs/WALKTHROUGH.md@): the location/greet/tendBar loop
-- covers Match\/Not\/Eq\/Neq, Insert\/Delete, spawning, single- and multi-role
-- practices, dataFacts, wants, and lookahead. The added drunkenness mechanic
-- covers @init@-on-spawn, @Call@ + functions with guarded cases, @Calc@, and
-- @Cmp@; the "busy bar" bell covers @Subquery@ + @Count@ + @Cmp@.
module Prax.Worlds.Bar
  ( barWorld
  , playerName
  ) where

import           Prax.Query
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome)

-- | The character the human player controls.
playerName :: String
playerName = "you"

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
        -- Doing nothing is a real choice: without it, an agent with no useful
        -- move is forced to wander. Empty outcomes leave the world unchanged.
      , action "[Actor]: Wait a moment"
          [ Match "practice.world.World.at.Actor!Place" ]
          []
      ]
  }

-- Greeting between co-located characters (once each direction).
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
          , Not "practice.greet.World.greeted.Actor.Other" ]
          [ Insert "practice.greet.World.greeted.Actor.Other" ]
      ]
  }

-- A patron: exists to hold a per-person drink counter, seeded on spawn.
patronP :: Practice
patronP = practice
  { practiceId = "patron"
  , practiceName = "[Patron] is a patron"
  , roles = ["Patron"]
  , initOutcomes = [ Insert "practice.patron.Patron.drinks!0" ]  -- init-on-spawn
  }

-- Tending bar: order a beverage, have it fulfilled, drink it (getting tipsy
-- after two alcoholic drinks), and — when the bar is busy — ring the bell.
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
          , Insert "practice.tendBar.Place.Bartender.customer.Customer!beverage!Beverage" ]
        -- Drinking records the drink: Call into a function that (for alcoholic
        -- drinks) increments the patron's counter and checks the tipsy threshold.
      , action "[Actor]: Drink the [Beverage]"
          [ Match "practice.tendBar.Place.Bartender.customer.Actor!beverage!Beverage"
          , Match "practiceData.tendBar.beverageType.Beverage!Kind"
          , Match "practice.patron.Actor.drinks!N" ]
          [ Delete "practice.tendBar.Place.Bartender.customer.Actor!beverage"
          , Call "recordDrink" ["Actor", "Kind", "N"] ]
        -- Available only when the bar is busy: two or more customers present.
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
      [ -- Increment the patron's counter for an alcoholic drink, then check
        -- whether they have crossed the tipsy threshold. Non-alcoholic drinks
        -- fall through to the no-op case.
        Function "recordDrink" ["P", "Kind", "N"]
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

-- The bartender dislikes leaving any order outstanding (negative utility) and
-- likes staying at the bar, so she tends it rather than wandering off.
ada :: Character
ada = (character "ada")
  { charWants =
      [ Want [ Match "practice.tendBar.Place.ada.customer.Customer!order" ] (-5)
      , Want [ Match "practice.world.world.at.ada!bar" ] 1
      ] }

-- A patron who wants a beer in hand, and likes the bar.
bex :: Character
bex = (character "bex")
  { charWants =
      [ Want [ Match "practice.tendBar.Place.ada.customer.bex!order!beer" ] 4
      , Want [ Match "practice.tendBar.Place.ada.customer.bex!beverage!beer" ] 9
      , Want [ Match "practice.world.world.at.bex!bar" ] 1
      ] }

-- Initial world ----------------------------------------------------------------

-- | The fully initialized bar: practices defined, instances spawned, cast placed.
barWorld :: PraxState
barWorld =
  foldl' (flip performOutcome) withPractices setup
  where
    withPractices =
      (definePractices [worldP, greetP, patronP, tendBarP] emptyState)
        { characters = [you, ada, bex] }
    setup =
      [ -- two connected rooms
        Insert "practice.world.world.connected.entrance.bar"
      , Insert "practice.world.world.connected.bar.entrance"
        -- everyone starts at the entrance except the bartender
      , Insert "practice.world.world.at.you!entrance"
      , Insert "practice.world.world.at.bex!entrance"
      , Insert "practice.world.world.at.ada!bar"
        -- the two patrons (spawns their drink counters via init)
      , Insert "practice.patron.you"
      , Insert "practice.patron.bex"
        -- activate greeting and the bar
      , Insert "practice.greet.world"
      , Insert "practice.tendBar.bar.ada"
      ]
