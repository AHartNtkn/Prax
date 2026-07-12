module Prax.DebtSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Either (isLeft)
import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, possibleActions, performAction, setAxioms, setCharacters)
import           Prax.Deontic (oblige, discharge, obligationPath, breach)
import           Prax.Witness (CoPresence, observable)
import           Prax.Repute (standingUnless)
import           Prax.Debt

applyAll :: PraxState -> [Outcome] -> PraxState
applyAll = foldl (flip performOutcome)

-- The tale: cora lends dell a favor. When dell stiffs her, cora calls it in
-- (co-present) — a witnessed default sours dell's standing with whoever was
-- there to see it; an unwitnessed one leaves no such trace. Repaying settles
-- the debt AND the standing, though the witness's own memory never clears.
together :: CoPresence
together = [ Match "at.Actor!P", Match "at.Witness!P" ]

demand :: Action
demand = observable together "violated.dell.defaulted" $
  action "[Actor]: demand dell repay the favor"
    [ owes "Actor" "dell" "favor"
    , Match "at.Actor!P", Match "at.dell!P"
    , Not "demanded.Actor.dell.favor" ]
    [ Insert "demanded.Actor.dell.favor"
    , breach "dell" "defaulted" ]

repay :: Action
repay = action "[Actor]: dell repays cora the favor"
  [ Eq "Actor" "dell", owes "cora" "Actor" "favor" ]
  (settle "cora" "dell" "favor"
    ++ [ Insert "atoned.dell", Delete "violated.dell.defaulted" ])

debtPractice :: Practice
debtPractice = practice
  { practiceId = "debt", roles = ["R"]
  , actions = [ demand, repay ] }

-- @witnessPresent@ toggles whether ren shares cora and dell's square (versus
-- being off at the mill) when the default happens.
mkWorld :: Bool -> PraxState
mkWorld witnessPresent =
  setAxioms [ standingUnless "violated.Debtor.defaulted" "atoned.Debtor" "deadbeat" ]
    (foldl (flip performOutcome) base setup)
  where
    base = setCharacters (map character ["cora", "dell", "ren"])
             (definePractices [debtPractice] emptyState)
    setup =
      [ Insert "practice.debt.here" ]
      ++ owe "cora" "dell" "favor"
      ++ [ Insert "at.cora!square", Insert "at.dell!square"
         , Insert ("at.ren!" ++ if witnessPresent then "square" else "mill") ]

witnessedWorld :: PraxState
witnessedWorld = mkWorld True

aloneWorld :: PraxState
aloneWorld = mkWorld False

-- Perform the named actor's action whose label mentions @needle@.
doAct :: String -> String -> PraxState -> PraxState
doAct who needle st =
  case [ ga | ga <- possibleActions st who, needle `isInfixOf` gaLabel ga ] of
    (ga : _) -> performAction st ga
    []       -> error ("no action for " ++ who ++ " matching " ++ show needle
                       ++ "; had: " ++ show (map gaLabel (possibleActions st who)))

tests :: TestTree
tests = testGroup "Prax.Debt"
  [ testGroup "fact conventions"
    [ testCase "debtPath is debt.<creditor>.<debtor>.<content>" $
        debtPath "cora" "dell" "favor" @?= "debt.cora.dell.favor"
    , testCase "owes matches the debt fact" $
        owes "cora" "dell" "favor" @?= Match "debt.cora.dell.favor"
    , testCase "owe asserts the debt fact and Deontic's oblige, in one call" $
        owe "cora" "dell" "favor"
          @?= [ Insert "debt.cora.dell.favor", oblige "dell" "favor" ]
    , testCase "settle retracts the debt fact and Deontic's discharge" $
        settle "cora" "dell" "favor"
          @?= [ Delete "debt.cora.dell.favor", discharge "dell" "favor" ]
    ]

  , testGroup "lifecycle: owe creates BOTH facts; settle removes BOTH"
    [ testCase "owe inserts the debt fact and the obligation; settle removes both" $ do
        let content = "repaid.dell.cora.coin"
            owed = applyAll emptyState (owe "cora" "dell" content)
        assertBool "debt fact"
          (exists "debt.cora.dell.repaid.dell.cora.coin" (db owed))
        assertBool "obligation fact"
          (exists (obligationPath "dell" content) (db owed))
        let settled = applyAll owed (settle "cora" "dell" content)
        assertBool "debt fact gone"
          (not (exists "debt.cora.dell.repaid.dell.cora.coin" (db settled)))
        assertBool "obligation gone"
          (not (exists (obligationPath "dell" content) (db settled)))
    ]

  , testGroup "guards"
    [ testCase "a dotted creditor name errors loudly" $ do
        r <- try (evaluate (length (debtPath "cor.a" "dell" "favor")))
        assertBool "dotted creditor is an error" (isLeft (r :: Either ErrorCall Int))
    , testCase "a punctuated debtor name errors loudly" $ do
        r <- try (evaluate (length (debtPath "cora" "de!l" "favor")))
        assertBool "punctuated debtor is an error" (isLeft (r :: Either ErrorCall Int))
    ]

  , testGroup "demand -> deadbeat: belief-gated standing, defeated by settling"
    [ testCase "an unwitnessed default derives no third-party regard" $ do
        let st = doAct "cora" "demand" aloneWorld
        assertBool "the world records the breach" (exists "violated.dell.defaulted" (db st))
        assertBool "ren, elsewhere, holds no belief of it"
          (not (exists "ren.believes.violated.dell.defaulted.seen" (db st)))
        assertBool "and so derives no THIRD-PARTY deadbeat regard"
          (not (exists "regards.ren.dell.deadbeat" (readView st)))
        assertBool "dell is unavoidably co-present at his own default, so he still\
                   \ witnesses (and regards) himself — belief-gating is per observer,\
                   \ not a blanket suppression"
          (exists "regards.dell.dell.deadbeat" (readView st))

    , testCase "a witnessed default derives deadbeat standing for the witness" $ do
        let st = doAct "cora" "demand" witnessedWorld
        assertBool "ren saw it"
          (exists "ren.believes.violated.dell.defaulted.seen" (db st))
        assertBool "ren regards dell a deadbeat"
          (exists "regards.ren.dell.deadbeat" (readView st))

    , testCase "settling (atoned-shaped) defeats the standing; the witness's memory persists" $ do
        let st1 = doAct "cora" "demand" witnessedWorld
            st2 = doAct "dell" "repays" st1
        assertBool "settled: the debt is gone"
          (not (exists "debt.cora.dell.favor" (db st2)))
        assertBool "no regard survives settling"
          (not (exists "regards.ren.dell.deadbeat" (readView st2)))
        assertBool "ren still remembers the default"
          (exists "ren.believes.violated.dell.defaulted.seen" (db st2))
    ]
  ]
