module Prax.RelevanceSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool)

import           Prax.Engine (setDesires)
import           Prax.Types
import           Prax.Worlds.Village (villageWorld)
import           Prax.Relevance

tests :: TestTree
tests = testGroup "Prax.Relevance"
  [ testCase "mayUnify: variables are wildcards, prefixes are compatible" $ do
      assertBool "var vs concrete" (mayUnify "lied.Actor.H.stole.C.loaf"
                                             "lied.eve.dana.stole.carol.loaf")
      assertBool "prefix compatibility (longer insert, shorter pattern)"
        (mayUnify "Hearer.believes.took.Culprit.gem.heard.Actor"
                  "oz.believes.took.kit.gem")
      assertBool "distinct constants do not unify"
        (not (mayUnify "regards.W.carol.thief" "practice.earnBread.Owner.done.S"))

  , testCase "the village table: conscience dead, spite and pursuit live" $ do
      let tbl = improvableDesires (practiceDefs villageWorld)
                                  (axioms villageWorld)
                                  (desires villageWorld)
      -- No authored village action Deletes a lied-mark, no axiom head touches
      -- one: a conscience-only believed model can never be improved.
      assertBool "clean-conscience is not improvable"
        ("clean-conscience" `notElem` tbl)
      -- spites-carol counts DERIVED regards facts (standingUnless's head):
      -- conservatively improvable, so eve's predicted whisper stays live.
      assertBool "spites-carol is improvable" ("spites-carol" `elem` tbl)
      -- pursuit counts base done-facts the stage actions Insert.
      assertBool "pursues-earnBread is improvable"
        ("pursues-earnBread" `elem` tbl)

  , testCase "the state carries the table and setDesires rebuilds it" $ do
      assertBool "villageWorld's field matches the module computation"
        (improvables villageWorld
           == improvableDesires (practiceDefs villageWorld)
                                (axioms villageWorld)
                                (desires villageWorld))
      let st = setDesires [ d | d <- desires villageWorld
                              , desireName d == "spites-carol" ] villageWorld
      assertBool "narrowed vocabulary narrows the table"
        ("pursues-earnBread" `notElem` improvables st)
  ]
