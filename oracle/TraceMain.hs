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

import           Data.List (foldl', intercalate, sort, sortOn)
import           Data.Maybe (isNothing, listToMaybe, fromMaybe)
import qualified Data.Map.Strict as Map
import           Data.Word (Word64)
import           GHC.Float (castDoubleToWord64)
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
                              performOutcome, definePractice, definePractices,
                              defineFunctions, setAxioms, setCharacters,
                              setDesires, setSchedule, seedDie)
import           Prax.Loop (advance, npcAct, runNpcTicks)
import           Prax.TypeCheck (TypeError (..), typeCheck)
import           Prax.EL (meet, leq)
import           Prax.Query (Condition (..), CookedCondition (..), CmpOp (..),
                             CalcOp (..), forAll, implies, query)
import           Prax.Planner (candidateActions, motiveSignature, pickAction,
                               predictMove, scoreActions)
import           Prax.Relevance (moverReadAnchors)
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
  "planner" -> putJSON plannerFixture
  "npc"     -> putJSON npcFixture
  _        -> dieMsg ("unknown fixture " ++ show name
                      ++ " (one of db el query derive kin div1 engine planner npc)")

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

-- planner corpus (S6) ---------------------------------------------------------
--
-- The stage's most valuable artifact: synthetic worlds built HERE (the library
-- is imported, never edited) whose planner observables are dumped from the
-- FROZEN engine. The Rust replay reconstructs each world with its own builder
-- API and asserts every dump equal.
--
-- Two emission rules are specific to this corpus:
--
--   * [D-C1] scored tables are emitted in NATIVE result order — the ordering IS
--     the observable under test, so the oracle's sort-everything convention is
--     suspended for the scored rows (and for them alone).
--   * [D-I1] every score is emitted as its RAW IEEE-754 bit pattern
--     ('castDoubleToWord64', as a JSON integer). There is no decimal in the
--     trusted comparison base: the replay compares @u64 == f64::to_bits@.
--
-- The relevance tables (improvables/liveness/caresAbout/moverReadAnchors) are
-- likewise emitted in their native table order: each is a pure function of the
-- compiled world (Map traversals are name-ordered on both sides), never
-- run-dependent, and its order is itself part of the contract.

-- | A path pattern rendered by name (the ONE rendering both sides produce:
-- interned segment names joined by @.@ — cooked conditions carry no
-- punctuation, so none is rendered).
renderSyms :: [Sym] -> String
renderSyms = intercalate "." . map symName

-- | A score as its raw IEEE-754 bits [D-I1]. Emitted as an 'Integer' so Aeson
-- prints the 64-bit value exactly (no decimal anywhere in the channel).
bitsJSON :: Double -> Value
bitsJSON = toJSON . (toInteger :: Word64 -> Integer) . castDoubleToWord64

-- | A dead-now recipe, rendered: the tag, and for a 'GateCheck' each gate's
-- conjunct patterns by name. Loud on any gate shape 'livenessOf' does not
-- build (it emits single-'CMatch' gates only) — a silent skip here would hide
-- a real divergence.
livenessJSON :: Liveness -> Value
livenessJSON FloorCheck     = toJSON ("FloorCheck" :: String)
livenessJSON AlwaysLive     = toJSON ("AlwaysLive" :: String)
livenessJSON (GateCheck gs) = object [ "GateCheck" .= map (map one) gs ]
  where
    one (CMatch p) = renderSyms p
    one c          = error ("livenessJSON: unexpected gate condition " ++ show c)

-- | A motive signature, field for field.
sigJSON :: MotiveSignature -> Value
sigJSON ms = object
  [ "bearing"      .= msBearing ms
  , "satisfaction" .= msSatisfaction ms
  , "liveDesires"  .= msLiveDesires ms
  , "knownMotives" .= [ [m, d] | (m, d) <- msKnownMotives ms ] ]

-- | Apply outcomes left to right.
perf :: [Outcome] -> PraxState -> PraxState
perf os st = foldl' (flip performOutcome) st os

-- | Every planner observable of one synthetic world.
plannerWorldJSON :: (String, PraxState) -> Value
plannerWorldJSON (nm, st) = object
  [ "name"        .= nm
  , "improvables" .= improvables st
  , "liveness"    .= object [ K.fromString k .= livenessJSON v
                            | (k, v) <- Map.toList (liveness st) ]
  , "caresAbout"  .= object [ K.fromString k .= v
                            | (k, v) <- Map.toList (caresAbout st) ]
  , "readAnchors" .= [ object [ "actor" .= charName a, "mover" .= charName m
                              , "anchors" .= map renderSyms (moverReadAnchors st a m) ]
                     | a <- cast, m <- cast, charName a /= charName m ]
  , "predict"     .= [ object [ "predictor" .= charName p, "mover" .= charName m
                              , "action" .= fmap gaLabel (predictMove st p m) ]
                     | p <- cast, m <- cast, charName p /= charName m ]
  , "signatures"  .= [ object [ "character" .= charName c
                              , "signature" .= sigJSON (motiveSignature st c) ]
                     | c <- cast ]
  , "candidates"  .= [ object [ "character" .= charName c
                              , "actions" .= map gaLabel (candidateActions st c) ]
                     | c <- cast ]
  , "scored"      .= [ object [ "actor" .= charName c, "depth" .= d
                              , "rows" .= [ object [ "label" .= gaLabel ga
                                                   , "bits"  .= bitsJSON s ]
                                          | (ga, s) <- scoreActions d st c ] ]
                     | c <- cast, d <- depths ]
  , "pick"        .= [ object [ "actor" .= charName c, "depth" .= d
                              , "action" .= fmap gaLabel (pickAction d st c) ]
                     | c <- cast, d <- depths ]
  ]
  where
    cast   = characters st
    depths = [0, 1, 2] :: [Int]

plannerFixture :: Value
plannerFixture = object
  [ "format" .= (1 :: Int)
  , "worlds" .= map plannerWorldJSON plannerWorlds ]

-- The corpus's worlds. Every one is built from library setters only.
plannerWorlds :: [(String, PraxState)]
plannerWorlds =
  [ ("tendBar: two instances, two customers", wTendBar)
  , ("forall-host: a universal desire and a vacuous implication", wForallHost)
  , ("models: gossiped, seen, and false believed minds", wModels)
  , ("scope: the pair apart", wScopeApart)
  , ("scope: the pair together", wScopeTogether)
  , ("deadNow: floor shut, gate shut, subquery always live", wDeadNowShut)
  , ("deadNow: floor marked, gate open", wDeadNowLive)
  , ("reuse: the cone-mediated read (a derived head only)", wReuseCone)
  , ("reuse: the eviction shadow (an exclusion displaces the read)", wReuseEviction)
  , ("collision: a Calc-minted constant colliding with a scope literal", wCollision)
  , ("wild Call: cares_about bears on everyone", wWildCall)
  , ("the fold-order canary", wCanary)
  ]

-- W1: the tendBar shape — one practice, TWO instances, two customers with
-- different beverage wants. Exercises multi-instance candidate enumeration and
-- the instance-binding order the scored table's ties fall back to.
wTendBar :: PraxState
wTendBar = perf
  [ Insert "practice.tendBar.ada", Insert "practice.tendBar.cleo"
  , Insert "practice.tendBar.ada.customer.beth" ]
  (setCharacters [ bethCider, danaSoda, character "ada", character "cleo" ]
     (definePractices [tendBarP] emptyState))
  where
    bethCider = (character "beth")
      { charWants = [ Want [ Match "practice.tendBar.Bartender.customer.beth!order!cider" ] 10 ] }
    danaSoda = (character "dana")
      { charWants = [ Want [ Match "practice.tendBar.Bartender.customer.dana!order!soda" ] 8 ] }

tendBarP :: Practice
tendBarP = practice
  { practiceId = "tendBar"
  , practiceName = "[Bartender] is tending bar"
  , roles = ["Bartender"]
  , dataFacts =
      [ "beverageType.beer!alcoholic", "beverageType.cider!alcoholic"
      , "beverageType.soda!nonalcoholic" ]
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
      ]
  }

-- W2: the ∀-host — a universally quantified want (Absent/Absent) plus a
-- vacuously true implication (Or/Absent), so both compiled quantifier shapes
-- appear in the scored arithmetic AND in the read-anchor walk.
wForallHost :: PraxState
wForallHost = perf
  [ Insert "guest.a", Insert "guest.b", Insert "hasDrink.a"
  , Insert "practice.serve.host" ]
  (setCharacters [ host, character "b" ] (definePractice serveP emptyState))
  where
    serveP = practice
      { practiceId = "serve", practiceName = "[Host] hosts", roles = ["Host"]
      , actions =
          [ action "[Actor]: pour a drink for [Guest]"
              [ Match "guest.Guest", Not "hasDrink.Guest" ]
              [ Insert "hasDrink.Guest" ]
          , action "[Actor]: rest" [] [] ] }
    host = (character "host")
      { charWants = [ Want [ forAll [Match "guest.G"] [Match "hasDrink.G"] ] 10
                    , Want [ implies [Match "raining"] [Match "wet"] ] 4 ] }

-- W3: the believed-model divergence — ada's model of each mover is what drives
-- prediction, and it is wrong in two directions: beth genuinely holds the
-- craving (gossiped), dana holds it too (seen), cleo does NOT (presumed —
-- a FALSE belief that predicts a move cleo would never take).
wModels :: PraxState
wModels = perf
  [ Insert "practice.tendBar.ada"
  , Insert "practice.tendBar.ada.customer.beth"
  , Insert "practice.tendBar.ada.customer.cleo"
  , Insert "practice.tendBar.ada.customer.dana"
  , Insert "ada.believes.desires.beth.cider-craving.heard.gossip"
  , Insert "ada.believes.desires.dana.cider-craving.seen"
  , Insert "ada.believes.desires.cleo.cider-craving.presumed" ]
  (setDesires vocab
     (setCharacters [ character "ada", holder "beth", character "cleo", holder "dana" ]
        (definePractices [tendBarP] emptyState)))
  where
    vocab = [ Desire "cider-craving"
                (Want [ Match "practice.tendBar.Bartender.customer.Owner!order!cider" ] 10) ]
    holder n = (character n) { charDesires = ["cider-craving"] }

-- W4: the prediction scope. The same heist twice — the pair in different rooms
-- (the scope query fails, the mover is modelled as still) and in the same room
-- (the scope passes and the enabling move is credited).
heistP :: Practice
heistP = practice
  { practiceId = "heist", roles = ["R"]
  , actions =
      [ action "[Actor]: grab the relic"
          [ Match "gate.open", Not "grabbed.inge", Eq "Actor" "inge" ]
          [ Insert "grabbed.inge" ]
      , action "[Actor]: open the gate"
          [ Eq "Actor" "olaf", Not "gate.open" ]
          [ Insert "gate.open" ]
      , action "[Actor]: Wait about" [] [] ]
  }

heistBase :: PraxState
heistBase =
  (setDesires [ Desire "covet-relic" (Want [ Match "grabbed.Owner" ] 10) ]
     (setCharacters [ olaf, inge ] (definePractices [heistP] emptyState)))
    { predictionScope = [ Match "at.Actor!Room", Match "at.Witness!Room" ] }
  where
    olaf = (character "olaf") { charWants = [ Want [ Match "grabbed.inge" ] 6 ] }
    inge = (character "inge") { charDesires = ["covet-relic"] }

wScopeApart :: PraxState
wScopeApart = perf
  [ Insert "practice.heist.here"
  , Insert "olaf.believes.desires.inge.covet-relic.heard.inge"
  , Insert "at.olaf!gatehouse", Insert "at.inge!vault" ] heistBase

wScopeTogether :: PraxState
wScopeTogether = perf
  [ Insert "practice.heist.here"
  , Insert "olaf.believes.desires.inge.covet-relic.heard.inge"
  , Insert "at.olaf!vault", Insert "at.inge!vault" ] heistBase

-- W5: all three dead-now recipes in ONE vocabulary, dumped in both states.
--
--   * @hates-lying@ (negative) → FloorCheck: dead while no lied mark stands.
--   * @wants-market@ (positive, gated on a fact NO mover action inserts and no
--     axiom derives — only the engine schedule moves it) → GateCheck.
--   * @counts-neighbours@ (Subquery/Count-tainted) → AlwaysLive.
deadNowBase :: PraxState
deadNowBase =
  setSchedule [ marketRule ]
    (setDesires vocab
       (setCharacters [ priya, beth ] (definePractices [townP] emptyState)))
  where
    priya = (character "priya") { charWants = [ Want [ Match "sold.beth" ] 5 ] }
    beth  = (character "beth")
      { charDesires = ["hates-lying", "wants-market", "counts-neighbours"] }
    vocab =
      [ Desire "hates-lying" (Want [ Match "lied.Owner" ] (-5))
      , Desire "wants-market" (Want [ Match "marketDay", Match "sold.Owner" ] 5)
      , Desire "counts-neighbours"
          (Want [ Subquery { subSet = "Ns", subFind = ["N"]
                           , subWhere = [ Match "neighbour.N" ] }
                , Count "K" "Ns", Cmp Gte "K" "1" ] 5)
      ]
    townP = practice
      { practiceId = "town", roles = ["R"]
      , actions =
          [ action "[Actor]: confess"
              [ Match "lied.Actor" ] [ Delete "lied.Actor" ]
          , action "[Actor]: sell at the market"
              [ Match "marketDay", Not "sold.Actor" ] [ Insert "sold.Actor" ]
          , action "[Actor]: greet a neighbour"
              [ Not "neighbour.Actor" ] [ Insert "neighbour.Actor" ]
          , action "[Actor]: Wait about" [] [] ] }
    -- The schedule is not a mover: the fact it moves stays an environment gate.
    marketRule = ScheduleRule
      { srName = "market", srPeriod = 2
      , srBody = [ ([ Not "marketDay" ], [ Insert "marketDay" ]) ] }

wDeadNowShut :: PraxState
wDeadNowShut = perf
  [ Insert "practice.town.here"
  , Insert "priya.believes.desires.beth.hates-lying.heard.gossip"
  , Insert "priya.believes.desires.beth.wants-market.heard.gossip" ] deadNowBase

wDeadNowLive :: PraxState
wDeadNowLive = perf [ Insert "lied.beth", Insert "marketDay" ] wDeadNowShut

-- W6 [S-I3]: the CONE-mediated reuse case. beth's believed desire reads only a
-- DERIVED head (@regards.*@); priya's candidate inserts the BASE fact the axiom
-- fires on. Only extendDelta's cone fold puts the head into the delta, so only
-- it stops the root's "beth is unmotivated" from being reused.
wReuseCone :: PraxState
wReuseCone = perf
  [ Insert "practice.court.here"
  , Insert "priya.believes.desires.beth.hates-infamy.heard.gossip" ]
  (setAxioms [ axiom [ Match "W.believes.C.thief", Not "recanted.C" ]
                     [ "regards.W.C.thief" ] ]
     (setDesires [ Desire "hates-infamy" (Want [ Match "regards.V.Owner.thief" ] (-8)) ]
        (setCharacters [ priya, character "beth" ]
           (definePractices [courtP] emptyState))))
  where
    priya = (character "priya") { charWants = [ Want [ Match "apology.beth" ] 10 ] }
    courtP = practice
      { practiceId = "court", roles = ["R"]
      , actions =
          [ action "[Actor]: denounce beth"
              [ Neq "Actor" "beth" ] [ Insert "Actor.believes.beth.thief" ]
          , action "[Actor]: make amends"
              [ Match "regards.V.Actor.thief" ]
              [ Insert "recanted.Actor", Insert "apology.Actor" ]
          , action "[Actor]: bide time" [] [] ] }

-- W7 [S-I3]: the EVICTION-SHADOW reuse case. beth's read anchor is
-- @mood.beth!sad@; alice's candidate inserts @mood.beth!happy@, whose own
-- anchor does NOT may-unify the read (two distinct literals in the last
-- segment). Only the exclusion's eviction shadow (@mood.beth.PraxEvicted@)
-- intersects — so only a delta carrying shadows blocks the stale reuse.
wReuseEviction :: PraxState
wReuseEviction = perf
  [ Insert "practice.parlour.here", Insert "mood.beth!sad"
  , Insert "alice.believes.desires.beth.wants-to-mope.heard.gossip" ]
  (setDesires [ Desire "wants-to-mope" (Want [ Match "moped.Owner" ] 5) ]
     (setCharacters [ alice, character "beth" ]
        (definePractices [parlourP] emptyState)))
  where
    alice = (character "alice") { charWants = [ Want [ Match "moped.beth" ] 6 ] }
    parlourP = practice
      { practiceId = "parlour", roles = ["R"]
      , actions =
          [ action "[Actor]: mope"
              [ Eq "Actor" "beth", Match "mood.Actor!sad", Not "moped.Actor" ]
              [ Insert "moped.Actor" ]
          , action "[Actor]: console beth"
              -- Deliberately guarded on the ACTOR alone: any guard mentioning
              -- the mood family would itself become a read anchor of every
              -- mover, and the insert's own anchor would then block the reuse
              -- without the shadow ever being consulted. With only the actor
              -- test, @mood.beth!happy@ shares no anchor with @mood.beth!sad@
              -- (two distinct literals in the last segment), so the EVICTION
              -- SHADOW is the only thing that can stop the stale reuse.
              [ Eq "Actor" "alice" ]
              [ Insert "mood.beth!happy" ]
          , action "[Actor]: Wait about" [] [] ] }

-- W8 [S-C1]: the collision fixture. The prediction scope reads the LITERAL
-- @gate.2@; alice's candidate MINTS the name @2@ at run time (a Calc result)
-- and inserts @gate.2@. The delta anchor is that runtime-minted constant and
-- the read anchor is the compile-time one — they must compare EQUAL, or the
-- gate misses the intersection and reuses the root's out-of-scope Nothing
-- after the move has brought the mover into scope.
--
-- The action is guarded on @computed.Actor@, NOT on @gate.2@. A @gate.2@ guard
-- reads that family, so it becomes a read anchor of every mover in its own
-- right (the affordance walk grounds Actor:=mover) — the scope's contribution
-- to moverReadAnchors is then a duplicate, and dropping the SCOPE component
-- entirely leaves every score and pick unchanged. With the guard moved off the
-- gate family, the scope read is the ONLY anchor that intersects the delta:
-- suppress it and alice's depth-1/2 pick flips from "compute the gate" to
-- "Wait about", because bob's move is reused stale as Nothing.
wCollision :: PraxState
wCollision = perf
  [ Insert "practice.signal.here"
  , Insert "alice.believes.desires.bob.wants-cheer.heard.gossip" ]
  ((setDesires [ Desire "wants-cheer" (Want [ Match "cheer.Owner" ] 5) ]
      (setCharacters [ alice, character "bob" ]
         (definePractices [signalP] emptyState)))
     { predictionScope = [ Match "gate.2" ] })
  where
    alice = (character "alice") { charWants = [ Want [ Match "cheer.bob" ] 4 ] }
    signalP = practice
      { practiceId = "signal", roles = ["R"]
      , actions =
          [ action "[Actor]: compute the gate"
              [ Eq "Actor" "alice", Not "computed.Actor", Calc "Sum" Add "1" "1" ]
              [ Insert "computed.Actor", Insert "gate.Sum" ]
          , action "[Actor]: cheer"
              [ Eq "Actor" "bob", Not "cheer.Actor" ] [ Insert "cheer.Actor" ]
          , action "[Actor]: Wait about" [] [] ] }

-- W9: the wild-Call branch of bearingTemplates. @rouse@'s outcome is a Call to
-- a function that is NOT registered, so its atom set is unresolvable — every
-- character bears it, and every desire is conservatively improvable/AlwaysLive.
wWildCall :: PraxState
wWildCall = perf [ Insert "practice.rumour.here" ]
  (setDesires [ Desire "wants-quiet" (Want [ Match "quiet.Owner" ] 5) ]
     (setCharacters [ alice, beth ] (definePractices [rumourP] emptyState)))
  where
    alice = (character "alice") { charWants = [ Want [ Match "quiet.alice" ] 3 ] }
    beth  = (character "beth") { charDesires = ["wants-quiet"] }
    rumourP = practice
      { practiceId = "rumour", roles = ["R"]
      , actions =
          [ action "[Actor]: rouse the room" [] [ Call "noSuchFunction" [] ]
          , action "[Actor]: hush" [ Not "quiet.Actor" ] [ Insert "quiet.Actor" ]
          , action "[Actor]: Wait about" [] [] ] }

-- W10: THE FOLD-ORDER CANARY, as a world [S-I2].
--
-- alice's "raise" candidate at depth 2 is engineered to hit the corrected
-- payoffs exactly:
--
--   base = 12   — @mark.p@ (utility 12) holds after raise
--   acc  = 3.5  — three predicted movers, cumulative evals 7, 0, 0
--                 (bob swaps p for q: 7; cara clears q: 0; dan sits: 0)
--   v    = 0.9  — the depth-1 continuation: alice's only remaining candidate
--                 ("reach") scores 0 + (0 + 0.9*1), the 1 being the depth-0
--                 "mark s" (utility 1)
--
-- so the score is @12 + (3.5 + 0.9*0.9)@. The two associations land exactly one
-- ULP apart (…696 vs …695), and the replay compares raw bits — re-associating
-- the Rust fold reddens THIS fixture, not merely the native unit canary.
--
-- Two further discriminators ride along: eve is predicted Nothing (no believed
-- model) yet HAS a candidate that would insert @mark.s@ — an implementation
-- that contributed a term for an unmotivated mover moves the bits; and the
-- nested 0.9 (v is itself 0.9·1) separates a misplaced discount from the right
-- one.
wCanary :: PraxState
wCanary = perf
  [ Insert "practice.stage.here"
  , Insert "alice.believes.desires.bob.swap-marks.heard.gossip"
  , Insert "alice.believes.desires.cara.tidy-marks.heard.gossip"
  , Insert "alice.believes.desires.dan.take-a-seat.heard.gossip" ]
  (setDesires vocab
     (setCharacters [ alice, character "bob", character "cara"
                    , character "dan", character "eve" ]
        (definePractices [stageP] emptyState)))
  where
    alice = (character "alice")
      { charWants = [ Want [ Match "mark.p" ] 12
                    , Want [ Match "mark.q" ] 7
                    , Want [ Match "mark.s" ] 1 ] }
    vocab =
      [ Desire "swap-marks"   (Want [ Match "mark.q" ] 5)
      , Desire "tidy-marks"   (Want [ Not "mark.q" ] 5)
      , Desire "take-a-seat"  (Want [ Match "chair.Owner" ] 5) ]
    stageP = practice
      { practiceId = "stage", roles = ["R"]
      , actions =
          [ action "[Actor]: raise the mark"
              [ Eq "Actor" "alice", Not "raised.Actor" ]
              [ Insert "raised.Actor", Insert "mark.p" ]
          , action "[Actor]: reach for the shelf"
              [ Eq "Actor" "alice", Not "reached.Actor" ]
              [ Insert "reached.Actor" ]
          , action "[Actor]: take the small mark"
              [ Eq "Actor" "alice", Match "reached.Actor", Not "mark.s" ]
              [ Insert "mark.s" ]
          , action "[Actor]: swap the marks"
              [ Eq "Actor" "bob", Match "mark.p" ]
              [ Delete "mark.p", Insert "mark.q" ]
          , action "[Actor]: tidy the marks"
              [ Eq "Actor" "cara", Match "mark.q" ]
              [ Delete "mark.q" ]
          , action "[Actor]: take a seat"
              [ Eq "Actor" "dan", Not "chair.Actor" ]
              [ Insert "chair.Actor" ]
          , action "[Actor]: polish the small mark"
              [ Eq "Actor" "eve", Not "mark.s" ]
              [ Insert "mark.s" ]
          ] }

-- npc corpus (S6) -------------------------------------------------------------
--
-- 'Prax.Loop.runNpcTicks' end to end, before any shipped world exists: the
-- narration of a 24-turn run over a synthetic cast, plus the full final engine
-- dump, the standing-intention map, and who is still alive. The three scenarios
-- cover the loop's whole S6 surface — round boundaries firing on the rotation
-- wrap, a death mid-run (the corpse is skipped for the rest of the run), and the
-- v35 commitment semantics in both directions: intentions HOLDING through quiet
-- rounds and WAKING when a SCHEDULE rule changes the world under them (the v37
-- wake: a gated desire's liveness flips, so the motive signature no longer
-- matches the standing intention's basis).

intentionJSON :: Intention -> Value
intentionJSON i = object
  [ "act"   .= fmap gaLabel (intentAct i)
  , "basis" .= sigJSON (intentBasis i) ]

npcScenarioJSON :: (String, Int, Int, PraxState) -> Value
npcScenarioJSON (nm, depth, steps, st0) = object
  [ "name"       .= nm
  , "depth"      .= depth
  , "steps"      .= steps
  , "narration"  .= narration
  , "final"      .= engineDump stF
  , "intentions" .= object [ K.fromString k .= intentionJSON v
                           | (k, v) <- Map.toList (intentions stF) ]
  , "alive"      .= [ charName c | c <- characters stF
                    , not (exists (deadSentence (charName c)) (db stF)) ]
  ]
  where (narration, stF) = runNpcTicks depth steps st0

npcFixture :: Value
npcFixture = object
  [ "format"    .= (1 :: Int)
  , "scenarios" .= map npcScenarioJSON npcScenarios ]

npcScenarios :: [(String, Int, Int, PraxState)]
npcScenarios =
  [ ("npc: boundaries and quiet holds", 2, 24, nQuiet)
  , ("npc: a death mid-run", 2, 24, nDeath)
  , ("npc: the schedule-gated wake", 2, 24, nWake)
  ]

-- N1: three characters, two small wants, no schedule. Every rotation wrap fires
-- a round boundary (the clock advances); once each want is satisfied the cast
-- idles and every standing intention HOLDS — the narration goes quiet and stays
-- quiet, which is exactly what commitment looks like.
nQuiet :: PraxState
nQuiet = perf [ Insert "practice.yard.here" ]
  (setCharacters [ alice, bob, character "cara" ]
     (definePractices [yardP] emptyState))
  where
    alice = (character "alice") { charWants = [ Want [ Match "swept.alice" ] 2 ] }
    bob   = (character "bob")   { charWants = [ Want [ Match "swept.bob" ] 2 ] }

yardP :: Practice
yardP = practice
  { practiceId = "yard", roles = ["R"]
  , actions =
      [ action "[Actor]: sweep the step"
          [ Not "swept.Actor" ] [ Insert "swept.Actor" ]
      , action "[Actor]: idle about" [] [] ]
  }

-- N2: a death mid-run. cara wants bob dead and can strike him; from the turn
-- the mark lands, 'Prax.Loop.advance' skips the corpse and 'candidateActions'
-- gives him nothing — he appears in no further narration line and in no
-- prediction.
nDeath :: PraxState
nDeath = perf [ Insert "practice.duel.here" ]
  (setCharacters [ alice, character "bob", cara ]
     (definePractices [duelP] emptyState))
  where
    alice = (character "alice") { charWants = [ Want [ Match "swept.alice" ] 2 ] }
    cara  = (character "cara")  { charWants = [ Want [ Match "dead.bob" ] 9 ] }
    duelP = practice
      { practiceId = "duel", roles = ["R"]
      , actions =
          [ action "[Actor]: strike bob"
              [ Eq "Actor" "cara", Not "dead.bob" ] [ Insert "dead.bob" ]
          , action "[Actor]: sweep the step"
              [ Not "swept.Actor" ] [ Insert "swept.Actor" ]
          , action "[Actor]: idle about" [] [] ]
      }

-- N3: THE SCHEDULE-GATED WAKE. alice holds a desire gated on @marketDay@ — a
-- fact NO mover action inserts and no axiom derives, so 'Prax.Relevance.livenessOf'
-- classifies it @GateCheck@ and it reads DEAD while the gate is shut. She commits
-- to idling. Three boundaries in, the schedule rule opens the market: her live
-- desire set gains @wants-market@, her motive signature no longer equals her
-- standing intention's basis, and she deliberates afresh — and sells.
nWake :: PraxState
nWake = perf [ Insert "practice.square.here" ]
  (setSchedule [ marketRule ]
     (setDesires vocab
        (setCharacters [ alice, bob, character "cara" ]
           (definePractices [squareP] emptyState))))
  where
    alice = (character "alice") { charDesires = ["wants-market"] }
    bob   = (character "bob")   { charWants = [ Want [ Match "swept.bob" ] 2 ] }
    vocab = [ Desire "wants-market" (Want [ Match "marketDay", Match "sold.Owner" ] 5) ]
    marketRule = ScheduleRule
      { srName = "market", srPeriod = 3
      , srBody = [ ([ Not "marketDay" ], [ Insert "marketDay" ]) ] }
    squareP = practice
      { practiceId = "square", roles = ["R"]
      , actions =
          [ action "[Actor]: sell at the market"
              [ Match "marketDay", Not "sold.Actor" ] [ Insert "sold.Actor" ]
          , action "[Actor]: sweep the step"
              [ Not "swept.Actor" ] [ Insert "swept.Actor" ]
          , action "[Actor]: idle about" [] [] ]
      }

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
           , "  prax-oracle fixtures db|el|query|derive|kin|div1|engine|planner|npc" ])
