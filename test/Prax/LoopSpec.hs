module Prax.LoopSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, (@?=), assertBool)

import           Data.List (isInfixOf)

import           Prax.Db (dbToSentences)
import           Prax.Query
import           Prax.Types
import           Prax.Engine
import           Prax.Loop (runNpcTicks, npcAct)
import           Prax.Planner (pickAction)
import           Prax.Worlds.Bar (barWorld)

-- Deterministic narration from driving every character with the planner (depth
-- 2) for 25 round-robin turns (idle turns, and the silent sight ticker's turns,
-- produce no line). 25, not 20: the bar's cast now includes the bodiless sight
-- ticker (Prax.Sight), so each round is 5 turns, not 4 — the same 5 rounds this
-- golden trace always covered (20 = 5x4) now take 25 (5x5). A golden replay of
-- the whole emergent arc: greet, serve, respond, take offense at a snub, buy a
-- friend a drink, and then — once the room is warm — the director steps in and
-- turns two friends against each other, after which they cool. ('you' has no
-- wants, so it paces; in the CLI 'you' is the human.)
expectedTrace :: [String]
expectedTrace =
  [ "you: Go to bar"
  , "ada: Greet you"
  , "bex: Go to bar"
  , "you: Go to entrance"
  , "ada: Greet bex"
  , "bex: Order beer"
  , "you: Go to bar"
  , "ada: Fulfill bex's order"
  , "bex: Greet ada back"
  , "you: Go to entrance"
  , "ada: Take offense that you ignored your greeting"
  , "bex: Buy ada a drink"
  , "director: turn ada against bex to stir up the evening"
  , "you: Go to bar"
  , "ada: Wait a moment"
  , "bex: settle in, feeling you belong here"
  ]

tests :: TestTree
tests = testGroup "Prax.Loop"
  [ testCase "25-turn NPC replay matches the golden narration" $
      fst (runNpcTicks 2 25 barWorld) @?= expectedTrace

  , testCase "the emergent + director-driven outcomes hold after the replay" $ do
      let facts = dbToSentences (db (snd (runNpcTicks 2 25 barWorld)))
          has f = assertBool f (f `elem` facts)
      -- bex responded to ada's greeting via the reaction
      has "practice.greet.world.greeted.bex.ada"
      -- the player's ignored greeting left ada with a grievance
      has "practice.greet.world.grievance.ada.you"
      -- the director intervened once, injecting a rivalry between the two friends
      has "dm.stirred"
      has "practice.greet.world.grievance.ada.bex"
      -- bex's arc reaches belonging (its own warmth held even as the director
      -- soured ada toward it); no NPC ever chose the against-desires transformation
      has "bex.arc.belonging"
      assertBool "no NPC resigned to solitude"
        ("bex.arc.lonely" `notElem` facts && "you.arc.lonely" `notElem` facts)

  , testCase "a dead character is skipped in turn-taking" $ do
      -- mark bex dead; over a full run bex must never act again
      let dead = performOutcome (Insert "dead.bex") barWorld
          (tr, _) = runNpcTicks 2 16 dead
      assertBool "bex takes no turns once dead" (not (any ("bex:" `isInfixOf`) tr))

  , testCase "a quiet character acts their standing intention — even when fresh deliberation would differ" $ do
      -- priya's goad pays off unless beth retaliates; beth's believed
      -- vengefulness is gated on a grudge FACT. Establish priya's intention
      -- (goad) while beth is harmless; then the grudge lands through an
      -- EXTERNAL event priya has not processed: none of priya's four
      -- signature components move (her options are unconditional, her want
      -- reads slapped.priya which is still absent, she has no own vocabulary
      -- desires, and she already believed beth vengeful before the grudge).
      -- Fresh deliberation now prefers waiting (goad -> predicted slap costs
      -- her more than the goad gains); the standing intention goads anyway —
      -- the spec's accepted one-beat lag, pinned as INTENDED.
      let p = practice
            { practiceId = "spat", roles = ["R"]
            , actions =
                [ action "[Actor]: goad beth" [ Neq "Actor" "beth" ]
                    [ Insert "goaded.beth" ]
                , action "[Actor]: slap priya"
                    [ Match "grudge.Actor", Match "goaded.Actor" ]
                    [ Insert "slapped.priya" ]
                , action "[Actor]: wait about" [] []
                ]
            }
          vocab = [ Desire "vengeful"
                      (Want [ Match "grudge.Owner", Match "slapped.priya" ] 8) ]
          priya = (character "priya")
            { charWants = [ Want [ Match "goaded.beth" ] 5
                          , Want [ Match "slapped.priya" ] (-20) ] }
          beth' = character "beth"
          st0 = setDesires vocab
                  (setCharacters [priya, beth'] (definePractices [p] emptyState))
          st1 = performOutcome (Insert "practice.spat.here") st0
          st  = performOutcome
                  (Insert "priya.believes.desires.beth.vengeful.heard.gossip") st1
      -- First turn: no standing intention -- deliberates, goads (beth
      -- harmless: no grudge, prediction Nothing), intention stored.
      let (a1, stA) = npcAct 2 priya st
      fmap gaLabel a1 @?= Just "priya: goad beth"
      -- Rewind the goad itself but keep the stored intention: rebuild the
      -- pre-goad state and graft the intentions map (the test needs the
      -- external event to be the ONLY difference).
      let stKept = st { intentions = intentions stA }
          grudged = performOutcome (Insert "grudge.beth") stKept
      -- Fresh deliberation WOULD now wait (goad invites a -20 slap):
      fmap gaLabel (pickAction 2 grudged priya) @?= Just "priya: wait about"
      -- ...but priya is quiet: all four components unchanged -- she goads.
      let (a2, _) = npcAct 2 priya grudged
      fmap gaLabel a2 @?= Just "priya: goad beth"

  , testCase "each trigger reconsiders: options, satisfaction, live drive, learned motive" $ do
      -- Four minimal worlds, one per component; in each: establish a standing
      -- intention, move ONLY that component, npcAct must deliberate afresh
      -- (observable: the pick changes to the newly-correct action).
      -- IMPLEMENTER CONTRACT: each arm is SELF-VERIFYING by construction --
      -- it asserts (a) the standing pick by label, (b) the post-change
      -- npcAct pick by label, AND (c) that (b) differs from (a).

      -- OPTIONS: beth's standing intention is idle (only unconditional action
      -- grounds); an external hunger fact grounds the eat candidate, which
      -- beth then prefers (a new option appears -> re-deliberate).
      let optP = practice
            { practiceId = "mess", roles = ["R"]
            , actions =
                [ action "[Actor]: eat lunch" [ Match "hungry.Actor" ]
                    [ Insert "meal.Actor" ]
                , action "[Actor]: idle about" [] []
                ]
            }
          optBeth = (character "beth")
            { charWants = [ Want [ Match "meal.beth" ] 10 ] }
          optSt = performOutcome (Insert "practice.mess.here")
                    (setCharacters [optBeth] (definePractices [optP] emptyState))
      let (o1, optA) = npcAct 2 optBeth optSt
          optKept = optSt { intentions = intentions optA }
          optHungry = performOutcome (Insert "hungry.beth") optKept
          (o2, _) = npcAct 2 optBeth optHungry
      fmap gaLabel o1 @?= Just "beth: idle about"       -- (a) standing
      fmap gaLabel o2 @?= Just "beth: eat lunch"        -- (b) post-change
      assertBool "options: pick changed"
        (fmap gaLabel o1 /= fmap gaLabel o2)            -- (c)

      -- SATISFACTION: beth dislikes crumbs (a negative want); with no crumbs
      -- she idles. An external crumb moves her satisfaction count 0->1; she
      -- re-deliberates and sweeps (which removes the crumb she now dislikes).
      let satP = practice
            { practiceId = "chores", roles = ["R"]
            , actions =
                [ action "[Actor]: sweep" [ Match "crumbs.C" ] [ Delete "crumbs.C" ]
                , action "[Actor]: idle about" [] []
                ]
            }
          satBeth = (character "beth")
            { charWants = [ Want [ Match "crumbs.C" ] (-2) ] }
          satSt = performOutcome (Insert "practice.chores.here")
                    (setCharacters [satBeth] (definePractices [satP] emptyState))
      let (s1, satA) = npcAct 2 satBeth satSt
          satKept = satSt { intentions = intentions satA }
          satCrumb = performOutcome (Insert "crumbs.floor") satKept
          (s2, _) = npcAct 2 satBeth satCrumb
      fmap gaLabel s1 @?= Just "beth: idle about"       -- (a) standing
      fmap gaLabel s2 @?= Just "beth: sweep"            -- (b) post-change
      assertBool "satisfaction: pick changed"
        (fmap gaLabel s1 /= fmap gaLabel s2)            -- (c)

      -- LIVE DRIVE: beth holds an own vocabulary desire (wants-food) whose
      -- gate fact (hungry.beth) appears ONLY in the want, never in an action
      -- condition -- so the gate flip moves the live-desire set alone, leaving
      -- her options unchanged (eat is gated on the practice fact, present
      -- throughout). With the want unsatisfiable she dawdles (label tie);
      -- once hungry, eating satisfies it and she re-deliberates to eat.
      let drvP = practice
            { practiceId = "diner", roles = ["R"]
            , actions =
                [ action "[Actor]: eat lunch" [ Match "practice.diner.here" ]
                    [ Insert "meal.Actor" ]
                , action "[Actor]: dawdle" [] []
                ]
            }
          drvVocab = [ Desire "wants-food"
                         (Want [ Match "hungry.Owner", Match "meal.M" ] 5) ]
          drvBeth = (character "beth") { charDesires = ["wants-food"] }
          drvSt = performOutcome (Insert "practice.diner.here")
                    (setDesires drvVocab
                       (setCharacters [drvBeth] (definePractices [drvP] emptyState)))
      let (d1, drvA) = npcAct 2 drvBeth drvSt
          drvKept = drvSt { intentions = intentions drvA }
          drvHungry = performOutcome (Insert "hungry.beth") drvKept
          (d2, _) = npcAct 2 drvBeth drvHungry
      fmap gaLabel d1 @?= Just "beth: dawdle"           -- (a) standing
      fmap gaLabel d2 @?= Just "beth: eat lunch"        -- (b) post-change
      assertBool "live drive: pick changed"
        (fmap gaLabel d1 /= fmap gaLabel d2)            -- (c)

      -- LEARNED MOTIVE: beth wants carl fed (meal.carl). Feeding carl makes
      -- him hungry; a hungry carl is predicted to eat ONLY if beth believes
      -- he wants food. Without the belief, feeding gains nothing and beth
      -- bides (the do-nothing sorts before "feed carl" at the 0 tie). An
      -- external belief fact moves the known-motive set alone; she
      -- re-deliberates and feeds, banking carl's predicted meal.
      let lrnP = practice
            { practiceId = "table", roles = ["R"]
            , actions =
                [ action "[Actor]: feed carl" [ Neq "Actor" "carl" ]
                    [ Insert "hungry.carl" ]
                , action "[Actor]: eat lunch" [ Match "hungry.Actor" ]
                    [ Insert "meal.Actor" ]
                , action "[Actor]: bide time" [] []
                ]
            }
          lrnVocab = [ Desire "wants-food"
                         (Want [ Match "hungry.Owner", Match "meal.Owner" ] 5) ]
          lrnBeth = (character "beth")
            { charWants = [ Want [ Match "meal.carl" ] 10 ] }
          lrnCarl = character "carl"
          lrnSt = performOutcome (Insert "practice.table.here")
                    (setDesires lrnVocab
                       (setCharacters [lrnBeth, lrnCarl] (definePractices [lrnP] emptyState)))
      let (l1, lrnA) = npcAct 2 lrnBeth lrnSt
          lrnKept = lrnSt { intentions = intentions lrnA }
          lrnTold = performOutcome
                      (Insert "beth.believes.desires.carl.wants-food.heard.gossip") lrnKept
          (l2, _) = npcAct 2 lrnBeth lrnTold
      fmap gaLabel l1 @?= Just "beth: bide time"        -- (a) standing
      fmap gaLabel l2 @?= Just "beth: feed carl"        -- (b) post-change
      assertBool "learned motive: pick changed"
        (fmap gaLabel l1 /= fmap gaLabel l2)            -- (c)

  , testCase "a NON-bearing template appearing does not reconsider (irrelevant comings and goings)" $ do
      -- "amble" has no outcomes: it bears on nothing beth wants, so its
      -- appearance moves no signature component. Fresh deliberation WOULD
      -- switch to it (a 0-0 tie broken by label: amble < idle) -- the
      -- standing intention holds instead. This is the want-bearing filter's
      -- defining divergence, pinned as INTENDED.
      let p' = practice
            { practiceId = "lull", roles = ["R"]
            , actions =
                [ action "[Actor]: amble over" [ Match "gate.open" ] []
                , action "[Actor]: idle about" [] []
                ]
            }
          beth' = (character "beth")
            { charWants = [ Want [ Match "meal.beth" ] 10 ] }
          st0 = performOutcome (Insert "practice.lull.here")
                  (setCharacters [beth'] (definePractices [p'] emptyState))
      let (q1, stQ) = npcAct 2 beth' st0
          opened = performOutcome (Insert "gate.open") (st0 { intentions = intentions stQ })
      fmap gaLabel q1 @?= Just "beth: idle about"
      fmap gaLabel (pickAction 2 opened beth') @?= Just "beth: amble over"
      let (q2, _) = npcAct 2 beth' opened
      fmap gaLabel q2 @?= Just "beth: idle about"

  , testCase "a standing action that is no longer offered forces re-deliberation" $ do
      -- beth's standing pick is the non-bearing "amble over" (label
      -- tie-break); its grounding fact is then retracted externally. Every
      -- signature component is unchanged (amble bears on nothing), but the
      -- standing action left the candidate set -- npcAct must deliberate,
      -- not perform a vanished action.
      let p' = practice
            { practiceId = "lull2", roles = ["R"]
            , actions =
                [ action "[Actor]: amble over" [ Match "roomy.here" ] []
                , action "[Actor]: idle about" [] []
                ]
            }
          beth' = (character "beth")
            { charWants = [ Want [ Match "meal.beth" ] 10 ] }
          st0 = performOutcome (Insert "roomy.here")
                  (performOutcome (Insert "practice.lull2.here")
                     (setCharacters [beth'] (definePractices [p'] emptyState)))
      let (g1, stG) = npcAct 2 beth' st0
          gone = performOutcome (Delete "roomy.here") (st0 { intentions = intentions stG })
      fmap gaLabel g1 @?= Just "beth: amble over"
      let (g2, _) = npcAct 2 beth' gone
      fmap gaLabel g2 @?= Just "beth: idle about"
  ]
