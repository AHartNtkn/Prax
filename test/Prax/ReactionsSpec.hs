module Prax.ReactionsSpec (tests) where

import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, assertFailure, (@?=))

import           Prax.Db (dbToSentences)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, possibleActions, performAction, setCharacters)
import           Prax.Planner (pickAction)
import           Prax.Core (coreLib)
import           Prax.Reactions

-- Perform the first action whose label contains `needle`.
perform :: PraxState -> String -> String -> IO PraxState
perform st actor needle =
  case filter ((needle `isInfixOf`) . gaLabel) (possibleActions st actor) of
    (ga : _) -> pure (performAction st ga)
    []       -> assertFailure
                  ("no action matching " ++ show needle ++ " for " ++ actor
                   ++ "; had: " ++ show (map gaLabel (possibleActions st actor)))
                  >> pure st

labels :: PraxState -> String -> [String]
labels st actor = map gaLabel (possibleActions st actor)

facts :: PraxState -> [String]
facts = dbToSentences . db

base :: PraxState
base = definePractices [coreLib, disapprovalP] emptyState

-- A tiny reaction whose response spawns a further (disapproval) reaction,
-- to exercise chaining generically.
chainerP :: Practice
chainerP = practice
  { practiceId = "chainer"
  , practiceName = "[A] provoked [B]"
  , roles = ["A", "B"]
  , actions =
      [ action "[Actor]: React to [A]"
          [ Eq "Actor" "B" ]
          [ spawnReaction "disapproval" ["A", "B"]
          , endReaction "chainer" ["A", "B"] ]
      ]
  }

tests :: TestTree
tests = testGroup "Prax.Reactions"
  [ testGroup "path helpers"
    [ testCase "reactionPath builds practice.<id>.<parts>" $
        reactionPath "settleUp" ["bex", "ada"] @?= "practice.settleUp.bex.ada"
    , testCase "violationOf builds the expected match" $
        violationOf "bex" "tipping" @?= Match "violated.bex.tipping"
    ]

  , testGroup "disapproval reaction"
    [ testCase "spawning offers the response only to the onlooker" $ do
        -- ada (onlooker) saw bex (offender) break a norm.
        let st = performOutcome (spawnReaction "disapproval" ["bex", "ada"]) base
        assertBool "onlooker can disapprove"
          (any ("Disapprove of bex" `isInfixOf`) (labels st "ada"))
        assertBool "offender cannot"
          (not (any ("Disapprove" `isInfixOf`) (labels st "bex")))

    , testCase "disapproving cools the relationship and consumes the reaction" $ do
        let st = performOutcome (spawnReaction "disapproval" ["bex", "ada"]) base
        st' <- perform st "ada" "Disapprove of bex"
        let fs = facts st'
        assertBool "ada is annoyed at bex" ("ada.feels.annoyed.toward.bex" `elem` fs)
        assertBool "warmth cooled" ("ada.relationship.bex.warmth.score.-20" `elem` fs)
        -- the instance is consumed: the reaction offers ada nothing further
        assertBool "reaction instance gone" ("practice.disapproval.bex.ada" `notElem` fs)
        assertBool "no more disapproval option"
          (not (any ("Disapprove" `isInfixOf`) (labels st' "ada")))

    , testCase "forgiving also consumes the reaction (no cooling)" $ do
        let st = performOutcome (spawnReaction "disapproval" ["bex", "ada"]) base
        st' <- perform st "ada" "Let bex's lapse slide"
        let fs = facts st'
        assertBool "reaction instance gone" ("practice.disapproval.bex.ada" `notElem` fs)
        assertBool "no warmth penalty"
          (not (any ("ada.relationship.bex.warmth.score.-" `isInfixOf`) fs))
    ]

  , testGroup "norm violations"
    [ testCase "markViolation records the fact" $ do
        let st = performOutcome (markViolation "bex" "tipping") base
        assertBool "violation recorded" ("violated.bex.tipping" `elem` facts st)

    , testCase "an agent avoids an action that violates a norm it wants to respect" $ do
        -- A conduct practice offering a compliant and a violating option, and an
        -- agent with a strong negative want on its own violation.
        let conductP = practice
              { practiceId = "conduct", practiceName = "conduct", roles = ["X"]
              , actions =
                  [ action "[Actor]: Behave" [] []
                  , action "[Actor]: Misbehave" [] [ markViolation "Actor" "tipping" ] ] }
            bex = (character "bex")
              { charWants = [ Want [ violationOf "bex" "tipping" ] (-50) ] }
            st0 = setCharacters [bex] (definePractices [coreLib, conductP] emptyState)
            st  = performOutcome (spawnReaction "conduct" ["bex"]) st0
        -- Both options are on the table…
        assertBool "can behave"    (any ("Behave"    `isInfixOf`) (labels st "bex"))
        assertBool "can misbehave" (any ("Misbehave" `isInfixOf`) (labels st "bex"))
        -- …but the planner picks the compliant one (the violation future scores -50).
        fmap gaLabel (pickAction 1 st bex) @?= Just "bex: Behave"
    ]

  , testGroup "chaining"
    [ testCase "a response can spawn a further reaction" $ do
        let base' = definePractices [coreLib, disapprovalP, chainerP] emptyState
            st = performOutcome (spawnReaction "chainer" ["bex", "ada"]) base'
        st' <- perform st "ada" "React to bex"
        let fs = facts st'
        assertBool "original reaction consumed" ("practice.chainer.bex.ada" `notElem` fs)
        assertBool "follow-up disapproval spawned"
          ("practice.disapproval.bex.ada" `elem` fs)
    ]
  ]
