{-# LANGUAGE OverloadedStrings #-}
module Prax.Script.JsonSpec (tests) where

import           Data.Aeson (decode, encode)
import           Data.Either (isLeft)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Query (Condition (..))
import           Prax.Types (Outcome (..))
import           Prax.Script (compile, currentSceneOf)
import           Prax.Script.Json (encodeScript, decodeScript)
import           Prax.Worlds.Play (playScript)

tests :: TestTree
tests = testGroup "Prax.Script.Json"
  [ testCase "a play-script round-trips through JSON exactly" $
      -- structural equality: encode then decode is the identity on the AST,
      -- so the compiled world (and every ending it reaches) is identical too
      decodeScript (encodeScript playScript) @?= Right playScript

  , testCase "the decoded script still compiles to a runnable world" $
      case decodeScript (encodeScript playScript) of
        Right s  -> currentSceneOf (compile s) @?= Just "confidence"
        Left err -> assertBool ("should decode, but: " ++ err) False

  , testCase "malformed JSON reports an error rather than failing silently" $
      assertBool "expected a Left" (isLeft (decodeScript "{ \"start\": 3 }"))

  , testCase "a ForEach outcome round-trips through JSON" $ do
      let o = ForEach [ Match "at.Witness!P", Neq "Witness" "Actor" ]
                      [ Insert "Witness.believes.stole.Actor.loaf!seen" ]
      decode (encode o) @?= Just o
  ]
