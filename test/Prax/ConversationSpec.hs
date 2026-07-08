module Prax.ConversationSpec (tests) where

import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, assertFailure)

import           Prax.Db (dbToSentences)
import           Prax.Types
import           Prax.Engine (definePractice, performOutcome, possibleActions, performAction)
import           Prax.Conversation

-- A minimal conversation practice for testing the scaffolding.
talkP :: Practice
talkP = practice
  { practiceId = "converse"
  , practiceName = "[A] and [B] are talking"
  , roles = ["A", "B"]
  , actions =
      [ quip "hi" "[Actor]: say hi to [Partner]" "greetings" []
          [ Insert "Actor.saidHiTo.Partner" ]
      , quip "weather" "[Actor]: remark on the weather to [Partner]" "weather" []
          [ Insert "Actor.talkedWeatherWith.Partner" ]
      , changeSubject "[Actor]: change the subject to the weather" "weather"
      , endConversation "[Actor]: wrap up the chat"
      ]
  }

base :: PraxState
base = definePractice talkP emptyState

start :: PraxState
start = foldl (flip performOutcome) base (beginConversation "ada" "bex" "greetings")

opts :: PraxState -> String -> [String]
opts st a = map gaLabel (possibleActions st a)

perform :: PraxState -> String -> String -> IO PraxState
perform st actor needle =
  case filter ((needle `isInfixOf`) . gaLabel) (possibleActions st actor) of
    (ga : _) -> pure (performAction st ga)
    []       -> assertFailure ("no " ++ show needle ++ " for " ++ actor
                               ++ "; had: " ++ show (opts st actor)) >> pure st

tests :: TestTree
tests = testGroup "Prax.Conversation"
  [ testCase "beginConversation sets speaker, listener and topic" $ do
      let fs = dbToSentences (db start)
      assertBool "speaker is ada" ("practice.converse.ada.bex.speaker.ada" `elem` fs)
      assertBool "listener is bex" ("practice.converse.ada.bex.listener.bex" `elem` fs)
      assertBool "topic is greetings" ("practice.converse.ada.bex.topic.greetings" `elem` fs)

  , testCase "only the current speaker may quip, and only on the current topic" $ do
      assertBool "speaker can say hi" (any ("say hi to bex" `isInfixOf`) (opts start "ada"))
      assertBool "listener cannot quip"
        (not (any (`isInfixOf'` opts start "bex") ["say hi", "remark on the weather"]))
      -- the weather quip is off-topic (topic is greetings), so not offered
      assertBool "off-topic quip withheld"
        (not (any ("remark on the weather" `isInfixOf`) (opts start "ada")))

  , testCase "a quip applies its effect, is one-shot, and passes the turn" $ do
      st <- perform start "ada" "say hi to bex"
      let fs = dbToSentences (db st)
      assertBool "effect applied" ("ada.saidHiTo.bex" `elem` fs)
      assertBool "turn passed to bex" ("practice.converse.ada.bex.speaker.bex" `elem` fs)
      -- ada is no longer speaker, so cannot immediately say hi again
      assertBool "ada cannot quip out of turn"
        (not (any ("say hi" `isInfixOf`) (opts st "ada")))

  , testCase "changing the subject switches the active topic" $ do
      -- ada says hi; turn passes to bex; bex steers to the weather.
      st1 <- perform start "ada" "say hi to bex"
      st2 <- perform st1 "bex" "change the subject to the weather"
      let fs = dbToSentences (db st2)
      assertBool "topic is now weather" ("practice.converse.ada.bex.topic.weather" `elem` fs)
      -- now the speaker (ada again) can make the weather quip
      assertBool "on-topic weather quip offered"
        (any ("remark on the weather" `isInfixOf`) (opts st2 "ada"))

  , testCase "ending the conversation removes the instance" $ do
      st <- perform start "ada" "wrap up the chat"
      assertBool "conversation gone"
        ("practice.converse.ada.bex" `notElem` dbToSentences (db st))
  ]

-- helper: does any needle appear in the list of strings?
isInfixOf' :: String -> [String] -> Bool
isInfixOf' needle = any (needle `isInfixOf`)
