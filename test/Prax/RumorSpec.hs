module Prax.RumorSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Either (isLeft)
import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, possibleActions, performAction)
import           Prax.Witness (CoPresence)
import           Prax.Rumor

-- One yard; everyone in it can be told.
together :: CoPresence
together = [ Match "at.Actor!P", Match "at.Witness!P" ]

-- The tale: tess tripped. sam and rita saw it; hana and pip know nothing.
-- The world's gate: you don't gossip with someone you hold a grudge against.
tell :: Action
tell = gossip together [ Not "grudge.Actor.Hearer" ] "tripped.Klutz"
         "[Actor]: tell [Hearer] that [Klutz] tripped"

world :: PraxState
world = foldl (flip performOutcome) base setup
  where
    base = (definePractices [p] emptyState)
             { characters = map character ["sam", "rita", "hana", "tess", "pip"] }
    p = practice { practiceId = "yard", roles = ["R"], actions = [tell] }
    setup =
      [ Insert "practice.yard.here"
      , Insert "at.sam!yard", Insert "at.rita!yard", Insert "at.hana!yard"
      , Insert "at.tess!yard", Insert "at.pip!yard"
      , Insert "sam.believes.tripped.tess.seen"
      , Insert "rita.believes.tripped.tess.seen"
      ]

-- @teller@ performs their tell aimed at @hearer@.
tellTo :: String -> String -> PraxState -> PraxState
tellTo teller hearer st =
  case [ ga | ga <- possibleActions st teller
            , ("tell " ++ hearer) `isInfixOf` gaLabel ga ] of
    (ga : _) -> performAction st ga
    []       -> error ("no tell to " ++ hearer ++ " offered to " ++ teller
                       ++ "; had: " ++ show (map gaLabel (possibleActions st teller)))

-- Is @teller@ offered a tell to @hearer@?
offers :: String -> String -> PraxState -> Bool
offers teller hearer st =
  any ((("tell " ++ hearer) `isInfixOf`) . gaLabel) (possibleActions st teller)

tests :: TestTree
tests = testGroup "Prax.Rumor"
  [ testCase "telling plants hearsay with the teller as source" $ do
      let st = tellTo "sam" "hana" world
      assertBool "hana heard it from sam"
        (exists "hana.believes.tripped.tess.heard.sam" (db st))

  , testCase "the subject of the rumor is never offered as hearer" $
      assertBool "no telling tess about tess" (not (offers "sam" "tess" world))

  , testCase "a hearer who saw the event is not told (no news value)" $
      assertBool "sam not offered telling rita (an eyewitness)"
        (not (offers "sam" "rita" world))

  , testCase "retelling to the same hearer is not offered (one-shot per teller)" $ do
      let st = tellTo "sam" "hana" world
      assertBool "sam cannot retell hana" (not (offers "sam" "hana" st))

  , testCase "a second teller adds a second heard edge (corroboration)" $ do
      let st = tellTo "rita" "hana" (tellTo "sam" "hana" world)
      assertBool "heard from sam"  (exists "hana.believes.tripped.tess.heard.sam" (db st))
      assertBool "heard from rita" (exists "hana.believes.tripped.tess.heard.rita" (db st))

  , testCase "hearsay can be retold (rumor chains)" $ do
      let st = tellTo "hana" "pip" (tellTo "sam" "hana" world)
      assertBool "pip heard it from hana"
        (exists "pip.believes.tripped.tess.heard.hana" (db st))

  , testCase "no evidence, nothing to tell" $
      assertBool "hana (who knows nothing yet) offers no tells"
        (not (any (("tell" `isInfixOf`) . gaLabel) (possibleActions world "hana")))

  , testCase "the world's gate closes the channel" $ do
      let st = performOutcome (Insert "grudge.sam.hana") world
      assertBool "sam won't gossip with hana" (not (offers "sam" "hana" st))
      assertBool "but rita still will" (offers "rita" "hana" st)

  , testCase "heard is a boolean exists (no per-source bindings leak)" $
      heard "W" "tripped.tess"
        @?= Exists [ Match "W.believes.tripped.tess.heard.Src" ]

  , testCase "a pattern with no variable errors loudly" $ do
      r <- try (evaluate (length (show (gossip together [] "somethinghappened"
                 "[Actor]: mention it to [Hearer]"))))
      assertBool "gossip on a subject-less pattern is an error"
        (isLeft (r :: Either ErrorCall Int))
  ]
