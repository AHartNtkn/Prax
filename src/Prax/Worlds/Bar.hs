-- | A small demo storyworld: a bar with a movement/location practice, greeting,
-- and tending bar (order → fulfill → drink). Adapted from Praxish's @demos/pwim@
-- domain into the eDSL.
--
-- Cast: @you@ (the player), @ada@ (an NPC bartender who dislikes leaving orders
-- outstanding), and @bex@ (an NPC patron who wants a beer). With lookahead, bex
-- will walk to the bar to enable ordering, then order; ada will fulfill.
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

-- Tending bar: order a beverage, have it fulfilled, drink it.
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
      , action "[Actor]: Drink the [Beverage]"
          [ Match "practice.tendBar.Place.Bartender.customer.Actor!beverage!Beverage" ]
          [ Delete "practice.tendBar.Place.Bartender.customer.Actor!beverage" ]
      ]
  }

-- Cast --------------------------------------------------------------------------

you :: Character
you = character playerName   -- the player chooses; no wants drive them

-- The bartender dislikes leaving any order outstanding (negative utility), so
-- fulfilling orders raises her score.
ada :: Character
ada = (character "ada")
  { charWants =
      [ Want [ Match "practice.tendBar.Place.ada.customer.Customer!order" ] (-5)
      , Want [ Match "practice.world.world.at.ada!bar" ] 1  -- stay and tend the bar
      ] }

-- A patron who wants a beer in hand.
bex :: Character
bex = (character "bex")
  { charWants =
      [ Want [ Match "practice.tendBar.Place.ada.customer.bex!order!beer" ] 4
      , Want [ Match "practice.tendBar.Place.ada.customer.bex!beverage!beer" ] 9
      , Want [ Match "practice.world.world.at.bex!bar" ] 1  -- a patron; likes the bar
      ] }

-- Initial world ----------------------------------------------------------------

-- | The fully initialized bar: practices defined, instances spawned, cast placed.
barWorld :: PraxState
barWorld =
  foldl' (flip performOutcome) withPractices setup
  where
    withPractices =
      (definePractices [worldP, greetP, tendBarP] emptyState)
        { characters = [you, ada, bex] }
    setup =
      [ -- two connected rooms
        Insert "practice.world.world.connected.entrance.bar"
      , Insert "practice.world.world.connected.bar.entrance"
        -- everyone starts at the entrance except the bartender
      , Insert "practice.world.world.at.you!entrance"
      , Insert "practice.world.world.at.bex!entrance"
      , Insert "practice.world.world.at.ada!bar"
        -- activate greeting and the bar
      , Insert "practice.greet.world"
      , Insert "practice.tendBar.bar.ada"
      ]
