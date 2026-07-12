module Prax.DeonticSpec (tests) where

import           Data.List (isInfixOf)
import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (dbToSentences)
import           Prax.Query (Condition (..), satisfies)
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, possibleActions, performAction, setCharacters)
import           Prax.Planner (pickAction)
import           Prax.Deontic
import           Prax.Reactions (violationOf)

facts :: PraxState -> [String]
facts = dbToSentences . db

applyAll :: PraxState -> [Outcome] -> PraxState
applyAll = foldl (flip performOutcome)

sat :: PraxState -> [Condition] -> Bool
sat st conds = satisfies (db st) conds Map.empty

-- A minimal "conduct" world: an agent "gil" whose available actions (each a
-- label, its preconditions, and the outcomes it fires) bring about different
-- contents, so the planner's choice is driven purely by its wants.
conductWorld :: [(String, [Condition], [Outcome])] -> [Want] -> PraxState
conductWorld acts wants =
  let p = practice
        { practiceId = "conduct", practiceName = "conduct", roles = ["X"]
        , actions = [ action lbl cs os | (lbl, cs, os) <- acts ] }
      agent = (character "gil") { charWants = wants }
      st0 = setCharacters [agent] (definePractices [p] emptyState)
  in performOutcome (Insert "practice.conduct.gil") st0

gil :: PraxState -> Character
gil st = case [ c | c <- characters st, charName c == "gil" ] of
  (c : _) -> c
  []      -> error "gil not in cast"

tests :: TestTree
tests = testGroup "Prax.Deontic"
  [ testGroup "fact conventions"
    [ testCase "obligationPath is obliged.<who>.<content>" $
        obligationPath "bex" "settle.up" @?= "obliged.bex.settle.up"
    , testCase "isObliged matches the obligation fact" $
        isObliged "bex" "settle.up" @?= Match "obliged.bex.settle.up"
    , testCase "inBreach is a violation (breach reuses the norm machinery)" $
        inBreach "bex" "settle.up" @?= violationOf "bex" "settle.up"
    ]

  , testGroup "obligation lifecycle"
    [ testCase "oblige asserts, discharge retracts" $ do
        let st  = performOutcome (oblige "bex" "settle.up") emptyState
        assertBool "obliged"     ("obliged.bex.settle.up" `elem` facts st)
        assertBool "isObliged"   (sat st [isObliged "bex" "settle.up"])
        let st2 = performOutcome (discharge "bex" "settle.up") st
        assertBool "discharged"  ("obliged.bex.settle.up" `notElem` facts st2)

    , testCase "breach records a violation matched by inBreach" $ do
        let st = performOutcome (breach "bex" "stiffed") emptyState
        assertBool "violated fact" ("violated.bex.stiffed" `elem` facts st)
        assertBool "inBreach"      (sat st [inBreach "bex" "stiffed"])

    , testCase "fulfilled needs the obligation AND its content (ought is not is)" $ do
        let obligedOnly = applyAll emptyState [ oblige "bex" "tipped.ada" ]
            met         = applyAll emptyState [ oblige "bex" "tipped.ada", Insert "tipped.ada" ]
        assertBool "unmet while content absent" (not (sat obligedOnly (fulfilled "bex" "tipped.ada")))
        assertBool "met when content holds"     (sat met (fulfilled "bex" "tipped.ada"))
    ]

  , testGroup "contrary-to-duty (iterated box)"
    [ testCase "obligeReparative nests the obligation (square-square)" $ do
        let st = performOutcome (obligeReparative "bex" "make.amends") emptyState
        assertBool "nested reparative duty"
          ("obliged.bex.obliged.bex.make.amends" `elem` facts st)
        assertBool "matchable as an obligation-about-an-obligation"
          (sat st [ isObliged "bex" (obligationPath "bex" "make.amends") ])
    ]

  , testGroup "conflict detection (paper property 2)"
    [ testCase "incompatible: exclusive values of one slot collapse" $
        conflicts "go!true" "go!false" @?= True
    , testCase "compatible: multi-valued siblings coexist" $
        conflicts "likes.a" "likes.b" @?= False
    , testCase "unrelated slots never conflict" $
        conflicts "tipped.ada" "boarded.train" @?= False
    , testCase "a content does not conflict with itself" $
        conflicts "go!true" "go!true" @?= False
    , testCase "two exclusive locations for one agent conflict" $
        conflicts "practice.world.world.at.bex!bar"
                  "practice.world.world.at.bex!entrance" @?= True
    , testCase "incompatiblePairs finds exactly the incompatible pair" $
        incompatiblePairs ["at!bar", "at!entrance", "tipped.ada"]
          @?= [("at!bar", "at!entrance")]
    ]

  , testGroup "introspection"
    [ testCase "obligationsOf lists an agent's current duties" $ do
        let st = applyAll emptyState [ oblige "bex" "settle.up", oblige "bex" "greet.ada"
                                     , oblige "cai" "leave.now" ]
        assertBool "settle listed" ("settle.up" `elem` obligationsOf "bex" (db st))
        assertBool "greet listed"  ("greet.ada" `elem` obligationsOf "bex" (db st))
        assertBool "other agents excluded"
          (not (any ("leave" `isInfixOf`) (obligationsOf "bex" (db st))))
    ]

  , testGroup "behavioural coupling (planner unchanged)"
    [ testCase "an agent pursues the action that fulfils its obligation" $ do
        -- obliged to reach `did.duty`; one action does it, one doesn't
        let st0 = conductWorld
                    [ ("[Actor]: do the duty", [], [ Insert "did.duty" ])
                    , ("[Actor]: slack off",   [], [ Insert "did.nothing" ]) ]
                    [ wantFulfilled "gil" "did.duty" 20 ]
            st  = performOutcome (oblige "gil" "did.duty") st0
        fmap gaLabel (pickAction 1 st (gil st)) @?= Just "gil: do the duty"

    , testCase "an agent avoids an action that would breach a duty" $ do
        -- re-expresses bex's norm-avoidance through the deontic API
        let st = conductWorld
                   [ ("[Actor]: behave",     [], [])
                   , ("[Actor]: transgress", [], [ breach "gil" "tipping" ]) ]
                   [ avoidBreach "gil" "tipping" 50 ]
        fmap gaLabel (pickAction 1 st (gil st)) @?= Just "gil: behave"

    , testCase "conflicting duties: planner fulfils the higher-valued; the other is foreclosed" $ do
        -- two live duties whose fulfilments compete for one exclusive commitment:
        -- committing to one (single-valued `committed!`) gates out the other, so
        -- the two obligations genuinely cannot both be met.
        let st0 = conductWorld
                    [ ("[Actor]: help alice", [ Not "committed" ]
                      , [ Insert "helped.alice", Insert "committed!alice" ])
                    , ("[Actor]: help bob",   [ Not "committed" ]
                      , [ Insert "helped.bob",   Insert "committed!bob" ]) ]
                    [ wantFulfilled "gil" "helped.alice" 30   -- alice matters more
                    , wantFulfilled "gil" "helped.bob"   10 ]
            st  = applyAll st0 [ oblige "gil" "helped.alice", oblige "gil" "helped.bob" ]
        -- both duties are genuinely held at once (distinct contents ⇒ they coexist)
        assertBool "both duties live"
          (sat st [isObliged "gil" "helped.alice"] && sat st [isObliged "gil" "helped.bob"])
        -- resolution is emergent: the higher-utility duty wins
        let chosen = pickAction 1 st (gil st)
        fmap gaLabel chosen @?= Just "gil: help alice"
        -- having helped alice, bob's duty is foreclosed — still owed, but unmeetable
        let st' = maybe st (performAction st) chosen
        assertBool "helping bob is now foreclosed"
          (not (any (("help bob" `isInfixOf`) . gaLabel) (possibleActions st' "gil")))
        assertBool "bob duty still owed but unfulfilled"
          (sat st' [isObliged "gil" "helped.bob"]
            && not (sat st' (fulfilled "gil" "helped.bob")))
    ]
  ]
