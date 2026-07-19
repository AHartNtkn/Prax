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
  , defineFunctions
  , renderText
  , withDb
  , setAxioms
  , setDesires
  , setCharacters
  , setSchedule
  , seedDie
  , registerEngineRules
  , possibleActions
  , performAction
  , performOutcome
  , performCooked
  , groundOutcome
  , relevantDelta
  , monotoneInsert
  , groundedDeltaAnchors
  , currentTurn
  , roundBoundary
  ) where

import           Data.List (intercalate, isPrefixOf, nub, (\\))
import           Data.Maybe (listToMaybe)
import qualified Data.Map.Strict as Map
import qualified Data.Set as Set
import           Data.Set (Set)
import           Text.Read (readMaybe)

import           Prax.Db
import           Prax.Query (queryCooked, groundCondition, groundNames, CookedCondition(..), cookCondition)
import           Prax.Types
import           Prax.Derive (Axiom, axiomFootprint, axiomNegPatterns, axiomHeadPatterns, monotoneAxioms, cookAxioms, runCooked)
import           Prax.Relevance (improvableDesires, livenessOf, mayUnifySyms, evictionShadowNames, bearingTemplates)
import           Prax.Cooked (cookOutcome, cookPractice, cookFunctions, cookScheduleRule, groundCookedOutcome)
import           Prax.Rng (rollStep, seedBounds)
import           Prax.Sym (Sym, intern, symIsVar, symName)

-- | Rebuild the derived vocabulary tables. Internal: every helper that changes
-- 'practiceDefs', 'axioms', 'desires', 'characters', 'schedule', or 'worldFns'
-- must end here. 'cookedRules' is recooked from the axiom list alone
-- ('cookAxioms', census-free) — whatever □-lifted rules a deontic world declared
-- ("Prax.Deontic.obligedClose") are already in 'axioms', so retable does not
-- decide any lift. The axiom-derived tables (footprint\/axiomHeads\/negFootprint\/
-- contMonotone) then read that 'cookedRules'.
retable :: PraxState -> PraxState
retable st0 =
  let st = st0
        { cookedDefs     = Map.map cookPractice (practiceDefs st0)
        , cookedSchedule = map cookScheduleRule (schedule st0)
        , cookedWants   = Map.fromList
            [ (charName c, [ map cookCondition (wantConditions w) | w <- charWants c ])
            | c <- characters st0 ]
        , cookedDesires = Map.fromList
            [ (desireName d, map cookCondition (wantConditions (desireWant d)))
            | d <- desires st0 ] }
      st' = st { cookedRules = cookAxioms (axioms st0) }
  in st'
     { improvables  = improvableDesires st'
     , caresAbout   = bearingTemplates st'
     , liveness     = livenessOf st'
     , footprint    = axiomFootprint (cookedRules st')
     , axiomHeads   = axiomHeadPatterns (cookedRules st')
                      ++ [[intern "contradiction"]]
     , negFootprint = axiomNegPatterns (cookedRules st')
     , contMonotone = monotoneAxioms (cookedRules st') }

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
definePractice p st
  | a : _ <- dupActions =
      error ("Prax.Engine.definePractice: practice " ++ show (practiceId p)
             ++ " declares two actions named " ++ show a
             ++ " -- action names are lookup keys (delta anchors, standing"
             ++ " intentions); rename one")
  | otherwise =
      retable (withDb (insertAll (map (prefix ++) (dataFacts p))) st)
        { practiceDefs = Map.insert (practiceId p) p (practiceDefs st) }
  where
    prefix = "practiceData." ++ practiceId p ++ "."
    names = map actionName (actions p)
    dupActions = [ n | (n, i) <- zip names [0 :: Int ..], n `elem` take i names ]

-- | Register several practices in order.
definePractices :: [Practice] -> PraxState -> PraxState
definePractices ps st = foldl' (flip definePractice) st ps

-- | Register the world's functions — the one home ('cookedFns'). 'Practice'
-- carries no functions since v47: practice-locality was fiction, resolution
-- was always global ('lookupCookedFn' searched every practice first-wins,
-- 'Call' sites name bare functions). Cooks each into the registry and retables
-- so the fn-pool analyses ('Prax.Relevance.producibleAtoms',
-- 'improvableDesires', 'livenessOf', 'bearingTemplates') see the vocabulary.
-- Loud on a duplicate 'fnName' — within this batch OR against the
-- already-registered set (v43's two per-practice collision arms collapse into
-- this one check: a Map cannot hold a duplicate silently, so the guard makes
-- the attempt loud). Order relative to 'definePractices' does not matter: both
-- setters persist their own field ('cookedFns'\/'practiceDefs') and 'retable'
-- reads both, so whichever runs last leaves every table coherent.
defineFunctions :: [Function] -> PraxState -> PraxState
defineFunctions fs st
  | (fn : _) <- clashes =
      error ("Prax.Engine.defineFunctions: function " ++ show fn
             ++ " is already registered -- Call resolution is by bare name"
             ++ " (lookupCookedFn); rename one")
  | otherwise =
      retable st { worldFns  = worldFns st ++ fs
                 , cookedFns = Map.union (cookedFns st) (cookFunctions fs) }
  where
    newNames = map fnName fs
    existing = Map.keys (cookedFns st)
    clashes  = (newNames \\ nub newNames) ++ filter (`elem` existing) newNames

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
-- (so this ~5,400-calls\/round path never re-cooks the axiom set — 'retable'
-- maintains it, gated). With no axioms this is exactly @db st@, so un-axiomatised
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

-- | The only sanctioned way to change the axioms of a built state. Retable
-- FIRST (it maintains 'cookedRules'), THEN reclose: reclose's 'runCooked' reads
-- the 'cookedRules' retable just set, so the cached view closes under exactly
-- the axiom set. Order-insensitive in the axiom set itself — 'cookAxioms' reads
-- only 'axioms', so no producer setup fact need precede this call.
setAxioms :: [Axiom] -> PraxState -> PraxState
setAxioms axs st = reclose (retable st { axioms = axs })

-- | The only sanctioned way to change the desire vocabulary of a built state.
setDesires :: [Desire] -> PraxState -> PraxState
setDesires ds st = retable st { desires = ds }

-- | The only sanctioned way to change the character roster of a built state.
setCharacters :: [Character] -> PraxState -> PraxState
setCharacters cs st = retable st { characters = cs }

-- | The AUTHORING door onto a world's engine schedule (spec v44): install
-- authored recurring rules. This is the one choke point for schedule-rule
-- authoring hygiene — on every clause of every rule the v40 splice check
-- ('authoredVarClash' @["Actor"]@): the @Prax@ namespace is reserved for engine
-- machinery and @Actor@ is reserved for movers (a schedule rule has no actor at
-- all). Everything else (single-segment names, positive periods, no duplicate
-- names, due seeding) is the shared 'addScheduleRules' core, which
-- 'registerEngineRules' shares.
setSchedule :: [ScheduleRule] -> PraxState -> PraxState
setSchedule rules st
  | (v : _) <- offenders =
      error ("Prax.Engine.setSchedule: a rule authors " ++ show v
             ++ " -- the Prax namespace is reserved for engine machinery, and Actor"
             ++ " is reserved for movers (a schedule rule has no actor at all)")
  | otherwise = addScheduleRules rules st
  where
    offenders = concat [ authoredVarClash ["Actor"] conds outs
                       | r <- rules, (conds, outs) <- srBody r ]

-- | Seed the drama die (spec v50): install the RNG stream's start position into
-- engine state ('rngSeed'). Loud on a seed outside the stream's domain
-- ('Prax.Rng.seedBounds' — @0@ and multiples of the modulus are fixed points, a
-- die that always rolls the same face). A world calls this once at build time
-- (the seed is an authored world parameter that selects the playthrough's
-- fate); an unseeded world's 'Roll' is a loud error, caught statically by the
-- 'Prax.TypeCheck.SeedlessDraw' check.
seedDie :: Integer -> PraxState -> PraxState
seedDie s st
  | s < lo || s > hi =
      error ("Prax.Engine.seedDie: seed " ++ show s ++ " lies outside the die's"
             ++ " domain [" ++ show lo ++ ", " ++ show hi ++ "] -- 0 and multiples"
             ++ " of the modulus are fixed points (a die that always rolls the"
             ++ " same face)")
  | otherwise = st { rngSeed = Just s }
  where (lo, hi) = seedBounds

-- | The COMPILER-LEVEL door onto the engine schedule (spec v46): register rules
-- that authoring code could not. The compiled "Prax.Script" story rule is
-- compiler-generated, not authored — its clauses carry compiler-owned
-- mechanics ('Prax.Script.compile' has already hygiene-checked the verbatim
-- author fragments it splices in) — so it registers here rather than through
-- 'setSchedule'. This door omits ONLY the v40 splice guard: its caller is
-- compiler-level code ('Prax.Script.compile'), squarely inside v45's
-- threat model (the family of reserved-namespace writes that only mechanism may
-- make), so it carries no authoring guard BY DESIGN. It is deliberately NOT
-- re-exported from any authoring-surface module. Every other schedule-rule
-- invariant still holds (single-segment names, positive periods, no duplicate
-- names ACROSS BOTH DOORS, dues seeded one period out) — the shared
-- 'addScheduleRules' core.
--
-- Beyond installing the rules, this door RECORDS which names it installed in
-- 'engineRuleNames' — the provenance the reserved-family scan ('Prax.TypeCheck')
-- consults so machinery may write reserved compiler families (spec v53). The
-- record update forces 'addScheduleRules' to WHNF, so its duplicate-name guard
-- still fires loudly BEFORE any name is recorded (a duplicate is never silently
-- exempted).
registerEngineRules :: [ScheduleRule] -> PraxState -> PraxState
registerEngineRules rules st =
  (addScheduleRules rules st)
    { engineRuleNames = engineRuleNames st ++ map srName rules }

-- | Install schedule rules onto the world, APPENDING to any already registered
-- (both doors write the one globally-keyed rule table): store the declarations,
-- cook their mirror ('retable'), and seed each rule's next-due one full period
-- out (the start-sated convention — @currentTurn + srPeriod@, uniform across
-- rules). Loud construction-time errors: single-segment rule names (a
-- multi-segment name would corrupt the by-name due keying), positive periods,
-- and no duplicate names — checked both within this batch AND against rules the
-- other door already registered (the dues map is keyed by name; a collision
-- would share one due key), naming both doors so the author knows where to look.
addScheduleRules :: [ScheduleRule] -> PraxState -> PraxState
addScheduleRules rules st
  | (r : _) <- filter ((/= 1) . length . pathNames . srName) rules =
      error ("Prax.Engine: schedule rule name must be a single segment: "
             ++ show (srName r))
  | (r : _) <- filter ((< 1) . srPeriod) rules =
      error ("Prax.Engine: schedule rule " ++ show (srName r)
             ++ " needs a positive period")
  | (n : _) <- clashes =
      error ("Prax.Engine: duplicate schedule-rule name " ++ show n
             ++ " would share one due key -- rule names are globally keyed across"
             ++ " both registration doors (Prax.Engine.setSchedule for authored"
             ++ " rules, Prax.Engine.registerEngineRules for compiler-level rules);"
             ++ " rename one")
  | otherwise =
      retable st { schedule = schedule st ++ rules
                 , scheduleDues = Map.union (scheduleDues st)
                     (Map.fromList
                        [ (srName r, currentTurn st + srPeriod r) | r <- rules ]) }
  where
    newNames = map srName rules
    existing = map srName (schedule st)
    clashes  = (newNames \\ nub newNames) ++ filter (`elem` existing) newNames

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
groundOutcome (InsertFor n s)     b = InsertFor n (ground s b)
groundOutcome (Call fn args)      b = Call fn (map (`ground` b) args)
groundOutcome (ForEach conds outs) b =
  ForEach (map (groundCondition b) conds) (map (`groundOutcome` b) outs)
groundOutcome (Roll num den conds outs) b =
  Roll num den (map (groundCondition b) conds) (map (`groundOutcome` b) outs)

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
performCooked (CDelete toks) st0
  | relevantNames names shadows st = withDb (retractNames names) st
  | otherwise                      = applyDirect (retractNames names) st
  where
    -- A subtree delete takes its descendants' pending timers (spec v44):
    -- eagerly purge every expiry entry AT OR UNDER the deleted path (by
    -- name-prefix), so no later retract can fire on a fact already gone.
    st = st0 { expiries = Map.filterWithKey (\k _ -> not (names `isPrefixOf` map fst k))
                                            (expiries st0) }
    names = map fst toks
    shadows = evictionShadowNames toks
performCooked (CInsert toks) st0 =
  let -- A bare insert of a path CANCELS any pending expiry on it (spec v44's
      -- supersession law: a permanent assertion never dies on a stale timer).
      st = st0 { expiries = Map.delete toks (expiries st0) }
      names = map fst toks
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
performCooked (CInsertFor n toks) st =
  -- Insert now (through the ordinary CInsert path, so relevance/closure tiers
  -- and spawning all apply, and any stale timer on the path is cancelled),
  -- then arm a fresh expiry n round boundaries out. Re-inserting the exact
  -- path with a lifetime therefore REFRESHES the due (spec v44).
  let st' = performCooked (CInsert toks) st
  in st' { expiries = Map.insert toks (currentTurn st' + n) (expiries st') }
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
performCooked (CRoll num den conds outs) st =
  case rngSeed st of
    Nothing -> error "Prax.Engine: Roll executed on an unseeded die \
                     \(a draw in a world that never called seedDie)"
    Just s  ->
      -- Advance the stream UNCONDITIONALLY (the frozen-die law: every draw
      -- spends one step, hit or miss), then roll on the advanced value — on a
      -- hit apply the body exactly as a CForEach (same snapshot semantics).
      let s'  = rollStep s
          st1 = st { rngSeed = Just s' }
      in if s' `mod` fromIntegral den < fromIntegral num
           then performCooked (CForEach conds outs) st1
           else st1

-- | The engine clock's current value — the single @turn@ child in the db
-- (spec v44). Loud error if absent or not a lone numeric value: the clock is
-- seeded in 'emptyState' and only ever advanced by 'roundBoundary', so its
-- absence is a construction bug, and silence is banned.
currentTurn :: PraxState -> Int
currentTurn st = case childKeys turnPath (db st) of
  [n] | Just i <- readMaybe n -> i
  ks -> error ("Prax.Engine.currentTurn: expected exactly one numeric " ++ show turnPath
               ++ " value in the db, found " ++ show ks)

-- | One round boundary (spec v44): advance the clock, fire due expiries
-- (existence-guarded: an entry whose exact fact was evicted since drops
-- silently — no retract, no recompute), then due schedule rules in
-- declaration order, re-arming each period boundaries from NOW. A pure
-- function of the state; the loop runs it at rotation wrap (Task 2).
--
-- Expiries fire BEFORE rules for a stated reason: a fact with lifetime n is
-- present rounds onset..onset+n-1 and GONE at the boundary — rules-first would
-- let a period-1 rule stamp a belief about a fact expiring that instant (a
-- ghost observation).
roundBoundary :: PraxState -> PraxState
roundBoundary st0 = foldl' fireRule stExpired dueRules
  where
    now  = currentTurn st0 + 1
    -- Advance the clock on the ordinary insert path so relevance/closure tiers
    -- apply (turn!now excludes turn!prev — the @!@ is in the seeded fact).
    st   = performCooked (CInsert (internTokens (turnPath ++ "!" ++ show now))) st0
    (due, keep) = Map.partition (<= now) (expiries st)
    stExpired = foldl' expireOne (st { expiries = keep }) (Map.keys due)
    expireOne s toks
      | exists (tokensToSentence toks) (db s) = performCooked (CDelete toks) s
      | otherwise                             = s          -- evicted since: silent drop
    dueRules = [ r | r <- cookedSchedule st0
                   , Map.findWithDefault maxBound (csrName r) (scheduleDues st0) <= now ]
    fireRule s r =
      (foldl' (\s' (conds, outs) -> performCooked (CForEach conds outs) s') s (csrBody r))
        { scheduleDues = Map.insert (csrName r) (now + periodOf r) (scheduleDues s) }
    -- The rule's authored period, from the string-side schedule by name; a
    -- cooked rule with no string-side declaration is a construction bug.
    periodOf r = case [ srPeriod sr | sr <- schedule st0, srName sr == csrName r ] of
      (p : _) -> p
      []      -> error ("Prax.Engine.roundBoundary: no schedule rule named "
                        ++ show (csrName r) ++ " to resolve its period")

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

-- | The function named @fn@'s params and cooked cases — a plain lookup in the
-- one registry ('cookedFns'). Since v47 functions have a single home (the
-- practice-fold-and-first-wins resolution is gone), so this is exactly
-- @Map.lookup fn (cookedFns st)@.
lookupCookedFn :: String -> PraxState -> Maybe ([String], [([CookedCondition], [CookedOutcome])])
lookupCookedFn fn st = Map.lookup fn (cookedFns st)

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
      CInsertFor _ toks -> go safe (CInsert toks)   -- the deferred retract is
                                                    -- environment: same anchors as CInsert
      CDelete toks ->
        let names = map fst toks
        in if unanchored names
             then Nothing
             else Just (names : evictionShadowNames toks)
      CForEach conds os -> go' (safe `Set.union` safeBinders conds) os
      CRoll _ _ conds os -> go' (safe `Set.union` safeBinders conds) os
        -- the body may fire (the roll may hit): its anchors count, exactly as
        -- a CForEach's, and the roll's guard binds like a CForEach's
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
