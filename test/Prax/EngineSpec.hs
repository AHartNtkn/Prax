module Prax.EngineSpec (tests) where

import           Control.Exception (ErrorCall, evaluate, try)
import           Data.Either (isLeft)
import           Data.List (isInfixOf)
import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, (@?=), assertBool, assertFailure)

import           Prax.Db (dbToSentences, exists, pathNames)
import           Prax.Query
import           Prax.Types
import           Prax.Engine
import           Prax.Derive (axiom)
import           Prax.Sym (intern)

-- Practices ported from praxish demos/test/tests.js into the eDSL. --------------

greetP :: Practice
greetP = practice
  { practiceId = "greet"
  , practiceName = "[Greeter] is greeting [Greeted]"
  , roles = ["Greeter", "Greeted"]
  , actions =
      [ action "[Actor]: Greet [Other]"
          [ Eq "Actor" "Greeter", Eq "Other" "Greeted" ]
          [ Delete "practice.greet.Actor.Other" ]
      ]
  }

tendBarP :: Practice
tendBarP = practice
  { practiceId = "tendBar"
  , practiceName = "[Bartender] is tending bar"
  , roles = ["Bartender"]
  , dataFacts =
      [ "beverageType.beer!alcoholic", "beverageType.cider!alcoholic"
      , "beverageType.soda!nonalcoholic", "beverageType.water!nonalcoholic" ]
  , actions =
      [ action "[Actor]: Walk up to bar"
          [ Neq "Actor" "Bartender"
          , Not "practice.tendBar.Bartender.customer.Actor" ]
          [ Insert "practice.tendBar.Bartender.customer.Actor" ]
      , action "[Actor]: Order [Beverage]"
          [ Match "practice.tendBar.Bartender.customer.Actor"
          , Not "practice.tendBar.Bartender.customer.Actor!beverage"
          , Match "practiceData.tendBar.beverageType.Beverage" ]
          [ Insert "practice.tendBar.Bartender.customer.Actor!order!Beverage" ]
      , action "[Actor]: Fulfill [Customer]'s order"
          [ Eq "Actor" "Bartender"
          , Match "practice.tendBar.Bartender.customer.Customer!order!Beverage" ]
          [ Delete "practice.tendBar.Bartender.customer.Customer!order"
          , Insert "practice.tendBar.Bartender.customer.Customer!beverage!Beverage" ]
      ]
  }

-- A practice with an `init` that seeds turn state on spawn.
duelP :: Practice
duelP = practice
  { practiceId = "duel"
  , practiceName = "[A] duels [B]"
  , roles = ["A", "B"]
  , initOutcomes = [ Insert "practice.duel.A.B.turn!A" ]
  , actions =
      [ action "[Actor]: Strike"
          [ Match "practice.duel.A.B.turn!Actor" ]
          [ Insert "practice.duel.A.B.struck!Actor" ]
      ]
  }

-- A practice exercising `call` into a guarded function with a calc.
mathP :: Practice
mathP = practice
  { practiceId = "math"
  , practiceName = "math box [M]"
  , roles = ["M"]
  , initOutcomes = [ Insert "practice.math.M.n!3" ]
  , actions =
      [ action "[Actor]: Double"
          [ Match "practice.math.M.n!N" ]
          [ Call "dbl" ["M", "N"] ]
      ]
  }

-- mathP's registered function (spec v47: the world registry, not a practice field).
dblFn :: Function
dblFn = Function "dbl" ["M", "N"]
  [ FnCase [ Calc "R" Mul "N" "2" ]
           [ Insert "practice.math.M.n!R" ] ]

-- Test driver: perform the first action whose label contains `needle`. ---------

step :: PraxState -> String -> String -> IO PraxState
step st actor needle =
  case filter ((needle `isInfixOf`) . gaLabel) (possibleActions st actor) of
    (ga : _) -> pure (performAction st ga)
    []       -> assertFailure
                  ("no action matching " ++ show needle ++ " for " ++ actor
                   ++ "; available: "
                   ++ show (map gaLabel (possibleActions st actor)))
                  >> pure st

labels :: PraxState -> String -> [String]
labels st actor = map gaLabel (possibleActions st actor)

tests :: TestTree
tests = testGroup "Prax.Engine"
  [ testCase "cookedDefs mirrors practiceDefs' keys after definePractices" $
      let st = defineFunctions [dblFn] (definePractices [greetP, tendBarP, duelP, mathP] emptyState)
      in Map.keys (cookedDefs st) @?= Map.keys (practiceDefs st)

  , testCase "definePractice inserts static data under practiceData" $
      let st = definePractice tendBarP emptyState
      in assertBool "beverageType present"
           ("practiceData.tendBar.beverageType.cider.alcoholic"
              `elem` dbToSentences (db st))

  , testCase "greet: affordance appears, and performing it consumes the instance" $ do
      let st0 = definePractice greetP emptyState
          st1 = performOutcome (Insert "practice.greet.max.isaac") st0
      labels st1 "max" @?= ["max: Greet isaac"]
      st2 <- step st1 "max" "Greet isaac"
      -- The greet action deletes its own instance, so no more greet affordances.
      labels st2 "max" @?= []

  , testCase "tendBar: walk up -> order -> fulfill delivers the drink" $ do
      let st0 = definePractices [tendBarP] emptyState
          st1 = performOutcome (Insert "practice.tendBar.ada") st0
      -- beth can only walk up initially.
      labels st1 "beth" @?= ["beth: Walk up to bar"]
      st2 <- step st1 "beth" "Walk up to bar"
      -- now beth can order any of the four beverages
      assertBool "can order cider" ("beth: Order cider" `elem` labels st2 "beth")
      st3 <- step st2 "beth" "Order cider"
      -- ada (bartender) can now fulfill the order
      assertBool "ada can fulfill" (any ("Fulfill" `isInfixOf`) (labels st3 "ada"))
      st4 <- step st3 "ada" "Fulfill"
      let facts = dbToSentences (db st4)
      assertBool "beverage delivered"
        ("practice.tendBar.ada.customer.beth.beverage.cider" `elem` facts)
      assertBool "pending order cleared"
        (not (any (\f -> "customer.beth.order" `isInfixOf` f) facts))

  , testCase "spawning runs init once; only the whose-turn actor can strike" $ do
      let st0 = definePractice duelP emptyState
          st1 = performOutcome (Insert "practice.duel.max.nic") st0
      assertBool "init seeded turn"
        ("practice.duel.max.nic.turn.max" `elem` dbToSentences (db st1))
      labels st1 "max" @?= ["max: Strike"]
      labels st1 "nic" @?= []

  , testCase "call into a guarded function applies its calc effect" $ do
      let st0 = defineFunctions [dblFn] (definePractice mathP emptyState)
          st1 = performOutcome (Insert "practice.math.box") st0
      assertBool "init n=3" ("practice.math.box.n.3" `elem` dbToSentences (db st1))
      st2 <- step st1 "alice" "Double"
      assertBool "n doubled to 6"
        ("practice.math.box.n.6" `elem` dbToSentences (db st2))

  , testCase "ForEach applies its outcomes for every binding" $ do
      let st = foldl (flip performOutcome) emptyState
                 [ Insert "member.a", Insert "member.b", Insert "member.c" ]
          st' = performOutcome (ForEach [ Match "member.X" ] [ Insert "greeted.X" ]) st
      mapM_ (\n -> assertBool ("greeted." ++ n) (exists ("greeted." ++ n) (db st')))
            ["a", "b", "c"]

  , testCase "ForEach with zero bindings is a no-op" $ do
      let st  = performOutcome (Insert "unrelated") emptyState
          st' = performOutcome (ForEach [ Match "member.X" ] [ Insert "greeted.X" ]) st
      db st' @?= db st

  , testCase "ForEach snapshots its bindings: mutations cannot extend the quantification" $ do
      -- Inserting a new member from inside the fold must NOT add a binding:
      -- quantification is over the state at entry.
      let st  = performOutcome (Insert "member.a") emptyState
          st' = performOutcome
                  (ForEach [ Match "member.X" ]
                           [ Insert "member.b", Insert "visited.X" ]) st
      assertBool "visited the original member" (exists "visited.a" (db st'))
      assertBool "did NOT visit the member inserted mid-fold"
        (not (exists "visited.b" (db st')))

  , testCase "ForEach grounds the enclosing action's bindings first" $ do
      let p = practice
            { practiceId = "tell", roles = ["R"]
            , actions = [ action "[Actor]: tell friends about [Target]"
                            [ Match "target.Target" ]
                            [ ForEach [ Match "friend.Target.W" ]
                                      [ Insert "told.W.Target" ] ] ] }
          st = foldl (flip performOutcome)
                 (setCharacters [ character "ann" ]
                   (definePractices [p] emptyState))
                 [ Insert "practice.tell.stage"
                 , Insert "target.bob"
                 , Insert "friend.bob.carol", Insert "friend.bob.dave"
                 , Insert "friend.eve.mallory" ]   -- a different target's friend: must not fire
          st' = case possibleActions st "ann" of
                  (ga : _) -> performAction st ga
                  []       -> error "tell action not offered"
      assertBool "told carol about bob" (exists "told.carol.bob" (db st'))
      assertBool "told dave about bob"  (exists "told.dave.bob" (db st'))
      assertBool "eve's friend untouched" (not (exists "told.mallory.eve" (db st')))

  , testCase "ForEach nests: outer bindings ground the inner quantifier" $ do
      let st = foldl (flip performOutcome) emptyState
                 [ Insert "row.a", Insert "row.b", Insert "col.x", Insert "col.y" ]
          st' = performOutcome
                  (ForEach [ Match "row.R" ]
                           [ ForEach [ Match "col.C" ] [ Insert "cell.R.C" ] ]) st
      mapM_ (\s -> assertBool s (exists s (db st')))
            [ "cell.a.x", "cell.a.y", "cell.b.x", "cell.b.y" ]

  , testCase "ForEach snapshot holds for Delete: removing a member mid-fold still visits all" $ do
      let st = foldl (flip performOutcome) emptyState
                 [ Insert "member.a", Insert "member.b" ]
          st' = performOutcome
                  (ForEach [ Match "member.X" ]
                           [ Delete "member.X", Insert "visited.X" ]) st
      assertBool "visited a" (exists "visited.a" (db st'))
      assertBool "visited b" (exists "visited.b" (db st'))
      assertBool "member a gone" (not (exists "member.a" (db st')))
      assertBool "member b gone" (not (exists "member.b" (db st')))

  , testCase "ForEach with no conditions applies its outcomes exactly once" $ do
      let st = foldl (flip performOutcome) emptyState [ Insert "counter!0" ]
          st' = performOutcome
                  (ForEach [] [ ForEach [ Match "counter!N", Calc "M" Add "N" "1" ]
                                        [ Insert "counter!M" ] ]) st
      assertBool "ran exactly once" (exists "counter!1" (db st'))
      assertBool "not twice" (not (exists "counter!2" (db st')))

  , testCase "setAxioms re-derives the cached view on a built state" $ do
      let ax = axiom [ Match "parent.X.Y" ] [ "elder.X" ]
          st0 = performOutcome (Insert "parent.ada.bea") emptyState
      assertBool "no axioms: nothing derived"
        (not (exists "elder.ada" (readView st0)))
      let st1 = setAxioms [ax] st0
      assertBool "derived after setAxioms" (exists "elder.ada" (readView st1))
      -- and the view tracks subsequent writes through the helpers
      let st2 = performOutcome (Insert "parent.bea.cal") st1
      assertBool "new base fact derives too" (exists "elder.bea" (readView st2))

  , testCase "groundedDeltaAnchors: bounded effects, shadows, spawn opacity, Call resolution" $ do
      let p = practice
            { practiceId = "market", roles = ["R"]
            , actions =
                [ action "[Actor]: trade"
                    [] [ Insert "coin.Actor!spent", Delete "stock.Actor" ]
                , action "[Actor]: enroll"
                    [] [ Insert "practice.market.Actor" ]
                , action "[Actor]: ritual" [] [ Call "bless" ["Actor"] ]
                , action "[Actor]: chant"  [] [ Call "unknownFn" ["Actor"] ]
                ]
            }
          blessFn = Function "bless" ["Who"]
                      [ FnCase [] [ Insert "blessed.Who" ] ]
          st = defineFunctions [blessFn] (definePractices [p] emptyState)
          st1 = performOutcome (Insert "practice.market.here") st
          gaOf label = case [ ga | ga <- possibleActions st1 "ada", gaLabel ga == label ] of
            (ga : _) -> ga
            []       -> error ("no such grounded action: " ++ label)
          anchorsOf label = groundedDeltaAnchors st1 (gaOf label)
          has s as = map intern (pathNames s) `elem` as
      case anchorsOf "ada: trade" of
        Nothing -> assertFailure "trade must be bounded"
        Just as -> do
          assertBool "grounded insert path" (has "coin.ada.spent" as)
          assertBool "eviction shadow"      (has "coin.ada.PraxEvicted" as)
          assertBool "grounded delete path" (has "stock.ada" as)
      anchorsOf "ada: enroll" @?= Nothing           -- spawn opacity
      case anchorsOf "ada: ritual" of
        Nothing -> assertFailure "resolvable Call must be bounded"
        Just as -> assertBool "Call-resolved insert, arg-grounded" (has "blessed.ada" as)
      anchorsOf "ada: chant" @?= Nothing            -- unresolvable Call

  , testCase "groundedDeltaAnchors: safe ForEach binders bound; unsafe heads stay opaque" $ do
      let p = practice
            { practiceId = "gossipy", roles = ["R"]
            , actions =
                [ action "[Actor]: broadcast"
                    [] [ ForEach [ Match "together.W" ]
                           [ Insert "W.believes.rumor" ] ]
                , action "[Actor]: reshape"
                    [] [ ForEach [ Match "X.y.Z" ]
                           [ Insert "X.marked" ] ]
                , action "[Actor]: phantom"
                    [] [ ForEach [ Exists [ Match "roster.W" ] ]
                           [ Insert "W.tagged" ] ]
                , action "[Actor]: void gesture"
                    [] [ ForEach [ Match "roster.W" ]
                           [ Insert "W" ] ]
                ]
            }
          st = definePractices [p] emptyState
          st1 = performOutcome (Insert "practice.gossipy.here") st
          gaOf label = case [ ga | ga <- possibleActions st1 "ada", gaLabel ga == label ] of
            (ga : _) -> ga
            []       -> error ("no such grounded action: " ++ label)
      -- The broadcast: W is bound at position 2 of a top-level Match — a
      -- safe binder; the insert is bounded with W as a wildcard anchor.
      case groundedDeltaAnchors st1 (gaOf "ada: broadcast") of
        Nothing -> assertFailure "broadcast must be bounded (safe binder)"
        Just as -> assertBool "wildcard-headed believes anchor"
          (map intern (pathNames "W.believes.rumor") `elem` as)
      -- A position-1 binder really can unify practice-registry paths.
      groundedDeltaAnchors st1 (gaOf "ada: reshape") @?= Nothing
      -- Exists does not bind outward; its "binder" is not safe.
      groundedDeltaAnchors st1 (gaOf "ada: phantom") @?= Nothing
      -- A safe binder heading an ALL-VARIABLE path: no literal anchor, no
      -- evidence — must be opaque, not bounded.
      groundedDeltaAnchors st1 (gaOf "ada: void gesture") @?= Nothing

  , testCase "axiomHeads: fireable heads, lifted forms, the contradiction witness" $ do
      let axs = [ axiom [ Match "starving.X" ] [ "hungry.X" ] ]
          st = setAxioms axs emptyState
          has s = map intern (pathNames s) `elem` axiomHeads st
      assertBool "the head"        (has "hungry.X")
      assertBool "the lifted head" (has "obliged.Obligor.hungry.X")
      assertBool "the ⊥ witness"   (has "contradiction")

  , testGroup "collision guards (v43, re-expressed against the v47 registry): action names and registered function names must each be unique"
    [ testCase "two actions with the same name in one practice is a loud construction-time error" $ do
        let p = practice { practiceId = "dup", roles = ["R"]
                          , actions = [ action "dup" [] [], action "dup" [] [] ] }
        r <- try (evaluate (Map.size (practiceDefs (definePractice p emptyState))))
        assertBool "duplicate action names rejected" (isLeft (r :: Either ErrorCall Int))

    , testCase "two functions with the same name within ONE defineFunctions batch is a loud error" $ do
        r <- try (evaluate (Map.size (cookedFns
                    (defineFunctions [ Function "f" [] [], Function "f" [] [] ] emptyState))))
        assertBool "within-batch function name collision rejected" (isLeft (r :: Either ErrorCall Int))

    , testCase "a function name already registered by an EARLIER defineFunctions call is a loud error" $ do
        let st1 = defineFunctions [ Function "f" [] [] ] emptyState
        r <- try (evaluate (Map.size (cookedFns
                    (defineFunctions [ Function "f" [] [] ] st1))))
        assertBool "across-call function name collision rejected" (isLeft (r :: Either ErrorCall Int))

    , testCase "distinct function names across two calls register cleanly (accumulation)" $ do
        let st2 = defineFunctions [ Function "g" [] [] ]
                    (defineFunctions [ Function "f" [] [] ] emptyState)
        Map.keys (cookedFns st2) @?= ["f", "g"]
    ]
  ]
