-- | Tests for the Exclusion-Logic lattice core, checked against the DEON paper's
-- own semantics (Def 6–8): meet/⊥, the information order, and round-tripping.
module Prax.ELSpec (tests) where

import           Data.Maybe (isJust, isNothing)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.EL

-- meet, then read back the sentences (Nothing = ⊥)
meetS :: [String] -> [String] -> Maybe [String]
meetS a b = toSentences <$> meet (fromSentences a) (fromSentences b)

tests :: TestTree
tests = testGroup "Prax.EL (exclusion-logic lattice)"
  [ testGroup "sentences round-trip"
    [ testCase "multi-valued siblings are kept" $
        toSentences (fromSentences ["a.b", "a.c"]) @?= ["a.b", "a.c"]
    , testCase "a nested path survives" $
        toSentences (fromSentences ["x.y.z"]) @?= ["x.y.z"]
    ]

  , testGroup "meet ⊓ (Def 8) and incompatibility ⊥ (Def 7)"
    [ testCase "compatible multi facts conjoin" $
        meetS ["a.b"] ["a.c"] @?= Just ["a.b", "a.c"]
    , testCase "the same exclusive fact is idempotent" $
        meetS ["x!a"] ["x!a"] @?= Just ["x.a"]
    , testCase "exclusive slot forced to two values is ⊥" $
        assertBool "⊥" (isNothing (meet (fromSentences ["x!a"]) (fromSentences ["x!b"])))
    , testCase "an exclusive claim vs a different multi child is still ⊥" $
        -- x!a says a is the ONLY child; x.b adds another ⇒ incompatible
        assertBool "⊥" (isNothing (meet (sentenceToNode "x!a") (sentenceToNode "x.b")))
    , testCase "two multi children never conflict" $
        assertBool "ok" (isJust (meet (sentenceToNode "x.a") (sentenceToNode "x.b")))
    , testCase "a conflict deep in the tree propagates to ⊥" $
        assertBool "⊥" (isNothing (meet (sentenceToNode "p.q.r!a") (sentenceToNode "p.q.r!b")))
    , testCase "disjoint slots conjoin freely" $
        meetS ["at!bar"] ["mood!happy"] @?= Just ["at.bar", "mood.happy"]
    ]

  , testGroup "information order ≤ (Def 6): a ≤ b means a entails b"
    [ testCase "more facts entail fewer" $
        assertBool "a.b,a.c ≤ a.b" (leq (fromSentences ["a.b", "a.c"]) (fromSentences ["a.b"]))
    , testCase "fewer facts do NOT entail more" $
        assertBool "a.b ⋠ a.b,a.c" (not (leq (fromSentences ["a.b"]) (fromSentences ["a.b", "a.c"])))
    , testCase "a specific label entails the general (Excl ≤ Multi)" $
        assertBool "x!a ≤ x.a" (leq (sentenceToNode "x!a") (sentenceToNode "x.a"))
    , testCase "the general does NOT entail the specific (Multi ⋠ Excl)" $
        assertBool "x.a ⋠ x!a" (not (leq (sentenceToNode "x.a") (sentenceToNode "x!a")))
    , testCase "everything entails the empty model" $
        assertBool "a.b ≤ ⊤" (leq (fromSentences ["a.b"]) leaf)
    ]
  ]
