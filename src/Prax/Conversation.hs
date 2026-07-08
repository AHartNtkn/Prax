-- | Conversation: speakers, topics, and quips (Versu paper §X; Emily Short's
-- blog on Versu conversation).
--
-- A conversation is a two-party practice with a /selected speaker/ (turn-taking)
-- and a single current /topic/. A **quip** is an ordinary action that says a line
-- and applies effects — on the core model and beliefs — exactly the machinery
-- from "Prax.Core" and "Prax.Beliefs"; "a response is just a normal type of
-- action … the same utility-planner." Characters stay on topic (a quip is only
-- available on its topic) until someone deliberately changes the subject.
--
-- This is a reusable library on the existing engine (no new machinery): the
-- conversation is @practice.converse.\<A\>.\<B\>@ with @speaker!@, @listener!@,
-- @topic!@ facts. A world supplies the concrete quips via the 'quip' /
-- 'changeSubject' / 'endConversation' builders inside a practice with
-- roles @["A","B"]@.
module Prax.Conversation
  ( talkPath
  , talkingWith
  , beginConversation
  , quip
  , changeSubject
  , endConversation
  ) where

import           Prax.Query (Condition (..))
import           Prax.Types (Outcome (..), Action, action)

-- | Path of a conversation instance between @x@ and @y@ (in this order).
talkPath :: String -> String -> String
talkPath x y = "practice.converse." ++ x ++ "." ++ y

-- | Condition: @x@ and @y@ are conversing (this ordering).
talkingWith :: String -> String -> Condition
talkingWith x y = Match (talkPath x y)

-- | Outcomes that open a conversation: @opener@ becomes the first speaker,
-- @other@ the listener, starting on @topic@. Records @topic@ as visited and
-- marks the pair as having chatted (so a world can prevent re-opening).
beginConversation :: String -> String -> String -> [Outcome]
beginConversation opener other topic =
  [ Insert base
  , Insert (base ++ ".speaker!" ++ opener)
  , Insert (base ++ ".listener!" ++ other)
  , Insert (base ++ ".topic!" ++ topic)
  , Insert (base ++ ".visited." ++ topic)
  , Insert (opener ++ ".chattedWith." ++ other)
  , Insert (other ++ ".chattedWith." ++ opener)
  ]
  where base = talkPath opener other

-- Within-practice actions reference the instance by its role variables A and B.
convP :: String
convP = "practice.converse.A.B"

-- The current speaker is the actor; bind the listener as Partner.
speakerConds :: [Condition]
speakerConds =
  [ Match (convP ++ ".speaker!Actor")
  , Match (convP ++ ".listener!Partner")
  ]

-- Hand the floor to the other party.
passTurn :: [Outcome]
passTurn =
  [ Insert (convP ++ ".speaker!Partner")
  , Insert (convP ++ ".listener!Actor")
  ]

-- | A quip: a line the current speaker can say once, on the given @topic@, with
-- @effects@ (and any extra @conds@). Saying it passes the turn.
--
-- @key@ is a short unique tag used to record that this quip has been said (so it
-- is one-shot per speaker); @label@ is the displayed line template.
quip :: String -> String -> String -> [Condition] -> [Outcome] -> Action
quip key label topic conds effects =
  action label
    (speakerConds
      ++ [ Match (convP ++ ".topic!" ++ topic)
         , Not (convP ++ ".said." ++ key ++ ".Actor") ]
      ++ conds)
    ([ Insert (convP ++ ".said." ++ key ++ ".Actor") ] ++ effects ++ passTurn)

-- | Steer the conversation onto @newTopic@ (only if not already there and not
-- already covered). Passes the turn.
changeSubject :: String -> String -> Action
changeSubject label newTopic =
  action label
    (speakerConds
      ++ [ Not (convP ++ ".topic!" ++ newTopic)
         , Not (convP ++ ".visited." ++ newTopic) ])
    ([ Insert (convP ++ ".topic!" ++ newTopic)
     , Insert (convP ++ ".visited." ++ newTopic) ] ++ passTurn)

-- | End the conversation (removes the instance).
endConversation :: String -> Action
endConversation label =
  action label speakerConds [ Delete convP ]
