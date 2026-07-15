module Prax.RngSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Either (isLeft)
import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists, unify, valToString)
import           Prax.Sym (intern)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome)
import           Prax.TypeCheck (TypeError (..), typeCheck)
import           Prax.Rng

-- Park & Miller MINSTD, pinned here independently of "Prax.Rng" so the tests
-- are self-checking arithmetic, not a restatement of the implementation.
lehmerNext :: Integer -> Integer
lehmerNext s = (s * 16807) `mod` 2147483647

-- A world with the die seeded and nothing else.
seeded :: Integer -> PraxState
seeded s = foldl (flip performOutcome) emptyState (rngSetup s)

-- Apply one 'draw' fragment's compiled outcomes to a state.
applyDraw :: Int -> Int -> [Condition] -> [Outcome] -> PraxState -> PraxState
applyDraw num den conds outs st = foldl (flip performOutcome) st (draw num den conds outs)

-- Read the current stream position back out of the db.
readSeed :: PraxState -> Integer
readSeed st =
  case [ v | b <- unify "seed!S" (db st) Map.empty
           , Just v <- [Map.lookup (intern "S") b] ] of
    (v : _) -> read (valToString v)
    []      -> error "Prax.RngSpec: no seed fact in db"

isSeedlessDraw :: TypeError -> Bool
isSeedlessDraw SeedlessDraw = True
isSeedlessDraw _            = False

-- A practice whose action's outcomes are exactly a compiled draw — used to
-- drive the SeedlessDraw check.
provokeP :: Practice
provokeP = practice
  { practiceId = "provoke", roles = ["Actor"]
  , actions = [ action "[Actor]: provoke" []
                  (draw 1 2 [] [Insert "provoked.mark"]) ] }

tests :: TestTree
tests = testGroup "Prax.Rng"
  [ testGroup "determinism"
    [ testCase "same seed yields the same three-draw Lehmer stream, exactly" $ do
        let s0 = 12345
            s1 = lehmerNext s0
            s2 = lehmerNext s1
            s3 = lehmerNext s2
            st0 = seeded s0
            st1 = applyDraw 1 2 [] [] st0
            st2 = applyDraw 1 2 [] [] st1
            st3 = applyDraw 1 2 [] [] st2
        readSeed st1 @?= s1
        readSeed st2 @?= s2
        readSeed st3 @?= s3
    ]

  , testGroup "the frozen-die law"
    [ testCase "two draws with unsatisfiable guards still advance the seed twice" $ do
        let s0 = 5
            st0 = seeded s0
            impossible = [ Match "ghost.nothing" ]  -- no such fact anywhere
            st1 = applyDraw 1 2 impossible [Insert "should.never.fire"] st0
            st2 = applyDraw 1 2 impossible [Insert "should.never.fire"] st1
        readSeed st2 @?= lehmerNext (lehmerNext s0)
        assertBool "the unsatisfiable-guard outs never fired"
          (not (exists "should.never.fire" (db st2)))
    ]

  , testGroup "hit / miss"
    [ testCase "a hit applies outs (seed 2, odds 1/2 -> lehmerNext 2 is even)" $ do
        let s0 = 2
        assertBool "fixture check: lehmerNext 2 mod 2 == 0 (a hit)"
          (even (lehmerNext s0))
        let st = applyDraw 1 2 [] [Insert "hit.mark"] (seeded s0)
        assertBool "hit.mark inserted on a hit" (exists "hit.mark" (db st))
        readSeed st @?= lehmerNext s0

    , testCase "a miss does not apply outs (seed 1, odds 1/2 -> lehmerNext 1 is odd)" $ do
        let s0 = 1
        assertBool "fixture check: lehmerNext 1 mod 2 == 1 (a miss)"
          (odd (lehmerNext s0))
        let st = applyDraw 1 2 [] [Insert "hit.mark"] (seeded s0)
        assertBool "hit.mark absent on a miss" (not (exists "hit.mark" (db st)))
        readSeed st @?= lehmerNext s0
    ]

  , testGroup "loud guards (try/evaluate/ErrorCall)"
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

    , testCase "the usability win: S/S2/S3/R are ordinary variables now, no longer reserved" $ do
        r <- try (evaluate (length (draw 1 2 [Match "flag.S"] [Insert "marked.R"])))
        assertBool "S and R are unremarkable author variables post-v40"
          (not (isLeft (r :: Either ErrorCall Int)))

    , testCase "rngSetup rejects a seed of 0" $ do
        r <- try (evaluate (length (rngSetup 0)))
        assertBool "seed 0 rejected" (isLeft (r :: Either ErrorCall Int))

    , testCase "rngSetup rejects a seed at or above the modulus" $ do
        r <- try (evaluate (length (rngSetup 2147483647)))
        assertBool "seed >= modulus rejected" (isLeft (r :: Either ErrorCall Int))

    , testCase "rngSetup rejects a negative seed" $ do
        r <- try (evaluate (length (rngSetup (-5))))
        assertBool "negative seed rejected" (isLeft (r :: Either ErrorCall Int))
    ]

  , testGroup "SeedlessDraw"
    [ testCase "a world with a draw and no seed is flagged; rngSetup clears it" $ do
        let wSeedless = definePractices [provokeP] emptyState
        assertBool "SeedlessDraw flagged" (any isSeedlessDraw (typeCheck wSeedless))
        let wSeeded = foldl (flip performOutcome) wSeedless (rngSetup 12345)
        assertBool "cleared once seeded" (not (any isSeedlessDraw (typeCheck wSeeded)))
    ]
  ]
