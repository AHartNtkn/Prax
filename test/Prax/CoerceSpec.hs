module Prax.CoerceSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Either (isLeft)
import           Data.List (isInfixOf)
import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, possibleActions, performAction,
                              setDesires, setCharacters, roundBoundary)
import           Prax.Planner (pickAction, scoreActions)
import           Prax.Debt (owe, owes)
import           Prax.Witness (CoPresence, asRole)
import           Prax.Persist (serializeState, deserializeState)
import           Prax.Coerce (Coercion (..), coerce)

-- A protection racket, the SECOND instance of the leverage skeleton (blackmail
-- is the first): mob threatens to burn a barn-owner's barn unless they do a
-- favor. It is EVIDENCE-FREE — the trigger is merely owning a barn — and its
-- punitive kernel is VENGEANCE, not exposure: mob values the burned barn of
-- anyone it has threatened or been defied by. The kernel is authored with the
-- plain victim name @V@; 'coerce' lifts it to @PraxD@ (the rename law).
racket :: Coercion
racket = Coercion
  { coId            = "racket"
  , coVictim        = "V"
  , coTrigger       = [ Match "barn.V" ]
  , coThreatenLabel = "[Actor]: threaten [V]"
  , coDemandLabel   = "[Actor]: do [E]'s favor"
  , coDemand        = owe "E" "Actor" "favor"
  , coPunishLabel   = "[Actor]: burn [V]'s barn"
  , coPunishWhen    = [ Match "barn.V", Not "burned.barn.V" ]
  , coPunishOuts    = [ Insert "burned.barn.V" ]
  , coKernel        = [ Match "burned.barn.V" ]
  , coWeight        = 9
  , coThreatLasts     = Nothing
  , coComplianceLasts = Nothing
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

-- The BLUFF world (spec v54 §2): the punitive desire is REGISTERED (mkWorld
-- always calls setDesires) but NOT held (holdsWant = False leaves charDesires
-- empty). The victim's fear is identical to the genuine case; the extorter,
-- holding no punitive want, never chooses punish.
noWantWorld :: PraxState
noWantWorld = mkWorld False

-- v54: a SERIAL-extortion racket. The complied marker expires after 2
-- boundaries (coComplianceLasts = Just 2 → coerce reads InsertFor 2), so the
-- racket cycles: one purchase per bought period, then the extorter extracts
-- again. Nothing else changes — comply deletes the threat, threaten re-arms.
cyclingRacket :: Coercion
cyclingRacket = racket { coComplianceLasts = Just 2 }

-- v54: an EXPIRING threat. The threat marker retracts 2 boundaries after
-- threaten (coThreatLasts = Just 2). The DEFIED arm is untouched — defy's
-- Insert is permanent, so a defied threat's punish survives the expiry.
expiringRacket :: Coercion
expiringRacket = racket { coThreatLasts = Just 2 }

-- A turf world installing a given racket's actions; mob genuinely holds the
-- named vengeance want, vic fears the burned barn and the debt (the mkWorld
-- scaffold, parameterized on which racket compiles the actions).
mkTurfWorld :: Coercion -> PraxState
mkTurfWorld co =
  setDesires [ pun ] (foldl (flip performOutcome) base setup)
  where
    (pun, acts) = coerce co
    base = setCharacters
             [ (character "mob") { charWants   = []
                                 , charDesires = [ desireName pun ] }
             , (character "vic") { charWants = [ Want [ Match "burned.barn.vic" ] (-12)
                                               , Want [ owes "mob" "vic" "favor" ] (-4) ] } ]
             (definePractices
                [ practice { practiceId = "turf", roles = ["R"]
                           , actions = acts ++ [bideAct] } ]
                emptyState)
    setup = [ Insert "practice.turf.here", Insert "barn.vic" ]

cyclingWorld :: PraxState
cyclingWorld = mkTurfWorld cyclingRacket

expiringWorld :: PraxState
expiringWorld = mkTurfWorld expiringRacket

-- Drive k round boundaries (the ScheduleSpec idiom — InsertFor retracts only at
-- a boundary, which the racket harness never fires on its own).
boundaries :: Int -> PraxState -> PraxState
boundaries k st = iterate roundBoundary st !! k

isInsertFor :: Outcome -> Bool
isInsertFor InsertFor{} = True
isInsertFor _           = False

-- v54: counter-coercion is composition. vic's own Coercion whose trigger reads
-- the mark mob's extortion deposits (@M.extorted.vic.racket@) — the extorter
-- slot @M@ is a VARIABLE, so the trigger BINDS the counter's victim to whoever
-- extorted vic (being known as an extorter is the exposure vic holds over M).
counter :: Coercion
counter = Coercion
  { coId            = "counter"
  , coVictim        = "M"
  , coTrigger       = [ Match "M.extorted.vic.racket" ]
  , coThreatenLabel = "[Actor]: threaten to expose [M]"
  , coDemandLabel   = "[Actor]: take [E]'s protection"
  , coDemand        = owe "E" "Actor" "protection"
  , coPunishLabel   = "[Actor]: expose [M] as an extorter"
  , coPunishWhen    = [ Not "exposed.M" ]
  , coPunishOuts    = [ Insert "exposed.M" ]
  , coKernel        = [ Match "exposed.M" ]
  , coWeight        = 9
  , coThreatLasts     = Nothing
  , coComplianceLasts = Nothing
  }

-- mob genuinely extorts vic (holds punishes-racket); vic holds punishes-counter
-- and values M exposed once vic has counter-threatened. Both practices are
-- installed; the world gives vic the leverage material the racket world lacks.
counterWorld :: PraxState
counterWorld =
  setDesires [ punishesRacket, fst (coerce counter) ]
    (foldl (flip performOutcome) base setup)
  where
    base = setCharacters
             [ (character "mob") { charWants = [], charDesires = ["punishes-racket"] }
             , (character "vic") { charWants = [], charDesires = ["punishes-counter"] } ]
             (definePractices
                [ practice { practiceId = "turf", roles = ["R"]
                           , actions = snd (coerce racket) ++ [bideAct] }
                , practice { practiceId = "countyard", roles = ["R"]
                           , actions = snd (coerce counter) ++ [bideAct] } ]
                emptyState)
    setup = [ Insert "practice.turf.here"
            , Insert "practice.countyard.here"
            , Insert "barn.vic" ]

-- Regression fixture (v49 Task 1 fix wave): a BLACKMAIL-SHAPED coercion built
-- straight through 'coerce' — an evidence trigger naming @Actor@ (the
-- extorter's own frame variable), a debt-shaped demand, and an expose-shaped
-- punish, exactly the shape 'Prax.Blackmail.shakedown' will be re-founded on
-- in Task 2 (plan @2026-07-17-v49-coercion.md:112@: @trigger = Match
-- (beliefAbout "Actor" pat) : asRole victim copresence@). This is the shape
-- that exposed the Critical finding: the extorter's evidence-holding
-- ("Actor.believes.stole.V.loaf") is a legitimate frame reference in
-- threaten's own query (Actor IS the extorter there), not a capture — so it
-- must construct without error and its threaten must actually fire.
court :: CoPresence
court = [ Match "at.Actor!P", Match "at.Witness!P" ]

blackmailShaped :: Coercion
blackmailShaped = Coercion
  { coId            = "leverage"
  , coVictim        = "V"
  , coTrigger       = Match "Actor.believes.stole.V.loaf" : asRole "V" court
  , coThreatenLabel = "[Actor]: threaten [V] with what you know"
  , coDemandLabel   = "[Actor]: buy [E]'s silence"
  , coDemand        = owe "E" "Actor" "silence"
  , coPunishLabel   = "[Actor]: expose [V] to [Hearer]"
  , coPunishWhen    = Match "Actor.believes.stole.V.loaf" : Neq "Hearer" "V" : asRole "Hearer" court
  , coPunishOuts    = [ Insert "Hearer.believes.stole.V.loaf" ]
  , coKernel        = [ Match "W.believes.stole.V.loaf" ]
  , coWeight        = 6
  , coThreatLasts     = Nothing
  , coComplianceLasts = Nothing
  }

leverageThreaten, leverageComply, leverageDefy, leveragePunish :: Action
(leverageThreaten, leverageComply, leverageDefy, leveragePunish) = case snd (coerce blackmailShaped) of
  [t, c, d, p] -> (t, c, d, p)
  acts -> error ("coerce produced " ++ show (length acts) ++ " actions, expected 4")

courtPractice :: Practice
courtPractice = practice
  { practiceId = "court", roles = ["R"]
  , actions = [ leverageThreaten, leverageComply, leverageDefy, leveragePunish ] }

leverageWorld :: PraxState
leverageWorld =
  setDesires [ fst (coerce blackmailShaped) ]
    (foldl (flip performOutcome) base setup)
  where
    base = setCharacters
             [ (character "mel") { charWants = [], charDesires = ["punishes-leverage"] }
             , character "vic" ]
             (definePractices [courtPractice] emptyState)
    setup =
      [ Insert "practice.court.here"
      , Insert "mel.believes.stole.vic.loaf"
      , Insert "at.mel.court"
      , Insert "at.vic.court" ]

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

    , testCase "a trigger naming a Prax-namespaced variable errors loudly (the v40 law, frame-independent)" $ do
        r <- try (evaluate (length (show (coerce racket { coTrigger = [ Match "spy.PraxW" ] }))))
        assertBool "a Prax var in the trigger is rejected" (isLeft (r :: Either ErrorCall Int))
    ]

  , testGroup "regression: the trigger guard must not forbid Actor (blackmail-shaped evidence, v49 Task 1 fix wave)"
    [ testCase "a blackmail-shaped Coercion whose trigger names Actor (the extorter's own evidence) constructs without error" $ do
        r <- try (evaluate (length (show (coerce blackmailShaped))))
        assertBool "Actor is bound in threaten's own frame, not a capture" (not (isLeft (r :: Either ErrorCall Int)))

    , testCase "threaten is offered and fires, depositing the threat marker" $ do
        assertBool "threaten offered"
          (any (\ga -> "threaten" `isInfixOf` gaLabel ga) (possibleActions leverageWorld "mel"))
        let st = doAct "mel" "threaten" leverageWorld
        assertBool "the threatened fact" (exists "threatened.leverage.mel.vic" (db st))

    -- Important finding: the boundary table lists "label" in threaten's
    -- CONTENT column, same as comply/punish, but the record had no
    -- coThreatenLabel field to carry it — so BlackmailSpec:171's pinned
    -- "mel: threaten vic with what you know" (the evidence-flavor suffix)
    -- could never be reproduced through the primitive. Task 2 needs this
    -- EXACT shape producible; pin it here with the flagship's own names so
    -- Task 2 inherits it proven.
    , testCase "the authored threaten label surfaces exactly (BlackmailSpec:171's pinned shape, proven producible)" $
        (case [ gaLabel ga | ga <- possibleActions leverageWorld "mel", "threaten" `isInfixOf` gaLabel ga ] of
           (l : _) -> l
           []      -> error "no threaten action offered")
          @?= "mel: threaten vic with what you know"
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

  , testGroup "v54 property 1: Nothing is today (the default markers compile to permanent Inserts)"
    [ testCase "the racket's compiled threaten and comply carry Insert for both markers, no InsertFor" $ do
        assertBool "threaten's marker is a permanent Insert"
          (Insert "threatened.racket.Actor.V" `elem` actionOutcomes threatenAct)
        assertBool "comply's complied marker is a permanent Insert"
          (Insert "complied.racket.E.Actor" `elem` actionOutcomes complyAct)
        assertBool "neither marker is an InsertFor (byte-identical to the shipped compilation)"
          (not (any isInsertFor (actionOutcomes threatenAct ++ actionOutcomes complyAct)))
    ]

  , testGroup "v54 property 2: the racket cycles under an expiring complied marker"
    [ testCase "one purchase per bought period; the marker expires and the extorter extracts again" $ do
        let threatened1 = doAct "mob" "threaten" cyclingWorld
            complied1   = doAct "vic" "favor" threatened1
        assertBool "the demand fact lands (first extraction)"
          (exists "debt.mob.vic.favor" (db complied1))
        assertBool "the complied marker holds after compliance"
          (exists "complied.racket.mob.vic" (db complied1))
        let rethreat = doAct "mob" "threaten" complied1
        assertBool "the re-threat stands" (exists "threatened.racket.mob.vic" (db rethreat))
        assertBool "comply is BLOCKED within the bought period (the complied marker gates it)"
          (not (any (("favor" `isInfixOf`) . gaLabel) (possibleActions rethreat "vic")))
        let expired = boundaries 2 rethreat
        assertBool "the complied marker expired after 2 boundaries"
          (not (exists "complied.racket.mob.vic" (db expired)))
        assertBool "comply is available AGAIN under the still-standing threat"
          (any (("favor" `isInfixOf`) . gaLabel) (possibleActions expired "vic"))
        let complied2 = doAct "vic" "favor" expired
        assertBool "the second extraction completes: the threat is bought off again"
          (not (exists "threatened.racket.mob.vic" (db complied2)))
        assertBool "the complied marker is re-armed for the next bought period"
          (exists "complied.racket.mob.vic" (db complied2))
    ]

  , testGroup "v54: the deal economics, as observed (the no-fiat ruling — no marker enforces the deal)"
    [ testCase "during the bought period the extorter's punish-vs-bide goes where the vengeance utilities point" $ do
        let rethreat = doAct "mob" "threaten" (doAct "vic" "favor" (doAct "mob" "threaten" cyclingWorld))
            scores   = scoreActions 2 rethreat (member rethreat "mob")
            burn     = scoreOf scores "burn"
            bide     = scoreOf scores "mob: bide"
            chose    = fmap gaLabel (pickAction 2 rethreat (member rethreat "mob"))
        -- OBSERVED (not guaranteed): burn=24.39 vs bide=15.39 — a 9.0 gap, the
        -- vengeance kernel's own weight. Burning the barn satisfies the kernel
        -- NOW; biding gains nothing and the purse is already taken, so continued
        -- extortion is worth less than punishing and the extorter BETRAYS.
        -- Paying bought the purse for the period, never the person. No marker
        -- enforces the deal; this is ordinary pickAction scoring (the no-fiat
        -- ruling). Scores also echoed in the assertion message.
        assertBool ("observed betrayal: burn=" ++ show burn ++ " bide=" ++ show bide
                    ++ " chose=" ++ show chose)
          (burn > bide)
        chose @?= Just "mob: burn vic's barn"
    ]

  , testGroup "v54 property 3: a stale threat is spent, but a defied threat's punish survives"
    [ testCase "after n boundaries an unanswered threat stops offering punish and stops pressuring comply" $ do
        let threatened = doAct "mob" "threaten" expiringWorld
        assertBool "punish (burn) offered while the threat stands"
          (any (("burn" `isInfixOf`) . gaLabel) (possibleActions threatened "mob"))
        let stale = boundaries 2 threatened
        assertBool "the threat marker expired after 2 boundaries"
          (not (exists "threatened.racket.mob.vic" (db stale)))
        assertBool "punish's standing arm is gone (no burn offered)"
          (not (any (("burn" `isInfixOf`) . gaLabel) (possibleActions stale "mob")))
        assertBool "comply is no longer pressured (not offered — no standing threat)"
          (not (any (("favor" `isInfixOf`) . gaLabel) (possibleActions stale "vic")))

    , testCase "a DEFIED threat's punish survives expiry (the defied marker is permanent)" $ do
        let threatened = doAct "mob" "threaten" expiringWorld
            defied     = doAct "vic" "defy" threatened
            later      = boundaries 3 defied
        assertBool "the defied marker stands permanently past the threat's expiry"
          (exists "defied.racket.vic.mob" (db later))
        assertBool "burn is still offered via the defied arm, past expiry"
          (any (("burn" `isInfixOf`) . gaLabel) (possibleActions later "mob"))
    ]

  , testGroup "v54 property 5: the bluff pair — registered-not-held vs registered-and-held"
    [ testCase "the victim's comply/defy decision is IDENTICAL (the fear resolves on the registered vocabulary)" $ do
        let gThreat = doAct "mob" "threaten" racketWorld   -- genuine: registered AND held
            bThreat = doAct "mob" "threaten" noWantWorld   -- bluff:   registered, NOT held
            gScores = scoreActions 2 gThreat (member gThreat "vic")
            bScores = scoreActions 2 bThreat (member bThreat "vic")
        scoreOf gScores "favor"     @?= scoreOf bScores "favor"
        scoreOf gScores "vic: defy" @?= scoreOf bScores "vic: defy"
        scoreOf gScores "vic: bide" @?= scoreOf bScores "vic: bide"
        fmap gaLabel (pickAction 2 gThreat (member gThreat "vic"))
          @?= fmap gaLabel (pickAction 2 bThreat (member bThreat "vic"))

    , testCase "defied, the genuine extorter picks punish; the bluffer (holding no want) does not" $ do
        let gDefied = doAct "vic" "defy" (doAct "mob" "threaten" racketWorld)
            bDefied = doAct "vic" "defy" (doAct "mob" "threaten" noWantWorld)
        assertBool "genuine mob punishes after defiance"
          (maybe False ("burn" `isInfixOf`)
             (fmap gaLabel (pickAction 2 gDefied (member gDefied "mob"))))
        assertBool "the bluffing mob declines punish (its choice is bide, not burn)"
          (not (maybe False ("burn" `isInfixOf`)
                  (fmap gaLabel (pickAction 2 bDefied (member bDefied "mob")))))
    ]

  , testGroup "v54 property 7: the table turns — counter-coercion is composition"
    [ testCase "once extorted, the victim reaches a standing counter-threat from pure content over the shipped surface" $ do
        let extorted = doAct "mob" "threaten" counterWorld
        assertBool "mob's threaten deposited the extorted mark the counter reads"
          (exists "mob.extorted.vic.racket" (db extorted))
        assertBool "vic can now counter-threaten mob (the variable extorter slot bound M to mob)"
          (any (("threaten to expose" `isInfixOf`) . gaLabel) (possibleActions extorted "vic"))
        fmap gaLabel (pickAction 2 extorted (member extorted "vic"))
          @?= Just "vic: threaten to expose mob"
        let both = doAct "vic" "threaten to expose" extorted
        assertBool "the racket threat stands" (exists "threatened.racket.mob.vic" (db both))
        assertBool "the counter threat stands — both threats live at once"
          (exists "threatened.counter.vic.mob" (db both))
    ]

  , testGroup "v54: mid-racket save/resume (home: CoerceSpec — it exercises the racket's own cycle)"
    [ testCase "a save carrying the pending complied-expiry reloads and the cycle resumes on schedule" $ do
        let complied = doAct "vic" "favor" (doAct "mob" "threaten" cyclingWorld)
        assertBool "the complied-expiry due is pending before save"
          (not (Map.null (expiries complied)))
        let reloaded = deserializeState (serializeState complied) cyclingWorld
        assertBool "the complied marker reloaded" (exists "complied.racket.mob.vic" (db reloaded))
        let rethreat = doAct "mob" "threaten" reloaded
        assertBool "comply is BLOCKED right after reload (still within the bought period)"
          (not (any (("favor" `isInfixOf`) . gaLabel) (possibleActions rethreat "vic")))
        let expired = boundaries 2 rethreat
        assertBool "the complied marker expired on schedule after reload"
          (not (exists "complied.racket.mob.vic" (db expired)))
        assertBool "the cycle resumes: comply is available again"
          (any (("favor" `isInfixOf`) . gaLabel) (possibleActions expired "vic"))
    ]
  ]
