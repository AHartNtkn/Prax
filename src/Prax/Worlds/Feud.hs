-- | An __emergent sandbox__ demo (the direction the engine is heading — a social
-- substrate for other games, not authored IF). It shows the derivation layer
-- ("Prax.Derive") doing the work: from /one/ authored fact and three domain rules,
-- a whole feud emerges — people who never met come to resent someone purely
-- through the alliance network — and it is __defeasible__: make amends and the
-- enmity dissolves on its own.
--
-- Setup: Alice wronged Bob; Bob is allied with Carol; Carol with Dave. The rules
-- (authored once) say alliances are mutual, the wronged resent the wrongdoer, and
-- "the enemy of my ally is my enemy". Forward-chaining ('readView') then derives
-- @resents.bob.alice@, @resents.carol.alice@, @resents.dave.alice@ — Alice is shunned
-- by Bob's entire network though she only ever wronged Bob. Retract the wrong
-- (Alice "makes amends") and every derived @resents@ — and the shunning — vanishes.
module Prax.Worlds.Feud
  ( feudWorld
  , bigFeud
  , feudAxioms
  , playerName
  ) where

import           Prax.Query
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, setAxioms)
import           Prax.Derive (Axiom, axiom)

-- | You are Alice — the one who gave offence.
playerName :: String
playerName = "alice"

-- Domain knowledge, authored once. Reads see the forward-chained closure.
feudAxioms :: [Axiom]
feudAxioms =
  [ axiom [ Match "allied.X.Y" ]                     [ "allied.Y.X" ]    -- alliances are mutual
  , axiom [ Match "wronged.X.Y" ]                    [ "resents.Y.X" ]   -- the wronged resent the wrongdoer
  , axiom [ Match "resents.A.B", Match "allied.A.C" ][ "resents.C.B" ]   -- the enemy of my ally is my enemy
  ]

-- The one place everyone shares. Affordances key off /derived/ enmity.
societyP :: Practice
societyP = practice
  { practiceId   = "society"
  , practiceName = "the company keeps its grudges"
  , roles        = ["Room"]
  , actions =
      [ -- available only to someone who resents the target — and @resents@ is
        -- almost always a derived fact, not an authored one
        action "[Actor]: shun [Target]"
          [ Match "resents.Actor.Target"
          , Neq "Actor" "Target"
          , Not "shunned.Actor.Target" ]
          [ Insert "shunned.Actor.Target" ]

        -- the wrongdoer can make amends: retracting the base wrong dissolves every
        -- resentment derived from it (defeasibility as a game move)
      , action "[Actor]: make amends with [Target]"
          [ Match "wronged.Actor.Target" ]
          [ Delete "wronged.Actor.Target" ]
      ]
  }

-- Cast -------------------------------------------------------------------------

-- Someone who acts on their grudges: wants to shun anyone they (come to) resent.
grudgeBearer :: String -> Character
grudgeBearer n = (character n)
  { charWants = [ Want [ Match ("shunned." ++ n ++ ".Target") ] 5 ] }

alice :: Character
alice = (character playerName)
  { charWants = [ Want [ Match "shunned.Other.alice" ] (-10) ] }  -- dislikes being shunned

-- | The set-up sandbox: three domain rules + one act of offence, and a feud
-- assembles itself.
feudWorld :: PraxState
feudWorld =
  setAxioms feudAxioms (foldl (flip performOutcome) withPractices setup)
  where
    withPractices =
      (definePractices [ societyP ] emptyState)
        { characters = [ alice, grudgeBearer "bob", grudgeBearer "carol", grudgeBearer "dave" ] }
    setup =
      [ Insert "practice.society.here"
      , Insert "wronged.alice.bob"      -- the single authored grievance
      , Insert "allied.bob.carol"
      , Insert "allied.carol.dave"
      ]

-- | A scaled feud (for scale demos / benchmarks): @n@ grudge-bearers in an
-- alliance chain, all turned against Alice by the one original wrong — so the
-- closure derives @O(n)@ enmities and the planner has @n+1@ movers per node.
bigFeud :: Int -> PraxState
bigFeud n =
  setAxioms feudAxioms (foldl (flip performOutcome) withPractices setup)
  where
    names = [ "a" ++ show i | i <- [1 .. n] ]
    withPractices =
      (definePractices [ societyP ] emptyState)
        { characters = alice : map grudgeBearer names }
    setup =
      [ Insert "practice.society.here"
      , Insert "wronged.alice.a1" ]
      ++ [ Insert ("allied.a" ++ show i ++ ".a" ++ show (i + 1)) | i <- [1 .. n - 1] ]
