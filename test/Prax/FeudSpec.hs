-- | The emergent-sandbox demo, end-to-end: derived enmity that no one authored,
-- a derived fact gating an affordance, and defeasibility (making amends dissolves
-- the feud) — all through the engine's normal read path ('readView').
module Prax.FeudSpec (tests) where

import           Data.List (isInfixOf, isPrefixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, assertFailure)

import           Prax.Db (dbToSentences)
import           Prax.Types
import           Prax.Engine (readView, possibleActions, performAction)
import           Prax.Loop (advance, npcAct)
import           Prax.Worlds.Feud (feudWorld)

-- Run with `idle` (the player, Alice) never acting and everyone else planner-driven.
runWithPassive :: String -> Int -> PraxState -> PraxState
runWithPassive idle = go
  where
    go 0 st = st
    go k st = let (actor, st1) = advance st
                  st2 | charName actor == idle = st1
                      | otherwise              = snd (npcAct 2 actor st1)
              in go (k - 1) st2

viewFacts :: PraxState -> [String]
viewFacts = dbToSentences . readView

canShun :: PraxState -> String -> [String]   -- targets `who` can shun right now
canShun st who =
  [ drop (length pre) lbl
  | ga <- possibleActions st who, let lbl = gaLabel ga, pre `isPrefixOf` lbl ]
  where pre = who ++ ": shun "

perform :: PraxState -> String -> String -> IO PraxState
perform st who needle =
  case filter ((needle `isInfixOf`) . gaLabel) (possibleActions st who) of
    (ga : _) -> pure (performAction st ga)
    []       -> assertFailure ("no action " ++ show needle ++ " for " ++ who) >> pure st

tests :: TestTree
tests = testGroup "Prax.Worlds.Feud (emergent sandbox)"
  [ testCase "enmity is DERIVED, not authored: Alice's wrong spreads through the network" $ do
      let vs = viewFacts feudWorld
      -- authored: only that Alice wronged Bob
      assertBool "the one authored grievance" ("wronged.alice.bob" `elem` dbToSentences (db feudWorld))
      -- derived: Bob and — though they never met Alice — Carol and Dave resent her
      assertBool "bob resents alice (wronged)"   ("resents.bob.alice"   `elem` vs)
      assertBool "carol resents alice (ally)"    ("resents.carol.alice" `elem` vs)
      assertBool "dave resents alice (ally's ally)" ("resents.dave.alice" `elem` vs)
      -- none of these are in the base — they exist only in the closure
      assertBool "not authored in the base"
        (not (any ("resents." `isInfixOf`) (dbToSentences (db feudWorld))))

  , testCase "a derived fact GATES an affordance: Carol may shun Alice, Alice may shun no one" $ do
      assertBool "carol can shun alice (via derived enmity)" ("alice" `elem` canShun feudWorld "carol")
      assertBool "alice resents no one, so shuns no one"     (null (canShun feudWorld "alice"))

  , testCase "with Alice passive, the network shuns her on its own (emergent behaviour)" $ do
      -- (an *active* Alice would rationally make amends first — see the last case)
      let st = runWithPassive "alice" 12 feudWorld
          vs = dbToSentences (db st)
      assertBool "bob shunned alice"   ("shunned.bob.alice"   `elem` vs)
      assertBool "carol shunned alice" ("shunned.carol.alice" `elem` vs)
      assertBool "dave shunned alice"  ("shunned.dave.alice"  `elem` vs)

  , testCase "DEFEASIBLE: making amends retracts the wrong and dissolves the whole feud" $ do
      amended <- perform feudWorld "alice" "make amends with bob"
      let vs = viewFacts amended
      assertBool "the wrong is gone"        ("wronged.alice.bob" `notElem` dbToSentences (db amended))
      assertBool "carol no longer resents"  ("resents.carol.alice" `notElem` vs)
      assertBool "dave no longer resents"   ("resents.dave.alice"  `notElem` vs)
      assertBool "and can no longer shun her" (null (canShun amended "carol"))
  ]
