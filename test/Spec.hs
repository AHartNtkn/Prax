module Main (main) where

import Test.Tasty (defaultMain, testGroup)

import qualified Prax.DbSpec
import qualified Prax.QuerySpec
import qualified Prax.EngineSpec
import qualified Prax.PlannerSpec
import qualified Prax.LoopSpec
import qualified Prax.BarSpec
import qualified Prax.CoreSpec
import qualified Prax.ReactionsSpec
import qualified Prax.BeliefsSpec
import qualified Prax.ConversationSpec

main :: IO ()
main = defaultMain $ testGroup "prax"
  [ Prax.DbSpec.tests
  , Prax.QuerySpec.tests
  , Prax.EngineSpec.tests
  , Prax.PlannerSpec.tests
  , Prax.LoopSpec.tests
  , Prax.BarSpec.tests
  , Prax.CoreSpec.tests
  , Prax.ReactionsSpec.tests
  , Prax.BeliefsSpec.tests
  , Prax.ConversationSpec.tests
  ]
