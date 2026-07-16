module Prax.SightSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Either (isLeft)
import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (Val (..), exists)
import           Prax.Query (Condition (..), groundCondition, query)
import           Prax.Sym (intern)
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, possibleActions,
                               performAction, setCharacters)
import           Prax.TypeCheck (typeCheck)
import           Prax.Sight

-- Two rooms; ute and vic share one, wes is alone in the other.
sighting :: [Condition]
sighting = [ Match "at.Seer!Spot", Match "at.Seen!Spot" ]

world :: PraxState
world = foldl (flip performOutcome) base (sightSetup ++ setup)
  where
    base = setCharacters (map character ["ute", "vic", "wes"] ++ [sightChar])
             (definePractices [sightP sighting] emptyState)
    setup = [ Insert "at.ute!hall", Insert "at.vic!hall", Insert "at.wes!attic" ]

-- One tick of the perception clock.
tick :: PraxState -> PraxState
tick st = case possibleActions st sightName of
  (ga : _) -> performAction st ga
  []       -> error "the sight ticker has no action"

tests :: TestTree
tests = testGroup "Prax.Sight"
  [ testCase "the ticker advances the world turn" $ do
      assertBool "turn 0 at setup" (exists "turn!0" (db world))
      assertBool "turn 1 after a tick" (exists "turn!1" (db (tick world)))
      assertBool "turn 2 after two" (exists "turn!2" (db (tick (tick world))))

  , testCase "co-presence deposits sightings, both ways; the absent see nothing" $ do
      let st = tick world
      assertBool "ute sighted vic in the hall" (exists "ute.believes.at.vic!hall" (db st))
      assertBool "vic sighted ute" (exists "vic.believes.at.ute!hall" (db st))
      assertBool "stamped with the turn" (exists "ute.believes.atSince.vic!1" (db st))
      assertBool "nobody sighted wes" (not (exists "ute.believes.at.wes" (db st)))
      assertBool "wes sighted nobody" (not (exists "wes.believes.at.ute" (db st)))

  , testCase "a sighting persists after separation, and a new one overwrites it" $ do
      let st1 = tick world                                        -- ute sees vic in hall
          st2 = tick (performOutcome (Insert "at.vic!attic") st1) -- vic left; tick again
      assertBool "ute still believes vic is in the hall (stale)"
        (exists "ute.believes.at.vic!hall" (db st2))
      assertBool "and the stamp did not refresh" (exists "ute.believes.atSince.vic!1" (db st2))
      -- ute follows and re-sights: overwrite
      let st3 = tick (performOutcome (Insert "at.ute!attic") st2)
      assertBool "belief updated" (exists "ute.believes.at.vic!attic" (db st3))
      assertBool "old belief gone" (not (exists "ute.believes.at.vic!hall" (db st3)))
      assertBool "stamp refreshed" (exists "ute.believes.atSince.vic!3" (db st3))

  , testCase "sightedWithin is a window over the stamp" $ do
      -- Direct query of the scope fragment, grounded to a fixed Actor/Witness pair.
      let grounded h = map (groundCondition (Map.fromList [ (intern "Actor", VSym (intern "ute"))
                                                            , (intern "Witness", VSym (intern "vic")) ]))
                            (sightedWithin h)
          holds h st = not (null (query (readView st) (grounded h) Map.empty))

          st1 = tick world                                        -- turn 1: ute sights vic
          -- Separate them so later ticks (which advance the turn) don't refresh the
          -- stamp — otherwise co-presence would keep re-sighting every tick and the
          -- window would never lapse.
          st1' = performOutcome (Insert "at.vic!attic") st1
          st2  = tick (tick st1')                                 -- turn 3 = expiry (1+2)
          st3  = tick st2                                         -- turn 4: window lapsed

      assertBool "holds right after the sighting" (holds 2 st1)
      assertBool "still holds at the expiry turn" (holds 2 st2)
      assertBool "fails once the window has lapsed" (not (holds 2 st3))

  , testCase "the fixture world is well-formed" $
      typeCheck world @?= []

  , testGroup "the v40 guard (previously latent: an authored sighting template had no guard at all)"
    [ testCase "a sighting template authoring the Prax namespace is a loud construction-time error" $ do
        r <- try (evaluate (length (roles (sightP [Match "at.PraxN!Spot", Match "at.Seen!Spot"]))))
        assertBool "PraxN in the sighting template is rejected" (isLeft (r :: Either ErrorCall Int))

    , testCase "Seer/Seen/Spot remain legal -- they are the contract, not forbidden" $ do
        r <- try (evaluate (length (roles (sightP sighting))))
        assertBool "the ordinary Seer/Seen/Spot fixture is NOT rejected"
          (not (isLeft (r :: Either ErrorCall Int)))
    ]

  , testGroup "the v43 guard: a sighting template must not author Actor (it would bind the ticker, never a mover)"
    [ testCase "a sighting template authoring Actor is a loud construction-time error" $ do
        r <- try (evaluate (length (roles (sightP [Match "at.Actor!Spot", Match "at.Seen!Spot"]))))
        assertBool "Actor in the sighting template is rejected" (isLeft (r :: Either ErrorCall Int))
    ]
  ]
