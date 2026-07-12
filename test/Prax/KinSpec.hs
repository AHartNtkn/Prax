module Prax.KinSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Either (isLeft)
import           Data.List (isInfixOf, isPrefixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, assertFailure)

import           Prax.Db (exists, dbToSentences)
import           Prax.Types
import           Prax.Engine (performOutcome, setAxioms, definePractice, possibleActions, performAction)
import           Prax.Faction (joins, comrades)
import           Prax.Kin

-- Two generations: gran > pat > {ana, ben}; a separate unmarried family
-- mia > {cass, dan}; ana marries chris (a stranger to both families).
kinSetup :: [Outcome]
kinSetup =
  [ Insert "parent.gran.pat"
  , Insert "parent.pat.ana"
  , Insert "parent.pat.ben"
  , Insert "parent.mia.cass"
  , Insert "parent.mia.dan"
  , Insert "married.ana.chris"
  ]

kinWorld :: PraxState
kinWorld = setAxioms kinAxioms (foldl (flip performOutcome) emptyState kinSetup)

kinView :: [String]
kinView = dbToSentences (readView kinWorld)

-- Two houses (hall: ana, ben; yard: cass), plus ana's parent pat, so a
-- wedding into yard has an in-law to un-derive on dissolution.
houseSetup :: [Outcome]
houseSetup =
  [ joins "ana" "hall", joins "ben" "hall", joins "cass" "yard"
  , Insert "parent.pat.ana" ]

houseWorld :: PraxState
houseWorld = setAxioms (kinAxioms ++ [comrades]) (foldl (flip performOutcome) emptyState houseSetup)

weddedWorld :: PraxState
weddedWorld = foldl (flip performOutcome) houseWorld (wed "ana" "yard" "cass")

-- Succession fixture: a single-role-free practice hosting the claim action for
-- office "throne", rex the holder, ana and ben his children, cass unrelated.
-- @roles = []@ is a zero-role practice, spawned by inserting the bare
-- @practice.succession@ fact with no trailing role value — the technique
-- generalizes ConversationSpec's practice-wrapper idiom for testing a bare
-- Action (there always instantiated with at least one role), but a
-- zero-role instance is novel to this spec, not itself precedented
-- elsewhere in the suite. It works because 'possibleActions' only requires
-- the instance fact to exist and unify — no role values to bind.
successionP :: Practice
successionP = practice
  { practiceId = "succession"
  , roles = []
  , actions = [ succession "throne" ]
  }

successionSetup :: [Outcome]
successionSetup =
  [ Insert "practice.succession"
  , Insert "office.throne!rex"
  , Insert "parent.rex.ana"
  , Insert "parent.rex.ben"
  ]

successionWorld :: PraxState
successionWorld =
  foldl (flip performOutcome) (definePractice successionP emptyState) successionSetup

opts :: PraxState -> String -> [String]
opts st a = map gaLabel (possibleActions st a)

perform :: PraxState -> String -> String -> IO PraxState
perform st actor needle =
  case filter ((needle `isInfixOf`) . gaLabel) (possibleActions st actor) of
    (ga : _) -> pure (performAction st ga)
    []       -> assertFailure ("no " ++ show needle ++ " for " ++ actor
                               ++ "; had: " ++ show (opts st actor)) >> pure st

tests :: TestTree
tests = testGroup "Prax.Kin"
  [ testCase "marriage symmetry: married.A.B derives married.B.A" $
      assertBool "chris married ana too" ("married.chris.ana" `elem` kinView)

  , testCase "marriage symmetry negative: an unmarried pair derives no symmetric fact" $
      assertBool "dan and cass are never married"
        (not (any (`elem` kinView) ["married.dan.cass", "married.cass.dan"]))

  , testCase "sibling: shared parent, X<>Y, derives sibling both ways" $
      assertBool "ana and ben are siblings"
        (all (`elem` kinView) ["sibling.ana.ben", "sibling.ben.ana"])

  , testCase "sibling negative: no shared parent, no sibling fact" $
      assertBool "ana and cass share no parent, not siblings"
        (not (any (`elem` kinView) ["sibling.ana.cass", "sibling.cass.ana"]))

  , testCase "grandparent: parent-of-parent derives grandparent" $
      assertBool "gran is ana's grandparent" ("grandparent.gran.ana" `elem` kinView)

  , testCase "grandparent negative: no parent chain, no grandparent fact" $
      assertBool "gran is not chris's grandparent" ("grandparent.gran.chris" `notElem` kinView)

  , testCase "inLaw (spouse's parent): married.A.B + parent.P.A derives inLaw.P.B" $
      assertBool "pat (ana's parent) is chris's in-law" ("inLaw.pat.chris" `elem` kinView)

  , testCase "inLaw (spouse's parent) negative: no marriage, no in-law via that parent" $
      assertBool "mia's unmarried children yield no inLaw.mia.* fact at all"
        (not (any ("inLaw.mia." `isPrefixOf`) kinView))

  , testCase "inLaw (sibling's spouse): married.A.B + sibling.A.S derives inLaw.S.B" $
      assertBool "ben (ana's sibling) is chris's in-law" ("inLaw.ben.chris" `elem` kinView)

  , testCase "inLaw (sibling's spouse) negative: siblings without a marriage derive no in-law" $
      assertBool "cass and dan are siblings but unmarried — neither is anyone's in-law"
        (not (any (\s -> "inLaw.cass." `isPrefixOf` s || "inLaw.dan." `isPrefixOf` s) kinView))

  , testCase "wed: guards forced — joiner and spouse must be single path segments" $ do
      r1 <- try (evaluate (length (wed "" "hall" "cass")))
      assertBool "an empty joiner name errors" (isLeft (r1 :: Either ErrorCall Int))
      r2 <- try (evaluate (length (wed "ana" "hall" "b.ad")))
      assertBool "a dotted spouse name errors" (isLeft (r2 :: Either ErrorCall Int))
      r3 <- try (evaluate (length (wed "ana" "hall" "b!ad")))
      assertBool "a bang'd spouse name errors" (isLeft (r3 :: Either ErrorCall Int))

  , testCase "wed: inserts the marriage fact and overwrites the joiner's membership" $ do
      assertBool "married.ana.cass is a base fact" (exists "married.ana.cass" (db weddedWorld))
      assertBool "ana's membership moved to yard" (exists "member.ana!yard" (db weddedWorld))
      assertBool "ana's old hall membership is gone" (not (exists "member.ana!hall" (db weddedWorld)))

  , testCase "wed: the membership overwrite un-derives the joiner's old alliances (Faction composition)" $ do
      let v = readView weddedWorld
      assertBool "ana no longer allied with ben (old house)" (not (exists "allied.ana.ben" v))
      assertBool "ben no longer allied with ana (old house)" (not (exists "allied.ben.ana" v))
      assertBool "ana is now allied with cass (new house)" (exists "allied.ana.cass" v)
      assertBool "cass is now allied with ana (new house)" (exists "allied.cass.ana" v)

  , testCase "dissolution: retracting married un-derives in-laws but leaves membership UNCHANGED" $ do
      assertBool "pre-dissolution: pat (ana's parent) is cass's in-law (via ana's marriage)"
        (exists "inLaw.pat.cass" (readView weddedWorld))
      let dissolved = performOutcome (Delete "married.ana.cass") weddedWorld
          v = readView dissolved
      assertBool "post-dissolution: the in-law fact is gone" (not (exists "inLaw.pat.cass" v))
      assertBool "post-dissolution: the symmetric marriage fact is gone" (not (exists "married.cass.ana" v))
      assertBool "post-dissolution: ana's membership is UNCHANGED in the base"
        (exists "member.ana!yard" (db dissolved))
      assertBool "post-dissolution: ana's membership is UNCHANGED in the view"
        (exists "member.ana!yard" v)
      assertBool "the designed asymmetry: membership-derived alliance survives (it was never a kin derivation)"
        (exists "allied.ana.cass" v)

  , testCase "succession: not offered while the holder lives" $
      assertBool "ana cannot claim the throne while rex lives"
        (not (any ("claim the office of throne" `isInfixOf`) (opts successionWorld "ana")))

  , testCase "succession: only children may claim, once the holder is dead" $ do
      let deadWorld = performOutcome (Insert "dead.rex") successionWorld
      assertBool "ana (a child) may claim" (any ("claim the office of throne" `isInfixOf`) (opts deadWorld "ana"))
      assertBool "ben (a child) may claim" (any ("claim the office of throne" `isInfixOf`) (opts deadWorld "ben"))
      assertBool "cass (not a child of rex) may not claim"
        (not (any ("claim the office of throne" `isInfixOf`) (opts deadWorld "cass")))

  , testCase "succession: a performed claim overwrites the slot and closes the affordance for the other child" $ do
      let deadWorld = performOutcome (Insert "dead.rex") successionWorld
      claimed <- perform deadWorld "ana" "claim the office of throne"
      assertBool "the office now belongs to ana" (exists "office.throne!ana" (db claimed))
      assertBool "rex's old holding is gone (single-slot overwrite)" (not (exists "office.throne!rex" (db claimed)))
      assertBool "ben can no longer claim — the race is closed"
        (not (any ("claim the office of throne" `isInfixOf`) (opts claimed "ben")))
      assertBool "ana cannot re-claim her own office (she is not dead)"
        (not (any ("claim the office of throne" `isInfixOf`) (opts claimed "ana")))

  , testCase "succession: guards forced — office names must be single path segments" $ do
      r1 <- try (evaluate (length (actionName (succession ""))))
      assertBool "an empty office name errors" (isLeft (r1 :: Either ErrorCall Int))
      r2 <- try (evaluate (length (actionName (succession "a.b"))))
      assertBool "a dotted office name errors" (isLeft (r2 :: Either ErrorCall Int))
      r3 <- try (evaluate (length (actionName (succession "a!b"))))
      assertBool "a bang'd office name errors" (isLeft (r3 :: Either ErrorCall Int))
  ]
