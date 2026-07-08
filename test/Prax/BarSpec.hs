module Prax.BarSpec (tests) where

import           Control.Monad (foldM)
import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, assertFailure)

import           Prax.Db (dbToSentences)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction, performOutcome)
import           Prax.Planner (pickAction)
import           Prax.Core (adjustScore, setMood, warmth, annoyed)
import           Prax.Beliefs (believe)
import           Prax.Conversation (beginConversation)
import           Prax.Worlds.Bar (barWorld)

-- Perform the first action whose label contains `needle` for `actor`.
act :: PraxState -> (String, String) -> IO PraxState
act st (actor, needle) =
  case filter ((needle `isInfixOf`) . gaLabel) (possibleActions st actor) of
    (ga : _) -> pure (performAction st ga)
    []       -> do
      _ <- assertFailure
             ("no action matching " ++ show needle ++ " for " ++ actor
              ++ "; available: " ++ show (map gaLabel (possibleActions st actor)))
      pure st

runSteps :: PraxState -> [(String, String)] -> IO PraxState
runSteps = foldM act

adaLabels :: PraxState -> [String]
adaLabels st = map gaLabel (possibleActions st "ada")

tests :: TestTree
tests = testGroup "Prax.Worlds.Bar (feature integration)"
  [ testCase "drinking two beers makes you tipsy (init/Call/Calc/Cmp)" $ do
      -- One beer: counter goes 0 -> 1, not yet tipsy.
      afterOne <- runSteps barWorld
        [ ("you", "Go to bar"), ("you", "Order beer")
        , ("ada", "Fulfill you"), ("you", "Drink the beer") ]
      let facts1 = dbToSentences (db afterOne)
      assertBool "counter at 1" ("practice.patron.you.drinks.1" `elem` facts1)
      assertBool "not tipsy yet" ("person.you.tipsy" `notElem` facts1)

      -- Second beer: counter 1 -> 2, crosses the threshold -> tipsy.
      afterTwo <- runSteps afterOne
        [ ("you", "Order beer"), ("ada", "Fulfill you"), ("you", "Drink the beer") ]
      let facts2 = dbToSentences (db afterTwo)
      assertBool "counter at 2" ("practice.patron.you.drinks.2" `elem` facts2)
      assertBool "now tipsy" ("person.you.tipsy" `elem` facts2)

  , testCase "the bell requires two customers (Subquery/Count/Cmp)" $ do
      -- One customer (bex ordered): no bell.
      oneCust <- runSteps barWorld [ ("bex", "Go to bar"), ("bex", "Order beer") ]
      assertBool "no bell with one customer"
        (not (any ("Ring the bell" `isInfixOf`) (adaLabels oneCust)))

      -- Two customers (you also ordered): the bell becomes available.
      twoCust <- runSteps oneCust [ ("you", "Go to bar"), ("you", "Order cider") ]
      assertBool "bell available with two customers"
        (any ("Ring the bell" `isInfixOf`) (adaLabels twoCust))

  , testCase "the buy-a-drink affordance is gated on relationship warmth" $ do
      -- Co-locate bex with ada at the bar.
      let atBar = performOutcome (Insert "practice.world.world.at.bex!bar") barWorld
          bexCanBuy s = any ("Buy ada a drink" `isInfixOf`) (map gaLabel (possibleActions s "bex"))
      -- Below the threshold: the gift is not offered.
      let cool = performOutcome (adjustScore "bex" "ada" warmth 10 "acquaintance") atBar
      assertBool "no buy option when only mildly warm" (not (bexCanBuy cool))
      -- Past the threshold: the gift appears — a relationship creating a new goal.
      let warm = performOutcome (adjustScore "bex" "ada" warmth 20 "fondness") atBar
      assertBool "buy option appears once warm enough" (bexCanBuy warm)

  , testCase "an annoyed mood withholds the friendly buy action" $ do
      let atBar = performOutcome (Insert "practice.world.world.at.bex!bar") barWorld
          bexCanBuy s = any ("Buy ada a drink" `isInfixOf`) (map gaLabel (possibleActions s "bex"))
      -- Warm enough to buy…
      let warm = performOutcome (adjustScore "bex" "ada" warmth 20 "fondness") atBar
      assertBool "warm bex can buy" (bexCanBuy warm)
      -- …but once annoyed at ada, bex withholds the gesture.
      let sulky = performOutcome (setMood "bex" annoyed "ada" "wasRude") warm
      assertBool "annoyed bex will not buy" (not (bexCanBuy sulky))

  , testCase "greeting spawns a response reaction the greeted party can take" $ do
      -- you go to the bar and greet ada; that spawns a respondGreet reaction.
      afterGreet <- runSteps barWorld [ ("you", "Go to bar"), ("you", "Greet ada") ]
      assertBool "reaction spawned"
        ("practice.respondGreet.you.ada" `elem` dbToSentences (db afterGreet))
      assertBool "ada can greet back (a response that only exists now)"
        (any ("Greet you back" `isInfixOf`) (map gaLabel (possibleActions afterGreet "ada")))
      -- ada greets back: mutual warmth, and the reaction is consumed.
      afterBack <- runSteps afterGreet [ ("ada", "Greet you back") ]
      let fs = dbToSentences (db afterBack)
      assertBool "ada greeted you back" ("practice.greet.world.greeted.ada.you" `elem` fs)
      assertBool "reaction consumed" ("practice.respondGreet.you.ada" `notElem` fs)

  , testCase "being served spawns a tip obligation; tipping respects the norm" $ do
      served <- runSteps barWorld
        [ ("bex", "Go to bar"), ("bex", "Order beer"), ("ada", "Fulfill bex") ]
      assertBool "settle-up obligation spawned"
        ("practice.settleUp.bex.ada" `elem` dbToSentences (db served))
      assertBool "a first-class tip obligation (a real □) arose on serve"
        ("obliged.bex.bex.tipped.ada" `elem` dbToSentences (db served))
      tipped <- runSteps served [ ("bex", "Tip ada") ]
      let fs = dbToSentences (db tipped)
      assertBool "bex tipped ada" ("bex.tipped.ada" `elem` fs)
      assertBool "no violation" ("violated.bex.stiffedTheBartender" `notElem` fs)
      assertBool "obligation cleared" ("practice.settleUp.bex.ada" `notElem` fs)
      assertBool "the □ obligation is discharged once tipped"
        ("obliged.bex.bex.tipped.ada" `notElem` fs)

  , testCase "leaving the tab is a violation that draws the bartender's disapproval" $ do
      served <- runSteps barWorld
        [ ("bex", "Go to bar"), ("bex", "Order beer"), ("ada", "Fulfill bex") ]
      stiffed <- runSteps served [ ("bex", "Leave ada") ]
      let fs = dbToSentences (db stiffed)
      assertBool "violation marked" ("violated.bex.stiffedTheBartender" `elem` fs)
      assertBool "a reparative □□ obligation arises after the breach (contrary-to-duty)"
        ("obliged.bex.obliged.bex.make.amends.with.ada" `elem` fs)
      assertBool "disapproval reaction spawned for ada"
        ("practice.disapproval.bex.ada" `elem` fs)
      -- ada disapproves: her warmth toward bex drops.
      judged <- runSteps stiffed [ ("ada", "Disapprove of bex") ]
      assertBool "ada cooled toward bex"
        ("ada.relationship.bex.warmth.score.-20" `elem` dbToSentences (db judged))

  , testCase "a believed grudge suppresses friendliness (a false belief drives behaviour)" $ do
      let atBar = performOutcome (Insert "practice.world.world.at.bex!bar") barWorld
          warm  = performOutcome (adjustScore "bex" "ada" warmth 20 "fond") atBar
          canBuy s   = any ("Buy ada a drink" `isInfixOf`) (map gaLabel (possibleActions s "bex"))
          canGreet s = any ("Greet ada"       `isInfixOf`) (map gaLabel (possibleActions s "bex"))
      assertBool "warm bex would buy ada a drink" (canBuy warm)
      assertBool "warm bex would greet ada"       (canGreet warm)
      -- bex comes to believe ada resents them (even though she's actually warm).
      let wary = performOutcome (believe "bex" "resentedBy.ada" "yes") warm
      assertBool "the belief blocks the gift"  (not (canBuy wary))
      assertBool "the belief blocks greeting"  (not (canGreet wary))

  , testCase "a grudge lets you plant a (possibly-false) rumour" $ do
      let s0 = foldl (flip performOutcome) barWorld
                 [ Insert "practice.world.world.at.ada!entrance"  -- ada steps out
                 , Insert "practice.world.world.at.you!bar"
                 , Insert "practice.world.world.at.bex!bar"
                 , setMood "you" annoyed "ada" "wasRude" ]        -- you're cross with ada
      assertBool "the rumour is available behind ada's back"
        (any ("Warn bex that ada resents" `isInfixOf`) (map gaLabel (possibleActions s0 "you")))
      s1 <- runSteps s0 [ ("you", "Warn bex that ada resents") ]
      assertBool "bex now believes ada resents them"
        ("bex.believes.resentedBy.ada.yes" `elem` dbToSentences (db s1))

  , testCase "evidence of warmth dispels a false belief" $ do
      let s0 = foldl (flip performOutcome) barWorld
                 [ Insert "practice.world.world.at.bex!bar"
                 , believe "bex" "resentedBy.ada" "yes"
                 , Insert "practice.greet.world.greeted.ada.bex" ]  -- ada actually greeted bex
      assertBool "bex can reconsider"
        (any ("Realize ada doesn't resent you" `isInfixOf`) (map gaLabel (possibleActions s0 "bex")))
      s1 <- runSteps s0 [ ("bex", "Realize ada doesn't resent you") ]
      assertBool "the false belief is dropped"
        (not (any ("bex.believes.resentedBy.ada" `isInfixOf`) (dbToSentences (db s1))))

  , testCase "friends can strike up a chat; quips stay on topic and shift feeling" $ do
      -- bex is fond of ada (warmth 20) and co-located: a conversation is possible.
      let warm = foldl (flip performOutcome) barWorld
                   [ Insert "practice.world.world.at.bex!bar"
                   , adjustScore "bex" "ada" warmth 20 "fond" ]
      assertBool "warm bex can strike up a conversation"
        (any ("Strike up a conversation with ada" `isInfixOf`) (map gaLabel (possibleActions warm "bex")))
      s1 <- runSteps warm [ ("bex", "Strike up a conversation with ada") ]
      -- opens on small talk: the compliment quip (rapport) is off-topic and withheld
      assertBool "small talk is on topic"
        (any ("Make small talk with ada" `isInfixOf`) (map gaLabel (possibleActions s1 "bex")))
      assertBool "compliment is off topic (withheld)"
        (not (any ("Compliment ada" `isInfixOf`) (map gaLabel (possibleActions s1 "bex"))))
      -- small talk (turn -> ada), ada steers to rapport (turn -> bex), bex compliments ada
      s2 <- runSteps s1
              [ ("bex", "Make small talk with ada")
              , ("ada", "Warm the talk toward rapport")
              , ("bex", "Compliment ada") ]
      assertBool "the compliment warmed ada toward bex"
        ("ada.relationship.bex.warmth.score.10" `elem` dbToSentences (db s2))

  , testCase "a gossip quip transmits a (possibly-false) belief in conversation" $ do
      -- bex, cross with you, is chatting with ada on the gossip topic.
      let g0 = foldl (flip performOutcome) barWorld
                 ([ setMood "bex" annoyed "you" "grudge" ]
                   ++ beginConversation "bex" "ada" "gossip")
      assertBool "the gossip quip is available to the speaker"
        (any ("Confide to ada that you resents them" `isInfixOf`)
             (map gaLabel (possibleActions g0 "bex")))
      g1 <- runSteps g0 [ ("bex", "Confide to ada that you resents them") ]
      assertBool "ada now believes you resent her"
        ("ada.believes.resentedBy.you.yes" `elem` dbToSentences (db g1))

  , testCase "the director (story manager) has nothing to do in a placid room" $
      -- No warm pair yet, so the director's metalevel action is unavailable…
      assertBool "director idle when nothing is warm"
        (null (possibleActions barWorld "director"))

  , testCase "the director injects a rivalry between two warm friends" $ do
      -- Make ada and bex fond of each other.
      let warm = foldl (flip performOutcome) barWorld
                   [ adjustScore "ada" "bex" warmth 25 "friends"
                   , adjustScore "bex" "ada" warmth 25 "friends" ]
      -- The director's only move is its metalevel one (it is bound; it has no body).
      let dirActs = possibleActions warm "director"
      assertBool "director can now act" (not (null dirActs))
      assertBool "director only acts through its own (dm) practice"
        (all ((== "dm") . gaPracticeId) dirActs)
      -- It turns the two friends against each other (once).
      stirred <- runSteps warm [ ("director", "turn ada against bex") ]
      let fs = dbToSentences (db stirred)
      assertBool "the beat is marked done" ("dm.stirred" `elem` fs)
      assertBool "ada now bears a grievance against bex"
        ("practice.greet.world.grievance.ada.bex" `elem` fs)
      assertBool "and their warmth has soured"
        ("ada.relationship.bex.warmth.score.-5" `elem` fs)

  , testCase "a character's arc advances to belonging once it feels at home" $ do
      -- bex feels genuinely warm toward ada.
      let warm = performOutcome (adjustScore "bex" "ada" warmth 20 "fond") barWorld
      assertBool "the belonging beat is available"
        (any ("settle in, feeling you belong" `isInfixOf`) (map gaLabel (possibleActions warm "bex")))
      settled <- runSteps warm [ ("bex", "settle in, feeling you belong") ]
      assertBool "bex now belongs" ("bex.arc.belonging" `elem` dbToSentences (db settled))

  , testCase "the against-desires transformation is offered but the planner refuses it" $ do
      -- Every hopeful patron *can* resign themselves to loneliness…
      assertBool "the transformation is on the table"
        (any ("give up on the evening" `isInfixOf`) (map gaLabel (possibleActions barWorld "bex")))
      -- …but an NPC never chooses it (sliding into loneliness only costs utility):
      -- true transformation is, in practice, a player-only act.
      let bexChar = head [ c | c <- characters barWorld, charName c == "bex" ]
      assertBool "bex never resigns to solitude on its own"
        (maybe True (not . ("give up" `isInfixOf`) . gaLabel) (pickAction 2 barWorld bexChar))
  ]
