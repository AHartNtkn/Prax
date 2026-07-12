-- | Tests for the forward-chaining derivation layer: relational closure, the
-- closure-as-view / defeasibility property, auto-□-lifting, and — the point of
-- moving to @m(X)@ — exact contradiction (⊥) detection.
module Prax.DeriveSpec (tests) where

import           Data.Either (isRight)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (Db, emptyDb, insert, insertAll, retract, dbToSentences, dbToLabeledSentences)
import           Prax.Query (Condition (..), CmpOp (..), CalcOp (..))
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
      assertBool "base does not"  ("in.bex.building" `notElem` dbToSentences base)
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

  , testCase "axiomFootprint collects bodies (any polarity), heads, and lifted forms" $ do
      let ax = axiom [ Match "parent.X.Y", Absent [ Match "dead.X" ] ] [ "elder.X" ]
          fp = axiomFootprint [ax]
      assertBool "body atom"          ("parent.X.Y" `elem` fp)
      assertBool "negated body atom"  ("dead.X" `elem` fp)
      assertBool "head"               ("elder.X" `elem` fp)
      -- an Absent body blocks □-lifting; an all-Match rule contributes both
      -- lifted body and lifted head:
      let fp2 = axiomFootprint [ axiom [ Match "a.X" ] [ "b.X" ] ]
      assertBool "lifted body" ("obliged.Obligor.a.X" `elem` fp2)
      assertBool "lifted head" ("obliged.Obligor.b.X" `elem` fp2)

  , testCase "closureFrom continues a closed model exactly as a from-scratch closure" $ do
      let axs = [ axiom [ Match "parent.X.Y" ] [ "elder.X" ]
                , axiom [ Match "elder.X", Match "wise.X" ] [ "sage.X" ] ]
          base = insertAll [ "parent.ada.bea", "wise.ada" ] emptyDb
          -- a monotone new fact cascading through BOTH rules:
          closed  = either (error . show) id (closure axs base)
          cont    = either (error . show) id (closureFrom axs closed [ "parent.cal.dun" ])
          scratch = either (error . show) id (closure axs (insert "parent.cal.dun" base))
      dbToLabeledSentences cont @?= dbToLabeledSentences scratch

  , testCase "axiomNegPatterns collects exactly the negated interiors" $ do
      let axs = [ axiom [ Match "a.X", Absent [ Match "b.X", Not "c.X" ] ] [ "d.X" ] ]
      assertBool "Absent interior" ("b.X" `elem` axiomNegPatterns axs)
      assertBool "Not inside Absent" ("c.X" `elem` axiomNegPatterns axs)
      assertBool "positive atom is NOT negated" ("a.X" `notElem` axiomNegPatterns axs)

  , testCase "monotoneAxioms accepts the count-threshold shape and rejects anti-monotone" $ do
      assertBool "Match-only is safe" (monotoneAxioms [ axiom [ Match "a.X" ] [ "b.X" ] ])
      assertBool "the notoriety shape (Subquery+Count+Cmp Gte literal) is safe"
        (monotoneAxioms [ axiom [ Subquery "Rs" ["W"] [ Match "r.W.T" ]
                                , Count "N" "Rs", Cmp Gte "N" "3" ] [ "n.T" ] ])
      assertBool "Cmp Lt with the literal on the right is anti-monotone"
        (not (monotoneAxioms [ axiom [ Count "N" "Rs", Cmp Lt "N" "3" ] [ "q.T" ] ]))
      assertBool "Calc disables the tier"
        (not (monotoneAxioms [ axiom [ Calc "M" Add "N" "1" ] [ "q.M" ] ]))
  ]
