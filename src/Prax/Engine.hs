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
  , setCharacters
  , possibleActions
  , performAction
  , performOutcome
  , performCooked
  , groundOutcome
  , relevantDelta
  , monotoneInsert
  , groundedDeltaAnchors
  ) where

import           Data.List (intercalate)
import           Data.Maybe (listToMaybe)
import qualified Data.Map.Strict as Map
import qualified Data.Set as Set
import           Data.Set (Set)

import           Prax.Db
import           Prax.Query (queryCooked, groundCondition, groundNames, CookedCondition(..), cookCondition)
import           Prax.Types
import           Prax.Derive (Axiom, axiomFootprint, axiomNegPatterns, axiomHeadPatterns, monotoneAxioms, cookAxioms, runCooked)
import           Prax.Relevance (improvableDesires, livenessOf, mayUnifySyms, evictionShadowNames, bearingTemplates)
import           Prax.Cooked (cookOutcome, cookPractice, groundCookedOutcome)
import           Prax.Sym (Sym, intern, symIsVar, symName)

-- | Rebuild the derived vocabulary tables. Internal: every helper that
-- changes 'practiceDefs', 'desires', or 'characters' must end here.
-- ('axioms' is the one exception — the axiom-derived tables here READ
-- 'cookedRules', which 'setAxioms' maintains directly and sets before its own
-- 'reclose' call, so it is already current by the time any retable runs; see
-- there.)
retable :: PraxState -> PraxState
retable st0 =
  let st = st0
        { cookedDefs    = Map.map cookPractice (practiceDefs st0)
        , cookedWants   = Map.fromList
            [ (charName c, [ map cookCondition (wantConditions w) | w <- charWants c ])
            | c <- characters st0 ]
        , cookedDesires = Map.fromList
            [ (desireName d, map cookCondition (wantConditions (desireWant d)))
            | d <- desires st0 ] }
  in st
     { improvables  = improvableDesires st
     , caresAbout   = bearingTemplates st
     , liveness     = livenessOf st
     , footprint    = axiomFootprint (cookedRules st)
     , axiomHeads   = axiomHeadPatterns (cookedRules st)
                      ++ [[intern "contradiction"]]
     , negFootprint = axiomNegPatterns (cookedRules st)
     , contMonotone = monotoneAxioms (cookedRules st) }

-- | 'relevantDelta' generalized to operate on already-split, interned names:
-- the primary delta's names and its eviction shadows (each already a name
-- list). The hot cooked path ('performCooked') calls this directly — it
-- never reparses or re-interns a sentence to recover what it already has as
-- tokens.
relevantNames :: [Sym] -> [[Sym]] -> PraxState -> Bool
relevantNames names shadows st =
  any (\ns -> any (mayUnifySyms ns) (footprint st)) (names : shadows)

-- | Can this ground delta change what the axioms derive? Conservative:
-- False only when the sentence — and anything its exclusions evict —
-- may-unify nothing in the axioms' footprint (v27 spec theorem). False is
-- the licence for 'performOutcome' to skip 'reclose'.
relevantDelta :: String -> PraxState -> Bool
relevantDelta s = relevantNames (map fst toks) (evictionShadowNames toks)
  where toks = internTokens s

-- | 'monotoneInsert' generalized to operate on already-split, interned
-- tokens.
monotoneToks :: [(Sym, Maybe Char)] -> PraxState -> Bool
monotoneToks toks st =
  contMonotone st
    && all ((/= Just '!') . snd) toks
    && not (any (mayUnifySyms (map fst toks)) (negFootprint st))

-- | May this insert take the continuation tier: the world is
-- continuation-safe, the insert evicts nothing, and it can defeat nothing.
monotoneInsert :: String -> PraxState -> Bool
monotoneInsert s = monotoneToks (internTokens s)

-- | The continuation tier, natively on tokens: grow the base and continue the
-- ALREADY-CLOSED view with the one new fact via 'runCooked' and the state's
-- precompiled 'cookedRules' — no string is ever rebuilt from @toks@. A
-- contradiction (⊥) falls back to the full 'reclose' path, which reaches the
-- same "contradiction" marker from scratch.
applyGrowToks :: [(Sym, Maybe Char)] -> PraxState -> PraxState
applyGrowToks toks st =
  case runCooked (cookedRules st) (insertToks toks (readView st)) (insertToks toks emptyDb) of
    Right v -> st { db = insertToks toks (db st), readView = v }
    Left _  -> withDb (insertToks toks) st

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
          maybe ('[' : name ++ "]") valToString (Map.lookup (intern name) b) ++ go after
        _ -> '[' : go rest      -- unterminated '['; emit literally
    go (c : rest) = c : go rest

-- | Rebuild the cached closed view: the base DB forward-chained under the
-- state's domain 'axioms', via 'runCooked' and the precompiled 'cookedRules'
-- (so this ~5,400-calls\/round path never re-cooks the axiom set — see
-- 'setAxioms'). With no axioms this is exactly @db st@, so un-axiomatised
-- worlds are unaffected and pay nothing. A contradiction (@⊥@) is surfaced as
-- a queryable @contradiction@ fact over the (still-consistent) base rather
-- than crashing, so a world or drama-manager can react to it. Internal:
-- every helper that changes 'db' or 'axioms' must end here.
reclose :: PraxState -> PraxState
reclose st = st { readView = case runCooked (cookedRules st) (db st) (db st) of
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

-- | The only sanctioned way to change the axioms of a built state.
-- 'cookedRules' is set in the SAME record update as 'axioms', before
-- 'reclose' runs, so 'reclose'\'s 'runCooked' call sees the new rules
-- (setting it in 'retable' instead would be too late: 'retable' runs AFTER
-- 'reclose' here).
setAxioms :: [Axiom] -> PraxState -> PraxState
setAxioms axs st = retable (reclose st { axioms = axs, cookedRules = cookAxioms axs })

-- | The only sanctioned way to change the desire vocabulary of a built state.
setDesires :: [Desire] -> PraxState -> PraxState
setDesires ds st = retable st { desires = ds }

-- | The only sanctioned way to change the character roster of a built state.
setCharacters :: [Character] -> PraxState -> PraxState
setCharacters cs st = retable st { characters = cs }

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
      , gaInstanceId = intercalate "." (map symName (groundNames inst (cpInstanceNames cp)))
      , gaActionId   = caName ca
      , gaBindings   = binding
      , gaLabel      = renderText (caName ca) binding
      }
  | pid <- childKeys "practice" view
  , Just cp <- [Map.lookup pid (cookedDefs st)]
  , inst <- unifySyms (cpInstanceNames cp) view (Map.singleton (intern "Actor") (VSym (intern actor)))
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
-- 'applyDirect'\/'applyGrowToks'\/'withDb' routing, same spawn-runs-inits
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
         let roleBindings = Map.fromList (zip (map intern (roles def)) (map VSym roleVals))
         in foldl' (\s2 o -> performCooked (groundCookedOutcome roleBindings o) s2)
                   st' (cpInits cp)
       Nothing -> st'
performCooked (CCall fn args) st =
  case lookupCookedFn fn st of
    Nothing -> st
    Just (params, cases) ->
      let paramBindings = Map.fromList (zip (map intern params) (map VSym args))
          firstMatch =
            [ (outs, res)
            | (conds, outs) <- cases
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
-- cooked form (for 'cpInits'), and the role values (still 'Sym's — the
-- caller builds 'Bindings' directly from them, no render/re-intern round
-- trip) — so its @init@ can run once.
spawnedInstanceNames :: [Sym] -> PraxState -> Maybe (Practice, CookedPractice, [Sym])
spawnedInstanceNames names st =
  case names of
    (p : pidSym : rest) | p == intern "practice" ->
      let pid = symName pidSym in do
        def <- Map.lookup pid (practiceDefs st)
        cp  <- Map.lookup pid (cookedDefs st)
        let roleVals = take (length (roles def)) rest
            instanceNames = p : pidSym : roleVals
        if length roleVals == length (roles def) && not (existedBefore instanceNames)
          then Just (def, cp, roleVals)
          else Nothing
    _ -> Nothing
  where
    -- The instance is newly spawned iff it did not exist before this insert.
    existedBefore ns = not (null (unifySyms ns (db st) Map.empty))

-- | The function named @fn@'s params and cooked cases: the first practice
-- (in 'Map.elems' order) whose 'cpFns' declares it — 'cpFns' itself is
-- first-wins within a practice ('Prax.Cooked.cookPractice'), so this single
-- lookup gives the correct two-level first-match resolution (first practice,
-- then first same-named function within it) on its own, with no string-side
-- fallback needed for 'fnParams'.
lookupCookedFn :: String -> PraxState -> Maybe ([String], [([CookedCondition], [CookedOutcome])])
lookupCookedFn fn st =
  case [ entry | cp <- Map.elems (cookedDefs st), Just entry <- [Map.lookup fn (cpFns cp)] ] of
    (e : _) -> Just e
    []      -> Nothing

-- | The insert\/delete anchor families one grounded action's outcomes can
-- touch — 'performAction''s effects, bounded statically per call by walking
-- the same cooked outcomes 'performAction' itself executes. @Nothing@ when
-- the effects cannot be bounded: an unresolvable 'CCall'; an insert whose
-- first segment IS the literal @practice@; or an insert whose first segment
-- is a variable that is not a safe ForEach binder ('safeBinders') — such a
-- head could ground to @practice@ and spawn an instance
-- ('spawnedInstanceNames'), running that practice's 'cpInits', arbitrary
-- further outcomes this walk does not model. Paths with no literal segment
-- at all are likewise opaque — they carry no anchor evidence for
-- 'Prax.Relevance.mayUnifySyms'. The caller (the planner's prediction reuse)
-- treats @Nothing@ as opaque: no reuse at or below the node. Conservative by
-- construction: 'CForEach' bodies are included whether or not their guards
-- would fire, a safe binder head kept as a 'mayUnifySyms' wildcard anchor;
-- 'CCall' includes every case of the resolved function, cycle-guarded like
-- 'Prax.Relevance''s string-side atom walk.
groundedDeltaAnchors :: PraxState -> GroundedAction -> Maybe [[Sym]]
groundedDeltaAnchors st ga = do
  cp <- Map.lookup (gaPracticeId ga) (cookedDefs st)
  ca <- listToMaybe [ a | a <- cpActions cp, caName a == gaActionId ga ]
  outcomeDeltaAnchors st [] (map (groundCookedOutcome (gaBindings ga)) (caOuts ca))

outcomeDeltaAnchors :: PraxState -> [String] -> [CookedOutcome] -> Maybe [[Sym]]
outcomeDeltaAnchors st visited = go' Set.empty
  where
    go' safe = fmap concat . traverse (go safe)
    go safe o = case o of
      CInsert toks ->
        let names = map fst toks
        in if mightSpawn safe names || unanchored names
             then Nothing
             else Just (names : evictionShadowNames toks)
      CDelete toks ->
        let names = map fst toks
        in if unanchored names
             then Nothing
             else Just (names : evictionShadowNames toks)
      CForEach conds os -> go' (safe `Set.union` safeBinders conds) os
      CCall fn args
        | fn `elem` visited -> Just []
        | otherwise -> case lookupCookedFn fn st of
            Nothing -> Nothing
            Just (params, cases) ->
              let b = Map.fromList (zip (map intern params) (map VSym args))
              in fmap concat (traverse
                   (\(_, os) -> outcomeDeltaAnchors st (fn : visited)
                                  (map (groundCookedOutcome b) os))
                   cases)
    mightSpawn safe (n : _)
      | symIsVar n = not (n `Set.member` safe)
      | otherwise  = n == intern "practice"
    mightSpawn _ [] = False
    -- A path with no literal segment carries no anchor evidence at all —
    -- 'mayUnifySyms' would clear it against every read pattern (the
    -- anchored-literal rule discards evidence-free overlaps), so bounding it
    -- would license unsound reuse. Opaque instead. Well-formed facts always
    -- carry a literal predicate segment (the authored-world invariant), so
    -- this arm is unreachable in shipped worlds.
    unanchored = all symIsVar

-- | The ForEach binders that provably cannot take the value @practice@:
-- variables bound at a NON-FIRST position of a top-level positive 'CMatch'
-- guard and never occurring at the first position of any such guard. Spends
-- the authored-world structural invariant (the family "Prax.Relevance"'s
-- header states for predicate literals, extended to the registry root): the
-- literal @practice@ roots practice-registry paths and is never an entity,
-- place, value, or id name — so a value read out of a fact's INTERIOR can
-- never be @practice@, and an insert headed by such a binder can never reach
-- 'spawnedInstanceNames'. Deliberately narrow: 'CExists'\/'CAbsent'\/'CNot'
-- do not bind outward, 'COr' branches may leave the binder unbound, subquery
-- variables carry sets, and a FIRST-position variable really can unify
-- @practice@ against the registry — none of those yield safe binders, and
-- 'CCall' resets the safe set (call-scoped parameters are not the mover's
-- bindings). Uncertainty stays opaque.
safeBinders :: [CookedCondition] -> Set Sym
safeBinders conds = Set.difference boundDeep firstPos
  where
    pats = [ p | CMatch p <- conds ]
    boundDeep = Set.fromList [ v | p <- pats, v <- drop 1 p, symIsVar v ]
    firstPos  = Set.fromList [ v | (v : _) <- pats, symIsVar v ]
