-- | The v40 world-source gate, made durable: the historical manual grep-gate
-- ("no world authors a Prax-namespaced variable") as a committed test, plus a
-- pin on the shared guard itself ("Prax.Types.authoredVarClash") that proves
-- its walker coverage (conditions AND outcomes, ForEach and Subquery
-- internals included — the v38 walkers it reuses).
module Prax.GateSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Char (isAlphaNum, isUpper)
import           Data.Either (isLeft)
import           Data.List (isSuffixOf, sort)
import           System.Directory (listDirectory)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Query (Condition (..))
import           Prax.Types (Outcome (..))
import           Prax.Rng (draw)

worldsDir :: FilePath
worldsDir = "src/Prax/Worlds"

-- | Every quoted string literal's content in a Haskell source file, found by
-- a naive scan for unescaped double quotes (good enough for authored
-- path\/pattern literals — the gate's job is to be simple and loud, not a
-- full Haskell lexer).
stringLiterals :: String -> [String]
stringLiterals [] = []
stringLiterals ('"' : rest) = let (lit, rest') = capture rest in lit : stringLiterals rest'
  where
    capture [] = ([], [])
    capture ('\\' : c : more) = let (l, r) = capture more in (c : l, r)
    capture ('"' : more) = ([], more)
    capture (c : more) = let (l, r) = capture more in (c : l, r)
stringLiterals (_ : rest) = stringLiterals rest

-- | Every @Prax[A-Z][A-Za-z0-9]*@-shaped token occurring anywhere in a
-- string — a substring scan, not a path-segment parse: false positives are
-- acceptable-conservative (the gate's own charter), absence of the scanner
-- is not.
praxTokens :: String -> [String]
praxTokens [] = []
praxTokens s@('P' : 'r' : 'a' : 'x' : c : rest)
  | isUpper c = let (more, rest') = span isAlphaNum rest
                in ("Prax" ++ c : more) : praxTokens rest'
  | otherwise = praxTokens (drop 1 s)
praxTokens (_ : rest) = praxTokens rest

-- | Every Prax-namespaced token found inside any quoted string literal of a
-- source file's content — the world-source gate's actual check.
worldSourceOffenders :: String -> [String]
worldSourceOffenders = concatMap praxTokens . stringLiterals

tests :: TestTree
tests = testGroup "Prax.Gate"
  [ testGroup "the scanner (mutation evidence: it must actually discriminate)"
    [ testCase "catches a Prax-namespaced token inside a quoted literal" $
        worldSourceOffenders "mkPat x = Match \"foo.PraxD.bar\"" @?= ["PraxD"]

    , testCase "catches more than one offender, in order" $
        worldSourceOffenders "[Match \"a.PraxW.b\", Match \"c.PraxF!d\"]"
          @?= ["PraxW", "PraxF"]

    , testCase "ignores ordinary quoted literals with no Prax-shaped token" $
        worldSourceOffenders "action \"[Actor]: greet [Other]\" [Match \"at.Actor!P\"] []" @?= []

    , testCase "ignores unquoted text (imports, comments) even if Prax-shaped" $
        worldSourceOffenders "import Prax.Drift (driftP) -- see PraxTypes.hs" @?= []
    ]

  , testCase "no world source file authors a Prax-namespaced variable in a quoted literal" $ do
      names <- listDirectory worldsDir
      let hsFiles = sort [ worldsDir ++ "/" ++ n | n <- names, ".hs" `isSuffixOf` n ]
      assertBool "at least one world file exists (an empty scan would be vacuous)"
        (not (null hsFiles))
      results <- mapM (\f -> (,) f . worldSourceOffenders <$> readFile f) hsFiles
      let violations = [ (f, os) | (f, os) <- results, not (null os) ]
      assertBool ("Prax-namespaced variable(s) found in authored world source: " ++ show violations)
        (null violations)

  , testGroup "the shared guard (Prax.Types.authoredVarClash), pinned through a real combinator"
    [ testCase "sanity: an ordinary fragment (no Prax namespace) is accepted" $ do
        r <- try (evaluate (length (draw 1 2 [ Match "flag.X" ] [ Insert "marked.X" ])))
        assertBool "an unremarkable fragment must NOT be rejected" (not (isLeft (r :: Either ErrorCall Int)))

    , testCase "a Prax-namespaced variable in the top-level conditions is caught" $ do
        r <- try (evaluate (length (draw 1 2 [ Match "flag.PraxD" ] [])))
        assertBool "PraxD in conds is rejected" (isLeft (r :: Either ErrorCall Int))

    , testCase "a Prax-namespaced variable in the top-level outcomes is caught" $ do
        r <- try (evaluate (length (draw 1 2 [] [ Insert "marked.PraxW" ])))
        assertBool "PraxW in outs is rejected" (isLeft (r :: Either ErrorCall Int))

    , testCase "a Prax-namespaced variable nested inside a ForEach outcome's own conditions is caught" $ do
        r <- try (evaluate (length (draw 1 2 [] [ ForEach [ Match "y.PraxD" ] [ Insert "done" ] ])))
        assertBool "PraxD inside a nested ForEach guard is rejected" (isLeft (r :: Either ErrorCall Int))

    , testCase "a Prax-namespaced variable in a Subquery's free-variable list is caught" $ do
        r <- try (evaluate (length
               (draw 1 2 [ Subquery "S" ["PraxD"] [ Match "seen.ok" ] ] [])))
        assertBool "PraxD in a Subquery's free-var list is rejected" (isLeft (r :: Either ErrorCall Int))
    ]
  ]
