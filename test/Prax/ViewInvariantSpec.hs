module Prax.ViewInvariantSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (dbToLabeledSentences, insert)
import           Prax.Derive (closure)
import           Prax.Types
import           Prax.Engine (performOutcome)
import           Prax.Loop (advance, npcAct)
import           Prax.Worlds.Village (villageWorld, playerName)
import           Prax.Worlds.Bar (barWorld)
import           Prax.Worlds.Intrigue (intrigueWorld)

-- The round's core invariant: the cached view IS the closure of the base
-- under the axioms — label-faithfully, whatever construction path built it.
viewConsistent :: PraxState -> Bool
viewConsistent st =
  dbToLabeledSentences (readView st) == dbToLabeledSentences recomputed
  where
    recomputed = case closure (axioms st) (db st) of
      Right c -> c
      Left _  -> insert "contradiction" (db st)

-- Drive n turns through the REAL loop; report the first turn (1-based) after
-- which the invariant fails, if any. The Maybe Int in the assertion message
-- names the offending turn directly.
firstDrift :: Maybe String -> Int -> PraxState -> Maybe Int
firstDrift idle n = go 1
  where
    go k st
      | k > n = Nothing
      | otherwise =
          let (actor, st1) = advance st
              st2 | Just (charName actor) == idle = st1
                  | otherwise                     = snd (npcAct 2 actor st1)
          in if viewConsistent st2 then go (k + 1) st2 else Just k

tests :: TestTree
tests = testGroup "Prax.ViewInvariant (readView == recomputed closure)"
  [ testCase "the checker catches a deliberately doctored stale view" $ do
      -- a raw record update that bypasses the Engine helpers — exactly the
      -- construction the src/ grep-gate bans — leaves the cached view behind
      let st       = performOutcome (Insert "probe.fact") villageWorld
          doctored = st { readView = readView villageWorld }
      assertBool "a helper-built state passes" (viewConsistent st)
      assertBool "the stale view is caught"    (not (viewConsistent doctored))

  , testCase "village: 3 rounds of free play, invariant after every turn" $
      firstDrift (Just playerName) 21 villageWorld @?= Nothing

  , testCase "bar: 12 turns, invariant after every turn" $
      firstDrift Nothing 12 barWorld @?= Nothing

  , testCase "intrigue: 12 turns, invariant after every turn" $
      firstDrift Nothing 12 intrigueWorld @?= Nothing
  ]
