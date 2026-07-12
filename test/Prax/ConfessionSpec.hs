module Prax.ConfessionSpec (tests) where

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
import           Prax.Derive (Axiom (..))
import qualified Prax.Planner as Planner
import           Prax.Planner (pickAction, scoreActions)
import           Prax.Minds (selfWants)
import           Prax.Witness (CoPresence)
import           Prax.Deceit (conceal)
import           Prax.Persona (Trait (..), personaVocabulary, cast)
import           Prax.Blackmail (shakedown)
import           Prax.Confession (confess, absolve, incorrigible)

together :: CoPresence
together = [ Match "at.Actor!P", Match "at.Witness!P" ]

-- Shared deed vocabulary across confess/absolve/incorrigible: the FIRST
-- variable (Doer) is both the deed's subject and its confessor -- confession
-- is self-incriminating by design (spec point 2), so a wronged/stolen mark's
-- own content names the wrongdoer, unlike Deceit's arm's-length lie about a
-- third party.
pat :: String
pat = "wronged.Doer.Victim"

member :: PraxState -> String -> Character
member st n = case [ c | c <- characters st, charName c == n ] of
  (c : _) -> c
  []      -> error ("no such character: " ++ n)

doAct :: String -> String -> PraxState -> PraxState
doAct who needle st =
  case [ ga | ga <- possibleActions st who, needle `isInfixOf` gaLabel ga ] of
    (ga : _) -> performAction st ga
    []       -> error ("no action for " ++ who ++ " matching " ++ show needle
                       ++ "; had: " ++ show (map gaLabel (possibleActions st who)))

offered :: String -> String -> PraxState -> Bool
offered who needle st = any ((needle `isInfixOf`) . gaLabel) (possibleActions st who)

scoreOf :: [(GroundedAction, Double)] -> String -> Double
scoreOf scores needle = case [ s | (ga, s) <- scores, needle `isInfixOf` gaLabel ga ] of
  (s : _) -> s
  []      -> error ("no scored action matching " ++ show needle
                    ++ "; had: " ++ show (map (gaLabel . fst) scores))

--------------------------------------------------------------------------------
-- Group 1: conversion mechanics + trait-priced relief (v25 composition)
--------------------------------------------------------------------------------

-- A trait pricing your OWN "lied" marks at -6 (guilt) and their "confessed"
-- form at 0 (a clean conscience, not a bonus) -- the residue the conversion
-- is FOR.
conscience :: Trait
conscience = Trait "conscience"
  [ Desire "guilt" (Want [ Match "Owner.lied.H.wronged.Owner.Victim" ] (-6))
  , Desire "clearConscience" (Want [ Match "Owner.confessed.H.wronged.Owner.Victim" ] 0) ]

confessAct :: Action
confessAct = confess "lied" together pat "[Actor]: confess to [Hearer] about wronging [Victim]"

convWorld :: PraxState
convWorld = foldl (flip performOutcome) base (personaFacts ++ setup)
  where
    (roster, personaFacts) = cast [conscience]
      [ ( character "bob", [conscience] )
      , ( character "fay", [] ) ]
    base = setDesires (personaVocabulary [conscience])
             (setCharacters roster (definePractices [p] emptyState))
    p = practice
      { practiceId = "confessional", roles = ["R"], actions = [confessAct] }
    setup =
      [ Insert "practice.confessional.here"
      , Insert "at.bob!yard", Insert "at.fay!yard"
        -- two DISTINCT deeds, same original hearer "edda": bob wronged carol,
        -- and separately wronged dana. Confessing one must not touch the other.
      , Insert "bob.lied.edda.wronged.bob.carol"
      , Insert "bob.lied.edda.wronged.bob.dana" ]

conversionTests :: TestTree
conversionTests = testGroup "conversion mechanics"
  [ testCase "confessing converts exactly the confessed deed's mark; a second lied-mark survives" $ do
      let st = doAct "bob" "confess to fay about wronging carol" convWorld
      assertBool "the carol mark converted to confessed"
        (exists "bob.confessed.edda.wronged.bob.carol" (db st))
      assertBool "the carol lied-mark is gone"
        (not (exists "bob.lied.edda.wronged.bob.carol" (db st)))
      assertBool "the dana lied-mark survives untouched"
        (exists "bob.lied.edda.wronged.bob.dana" (db st))

  , testCase "confession deposits the hearer's sourced belief (the ordinary hearsay channel)" $ do
      let st = doAct "bob" "confess to fay about wronging carol" convWorld
      assertBool "fay heard it from bob"
        (exists "fay.believes.wronged.bob.carol.heard.bob" (db st))

  , testCase "the converted deed is not re-offered; the surviving one still is" $ do
      let st = doAct "bob" "confess to fay about wronging carol" convWorld
      assertBool "no more confessing about carol (the mark is gone)"
        (not (offered "bob" "wronging carol" st))
      assertBool "confessing about dana remains offered"
        (offered "bob" "wronging dana" st)

  , testCase "a trait pricing lied at -6 and confessed at 0: the relief in evaluate/selfWants" $ do
      -- measured directly (v25's own idiom): two lied marks cost -6 each.
      Planner.evaluate convWorld (selfWants convWorld (member convWorld "bob")) @?= (-12)
      let st = doAct "bob" "confess to fay about wronging carol" convWorld
      -- one converted (0 residue) + one still lied (-6): -6, not -12.
      Planner.evaluate st (selfWants st (member st "bob")) @?= (-6)
  ]

--------------------------------------------------------------------------------
-- Group 2: absolution -- grant, refusal gate, gossip gate, double-absolution
--------------------------------------------------------------------------------

absolveAct :: Action
absolveAct = absolve "recanted" pat "fedUp" "[Actor]: absolve [Doer]"

absolveWorld :: PraxState
absolveWorld = setCharacters
  [ character "gwen"   -- fresh: will grant
  , character "hank"   -- fed up: refuses
  , character "ivy"    -- only gossip-sourced: never qualifies
  , character "cody" ] -- the confessor
  (foldl (flip performOutcome) base setup)
  where
    base = definePractices [p] emptyState
    p = practice
      { practiceId = "confessional", roles = ["R"], actions = [absolveAct] }
    setup =
      [ Insert "practice.confessional.here"
        -- gwen was confessed to directly (sourced from cody, the doer).
      , Insert "gwen.believes.wronged.cody.carol.heard.cody"
        -- hank was ALSO confessed to, but his patience is already spent.
      , Insert "hank.believes.wronged.cody.carol.heard.cody"
      , Insert "regards.hank.cody.fedUp"
        -- ivy only heard it from a third party (gossip) -- never confessed
        -- TO her, so the doer-sourced gate must block her.
      , Insert "ivy.believes.wronged.cody.carol.heard.jill" ]

absolutionTests :: TestTree
absolutionTests = testGroup "absolution: grant, refusal, gossip gate, double-grant"
  [ testCase "granting inserts the world's defeater" $ do
      let st = doAct "gwen" "absolve cody" absolveWorld
      assertBool "the defeater" (exists "recanted.cody" (db st))

  , testCase "a planted incorrigibility regard blocks the affordance (refusal)" $
      assertBool "hank, fed up, has no absolve action against cody"
        (not (offered "hank" "absolve cody" absolveWorld))

  , testCase "gossip-sourced belief does not qualify (heard from a non-doer)" $
      assertBool "ivy never confessed to, has no absolve action"
        (not (offered "ivy" "absolve cody" absolveWorld))

  , testCase "double-absolution is blocked while the defeater stands" $ do
      let st = doAct "gwen" "absolve cody" absolveWorld
      assertBool "gwen cannot grant a second time" (not (offered "gwen" "absolve cody" st))
  ]

--------------------------------------------------------------------------------
-- Group 3: incorrigibility -- k-1 vs k, gossip feeds it, per-absolver
-- independence, permanence
--------------------------------------------------------------------------------

incorrigAxiom :: Axiom
incorrigAxiom = incorrigible pat 2 "fedUp"

incorrigWorld :: PraxState
incorrigWorld = setAxioms [incorrigAxiom] (foldl (flip performOutcome) base setup)
  where
    base = setCharacters (map character ["mave", "hank", "cody"])
             (definePractices [] emptyState)
    setup =
      [ -- mave believes exactly ONE distinct instance of cody's wrongdoing
        -- (k-1 of k=2): must derive nothing.
        Insert "mave.believes.wronged.cody.carol.seen"
        -- hank believes TWO distinct instances -- one seen, one gossiped --
        -- reaching k=2: must derive the regard. Gossip feeds the count same
        -- as an eyewitness (the axiom reads .believes.<pat>, no provenance
        -- check).
      , Insert "hank.believes.wronged.cody.carol.seen"
      , Insert "hank.believes.wronged.cody.dana.heard.jill" ]

incorrigibilityTests :: TestTree
incorrigibilityTests = testGroup "incorrigibility: threshold, gossip, independence, permanence"
  [ testCase "k-1 believed deeds derive nothing" $
      assertBool "mave (one instance) does not regard cody as fed-up"
        (not (exists "regards.mave.cody.fedUp" (readView incorrigWorld)))

  , testCase "k believed deeds -- one via gossip -- derive the regard" $
      assertBool "hank (two instances, one gossiped) regards cody as fed-up"
        (exists "regards.hank.cody.fedUp" (readView incorrigWorld))

  , testCase "per-absolver independence: one fed up, one fresh, from the same facts" $ do
      let v = readView incorrigWorld
      assertBool "hank: fed up"   (exists "regards.hank.cody.fedUp" v)
      assertBool "mave: fresh"    (not (exists "regards.mave.cody.fedUp" v))

  , testCase "permanence: an absolution elsewhere does not retract the regard" $ do
      -- nothing in this fixture is a defeater FOR fedUp (incorrigible has
      -- none by design -- belief never dies); an unrelated defeater insert
      -- (as if some other absolver had granted cody absolution) leaves it.
      let st = performOutcome (Insert "recanted.cody") incorrigWorld
      assertBool "hank's fed-up regard survives an unrelated absolution"
        (exists "regards.hank.cody.fedUp" (readView st))
  ]

--------------------------------------------------------------------------------
-- Group 4: the probed arithmetic (measured live above; pinned both sides)
--------------------------------------------------------------------------------

-- 4a. Spontaneous confession: a conscience-bearer's mark conversion is
-- utility-improving against a MILD concealment stake, and NOT against an
-- expensive one. MEASURED via scoreActions before pinning (v30's idiom):
--   mild stake=4:  confess = 0.0,  hold your tongue = -2.0    -> confesses
--   big  stake=20: hold your tongue = 37.94, confess = 0.0    -> doesn't

spontConfessAct :: Action
spontConfessAct = confess "lied" together pat "[Actor]: confess to [Hearer] about wronging [Victim]"

holdTongue :: Action
holdTongue = action "[Actor]: hold your tongue" [ Match "at.Actor!P" ] []

spontPractice :: Practice
spontPractice = practice
  { practiceId = "confessional", roles = ["R"], actions = [spontConfessAct, holdTongue] }

spontWorld :: Int -> PraxState
spontWorld stake = foldl (flip performOutcome) base (personaFacts ++ setup)
  where
    (roster, personaFacts) = cast [conscience]
      [ ( (character "wade") { charWants = [ conceal "wronged.wade.carol" stake ] }, [conscience] )
      , ( character "priya", [] ) ]
    base = setDesires (personaVocabulary [conscience])
             (setCharacters roster (definePractices [spontPractice] emptyState))
    setup =
      [ Insert "practice.confessional.here"
      , Insert "at.wade!yard", Insert "at.priya!yard"
      , Insert "wade.lied.edda.wronged.wade.carol" ]

-- 4b. Confession as blackmail defense: the shakedown composed with confess
-- over the SAME self-incriminating pattern (the theft's own subject IS the
-- confessor). The victim ALSO bears the v25 conscience shape (guilt -6 on
-- the "lied" mark, 0 once "confessed") -- without it, confess/defy/wait
-- score IDENTICALLY (the threat's continuation doesn't depend on vic's own
-- move) and "confesses" would rest on nothing but the scoreActions label
-- tie-break, not on any merit. With it, confessing strictly dominates
-- defy/wait (it is the only move that sheds the -6 guilt) rather than
-- merely tying and winning alphabetically. MEASURED via scoreActions (both
-- threatened worlds, re-measured after adding the conscience desires):
--   price=30, fear=3: confess-to-cora = -16.26 (STRICTLY best),
--                      defy = wait = -25.26, comply = -120.06
--                      -> pickAction picks confess on the merits
--   price=1,  fear=30: comply = -159.81 (STRICTLY best),
--                      confess-to-cora = -162.60, defy = wait = -171.60
--                      -> pickAction picks comply
-- (confess-to-cora and confess-to-mel remain tied with EACH OTHER at every
-- price/fear pair -- an orthogonal, unproblematic tie over WHICH hearer to
-- spend the secret on, not over whether to confess at all.)
-- After confessing, mel's expose is verified dead: only "mel: wait" remains
-- (cora, the sole other co-present hearer, already believes -- sourced from
-- vic -- so expose's own "hearer doesn't already believe" gate closes for
-- every possible hearer at once).

blackmailPat :: String
blackmailPat = "stole.Vic.loaf"

shakedownParts :: (Desire, [Action])
shakedownParts = shakedown "theft" together blackmailPat "favor" 2

punishesTheft :: Desire
punishesTheft = fst shakedownParts

threatenAct, complyAct, defyAct, exposeAct :: Action
(threatenAct, complyAct, defyAct, exposeAct) = case snd shakedownParts of
  [t, c, d, e] -> (t, c, d, e)
  acts -> error ("shakedown produced " ++ show (length acts) ++ " actions, expected 4")

blackmailConfessAct :: Action
blackmailConfessAct = confess "lied" together blackmailPat "[Actor]: confess to [Hearer] about the loaf"

waitAct :: Action
waitAct = action "[Actor]: wait" [ Match "at.Actor!P" ] []

fearsScandal :: Int -> Desire
fearsScandal fear = Desire "fears-scandal" (Want [ Match "W.believes.stole.Owner.loaf" ] (negate fear))

-- The v25 conscience shape (as in "conscience" above), keyed to THIS
-- fixture's own deed pattern: -6 while the "lied" mark stands, 0 once
-- confessed -- the residue that makes confessing a merit-based move rather
-- than a label-order tie-break against defy/wait.
blackmailGuilt, blackmailClear :: Desire
blackmailGuilt = Desire "guilt" (Want [ Match "Owner.lied.H.stole.Owner.loaf" ] (-6))
blackmailClear = Desire "clearConscience" (Want [ Match "Owner.confessed.H.stole.Owner.loaf" ] 0)

blackmailPractice :: Practice
blackmailPractice = practice
  { practiceId = "yard", roles = ["R"]
  , actions = [threatenAct, complyAct, defyAct, exposeAct, blackmailConfessAct, waitAct] }

blackmailWorld :: Int -> Int -> PraxState
blackmailWorld price fear =
  setDesires [ punishesTheft, fearsScandal fear, blackmailGuilt, blackmailClear ]
    (foldl (flip performOutcome) base setup)
  where
    base = setCharacters
             [ (character "mel") { charDesires = ["punishes-theft"] }
             , (character "vic") { charWants = [ Want [ Match "debt.mel.vic.favor" ] (negate price) ]
                                  , charDesires = ["fears-scandal", "guilt", "clearConscience"] }
             , character "cora" ]
             (definePractices [blackmailPractice] emptyState)
    setup =
      [ Insert "practice.yard.here"
      , Insert "at.mel!court", Insert "at.vic!court", Insert "at.cora!court"
      , Insert "mel.believes.stole.vic.loaf.seen"
      , Insert "vic.lied.some.stole.vic.loaf" ]

probedArithmeticTests :: TestTree
probedArithmeticTests = testGroup "probed arithmetic: spontaneous confession, blackmail defense"
  [ testCase "spontaneous confession: a mild stake makes conscience relief worth it" $ do
      let w = spontWorld 4
          scores = scoreActions 2 w (member w "wade")
      scoreOf scores "confess to priya about wronging carol" @?= 0.0
      scoreOf scores "hold your tongue"                      @?= (-2.0)
      fmap gaLabel (pickAction 2 w (member w "wade"))
        @?= Just "wade: confess to priya about wronging carol"

  , testCase "spontaneous confession: an expensive stake makes it NOT worth it" $ do
      let w = spontWorld 20
          scores = scoreActions 2 w (member w "wade")
      scoreOf scores "hold your tongue"                      @?= 37.94
      scoreOf scores "confess to priya about wronging carol" @?= 0.0
      fmap gaLabel (pickAction 2 w (member w "wade"))
        @?= Just "wade: hold your tongue"

  , testCase "blackmail defense: a high price against a mild secret -- the victim confesses" $ do
      let w = blackmailWorld 30 3
          threatened = doAct "mel" "threaten vic" w
          scores = scoreActions 2 threatened (member threatened "vic")
      -- confess STRICTLY dominates defy/wait now (conscience relief), not a
      -- tie broken by the alphabet.
      scoreOf scores "confess to cora about the loaf" @?= (-16.259999999999998)
      scoreOf scores "defy mel"                        @?= (-25.259999999999998)
      scoreOf scores "wait"                            @?= (-25.259999999999998)
      scoreOf scores "buy mel's silence"               @?= (-120.06)
      fmap gaLabel (pickAction 2 threatened (member threatened "vic"))
        @?= Just "vic: confess to cora about the loaf"

  , testCase "blackmail defense: after confessing, mel's expose deposits nothing new (dead)" $ do
      let w = blackmailWorld 30 3
          threatened = doAct "mel" "threaten vic" w
          confessed = doAct "vic" "confess to cora about the loaf" threatened
      assertBool "cora now believes, sourced from vic himself"
        (exists "cora.believes.stole.vic.loaf.heard.vic" (db confessed))
      assertBool "mel has no expose action left (every co-present hearer already believes)"
        (not (any (\ga -> "expose" `isInfixOf` gaLabel ga) (possibleActions confessed "mel")))
      map gaLabel (possibleActions confessed "mel") @?= ["mel: wait"]

  , testCase "blackmail defense: the converse -- a cheap price against a severe fear -- complies" $ do
      let w = blackmailWorld 1 30
          threatened = doAct "mel" "threaten vic" w
          scores = scoreActions 2 threatened (member threatened "vic")
      -- comply STRICTLY beats confess here too, even with the same
      -- conscience relief on offer -- the price is cheap enough that paying
      -- it beats spending the secret.
      scoreOf scores "buy mel's silence"               @?= (-159.81)
      scoreOf scores "confess to cora about the loaf" @?= (-162.60000000000002)
      scoreOf scores "defy mel"                        @?= (-171.60000000000002)
      scoreOf scores "wait"                            @?= (-171.60000000000002)
      fmap gaLabel (pickAction 2 threatened (member threatened "vic"))
        @?= Just "vic: buy mel's silence"
  ]

--------------------------------------------------------------------------------
-- Group 5: re-offense snaps the defeater back (v21's idiom, pinned here at
-- module level ahead of the village wiring)
--------------------------------------------------------------------------------

-- A fixed second victim (dana) keeps the action's own precondition free of
-- any query variable beyond Actor -- all that matters here is the v21 idiom
-- itself: a fresh wrongdoing plants a new mark AND deletes the standing
-- defeater, snapping standing back before a now-less-patient audience.
reoffendAct :: Action
reoffendAct = action "[Actor]: wrong dana again"
  [ Match "at.Actor!P" ]
  [ Insert "Actor.lied.edda.wronged.Actor.dana"
  , Delete "recanted.Actor" ]

reoffenseWorld :: PraxState
reoffenseWorld = setCharacters [character "cody", character "gwen"]
  (foldl (flip performOutcome) base setup)
  where
    base = definePractices [p] emptyState
    p = practice
      { practiceId = "confessional", roles = ["R"]
      , actions = [absolveAct, reoffendAct] }
    setup =
      [ Insert "practice.confessional.here"
      , Insert "at.cody!yard"
      , Insert "gwen.believes.wronged.cody.carol.heard.cody" ]

reoffenseTests :: TestTree
reoffenseTests = testGroup "re-offense deletes the defeater (v21 idiom)"
  [ testCase "absolved, then re-offending snaps the defeater away" $ do
      let absolved = doAct "gwen" "absolve cody" reoffenseWorld
      assertBool "recanted stands" (exists "recanted.cody" (db absolved))
      let repeat_ = doAct "cody" "wrong dana again" absolved
      assertBool "recanted is gone: standing snaps back before a less patient audience"
        (not (exists "recanted.cody" (db repeat_)))
      assertBool "the new lied mark is planted"
        (exists "cody.lied.edda.wronged.cody.dana" (db repeat_))
  ]

--------------------------------------------------------------------------------
-- Group 6: guards forced
--------------------------------------------------------------------------------

guardTests :: TestTree
guardTests = testGroup "guards"
  [ testCase "confess rejects a dotted mark kind" $ do
      r <- try (evaluate (length (show (confess "li.ed" together pat "[Actor]: confess"))))
      assertBool "a dotted kind is an error" (isLeft (r :: Either ErrorCall Int))

  , testCase "confess rejects an event pattern reserving H/Hearer/Actor" $ do
      r <- try (evaluate (length (show (confess "lied" together "wronged.H.Victim" "[Actor]: confess"))))
      assertBool "H is reserved (the mark's own hearer slot)" (isLeft (r :: Either ErrorCall Int))

  , testCase "absolve rejects non-single-segment defeater/label" $ do
      r <- try (evaluate (length (show (absolve "re.canted" pat "fedUp" "[Actor]: absolve"))))
      assertBool "a dotted defeater is an error" (isLeft (r :: Either ErrorCall Int))

  , testCase "absolve rejects an event pattern reserving Actor" $ do
      r <- try (evaluate (length (show (absolve "recanted" "wronged.Actor.Victim" "fedUp" "[Actor]: absolve"))))
      assertBool "Actor is reserved" (isLeft (r :: Either ErrorCall Int))

  , testCase "absolve rejects a subject-less event pattern" $ do
      r <- try (evaluate (length (show (absolve "recanted" "somethinghappened" "fedUp" "[Actor]: absolve"))))
      assertBool "an absolution needs a confessor" (isLeft (r :: Either ErrorCall Int))

  , testCase "incorrigible rejects a dotted label" $ do
      r <- try (evaluate (length (show (axiomThen (incorrigible pat 2 "fed.up")))))
      assertBool "a dotted label is an error" (isLeft (r :: Either ErrorCall Int))

  , testCase "incorrigible rejects a pattern reserving W/Ds/N" $ do
      r <- try (evaluate (length (show (axiomThen (incorrigible "wronged.Doer.Ds" 2 "fedUp")))))
      assertBool "Ds is reserved (the subquery's own set variable)" (isLeft (r :: Either ErrorCall Int))

  , testCase "incorrigible rejects a pattern naming no offender" $ do
      r <- try (evaluate (length (show (axiomThen (incorrigible "somethinghappened" 2 "fedUp")))))
      assertBool "a threshold needs an offender" (isLeft (r :: Either ErrorCall Int))

  , testCase "incorrigible rejects a single-variable pattern (no deed variables to count)" $ do
      r <- try (evaluate (length (show (axiomThen (incorrigible "confessed.Doer" 2 "fedUp")))))
      assertBool "one variable can name only one instance -- a k>1 threshold could never fire"
        (isLeft (r :: Either ErrorCall Int))

  , testCase "incorrigible rejects a deed variable colliding with the witness-naming convention" $ do
      r <- try (evaluate (length (show (axiomThen (incorrigible "wronged.Doer.Victim.Victim0" 2 "fedUp")))))
      assertBool "Victim0 would collide with Victim's own outer witness dummy"
        (isLeft (r :: Either ErrorCall Int))
  ]

tests :: TestTree
tests = testGroup "Prax.Confession"
  [ conversionTests
  , absolutionTests
  , incorrigibilityTests
  , probedArithmeticTests
  , reoffenseTests
  , guardTests
  ]
