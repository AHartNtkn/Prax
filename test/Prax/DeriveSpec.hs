-- | Tests for the forward-chaining derivation layer: relational closure, the
-- closure-as-view / defeasibility property, auto-□-lifting, and — the point of
-- moving to @m(X)@ — exact contradiction (⊥) detection.
module Prax.DeriveSpec (tests) where

import           Data.Either (isRight)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (Db, emptyDb, insertAll, retract, dbToSentences)
import           Prax.Query (Condition (..))
import           Prax.Derive

build :: [String] -> Db
build ss = insertAll ss emptyDb

closedFacts :: [Axiom] -> Db -> [String]
closedFacts axs db = either (const []) dbToSentences (closure axs db)

has :: String -> [String] -> Bool
has = elem

tests :: TestTree
tests = testGroup "Prax.Derive (m(X) closure)"
  [ testCase "no axioms ⇒ the base is returned unchanged" $
      closure [] (build ["a.b"]) @?= Right (build ["a.b"])

  , testCase "a single domain rule derives a consequence" $ do
      let axs  = [ axiom [ Match "at.W.bar" ] [ "in.W.building" ] ]
      assertBool "in.bex.building derived" (has "in.bex.building" (closedFacts axs (build ["at.bex.bar"])))

  , testCase "closure reaches a multi-step (transitive) fixpoint" $ do
      let axs = [ axiom [ Match "reaches.X.Y", Match "reaches.Y.Z" ] [ "reaches.X.Z" ] ]
          d = derived axs (build ["reaches.a.b", "reaches.b.c", "reaches.c.d"])
      assertBool "a→c" ("reaches.a.c" `elem` d)
      assertBool "a→d" ("reaches.a.d" `elem` d)
      assertBool "b→d" ("reaches.b.d" `elem` d)

  , testCase "relational join with variable binding (grandparent)" $
      derived [ axiom [ Match "parent.X.Y", Match "parent.Y.Z" ] [ "grandparent.X.Z" ] ]
              (build ["parent.tom.bob", "parent.bob.ann"])
        @?= [ "grandparent.tom.ann" ]

  , testCase "closure is a VIEW: base untouched, so derivation is defeasible" $ do
      let axs  = [ axiom [ Match "at.W.bar" ] [ "in.W.building" ] ]
          base = build ["at.bex.bar"]
      assertBool "closure has it" (has "in.bex.building" (closedFacts axs base))
      assertBool "base does not"  (not ("in.bex.building" `elem` dbToSentences base))
      -- retract the premise, re-close: the conclusion is gone (no manual undo)
      let base' = retract "at.bex.bar" base
      assertBool "conclusion retracts too" (not (has "in.bex.building" (closedFacts axs base')))

  , testCase "AUTO-□-lift: a domain rule (written once) also closes under obligation" $ do
      -- only the world rule is authored; the obligation form is lifted for free
      let axs  = [ axiom [ Match "at.W.bar" ] [ "in.W.building" ] ]
          base = build ["obliged.bex.at.bex.bar"]   -- bex ought to be at the bar
      assertBool "sub-obligation derived"
        (has "obliged.bex.in.bex.building" (closedFacts axs base))

  , testCase "⊥ DETECTED: rules forcing one exclusive slot to two values contradict" $ do
      -- the spike's old pitfall — now caught exactly, and order-independently
      let a1 = axiom [ Match "trigger" ] [ "light!red" ]
          a2 = axiom [ Match "trigger" ] [ "light!green" ]
          base = build ["trigger"]
      assertBool "closure reports a contradiction, either order"
        (not (isRight (closure [a1, a2] base)) && not (isRight (closure [a2, a1] base)))
      assertBool "the ⊥ witness names an offending head"
        (contradiction [a1, a2] base `elem` [ Just "light!red", Just "light!green" ])

  , testCase "consistent exclusive derivation is fine (no false ⊥)" $ do
      let axs  = [ axiom [ Match "wedding.W" ] [ "status.W!married" ] ]
          base = build ["wedding.bex"]
      assertBool "no contradiction" (isRight (closure axs base))
      assertBool "status derived" (has "status.bex.married" (closedFacts axs base))

  , testCase "⊥ from EITHER side: a derived multi value clashes with a base EXCLUSIVE fact" $ do
      -- The base marks the slot exclusive with `!`; the rule derives a *different*
      -- value with `.`. Because the world state retains its labels, meet catches
      -- this even though the derived head is not itself exclusive.
      let axs  = [ axiom [ Match "summoned.W" ] [ "place.W.hall" ] ]
          base = build [ "place.bex!bar", "summoned.bex" ]   -- bex is exclusively at the bar
      assertBool "contradiction detected" (not (isRight (closure axs base)))
      contradiction axs base @?= Just "place.bex.hall"
  ]
