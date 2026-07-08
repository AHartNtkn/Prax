module Prax.IntrigueSpec (tests) where

import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, assertFailure)

import           Prax.Db (dbToSentences)
import           Prax.Types (PraxState, db, gaLabel)
import           Prax.Engine (possibleActions, performAction)
import           Prax.Loop (runNpcTicks)
import           Prax.Inspect (explain)
import           Prax.Worlds.Intrigue (intrigueWorld)

facts :: PraxState -> [String]
facts = dbToSentences . db

-- Perform the first action whose label contains `needle` for `actor`.
act :: PraxState -> String -> String -> IO PraxState
act st actor needle =
  case filter ((needle `isInfixOf`) . gaLabel) (possibleActions st actor) of
    (ga : _) -> pure (performAction st ga)
    []       -> assertFailure
                  ("no action matching " ++ show needle ++ " for " ++ actor
                   ++ "; had: " ++ show (map gaLabel (possibleActions st actor)))
                  >> pure st

-- The state just after Cassia has confided the plot to Marcus (turn 3), so
-- Marcus now knows and can act on it.
afterConfide :: PraxState
afterConfide = snd (runNpcTicks 2 3 intrigueWorld)

tests :: TestTree
tests = testGroup "Prax.Worlds.Intrigue (a dramatic slice)"
  [ testCase "the schemer confides, then Marcus learns the plot" $
      assertBool "Marcus believes Artus is in danger"
        ("marcus.believes.plotAgainst.artus.yes" `elem` facts afterConfide)

  , testCase "an idle player lets the plot run to the betrayal ending; the victim dies" $ do
      let (tr, st) = runNpcTicks 2 8 intrigueWorld
          fs = facts st
      assertBool "Cassia poisons Artus" ("cassia: slip poison into artus's cup" `elem` tr)
      assertBool "Artus is dead"     ("dead.artus"      `elem` fs)
      assertBool "the betrayal ending is reached" ("ending.betrayal" `elem` fs)
      -- and once dead, Artus takes no further turn
      assertBool "no action by the dead Artus after the poisoning"
        (not (any ("artus: " `isInfixOf`) (drop 6 tr)))

  , testCase "warning the patron reaches the loyalty ending, and he lives" $ do
      st <- act afterConfide "marcus" "warn artus"
      let fs = facts st
      assertBool "the loyalty ending is reached" ("ending.loyalty" `elem` fs)
      assertBool "Artus lives" ("dead.artus" `notElem` fs)
      assertBool "Artus is grateful (warmth toward Marcus)"
        (any ("artus.relationship.marcus.warmth" `isInfixOf`) fs)

  , testCase "the player can commit the murder themselves (complicity ending)" $ do
      st <- act afterConfide "marcus" "poison artus with your own hand"
      let fs = facts st
      assertBool "Artus dies by Marcus's hand" ("dead.artus"       `elem` fs)
      assertBool "the complicity ending is reached" ("ending.complicity" `elem` fs)

  , testCase "the player can romance the conspirator" $ do
      st <- act afterConfide "marcus" "warm to cassia"
      assertBool "Marcus and Cassia become lovers"
        ("bond.marcus.cassia.lovers" `elem` facts st)

  , testCase "once an ending is reached, the drama freezes (no further plot moves)" $ do
      -- reach an ending, then Cassia (who wanted Artus dead) has nothing left to do
      st <- act afterConfide "marcus" "warn artus"
      assertBool "no poisoning remains available"
        (not (any (("poison" `isInfixOf`) . gaLabel) (possibleActions st "cassia")))

  , testCase "the inspector explains why an action is (un)available" $ do
      -- before Marcus knows the plot, warning is blocked by the belief precondition
      let before = concat (explain intrigueWorld "marcus" "warn artus")
      assertBool ("blocked, reason mentions the belief: " ++ before)
        ("blocked by" `isInfixOf` before && "believes" `isInfixOf` before)
      -- once Cassia has confided, it becomes available
      let after = concat (explain afterConfide "marcus" "warn artus")
      assertBool ("now available: " ++ after) ("AVAILABLE" `isInfixOf` after)
  ]
