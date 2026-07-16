-- | The engine schedule's inert core (spec
-- @docs/specs/2026-07-16-v44-the-schedule.md@): the exact-path expiry queue's
-- supersession/purge/eviction laws and the round-boundary function's ordering
-- and re-arm laws, each driven directly on a tiny fixture world (no loop —
-- Task 2 wires 'roundBoundary' into the rotation).
module Prax.ScheduleSpec (tests) where

import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Cooked (cookScheduleRule)
import           Prax.Db (exists)
import           Prax.Engine (currentTurn, performOutcome, roundBoundary)
import           Prax.Query (Condition (..))
import           Prax.Types

-- A bare fixture: 'emptyState' already carries the seeded clock (turn!0).
base :: PraxState
base = emptyState

-- Install a schedule (both the authored declarations, for period lookup, and
-- their cooked mirror, which 'roundBoundary' fires) — the two fields a Task 2
-- setter will maintain together.
withSchedule :: [ScheduleRule] -> PraxState -> PraxState
withSchedule rules st =
  st { schedule = rules, cookedSchedule = map cookScheduleRule rules }

-- Seed a rule's next-due boundary.
dueAt :: String -> Int -> PraxState -> PraxState
dueAt name turn st = st { scheduleDues = Map.insert name turn (scheduleDues st) }

has :: String -> PraxState -> Bool
has s st = exists s (db st)

tests :: TestTree
tests = testGroup "Prax.Schedule"
  [ testGroup "the expiry queue"
    [ testCase "law 1: an InsertFor fact holds for n boundaries and is gone at the nth" $ do
        let s0 = performOutcome (InsertFor 2 "mood!a") base   -- due at turn 0+2 = 2
            s1 = roundBoundary s0                              -- now 1: not yet due
            s2 = roundBoundary s1                              -- now 2: due, retracted
        currentTurn s0 @?= 0
        assertBool "present at insert"        (has "mood.a" s0)
        assertBool "present after 1 boundary" (has "mood.a" s1)
        assertBool "gone at the 2nd boundary" (not (has "mood.a" s2))
        Map.null (expiries s2) @?= True

    , testCase "law 2: re-InsertFor before due refreshes — survives the old due, dies at the new" $ do
        let s0  = performOutcome (InsertFor 2 "mood!a") base   -- due at 2
            s1  = roundBoundary s0                             -- now 1: holds
            s1' = performOutcome (InsertFor 2 "mood!a") s1     -- refresh: due at 1+2 = 3
            s2  = roundBoundary s1'                            -- now 2: past the OLD due, still holds
            s3  = roundBoundary s2                             -- now 3: the new due, retracted
        assertBool "survives past the old due (turn 2)" (has "mood.a" s2)
        assertBool "dies at the refreshed due (turn 3)" (not (has "mood.a" s3))

    , testCase "law 3: a bare re-insert cancels the timer — the fact never expires" $ do
        let s0  = performOutcome (InsertFor 2 "mood!a") base
            s0' = performOutcome (Insert "mood!a") s0          -- bare: cancels the expiry
            sN  = iterate roundBoundary s0' !! 5
        Map.null (expiries s0') @?= True
        assertBool "still standing after five boundaries" (has "mood.a" sN)

    , testCase "law 4: an authored delete purges the subtree's timers — no later ghost retract" $ do
        let s0  = performOutcome (InsertFor 2 "feels.anger.toward.bob") base
            s0' = performOutcome (Delete "feels.anger") s0     -- purges the pending timer
            s1  = performOutcome (Insert "feels.anger.toward.bob") s0'  -- re-assert PERMANENTLY
            sN  = iterate roundBoundary s1 !! 5
        Map.null (expiries s0') @?= True
        assertBool "the re-asserted fact is untouched by any stale timer"
          (has "feels.anger.toward.bob" sN)

    , testCase "law 5: a !-eviction drops silently — the displaced timer fires on nothing" $ do
        let s0  = performOutcome (InsertFor 2 "mood!a") base   -- due at 2
            s0' = performOutcome (Insert "mood!b") s0          -- excludes: mood.a evicted, timer stays queued
            s1  = roundBoundary s0'                            -- now 1
            s2  = roundBoundary s1                             -- now 2: mood!a due, but its fact is gone
        assertBool "the displaced value is already gone" (not (has "mood.a" s0'))
        assertBool "the surviving value still stands"    (has "mood.b" s2)
        assertBool "no ghost retract touched mood.b"     (has "mood.b" s2)
        Map.null (expiries s2) @?= True

    , testCase "law 6: a lifetime on an interior path takes its descendants at expiry" $ do
        let s0  = performOutcome (InsertFor 2 "feels.anger") base
            s0' = performOutcome (Insert "feels.anger.toward.bob") s0
            s2  = roundBoundary (roundBoundary s0')            -- now 2: interior retract
        assertBool "the interior fact is gone"  (not (has "feels.anger" s2))
        assertBool "its descendant went with it" (not (has "feels.anger.toward.bob" s2))
    ]

  , testGroup "the round boundary"
    [ testCase "law 7 (ghost observation): expiries fire BEFORE rules — a period-1 rule does not see an expiring fact" $ do
        let sighting = ScheduleRule "sighting" 1
              [([Match "mood!Now"], [Insert "sighted.Now"])]
            -- Control: lifetime 2, so at boundary 1 the fact still stands and the rule stamps it.
            ctl  = dueAt "sighting" 1 (withSchedule [sighting]
                     (performOutcome (InsertFor 2 "mood!a") base))
            ctl1 = roundBoundary ctl
            -- Ghost: lifetime 1, so the fact expires AT boundary 1 — before the rule runs.
            ghost  = dueAt "sighting" 1 (withSchedule [sighting]
                       (performOutcome (InsertFor 1 "mood!a") base))
            ghost1 = roundBoundary ghost
        assertBool "control: the rule stamps a still-present fact" (has "sighted.a" ctl1)
        assertBool "ghost: the fact expired this boundary"         (not (has "mood.a" ghost1))
        assertBool "ghost: the rule never saw it (no stamp)"       (not (has "sighted.a" ghost1))

    , testCase "law 8a: a due rule re-arms period boundaries FROM NOW (fires at 1 and 3, not 2)" $ do
        let beat = ScheduleRule "beat" 2 [([Match "turn!Now"], [Insert "beat.Now"])]
            s0 = dueAt "beat" 1 (withSchedule [beat] base)
            s3 = iterate roundBoundary s0 !! 3                  -- boundaries 1, 2, 3
        assertBool "fired at boundary 1"       (has "beat.1" s3)
        assertBool "skipped boundary 2"        (not (has "beat.2" s3))
        assertBool "fired again at boundary 3" (has "beat.3" s3)

    , testCase "law 8b: due rules fire in declaration order within a boundary" $ do
        let open = ScheduleRule "open" 1 [([], [Insert "gate.open"])]
            pass = ScheduleRule "pass" 1 [([Match "gate.open"], [Insert "passed.here"])]
            -- [open, pass]: open runs first, so pass sees the gate the same boundary.
            s0 = dueAt "pass" 1 (dueAt "open" 1 (withSchedule [open, pass] base))
            s1 = roundBoundary s0
        assertBool "the second rule saw the first's effect" (has "passed.here" s1)

    , testCase "the clock advances one per boundary" $ do
        let s3 = iterate roundBoundary base !! 3
        currentTurn s3 @?= 3
    ]
  ]
