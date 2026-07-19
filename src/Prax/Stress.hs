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
--
-- Coverage over a single-valued fact family (a @\<family\>!\<id\>@ or
-- @\<family\>.\<id\>@ path — e.g. a "Prax.Script" world's @currentScene@) is
-- an optional, DECLARED parameter (@Maybe String@): the caller names the
-- family it wants tracked, or @Nothing@ to skip tracking. There is no
-- default family — @currentScene@ is not privileged by this module, only by
-- the CLI's own callers, which happen to name it.
--
-- STATED LIMIT of the dead-end detector (v46): the idle-pass counter tolerates
-- exactly ONE round boundary of move-less progression — a scene that advances
-- only via the engine schedule across TWO OR MORE boundaries (e.g. a beat-less
-- scene whose sole exit is @after n@ with n ≥ 2) reports a spurious dead end.
-- No shipped world has that shape (every scene offers a character beat); drive
-- such a world with 'Prax.Loop.runNpcTicks' instead, which has no detector.
module Prax.Stress
  ( RunResult(..)
  , StressReport(..)
  , runRandom
  , stressTest
  ) where

import           Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import           Data.Maybe (isNothing, listToMaybe)
import           Data.Set (Set)
import qualified Data.Set as Set
import           Data.Word (Word64)

import           Prax.Db (unify, valToString)
import           Prax.Sym (intern)
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
                  , Just e <- [valToString <$> Map.lookup (intern "E") b] ]

-- | The current member of a coverage @family@, if any (a @\<family\>.\<id\>@
-- path — e.g. a "Prax.Script" world's @currentScene!\<id\>@ fact). Worlds
-- with no fact in the family simply report nothing, so a caller can name any
-- family without checking first whether the world under test populates it.
familyReached :: String -> PraxState -> Maybe String
familyReached family st =
  listToMaybe [ s | b <- unify (family ++ ".S") (db st) Map.empty
                  , Just s <- [valToString <$> Map.lookup (intern "S") b] ]

-- | The result of one random play.
data RunResult = RunResult
  { runEnding  :: Maybe String   -- ^ the ending reached, if any
  , runActions :: Set String     -- ^ ids of actions performed
  , runVisited  :: Set String     -- ^ coverage-family members visited (empty
                                  --   if no family was named, or the world
                                  --   populates none of it)
  , runDeadEnd :: Bool           -- ^ a living character had no available action
  , runTurns   :: Int
  } deriving (Eq, Show)

-- | Play one game for up to @cap@ turns: each turn the next living character
-- performs a uniformly-random available action, stopping at an ending, the turn
-- cap, no one left alive, or a true dead end (a full round in which no living
-- character has any move). A character with no action simply passes.
-- @mFamily@ is the optional coverage family to track (see 'familyReached').
runRandom :: Int -> Word64 -> Maybe String -> PraxState -> RunResult
runRandom cap seed mFamily st0 = go cap seed Set.empty (visit Set.empty st0) 0 0 st0
  where
    -- Record the visited state's current member of the tracked family, if any.
    visit scs st = maybe scs (`Set.insert` scs) (mFamily >>= (`familyReached` st))
    -- passes = how many living characters in a row have had nothing to do.
    -- 'advance' fires the engine's round boundary once per rotation, on the
    -- wrap call (the first call after every living character has had a
    -- turn) — so a full rotation of idle passes (passes == length living)
    -- does NOT by itself prove the boundary has had its say: whichever idle
    -- streak starts right at the top of a fresh run (cursor == -1) needs one
    -- MORE call beyond that rotation before the wrap fires (advance's own
    -- wrap condition is "i <= cursor", and cursor starts at -1, one below
    -- every valid index). So only declare a dead end once passes exceeds a
    -- full rotation — that extra call guarantees a wrap, hence a boundary
    -- firing, occurred within the idle streak and still changed nothing.
    go 0 _ acc scs n _ st = RunResult (endingReached st) acc scs False n
    go k s acc scs n passes st =
      case endingReached st of
        Just e -> RunResult (Just e) acc scs False n
        Nothing
          | null living                 -> RunResult Nothing acc scs False n
          | passes > length living      -> RunResult Nothing acc scs True  n  -- everyone stuck, boundary included
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
  , srVisited   :: Map String Int   -- ^ coverage-family member -> how many runs visited it
  , srDeadEnds :: Int              -- ^ runs that hit a dead end
  , srNoEnding :: Int              -- ^ runs that hit the turn cap with no ending
  } deriving (Eq, Show)

-- | Run @runs@ seeded random games of up to @cap@ turns and aggregate the
-- report. @mFamily@ is the optional coverage family to track (see
-- 'familyReached'); the CLI's own callers pass the current-scene family
-- (@Just 'Prax.Script.currentScenePath'@).
stressTest :: Int -> Int -> Maybe String -> PraxState -> StressReport
stressTest runs cap mFamily st0 =
  foldl' tally (StressReport runs Map.empty Set.empty Map.empty 0 0)
    [ runRandom cap (seedFor i) mFamily st0 | i <- [1 .. runs] ]
  where
    seedFor i = fromIntegral i * 2654435761   -- spread the seeds apart
    tally r res =
      r { srEndings  = maybe (srEndings r)
                             (\e -> Map.insertWith (+) e 1 (srEndings r))
                             (runEnding res)
        , srCoverage = Set.union (srCoverage r) (runActions res)
        , srVisited   = foldr (\s -> Map.insertWith (+) s 1)
                             (srVisited r) (Set.toList (runVisited res))
        , srDeadEnds = srDeadEnds r + fromEnum (runDeadEnd res)
        , srNoEnding = srNoEnding r
                         + fromEnum (isNothing (runEnding res) && not (runDeadEnd res))
        }
