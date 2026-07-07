module Main (main) where

import Test.Tasty (defaultMain, testGroup)

import qualified Prax.DbSpec

main :: IO ()
main = defaultMain $ testGroup "prax"
  [ Prax.DbSpec.tests
  ]
