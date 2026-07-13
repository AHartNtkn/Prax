module Prax.RelevanceSpec (tests) where

import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Engine (setDesires, relevantDelta, monotoneInsert)
import           Prax.Query (Condition (..))
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
  ]
