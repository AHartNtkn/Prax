module Prax.DbSpec (tests) where

import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, (@?=))

import           Prax.Db
import           Prax.Sym (intern, symName)

-- Build a database from a list of sentences inserted left to right.
build :: [String] -> Db
build ss = insertAll ss emptyDb

tests :: TestTree
tests = testGroup "Prax.Db"
  [ testGroup "insert / dbToSentences"
    [ testCase "basic multi-valued facts" $
        dbToSentences (build ["foo.bar.baz", "foo.bar.woof", "foo.meow.woof"])
          @?= ["foo.bar.baz", "foo.bar.woof", "foo.meow.woof"]

    , testCase "exclusion replaces the old single value (x.age!32 then x.age!33)" $
        -- Both the buggy and corrected semantics agree here: age ends with the
        -- single child 33. This checks exclusion's replace behaviour, not the bug.
        dbToSentences (build ["x.age!32", "x.age!33"]) @?= ["x.age.33"]

    , testCase "REGRESSION: re-asserting an ! parent preserves its existing subtree" $
        -- The Praxish `!` bug: inserting `foo!bar.meow` after `foo!bar.baz`
        -- must keep `baz`, because `bar` is still the sole child of `foo` — the
        -- exclusion clears *siblings* of `bar`, not `bar`'s own subtree.
        -- Fails under the faithful (buggy) port; passes once corrected.
        dbToSentences (build ["foo!bar.baz", "foo!bar.meow"])
          @?= ["foo.bar.baz", "foo.bar.meow"]

    , testCase "exclusion clears siblings when the ! child changes" $
        -- p!a.x then p!b.y : switching the single child a->b drops all of a's
        -- data (paper's automatic-cleanup example).
        dbToSentences (build ["p!a.x", "p!b.y"]) @?= ["p.b.y"]

    , testCase "dot under an ! child accumulates" $
        dbToSentences (build ["g!closingStar!prebeginning"])
          @?= ["g.closingStar.prebeginning"]
    ]

  , testGroup "retract"
    [ testCase "removes a subtree by prefix" $
        dbToSentences (retract "foo.bar" (build ["foo.bar.baz", "foo.meow.woof"]))
          @?= ["foo.meow.woof"]
    , testCase "retracting a missing path is a no-op" $
        dbToSentences (retract "nope.nothere" (build ["foo.bar"]))
          @?= ["foo.bar"]
    ]

  , testGroup "unify"
    [ testCase "two-sentence join binds shared variable" $
        -- Port of tests.js: unifyAll(["X.Y.woof","fizz.buzz.X"]) over the DB.
        let db = build ["foo.bar.woof", "foo.meow.woof", "fizz.buzz.foo", "some.other.woof"]
            results = unifyAll ["X.Y.woof", "fizz.buzz.X"] db
        in do
          -- Both rows fix X=foo (the only head reachable via fizz.buzz.X); Y
          -- ranges over foo's children that have a `woof` leaf: bar and meow.
          map (Map.lookup (intern "X")) results @?= [Just (VSym (intern "foo")), Just (VSym (intern "foo"))]
          sortVals (concatMap (maybe [] pure . Map.lookup (intern "Y")) results)
            @?= [VSym (intern "bar"), VSym (intern "meow")]

    , testCase "bound variable descends deterministically" $
        let db = build ["char.tim", "char.kevin"]
        in length (unify "char.Who" db Map.empty) @?= 2

    , testCase "constant that is absent yields no bindings" $
        unify "char.nobody" (build ["char.tim"]) Map.empty @?= []
    ]

  , testGroup "ground"
    [ testCase "substitutes bound vars, preserves ! and ." $
        ground "practice.tendBar.B.customer.C!order!Bev"
          (Map.fromList [ (intern "B", VSym (intern "ada")), (intern "C", VSym (intern "beth"))
                        , (intern "Bev", VSym (intern "cider")) ])
          @?= "practice.tendBar.ada.customer.beth!order!cider"
    , testCase "unbound var grounds to its own name" $
        ground "foo.Bar" Map.empty @?= "foo.Bar"
    , testCase "set-valued binding renders as opaque marker" $
        ground "all.Dancers" (Map.fromList [(intern "Dancers", VSet [[intern "a"], [intern "b"]])])
          @?= "all.<Set(2)>"
    ]

  , testGroup "unifyNames"
    [ testCase "unifyNames is unify with the parse hoisted out" $ do
        let db = insertAll ["at.bob!square", "at.eve!mill"] emptyDb
        unifyNames (pathNames "at.Who!Where") db Map.empty
          @?= unify "at.Who!Where" db Map.empty
    ]

  , testGroup "groundTokens"
    [ testCase "groundTokens substitutes bindings segment-wise, preserving operators" $ do
        let toks = internTokens "at.Who!Where"
            b    = Map.fromList [ (intern "Who", VSym (intern "bob"))
                                 , (intern "Where", VSym (intern "square")) ]
        tokensToSentence (groundTokens toks b) @?= ground "at.Who!Where" b
        tokensToSentence (groundTokens (internTokens "plain.path") Map.empty)
          @?= "plain.path"
    ]

  , testGroup "internTokens / unifySyms (the Sym-level cores unify/unifyNames delegate to)"
    [ testCase "internTokens interns tokens' segment names, preserving operators" $
        map (\(s, op) -> (symName s, op)) (internTokens "at.Who!Where")
          @?= tokens "at.Who!Where"
    , testCase "unifySyms agrees with unifyNames (Bindings is Sym-keyed natively)" $ do
        let db = insertAll ["at.bob!square", "at.eve!mill"] emptyDb
            names = pathNames "at.Who!Where"
        unifySyms (map intern names) db Map.empty @?= unifyNames names db Map.empty
    , testCase "unifySyms branches unbound variables in name order, not id (encounter) order" $ do
        -- Insert children out of alphabetical order, so id order != name order.
        let db = insertAll ["at.zeta", "at.alpha", "at.mu"] emptyDb
            results = unifySyms (map intern (pathNames "at.Who")) db Map.empty
            names = [ symName who
                    | bs <- results, Just (VSym who) <- [Map.lookup (intern "Who") bs] ]
        names @?= ["alpha", "mu", "zeta"]
    ]
  ]

-- Deterministic ordering for value lists (Val has no Ord instance).
sortVals :: [Val] -> [Val]
sortVals = map (VSym . intern) . sortStr . map valToString
  where sortStr = foldr insertStr []
        insertStr x [] = [x]
        insertStr x (y:ys) | x <= y    = x : y : ys
                           | otherwise = y : insertStr x ys
