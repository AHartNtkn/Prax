module Prax.DriftSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Either (isLeft)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool)

import           Prax.Db (exists)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, setCharacters)
import           Prax.Loop (npcAct)
import           Prax.TypeCheck (TypeError (..), typeCheck)
import           Prax.Drift

-- A one-rule world: "tick" marks every flagged thing on the pulse.
markR :: DriftRule
markR = DriftRule "mark" 2 [([Match "flag.X"], [Insert "marked.X"])]

-- Build the drifter's world for a rule set, seeded and clocked at turn 0.
drifty :: [DriftRule] -> PraxState
drifty rules = foldl (flip performOutcome)
    (setCharacters [driftChar] (definePractices [driftP rules] emptyState))
    (driftSetup rules ++ [Insert "turn!0"])

-- One drifter turn.
pulse :: PraxState -> PraxState
pulse st = snd (npcAct 2 driftChar st)

-- Overwrite the clock (the unit tests own it; no sight ticker in fixtures).
atTurn :: Int -> PraxState -> PraxState
atTurn k = performOutcome (Insert ("turn!" ++ show k))

isClocklessDrift :: TypeError -> Bool
isClocklessDrift ClocklessDrift = True
isClocklessDrift _              = False

tests :: TestTree
tests = testGroup "Prax.Drift"
  [ testCase "a rule does not fire before its due" $ do
      let st = pulse (atTurn 1 (performOutcome (Insert "flag.a") (drifty [markR])))
      assertBool "marked.a absent (due seeded at 2, now 1)" (not (exists "marked.a" (db st)))

  , testCase "a rule fires at its due, covering every binding" $ do
      let base = performOutcome (Insert "flag.b")
                   (performOutcome (Insert "flag.a") (drifty [markR]))
          st = pulse (atTurn 2 base)
      assertBool "marked.a inserted" (exists "marked.a" (db st))
      assertBool "marked.b inserted" (exists "marked.b" (db st))

  , testCase "the due re-arms period rounds from now" $ do
      let base = performOutcome (Insert "flag.a") (drifty [markR])
          st2 = pulse (atTurn 2 base)
      assertBool "due bumped to 2 + period = 4" (exists "due.mark!4" (db st2))
      let st3 = pulse (atTurn 3 st2)
      assertBool "still due at 4 (not yet due at turn 3)" (exists "due.mark!4" (db st3))
      assertBool "no re-fire before the new due" (exists "marked.a" (db st3))

  , testCase "two rules with different periods fire on their own schedules" $ do
      let p2 = DriftRule "p2" 2 [([Match "flagA.X"], [Insert "markedA.X"])]
          p3 = DriftRule "p3" 3 [([Match "flagB.X"], [Insert "markedB.X"])]
          base = performOutcome (Insert "flagB.a")
                   (performOutcome (Insert "flagA.a") (drifty [p2, p3]))
          drive st k = pulse (atTurn k st)
          st1 = drive base 1
          st2 = drive st1 2
          st3 = drive st2 3
          st4 = drive st3 4
          st5 = drive st4 5
          st6 = drive st5 6

      -- turn 1: neither due (seeded at 2 / 3) — no re-arm.
      assertBool "p2 unchanged at turn 1" (exists "due.p2!2" (db st1))
      assertBool "p3 unchanged at turn 1" (exists "due.p3!3" (db st1))

      -- turn 2: p2 fires (2>=2), re-arms to 4; p3 not yet due.
      assertBool "p2 fires at turn 2, re-arms to 4" (exists "due.p2!4" (db st2))
      assertBool "p3 still due at 3 after turn 2" (exists "due.p3!3" (db st2))

      -- turn 3: p3 fires (3>=3), re-arms to 6; p2 not yet due (4).
      assertBool "p2 still due at 4 after turn 3" (exists "due.p2!4" (db st3))
      assertBool "p3 fires at turn 3, re-arms to 6" (exists "due.p3!6" (db st3))

      -- turn 4: p2 fires (4>=4), re-arms to 6; p3 not yet due (6).
      assertBool "p2 fires at turn 4, re-arms to 6" (exists "due.p2!6" (db st4))
      assertBool "p3 still due at 6 after turn 4" (exists "due.p3!6" (db st4))

      -- turn 5: neither due (p2 at 6, p3 at 6).
      assertBool "p2 unchanged at turn 5" (exists "due.p2!6" (db st5))
      assertBool "p3 unchanged at turn 5" (exists "due.p3!6" (db st5))

      -- turn 6: both fire (6>=6), re-arm to 8 and 9 respectively.
      assertBool "p2 fires at turn 6, re-arms to 8" (exists "due.p2!8" (db st6))
      assertBool "p3 fires at turn 6, re-arms to 9" (exists "due.p3!9" (db st6))

      -- the fired facts landed exactly as expected.
      assertBool "markedA.a present" (exists "markedA.a" (db st6))
      assertBool "markedB.a present" (exists "markedB.a" (db st6))

  , testCase "a multi-segment rule name is a loud construction-time error" $ do
      r <- try (evaluate (length (show (driftP [DriftRule "a.b" 2 []]))))
      assertBool "multi-segment name rejected" (isLeft (r :: Either ErrorCall Int))

  , testCase "a body mentioning the reserved Now variable is a loud error" $ do
      r <- try (evaluate (length (show
             (driftP [DriftRule "x" 2 [([Match "flag.Now"], [])]]))))
      assertBool "reserved variable Now rejected" (isLeft (r :: Either ErrorCall Int))

  , testCase "a zero period is a loud error" $ do
      r <- try (evaluate (length (show
             (driftP [DriftRule "x" 0 [([Match "flag.a"], [Insert "marked.a"])]]))))
      assertBool "zero period rejected" (isLeft (r :: Either ErrorCall Int))

  , testCase "a drift world with no clock is flagged; adding turn!0 clears it" $ do
      let w = setCharacters [driftChar] (definePractices [driftP [markR]] emptyState)
      assertBool "ClocklessDrift flagged" (any isClocklessDrift (typeCheck w))
      let w' = performOutcome (Insert "turn!0") w
      assertBool "clear once clocked" (not (any isClocklessDrift (typeCheck w')))

  , testCase "the drifty fixture world is well-formed" $
      assertBool "no type errors" (null (typeCheck (drifty [markR])))
  ]
