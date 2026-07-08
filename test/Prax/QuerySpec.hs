module Prax.QuerySpec (tests) where

import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, (@?=))

import           Prax.Db
import           Prax.Query

build :: [String] -> Db
build ss = insertAll ss emptyDb

look :: String -> Bindings -> Maybe Val
look = Map.lookup

tests :: TestTree
tests = testGroup "Prax.Query"
  [ testGroup "match / not"
    [ testCase "bare sentence unifies and binds" $
        let db = build ["char.tim", "char.kevin"]
        in length (query db [Match "char.Who"] Map.empty) @?= 2
    , testCase "negation as failure keeps binding when absent" $
        query (build ["char.tim"]) [Not "isDancing.tim"] Map.empty @?= [Map.empty]
    , testCase "negation as failure drops binding when present" $
        query (build ["isDancing.tim"]) [Not "isDancing.tim"] Map.empty @?= []
    ]

  , testGroup "eq / neq"
    [ testCase "eq binds an unbound variable to a constant" $
        let [b] = query emptyDb [Eq "X" "beer"] Map.empty
        in look "X" b @?= Just (VStr "beer")
    , testCase "eq of two equal bound values keeps the binding" $
        query emptyDb [Eq "X" "Y"] (Map.fromList [("X", VStr "a"), ("Y", VStr "a")])
          @?= [Map.fromList [("X", VStr "a"), ("Y", VStr "a")]]
    , testCase "eq of two differing bound values drops the binding" $
        query emptyDb [Eq "X" "Y"] (Map.fromList [("X", VStr "a"), ("Y", VStr "b")])
          @?= []
    , testCase "neq keeps distinct, drops equal" $ do
        query emptyDb [Neq "X" "Y"] (Map.fromList [("X", VStr "a"), ("Y", VStr "b")])
          @?= [Map.fromList [("X", VStr "a"), ("Y", VStr "b")]]
        query emptyDb [Neq "X" "Y"] (Map.fromList [("X", VStr "a"), ("Y", VStr "a")])
          @?= []
    , testCase "neq with an unbound operand drops the binding (tie-game reliance)" $
        query emptyDb [Neq "Actor" "Winner"] (Map.fromList [("Actor", VStr "tim")]) @?= []
    ]

  , testGroup "numeric: cmp / calc  (port of tests.js math block)"
    [ testCase "gt fails then passes across an exclusion update" $ do
        let db0 = build ["counter.0"]
        query db0 [Match "counter.Val", Cmp Gt "Val" "4"] Map.empty @?= []
        -- calc NewVal = 0 + 5
        let [b] = query db0 [Match "counter.Val", Calc "NewVal" Add "Val" "5"] Map.empty
        look "NewVal" b @?= Just (VNum 5)
        -- replace counter with the new value; now gt 4 holds
        let db1 = insert "counter!5" db0
            rs  = query db1 [Match "counter.Val", Cmp Gt "Val" "4"] Map.empty
        map (look "Val") rs @?= [Just (VStr "5")]
    , testCase "chained calc: mul then sub yields -20" $
        let db = build ["counter!5"]
            [b] = query db
                    [ Match "counter.Val"
                    , Calc "BigVal" Mul "Val" "Val"
                    , Cmp Lt "Val" "BigVal"
                    , Calc "TinyVal" Sub "Val" "BigVal"
                    ] Map.empty
        in do look "BigVal" b @?= Just (VNum 25)
              look "TinyVal" b @?= Just (VNum (-20))
    ]

  , testGroup "subquery / count  (port of tests.js subquery block)"
    [ testCase "count dancers other than the actor equals 2" $
        let db = build [ "char.tim", "char.kevin", "char.james", "char.jer"
                       , "isDancing.tim", "isDancing.kevin", "isDancing.jer" ]
            conds =
              [ Match "char.Actor"
              , Subquery { subSet = "Dancers", subFind = ["Dancer"]
                         , subWhere = [ Match "char.Dancer", Match "isDancing.Dancer"
                                      , Neq "Dancer" "Actor" ] }
              , Count "NumDancers" "Dancers"
              , Eq "NumDancers" "2"
              ]
            rs = query db conds (Map.fromList [("Actor", VStr "tim")])
        in do
          length rs @?= 1
          -- The set holds the two other dancers (kevin, jer), one column each.
          let Just (VSet rows) = look "Dancers" (head rs)
          fmap length (look "Dancers" (head rs) >>= asSet) @?= Just 2
          length rows @?= 2
    , testCase "eq on the count filters out the wrong actor" $
        let db = build [ "char.tim", "char.solo", "isDancing.tim" ]
            conds =
              [ Match "char.Actor"
              , Subquery { subSet = "Dancers", subFind = ["Dancer"]
                         , subWhere = [ Match "isDancing.Dancer", Neq "Dancer" "Actor" ] }
              , Count "NumDancers" "Dancers"
              , Eq "NumDancers" "2"
              ]
        in query db conds (Map.fromList [("Actor", VStr "solo")]) @?= []
          -- solo sees only tim dancing (count 1), so eq 2 fails.
    ]

  , testGroup "first-order connectives (∨, ¬compound, ∃, ∀, →)"
    [ testCase "Or binds via either clause (disjunction)" $
        let db = build ["p.a", "q.b"]
            rs = query db [ Or [ [Match "p.X"], [Match "q.X"] ] ] Map.empty
        in sortVals (concatMap (maybe [] pure . look "X") rs) @?= [VStr "a", VStr "b"]

    , testCase "Or deduplicates overlapping clauses" $
        let db = build ["p.a", "q.a"]  -- both clauses yield X=a
        in length (query db [ Or [ [Match "p.X"], [Match "q.X"] ] ] Map.empty) @?= 1

    , testCase "Absent is ¬∃ over a compound (no male leader)" $ do
        -- a male leader exists → Absent fails
        query (build ["leader.brown", "brown.sex!male"])
              [ Absent [ Match "leader.L", Match "L.sex!male" ] ] Map.empty @?= []
        -- only a female leader → Absent holds
        query (build ["leader.lucy", "lucy.sex!female"])
              [ Absent [ Match "leader.L", Match "L.sex!male" ] ] Map.empty @?= [Map.empty]

    , testCase "Exists is boolean ∃ — satisfiable without leaking witnesses" $ do
        let db = build ["char.tim", "char.kev", "here.ok"]
        -- bare Match multiplies over all chars…
        length (query db [ Match "here.OK", Match "char.Who" ] Map.empty) @?= 2
        -- …Exists keeps a single binding and does not bind Who
        let rs = query db [ Match "here.OK", Exists [ Match "char.Who" ] ] Map.empty
        length rs @?= 1
        look "Who" (head rs) @?= Nothing

    , testCase "forAll: every patron has a drink (flips when one lacks it)" $ do
        let has  = build ["patron.tim", "patron.kev", "drink.tim", "drink.kev"]
            lacks = build ["patron.tim", "patron.kev", "drink.tim"]
            q d = query d [ forAll [Match "patron.P"] [Match "drink.P"] ] Map.empty
        q has   @?= [Map.empty]
        q lacks @?= []

    , testCase "implies: A → B truth table" $ do
        let q facts = query (build facts) [ implies [Match "raining"] [Match "wet"] ] Map.empty
        q ["raining", "wet"]        @?= [Map.empty]   -- A ∧ B
        q ["raining"]               @?= []            -- A ∧ ¬B
        q ["wet"]                   @?= [Map.empty]   -- ¬A (vacuously true)
        q []                        @?= [Map.empty]   -- ¬A
    ]
  ]

sortVals :: [Val] -> [Val]
sortVals = map VStr . sortStr . map valToString
  where sortStr = foldr ins []
        ins x [] = [x]
        ins x (y:ys) | x <= y    = x : y : ys
                     | otherwise = y : ins x ys

asSet :: Val -> Maybe [[String]]
asSet (VSet xs) = Just xs
asSet _         = Nothing
