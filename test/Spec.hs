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
import qualified Prax.ArcSpec
import qualified Prax.IntrigueSpec
import qualified Prax.StressSpec
import qualified Prax.PersistSpec
import qualified Prax.ScriptSpec

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
  , Prax.ArcSpec.tests
  , Prax.IntrigueSpec.tests
  , Prax.StressSpec.tests
  , Prax.PersistSpec.tests
  , Prax.ScriptSpec.tests
  ]
