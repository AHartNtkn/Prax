module Prax.SymSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))
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
  ]
