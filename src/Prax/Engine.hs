-- | The interpreter: registering practices, discovering an actor's affordances,
-- and applying an action's outcomes to the world (including practice spawning
-- and function calls).
--
-- A typed port of the @Praxish.*@ functions in @praxish.js@. All operations are
-- pure state transformations — the planner ("Prax.Planner") exploits this by
-- speculatively applying an action and simply discarding the result, replacing
-- Praxish's clone-and-undo.
module Prax.Engine
  ( definePractice
  , definePractices
  , renderText
  , withDb
  , setAxioms
  , setDesires
  , possibleActions
  , performAction
  , performOutcome
  , performCooked
  , groundOutcome
  , relevantDelta
  , monotoneInsert
  ) where

import           Data.List (intercalate)
import qualified Data.Map.Strict as Map

import           Prax.Db
import           Prax.Query (queryCooked, groundCondition, CookedCondition)
import           Prax.Types
import           Prax.Derive (Axiom, closure, closureFrom, axiomFootprint, axiomNegPatterns, monotoneAxioms)
import           Prax.Relevance (improvableDesires, mayUnifyNames, evictionShadows)
import           Prax.Cooked (cookOutcome, cookPractice, groundCookedOutcome)

-- | Rebuild the derived vocabulary tables. Internal: every helper that
-- changes 'practiceDefs', 'axioms', or 'desires' must end here.
retable :: PraxState -> PraxState
retable st = st
  { cookedDefs   = Map.map cookPractice (practiceDefs st)
  , improvables  = improvableDesires (practiceDefs st) (axioms st) (desires st)
  , footprint    = map pathNames (axiomFootprint (axioms st))
  , negFootprint = map pathNames (axiomNegPatterns (axioms st))
  , contMonotone = monotoneAxioms (axioms st) }

-- | Eviction shadows computed directly from tokens — the names-level mirror
-- of 'Prax.Relevance.evictionShadows' (one shadow per @!@ operator: the names
-- up to and including that point, followed by a fresh @"Evicted"@ segment),
-- with no round trip through a rebuilt sentence.
evictionShadowNames :: [(String, Maybe Char)] -> [[String]]
evictionShadowNames toks =
  [ map fst (take i toks) ++ ["Evicted"]
  | (i, (_, op)) <- zip [1 ..] toks, op == Just '!' ]

-- | 'relevantDelta' generalized to operate on already-split names: the
-- primary delta's names and its eviction shadows (each already a name list).
-- The hot cooked path ('performCooked') calls this directly — it never
-- reparses a sentence to recover what it already has as tokens.
relevantNames :: [String] -> [[String]] -> PraxState -> Bool
relevantNames names shadows st =
  any (\ns -> any (mayUnifyNames ns) (footprint st)) (names : shadows)

-- | Can this ground delta change what the axioms derive? Conservative:
-- False only when the sentence — and anything its exclusions evict —
-- may-unify nothing in the axioms' footprint (v27 spec theorem). False is
-- the licence for 'performOutcome' to skip 'reclose'.
relevantDelta :: String -> PraxState -> Bool
relevantDelta s = relevantNames (pathNames s) (map pathNames (evictionShadows s))

-- | 'monotoneInsert' generalized to operate on already-split tokens.
monotoneToks :: [(String, Maybe Char)] -> PraxState -> Bool
monotoneToks toks st =
  contMonotone st
    && all ((/= Just '!') . snd) toks
    && not (any (mayUnifyNames (map fst toks)) (negFootprint st))

-- | May this insert take the continuation tier: the world is
-- continuation-safe, the insert evicts nothing, and it can defeat nothing.
monotoneInsert :: String -> PraxState -> Bool
monotoneInsert s = monotoneToks (tokens s)

-- | 'applyGrow' from already-split tokens. Bridges to the sentence form only
-- because 'Prax.Derive.closureFrom' — a module outside this task's touched
-- set — is typed over 'String'; the bridge is a cheap 'tokensToSentence'
-- concatenation, the exact inverse of the 'tokens' that produced @toks@, not
-- a re-parse of anything already computed.
applyGrowToks :: [(String, Maybe Char)] -> PraxState -> PraxState
applyGrowToks toks = applyGrow (tokensToSentence toks)

-- | Register a practice and insert its static @dataFacts@ under
-- @practiceData.<id>.@.
definePractice :: Practice -> PraxState -> PraxState
definePractice p st =
  retable (withDb (insertAll (map (prefix ++) (dataFacts p))) st)
    { practiceDefs = Map.insert (practiceId p) p (practiceDefs st) }
  where prefix = "practiceData." ++ practiceId p ++ "."

-- | Register several practices in order.
definePractices :: [Practice] -> PraxState -> PraxState
definePractices ps st = foldl' (flip definePractice) st ps

-- | Substitute @[Var]@ placeholders in a template using the bindings, leaving
-- unknown placeholders untouched.
renderText :: String -> Bindings -> String
renderText template b = go template
  where
    go [] = []
    go ('[' : rest) =
      case break (== ']') rest of
        (name, ']' : after) ->
          maybe ('[' : name ++ "]") valToString (Map.lookup name b) ++ go after
        _ -> '[' : go rest      -- unterminated '['; emit literally
    go (c : rest) = c : go rest

-- | Rebuild the cached closed view: the base DB forward-chained under the
-- state's domain 'axioms' (the derived closure). With no axioms this is
-- exactly @db st@, so un-axiomatised worlds are unaffected and pay nothing.
-- A contradiction (@⊥@) is surfaced as a queryable @contradiction@ fact over
-- the (still-consistent) base rather than crashing, so a world or
-- drama-manager can react to it. Internal: every helper that changes 'db' or
-- 'axioms' must end here.
reclose :: PraxState -> PraxState
reclose st = st { readView = case closure (axioms st) (db st) of
                               Right closed -> closed
                               Left _       -> insert "contradiction" (db st) }

-- | The only sanctioned way to change the fact base of a built state.
withDb :: (Db -> Db) -> PraxState -> PraxState
withDb f st = reclose st { db = f (db st) }

-- | Apply one delta to the base AND the cached view in lockstep — sound
-- exactly when 'relevantDelta' answered False (the delta commutes with
-- closure; v27 spec theorem). The only sanctioned 'readView' write outside
-- 'reclose'.
applyDirect :: (Db -> Db) -> PraxState -> PraxState
applyDirect f st = st { db = f (db st), readView = f (readView st) }

-- | The continuation tier: grow the base and continue the ALREADY-CLOSED
-- view with the one new fact. A contradiction (⊥) falls back to the full
-- reclose path, which reaches the same "contradiction" marker from scratch.
applyGrow :: String -> PraxState -> PraxState
applyGrow s st = case closureFrom (axioms st) (readView st) [s] of
  Right v -> st { db = insert s (db st), readView = v }
  Left _  -> withDb (insert s) st

-- | The only sanctioned way to change the axioms of a built state.
setAxioms :: [Axiom] -> PraxState -> PraxState
setAxioms axs st = retable (reclose st { axioms = axs })

-- | The only sanctioned way to change the desire vocabulary of a built state.
setDesires :: [Desire] -> PraxState -> PraxState
setDesires ds st = retable st { desires = ds }

-- | All actions the named actor can currently perform, across every
-- instantiated practice and every satisfying binding of each action. Conditions
-- (and practice instances) are evaluated against 'readView', so derived facts are
-- visible to preconditions. Instance unification and the inner condition query
-- both run on the precooked forms cached in 'cookedDefs' — the hot loop never
-- reparses an authored pattern.
possibleActions :: PraxState -> String -> [GroundedAction]
possibleActions st actor =
  [ GroundedAction
      { gaPracticeId = pid
      , gaInstanceId = ground (intercalate "." (cpInstanceNames cp)) inst
      , gaActionId   = caName ca
      , gaBindings   = binding
      , gaLabel      = renderText (caName ca) binding
      }
  | pid <- childKeys "practice" view
  , Just cp <- [Map.lookup pid (cookedDefs st)]
  , inst <- unifyNames (cpInstanceNames cp) view (Map.singleton "Actor" (VStr actor))
  , ca <- cpActions cp
  , binding <- queryCooked view (caConds ca) inst
  ]
  where view = readView st

-- | Apply every outcome of a grounded action, in order, through the cooked
-- engine.
performAction :: PraxState -> GroundedAction -> PraxState
performAction st ga =
  case Map.lookup (gaPracticeId ga) (cookedDefs st) of
    Nothing -> st
    Just cp ->
      case filter ((== gaActionId ga) . caName) (cpActions cp) of
        (ca : _) -> foldl' (\s o -> performCooked (groundCookedOutcome (gaBindings ga) o) s)
                           st (caOuts ca)
        [] -> st

-- | Substitute bound variables into an outcome's sentence(s)/args.
groundOutcome :: Outcome -> Bindings -> Outcome
groundOutcome (Insert s)          b = Insert (ground s b)
groundOutcome (Delete s)          b = Delete (ground s b)
groundOutcome (Call fn args)      b = Call fn (map (`ground` b) args)
groundOutcome (ForEach conds outs) b =
  ForEach (map (groundCondition b) conds) (map (`groundOutcome` b) outs)

-- | Apply a single, already-grounded outcome to the state — the public,
-- string-facing entry, cook-then-run: one engine, two doors.
performOutcome :: Outcome -> PraxState -> PraxState
performOutcome o = performCooked (cookOutcome o)

-- | Apply a single, already-grounded cooked outcome to the state.
-- Case-for-case with the string 'performOutcome': same classification order
-- (irrelevant → monotone-insert continuation → reclose), same
-- 'applyDirect'\/'applyGrow'\/'withDb' routing, same spawn-runs-inits
-- semantics, same 'ForEach' snapshot-then-apply behaviour — only the parsing
-- moves: classification, spawn detection, and insert/retract all run on
-- names already in hand.
performCooked :: CookedOutcome -> PraxState -> PraxState
performCooked (CDelete toks) st
  | relevantNames names shadows st = withDb (retractNames names) st
  | otherwise                      = applyDirect (retractNames names) st
  where
    names = map fst toks
    shadows = evictionShadowNames toks
performCooked (CInsert toks) st =
  let names = map fst toks
      shadows = evictionShadowNames toks
      st' | not (relevantNames names shadows st) = applyDirect (insertToks toks) st
          | monotoneToks toks st                  = applyGrowToks toks st
          | otherwise                              = withDb (insertToks toks) st
  in case spawnedInstanceNames names st of
       Just (def, cp, roleVals) ->
         let roleBindings = Map.fromList (zip (roles def) (map VStr roleVals))
         in foldl' (\s2 o -> performCooked (groundCookedOutcome roleBindings o) s2)
                   st' (cpInits cp)
       Nothing -> st'
performCooked (CCall fn args) st =
  case lookupFunction fn st of
    Nothing  -> st
    Just fdef ->
      let paramBindings = Map.fromList (zip (fnParams fdef) (map VStr args))
          firstMatch =
            [ (outs, res)
            | (conds, outs) <- lookupCookedFn fn st
            , res <- take 1 (queryCooked (db st) conds paramBindings) ]
      in case firstMatch of
           ((outs, res) : _) ->
             foldl' (\s o -> performCooked (groundCookedOutcome res o) s) st outs
           [] -> st
performCooked (CForEach conds outs) st =
  let bs = queryCooked (readView st) conds Map.empty   -- snapshot: all bindings up front
  in foldl' (\s b -> foldl' (\s2 o -> performCooked (groundCookedOutcome b o) s2) s outs) st bs

-- If inserting the sentence named by @names@ brings a not-yet-existing
-- practice instance into being, return its (string-side) definition, its
-- cooked form (for 'cpInits'), and the role values — so its @init@ can run
-- once.
spawnedInstanceNames :: [String] -> PraxState -> Maybe (Practice, CookedPractice, [String])
spawnedInstanceNames names st =
  case names of
    ("practice" : pid : rest) -> do
      def <- Map.lookup pid (practiceDefs st)
      cp  <- Map.lookup pid (cookedDefs st)
      let roleVals = take (length (roles def)) rest
          instanceNames = "practice" : pid : roleVals
      if length roleVals == length (roles def) && not (existedBefore instanceNames)
        then Just (def, cp, roleVals)
        else Nothing
    _ -> Nothing
  where
    -- The instance is newly spawned iff it did not exist before this insert.
    existedBefore ns = not (null (unifyNames ns (db st) Map.empty))

lookupFunction :: String -> PraxState -> Maybe Function
lookupFunction name st =
  case [ f | def <- Map.elems (practiceDefs st)
           , f <- functions def, fnName f == name ] of
    (f : _) -> Just f
    []      -> Nothing

-- | 'lookupFunction''s cooked mirror: the cases (cooked conditions/outcomes)
-- of the first practice whose 'cpFns' declares @fn@, in the same
-- 'Map.elems' order 'lookupFunction' scans.
lookupCookedFn :: String -> PraxState -> [([CookedCondition], [CookedOutcome])]
lookupCookedFn fn st =
  case [ cases | cp <- Map.elems (cookedDefs st), Just cases <- [Map.lookup fn (cpFns cp)] ] of
    (cs : _) -> cs
    []       -> []
