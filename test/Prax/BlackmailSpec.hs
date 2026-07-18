module Prax.BlackmailSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Either (isLeft)
import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, possibleActions, performAction,
                              setAxioms, setDesires, setCharacters)
import           Prax.Planner (pickAction, predictMove, scoreActions)
import           Prax.Minds (conventional)
import           Prax.Deceit (conceal)
import           Prax.Witness (CoPresence)
import           Prax.Debt (owes)
import           Prax.Deontic (obligationPath)
import           Prax.Persona (Trait (..), bearing, cast, personaVocabulary, transparent)
import           Prax.Blackmail (shakedown)

-- The tale ported from the session probe: mel (extortionist, holds evidence)
-- threatens vic (thief, fears exposure) with what mel saw. wit (and,
-- optionally, wit2) are onlookers at court. The audience arity is the whole
-- mechanic: two heads of exposure risk make compliance rational, one makes
-- defiance rational — pinned exactly at the probe's own numbers (never
-- tuned).
together :: CoPresence
together = [ Match "at.Actor!P", Match "at.Witness!P" ]

shakedownParts :: (Desire, [Action])
shakedownParts = shakedown "defiance" together "took.V.gem" "favor" 6

punishesDefiance :: Desire
punishesDefiance = fst shakedownParts

threatenAct, complyAct, defyAct, exposeAct :: Action
(threatenAct, complyAct, defyAct, exposeAct) = case snd shakedownParts of
  [t, c, d, e] -> (t, c, d, e)
  acts -> error ("shakedown produced " ++ show (length acts) ++ " actions, expected 4")

waitAction :: Action
waitAction = action "[Actor]: wait" [ Match "at.Actor!P" ] []

yardPractice :: Practice
yardPractice = practice
  { practiceId = "yard", roles = ["R"]
  , actions = [ threatenAct, complyAct, defyAct, exposeAct, waitAction ] }

fearsScandal :: Desire
fearsScandal = Desire "fears-scandal" (Want [ Match "W.believes.took.Owner.gem" ] (-10))

-- @twoOnlookers@ toggles whether wit2 shares mel and vic's court (versus off
-- at the mill) — the sole variable between the two pinned scenarios.
mkWorld :: Bool -> PraxState
mkWorld twoOnlookers =
  setDesires [ punishesDefiance, fearsScandal ]
    (setAxioms [conventional] (foldl (flip performOutcome) base setup))
  where
    base = setCharacters
             [ (character "mel") { charWants = [ Want [ owes "mel" "vic" "favor" ] 8 ]
                                  , charDesires = ["punishes-defiance"] }
             , (character "vic") { charWants = [ conceal "took.vic.gem" 12
                                                , Want [ Match "debt.C.vic.favor" ] (-4) ]
                                  , charDesires = ["fears-scandal"] }
             , character "wit"
             , character "wit2" ]
             (definePractices [yardPractice] emptyState)
    setup =
      [ Insert "practice.yard.here"
      , Insert "at.mel!court", Insert "at.vic!court", Insert "at.wit!court"
      , Insert ("at.wit2!" ++ if twoOnlookers then "court" else "mill")
      , Insert "mel.believes.took.vic.gem.seen" ]

twoOnlookerWorld :: PraxState
twoOnlookerWorld = mkWorld True

oneOnlookerWorld :: PraxState
oneOnlookerWorld = mkWorld False

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

-- Trait-priced deterrence fixture (v25 composition): hal and rex share the
-- identical extortion motive (the same debt want against vic); hal is
-- scrupulous (his own extorted marks cost him), rex is his unprincipled
-- twin. Same motive, one temperament.
scrupulous :: Trait
scrupulous = Trait "scrupulous"
  -- more than the discounted future value of the debt this deters (rex's
  -- identical want, undeterred, still nets his threat positive), less than
  -- everything: a real cost, not a prohibition. Priced on the SUBTREE mark
  -- @Owner.extorted.vic@ (v49): the mechanism now tails the extorted mark by
  -- the coercion id (@Owner.extorted.vic.defiance@), not the evidence pattern,
  -- and this subtree Match captures "having extorted vic" regardless of which
  -- coercion — the more faithful deterrence semantics.
  [ Desire "qualms" (Want [ Match "Owner.extorted.vic" ] (-20)) ]

deterrenceWorld :: PraxState
deterrenceWorld =
  setDesires (punishesDefiance : personaVocabulary [scrupulous])
    (setAxioms [conventional, transparent] (foldl (flip performOutcome) base (personaFacts ++ setup)))
  where
    (roster, personaFacts) = cast [scrupulous]
      [ ( (character "hal") { charWants = [ Want [ owes "hal" "vic" "favor" ] 8 ]
                             , charDesires = ["punishes-defiance"] }, [scrupulous] )
      , ( (character "rex") { charWants = [ Want [ owes "rex" "vic" "favor" ] 8 ]
                             , charDesires = ["punishes-defiance"] }, [] )
      , ( character "vic", [] )
      , ( character "wit", [] ) ]
    base = setCharacters roster (definePractices [yardPractice] emptyState)
    setup =
      [ Insert "practice.yard.here"
      , Insert "at.hal!court", Insert "at.rex!court", Insert "at.vic!court", Insert "at.wit!court"
      , Insert "hal.believes.took.vic.gem.seen"
      , Insert "rex.believes.took.vic.gem.seen" ]

tests :: TestTree
tests = testGroup "Prax.Blackmail"
  [ testGroup "guards"
    [ testCase "a dotted id errors loudly" $ do
        r <- try (evaluate (length (show (shakedown "de.f" together "took.V.gem" "favor" 6))))
        assertBool "a dotted id is an error" (isLeft (r :: Either ErrorCall Int))
    , testCase "an evidence pattern naming no one errors loudly" $ do
        r <- try (evaluate (length (show (shakedown "defiance" together "somethinghappened" "favor" 6))))
        assertBool "a threat must be about someone" (isLeft (r :: Either ErrorCall Int))
    , testCase "the usability win: a secondary evidence variable named D or W no longer collides (v40 moved the punitive desire's own machinery to the Prax namespace)" $ do
        r1 <- try (evaluate (length (show (shakedown "x" [] "took.V.from.D" "favor" 1))))
        assertBool "D is an unremarkable secondary variable post-v40"
          (not (isLeft (r1 :: Either ErrorCall Int)))
        r2 <- try (evaluate (length (show (shakedown "x" [] "took.V.by.W" "favor" 1))))
        assertBool "W is an unremarkable secondary variable post-v40"
          (not (isLeft (r2 :: Either ErrorCall Int)))
    , testCase "a secondary evidence variable authoring the Prax namespace collides with the punitive desire's own machinery" $ do
        r1 <- try (evaluate (length (show (shakedown "x" [] "took.V.from.PraxD" "favor" 1))))
        assertBool "PraxD is reserved for the punitive desire's own victim variable"
          (isLeft (r1 :: Either ErrorCall Int))
        r2 <- try (evaluate (length (show (shakedown "x" [] "took.V.by.PraxW" "favor" 1))))
        assertBool "PraxW is reserved for the punitive desire's own believer variable"
          (isLeft (r2 :: Either ErrorCall Int))
    , testCase "a secondary evidence variable named Actor or E is rejected (via the kernel flow — v49 review M2)" $ do
        r1 <- try (evaluate (length (show (shakedown "x" [] "took.V.by.E" "favor" 1))))
        assertBool "E is the comply/defy frame's extorter; a secondary named E merges"
          (isLeft (r1 :: Either ErrorCall Int))
        r2 <- try (evaluate (length (show (shakedown "x" [] "took.V.by.Actor" "favor" 1))))
        assertBool "Actor is every generated frame's actor; a secondary named Actor merges"
          (isLeft (r2 :: Either ErrorCall Int))
    , testCase "a secondary evidence variable named Owner collides with the desire's Owner" $ do
        r <- try (evaluate (length (show (shakedown "x" [] "took.V.for.Owner" "favor" 1))))
        assertBool "Owner is reserved for the desire's own Owner-templated variable"
          (isLeft (r :: Either ErrorCall Int))
    , testCase "a secondary evidence variable named Hearer collides with expose's own hearer" $ do
        r <- try (evaluate (length (show (shakedown "x" [] "took.V.before.Hearer" "favor" 1))))
        assertBool "Hearer is reserved for expose's (and gossip's) own hearer variable"
          (isLeft (r :: Either ErrorCall Int))
    , testCase "a secondary evidence variable named Actor collides with the generated actions' own actor" $ do
        r <- try (evaluate (length (show (shakedown "x" [] "took.V.with.Actor" "favor" 1))))
        assertBool "Actor is reserved for the generated actions' own actor variable"
          (isLeft (r :: Either ErrorCall Int))
    ]

  , testGroup "threaten: the extorter is motivated to threaten, and the threat deposits"
    [ testCase "the extorter threatens at depth 2, holding the punitive desire via charDesires" $
        fmap gaLabel (pickAction 2 twoOnlookerWorld (member twoOnlookerWorld "mel"))
          @?= Just "mel: threaten vic with what you know"

    , testCase "threatening deposits the ordinary fact, the motive-belief, and the extorted mark" $ do
        let st = doAct "mel" "threaten vic" twoOnlookerWorld
        assertBool "the threatened fact"
          (exists "threatened.defiance.mel.vic" (db st))
        assertBool "the motive-belief deposit: vic hears mel's professed punitive intent"
          (exists "vic.believes.desires.mel.punishes-defiance.heard.mel" (db st))
        assertBool "the extorted mark: mel's own memory of having extorted vic, tailed by the coercion id (v49)"
          (exists "mel.extorted.vic.defiance" (db st))
    ]

  , testGroup "two onlookers: the victim complies"
    [ testCase "with two heads of exposure risk, compliance dominates waiting and defiance (property 2, ordering)" $ do
        let threatened = doAct "mel" "threaten vic" twoOnlookerWorld
            scores = scoreActions 2 threatened (member threatened "vic")
            comply = scoreOf scores "vic: buy mel's silence"
            wait_  = scoreOf scores "vic: wait"
            defy   = scoreOf scores "vic: defy mel"
        -- The v49 contract is the ORDERING, not the decimals: two heads of
        -- exposure risk make compliance cheapest and defiance dearest. The
        -- v30 baselines (comply -63.84, wait -71.84, defy -75.80) reproduce
        -- identically under the re-founding — the punitive want and the scored
        -- state are byte-for-byte the same — but the pin no longer depends on
        -- that.
        assertBool ("comply must dominate the ordering; had comply=" ++ show comply
                    ++ " wait=" ++ show wait_ ++ " defy=" ++ show defy)
          (comply > wait_ && wait_ > defy)
        fmap gaLabel (pickAction 2 threatened (member threatened "vic"))
          @?= Just "vic: buy mel's silence"

    , testCase "complying leaves a debt and its obligation, and the threat is gone" $ do
        let threatened = doAct "mel" "threaten vic" twoOnlookerWorld
            complied = doAct "vic" "buy mel's silence" threatened
        assertBool "the debt fact" (exists "debt.mel.vic.favor" (db complied))
        assertBool "the obligation Debt composes it from"
          (exists (obligationPath "vic" "favor") (db complied))
        assertBool "the threat is gone" (not (exists "threatened.defiance.mel.vic" (db complied)))
        assertBool "expose is no longer offered against vic (no standing threat, no defiance)"
          (not (any (\ga -> "expose vic" `isInfixOf` gaLabel ga) (possibleActions complied "mel")))

    , testCase "a renewed threat after compliance extracts nothing (property 3 at the instance — v49 review M3)" $ do
        -- Property 3's primitive pin lives in CoerceSpec; this is the same
        -- law observed through blackmail's own shapes: mel may threaten
        -- again, but the permanent complied marker keeps buy off the table.
        let complied   = doAct "vic" "buy mel's silence"
                           (doAct "mel" "threaten vic" twoOnlookerWorld)
            rethreated = doAct "mel" "threaten vic" complied
        assertBool "the complied marker stands"
          (exists "complied.defiance.mel.vic" (db rethreated))
        assertBool "buy is not offered again under the renewed threat"
          (not (any (\ga -> "buy mel's silence" `isInfixOf` gaLabel ga)
                    (possibleActions rethreated "vic")))
    ]

  , testGroup "one onlooker: the victim rationally defies (both sides of the arithmetic)"
    [ testCase "with a single head of exposure risk, defiance ties waiting and both beat compliance (property 2, ordering)" $ do
        let threatened = doAct "mel" "threaten vic" oneOnlookerWorld
            scores = scoreActions 2 threatened (member threatened "vic")
            comply = scoreOf scores "vic: buy mel's silence"
            wait_  = scoreOf scores "vic: wait"
            defy   = scoreOf scores "vic: defy mel"
        -- The v49 contract: the defy/wait pair ties EXACTLY, and both dominate
        -- comply. v30 baselines (defy = wait = -54.2, comply -63.84) reproduce
        -- identically here.
        defy @?= wait_
        assertBool ("defy/wait must dominate comply; had defy=" ++ show defy
                    ++ " comply=" ++ show comply)
          (defy > comply)
        fmap gaLabel (pickAction 2 threatened (member threatened "vic"))
          @?= Just "vic: defy mel"

    , testCase "the stall-tie: scoreActions gives wait and defy the SAME score under a standing threat" $ do
        let threatened = doAct "mel" "threaten vic" oneOnlookerWorld
            scores = scoreActions 2 threatened (member threatened "vic")
        scoreOf scores "vic: wait" @?= scoreOf scores "vic: defy mel"

    , testCase "defiance leaves the defied fact and clears the threat" $ do
        let threatened = doAct "mel" "threaten vic" oneOnlookerWorld
            defied = doAct "vic" "defy mel" threatened
        assertBool "the defied fact" (exists "defied.defiance.vic.mel" (db defied))
        assertBool "the threat is gone" (not (exists "threatened.defiance.mel.vic" (db defied)))

    , testCase "the victim's model predicts mel's exposure after defiance" $ do
        let threatened = doAct "mel" "threaten vic" oneOnlookerWorld
            defied = doAct "vic" "defy mel" threatened
        fmap gaLabel (predictMove defied (member defied "vic") (member defied "mel"))
          @?= Just "mel: expose vic to wit"
    ]

  , testGroup "trait-priced deterrence (v25 composition)"
    [ testCase "a scrupulous extorter declines; an unprincipled twin with the same motive still threatens" $ do
        fmap gaLabel (pickAction 2 deterrenceWorld (member deterrenceWorld "hal"))
          @?= Just "hal: wait"
        fmap gaLabel (pickAction 2 deterrenceWorld (member deterrenceWorld "rex"))
          @?= Just "rex: threaten vic with what you know"

    , testCase "bearing endows the extorter with the qualm by name" $
        charDesires (bearing scrupulous (character "zed")) @?= ["qualms"]
    ]
  ]
