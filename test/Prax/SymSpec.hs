module Prax.SymSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, assertFailure, (@?=))
import           Prax.Sym

tests :: TestTree
tests = testGroup "Prax.Sym"
  [ testCase "intern/symName round-trips and is idempotent" $ do
      let a = intern "square"
      symName a @?= "square"
      intern "square" @?= a
  , testCase "variable-ness is packed into parity" $ do
      assertBool "constants are not variables" (not (symIsVar (intern "square")))
      assertBool "uppercase-initial segments are" (symIsVar (intern "Actor"))
      assertBool "empty segment is a constant" (not (symIsVar (intern "")))
  , testCase "distinct names get distinct symbols" $
      assertBool "" (intern "mill" /= intern "square")
  , testCase "symId/symOfId round-trip (Prax.Db's IntMap-keying escape hatch)" $ do
      let a = intern "gazebo"
      symOfId (symId a) @?= a
      symName (symOfId (symId a)) @?= "gazebo"
  , testCase "distinct symbols have distinct ids" $
      assertBool "" (symId (intern "alpha") /= symId (intern "beta"))
  , testCase "symName forces its argument before touching the pool (fresh, unforced Sym thunk)" $ do
      -- Regression for the same class of bug fixed in intern/symOfId:
      -- newtype pattern matches (Sym i) do NOT force their scrutinee (a
      -- newtype has no runtime tag to match against), so symName's body can
      -- run readIORef before intern's own pool write for a brand-new name
      -- has happened, if the Sym argument is still an unforced thunk. Route
      -- the thunk through a list-pattern-match indirection so GHC's
      -- strictness analyzer can't trivially see through it (see the report
      -- for whether this reproduces RED at the shipped -O1 or only -O0:
      -- GHC's own strictness analysis of symName's *definition* forces the
      -- ordering regardless of how the caller shapes the argument, so this
      -- is expected to stay green at -O1 either way).
      case [intern "v29t2-fresh-name"] of
        (s : _) -> symName s @?= "v29t2-fresh-name"
        []      -> assertFailure "impossible: singleton list"
  ]
