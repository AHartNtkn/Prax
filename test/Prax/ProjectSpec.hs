module Prax.ProjectSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Either (isLeft)
import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, possibleActions, performAction, setDesires)
import           Prax.Planner (pickAction, predictMove)
import           Prax.Project

-- The tale: mia builds an oven, stage by stage; pat looks on.
ovenStages :: [Stage]
ovenStages =
  [ Stage "[Actor]: sweep the hearth"  [] []
  , Stage "[Actor]: fetch the clay"    [ Match "clay.available" ]
                                       [ Insert "carrying.Owner.clay" ]
  , Stage "[Actor]: shape the oven"    [ Match "carrying.Owner.clay" ]
                                       [ Delete "carrying.Owner.clay" ]
  , Stage "[Actor]: fire it"           [] [ Insert "oven.standing" ]
  ]

ovenTake :: Action
ovenP    :: Practice
ovenPursuit :: Desire
(ovenTake, ovenP, ovenPursuit) =
  endeavor "oven" 3 "[Actor]: resolve to build an oven" [] ovenStages

world :: PraxState
world = foldl (flip performOutcome) base setup
  where
    base = setDesires [ ovenPursuit ]
             ((definePractices [ovenP, yardP] emptyState)
                { characters =
                    [ (character "mia")
                        { charDesires = ["pursues-oven"] }
                    , character "pat" ] })
    yardP = practice { practiceId = "yard", roles = ["R"]
                     , actions = [ ovenTake
                                 , action "[Actor]: Idle about" [] [] ] }
    setup = [ Insert "practice.yard.here", Insert "clay.available" ]

doAct :: String -> String -> PraxState -> PraxState
doAct who needle st =
  case [ ga | ga <- possibleActions st who, needle `isInfixOf` gaLabel ga ] of
    (ga : _) -> performAction st ga
    []       -> error ("no action for " ++ who ++ " matching " ++ show needle
                       ++ "; had: " ++ show (map gaLabel (possibleActions st who)))

offered :: String -> String -> PraxState -> Bool
offered who needle st =
  any ((needle `isInfixOf`) . gaLabel) (possibleActions st who)

mia :: Character
mia = (character "mia") { charDesires = ["pursues-oven"] }

tests :: TestTree
tests = testGroup "Prax.Project"
  [ testCase "undertaking spawns the instance and seeds stage 0" $ do
      let st = doAct "mia" "resolve to build" world
      assertBool "instance exists" (exists "practice.oven.mia" (db st))
      assertBool "stage seeded"    (exists "practice.oven.mia.stage!0" (db st))
      assertBool "undertaking twice is not offered"
        (not (offered "mia" "resolve to build" st))

  , testCase "stages fire in order, for the owner only, gated by needs" $ do
      let st0 = doAct "mia" "resolve to build" world
      assertBool "stage 1 offered, stage 2 not yet"
        (offered "mia" "sweep the hearth" st0 && not (offered "mia" "fetch the clay" st0))
      assertBool "pat cannot work mia's project" (not (offered "pat" "sweep the hearth" st0))
      let st1 = doAct "mia" "sweep the hearth" st0
      assertBool "stage 2 now offered" (offered "mia" "fetch the clay" st1)
      -- a need gates: remove the clay and stage 2 vanishes
      let dry = performOutcome (Delete "clay.available") st1
      assertBool "no clay, no fetching" (not (offered "mia" "fetch the clay" dry))

  , testCase "yields fire and done-facts accumulate" $ do
      let st = foldl (flip (doAct "mia"))
                 world [ "resolve to build", "sweep the hearth", "fetch the clay"
                       , "shape the oven", "fire it" ]
      assertBool "the oven stands" (exists "oven.standing" (db st))
      assertBool "clay consumed"   (not (exists "carrying.mia.clay" (db st)))
      mapM_ (\k -> assertBool ("done s" ++ show k)
                     (exists ("practice.oven.mia.done.s" ++ show k) (db st)))
            [1 .. 4 :: Int]

  , testCase "the pursuit desire: exact shape, and dormant without an instance" $ do
      ovenPursuit @?= Desire "pursues-oven"
                        (Want [ Match "practice.oven.Owner.done.S" ] 3)
      -- dormant: pat believes mia pursues it, but with no instance the
      -- believed model gains nothing from any move — no prediction.
      let told = performOutcome
                   (Insert "pat.believes.desires.mia.pursues-oven.heard.mia") world
      predictMove told (character "pat") mia @?= Nothing
      -- undertaken: the same belief now predicts the next stage.
      let live = doAct "mia" "resolve to build" told
      fmap gaLabel (predictMove live (character "pat") mia)
        @?= Just "mia: sweep the hearth"

  , testCase "the horizon regression: four stages pursued to completion at depth 2" $ do
      let step st = case pickAction 2 st mia of
                      Just ga -> performAction st ga
                      Nothing -> error "mia has no move"
          st5 = iterate step world !! 5   -- undertake + 4 stages
      assertBool "completed" (exists "practice.oven.mia.done.s4" (db st5))

  , testCase "an endeavor with no stages errors loudly" $ do
      r <- try (evaluate (length (show
             (endeavor "idle" 1 "[Actor]: do nothing much" [] []))))
      assertBool "an endeavor is work" (isLeft (r :: Either ErrorCall Int))

  , testCase "a dotted project id errors loudly" $ do
      r <- try (evaluate (length (show
             (endeavor "my.oven" 1 "[Actor]: x" [] ovenStages))))
      assertBool "id must be a single path segment" (isLeft (r :: Either ErrorCall Int))
  ]
