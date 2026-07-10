module Prax.MindsSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, readView)
import           Prax.Minds

-- The tale: a vocabulary of two desires; ida professes her sweet tooth,
-- norm-respect is conventional, and rex's grudge is neither.
vocab :: [Desire]
vocab =
  [ Desire "sweet-tooth" (Want [ Match "holding.Owner.cake" ] 5)
  , Desire "grudge-rex"  (Want [ Match "shamed.rex" ] 7)
  ]

world :: PraxState
world = (foldl (flip performOutcome) base setup)
          { axioms  = [ professed, conventional ]
          , desires = vocab }
  where
    base = (definePractices [] emptyState)
             { characters = [ character "ida"
                            , (character "rex") { charDesires = ["grudge-rex"] } ] }
    setup =
      [ Insert "character.ida", Insert "character.rex"
      , Insert "professes.ida.sweet-tooth" ]

tests :: TestTree
tests = testGroup "Prax.Minds"
  [ testCase "wantFor grounds the Owner variable" $
      wantFor "ida" (Desire "sweet-tooth" (Want [ Match "holding.Owner.cake" ] 5))
        @?= Want [ Match "holding.ida.cake" ] 5

  , testCase "selfWants = unnamed wants + own named desires, instantiated" $ do
      let rex = (character "rex") { charWants   = [ Want [ Match "x" ] 1 ]
                                  , charDesires = ["grudge-rex"] }
      selfWants world rex
        @?= [ Want [ Match "x" ] 1, Want [ Match "shamed.rex" ] 7 ]

  , testCase "a profession derives presumed motive-beliefs across the cast" $ do
      let v = readView world
      assertBool "rex presumes ida's sweet tooth"
        (exists "rex.believes.desires.ida.sweet-tooth.presumed" v)
      assertBool "nothing derives rex's unprofessed grudge"
        (not (exists "ida.believes.desires.rex.grudge-rex.presumed" v))

  , testCase "the profession is defeasible" $ do
      let w' = performOutcome (Delete "professes.ida.sweet-tooth") world
      assertBool "presumption dissolved"
        (not (exists "rex.believes.desires.ida.sweet-tooth.presumed" (readView w')))

  , testCase "a conventional desire is presumed of everyone — even non-holders" $ do
      let w' = performOutcome (Insert "conventional.sweet-tooth") world
          v  = readView w'
      assertBool "ida presumes rex's sweet tooth (he does not have one)"
        (exists "ida.believes.desires.rex.sweet-tooth.presumed" v)

  , testCase "believedWants reads any provenance, and only believed desires" $ do
      believedWants world (character "ida") (character "rex") @?= []
      let w' = performOutcome
                 (Insert "ida.believes.desires.rex.grudge-rex.heard.sam") world
      believedWants w' (character "ida") (character "rex")
        @?= [ Want [ Match "shamed.rex" ] 7 ]
      -- and presumption counts too:
      believedWants world (character "rex") (character "ida")
        @?= [ Want [ Match "holding.ida.cake" ] 5 ]
  ]
