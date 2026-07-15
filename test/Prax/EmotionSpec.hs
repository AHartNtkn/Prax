module Prax.EmotionSpec (tests) where

import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (dbToSentences, exists)
import           Prax.Query (Condition (..), query)
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, setCharacters)
import           Prax.Loop (npcAct)
import           Prax.Planner (candidateActions)
import           Prax.Drift (driftChar, driftP, driftSetup)
import           Prax.Emotion

facts :: PraxState -> [String]
facts = dbToSentences . db

-- Coexistence -------------------------------------------------------------------

tests :: TestTree
tests = testGroup "Prax.Emotion"
  [ testGroup "coexisting feelings"
    [ testCase "angry at carol and afraid of bob coexist independently" $ do
        let st0 = performOutcome (feelToward "ada" angry "carol") emptyState
            st1 = performOutcome (feelToward "ada" afraid "bob") st0
            fs  = facts st1
        assertBool "angry at carol" ("ada.feels.angry.toward.carol" `elem` fs)
        assertBool "afraid of bob"  ("ada.feels.afraid.toward.bob" `elem` fs)

    , testCase "unfeeling one leaves the other standing" $ do
        let st0 = performOutcome (feelToward "ada" angry "carol") emptyState
            st1 = performOutcome (feelToward "ada" afraid "bob") st0
            st2 = performOutcome (unfeelToward "ada" angry "carol") st1
            fs  = facts st2
        assertBool "angry at carol discharged" ("ada.feels.angry.toward.carol" `notElem` fs)
        assertBool "afraid of bob survives"    ("ada.feels.afraid.toward.bob" `elem` fs)

    , testCase "untargeted and targeted instances of the same emotion coexist" $ do
        -- 'exists' is what 'feeling' checks; and since v39's asserted-
        -- endpoint marking, the untargeted instance ALSO survives as its own
        -- asserted fact ('dbToSentences' now emits it) even after gaining a
        -- targeted sibling underneath — the old leaf-vs-ancestor ambiguity
        -- is gone.
        let st0 = performOutcome (feel "ada" happy) emptyState
            st1 = performOutcome (feelToward "ada" happy "carol") st0
        assertBool "untargeted happy" (exists "ada.feels.happy" (db st1))
        assertBool "targeted happy"   (exists "ada.feels.happy.toward.carol" (db st1))

    , testCase "unfeel discharges the whole feeling, targets included" $ do
        let st0 = performOutcome (feelToward "ada" angry "carol") emptyState
            st1 = performOutcome (feelToward "ada" angry "bob") st0
            st2 = performOutcome (unfeel "ada" angry) st1
            fs  = facts st2
        assertBool "angry at carol gone" ("ada.feels.angry.toward.carol" `notElem` fs)
        assertBool "angry at bob gone"   ("ada.feels.angry.toward.bob" `notElem` fs)
    ]

  , testGroup "feeling/feelingToward"
    [ testCase "feeling matches a targeted instance (Match sees subtrees)" $ do
        let st = performOutcome (feelToward "ada" annoyed "carol") emptyState
        assertBool "feeling annoyed matches the targeted instance"
          (not (null (query (db st) [feeling "ada" annoyed] Map.empty)))

    , testCase "feelingToward requires the specific target" $ do
        let st = performOutcome (feelToward "ada" annoyed "carol") emptyState
        assertBool "feelingToward carol holds"
          (not (null (query (db st) [feelingToward "ada" annoyed "carol"] Map.empty)))
        assertBool "feelingToward bob does not"
          (null (query (db st) [feelingToward "ada" annoyed "bob"] Map.empty))
    ]

  -- Fade-on-pulse, the DriftSpec hand-clock idiom: seed the drifter, jump the
  -- clock to the due turn, pulse once, observe both feelings gone.
  , testGroup "feelings fade"
    [ testCase "both feelings gone after the pulse; feeling again is possible" $ do
        let fadeR = feelingsFade 2
            atTurn :: Int -> PraxState -> PraxState
            atTurn k = performOutcome (Insert ("turn!" ++ show k))
            pulse st = snd (npcAct 2 driftChar st)
            base = foldl (flip performOutcome)
                     (setCharacters [driftChar] (definePractices [driftP [fadeR]] emptyState))
                     (driftSetup [fadeR] ++ [Insert "turn!0"])
            withFeelings =
              performOutcome (feelToward "ada" afraid "bob")
                (performOutcome (feelToward "ada" angry "carol") base)

        assertBool "angry present before the pulse"
          (exists "ada.feels.angry.toward.carol" (db withFeelings))
        assertBool "afraid present before the pulse"
          (exists "ada.feels.afraid.toward.bob" (db withFeelings))

        let st1 = pulse (atTurn 1 withFeelings)
        assertBool "not yet due at turn 1"
          (exists "ada.feels.angry.toward.carol" (db st1))

        let st2 = pulse (atTurn 2 st1)
        assertBool "angry gone at the due pulse"
          (not (exists "ada.feels.angry.toward.carol" (db st2)))
        assertBool "afraid gone at the due pulse"
          (not (exists "ada.feels.afraid.toward.bob" (db st2)))

        -- reappear-able: feeling again after the fade works exactly as before.
        let st3 = performOutcome (feelToward "ada" angry "carol") st2
        assertBool "angry can be felt again after fading"
          (exists "ada.feels.angry.toward.carol" (db st3))
    ]

  -- THE INVARIANT (load-bearing): emotions change decision-making, never what
  -- decisions can be made. Nothing in this module (and nothing pricing yet)
  -- may gate action availability — a fixture character's full candidateActions
  -- list must be identical with and without every vocabulary feeling present.
  , testGroup "the invariant: feelings never gate action availability"
    [ testCase "candidateActions is identical with and without every feeling" $ do
        let vocabulary = [happy, sad, angry, afraid, disgusted, surprised, annoyed, pleased]
            fixtureP = practice
              { practiceId = "fixture"
              , practiceName = "a fixture affordance"
              , roles = ["W"]
              , actions =
                  [ action "[Actor]: wave" [ Match "practice.fixture.W" ] []
                  , action "[Actor]: greet [Other]"
                      [ Match "character.Other", Neq "Actor" "Other" ] []
                  ]
              }
            fix    = character "fix"
            friend = character "pal"
            fixtureWorld =
              foldl (flip performOutcome)
                (setCharacters [fix, friend] (definePractices [fixtureP] emptyState))
                [ Insert "character.fix", Insert "character.pal"
                , Insert "practice.fixture.fix" ]
            baseline = candidateActions fixtureWorld fix
            withFeelings =
              foldl (flip performOutcome) fixtureWorld
                (concat [ [feel "fix" e, feelToward "fix" e "pal"] | e <- vocabulary ])
            afterFeelings = candidateActions withFeelings fix
        baseline @?= afterFeelings
    ]
  ]
