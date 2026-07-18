module Prax.RngSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Either (isLeft)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (seedDie, performOutcome)
import           Prax.Rng

-- Park & Miller MINSTD, pinned here independently of "Prax.Rng" so the tests
-- are self-checking arithmetic, not a restatement of the implementation.
lehmerNext :: Integer -> Integer
lehmerNext s = (s * 16807) `mod` 2147483647

-- The die seeded into engine state (v50: no fact, no db touch).
seeded :: Integer -> PraxState
seeded s = seedDie s emptyState

-- Apply one 'draw' fragment's compiled outcomes to a state.
applyDraw :: Int -> Int -> [Condition] -> [Outcome] -> PraxState -> PraxState
applyDraw num den conds outs st = foldl (flip performOutcome) st (draw num den conds outs)

tests :: TestTree
tests = testGroup "Prax.Rng"
  [ testGroup "the stream (engine state, v50)"
    [ testCase "rollStep is one Park-Miller step, and each draw advances the seed exactly once" $ do
        let s0 = 12345
            s1 = lehmerNext s0
            s2 = lehmerNext s1
            s3 = lehmerNext s2
        rollStep s0 @?= s1          -- the die math, pinned against the constants
        let st1 = applyDraw 1 2 [] [] (seeded s0)
            st2 = applyDraw 1 2 [] [] st1
            st3 = applyDraw 1 2 [] [] st2
        rngSeed st1 @?= Just s1
        rngSeed st2 @?= Just s2
        rngSeed st3 @?= Just s3
    ]

  , testGroup "the frozen-die law"
    [ testCase "two draws with unsatisfiable guards still advance the seed twice" $ do
        let s0 = 5
            impossible = [ Match "ghost.nothing" ]  -- no such fact anywhere
            st1 = applyDraw 1 2 impossible [Insert "should.never.fire"] (seeded s0)
            st2 = applyDraw 1 2 impossible [Insert "should.never.fire"] st1
        rngSeed st2 @?= Just (lehmerNext (lehmerNext s0))
        assertBool "the unsatisfiable-guard outs never fired"
          (not (exists "should.never.fire" (db st2)))

    , testCase "a miss advances: the SAME position, drawn twice, diverges on the next draw" $ do
        -- The whole point of the law: a failed roll is not sticky. From one
        -- position, two consecutive draws roll on DIFFERENT (successive) values.
        let s0 = 7
            r1 = lehmerNext s0            -- first draw rolls here
            r2 = lehmerNext r1            -- second draw rolls here (advanced again)
        assertBool "fixture: the two roll bases differ" (r1 /= r2)
        let st1 = applyDraw 1 2 [] [] (seeded s0)
        rngSeed st1 @?= Just r1
        rngSeed (applyDraw 1 2 [] [] st1) @?= Just r2
    ]

  , testGroup "hit / miss"
    [ testCase "a hit applies outs (seed 2, odds 1/2 -> rollStep 2 is even)" $ do
        let s0 = 2
        assertBool "fixture check: rollStep 2 mod 2 == 0 (a hit)"
          (even (rollStep s0))
        let st = applyDraw 1 2 [] [Insert "hit.mark"] (seeded s0)
        assertBool "hit.mark inserted on a hit" (exists "hit.mark" (db st))
        rngSeed st @?= Just (lehmerNext s0)

    , testCase "a miss does not apply outs (seed 1, odds 1/2 -> rollStep 1 is odd)" $ do
        let s0 = 1
        assertBool "fixture check: rollStep 1 mod 2 == 1 (a miss)"
          (odd (rollStep s0))
        let st = applyDraw 1 2 [] [Insert "hit.mark"] (seeded s0)
        assertBool "hit.mark absent on a miss" (not (exists "hit.mark" (db st)))
        rngSeed st @?= Just (lehmerNext s0)
    ]

  , testGroup "sequential multi-draw (Village.hs's two-arm shape)"
    [ testCase "two draws off one stream roll on successive values and advance twice" $ do
        let s0 = 1988                -- village's seed: both arms hit here
            s1 = lehmerNext s0
            s2 = lehmerNext s1
        assertBool "fixture: base arm hits (s1 mod 4 < 1)" (s1 `mod` 4 < 1)
        assertBool "fixture: trait arm hits (s2 mod 4 < 2)" (s2 `mod` 4 < 2)
        let st = applyDraw 2 4 [] [Insert "arm2.fired"]
                   (applyDraw 1 4 [] [Insert "arm1.fired"] (seeded s0))
        assertBool "arm1 (base, on s1) fired"  (exists "arm1.fired" (db st))
        assertBool "arm2 (trait, on s2) fired" (exists "arm2.fired" (db st))
        rngSeed st @?= Just s2
    ]

  , testGroup "an unseeded die is loud"
    [ testCase "executing a Roll with rngSeed == Nothing is a loud error" $ do
        r <- try (evaluate (rngSeed (performOutcome (Roll 1 2 [] [Insert "x"]) emptyState)))
        assertBool "unseeded Roll rejected" (isLeft (r :: Either ErrorCall (Maybe Integer)))
    ]

  , testGroup "seedDie domain guard"
    [ testCase "an in-domain seed is accepted" $
        rngSeed (seedDie 12345 emptyState) @?= Just 12345

    , testCase "the domain bounds are the open interval (0, modulus)" $
        seedBounds @?= (1, 2147483646)

    , testCase "seedDie rejects a seed of 0" $ do
        r <- try (evaluate (rngSeed (seedDie 0 emptyState)))
        assertBool "seed 0 rejected" (isLeft (r :: Either ErrorCall (Maybe Integer)))

    , testCase "seedDie rejects a seed at or above the modulus" $ do
        r <- try (evaluate (rngSeed (seedDie 2147483647 emptyState)))
        assertBool "seed >= modulus rejected" (isLeft (r :: Either ErrorCall (Maybe Integer)))

    , testCase "seedDie rejects a negative seed" $ do
        r <- try (evaluate (rngSeed (seedDie (-5) emptyState)))
        assertBool "negative seed rejected" (isLeft (r :: Either ErrorCall (Maybe Integer)))
    ]

  , testGroup "draw's authoring guards (surviving pins, re-pointed to the Roll form)"
    [ testCase "num == 0 is rejected" $ do
        r <- try (evaluate (length (draw 0 2 [] [])))
        assertBool "num 0 rejected" (isLeft (r :: Either ErrorCall Int))

    , testCase "num == den is rejected (certainty is not a chance)" $ do
        r <- try (evaluate (length (draw 2 2 [] [])))
        assertBool "num == den rejected" (isLeft (r :: Either ErrorCall Int))

    , testCase "num > den is rejected" $ do
        r <- try (evaluate (length (draw 3 2 [] [])))
        assertBool "num > den rejected" (isLeft (r :: Either ErrorCall Int))

    , testCase "the Prax namespace in the caller's conditions is rejected" $ do
        r <- try (evaluate (length (draw 1 2 [Match "flag.PraxS"] [])))
        assertBool "PraxS in conds rejected" (isLeft (r :: Either ErrorCall Int))

    , testCase "the Prax namespace in the caller's outcomes is rejected" $ do
        r <- try (evaluate (length (draw 1 2 [] [Insert "marked.PraxR"])))
        assertBool "PraxR in outs rejected" (isLeft (r :: Either ErrorCall Int))

    , testCase "ordinary variables S/S2/S3/R are unremarkable authoring names" $ do
        r <- try (evaluate (length (draw 1 2 [Match "flag.S"] [Insert "marked.R"])))
        assertBool "S and R are unremarkable author variables"
          (not (isLeft (r :: Either ErrorCall Int)))
    ]
  ]
