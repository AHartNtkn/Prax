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
import qualified Prax.DeonticSpec
import qualified Prax.ELSpec
import qualified Prax.DeriveSpec
import qualified Prax.FeudSpec
import qualified Prax.GoldenDriveSpec
import qualified Prax.ViewInvariantSpec
import qualified Prax.TypeCheckSpec
import qualified Prax.BeliefsSpec
import qualified Prax.WitnessSpec
import qualified Prax.RumorSpec
import qualified Prax.ReputeSpec
import qualified Prax.DeceitSpec
import qualified Prax.MindsSpec
import qualified Prax.PersonaSpec
import qualified Prax.RelevanceSpec
import qualified Prax.ProjectSpec
import qualified Prax.SightSpec
import qualified Prax.ConversationSpec
import qualified Prax.ArcSpec
import qualified Prax.IntrigueSpec
import qualified Prax.StressSpec
import qualified Prax.PersistSpec
import qualified Prax.ScriptSpec
import qualified Prax.Script.JsonSpec
import qualified Prax.DirectorSpec
import qualified Prax.VillageSpec

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
  , Prax.DeonticSpec.tests
  , Prax.ELSpec.tests
  , Prax.DeriveSpec.tests
  , Prax.FeudSpec.tests
  , Prax.GoldenDriveSpec.tests
  , Prax.ViewInvariantSpec.tests
  , Prax.TypeCheckSpec.tests
  , Prax.BeliefsSpec.tests
  , Prax.WitnessSpec.tests
  , Prax.RumorSpec.tests
  , Prax.ReputeSpec.tests
  , Prax.DeceitSpec.tests
  , Prax.MindsSpec.tests
  , Prax.PersonaSpec.tests
  , Prax.RelevanceSpec.tests
  , Prax.ProjectSpec.tests
  , Prax.SightSpec.tests
  , Prax.ConversationSpec.tests
  , Prax.ArcSpec.tests
  , Prax.IntrigueSpec.tests
  , Prax.StressSpec.tests
  , Prax.PersistSpec.tests
  , Prax.ScriptSpec.tests
  , Prax.Script.JsonSpec.tests
  , Prax.DirectorSpec.tests
  , Prax.VillageSpec.tests
  ]
