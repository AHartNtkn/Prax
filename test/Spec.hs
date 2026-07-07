module Main (main) where

import Test.Tasty (defaultMain, testGroup)

import qualified Prax.DbSpec
import qualified Prax.QuerySpec
import qualified Prax.EngineSpec
import qualified Prax.PlannerSpec
import qualified Prax.LoopSpec

main :: IO ()
main = defaultMain $ testGroup "prax"
  [ Prax.DbSpec.tests
  , Prax.QuerySpec.tests
  , Prax.EngineSpec.tests
  , Prax.PlannerSpec.tests
  , Prax.LoopSpec.tests
  ]
