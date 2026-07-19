{-# LANGUAGE LambdaCase #-}
module Prax.ScheduleRuleSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Either (isLeft)
import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (performOutcome, setSchedule, registerEngineRules, roundBoundary, definePractices)
import           Prax.TypeCheck (TypeError (..), typeCheck)
import           Prax.Schedule (gathering)
import           Prax.Worlds.Play (playWorld)

-- A one-rule schedule: "mark" flags every flagged thing every 2 boundaries
-- (the retired DriftSpec markR, now an engine ScheduleRule the boundary fires).
markR :: ScheduleRule
markR = ScheduleRule "mark" 2 [([Match "flag.X"], [Insert "marked.X"])]

-- Install a schedule on a fresh clocked world (emptyState seeds turn!0), then
-- seed the world's flag facts. setSchedule start-sates each rule one period out.
scheduled :: [ScheduleRule] -> [Outcome] -> PraxState
scheduled rules = foldl (flip performOutcome) (setSchedule rules emptyState)

dueOf :: String -> PraxState -> Maybe Int
dueOf name = Map.lookup name . scheduleDues

-- Force a schedule installation to WHNF (enough to hit setSchedule's guards).
rejects :: [ScheduleRule] -> IO Bool
rejects rules = isLeft <$> (try (evaluate (setSchedule rules emptyState))
                              :: IO (Either ErrorCall PraxState))

tests :: TestTree
tests = testGroup "Prax.ScheduleRule"
  [ testCase "a rule does not fire before its due" $ do
      let st = roundBoundary (scheduled [markR] [Insert "flag.a"])   -- turn 1; due at 2
      assertBool "marked.a absent (due seeded at 2, now 1)" (not (exists "marked.a" (db st)))

  , testCase "a rule fires at its due, covering every binding" $ do
      let base = scheduled [markR] [Insert "flag.a", Insert "flag.b"]
          st   = roundBoundary (roundBoundary base)                  -- turn 2 = due
      assertBool "marked.a inserted" (exists "marked.a" (db st))
      assertBool "marked.b inserted" (exists "marked.b" (db st))

  , testCase "the due re-arms period boundaries from now" $ do
      let base = scheduled [markR] [Insert "flag.a"]
          st2  = roundBoundary (roundBoundary base)                  -- turn 2: fires, re-arms
      dueOf "mark" st2 @?= Just 4
      let st3 = roundBoundary st2                                    -- turn 3: not yet due at 4
      dueOf "mark" st3 @?= Just 4
      assertBool "no re-fire before the new due (mark still stands from turn 2)"
        (exists "marked.a" (db st3))

  , testCase "two rules with different periods fire on their own schedules" $ do
      let p2 = ScheduleRule "p2" 2 [([Match "flagA.X"], [Insert "markedA.X"])]
          p3 = ScheduleRule "p3" 3 [([Match "flagB.X"], [Insert "markedB.X"])]
          base = scheduled [p2, p3] [Insert "flagA.a", Insert "flagB.a"]
          step = roundBoundary
          st1 = step base
          st2 = step st1
          st3 = step st2
          st4 = step st3
          st5 = step st4
          st6 = step st5

      -- turn 1: neither due (seeded at 2 / 3).
      dueOf "p2" st1 @?= Just 2
      dueOf "p3" st1 @?= Just 3
      -- turn 2: p2 fires, re-arms to 4; p3 not yet.
      dueOf "p2" st2 @?= Just 4
      dueOf "p3" st2 @?= Just 3
      -- turn 3: p3 fires, re-arms to 6; p2 not yet.
      dueOf "p2" st3 @?= Just 4
      dueOf "p3" st3 @?= Just 6
      -- turn 4: p2 fires, re-arms to 6.
      dueOf "p2" st4 @?= Just 6
      dueOf "p3" st4 @?= Just 6
      -- turn 5: neither due.
      dueOf "p2" st5 @?= Just 6
      dueOf "p3" st5 @?= Just 6
      -- turn 6: both fire, re-arm to 8 and 9.
      dueOf "p2" st6 @?= Just 8
      dueOf "p3" st6 @?= Just 9
      assertBool "markedA.a present" (exists "markedA.a" (db st6))
      assertBool "markedB.a present" (exists "markedB.a" (db st6))

  , testCase "a late fire (clock jumped past the due) re-arms FROM the boundary it fires at" $ do
      -- The clock-jump idiom: jump the clock forward (performOutcome, the
      -- only sanctioned author-free clock write) so the rule is overdue, then
      -- one boundary fires it late and re-arms a period from NOW, not the
      -- stale due.
      let base = performOutcome (Insert "turn!10")
                   (scheduled [markR] [Insert "flag.a"])              -- overdue (due 2, clock 10)
          st   = roundBoundary base                                   -- now 11: fires late
      assertBool "fired late" (exists "marked.a" (db st))
      dueOf "mark" st @?= Just 13                                     -- 11 + period, not 2 + period

  , testCase "duplicate rule names are a loud construction-time error (one due key each)" $
      rejects [ ScheduleRule "same" 2 [], ScheduleRule "same" 3 [] ] >>= assertBool "rejected"

  , testCase "a multi-segment rule name is a loud construction-time error" $
      rejects [ ScheduleRule "a.b" 2 [] ] >>= assertBool "rejected"

  , testCase "a body authoring the Prax namespace is a loud error" $
      rejects [ ScheduleRule "x" 2 [([Match "flag.PraxNow"], [])] ] >>= assertBool "rejected"

  , testCase "a body authoring Actor is a loud error (a schedule rule has no actor)" $
      rejects [ ScheduleRule "x" 2 [([Match "flag.Actor"], [])] ] >>= assertBool "rejected"

  , testCase "the usability win: D/D2/Now are ordinary variables, not reserved" $ do
      ok <- not <$> rejects [ ScheduleRule "x" 2 [([Match "flag.Now"], [Insert "marked.D"])] ]
      assertBool "D and Now are unremarkable author variables" ok

  , testCase "a zero period is a loud error" $
      rejects [ ScheduleRule "x" 0 [([Match "flag.a"], [Insert "marked.a"])] ] >>= assertBool "rejected"

  , testCase "a schedule world is well-formed" $ do
      -- markR's guard reads flag.X, produced by a registered practice's init
      -- (the DriftSpec "flagSeed" pattern) so the v42 dead-condition lint sees
      -- it -- this pins well-formedness of the schedule machinery.
      let flagSeed = practice { practiceId = "flagSeed", initOutcomes = [ Insert "flag.seed" ] }
          st = setSchedule [markR] (definePractices [flagSeed] emptyState)
      assertBool "no type errors" (null (typeCheck st))

  , testGroup "gathering (open fires; the fact expires -- no close rule)"
    [ testCase "opens at period, not before" $ do
        let base = scheduled [ gathering "fair" 3 1 [Insert "marketDay.now"] ] []
            st2  = roundBoundary (roundBoundary base)                 -- turn 2
        assertBool "not open before period" (not (exists "marketDay.now" (db st2)))
        let st3 = roundBoundary st2                                   -- turn 3 == period
        assertBool "opens exactly at period" (exists "marketDay.now" (db st3))

    , testCase "closes at period + duration (the expiry queue tears it down)" $ do
        let base = scheduled [ gathering "fair" 3 1 [Insert "marketDay.now"] ] []
            st3  = roundBoundary (roundBoundary (roundBoundary base)) -- turn 3: open
        assertBool "still open at period" (exists "marketDay.now" (db st3))
        let st4 = roundBoundary st3                                   -- turn 4 == period + duration
        assertBool "closed exactly at period + duration" (not (exists "marketDay.now" (db st4)))

    , testCase "recurs: opens again a full period later" $ do
        let base = scheduled [ gathering "fair" 3 1 [Insert "marketDay.now"] ] []
            drive k st = iterate roundBoundary st !! k
            st6 = drive 6 base                                        -- second open at 2*period
        assertBool "cycle 2 opens at 2 x period" (exists "marketDay.now" (db st6))
        dueOf "fair" st6 @?= Just 9                                   -- re-armed to 6 + period

    , testCase "duration == period is a loud construction-time error" $ do
        r <- try (evaluate (length (show (gathering "fair" 3 3 [Insert "x"]))))
        assertBool "duration == period rejected" (isLeft (r :: Either ErrorCall Int))

    , testCase "duration == 0 is a loud construction-time error" $ do
        r <- try (evaluate (length (show (gathering "fair" 3 0 [Insert "x"]))))
        assertBool "duration == 0 rejected" (isLeft (r :: Either ErrorCall Int))
    ]

  , testGroup "the compiler-level door shares the global rule-name table (v46)"
    [ testCase "registerEngineRules seeds a due exactly like setSchedule (one period out)" $ do
        -- emptyState clocks turn!0, so a period-1 engine rule is due at 1.
        let st = registerEngineRules [ScheduleRule "story" 1 []] emptyState
        dueOf "story" st @?= Just 1

    , testCase "adding an authored 'story' rule to a compiled script world is a loud collision" $ do
        -- playWorld already carries the engine-registered 'story' rule; an
        -- author naming a schedule rule 'story' would share its one due key.
        r <- try (evaluate (length (show (scheduleDues
               (setSchedule [ScheduleRule "story" 2 []] playWorld)))))
        assertBool "rejected: the compiled script already registered 'story'"
          (isLeft (r :: Either ErrorCall Int))

    , testCase "an authored 'story' rule blocks the engine door, and vice versa" $ do
        -- Direction 1: authored first, engine door second.
        let authored = setSchedule [ScheduleRule "story" 2 []] emptyState
        r1 <- try (evaluate (length (show (scheduleDues
                (registerEngineRules [ScheduleRule "story" 1 []] authored)))))
        assertBool "engine door rejected: 'story' already authored"
          (isLeft (r1 :: Either ErrorCall Int))
        -- Direction 2: engine door first, authored second.
        let engineFirst = registerEngineRules [ScheduleRule "story" 1 []] emptyState
        r2 <- try (evaluate (length (show (scheduleDues
                (setSchedule [ScheduleRule "story" 2 []] engineFirst)))))
        assertBool "authoring door rejected: 'story' already registered by the engine"
          (isLeft (r2 :: Either ErrorCall Int))
    ]

  , testGroup "engine-rule provenance exempts the reserved-family scan (v53)"
    [ testCase "registerEngineRules records the rule name; setSchedule does not" $ do
        engineRuleNames (registerEngineRules [ScheduleRule "story" 1 []] emptyState) @?= ["story"]
        engineRuleNames (setSchedule [ScheduleRule "auth" 1 []] emptyState)         @?= []

    , testCase "an authored rule writing a reserved family is flagged; the SAME body through the engine door is exempt" $ do
        -- The provenance pin, made non-vacuous: the rule body EXPLICITLY writes
        -- scenePatience.a.b (which no shipped story rule exercises). Same shape,
        -- opposite verdict — provenance, not the rule's shape, is what differs.
        let body = [([], [Insert "scenePatience.a.b"])]
            authored = setSchedule       [ScheduleRule "story" 1 body] emptyState
            engine   = registerEngineRules [ScheduleRule "story" 1 body] emptyState
        assertBool "authored rule flags ReservedFamily scenePatience"
          (any (\case ReservedFamily "scenePatience" _ _ -> True; _ -> False) (typeCheck authored))
        typeCheck engine @?= []          -- machinery may write reserved families (v45's charter)

    , testCase "a duplicate name through the engine door alone still errors loudly (the record-update forces the guard)" $ do
        -- The laziness question: registerEngineRules now record-updates
        -- engineRuleNames onto addScheduleRules' result. A record update forces
        -- its base to WHNF, so the duplicate-name guard still fires BEFORE any
        -- name is silently recorded. Forcing engineRuleNames alone must error.
        r <- try (evaluate (length (show (engineRuleNames
               (registerEngineRules [ScheduleRule "dup" 1 [], ScheduleRule "dup" 1 []] emptyState)))))
        assertBool "rejected: duplicate engine-door names share one due key"
          (isLeft (r :: Either ErrorCall Int))
    ]
  ]
