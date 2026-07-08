-- | Tests for the Exclusion-Logic lattice, checked against the DEON paper's own
-- semantics (Def 6–8): meet/⊥, the information order, over the labeled 'Db'.
module Prax.ELSpec (tests) where

import           Data.Maybe (isJust, isNothing)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (Db, emptyDb, insert, insertAll, dbToSentences)
import           Prax.EL (meet, leq)

s1 :: String -> Db          -- a single labeled fact as a model
s1 x = insert x emptyDb

mk :: [String] -> Db        -- a model from several facts
mk = flip insertAll emptyDb

meetS :: [String] -> [String] -> Maybe [String]
meetS a b = dbToSentences <$> meet (mk a) (mk b)

tests :: TestTree
tests = testGroup "Prax.EL (exclusion-logic lattice)"
  [ testGroup "meet ⊓ (Def 8) and incompatibility ⊥ (Def 7)"
    [ testCase "compatible multi facts conjoin" $
        meetS ["a.b"] ["a.c"] @?= Just ["a.b", "a.c"]
    , testCase "the same exclusive fact is idempotent" $
        meetS ["x!a"] ["x!a"] @?= Just ["x.a"]
    , testCase "exclusive slot forced to two values is ⊥" $
        assertBool "⊥" (isNothing (meet (s1 "x!a") (s1 "x!b")))
    , testCase "an exclusive claim vs a different multi child is still ⊥ (either side)" $ do
        assertBool "excl ⊓ multi" (isNothing (meet (s1 "x!a") (s1 "x.b")))
        assertBool "multi ⊓ excl" (isNothing (meet (s1 "x.b") (s1 "x!a")))
    , testCase "two multi children never conflict" $
        assertBool "ok" (isJust (meet (s1 "x.a") (s1 "x.b")))
    , testCase "a conflict deep in the tree propagates to ⊥" $
        assertBool "⊥" (isNothing (meet (s1 "p.q.r!a") (s1 "p.q.r!b")))
    , testCase "disjoint slots conjoin freely" $
        meetS ["at!bar"] ["mood!happy"] @?= Just ["at.bar", "mood.happy"]
    ]

  , testGroup "information order ≤ (Def 6): a ≤ b means a entails b"
    [ testCase "more facts entail fewer" $
        assertBool "a.b,a.c ≤ a.b" (leq (mk ["a.b", "a.c"]) (mk ["a.b"]))
    , testCase "fewer facts do NOT entail more" $
        assertBool "a.b ⋠ a.b,a.c" (not (leq (mk ["a.b"]) (mk ["a.b", "a.c"])))
    , testCase "a specific label entails the general (Excl ≤ Multi)" $
        assertBool "x!a ≤ x.a" (leq (s1 "x!a") (s1 "x.a"))
    , testCase "the general does NOT entail the specific (Multi ⋠ Excl)" $
        assertBool "x.a ⋠ x!a" (not (leq (s1 "x.a") (s1 "x!a")))
    , testCase "everything entails the empty model" $
        assertBool "a.b ≤ ⊤" (leq (mk ["a.b"]) emptyDb)
    ]
  ]
