module Main (main) where

import Test.Tasty (defaultMain, testGroup)

import qualified Prax.DbSpec
import qualified Prax.QuerySpec
import qualified Prax.EngineSpec

main :: IO ()
main = defaultMain $ testGroup "prax"
  [ Prax.DbSpec.tests
  , Prax.QuerySpec.tests
  , Prax.EngineSpec.tests
  ]
