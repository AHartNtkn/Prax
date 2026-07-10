module Prax.VillageSpec (tests) where

import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool)

import           Prax.Db (exists, dbToSentences)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction)
import           Prax.Worlds.Village

-- Perform the named actor's action whose label mentions @needle@.
doAct :: String -> String -> PraxState -> PraxState
doAct who needle st =
  case [ ga | ga <- possibleActions st who, needle `isInfixOf` gaLabel ga ] of
    (ga : _) -> performAction st ga
    []       -> error ("no action for " ++ who ++ " matching " ++ show needle
                       ++ "; had: " ++ show (map gaLabel (possibleActions st who)))

tests :: TestTree
tests = testGroup "Prax.Worlds.Village"
  [ testCase "the theft is witnessed by the square, not the mill" $ do
      let st = doAct "bob" "steal the loaf" villageWorld
      assertBool "carol (in the square) saw it"
        (exists "carol.believes.stole.bob.loaf!seen" (db st))
      assertBool "you (in the square) saw it"
        (exists "you.believes.stole.bob.loaf!seen" (db st))
      assertBool "dana (at the mill) holds no such belief"
        (not (exists "dana.believes.stole.bob.loaf!seen" (db st)))
      assertBool "bob is not his own witness"
        (not (exists "bob.believes.stole.bob.loaf!seen" (db st)))

  , testCase "movement is not news (undeclared actions deposit nothing)" $ do
      let st = doAct "bob" "Go to mill" villageWorld
      assertBool "no one 'believes' bob walked"
        (not (any (\w -> exists (w ++ ".believes.went.bob!seen") (db st))
                  ["you", "carol", "dana"]))

  , testCase "only a witness can confront the thief" $ do
      let st = doAct "bob" "steal the loaf" villageWorld
      assertBool "carol can confront"
        (any (("confront bob" `isInfixOf`) . gaLabel) (possibleActions st "carol"))
      assertBool "dana cannot"
        (not (any (("confront bob" `isInfixOf`) . gaLabel) (possibleActions st "dana")))

  , testCase "confronting cools the witness toward the thief, once" $ do
      let st  = doAct "carol" "confront bob" (doAct "bob" "steal the loaf" villageWorld)
      assertBool "trust dropped"
        ("carol.relationship.bob.trust.score.-10" `elem` dbToSentences (db st))
      assertBool "confront is one-shot"
        (not (any (("confront bob" `isInfixOf`) . gaLabel) (possibleActions st "carol")))
  ]
