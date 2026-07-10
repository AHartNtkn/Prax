module Prax.ReputeSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Either (isLeft)
import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, possibleActions, readView)
import           Prax.Derive (Axiom (..))
import           Prax.Repute

-- The tale: kai is believed (variously) to have kicked the dog. Standing is
-- derived from the evidence, defeated by forgiveness, and notorious at two.
world :: PraxState
world = (foldl (flip performOutcome) base setup)
          { axioms = [ standingUnless "kicked.Brute.dog" "forgiven.Brute" "brute"
                     , notoriety "brute" 2 ] }
  where
    base = (definePractices [p] emptyState)
             { characters = map character ["ana", "ben", "kai"] }
    -- an affordance gated on the DERIVED standing: preconditions read the view
    p = practice
      { practiceId = "yard", roles = ["R"]
      , actions = [ action "[Actor]: scowl at [B]"
                      [ regardedAs "Actor" "B" "brute", Not "scowled.Actor.B" ]
                      [ Insert "scowled.Actor.B" ] ] }
    setup =
      [ Insert "practice.yard.here"
      , Insert "ana.believes.kicked.kai.dog.seen"
      , Insert "ben.believes.kicked.kai.dog.heard.ana" ]

tests :: TestTree
tests = testGroup "Prax.Repute"
  [ testCase "evidence derives per-observer standing (seen and heard alike)" $ do
      let v = readView world
      assertBool "ana (eyewitness) regards kai a brute" (exists "regards.ana.kai.brute" v)
      assertBool "ben (hearsay) regards kai a brute"    (exists "regards.ben.kai.brute" v)
      assertBool "kai holds no self-regard"             (not (exists "regards.kai.kai.brute" v))

  , testCase "notoriety holds at the threshold, not below" $ do
      assertBool "two regarders: notorious" (exists "notorious.kai.brute" (readView world))
      let one = performOutcome (Delete "ben.believes.kicked.kai.dog") world
      assertBool "one regarder: not notorious"
        (not (exists "notorious.kai.brute" (readView one)))
      assertBool "and ben's regard dissolved with his evidence"
        (not (exists "regards.ben.kai.brute" (readView one)))

  , testCase "the defeater dissolves standing while memory persists" $ do
      let f = performOutcome (Insert "forgiven.kai") world
          v = readView f
      assertBool "no regard survives forgiveness"   (not (exists "regards.ana.kai.brute" v))
      assertBool "no notoriety survives"            (not (exists "notorious.kai.brute" v))
      assertBool "ana still remembers what she saw" (exists "ana.believes.kicked.kai.dog.seen" (db f))

  , testCase "a derived-standing-gated affordance appears and disappears" $ do
      assertBool "ana may scowl at kai"
        (any (("scowl at kai" `isInfixOf`) . gaLabel) (possibleActions world "ana"))
      let f = performOutcome (Insert "forgiven.kai") world
      assertBool "forgiven: the scowl is gone"
        (not (any (("scowl at kai" `isInfixOf`) . gaLabel) (possibleActions f "ana")))

  , testCase "regardedAs is the standing condition" $
      regardedAs "W" "kai" "brute" @?= Match "regards.W.kai.brute"

  , testCase "the deed pattern's FIRST variable is the subject" $
      axiomThen (standing "sold.Seller.Buyer.secret" "snitch")
        @?= [ "regards.Regarder.Seller.snitch" ]

  , testCase "a deed pattern with no variable errors loudly" $ do
      r <- try (evaluate (length (show (standing "somethinghappened" "x"))))
      assertBool "standing on a subject-less pattern is an error"
        (isLeft (r :: Either ErrorCall Int))
  ]
