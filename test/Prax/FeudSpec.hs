-- | The emergent-sandbox demo, end-to-end: derived enmity that no one authored,
-- a derived fact gating an affordance, and defeasibility (making amends dissolves
-- the feud) — all through the engine's normal read path ('readView').
module Prax.FeudSpec (tests) where

import           Data.List (isInfixOf, isPrefixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, assertFailure)

import           Prax.Db (dbToSentences, exists)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction, performOutcome)
import           Prax.Loop (advance, npcAct)
import           Prax.Kin (wed)
import           Prax.Worlds.Feud (feudWorld, bigFeud)

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

-- Esme, wed into the feud's kestrel house (the bride moves — authored direction).
weddedWorld :: PraxState
weddedWorld = foldl (flip performOutcome) feudWorld (wed "esme" "kestrel" "dave")

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

  , testCase "the feud scales: bigFeud turns every ally in the chain against Alice" $ do
      -- guards semi-naive closure correctness at scale (no derivation dropped)
      let n  = 20
          vs = viewFacts (bigFeud n)
      assertBool "every one of the chain's members (transitively) resents alice"
        (all (\i -> ("resents.a" ++ show i ++ ".alice") `elem` vs) [1 .. n])

  , testCase "DEFEASIBLE: making amends retracts the wrong and dissolves the whole feud" $ do
      amended <- perform feudWorld "alice" "make amends with bob"
      let vs = viewFacts amended
      assertBool "the wrong is gone"        ("wronged.alice.bob" `notElem` dbToSentences (db amended))
      assertBool "carol no longer resents"  ("resents.carol.alice" `notElem` vs)
      assertBool "dave no longer resents"   ("resents.dave.alice"  `notElem` vs)
      assertBool "and can no longer shun her" (null (canShun amended "carol"))

  , testCase "pre-wedding: esme is inert to the feud — her own house, no resentment, no kestrel ties" $ do
      assertBool "esme starts in her own house (wren)" (exists "member.esme!wren" (db feudWorld))
      let vs = viewFacts feudWorld
      assertBool "esme resents no one yet" (not (any ("resents.esme." `isPrefixOf`) vs))
      assertBool "esme is not yet allied with the kestrel house"
        (not (any (`elem` vs) ["allied.esme.bob", "allied.esme.carol", "allied.esme.dave"]))

  , testCase "the wedding: wed moves esme's membership; the derived world flips" $ do
      assertBool "esme's membership moved to kestrel" (exists "member.esme!kestrel" (db weddedWorld))
      assertBool "esme's old wren membership is gone" (not (exists "member.esme!wren" (db weddedWorld)))
      let vs = viewFacts weddedWorld
      assertBool "esme is now allied with the whole kestrel house (comrades)"
        (all (`elem` vs) ["allied.esme.bob", "allied.esme.carol", "allied.esme.dave"])
      assertBool "esme inherits her in-laws' grudge: resents.esme.alice is derived"
        ("resents.esme.alice" `elem` vs)
      assertBool "married.dave.esme is derived (marriage symmetry, kinAxioms)"
        ("married.dave.esme" `elem` vs)

  , testCase "the driven beat: after the wedding, esme (a grudgeBearer) shuns alice unprompted" $ do
      let driven = runWithPassive "alice" 12 weddedWorld
          vs = dbToSentences (db driven)
      assertBool "esme shunned alice" ("shunned.esme.alice" `elem` vs)
  ]
