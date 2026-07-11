module Prax.DeceitSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Either (isLeft)
import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, possibleActions, performAction)
import           Prax.Planner (pickAction)
import           Prax.Witness (CoPresence, observable)
import           Prax.Rumor (gossip)
import           Prax.Deceit

together :: CoPresence
together = [ Match "at.Actor!P", Match "at.Witness!P" ]

-- The tale: sid covets the gem but not being seen taking it matters more;
-- nia whispers lies about kit. oz and kit share the yard; mia is at the shed.
world :: PraxState
world = foldl (flip performOutcome) base setup
  where
    base = (definePractices [p] emptyState)
             { characters =
                 [ (character "sid")
                     { charWants = [ Want [ Match "holding.sid.gem" ] 5
                                   , conceal "took.sid.gem" 8 ] }
                 , (character "nia")
                     { charWants = [ Want [ Match "W.believes.took.kit.gem" ] 4 ] }
                 , character "oz", character "kit", character "mia" ] }
    p = practice
      { practiceId = "vault", roles = ["R"]
      , actions =
          [ observable together "took.Actor.gem" $
            action "[Actor]: take the gem"
              [ Match "at.Actor!yard", Match "gem.here" ]
              [ Delete "gem.here", Insert "holding.Actor.gem" ]
          , gossip together [] "took.Culprit.gem"
              "[Actor]: tell [Hearer] that [Culprit] took the gem"
          , lie together []
              [ Match "at.Culprit!Anywhere" ]
              "took.Culprit.gem"
              "[Actor]: whisper to [Hearer] that [Culprit] took the gem"
          ] }
    setup =
      [ Insert "practice.vault.here"
      , Insert "at.sid!yard", Insert "at.nia!yard", Insert "at.oz!yard"
      , Insert "at.kit!yard", Insert "at.mia!shed"
      , Insert "gem.here" ]

charNamed :: String -> Character
charNamed n = case [ c | c <- characters world, charName c == n ] of
  (c : _) -> c
  []      -> error ("no such character: " ++ n)

-- Perform the named actor's action whose label mentions @needle@.
doAct :: String -> String -> PraxState -> PraxState
doAct who needle st =
  case [ ga | ga <- possibleActions st who, needle `isInfixOf` gaLabel ga ] of
    (ga : _) -> performAction st ga
    []       -> error ("no action for " ++ who ++ " matching " ++ show needle
                       ++ "; had: " ++ show (map gaLabel (possibleActions st who)))

offered :: String -> String -> PraxState -> Bool
offered who needle st =
  any ((needle `isInfixOf`) . gaLabel) (possibleActions st who)

-- Dedicated fixture for testing lie over motive patterns. Same structure as world,
-- but with a "revenge" desire and a motive-pattern lie action in the practice.
motiveLieWorld :: PraxState
motiveLieWorld = foldl (flip performOutcome) base setup
  where
    base = (definePractices [pMotive] emptyState)
             { characters =
                 [ character "nia", character "oz", character "kit" ]
             , desires = [ Desire "revenge" (Want [ Match "harms.Owner" ] 10) ]
             }
    pMotive = practice
      { practiceId = "rumor", roles = ["R"]
      , actions =
          [ lie together []
              [ Match "at.Culprit!Anywhere" ]
              "desires.Culprit.revenge"
              "[Actor]: whisper to [Hearer] that [Culprit] wants revenge"
          ]
      }
    setup =
      [ Insert "practice.rumor.here"
      , Insert "at.nia!yard", Insert "at.oz!yard", Insert "at.kit!yard"
      ]

tests :: TestTree
tests = testGroup "Prax.Deceit"
  [ testCase "conceal is the nobody-believes want" $
      conceal "took.sid.gem" 8
        @?= Want [ Absent [ Match "Anyone.believes.took.sid.gem" ] ] 8

  , testCase "conceal rejects a variable-bearing event loudly" $ do
      r <- try (evaluate (length (show (conceal "took.Who.gem" 8))))
      assertBool "variables in a secret are an error"
        (isLeft (r :: Either ErrorCall Int))

  , testCase "a concealer waits for privacy, then acts, and no one knows" $ do
      assertBool "watched: sid does not take the gem"
        (fmap gaLabel (pickAction 2 world (charNamed "sid"))
           /= Just "sid: take the gem")
      let alone = foldl (flip performOutcome) world
                    [ Insert "at.nia!shed", Insert "at.oz!shed", Insert "at.kit!shed" ]
      fmap gaLabel (pickAction 2 alone (charNamed "sid"))
        @?= Just "sid: take the gem"
      let st = doAct "sid" "take the gem" alone
      assertBool "took it" (exists "holding.sid.gem" (db st))
      assertBool "and nobody believes it"
        (not (any (\w -> exists (w ++ ".believes.took.sid.gem") (db st))
                  ["nia", "oz", "kit", "mia"]))

  , testCase "a lie plants sourced hearsay the liar never had evidence for" $ do
      let st = doAct "nia" "whisper to oz that kit took the gem" world
      assertBool "oz heard it from nia"
        (exists "oz.believes.took.kit.gem.heard.nia" (db st))
      assertBool "nia still has no evidence of her own"
        (not (exists "nia.believes.took.kit.gem" (db st)))

  , testCase "hearing your own lie back turns it into gossip" $ do
      let st1 = doAct "nia" "whisper to oz that kit took the gem" world
          st2 = doAct "oz" "tell nia that kit took the gem" st1
      assertBool "nia now holds (fabricated) evidence"
        (exists "nia.believes.took.kit.gem.heard.oz" (db st2))
      assertBool "the lie action is gone (its no-evidence gate closed)"
        (not (offered "nia" "whisper to oz that kit" st2)
         && not (any (\ga -> "whisper" `isInfixOf` gaLabel ga
                             && "kit took" `isInfixOf` gaLabel ga)
                     (possibleActions st2 "nia")))
      assertBool "plain gossip appears in its place"
        (offered "nia" "tell oz that kit took" st2
         || offered "nia" "tell mia that kit took" st2
         || offered "nia" "tell sid that kit took" st2)

  , testCase "you cannot frame yourself (a lie about yourself is a confession)" $
      assertBool "no whisper names the whisperer"
        (not (any (\ga -> "whisper" `isInfixOf` gaLabel ga
                          && "nia took" `isInfixOf` gaLabel ga)
                  (possibleActions world "nia")))

  , testCase "the subject of the lie is never the hearer" $
      assertBool "no whispering to kit about kit"
        (not (any (\ga -> "whisper to kit that kit" `isInfixOf` gaLabel ga)
                  (possibleActions world "nia")))

  , testCase "lying to the same hearer twice is not offered" $ do
      let st = doAct "nia" "whisper to oz that kit took the gem" world
      assertBool "one-shot per hearer"
        (not (offered "nia" "whisper to oz that kit" st))

  , testCase "a subject-less pattern errors loudly" $ do
      r <- try (evaluate (length (show
             (lie together [] [] "somethinghappened"
                "[Actor]: mention it to [Hearer]"))))
      assertBool "a lie must be about someone"
        (isLeft (r :: Either ErrorCall Int))

  , testCase "a lie can fabricate a MOTIVE: desires.* patterns work like deed patterns" $ do
      -- nia whispers that kit nurses a revenge desire — evidence-free motive framing.
      let st = doAct "nia" "whisper to oz that kit wants revenge" motiveLieWorld
      assertBool "oz believes kit desires revenge (heard from nia)"
        (exists "oz.believes.desires.kit.revenge.heard.nia" (db st))
      assertBool "nia keeps no evidence of her own motive claim"
        (not (exists "nia.believes.desires.kit.revenge" (db st)))
      assertBool "kit is never offered as hearer (subject cannot be hearer)"
        (not (offered "nia" "whisper to kit that kit wants" motiveLieWorld))
  ]
