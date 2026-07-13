module Prax.DbSpec (tests) where

import           Data.Bifunctor (first)
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

    , testCase "INSTANCE PERSISTENCE: draining transient state nested under an asserted path leaves the path intact" $
        -- Distills Prax.Worlds.Bar's `tendBarP` pattern to the Db level: an
        -- instance fact (e.g. "practice.tendBar.bar.ada") doubles as the
        -- parent namespace for fully-drainable, transient per-customer state
        -- nested underneath the SAME path (order -> fulfill deletes order,
        -- inserts beverage -> drink deletes beverage, nothing reinserted).
        -- `Prax.Engine.possibleActions` discovers practice instances by trie
        -- presence alone (no separate registry), so retracting the last
        -- transient child down to nothing must NOT take the instance path
        -- down with it, or the bartender's own instance is gone forever with
        -- no way to ever reinsert it.
        --
        -- This is the regression net for a real bug found (and reverted) in
        -- this task: pruning ancestors left childless by retract broke
        -- exactly this pattern (`BarSpec`'s "drinking two beers" test failed
        -- on the second order after that fix). See
        -- `.superpowers/sdd/task-2b-report.md` for the full trace. The trie
        -- currently cannot distinguish "asserted instance fact that happens
        -- to have children" from "ordinary ancestor, now childless" — see
        -- `retract`'s haddock — so this drained ancestor is (unavoidably,
        -- for now) ALSO what `dbToSentences` emits as a phantom fact; that
        -- emission is the documented cost of keeping the instance alive.
        let instanceFact = "practice.tendBar.bar.ada"
            db0 = build [instanceFact]
            db1 = insert (instanceFact ++ ".customer.you!order!beer") db0
            db2 = retract (instanceFact ++ ".customer.you!order") db1
            db3 = insert (instanceFact ++ ".customer.you!beverage!beer") db2
            db4 = retract (instanceFact ++ ".customer.you!beverage") db3
        in do
          exists instanceFact db4 @?= True
          exists (instanceFact ++ ".customer.you") db4 @?= True
          dbToSentences db4 @?= [instanceFact ++ ".customer.you"]

    , testCase "sibling and shared ancestors survive retracting the other sibling" $
        -- Two facts sharing a prefix (two children under `carol`): retracting
        -- one must prune nothing above `carol`, since `carol` still has the
        -- surviving sibling as a child.
        let db  = build ["eve.lied.dana.stole.carol.loaf", "eve.lied.dana.stole.carol.purse"]
            db' = retract "eve.lied.dana.stole.carol.loaf" db
        in do
          exists "eve.lied.dana.stole.carol.loaf" db' @?= False
          exists "eve.lied.dana.stole.carol.purse" db' @?= True
          exists "eve.lied.dana.stole.carol" db' @?= True
          exists "eve.lied.dana.stole" db' @?= True
          exists "eve.lied.dana" db' @?= True
          exists "eve.lied" db' @?= True
          exists "eve" db' @?= True
          dbToSentences db' @?= ["eve.lied.dana.stole.carol.purse"]
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
        map (first symName) (internTokens "at.Who!Where")
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
