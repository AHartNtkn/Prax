module Prax.RelevanceSpec (tests) where

import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Drift (DriftRule (..), driftChar, driftP)
import           Prax.Engine (setDesires, setCharacters, definePractices, relevantDelta, monotoneInsert)
import           Prax.Query (Condition (..), CmpOp (..), CalcOp (..), cookCondition)
import           Prax.Derive (axiom)
import           Prax.Db (pathNames)
import           Prax.Sym (intern)
import           Prax.Types
import           Prax.Worlds.Village (villageWorld)
import           Prax.Relevance

tests :: TestTree
tests = testGroup "Prax.Relevance"
  [ testCase "mayUnify: variables are wildcards, prefixes are compatible" $ do
      assertBool "var vs concrete" (mayUnify "lied.Actor.H.stole.C.loaf"
                                             "lied.eve.dana.stole.carol.loaf")
      assertBool "prefix compatibility (longer insert, shorter pattern)"
        (mayUnify "Hearer.believes.took.Culprit.gem.heard.Actor"
                  "oz.believes.took.kit.gem")
      assertBool "distinct constants do not unify"
        (not (mayUnify "regards.W.carol.thief" "practice.earnBread.Owner.done.S"))

  , testCase "the village table: conscience live, spite and pursuit live" $ do
      let tbl = improvableDesires (practiceDefs villageWorld)
                                  (axioms villageWorld)
                                  (desires villageWorld)
      -- v32: confess's own outcome list Deletes exactly the "lied"-shaped
      -- mark clean-conscience's condition matches (villageP's confessWhisper
      -- now authors that delete) -- a conscience-only believed model CAN now
      -- be improved (predicting a confession relieves it), so the table
      -- correctly flips from the pre-v32 "never improvable" finding.
      --
      -- Performance consequence, recorded honestly rather than papered over:
      -- 'Prax.Planner.predictMove''s v26 pre-filter skips grounding/scoring a
      -- mover's candidates entirely when EVERY desire in the predictor's
      -- believed model is un-improvable (no authored action could possibly
      -- improve it, so predicting is pointless work). Gale's "honest" trait
      -- is presumed by every character from t=0 ('Prax.Persona.transparent'),
      -- so any predictor whose believed model of gale is conscience-only
      -- used to hit that skip for free. Now that clean-conscience (and its
      -- v32 sibling conscience-remembers) are improvable, that skip no
      -- longer fires for her -- every in-scope predictor evaluates her
      -- candidates again, same as any other motivated mind. The cost is
      -- real but bounded (gale's own candidate set, not the whole village);
      -- a measured perf note is Task 3's to land in the LEDGER, not mine to
      -- pre-empt here.
      assertBool "clean-conscience is improvable"
        ("clean-conscience" `elem` tbl)
      -- spites-carol counts DERIVED regards facts (standingUnless's head):
      -- conservatively improvable, so eve's predicted whisper stays live.
      assertBool "spites-carol is improvable" ("spites-carol" `elem` tbl)
      -- pursuit counts base done-facts the stage actions Insert.
      assertBool "pursues-earnBread is improvable"
        ("pursues-earnBread" `elem` tbl)

  , testCase "the state carries the table and setDesires rebuilds it" $ do
      assertBool "villageWorld's field matches the module computation"
        (improvables villageWorld
           == improvableDesires (practiceDefs villageWorld)
                                (axioms villageWorld)
                                (desires villageWorld))
      let st = setDesires [ d | d <- desires villageWorld
                              , desireName d == "spites-carol" ] villageWorld
      assertBool "narrowed vocabulary narrows the table"
        ("pursues-earnBread" `notElem` improvables st)

  , testCase "an exclusion insert counts as evicting ANY sibling on the delete side" $ do
      -- The gem displaces whatever sat in the slot: a negative want on the
      -- stone is improvable ONLY through that eviction, and the victim's name
      -- appears in no outcome — the analysis must not conclude "unimprovable"
      -- from the gem's name alone.
      let shrine = practice
            { practiceId = "shrine", roles = ["R"]
            , actions = [ action "[Actor]: enshrine the gem"
                            [ Match "slot.stone" ]
                            [ Insert "slot!gem" ] ] }
          ds = [ Desire "hates-the-stone" (Want [ Match "slot.stone" ] (-2)) ]
      improvableDesires (Map.fromList [("shrine", shrine)]) [] ds
        @?= ["hates-the-stone"]

  , testCase "eviction covers the WHOLE displaced subtree, not just the shadow's shape" $ do
      -- Two exclusion points: the first eviction clears everything under
      -- altar (arbitrary depth and shape), including branches that diverge
      -- from the insert's own path right after the '!'.
      let temple = practice
            { practiceId = "temple", roles = ["R"]
            , actions = [ action "[Actor]: rededicate the altar"
                            [ Match "shrine.here" ]
                            [ Insert "altar!new.rite!dawn" ] ] }
          ds = [ Desire "mourns-the-relic"
                   (Want [ Match "altar.old.relic.jade" ] (-2)) ]
      improvableDesires (Map.fromList [("temple", temple)]) [] ds
        @?= ["mourns-the-relic"]

  , testCase "delta relevance against the village's axioms" $ do
      assertBool "movement commutes with closure (fast path)"
        (not (relevantDelta "practice.world.world.at.bob!square" villageWorld))
      assertBool "a witness deposit is relevant (standingUnless reads believes)"
        (relevantDelta "you.believes.stole.bob.loaf.seen" villageWorld)
      assertBool "an atonement is relevant (it defeats standing)"
        (relevantDelta "atoned.bob" villageWorld)
      assertBool "the stall's stock is not"
        (not (relevantDelta "stall.loaf" villageWorld))

  , testCase "monotone-insert classification against the village" $ do
      assertBool "the village's axioms admit the continuation tier"
        (contMonotone villageWorld)
      assertBool "a witness deposit grows monotonically"
        (monotoneInsert "you.believes.stole.bob.loaf.seen" villageWorld)
      assertBool "atonement defeats standing: full reclose"
        (not (monotoneInsert "atoned.bob" villageWorld))
      assertBool "an exclusion insert never takes the continuation"
        (not (monotoneInsert "practice.world.world.at.bob!square" villageWorld))

  , testCase "livenessOf: a negative desire is FloorCheck unconditionally" $ do
      let ds = [ Desire "hates-mud" (Want [ Match "muddy.Owner" ] (-3)) ]
      livenessOf Map.empty [] ds @?= Map.fromList [ ("hates-mud", FloorCheck) ]

  , testCase "livenessOf: a weight-0 desire is AlwaysLive (defensive; screened statically first)" $ do
      let ds = [ Desire "indifferent" (Want [ Match "whatever.Owner" ] 0) ]
      livenessOf Map.empty [] ds @?= Map.fromList [ ("indifferent", AlwaysLive) ]

  , testCase "livenessOf: a positive desire with a ticker-only conjunct gates on it alone" $ do
      -- The only action in this world inserts meal.*, never hungry.* -- so
      -- "hungry.Owner" is environment-gated (no authored outcome can raise
      -- it) while "meal.M" is action-insertable and so is NOT a gate.
      let bakery = practice
            { practiceId = "bakery", roles = ["R"]
            , actions = [ action "[Actor]: bake"
                            [ Match "practice.bakery.here" ]
                            [ Insert "meal.bread" ] ] }
          ds = [ Desire "pursues-lunch"
                   (Want [ Match "hungry.Owner", Match "meal.M" ] 5) ]
          tbl = livenessOf (Map.fromList [("bakery", bakery)]) [] ds
      tbl @?= Map.fromList
        [ ("pursues-lunch", GateCheck [ [ cookCondition (Match "hungry.Owner") ] ]) ]

  , testCase "livenessOf: an axiom-derivable candidate gate never qualifies (conservative)" $ do
      -- "hungry.Owner" is never Inserted, but an axiom's head unifies it, so
      -- it is conservatively excluded from gating and no other conjunct
      -- qualifies -- the whole want stays AlwaysLive.
      let ax = axiom [ Match "starving.Owner" ] [ "hungry.Owner" ]
          ds = [ Desire "pursues-food" (Want [ Match "hungry.Owner" ] 5) ]
      livenessOf Map.empty [ax] ds @?= Map.fromList [ ("pursues-food", AlwaysLive) ]

  , testCase "livenessOf: a Subquery-bearing want is AlwaysLive (uncertainty always wins)" $ do
      let ds = [ Desire "counts-friends"
                   (Want [ Subquery "Fs" ["F"] [ Match "friend.Owner.F" ] ] 5) ]
      livenessOf Map.empty [] ds @?= Map.fromList [ ("counts-friends", AlwaysLive) ]

  , testCase "the village's liveness field: floors for consciences, classes for the rest" $ do
      let tbl = liveness villageWorld
      tbl Map.! "clean-conscience" @?= FloorCheck
      tbl Map.! "conscience-remembers" @?= FloorCheck
      -- pursues-earnBread's condition is a done-fact every stage action
      -- inserts (practice.earnBread.Owner.done.S) -- action-insertable, so
      -- no conjunct qualifies as a gate.
      tbl Map.! "pursues-earnBread" @?= AlwaysLive
      -- spites-carol's condition (regards.W.carol.thief) is standingUnless's
      -- own axiom head -- conservatively excluded from gating.
      tbl Map.! "spites-carol" @?= AlwaysLive
      -- punishes-whisper's top-level conjuncts are an Or (never a gate
      -- candidate) and a belief-Match that expose's own outcome inserts --
      -- action-insertable, so again no qualifying conjunct remains.
      tbl Map.! "punishes-whisper" @?= AlwaysLive
      -- v36: suffers-hunger is a negative desire (-22) -- FloorCheck
      -- unconditionally, same as the consciences above, so a sated bob's
      -- pair-skip against it fires between meals.
      tbl Map.! "suffers-hunger" @?= FloorCheck
      -- v37: drawn-to-market's first conjunct (marketDay.square) is
      -- clock-moved only -- no authored action ever inserts it, so it
      -- qualifies as the sole gate; the second conjunct
      -- (practice.world.world.at.Owner!square) is action-insertable ("Go to
      -- [Place]" inserts exactly this shape) and so never qualifies --
      -- confirmed, not assumed, against the actual computed table.
      tbl Map.! "drawn-to-market"
        @?= GateCheck [ [ cookCondition (Match "marketDay.square") ] ]
      assertBool "the field matches the module computation"
        (liveness villageWorld
           == livenessOf (practiceDefs villageWorld)
                         (axioms villageWorld)
                         (desires villageWorld))

  , testCase "cookedReadAnchors walks every polarity, including subquery internals" $ do
      let conds = map cookCondition
            [ Match "a.X", Not "b.X"
            , Subquery "S" ["W"] [ Match "c.W.deed", Cmp Gte "N" "2" ]
            , Count "N" "S", Calc "M" Add "N" "1", Eq "X" "y"
            , Or [ [ Match "d.X" ], [ Absent [ Match "e.X" ] ] ]
            ]
          anchors = cookedReadAnchors conds
          want p = map intern (pathNames p) `elem` anchors
      assertBool "a.X read"        (want "a.X")
      assertBool "b.X (Not) read"  (want "b.X")
      assertBool "subquery inner read" (want "c.W.deed")
      assertBool "Or branch read"  (want "d.X")
      assertBool "Absent-in-Or read" (want "e.X")
      length anchors @?= 5

  , testCase "moverReadAnchors: scope, believes, death, affordances, desires — grounded to the pair" $ do
      let p = practice
            { practiceId = "eatery", roles = ["R"]
            , actions = [ action "[Actor]: eat"
                            [ Match "hungry.Actor" ]
                            [ ForEach [ Match "crumb.C" ] [ Delete "crumb.C" ]
                            , Insert "meal.Actor" ]
                        , action "[Actor]: clean up" [] [ Call "tidy" ["Actor"] ]
                        ]
            , functions =
                [ Function "tidy" ["Who"]
                    [ FnCase [] [ ForEach [ Match "dish.D" ] [ Delete "dish.D" ] ] ] ]
            }
          vocab = [ Desire "wants-food" (Want [ Match "hungry.Owner" ] 5) ]
          priya = character "priya"
          beth' = character "beth"
          st = setDesires vocab
                 (setCharacters [priya, beth'] (definePractices [p] emptyState))
          anchors = moverReadAnchors st priya beth'
          has s = map intern (pathNames s) `elem` anchors
      assertBool "believes family, actor+mover grounded"
        (has "priya.believes.desires.beth.PraxD")
      assertBool "death mark" (has "dead.beth")
      assertBool "affordance condition, Actor:=beth" (has "hungry.beth")
      assertBool "ForEach guard read" (has "crumb.C")
      assertBool "function-body ForEach guard read" (has "dish.D")
      assertBool "desire condition, Owner:=beth" (has "hungry.beth")
      assertBool "NOT grounded to the predictor" (not (has "hungry.priya"))

  , testCase "clock-moved facts are environment gates (the ticker is not an author)" $ do
      -- "festive.now" is inserted ONLY by a drift pulse; the desire needs it
      -- plus an action-reachable conjunct. Pre-v37 the drifter's outcomes
      -- polluted the insert pool and this classified AlwaysLive; it is a
      -- GateCheck on the festive conjunct.
      let p = practice
            { practiceId = "plaza", roles = ["R"]
            , actions = [ action "[Actor]: stroll the plaza"
                            [ Match "practice.plaza.here" ]
                            [ Insert "strolled.Actor" ] ]
            }
          pulse = DriftRule "festival" 4
            [ ( [], [ Insert "festive.now" ] ) ]
          vocab = [ Desire "loves-a-crowd"
                      (Want [ Match "festive.now", Match "strolled.Owner" ] 3) ]
          st = setDesires vocab
                 (setCharacters [character "ana", driftChar]
                    (definePractices [p, driftP [pulse]] emptyState))
      Map.lookup "loves-a-crowd" (liveness st)
        @?= Just (GateCheck [[cookCondition (Match "festive.now")]])

  , testCase "action-insertable facts still never gate; the clock cannot launder them" $ do
      -- Same shape, but a PERSON action also inserts festive.now (lighting
      -- the lanterns) -- the pool sees it via that authored outcome
      -- regardless of the drift exclusion, so no conjunct qualifies:
      -- AlwaysLive (conservative as ever).
      let p = practice
            { practiceId = "plaza", roles = ["R"]
            , actions = [ action "[Actor]: stroll the plaza"
                            [ Match "practice.plaza.here" ]
                            [ Insert "strolled.Actor" ]
                        , action "[Actor]: light the lanterns"
                            [ Match "practice.plaza.here" ]
                            [ Insert "festive.now" ]
                        ]
            }
          pulse = DriftRule "festival" 4
            [ ( [], [ Insert "festive.now" ] ) ]
          vocab = [ Desire "loves-a-crowd"
                      (Want [ Match "festive.now", Match "strolled.Owner" ] 3) ]
          st = setDesires vocab
                 (setCharacters [character "ana", driftChar]
                    (definePractices [p, driftP [pulse]] emptyState))
      Map.lookup "loves-a-crowd" (liveness st) @?= Just AlwaysLive

  , testCase "the village hunger want-shape regains its gate under the reclassification" $ do
      -- v33's eatery shape: eating only inserts meal.Actor, never
      -- hungry.itself; ONLY the drift pulse inserts hungry.* (the v36
      -- shape). Pre-v37 this classified AlwaysLive (the drifter's insert
      -- polluted the pool the moment it joined the world, the silent
      -- regression the spec diagnoses); post-v37 the clock-moved
      -- hungry.Owner regains its GateCheck.
      let eatery = practice
            { practiceId = "eatery", roles = ["R"]
            , actions = [ action "[Actor]: eat"
                            [ Match "hungry.Actor" ]
                            [ Insert "meal.Actor" ] ] }
          pulse = DriftRule "hunger" 3
            [ ( [ Match "appetite.X", Not "hungry.X" ], [ Insert "hungry.X" ] ) ]
          vocab = [ Desire "wants-food"
                      (Want [ Match "hungry.Owner", Match "meal.M" ] 5) ]
          st = setDesires vocab
                 (setCharacters [character "bob", driftChar]
                    (definePractices [eatery, driftP [pulse]] emptyState))
      Map.lookup "wants-food" (liveness st)
        @?= Just (GateCheck [[cookCondition (Match "hungry.Owner")]])
  ]
