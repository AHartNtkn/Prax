module Prax.PersonaSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Either (isLeft)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists)
import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, setAxioms)
import qualified Prax.Planner as Planner
import           Prax.Minds (selfWants)
import           Prax.Witness (CoPresence)
import           Prax.Deceit (lie)
import           Prax.Persona

together :: CoPresence
together = [ Match "at.Actor!P", Match "at.Witness!P" ]

-- Temperament: each of your own lie-marks costs you 6 — more than the 4 a
-- deceived head is worth below, less than everything. Costs, not prohibitions.
plainspoken :: Trait
plainspoken = Trait "plainspoken"
  [ Desire "clean-conscience" (Want [ Match "Owner.lied.H.took.C.gem" ] (-6)) ]

-- The tale: ada and bea both covet oz's credulity (+4 if oz believes kit took
-- the gem — a want only a lie can serve here, the temptation); ada is
-- plainspoken, bea is her unprincipled twin. Same motive, one temperament.
world :: PraxState
world = foldl (flip performOutcome) base (personaFacts ++ setup)
  where
    (roster, personaFacts) = cast [plainspoken]
      [ ( (character "ada") { charDesires = ["covets-credulity"] }, [plainspoken] )
      , ( (character "bea") { charDesires = ["covets-credulity"] }, [] )
      , ( character "oz", [] )
      , ( character "kit", [] ) ]
    base = (setAxioms [transparent] (definePractices [p] emptyState))
             { characters = roster
             , desires = Desire "covets-credulity"
                           (Want [ Match "oz.believes.took.kit.gem" ] 4)
                         : personaVocabulary [plainspoken]
             }
    p = practice
      { practiceId = "yard", roles = ["R"]
      , actions =
          [ lie together [] [ Match "at.Culprit!Anywhere" ] "took.Culprit.gem"
              "[Actor]: whisper to [Hearer] that [Culprit] took the gem"
          , action "[Actor]: hold your peace" [ Match "at.Actor!P" ] []
          ] }
    setup =
      [ Insert "practice.yard.here"
      , Insert "at.ada!yard", Insert "at.bea!yard"
      , Insert "at.oz!yard", Insert "at.kit!yard" ]

member :: String -> Character
member n = case [ c | c <- characters world, charName c == n ] of
  (c : _) -> c
  []      -> error ("no such member: " ++ n)

tests :: TestTree
tests = testGroup "Prax.Persona"
  [ testCase "bearing endows a character with the trait's desires by name" $
      charDesires (bearing plainspoken (character "zed"))
        @?= ["clean-conscience"]

  , testCase "cast assembles the roster and the facts transparent reads" $ do
      assertBool "ada bears the trait" (exists "trait.ada.plainspoken" (db world))
      assertBool "the trait's desires are data facts"
        (exists "traitDesire.plainspoken.clean-conscience" (db world))
      assertBool "every member is a character fact"
        (all (\n -> exists ("character." ++ n) (db world))
             ["ada", "bea", "oz", "kit"])
      assertBool "bea bears nothing" (not (exists "trait.bea" (db world)))
      charDesires (member "ada") @?= ["covets-credulity", "clean-conscience"]

  , testCase "personaVocabulary rejects duplicate desire names loudly" $ do
      r <- try (evaluate (length (personaVocabulary
             [ plainspoken
             , Trait "echo" [ Desire "clean-conscience"
                                (Want [ Match "x.y" ] 1) ] ])))
      assertBool "bundles must not collide" (isLeft (r :: Either ErrorCall Int))

  , testCase "cast rejects a dotted trait name loudly" $ do
      r <- try (evaluate (length (show (snd
             (cast [ Trait "two.part" [] ] [])))))
      assertBool "a trait name is a single path segment"
        (isLeft (r :: Either ErrorCall Int))

  , testCase "cast rejects a borne trait missing from the vocabulary" $ do
      r <- try (evaluate (length (show (snd
             (cast [] [ (character "zed", [plainspoken]) ])))))
      assertBool "a stray bearing would be silently illegible"
        (isLeft (r :: Either ErrorCall Int))

  , testCase "temperament is legible: everyone presumes a bearer's valuations" $ do
      let v = readView world
      assertBool "oz presumes ada's conscience"
        (exists "oz.believes.desires.ada.clean-conscience.presumed" v)
      assertBool "bea presumes it too"
        (exists "bea.believes.desires.ada.clean-conscience.presumed" v)
      assertBool "no conscience is presumed of bea (she bears no trait)"
        (not (exists "oz.believes.desires.bea.clean-conscience" v))
      assertBool "the covets want, unheralded, is presumed of no one"
        (not (exists "oz.believes.desires.ada.covets-credulity" v))

  , testCase "the conduct-valuation core: the temptation splits the twins" $ do
      -- identical motives, identical affordances; only the trait differs.
      fmap gaLabel (Planner.pickAction 2 world (member "bea"))
        @?= Just "bea: whisper to oz that kit took the gem"
      fmap gaLabel (Planner.pickAction 2 world (member "ada"))
        @?= Just "ada: hold your peace"

  , testCase "a conscience with a memory: each mark costs again, and forgetting relieves" $ do
      -- plant one prior lie on ada's psyche: the NEXT lie still nets -2
      -- (no fall-from-grace discount) ...
      let st = performOutcome (Insert "ada.lied.bea.took.oz.gem") world
      fmap gaLabel (Planner.pickAction 2 st (member "ada"))
        @?= Just "ada: hold your peace"
      -- ... and the arithmetic is per-mark, exactly:
      Planner.evaluate st (selfWants st (member "ada")) @?= (-6)
      let st2 = performOutcome (Insert "ada.lied.oz.took.bea.gem") st
      Planner.evaluate st2 (selfWants st2 (member "ada")) @?= (-12)
      let relieved = performOutcome (Delete "ada.lied") st2
      Planner.evaluate relieved (selfWants relieved (member "ada")) @?= 0

  , testCase "a believed conscience nets against a believed motive in prediction" $ do
      -- kit is told both women covet oz's credulity; ada's conscience he has
      -- presumed all along (transparent). Motive alone predicts the whisper;
      -- motive netted against conscience predicts nothing.
      let seer = foldl (flip performOutcome) world
                   [ Insert "kit.believes.desires.ada.covets-credulity.seen"
                   , Insert "kit.believes.desires.bea.covets-credulity.seen" ]
      fmap gaLabel (Planner.predictMove seer (member "kit") (member "bea"))
        @?= Just "bea: whisper to oz that kit took the gem"
      Planner.predictMove seer (member "kit") (member "ada") @?= Nothing
  ]
