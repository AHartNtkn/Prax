{-# LANGUAGE OverloadedStrings #-}

-- | The differential-oracle executable: the one additive surface on the frozen
-- Haskell tree (the library is imported, never edited). It emits canonical
-- JSON(L) the Rust comparator consumes to check the rewrite record-for-record.
--
-- Subcommands (see @docs/rewrite/PLAN.md@):
--
--   * @trace \<world\> --turns N [--idle \<name\>] [--depth D] --mode M@ — replay
--     the @Prax.GoldenDriveSpec.driveLabels@ walk (advance → npcAct, optional
--     idler skip), one JSONL record per turn.
--   * @randtrace \<world\> --seed S --cap N [--candidates]@ — replay
--     @Prax.Stress.runRandom@ step-for-step, full state per turn.
--   * @check \<world\>@ — @Prax.TypeCheck.typeCheck@ as a sorted JSON array of
--     rendered errors.
--   * @fixtures \<name\>@ — deterministic unit-fixture corpora (@db@\/@el@\/
--     @query@\/@derive@) as one JSON value on stdout.
--
-- Every list that could carry run-dependent order (a fact set, a binding row,
-- a candidate list) is sorted by rendered name before emission, so the bytes
-- are a pure function of the inputs.
module Main (main) where

import           Data.List (foldl', sort, sortOn)
import           Data.Maybe (isNothing, listToMaybe, fromMaybe)
import qualified Data.Map.Strict as Map
import           Data.Word (Word64)
import qualified Data.ByteString.Lazy.Char8 as BLC
import           Data.Aeson (Value, object, (.=), toJSON, encode)
import qualified Data.Aeson.Key as K
import           System.Environment (getArgs)
import           System.Exit (exitFailure)
import           System.IO (hPutStrLn, stderr)

import           Prax.Db
import           Prax.Sym (Sym, intern, symName)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction, currentTurn,
                              performOutcome, definePractices, defineFunctions,
                              setAxioms, seedDie)
import           Prax.Loop (advance, npcAct)
import           Prax.TypeCheck (TypeError (..), typeCheck)
import           Prax.EL (meet, leq)
import           Prax.Query (Condition (..), CmpOp (..), CalcOp (..), query)
import           Prax.Derive (Axiom (..), Contradiction (..), closure, axiom)
import           Prax.Kin (kinAxioms)
import qualified Prax.Worlds.Bar as Bar
import qualified Prax.Worlds.Intrigue as Intrigue
import qualified Prax.Worlds.Play as Play
import qualified Prax.Worlds.Feud as Feud
import qualified Prax.Worlds.Audience as Audience
import qualified Prax.Worlds.Village as Village

-- Worlds ---------------------------------------------------------------------

-- | A world by name — the same selection as @app/Main.hs@'s @worldNamed@,
-- plus the drama-manager variant of the bar under @dm@. Returns the built
-- state and its player name (the driveLabels idler for the golden worlds).
worldNamed :: String -> Maybe (PraxState, String)
worldNamed n = case n of
  "bar"      -> Just (Bar.barWorld,          Bar.playerName)
  "dm"       -> Just (Bar.barDirectorWorld,  Bar.directorName)
  "intrigue" -> Just (Intrigue.intrigueWorld, Intrigue.playerName)
  "play"     -> Just (Play.playWorld,        Play.playerName)
  "feud"     -> Just (Feud.feudWorld,        Feud.playerName)
  "audience" -> Just (Audience.audienceWorld, Audience.playerName)
  "village"  -> Just (Village.villageWorld,  Village.playerName)
  _          -> Nothing

allWorldNames :: [String]
allWorldNames = ["bar", "dm", "intrigue", "play", "feud", "audience", "village"]

-- Argument helpers -----------------------------------------------------------

-- | The value following @flag@ in the argument list, if present.
flagVal :: String -> [String] -> Maybe String
flagVal flag = go
  where
    go (f : v : _) | f == flag = Just v
    go (_ : rest)              = go rest
    go []                      = Nothing

hasFlag :: String -> [String] -> Bool
hasFlag flag = elem flag

-- | Read an @Int@ flag or die loudly (no silent default for required numbers).
intFlag :: String -> [String] -> IO Int
intFlag flag args = case flagVal flag args of
  Just s | [(n, "")] <- reads s -> pure n
  Just s  -> dieMsg (flag ++ " expects an integer, got " ++ show s)
  Nothing -> dieMsg ("missing required flag " ++ flag)

dieMsg :: String -> IO a
dieMsg m = hPutStrLn stderr ("prax-oracle: " ++ m) >> exitFailure

-- JSON emission --------------------------------------------------------------

-- | Print each value as one compact JSON line (JSONL).
putJSONL :: [Value] -> IO ()
putJSONL = mapM_ (BLC.putStrLn . encode)

-- | Print a single JSON value (the fixture corpora).
putJSON :: Value -> IO ()
putJSON = BLC.putStrLn . encode

-- | A binding row as a JSON object, keyed by variable name in name order (never
-- the run-dependent 'Sym' id order 'Map.toList' would give).
bindingJSON :: Bindings -> Value
bindingJSON b = object
  [ K.fromString (symName k) .= valToString v
  | (k, v) <- sortOn (symName . fst) (Map.toList b) ]

-- | The engine RNG stream position ('Prax.Types.rngSeed') as JSON: Null when
-- unseeded, else the numeric Lehmer value.
rngJSON :: Maybe Integer -> Value
rngJSON = toJSON

-- | The schedule dues map as a JSON object (name → next-due boundary), sorted
-- by name via 'Map.toList'.
duesJSON :: Map.Map String Int -> Value
duesJSON = toJSON

-- | The one-shot expiry queue as a JSON object: exact labeled path → the
-- boundary it fires at. Keys are the rendered paths ('tokensToSentence').
expiriesJSON :: Map.Map [(Sym, Maybe Char)] Int -> Value
expiriesJSON m = toJSON (Map.fromList
  [ (tokensToSentence k, v) | (k, v) <- Map.toList m ])

-- The trace/randtrace per-turn state fields shared by both walks.
stateFields :: PraxState -> [(K.Key, Value)]
stateFields st =
  [ "cursor"   .= cursor st
  , "rng"      .= rngJSON (rngSeed st)
  , "dues"     .= duesJSON (scheduleDues st)
  , "expiries" .= expiriesJSON (expiries st) ]

-- trace ----------------------------------------------------------------------

data Mode = Decisions | State | View deriving (Eq)

parseMode :: String -> Maybe Mode
parseMode "decisions" = Just Decisions
parseMode "state"     = Just State
parseMode "view"      = Just View
parseMode _           = Nothing

modeStr :: Mode -> String
modeStr Decisions = "decisions"
modeStr State     = "state"
modeStr View      = "view"

-- | The facts fields for a turn record, by mode: decisions omits facts
-- entirely; state adds the base db's labeled sentences; view additionally
-- adds the closed view's labeled sentences.
factFields :: Mode -> PraxState -> [(K.Key, Value)]
factFields Decisions _  = []
factFields State     st = [ "facts" .= dbToLabeledSentences (db st) ]
factFields View      st = [ "facts" .= dbToLabeledSentences (db st)
                          , "view"  .= dbToLabeledSentences (readView st) ]

runTrace :: String -> [String] -> IO ()
runTrace world args = case worldNamed world of
  Nothing -> dieMsg ("unknown world " ++ show world ++ " (one of "
                     ++ unwords allWorldNames ++ ")")
  Just (st0, _) -> do
    turns <- intFlag "--turns" args
    let idle  = flagVal "--idle" args
        depth = fromMaybe 2 (flagVal "--depth" args >>= readInt)
    mode <- case flagVal "--mode" args of
      Nothing -> pure Decisions
      Just m  -> maybe (dieMsg ("bad --mode " ++ show m)) pure (parseMode m)
    let header = object
          [ "format" .= (1 :: Int), "engine" .= ("haskell" :: String)
          , "world" .= world, "turns" .= turns, "idle" .= idle
          , "depth" .= depth, "mode" .= modeStr mode
          , "seed" .= (Nothing :: Maybe Integer) ]
    putJSONL (header : traceWalk depth turns idle mode st0)

-- | One record per turn, faithfully mirroring
-- 'Prax.GoldenDriveSpec.driveLabels': advance, and unless the actor is the
-- idler, have them act ('npcAct'). The state fields report the carry-forward
-- state (post-action), and @boundary@ is whether 'advance' fired a round
-- boundary (the engine clock ticked).
traceWalk :: Int -> Int -> Maybe String -> Mode -> PraxState -> [Value]
traceWalk depth total idle mode = go 1
  where
    go t st
      | t > total = []
      | otherwise =
          let before        = currentTurn st
              (actor, st1)   = advance st
              boundary       = currentTurn st1 /= before
              nm             = charName actor
          in if Just nm == idle
               then record t boundary nm ("-" :: String) True st1 : go (t + 1) st1
               else case npcAct depth actor st1 of
                      (mga, st2) ->
                        record t boundary nm (maybe "-" gaLabel mga) (isNothing mga) st2
                          : go (t + 1) st2
    record t boundary actor action idled st = object $
      [ "t" .= t, "boundary" .= boundary, "actor" .= actor
      , "action" .= action, "idle" .= idled ]
      ++ stateFields st ++ factFields mode st

-- randtrace ------------------------------------------------------------------

-- A minimal linear-congruential PRNG (Knuth's MMIX constants) — the exact
-- generator 'Prax.Stress.runRandom' walks; reproduced here so the replay
-- steps the same stream.
lcg :: Word64 -> Word64
lcg x = 6364136223846793005 * x + 1442695040888963407

-- A uniform index in @[0, n)@ and the next seed (n must be > 0).
pick :: Int -> Word64 -> (Int, Word64)
pick n s = let s' = lcg s in (fromIntegral (s' `mod` fromIntegral n), s')

-- | The ending reached, if any (an @ending.\<key\>@ fact) — copied from
-- 'Prax.Stress' (not exported) so the replay stops exactly where it does.
endingReached :: PraxState -> Maybe String
endingReached st =
  listToMaybe [ e | b <- unify "ending.E" (db st) Map.empty
                  , Just e <- [valToString <$> Map.lookup (intern "E") b] ]

runRandtrace :: String -> [String] -> IO ()
runRandtrace world args = case worldNamed world of
  Nothing -> dieMsg ("unknown world " ++ show world)
  Just (st0, _) -> do
    seed <- intFlag "--seed" args
    cap  <- intFlag "--cap" args
    let cands  = hasFlag "--candidates" args
        header = object
          [ "format" .= (1 :: Int), "engine" .= ("haskell" :: String)
          , "world" .= world, "seed" .= seed, "cap" .= cap
          , "candidates" .= cands ]
    putJSONL (header : randWalk cands cap (fromIntegral seed) st0)

-- | Replay 'Prax.Stress.runRandom' step-for-step, emitting one record per
-- advance (idle passes included). Control flow and arithmetic are copied
-- verbatim from that function; the coverage-family tracking (which does not
-- affect the walk) is dropped.
randWalk :: Bool -> Int -> Word64 -> PraxState -> [Value]
randWalk cands cap seed0 = stepWith (0 :: Int) (1 :: Int) cap seed0
  where
    stepWith passes t k s st
      | k == 0 = []
      | otherwise = case endingReached st of
          Just _ -> []
          Nothing
            | null living            -> []
            | passes > length living -> []                       -- true dead end
            | otherwise ->
                let before       = currentTurn st
                    (actor, st1) = advance st
                    boundary     = currentTurn st1 /= before
                    nm           = charName actor
                    acts         = possibleActions st1 nm
                in case acts of
                     [] -> recIdle t boundary nm st1
                             : stepWith (passes + 1) t k s st1
                     _  -> let (i, s') = pick (length acts) s
                               ga      = acts !! i
                               st2     = performAction st1 ga
                           in recAct t boundary nm ga acts s' st2
                                : stepWith 0 (t + 1) (k - 1) s' st2
          where living = livingCharacters st
    recIdle t boundary nm st = object $
      [ "t" .= t, "boundary" .= boundary, "actor" .= nm
      , "action" .= (Nothing :: Maybe String), "idle" .= True
      , "walkSeed" .= toWordJSON Nothing ]
      ++ stateFields st
      ++ [ "facts" .= dbToLabeledSentences (db st) ]
      ++ [ "candidates" .= ([] :: [String]) | cands ]
    recAct t boundary nm ga acts s' st = object $
      [ "t" .= t, "boundary" .= boundary, "actor" .= nm
      , "action" .= gaLabel ga, "idle" .= False
      , "walkSeed" .= toWordJSON (Just s') ]
      ++ stateFields st
      ++ [ "facts" .= dbToLabeledSentences (db st) ]
      ++ [ "candidates" .= sort (map gaLabel acts) | cands ]

-- | A 'Word64' walk-seed as JSON (an 'Integer' to stay exact across the
-- 64-bit range).
toWordJSON :: Maybe Word64 -> Value
toWordJSON = toJSON . fmap (toInteger :: Word64 -> Integer)

readInt :: String -> Maybe Int
readInt s = case reads s of [(n, "")] -> Just n; _ -> Nothing

-- check ----------------------------------------------------------------------

runCheckCmd :: String -> IO ()
runCheckCmd world = case worldNamed world of
  Nothing        -> dieMsg ("unknown world " ++ show world)
  Just (st0, _)  -> putJSON (toJSON (sort (map describe (typeCheck st0))))

-- The renderings from @app/Main.hs@'s @runCheck@, reproduced (that module is
-- frozen; the strings are the checker's public surface).
describe :: TypeError -> String
describe (UnboundVar w v s) =
  "unbound variable " ++ v ++ " in \"" ++ s ++ "\" (" ++ w ++ ")"
describe (CardinalityClash slot) =
  "relation " ++ slot ++ " is used both single-valued (!) and multi-valued (.)"
describe (UndefinedRef w n) =
  "undefined reference " ++ n ++ " (" ++ w ++ ")"
describe (SortConflict w d) =
  "sort conflict at " ++ w ++ ": " ++ d
describe (ReservedFamily fam w s) =
  "reserved family " ++ fam ++ ": " ++ show s ++ " (" ++ w
    ++ ") -- engine-owned; authored code may not touch it"
describe SeedlessDraw =
  "draw used but the die is unseeded: seed it with Prax.Engine.seedDie when building the world"
describe (DeadCondition w s) =
  "dead condition \"" ++ s ++ "\" (" ++ w
    ++ "): no action, initial fact, or axiom head can ever produce a match"
describe (DeonticUnclosed s) =
  "unclosed obligation rule \"" ++ s ++ "\": this world can invoke an"
    ++ " obligation but did not declare its □-closure -- wrap its axioms in"
    ++ " Prax.Deontic.obligedClose"
describe (CoercionUnmotivated n) =
  "unmotivated coercion \"" ++ n ++ "\": a threat deposits this punitive"
    ++ " belief but no such desire is registered, so the threat is silently"
    ++ " inert -- register it with Prax.Engine.setDesires (hold it or not is"
    ++ " the genuine/bluff choice)"

-- fixtures -------------------------------------------------------------------

runFixtures :: String -> IO ()
runFixtures name = case name of
  "db"     -> putJSON dbFixture
  "el"     -> putJSON elFixture
  "query"  -> putJSON queryFixture
  "derive" -> putJSON deriveFixture
  "kin"    -> putJSON kinFixture
  "div1"   -> putJSON div1Fixture
  "engine" -> putJSON engineFixture
  _        -> dieMsg ("unknown fixture " ++ show name
                      ++ " (one of db el query derive kin div1 engine)")

-- An axiom rendered for a fixture: the body conditions via Haskell `show` (the
-- same serialization the query corpus uses) and the head sentence templates.
axiomJSON :: Axiom -> Value
axiomJSON (Axiom whenC thenH) =
  object [ "when" .= map show whenC, "then" .= thenH ]

-- A closure result: the sorted labeled sentences under "ok", or the ⊥ witness
-- under "contradiction".
closureJSON :: Either Contradiction Db -> Value
closureJSON (Right d)                = object [ "ok" .= dbToLabeledSentences d ]
closureJSON (Left (Contradiction h)) = object [ "contradiction" .= h ]

-- A fact set as a fresh Db (facts inserted left to right).
buildDb :: [String] -> Db
buildDb ss = insertAll ss emptyDb

-- db corpus: the insert/retract/unify/ground scenarios from Prax.DbSpec, with
-- every expected value COMPUTED by the frozen functions (never transcribed).
data Op = Ins String | Ret String

applyOp :: Db -> Op -> Db
applyOp d (Ins s) = insert s d
applyOp d (Ret s) = retract s d

dbFixture :: Value
dbFixture = object
  [ "format"    .= (1 :: Int)
  , "mutations" .= map mutCase mutScenarios
  , "unify"     .= map unifyCase unifyScenarios
  , "ground"    .= map groundCase groundScenarios ]
  where
    mutCase (nm, ops, probes) =
      let d = foldl' applyOp emptyDb ops
      in object
           [ "name"      .= nm
           , "ops"       .= map opJSON ops
           , "sentences" .= dbToSentences d
           , "labeled"   .= dbToLabeledSentences d
           , "exists"    .= object [ K.fromString p .= exists p d | p <- probes ] ]
    opJSON (Ins s) = object [ "op" .= ("insert" :: String), "arg" .= s ]
    opJSON (Ret s) = object [ "op" .= ("retract" :: String), "arg" .= s ]
    unifyCase (nm, facts, patterns) =
      object
        [ "name"     .= nm
        , "facts"    .= facts
        , "patterns" .= patterns
        , "bindings" .= map bindingJSON (unifyAll patterns (buildDb facts)) ]
    groundCase (nm, pat, b) =
      object [ "name" .= nm, "pattern" .= pat
             , "binding" .= bindingJSON b, "result" .= ground pat b ]

mutScenarios :: [(String, [Op], [String])]
mutScenarios =
  [ ("basic multi-valued facts"
    , [Ins "foo.bar.baz", Ins "foo.bar.woof", Ins "foo.meow.woof"], [])
  , ("exclusion replaces single value"
    , [Ins "x.age!32", Ins "x.age!33"], [])
  , ("re-asserting ! parent preserves subtree"
    , [Ins "foo!bar.baz", Ins "foo!bar.meow"], [])
  , ("exclusion clears siblings when ! child changes"
    , [Ins "p!a.x", Ins "p!b.y"], [])
  , ("dot under ! child accumulates"
    , [Ins "g!closingStar!prebeginning"], [])
  , ("retract removes subtree by prefix"
    , [Ins "foo.bar.baz", Ins "foo.meow.woof", Ret "foo.bar"], [])
  , ("retract missing path is a no-op"
    , [Ins "foo.bar", Ret "nope.nothere"], [])
  , ("instance persistence: asserted instance survives its transient children draining"
    , [ Ins "practice.tendBar.bar.ada"
      , Ins "practice.tendBar.bar.ada.customer.you!order!beer"
      , Ret "practice.tendBar.bar.ada.customer.you!order"
      , Ins "practice.tendBar.bar.ada.customer.you!beverage!beer"
      , Ret "practice.tendBar.bar.ada.customer.you!beverage" ]
    , ["practice.tendBar.bar.ada", "practice.tendBar.bar.ada.customer.you"])
  , ("siblings and shared ancestors survive retracting the other sibling"
    , [ Ins "eve.lied.dana.stole.carol.loaf", Ins "eve.lied.dana.stole.carol.purse"
      , Ret "eve.lied.dana.stole.carol.loaf" ]
    , [ "eve.lied.dana.stole.carol.loaf", "eve.lied.dana.stole.carol.purse"
      , "eve.lied.dana.stole.carol", "eve.lied.dana.stole", "eve.lied.dana"
      , "eve.lied", "eve" ])
  , ("v38 repro: retracting the last targeted leaf prunes the drained toward ancestor"
    , [ Ins "carol.feels.angry.toward.bob", Ret "carol.feels.angry.toward.bob" ]
    , [ "carol.feels.angry.toward", "carol.feels.angry" ])
  , ("re-asserted scaffold survives its deep leaf retract"
    , [ Ins "carol.feels.angry.toward.bob", Ins "carol.feels.angry"
      , Ret "carol.feels.angry.toward.bob" ]
    , [ "carol.feels.angry", "carol.feels.angry.toward" ])
  ]

unifyScenarios :: [(String, [String], [String])]
unifyScenarios =
  [ ("two-sentence join binds shared variable"
    , ["foo.bar.woof", "foo.meow.woof", "fizz.buzz.foo", "some.other.woof"]
    , ["X.Y.woof", "fizz.buzz.X"])
  , ("bound variable descends deterministically"
    , ["char.tim", "char.kevin"], ["char.Who"])
  , ("absent constant yields no bindings"
    , ["char.tim"], ["char.nobody"])
  , ("unbound variable branches in name order, not id order"
    , ["at.zeta", "at.alpha", "at.mu"], ["at.Who"])
  ]

groundScenarios :: [(String, String, Bindings)]
groundScenarios =
  [ ("substitutes bound vars, preserves ! and ."
    , "practice.tendBar.B.customer.C!order!Bev"
    , Map.fromList [ (intern "B", VSym (intern "ada"))
                   , (intern "C", VSym (intern "beth"))
                   , (intern "Bev", VSym (intern "cider")) ])
  , ("unbound var grounds to its own name", "foo.Bar", Map.empty)
  , ("set-valued binding renders as opaque marker", "all.Dancers"
    , Map.fromList [(intern "Dancers", VSet [[intern "a"], [intern "b"]])])
  ]

-- el corpus: the meet/leq/bottom tables from Prax.ELSpec.
elFixture :: Value
elFixture = object
  [ "format" .= (1 :: Int)
  , "meet"   .= map meetCase meetScenarios
  , "leq"    .= map leqCase leqScenarios ]
  where
    meetCase (nm, a, b) = object
      [ "name" .= nm, "a" .= a, "b" .= b
      , "result" .= (toJSON (dbToSentences <$> meet (buildDb a) (buildDb b))) ]
    leqCase (nm, a, b) = object
      [ "name" .= nm, "a" .= a, "b" .= b
      , "result" .= leq (buildDb a) (buildDb b) ]

meetScenarios :: [(String, [String], [String])]
meetScenarios =
  [ ("compatible multi facts conjoin", ["a.b"], ["a.c"])
  , ("same exclusive fact is idempotent", ["x!a"], ["x!a"])
  , ("exclusive slot forced to two values is bottom", ["x!a"], ["x!b"])
  , ("exclusive vs different multi child is bottom (left)", ["x!a"], ["x.b"])
  , ("exclusive vs different multi child is bottom (right)", ["x.b"], ["x!a"])
  , ("two multi children never conflict", ["x.a"], ["x.b"])
  , ("a conflict deep in the tree propagates to bottom", ["p.q.r!a"], ["p.q.r!b"])
  , ("disjoint slots conjoin freely", ["at!bar"], ["mood!happy"])
  , ("meet preserves an assertion (disjunction of marks)", ["a", "a.b"], ["a.b"])
  ]

leqScenarios :: [(String, [String], [String])]
leqScenarios =
  [ ("more facts entail fewer", ["a.b", "a.c"], ["a.b"])
  , ("fewer facts do not entail more", ["a.b"], ["a.b", "a.c"])
  , ("a specific label entails the general (Excl <= Multi)", ["x!a"], ["x.a"])
  , ("the general does not entail the specific (Multi not<= Excl)", ["x.a"], ["x!a"])
  , ("everything entails the empty model", ["a.b"], [])
  , ("an asserted fact entails its scaffold", ["a", "a.b"], ["a.b"])
  , ("a scaffold does not entail the asserted fact", ["a.b"], ["a", "a.b"])
  ]

-- query corpus: the condition-eval tables from Prax.QuerySpec, over a fixed db.
queryFixture :: Value
queryFixture = object
  [ "format" .= (1 :: Int), "cases" .= map queryCase queryScenarios ]
  where
    queryCase (nm, facts, conds, initb) = object
      [ "name"    .= nm
      , "facts"   .= facts
      , "conds"   .= map show conds
      , "initial" .= bindingJSON initb
      , "results" .= map bindingJSON (query (buildDb facts) conds initb) ]

mkB :: [(String, String)] -> Bindings
mkB = Map.fromList . map (\(k, v) -> (intern k, VSym (intern v)))

queryScenarios :: [(String, [String], [Condition], Bindings)]
queryScenarios =
  [ ("bare sentence unifies and binds", ["char.tim", "char.kevin"]
    , [Match "char.Who"], Map.empty)
  , ("negation as failure keeps binding when absent", ["char.tim"]
    , [Not "isDancing.tim"], Map.empty)
  , ("negation as failure drops binding when present", ["isDancing.tim"]
    , [Not "isDancing.tim"], Map.empty)
  , ("eq binds an unbound variable to a constant", []
    , [Eq "X" "beer"], Map.empty)
  , ("eq of two equal bound values keeps the binding", []
    , [Eq "X" "Y"], mkB [("X", "a"), ("Y", "a")])
  , ("eq of two differing bound values drops the binding", []
    , [Eq "X" "Y"], mkB [("X", "a"), ("Y", "b")])
  , ("neq keeps distinct", []
    , [Neq "X" "Y"], mkB [("X", "a"), ("Y", "b")])
  , ("neq drops equal", []
    , [Neq "X" "Y"], mkB [("X", "a"), ("Y", "a")])
  , ("neq with an unbound operand drops the binding", []
    , [Neq "Actor" "Winner"], mkB [("Actor", "tim")])
  , ("gt fails below the threshold", ["counter.0"]
    , [Match "counter.Val", Cmp Gt "Val" "4"], Map.empty)
  , ("calc add binds the new value", ["counter.0"]
    , [Match "counter.Val", Calc "NewVal" Add "Val" "5"], Map.empty)
  , ("gt passes after an exclusion update", ["counter!5"]
    , [Match "counter.Val", Cmp Gt "Val" "4"], Map.empty)
  , ("chained calc: mul then sub yields -20", ["counter!5"]
    , [ Match "counter.Val", Calc "BigVal" Mul "Val" "Val"
      , Cmp Lt "Val" "BigVal", Calc "TinyVal" Sub "Val" "BigVal" ], Map.empty)
  , ("mod binds 17 mod 5 = 2", []
    , [Calc "R" Mod "17" "5"], Map.empty)
  , ("mod on a negative left operand: -3 mod 5 = 2", []
    , [Calc "R" Mod "-3" "5"], Map.empty)
  , ("count dancers other than the actor equals 2"
    , ["char.tim", "char.kevin", "char.james", "char.jer"
      , "isDancing.tim", "isDancing.kevin", "isDancing.jer"]
    , [ Match "char.Actor"
      , Subquery { subSet = "Dancers", subFind = ["Dancer"]
                 , subWhere = [ Match "char.Dancer", Match "isDancing.Dancer"
                              , Neq "Dancer" "Actor" ] }
      , Count "NumDancers" "Dancers", Eq "NumDancers" "2" ]
    , mkB [("Actor", "tim")])
  , ("eq on the count filters out the wrong actor"
    , ["char.tim", "char.solo", "isDancing.tim"]
    , [ Match "char.Actor"
      , Subquery { subSet = "Dancers", subFind = ["Dancer"]
                 , subWhere = [ Match "isDancing.Dancer", Neq "Dancer" "Actor" ] }
      , Count "NumDancers" "Dancers", Eq "NumDancers" "2" ]
    , mkB [("Actor", "solo")])
  , ("Or binds via either clause", ["p.a", "q.b"]
    , [Or [[Match "p.X"], [Match "q.X"]]], Map.empty)
  , ("Or deduplicates overlapping clauses", ["p.a", "q.a"]
    , [Or [[Match "p.X"], [Match "q.X"]]], Map.empty)
  , ("Absent holds when no male leader", ["leader.lucy", "lucy.sex!female"]
    , [Absent [Match "leader.L", Match "L.sex!male"]], Map.empty)
  , ("Absent fails when a male leader exists", ["leader.brown", "brown.sex!male"]
    , [Absent [Match "leader.L", Match "L.sex!male"]], Map.empty)
  , ("Exists is boolean and does not bind the witness"
    , ["char.tim", "char.kev", "here.ok"]
    , [Match "here.OK", Exists [Match "char.Who"]], Map.empty)
  , ("forAll holds when every patron has a drink"
    , ["patron.tim", "patron.kev", "drink.tim", "drink.kev"]
    , [forAllC [Match "patron.P"] [Match "drink.P"]], Map.empty)
  , ("forAll fails when one patron lacks a drink"
    , ["patron.tim", "patron.kev", "drink.tim"]
    , [forAllC [Match "patron.P"] [Match "drink.P"]], Map.empty)
  , ("implies: A and B", ["raining", "wet"]
    , [impliesC [Match "raining"] [Match "wet"]], Map.empty)
  , ("implies: A and not B", ["raining"]
    , [impliesC [Match "raining"] [Match "wet"]], Map.empty)
  , ("implies: not A (vacuous)", ["wet"]
    , [impliesC [Match "raining"] [Match "wet"]], Map.empty)
  , ("implies: empty world (vacuous)", []
    , [impliesC [Match "raining"] [Match "wet"]], Map.empty)
  ]

-- 'Prax.Query.forAll'/'implies' rebuilt here so the rendered @conds@ show the
-- compiled (Absent/Or) shape the evaluator actually runs.
forAllC :: [Condition] -> [Condition] -> Condition
forAllC guard body = Absent (guard ++ [Absent body])

impliesC :: [Condition] -> [Condition] -> Condition
impliesC a b = Or [[Absent a], b]

-- derive corpus: each shipped world's axiom set rendered, plus the closure of
-- its setup db (sorted labeled sentences), via the reference 'closure'.
deriveFixture :: Value
deriveFixture = object
  [ "format" .= (1 :: Int)
  , "worlds" .= [ worldDerive nm st | nm <- allWorldNames
                                     , Just (st, _) <- [worldNamed nm] ] ]
  where
    worldDerive nm st = object
      [ "world"   .= nm
      , "axioms"  .= map axiomJSON (axioms st)
      , "base"    .= dbToLabeledSentences (db st)
      , "closure" .= closureJSON (closure (axioms st) (db st)) ]

-- kin corpus: the Kin axioms' recursive closure over the KinSpec base (two
-- generations plus a marriage into a stranger family) — a genuinely multi-round
-- recursion (a derived `sibling` feeds a later `inLaw`). Emitted derive-shaped
-- (axioms/base/closure) so the replay reuses the derive machinery.
kinBase :: [String]
kinBase =
  [ "parent.gran.pat", "parent.pat.ana", "parent.pat.ben"
  , "parent.mia.cass", "parent.mia.dan", "married.ana.chris" ]

kinFixture :: Value
kinFixture = object
  [ "format"  .= (1 :: Int)
  , "axioms"  .= map axiomJSON kinAxioms
  , "base"    .= dbToLabeledSentences (buildDb kinBase)
  , "closure" .= closureJSON (closure kinAxioms (buildDb kinBase)) ]

-- DIV-1 negative fixture (docs/rewrite/DIVERGENCES.md): the probe on which the
-- frozen semi-naive under-derives. `frozen` is what the buggy engine computes
-- (r.a MISSING); `correct` is the hand-derived least fixpoint (r.a present),
-- which the Rust `close` must equal. Recording both makes the divergence a
-- committed red/green artifact rather than prose.
div1Axioms :: [Axiom]
div1Axioms =
  [ axiom [ Match "p.X", Exists [ Match "q.Y" ] ] [ "r.X" ]  -- Exists reads DERIVED q
  , axiom [ Match "trigger" ]                     [ "q.thing" ] ]
  where axiom = Axiom

div1Base :: [String]
div1Base = [ "p.a", "trigger" ]

div1Fixture :: Value
div1Fixture = object
  [ "format"  .= (1 :: Int)
  , "note"    .= ("DIV-1: an independent Exists over a derived, disjoint predicate; \
                  \frozen semi-naive drops r.a, the correct closure keeps it." :: String)
  , "axioms"  .= map axiomJSON div1Axioms
  , "base"    .= dbToLabeledSentences (buildDb div1Base)
  , "frozen"  .= closureJSON (closure div1Axioms (buildDb div1Base))
  , "correct" .= ([ "p.a", "q.thing", "r.a", "trigger" ] :: [String]) ]

-- engine corpus (D-panel I4): unit perform-sequences whose full state dumps
-- are OBSERVED from the frozen engine. Each scenario builds a world then applies
-- labeled steps; every step's post-state is dumped (labeled base facts, closed
-- view, cursor, rng, dues, expiries). The Rust replay reconstructs the same
-- world + steps and asserts each dump byte-for-byte — perform semantics pinned by
-- observation, not transcription. Corners: spawn (base-vs-view opacity, re-spawn
-- after delete), ForEach snapshot, Call (BASE-db quirk, first-case-first-binding),
-- expiry arm/refresh/cancel/purge, Roll advance-on-miss and hit, ⊥ collapse.
engineDump :: PraxState -> Value
engineDump st = object
  [ "facts"    .= dbToLabeledSentences (db st)
  , "view"     .= dbToLabeledSentences (readView st)
  , "cursor"   .= cursor st
  , "rng"      .= rngJSON (rngSeed st)
  , "dues"     .= duesJSON (scheduleDues st)
  , "expiries" .= expiriesJSON (expiries st) ]

-- A labeled state transition applied to the running scenario state.
type EngineStep = (String, PraxState -> PraxState)

engineScenario :: String -> PraxState -> [EngineStep] -> Value
engineScenario nm st0 steps = object
  [ "name"  .= nm
  , "steps" .= scanSteps st0 (("<initial>", id) : steps) ]
  where
    scanSteps _  []                = []
    scanSteps st ((lbl, f) : rest) =
      let st' = f st
      in object [ "label" .= lbl, "dump" .= engineDump st' ] : scanSteps st' rest

engineFixture :: Value
engineFixture = object
  [ "format"    .= (1 :: Int)
  , "scenarios" .=
      [ scenarioSpawnOpacity, scenarioRespawn, scenarioForEachSnapshot
      , scenarioCall, scenarioExpiry, scenarioRollMiss, scenarioRollHit
      , scenarioBottom ] ]
  where
    -- spawn: existedBefore reads the BASE db, so an instance the VIEW already
    -- shows (derived by an axiom) still spawns and runs its inits.
    spawnP = practice
      { practiceId = "pp", roles = ["R"]
      , initOutcomes = [ Insert "practice.pp.R.mark" ] }
    scenarioSpawnOpacity = engineScenario
      "spawn: base-vs-view opacity, inits run despite a view-visible instance"
      (setAxioms [ axiom [ Match "seed.X" ] [ "practice.pp.X" ] ]
                 (definePractices [spawnP] emptyState))
      [ ("insert seed.a (view derives practice.pp.a; base has not)"
        , performOutcome (Insert "seed.a"))
      , ("insert practice.pp.a (existedBefore reads BASE, so it spawns + inits)"
        , performOutcome (Insert "practice.pp.a")) ]

    respawnP = practice
      { practiceId = "rp", roles = ["R"]
      , initOutcomes = [ Insert "practice.rp.R.mark" ] }
    scenarioRespawn = engineScenario
      "spawn: re-spawn after delete re-runs init"
      (definePractices [respawnP] emptyState)
      [ ("insert practice.rp.a (spawns, mark set)"
        , performOutcome (Insert "practice.rp.a"))
      , ("delete practice.rp.a (subtree incl. mark gone)"
        , performOutcome (Delete "practice.rp.a"))
      , ("insert practice.rp.a again (re-spawns, mark set again)"
        , performOutcome (Insert "practice.rp.a")) ]

    scenarioForEachSnapshot = engineScenario
      "ForEach snapshots bindings: a member inserted mid-fold is not visited"
      emptyState
      [ ("insert member.a", performOutcome (Insert "member.a"))
      , ("ForEach member.X { insert member.b; insert visited.X }"
        , performOutcome (ForEach [ Match "member.X" ]
                                  [ Insert "member.b", Insert "visited.X" ])) ]

    -- Call queries the BASE db (not the view); it fires the first case and,
    -- within it, only the first binding.
    pickFn = Function "pick" ["Who"]
      [ FnCase [ Match "cand.Who.X" ] [ Insert "chose.X" ]
      , FnCase [] [ Insert "fallback" ] ]
    scenarioCall = engineScenario
      "Call: queries BASE (not the view), first case + first binding only"
      (setAxioms [ axiom [ Match "trig.W" ] [ "cand.W.zzz" ] ]
                 (defineFunctions [pickFn] emptyState))
      [ ("insert cand.gil.beta",  performOutcome (Insert "cand.gil.beta"))
      , ("insert cand.gil.alpha", performOutcome (Insert "cand.gil.alpha"))
      , ("insert trig.gil (view derives cand.gil.zzz; base has not)"
        , performOutcome (Insert "trig.gil"))
      , ("Call pick [gil] -> chose.alpha (base-only, name-first; no fallback)"
        , performOutcome (Call "pick" ["gil"])) ]

    scenarioExpiry = engineScenario
      "InsertFor: arm, refresh, bare-insert cancel, sibling arm, delete purge"
      emptyState
      [ ("InsertFor 3 a.b.c (arm due=3)",  performOutcome (InsertFor 3 "a.b.c"))
      , ("InsertFor 5 a.b.c (refresh due=5)", performOutcome (InsertFor 5 "a.b.c"))
      , ("Insert a.b.c bare (supersession cancels the timer)"
        , performOutcome (Insert "a.b.c"))
      , ("InsertFor 4 a.b.c (re-arm)",     performOutcome (InsertFor 4 "a.b.c"))
      , ("InsertFor 4 a.b.d (sibling arm)", performOutcome (InsertFor 4 "a.b.d"))
      , ("Delete a.b (purge every timer at or under)"
        , performOutcome (Delete "a.b")) ]

    scenarioRollMiss = engineScenario
      "Roll: unconditional advance on a miss (seed 1: rollStep is odd -> miss)"
      (seedDie 1 emptyState)
      [ ("Roll 1/2 [] [Insert roll.a] -> miss, seed advances anyway"
        , performOutcome (Roll 1 2 [] [ Insert "roll.a" ]))
      , ("Roll 1/2 [] [Insert roll.b] -> advances again (a miss is not sticky)"
        , performOutcome (Roll 1 2 [] [ Insert "roll.b" ])) ]

    scenarioRollHit = engineScenario
      "Roll: a hit applies the body (seed 2: rollStep is even -> hit)"
      (seedDie 2 emptyState)
      [ ("Roll 1/2 [] [Insert roll.hit] -> hit, seed advances"
        , performOutcome (Roll 1 2 [] [ Insert "roll.hit" ])) ]

    scenarioBottom = engineScenario
      "bottom collapse: a contradicting insert surfaces `contradiction` in the view"
      (setAxioms [ axiom [ Match "trig" ] [ "light!red" ]
                 , axiom [ Match "trig" ] [ "light!green" ] ]
                 emptyState)
      [ ("insert trig (closure hits bottom -> view = base + contradiction)"
        , performOutcome (Insert "trig")) ]

-- Entry point ----------------------------------------------------------------

main :: IO ()
main = do
  args <- getArgs
  case args of
    ("trace" : world : rest)     -> runTrace world rest
    ("randtrace" : world : rest) -> runRandtrace world rest
    ("check" : world : _)        -> runCheckCmd world
    ("fixtures" : name : _)      -> runFixtures name
    _ -> dieMsg (unlines
           [ "usage:"
           , "  prax-oracle trace <world> --turns N [--idle NAME] [--depth D] --mode decisions|state|view"
           , "  prax-oracle randtrace <world> --seed S --cap N [--candidates]"
           , "  prax-oracle check <world>"
           , "  prax-oracle fixtures db|el|query|derive|kin|div1|engine" ])
