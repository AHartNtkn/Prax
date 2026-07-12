module Prax.MindsSpec (tests) where

import           Data.List (isInfixOf)
import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists)
import           Prax.Query (Condition (..), CookedCondition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, setAxioms, setDesires, possibleActions, performAction, setCharacters)
import           Prax.Minds
import           Prax.Planner (predictMove)
import           Prax.Rumor (gossip)

-- The tale: a vocabulary of two desires; ida professes her sweet tooth,
-- norm-respect is conventional, and rex's grudge is neither.
vocab :: [Desire]
vocab =
  [ Desire "sweet-tooth" (Want [ Match "holding.Owner.cake" ] 5)
  , Desire "grudge-rex"  (Want [ Match "shamed.rex" ] 7)
  ]

world :: PraxState
world = setDesires vocab
          (setAxioms [ professed, conventional ] (foldl (flip performOutcome) base setup))
  where
    base = setCharacters [ character "ida"
                          , (character "rex") { charDesires = ["grudge-rex"] } ]
             (definePractices [] emptyState)
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

  , testCase "setCharacters retables cookedWants; retable tracks cookedDesires" $ do
      -- cookedWants: keyed by charName, each want's conditions precooked, in
      -- charWants' own order — read back from a fresh setCharacters call.
      let rex = (character "rex") { charWants   = [ Want [ Match "x" ] 1, Want [ Match "y.Z" ] 2 ]
                                   , charDesires = ["grudge-rex"] }
          st  = setCharacters [ character "ida", rex ] world
      Map.keys (cookedWants st) @?= ["ida", "rex"]
      Map.lookup "ida" (cookedWants st) @?= Just []
      Map.lookup "rex" (cookedWants st)
        @?= Just [ [ CMatch ["x"] ], [ CMatch ["y", "Z"] ] ]
      -- cookedDesires: keyed by desireName, the vocabulary's Owner-template
      -- conditions precooked once, independent of which characters hold them.
      Map.keys (cookedDesires st) @?= ["grudge-rex", "sweet-tooth"]
      Map.lookup "grudge-rex" (cookedDesires st) @?= Just [ CMatch ["shamed", "rex"] ]
      Map.lookup "sweet-tooth" (cookedDesires st)
        @?= Just [ CMatch ["holding", "Owner", "cake"] ]

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

  , testCase "gossip about a motive flips a third party's prediction" $ do
      -- rex's grudge (vocab) drives him to seek petty revenge once someone
      -- believes he holds it. ida already knows (an eyewitness); she tells
      -- nia, and nia's arrival at a believed model of rex flips her
      -- prediction of him from unreadable to a move.
      let together = [ Match "at.Actor!P", Match "at.Witness!P" ]
          tellGrudge = gossip together [] "desires.Culprit.grudge-rex"
                         "[Actor]: mention [Culprit]'s grudge to [Hearer]"
          revenge = action "[Actor]: seek petty revenge" [] [ Insert "shamed.Actor" ]
          townP = practice
            { practiceId = "town", roles = ["Place"]
            , actions = [ tellGrudge, revenge ] }
          rex' = (character "rex") { charDesires = ["grudge-rex"] }
          st0 = foldl (flip performOutcome)
                  (setDesires vocab
                     (setCharacters [ character "ida", rex', character "nia" ]
                        (definePractices [townP] emptyState)))
                  [ Insert "practice.town.village"
                  , Insert "at.ida!village", Insert "at.nia!village"
                  , Insert "ida.believes.desires.rex.grudge-rex.seen" ]
          told = case [ ga | ga <- possibleActions st0 "ida"
                            , "mention rex's grudge to nia" `isInfixOf` gaLabel ga ] of
                   (ga : _) -> performAction st0 ga
                   []       -> error "no gossip action offered to ida"
      predictMove st0 (character "nia") rex' @?= Nothing
      fmap gaLabel (predictMove told (character "nia") rex')
        @?= Just "rex: seek petty revenge"
  ]
