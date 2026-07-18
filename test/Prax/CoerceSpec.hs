module Prax.CoerceSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Either (isLeft)
import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, possibleActions, performAction,
                              setDesires, setCharacters)
import           Prax.Planner (pickAction, scoreActions)
import           Prax.Debt (owe, owes)
import           Prax.Coerce (Coercion (..), coerce)

-- A protection racket, the SECOND instance of the leverage skeleton (blackmail
-- is the first): mob threatens to burn a barn-owner's barn unless they do a
-- favor. It is EVIDENCE-FREE — the trigger is merely owning a barn — and its
-- punitive kernel is VENGEANCE, not exposure: mob values the burned barn of
-- anyone it has threatened or been defied by. The kernel is authored with the
-- plain victim name @V@; 'coerce' lifts it to @PraxD@ (the rename law).
racket :: Coercion
racket = Coercion
  { coId          = "racket"
  , coVictim      = "V"
  , coTrigger     = [ Match "barn.V" ]
  , coDemandLabel = "[Actor]: do [E]'s favor"
  , coDemand      = owe "E" "Actor" "favor"
  , coPunishLabel = "[Actor]: burn [V]'s barn"
  , coPunishWhen  = [ Match "barn.V", Not "burned.barn.V" ]
  , coPunishOuts  = [ Insert "burned.barn.V" ]
  , coKernel      = [ Match "burned.barn.V" ]
  , coWeight      = 9
  }

-- A kernel with TWO fresh quantifiers beyond the victim — for pinning that the
-- rename gives them distinct names (@PraxW@, @PraxW2@) in first-appearance
-- order.
twoQuantRacket :: Coercion
twoQuantRacket = racket { coKernel = [ Match "burned.barn.V", Match "ally.A.of.B" ] }

punishesRacket :: Desire
punishesRacket = fst (coerce racket)

threatenAct, complyAct, defyAct, punishAct :: Action
(threatenAct, complyAct, defyAct, punishAct) = case snd (coerce racket) of
  [t, c, d, p] -> (t, c, d, p)
  acts -> error ("coerce produced " ++ show (length acts) ++ " actions, expected 4")

-- The do-nothing alternative. Its label sorts BEFORE "threaten" ("bide" < "t")
-- so that when the punitive want is ABSENT and threatening ties it at zero, the
-- tie-break by label picks bide — i.e. removing the want flips the vengeance
-- pin (the v34 tie-break discipline).
bideAct :: Action
bideAct = action "[Actor]: bide" [] []

turfPractice :: Practice
turfPractice = practice
  { practiceId = "turf", roles = ["R"]
  , actions = [ threatenAct, complyAct, defyAct, punishAct, bideAct ] }

-- @holdsWant@ toggles whether mob actually HOLDS the punitive desire — the sole
-- variable behind the vengeance self-motivation pin. mob has no other reason to
-- threaten (empty charWants), so threatening is chosen only when the punitive
-- want makes the future barn-burning worth setting up.
mkWorld :: Bool -> PraxState
mkWorld holdsWant =
  setDesires [ punishesRacket ] (foldl (flip performOutcome) base setup)
  where
    base = setCharacters
             [ (character "mob") { charWants   = []
                                 , charDesires = [ "punishes-racket" | holdsWant ] }
             , (character "vic") { charWants = [ Want [ Match "burned.barn.vic" ] (-12)
                                               , Want [ owes "mob" "vic" "favor" ] (-4) ] } ]
             (definePractices [turfPractice] emptyState)
    setup =
      [ Insert "practice.turf.here"
      , Insert "barn.vic" ]

racketWorld :: PraxState
racketWorld = mkWorld True

noWantWorld :: PraxState
noWantWorld = mkWorld False

member :: PraxState -> String -> Character
member st n = case [ c | c <- characters st, charName c == n ] of
  (c : _) -> c
  []      -> error ("no such character: " ++ n)

-- Perform the named actor's action whose label mentions @needle@.
doAct :: String -> String -> PraxState -> PraxState
doAct who needle st =
  case [ ga | ga <- possibleActions st who, needle `isInfixOf` gaLabel ga ] of
    (ga : _) -> performAction st ga
    []       -> error ("no action for " ++ who ++ " matching " ++ show needle
                       ++ "; had: " ++ show (map gaLabel (possibleActions st who)))

scoreOf :: [(GroundedAction, Double)] -> String -> Double
scoreOf scores needle = case [ s | (ga, s) <- scores, needle `isInfixOf` gaLabel ga ] of
  (s : _) -> s
  []      -> error ("no scored action matching " ++ show needle
                    ++ "; had: " ++ show (map (gaLabel . fst) scores))

tests :: TestTree
tests = testGroup "Prax.Coerce"
  [ testGroup "guards"
    [ testCase "a dotted id errors loudly" $ do
        r <- try (evaluate (length (show (coerce racket { coId = "rac.ket" }))))
        assertBool "a dotted id is an error" (isLeft (r :: Either ErrorCall Int))

    , testCase "the reserved set: victim Actor errors (the previously-latent hole)" $ do
        r <- try (evaluate (length (show (coerce racket { coVictim = "Actor" }))))
        assertBool "victim Actor is reserved" (isLeft (r :: Either ErrorCall Int))

    , testCase "the reserved set: victim PraxV errors" $ do
        r <- try (evaluate (length (show (coerce racket { coVictim = "PraxV" }))))
        assertBool "a Prax-namespaced victim is reserved" (isLeft (r :: Either ErrorCall Int))

    , testCase "the reserved set: a legal victim passes" $ do
        r <- try (evaluate (length (show (coerce racket))))
        assertBool "an ordinary victim variable is fine" (not (isLeft (r :: Either ErrorCall Int)))

    , testCase "a trigger naming the extorter's Actor role errors loudly" $ do
        r <- try (evaluate (length (show (coerce racket { coTrigger = [ Match "spy.Actor" ] }))))
        assertBool "Actor is mechanism-owned in the threaten query" (isLeft (r :: Either ErrorCall Int))
    ]

  , testGroup "the rename law"
    [ testCase "an authored plain-var kernel lifts the victim to PraxD" $
        assertBool "kernel V became PraxD"
          ("burned.barn.PraxD" `isInfixOf` show (desireWant punishesRacket))

    , testCase "two fresh quantifiers get distinct names (PraxW, PraxW2), first-appearance order" $
        assertBool "A -> PraxW and B -> PraxW2"
          ("ally.PraxW.of.PraxW2" `isInfixOf` show (desireWant (fst (coerce twoQuantRacket))))

    , testCase "an authored kernel naming a Prax variable errors loudly (no author writes one)" $ do
        r <- try (evaluate (length (show (coerce racket { coKernel = [ Match "burned.barn.PraxW" ] }))))
        assertBool "a Prax var in the kernel is rejected" (isLeft (r :: Either ErrorCall Int))
    ]

  , testGroup "threaten deposits and the punish gate"
    [ testCase "threatening deposits the marker, the motive-belief, and the extorted mark" $ do
        let st = doAct "mob" "threaten" racketWorld
        assertBool "the threatened fact"
          (exists "threatened.racket.mob.vic" (db st))
        assertBool "the motive-belief deposit: vic hears mob's professed punitive intent"
          (exists "vic.believes.desires.mob.punishes-racket.heard.mob" (db st))
        assertBool "the extorted mark, tailed by the coercion id"
          (exists "mob.extorted.vic.racket" (db st))

    , testCase "punish fires against a STANDING threat, with no defiance" $ do
        let threatened = doAct "mob" "threaten" racketWorld
        assertBool "vic has not defied" (not (exists "defied.racket.vic.mob" (db threatened)))
        assertBool "burning is offered against the standing threat alone"
          (any (\ga -> "burn" `isInfixOf` gaLabel ga) (possibleActions threatened "mob"))
        let burned = doAct "mob" "burn" threatened
        assertBool "the barn is burned" (exists "burned.barn.vic" (db burned))
    ]

  , testGroup "property 1: stalling never dominates"
    [ testCase "under a standing threat, bide never strictly beats both comply and defy" $ do
        let threatened = doAct "mob" "threaten" racketWorld
            scores = scoreActions 2 threatened (member threatened "vic")
            bide  = scoreOf scores "vic: bide"
            comply = scoreOf scores "favor"
            defy  = scoreOf scores "vic: defy"
        assertBool ("bide must not dominate; had bide=" ++ show bide
                    ++ " comply=" ++ show comply ++ " defy=" ++ show defy)
          (not (bide > comply && bide > defy))
    ]

  , testGroup "property 3: repeat extraction is impossible"
    [ testCase "after compliance, a renewed threat offers no re-buy (the permanent complied marker)" $ do
        let threatened = doAct "mob" "threaten" racketWorld
            complied   = doAct "vic" "favor" threatened
            rethreat   = doAct "mob" "threaten" complied
        assertBool "the permanent complied marker survives" (exists "complied.racket.mob.vic" (db rethreat))
        assertBool "the renewed threat is standing" (exists "threatened.racket.mob.vic" (db rethreat))
        assertBool "comply is not offered a second time"
          (not (any (\ga -> "favor" `isInfixOf` gaLabel ga) (possibleActions rethreat "vic")))
        assertBool "the victim's chosen response is not a re-buy"
          (not (any (("favor" `isInfixOf`) . gaLabel)
                  (pickAction 2 rethreat (member rethreat "vic"))))
    ]

  , testGroup "property 5: credibility is self-motivation (the vengeance, base-less kernel)"
    [ testCase "the extorter CHOOSES to threaten at depth 2, holding the punitive want" $
        fmap gaLabel (pickAction 2 racketWorld (member racketWorld "mob"))
          @?= Just "mob: threaten vic"

    , testCase "without the punitive want, the same choice collapses to doing nothing" $
        fmap gaLabel (pickAction 2 noWantWorld (member noWantWorld "mob"))
          @?= Just "mob: bide"
    ]
  ]
