{-# LANGUAGE OverloadedStrings #-}
module Prax.Script.JsonSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Aeson (decode, encode)
import           Data.Either (isLeft)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Query (CalcOp (..), Condition (..))
import           Prax.Types (Outcome (..))
import           Prax.Script (compile, currentSceneOf, timeout)
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

  , testCase "every CalcOp round-trips through JSON, Mod included" $ do
      let cs = [ Calc "R" op "17" "5" | op <- [Add, Sub, Mul, Mod] ]
      decode (encode cs) @?= Just cs

  , testCase "a ForEach outcome round-trips through JSON" $ do
      let o = ForEach [ Match "at.Witness!P", Neq "Witness" "Actor" ]
                      [ Insert "Witness.believes.stole.Actor.loaf.seen" ]
      decode (encode o) @?= Just o

  , testCase "an InsertFor outcome round-trips through JSON" $
      decode (encode (InsertFor 3 "mood!a")) @?= Just (InsertFor 3 "mood!a")

    -- v44 fix wave 2: the Junction "after" field, and the uniform compile-time
    -- guard reachable from JSON (JSON has no other way to author a timeout: it
    -- carries no "after" tag pre-fix, so a JSON author's only route to a timed
    -- junction was spelling PraxE/PraxNow/PraxD out literally in "when") ------
  , testCase "a timed junction's \"after\" field round-trips through JSON" $ do
      let j = timeout "gaveUp" 5
      decode (encode j) @?= Just j

  , testCase "decoding+compiling the reviewer's JSON repro rejects the \
             \Prax-namespaced goto condition (guard-trigger, at the JSON \
             \authoring surface FromJSON Junction decodes straight into)" $ do
      let json = "{ \"start\": \"a\", \
                  \  \"cast\": [ { \"name\": \"p\", \"playable\": true } ], \
                  \  \"scenes\": [ \
                  \    { \"id\": \"a\", \"junctions\": [ \
                  \        { \"name\": \"go\", \"to\": \"b\", \"when\": \
                  \            [ { \"match\": \"chapter!PraxNow\" } ] } ] }, \
                  \    { \"id\": \"b\" } ] }"
      case decodeScript json of
        Left err -> assertBool ("expected to decode, but: " ++ err) False
        Right sc -> do
          r <- try (evaluate (currentSceneOf (compile sc)))
          assertBool "compile rejects the JSON-authored PraxNow goto condition"
            (isLeft (r :: Either ErrorCall (Maybe String)))
  ]
