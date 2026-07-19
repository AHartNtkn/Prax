{-# LANGUAGE LambdaCase #-}
module Prax.TypeCheckSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Types
import           Prax.Engine (definePractices, defineFunctions, performOutcome, setAxioms, setDesires, setCharacters, seedDie, setSchedule)
import           Prax.Query (Condition (..), CmpOp (..))
import           Prax.Derive (Axiom (..), axiom)
import           Prax.Deontic (obligedClose)
import           Prax.Rng (draw)
import           Prax.TypeCheck
import qualified Prax.Worlds.Bar as Bar
import qualified Prax.Worlds.Intrigue as Intrigue
import qualified Prax.Worlds.Play as Play
import qualified Prax.Worlds.Feud as Feud
import qualified Prax.Worlds.Village as Village
import qualified Prax.Worlds.Audience as Audience

-- A one-practice world for the seeded-bug cases.
world1 :: Practice -> PraxState
world1 p = definePractices [p] emptyState

runOutcomes :: PraxState -> [Outcome] -> PraxState
runOutcomes = foldl (flip performOutcome)

isSortConflict :: TypeError -> Bool
isSortConflict SortConflict{} = True
isSortConflict _              = False

tests :: TestTree
tests = testGroup "Prax.TypeCheck"
  [ testCase "every shipped world is well-formed" $ do
      typeCheck Bar.barWorld          @?= []
      typeCheck Bar.barDirectorWorld  @?= []
      typeCheck Intrigue.intrigueWorld @?= []
      typeCheck Play.playWorld        @?= []
      typeCheck Feud.feudWorld        @?= []
        -- feud wires Prax.Kin.kinAxioms wholesale; the fixture's own haddock
        -- documents the un-derivable remainder (no parent.* base fact exists
        -- until a wedding inserts one) as deliberate — axiom bodies are out
        -- of the dead-condition lint's scope, so this stays clean.
      typeCheck Village.villageWorld  @?= []
        -- village's drawn-to-market desire reads marketDay.square, which
        -- only the market schedule rule inserts — the pin holds because
        -- 'producibleAtoms' folds the schedule surface, unlike the planner's
        -- mover-only worldAtomPools.
      typeCheck Audience.audienceWorld @?= []

  , testCase "an outcome variable bound by nothing is caught" $ do
      let p = practice
            { practiceId = "bug", roles = ["R"]
            , actions = [ action "[Actor]: x" [] [ Insert "foo.Ghost" ] ] }
      assertBool "UnboundVar Ghost"
        (any (\case UnboundVar _ "Ghost" _ -> True; _ -> False) (typeCheck (world1 p)))

  , testCase "an axiom head variable absent from the body is caught" $ do
      let w = setAxioms [ Axiom [ Match "p.X" ] [ "q.X.Y" ] ] emptyState
      assertBool "UnboundVar Y"
        (any (\case UnboundVar "axiom" "Y" _ -> True; _ -> False) (typeCheck w))

  , testCase "a relation used as both ! and . is caught" $ do
      let p = practice
            { practiceId = "c"
            , actions = [ action "[Actor]: a" [] [ Insert "a.mood!happy" ]
                        , action "[Actor]: b" [] [ Insert "a.mood.sad" ] ] }
      assertBool "clash on a.mood" (CardinalityClash "a.mood" `elem` typeCheck (world1 p))

  , testCase "a Call to an undefined function is caught" $ do
      let p = practice
            { practiceId = "d", roles = ["R"]
            , actions = [ action "[Actor]: y" [] [ Call "nope" ["R"] ] ] }
      assertBool "UndefinedRef nope"
        (any (\case UndefinedRef _ "nope" -> True; _ -> False) (typeCheck (world1 p)))

  , testCase "spawning an undefined practice is caught" $ do
      let p = practice
            { practiceId = "e", roles = ["R"]
            , actions = [ action "[Actor]: z" [] [ Insert "practice.ghost.R" ] ] }
      assertBool "UndefinedRef practice.ghost"
        (any (\case UndefinedRef _ "practice.ghost" -> True; _ -> False) (typeCheck (world1 p)))

  , testCase "an unbound variable in a registered function's case is caught, sited at fn <name>" $ do
      -- The function's case outcome uses Ghost, bound by neither its params
      -- nor its (empty) conditions -- caught by the registry-level walk, whose
      -- site label drops the phantom practice prefix (spec v47).
      let f = Function "grant" ["P"] [ FnCase [] [ Insert "gift.Ghost" ] ]
          w = defineFunctions [f] emptyState
      assertBool "UnboundVar Ghost sited at \"fn grant\""
        (any (\case UnboundVar "fn grant" "Ghost" _ -> True; _ -> False) (typeCheck w))

  , testCase "a correct little practice is well-formed" $ do
      let p = practice
            { practiceId = "ok", roles = ["R"]
            , initOutcomes = [ Insert "here.someone" ]
            , actions = [ action "[Actor]: greet [R]"
                            [ Match "here.Actor", Match "here.R" ]
                            [ Insert "greeted.Actor.R" ] ] }
      typeCheck (world1 p) @?= []

    -- ML-style sort inference (only active when sorts are declared) -----------
  , testCase "no sort declarations ⇒ no sort errors" $ do
      let p = practice { practiceId = "z"
            , actions = [ action "[Actor]: a" [] [ Insert "cup.beer", Insert "cup.bar" ] ] }
      typeCheck (world1 p) @?= []                    -- with sorts=[] this is fine

  , testCase "a position given values of two sorts is caught" $ do
      let p = practice { practiceId = "menu"
            , actions = [ action "[Actor]: pour a beer" [] [ Insert "cup.beer" ]
                        , action "[Actor]: pour a bar!" [] [ Insert "cup.bar" ] ] }
          w = (world1 p) { sorts = [ ("beverage", ["beer"]), ("place", ["bar"]) ] }
      assertBool "SortConflict on cup"
        (any (\case SortConflict "cup" _ -> True; _ -> False) (typeCheck w))

  , testCase "a variable used in two different sorts is caught" $ do
      let p = practice { practiceId = "v", roles = ["X"]
            , actions = [ action "[Actor]: mix" [] [ Insert "cup.X", Insert "spot.X" ] ] }
          w = runOutcomes ((world1 p) { sorts = [ ("beverage", ["beer"]), ("place", ["bar"]) ] })
                          [ Insert "cup.beer", Insert "spot.bar" ]
      assertBool "SortConflict from X" (any isSortConflict (typeCheck w))

  , testCase "a constant declared in two sorts is caught" $ do
      let w = emptyState { sorts = [ ("agent", ["x"]), ("beverage", ["x"]) ] }
      assertBool "SortConflict on x"
        (any (\case SortConflict "x" d -> "agent" `elem` words' d && "beverage" `elem` words' d
                    _                  -> False) (typeCheck w))

    -- ForEach support --------------------------------------------------------
  , testCase "a variable bound by ForEach conditions is not unbound" $ do
      let p = practice
            { practiceId = "w", roles = ["R"]
            , initOutcomes = [ Insert "member.someone" ]
            , actions = [ action "[Actor]: broadcast" []
                            [ ForEach [ Match "member.X" ] [ Insert "told.X" ] ] ] }
      typeCheck (world1 p) @?= []

  , testCase "a genuinely unbound variable inside ForEach is flagged" $ do
      let p = practice
            { practiceId = "w", roles = ["R"]
            , actions = [ action "[Actor]: broadcast" []
                            [ ForEach [ Match "member.X" ] [ Insert "told.Ghost" ] ] ] }
      assertBool "UnboundVar Ghost"
        (any (\case UnboundVar _ "Ghost" _ -> True; _ -> False) (typeCheck (world1 p)))

  , testCase "ForEach sub-inserts join the cardinality corpus" $ do
      -- The same relation asserted '!' at top level and '.' inside a ForEach must clash.
      let p = practice
            { practiceId = "w", roles = ["R"]
            , actions = [ action "[Actor]: a" [] [ Insert "mark.R!x" ]
                        , action "[Actor]: b" []
                            [ ForEach [ Match "member.X" ] [ Insert "mark.X.y" ] ] ] }
      assertBool "CardinalityClash detected"
        (any (\case CardinalityClash {} -> True; _ -> False) (typeCheck (world1 p)))

  , testCase "a dangling Call or spawn inside ForEach is caught" $ do
      let p = practice
            { practiceId = "w", roles = ["R"]
            , actions = [ action "[Actor]: broadcast" []
                            [ ForEach [ Match "member.X" ]
                                      [ Call "nope" ["X"]
                                      , Insert "practice.ghost.X" ] ] ] }
      assertBool "UndefinedRef nope"
        (any (\case UndefinedRef _ "nope" -> True; _ -> False) (typeCheck (world1 p)))
      assertBool "UndefinedRef practice.ghost"
        (any (\case UndefinedRef _ "practice.ghost" -> True; _ -> False) (typeCheck (world1 p)))

    -- Dead-condition lint -----------------------------------------------------
  , testCase "a dead action conjunct (typo'd predicate) is caught" $ do
      let p = practice
            { practiceId = "hunt"
            , initOutcomes = [ Insert "treasure.spot" ]
            , actions = [ action "[Actor]: dig" [ Match "tresure.spot" ] [ Insert "dug.Actor" ] ] }
      typeCheck (world1 p) @?= [ DeadCondition "hunt / [Actor]: dig" "tresure.spot" ]

  , testCase "the corrected twin of the typo is well-formed" $ do
      let p = practice
            { practiceId = "hunt"
            , initOutcomes = [ Insert "treasure.spot" ]
            , actions = [ action "[Actor]: dig" [ Match "treasure.spot" ] [ Insert "dug.Actor" ] ] }
      typeCheck (world1 p) @?= []

  , testCase "a dead positive inside Exists is caught" $ do
      let p = practice
            { practiceId = "hunt"
            , initOutcomes = [ Insert "treasure.spot" ]
            , actions = [ action "[Actor]: dig"
                            [ Exists [ Match "tresure.spot" ] ] [ Insert "dug.Actor" ] ] }
      typeCheck (world1 p) @?= [ DeadCondition "hunt / [Actor]: dig" "tresure.spot" ]

  , testCase "a dead ForEach guard is caught, sited as an effect guard" $ do
      let p = practice
            { practiceId = "hunt"
            , initOutcomes = [ Insert "treasure.spot" ]
            , actions = [ action "[Actor]: search" []
                            [ ForEach [ Match "tresure.spot" ] [ Insert "found" ] ] ] }
      typeCheck (world1 p) @?= [ DeadCondition "hunt / [Actor]: search (effect guard)" "tresure.spot" ]

  , testCase "a dead desire and a dead character want are each caught" $ do
      let desireW = Desire "wantGold" (Want [ Match "ghost.family" ] 5)
          vic = (character "vic") { charWants = [ Want [ Match "ghost.spirit" ] 3 ] }
          w = setCharacters [vic] (setDesires [desireW] emptyState)
      typeCheck w @?=
        [ DeadCondition "desire wantGold" "ghost.family"
        , DeadCondition "want of vic" "ghost.spirit" ]

  , testCase "a negation over a never-produced family is not flagged" $ do
      let p = practice
            { practiceId = "spookless"
            , actions = [ action "[Actor]: peek" [ Not "ghost.Actor" ] [ Insert "peeked.Actor" ] ] }
      typeCheck (world1 p) @?= []

  , testCase "a half-dead Or clause is not flagged" $ do
      let p = practice
            { practiceId = "hunt"
            , initOutcomes = [ Insert "treasure.spot" ]
            , actions = [ action "[Actor]: dig"
                            [ Or [ [ Match "tresure.spot" ], [ Match "treasure.spot" ] ] ]
                            [ Insert "dug.Actor" ] ] }
      typeCheck (world1 p) @?= []

  , testCase "a dead pattern inside a Subquery interior is not flagged" $ do
      let p = practice
            { practiceId = "hunt"
            , actions = [ action "[Actor]: check"
                            [ Subquery "S" [] [ Match "tresure.spot" ]
                            , Count "N" "S"
                            , Cmp Lte "N" "0" ]
                            [ Insert "checked.Actor" ] ] }
      typeCheck (world1 p) @?= []

  , testCase "a fully unanchored pattern (every segment a variable) is not flagged" $ do
      let p = practice
            { practiceId = "hunt", roles = ["X", "Y"]
            , actions = [ action "[Actor]: link" [ Match "X.Y" ] [ Insert "linked.X.Y" ] ] }
      typeCheck (world1 p) @?= []

  , testCase "a wild world (undefined Call) silences the lint, not the ref check" $ do
      let p = practice
            { practiceId = "hunt"
            , actions = [ action "[Actor]: dig" [ Match "tresure.spot" ] [ Call "missingFn" [] ] ] }
          es = typeCheck (world1 p)
      assertBool "UndefinedRef missingFn fires"
        (any (\case UndefinedRef _ "missingFn" -> True; _ -> False) es)
      assertBool "no DeadCondition"
        (not (any (\case DeadCondition {} -> True; _ -> False) es))

  , testCase "a dead axiom body is not flagged (axiom bodies are out of scope)" $ do
      let w = setAxioms [ axiom [ Match "parent.P.C" ] [ "kin.P.C" ] ] emptyState
      typeCheck w @?= []

    -- Protected families (v45, generalized from the v44 clock-write guard) ---
  , testCase "an authored write to the engine clock is flagged" $ do
      let p = practice
            { practiceId = "clocksmith"
            , actions = [ action "[Actor]: forge time" [] [ Insert "turn!99" ] ] }
      assertBool "ReservedFamily turn on the authored turn insert"
        (any (\case ReservedFamily "turn" _ "turn!99" -> True; _ -> False) (typeCheck (world1 p)))

  , testCase "an axiom head deriving the clock family is flagged" $ do
      let w = setAxioms [ Axiom [ Match "ping.X" ] [ "turn!5" ] ] emptyState
      assertBool "ReservedFamily turn on the axiom head"
        (any (\case ReservedFamily "turn" "axiom" "turn!5" -> True; _ -> False) (typeCheck w))

  , testCase "a performOutcome clock-jump is NOT flagged (typeCheck sees no authored write)" $ do
      -- Fixtures jump the clock through performOutcome, which touches no
      -- authored definition -- so a well-formed world with a jumped clock in
      -- its db stays clean.
      let ok = practice
            { practiceId = "ok"
            , actions = [ action "[Actor]: wait" [] [] ] }
          jumped = performOutcome (Insert "turn!42") (world1 ok)
      typeCheck jumped @?= []

  , testCase "an authored Delete of turn is flagged (the strengthening)" $ do
      let p = practice
            { practiceId = "clocksmith2"
            , actions = [ action "[Actor]: erase time" [] [ Delete "turn" ] ] }
      assertBool "ReservedFamily turn on the authored turn delete"
        (any (\case ReservedFamily "turn" _ "turn" -> True; _ -> False) (typeCheck (world1 p)))

  , testCase "SeedlessDraw: an unseeded world with a draw is flagged" $ do
      let p = practice
            { practiceId = "gambler"
            , actions = [ action "[Actor]: roll" [] (draw 1 2 [] [ Insert "hit.Actor" ]) ] }
      assertBool "SeedlessDraw flagged for an unseeded draw"
        (any (\case SeedlessDraw -> True; _ -> False) (typeCheck (world1 p)))

  , testCase "SeedlessDraw: seedDie clears it" $ do
      let p = practice
            { practiceId = "gambler"
            , actions = [ action "[Actor]: roll" [] (draw 1 2 [] [ Insert "hit.Actor" ]) ] }
      assertBool "cleared once the die is seeded"
        (not (any (\case SeedlessDraw -> True; _ -> False)
                  (typeCheck (seedDie 7 (world1 p)))))

  , testCase "SeedlessDraw: a draw nested under a ForEach is still found" $ do
      let p = practice
            { practiceId = "gambler2"
            , actions = [ action "[Actor]: roll all" []
                            [ ForEach [ Match "here.X" ] (draw 1 2 [] [ Insert "hit.X" ]) ] ] }
      assertBool "SeedlessDraw flagged through a ForEach"
        (any (\case SeedlessDraw -> True; _ -> False) (typeCheck (world1 p)))

  , testCase "SeedlessDraw: a draw in a schedule rule body is found (v50 T1 review M2)" $ do
      let r = ScheduleRule
                { srName = "storms"
                , srPeriod = 3
                , srBody = [ ([], draw 1 2 [] [ Insert "storm.here" ]) ] }
      assertBool "SeedlessDraw flagged for an unseeded schedule-rule draw"
        (any (\case SeedlessDraw -> True; _ -> False)
             (typeCheck (setSchedule [r] (world1 practice))))

  , testCase "an authored Insert of contradiction is flagged" $ do
      let p = practice
            { practiceId = "sophist"
            , actions = [ action "[Actor]: break logic" [] [ Insert "contradiction" ] ] }
      assertBool "ReservedFamily contradiction on the authored insert"
        (any (\case ReservedFamily "contradiction" _ "contradiction" -> True; _ -> False)
             (typeCheck (world1 p)))

  , testCase "an authored Match on contradiction is clean (reads free)" $ do
      let p = practice
            { practiceId = "sophist2"
            , actions = [ action "[Actor]: check logic" [ Match "contradiction" ] [] ] }
      typeCheck (world1 p) @?= []

    -- The literal-tailed compiler families (v53): scenePatience + currentScene --
  , testCase "an authored practice-action Insert of scenePatience is flagged" $ do
      let p = practice
            { practiceId = "meddler"
            , actions = [ action "[Actor]: forge patience" [] [ Insert "scenePatience.x.y" ] ] }
      assertBool "ReservedFamily scenePatience on the authored insert"
        (any (\case ReservedFamily "scenePatience" _ "scenePatience.x.y" -> True; _ -> False)
             (typeCheck (world1 p)))

  , testCase "a reserved write NESTED under a ForEach is still flagged (v53 final review M-2)" $ do
      let p = practice
            { practiceId = "meddler9"
            , initOutcomes = [ Insert "mark.here" ]
            , actions = [ action "[Actor]: forge patience for everyone" []
                            [ ForEach [ Match "mark.M" ]
                                [ Insert "scenePatience.n.M" ] ] ] }
      assertBool "ReservedFamily scenePatience through the ForEach recursion"
        (any (\case ReservedFamily "scenePatience" _ "scenePatience.n.M" -> True; _ -> False)
             (typeCheck (world1 p)))

  , testCase "an authored InsertFor of scenePatience in a practice init is flagged" $ do
      let p = practice
            { practiceId = "meddler2"
            , initOutcomes = [ InsertFor 3 "scenePatience.a.b" ] }
      assertBool "ReservedFamily scenePatience on the init InsertFor"
        (any (\case ReservedFamily "scenePatience" _ "scenePatience.a.b" -> True; _ -> False)
             (typeCheck (world1 p)))

  , testCase "an authored function-case write of scenePatience is flagged" $ do
      let f = Function "meddle" [] [ FnCase [] [ Insert "scenePatience.f.g" ] ]
          w = defineFunctions [f] emptyState
      assertBool "ReservedFamily scenePatience sited at the function"
        (any (\case ReservedFamily "scenePatience" _ "scenePatience.f.g" -> True; _ -> False)
             (typeCheck w))

  , testCase "an authored setSchedule rule body writing scenePatience is flagged" $ do
      let r = ScheduleRule "meddle" 2 [([], [Insert "scenePatience.s.t"])]
          w = setSchedule [r] emptyState
      assertBool "ReservedFamily scenePatience on the authored schedule rule"
        (any (\case ReservedFamily "scenePatience" _ "scenePatience.s.t" -> True; _ -> False)
             (typeCheck w))

  , testCase "an axiom head deriving scenePatience is flagged" $ do
      let w = setAxioms [ Axiom [ Match "trigger.X" ] [ "scenePatience.X.j" ] ] emptyState
      assertBool "ReservedFamily scenePatience on the axiom head"
        (any (\case ReservedFamily "scenePatience" "axiom" "scenePatience.X.j" -> True; _ -> False)
             (typeCheck w))

  , testCase "an authored Delete of scenePatience is flagged (a delete is a write)" $ do
      let p = practice
            { practiceId = "wrecker"
            , actions = [ action "[Actor]: cancel patience" [] [ Delete "scenePatience.x.y" ] ] }
      assertBool "ReservedFamily scenePatience on the authored delete"
        (any (\case ReservedFamily "scenePatience" _ "scenePatience.x.y" -> True; _ -> False)
             (typeCheck (world1 p)))

  , testCase "an authored write to the currentScene family is flagged" $ do
      let p = practice
            { practiceId = "director"
            , actions = [ action "[Actor]: seize the stage" [] [ Insert "currentScene!banquet" ] ] }
      assertBool "ReservedFamily currentScene on the authored insert"
        (any (\case ReservedFamily "currentScene" _ "currentScene!banquet" -> True; _ -> False)
             (typeCheck (world1 p)))

  , testCase "a practice injected onto compiled Audience writing its patience marker flags loudly (the v50 door)" $ do
      -- The exact mixed-layer door v50 left open: 'Prax.Script.compile''s guard
      -- cannot see a practice injected through the raw 'definePractices' door
      -- onto an already-compiled world. Audience's timed 'dismissed' junction
      -- keys scenePatience.audience.dismissed; a raw write to it silently
      -- refreshes a live timeout. The provenance-scoped reserved scan now
      -- catches it (Audience's own 'story' rule stays exempt, so the world is
      -- clean until this practice is added).
      let meddler = practice
            { practiceId = "meddler"
            , actions = [ action "[Actor]: reset the clock" []
                            [ Insert "scenePatience.audience.dismissed" ] ] }
          w = definePractices [meddler] Audience.audienceWorld
      assertBool "ReservedFamily scenePatience on the injected practice"
        (any (\case ReservedFamily "scenePatience" _ "scenePatience.audience.dismissed" -> True; _ -> False)
             (typeCheck w))

  , testCase "a sightedWithin-shaped authored condition is clean (turn reads free)" $ do
      let p = practice
            { practiceId = "watcher"
              -- a producer for the believes.atSince family, so Check 7 (dead
              -- conditions) does not itself flag the read below -- unrelated
              -- to the family guard under test here.
            , initOutcomes = [ Insert "carol.believes.atSince.bob!3" ]
            , actions =
                [ action "[Actor]: recall sighting"
                    [ Match "Actor.believes.atSince.Witness!Since"
                    , Match "turn!Now" ]
                    [] ] }
      typeCheck (world1 p) @?= []

    -- DeonticUnclosed: a world that can invoke obligation declares its closure --
  , testCase "a world that can produce obliged.* but omits its □-closure is flagged, naming the axiom" $ do
      -- census-true (the practice inserts an obliged.* fact) with a liftable
      -- domain rule whose lifted twin is absent: DEON property 1 would silently
      -- fail. The flag names the rule's first head and points at obligedClose.
      let w = setAxioms [ axiom [ Match "a.X" ] [ "b.X" ] ]
                        (definePractices [obligeProducer] emptyState)
      typeCheck w @?= [ DeonticUnclosed "b.X" ]

  , testCase "the same world declared via obligedClose is well-formed" $ do
      let w = setAxioms (obligedClose [ axiom [ Match "a.X" ] [ "b.X" ] ])
                        (definePractices [obligeProducer] emptyState)
      typeCheck w @?= []

  , testCase "an axiomless world that can invoke obligation is clean (nothing to close)" $ do
      -- the Bar shape: census-true but no axioms, so there is no unclosed rule
      typeCheck (definePractices [obligeProducer] emptyState) @?= []

  , testCase "a world with liftable axioms that CANNOT invoke obligation is clean (the Feud shape)" $ do
      -- census-false: no obliged.* producer anywhere, so its axioms owe no
      -- closure (Feud's own withheld lifted rows are the shipping example)
      typeCheck (setAxioms [ axiom [ Match "a.X" ] [ "b.X" ] ] emptyState) @?= []

  , testCase "partial closure flags each missing twin individually (v51 final review M2)" $ do
      -- two liftable rules, only the first one's twin declared: the check is
      -- per-axiom, so exactly the second rule flags — an all-or-nothing
      -- regression (any twin present ⇒ clean) would let it slip through.
      let a1 = axiom [ Match "a.X" ] [ "b.X" ]
          a2 = axiom [ Match "c.X" ] [ "d.X" ]
          w  = setAxioms (obligedClose [a1] ++ [a2])
                         (definePractices [obligeProducer] emptyState)
      typeCheck w @?= [ DeonticUnclosed "d.X" ]
  ]
  where
    words' = words . map (\c -> if c == ',' then ' ' else c)
    -- A practice whose action produces an obliged.* fact (a census-true world).
    obligeProducer = practice
      { practiceId = "oblige", roles = ["R"]
      , actions = [ action "[Actor]: swear a duty" [] [ Insert "obliged.Actor.duty" ] ] }
