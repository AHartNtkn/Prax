-- | Stress-testing and coverage (Versu's stress-test tool — "runs hundreds of
-- instances of the game … with all characters controlled by the computer … we
-- are able to find bugs and anomalies quickly" — plus Prompter's scene-coverage
-- reporting).
--
-- 'stressTest' plays many seeded games in which /every/ character takes a
-- uniformly-random available action each turn, and reports which endings were
-- reached, which action ids ever fired (coverage), and how many runs hit a
-- dead end (a living character with no move). Pure and deterministic given the
-- seeds — uses a tiny built-in LCG so there is no extra dependency.
module Prax.Stress
  ( RunResult(..)
  , StressReport(..)
  , runRandom
  , stressTest
  ) where

import           Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import           Data.Maybe (listToMaybe)
import           Data.Set (Set)
import qualified Data.Set as Set
import           Data.Word (Word64)

import           Prax.Db (unify, valToString)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction)
import           Prax.Loop (advance)

-- A minimal linear-congruential PRNG (Knuth's MMIX constants).
lcg :: Word64 -> Word64
lcg x = 6364136223846793005 * x + 1442695040888963407

-- A uniform index in @[0, n)@ and the next seed (n must be > 0).
pick :: Int -> Word64 -> (Int, Word64)
pick n s = let s' = lcg s in (fromIntegral (s' `mod` fromIntegral n), s')

-- The ending reached, if any (an @ending.\<key\>@ fact).
endingReached :: PraxState -> Maybe String
endingReached st =
  listToMaybe [ e | b <- unify "ending.E" (db st) Map.empty
                  , Just e <- [valToString <$> Map.lookup "E" b] ]

-- The active scene, if any (a @currentScene!\<id\>@ fact from a "Prax.Script"
-- world). Non-scene worlds have none, so scene coverage is simply empty there.
sceneReached :: PraxState -> Maybe String
sceneReached st =
  listToMaybe [ s | b <- unify "currentScene.S" (db st) Map.empty
                  , Just s <- [valToString <$> Map.lookup "S" b] ]

-- | The result of one random play.
data RunResult = RunResult
  { runEnding  :: Maybe String   -- ^ the ending reached, if any
  , runActions :: Set String     -- ^ ids of actions performed
  , runScenes  :: Set String     -- ^ scenes visited (empty for non-scene worlds)
  , runDeadEnd :: Bool           -- ^ a living character had no available action
  , runTurns   :: Int
  } deriving (Eq, Show)

-- | Play one game for up to @cap@ turns: each turn the next living character
-- performs a uniformly-random available action, stopping at an ending, the turn
-- cap, no one left alive, or a true dead end (a full round in which no living
-- character has any move). A character with no action simply passes.
runRandom :: Int -> Word64 -> PraxState -> RunResult
runRandom cap seed st0 = go cap seed Set.empty (visit Set.empty st0) 0 0 st0
  where
    -- Record the active scene (if any) of a visited state.
    visit scs st = maybe scs (`Set.insert` scs) (sceneReached st)
    -- passes = how many living characters in a row have had nothing to do
    go 0 _ acc scs n _ st = RunResult (endingReached st) acc scs False n
    go k s acc scs n passes st =
      case endingReached st of
        Just e -> RunResult (Just e) acc scs False n
        Nothing
          | null living                 -> RunResult Nothing acc scs False n
          | passes >= length living     -> RunResult Nothing acc scs True  n  -- everyone stuck
          | otherwise ->
              let (actor, st1) = advance st
                  acts = possibleActions st1 (charName actor)
              in case acts of
                   [] -> go k s acc (visit scs st1) n (passes + 1) st1        -- idle: pass, don't spend a turn
                   _  -> let (i, s') = pick (length acts) s
                             ga = acts !! i
                             st2 = performAction st1 ga
                         in go (k - 1) s' (Set.insert (gaActionId ga) acc)
                               (visit scs st2) (n + 1) 0 st2
      where living = livingCharacters st

-- | Aggregated report over many runs.
data StressReport = StressReport
  { srRuns     :: Int
  , srEndings  :: Map String Int   -- ^ ending → how many runs reached it
  , srCoverage :: Set String       -- ^ every action id that fired in any run
  , srScenes   :: Map String Int   -- ^ scene → how many runs visited it
  , srDeadEnds :: Int              -- ^ runs that hit a dead end
  , srNoEnding :: Int              -- ^ runs that hit the turn cap with no ending
  } deriving (Eq, Show)

-- | Run @runs@ seeded random games of up to @cap@ turns and aggregate the report.
stressTest :: Int -> Int -> PraxState -> StressReport
stressTest runs cap st0 =
  foldl' tally (StressReport runs Map.empty Set.empty Map.empty 0 0)
    [ runRandom cap (seedFor i) st0 | i <- [1 .. runs] ]
  where
    seedFor i = fromIntegral i * 2654435761   -- spread the seeds apart
    tally r res =
      r { srEndings  = maybe (srEndings r)
                             (\e -> Map.insertWith (+) e 1 (srEndings r))
                             (runEnding res)
        , srCoverage = Set.union (srCoverage r) (runActions res)
        , srScenes   = foldr (\s -> Map.insertWith (+) s 1)
                             (srScenes r) (Set.toList (runScenes res))
        , srDeadEnds = srDeadEnds r + fromEnum (runDeadEnd res)
        , srNoEnding = srNoEnding r
                         + fromEnum (runEnding res == Nothing && not (runDeadEnd res))
        }
