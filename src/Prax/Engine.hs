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
  , readView
  , possibleActions
  , performAction
  , performOutcome
  , groundOutcome
  ) where

import           Data.List (intercalate)
import qualified Data.Map.Strict as Map

import           Prax.Db
import           Prax.Query (query, groundCondition)
import           Prax.Types
import           Prax.Derive (closure)

-- | Register a practice and insert its static @dataFacts@ under
-- @practiceData.<id>.@.
definePractice :: Practice -> PraxState -> PraxState
definePractice p st = st
  { practiceDefs = Map.insert (practiceId p) p (practiceDefs st)
  , db = insertAll (map (prefix ++) (dataFacts p)) (db st)
  }
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

-- | The world as reads see it: the base DB forward-chained under the state's
-- domain 'axioms' (the derived closure). With no axioms this is exactly @db st@,
-- so un-axiomatised worlds are unaffected and pay nothing. A contradiction (@⊥@)
-- is surfaced as a queryable @contradiction@ fact over the (still-consistent) base
-- rather than crashing, so a world or drama-manager can react to it.
readView :: PraxState -> Db
readView st = case closure (axioms st) (db st) of
  Right closed           -> closed
  Left _                 -> insert "contradiction" (db st)

-- | All actions the named actor can currently perform, across every
-- instantiated practice and every satisfying binding of each action. Conditions
-- (and practice instances) are evaluated against 'readView', so derived facts are
-- visible to preconditions.
possibleActions :: PraxState -> String -> [GroundedAction]
possibleActions st actor =
  [ GroundedAction
      { gaPracticeId = pid
      , gaInstanceId = ground instanceQuery inst
      , gaActionId   = actionName act
      , gaBindings   = binding
      , gaLabel      = renderText (actionName act) binding
      }
  | pid <- childKeys "practice" view
  , Just def <- [Map.lookup pid (practiceDefs st)]
  , let instanceQuery = "practice." ++ pid ++ "." ++ intercalate "." (roles def)
  , inst <- unify instanceQuery view (Map.singleton "Actor" (VStr actor))
  , act  <- actions def
  , binding <- query view (actionConditions act) inst
  ]
  where view = readView st

-- | Apply every outcome of a grounded action, in order.
performAction :: PraxState -> GroundedAction -> PraxState
performAction st ga =
  case Map.lookup (gaPracticeId ga) (practiceDefs st) of
    Nothing  -> st
    Just def ->
      case filter ((== gaActionId ga) . actionName) (actions def) of
        (act : _) -> foldl' (\s o -> performOutcome (groundOutcome o (gaBindings ga)) s)
                            st (actionOutcomes act)
        [] -> st

-- | Substitute bound variables into an outcome's sentence(s)/args.
groundOutcome :: Outcome -> Bindings -> Outcome
groundOutcome (Insert s)          b = Insert (ground s b)
groundOutcome (Delete s)          b = Delete (ground s b)
groundOutcome (Call fn args)      b = Call fn (map (`ground` b) args)
groundOutcome (ForEach conds outs) b =
  ForEach (map (groundCondition b) conds) (map (`groundOutcome` b) outs)

-- | Apply a single, already-grounded outcome to the state.
performOutcome :: Outcome -> PraxState -> PraxState
performOutcome (Delete s) st = st { db = retract s (db st) }
performOutcome (Insert s) st =
  let st' = st { db = insert s (db st) }
  in case spawnedInstance s st of
       Just (def, roleVals) ->
         let roleBindings = Map.fromList (zip (roles def) (map VStr roleVals))
         in foldl' (\s2 o -> performOutcome (groundOutcome o roleBindings) s2)
                   st' (initOutcomes def)
       Nothing -> st'
performOutcome (Call fn args) st =
  case lookupFunction fn st of
    Nothing  -> st
    Just fdef ->
      let paramBindings = Map.fromList (zip (fnParams fdef) (map VStr args))
          firstMatch =
            [ (caseOutcomes c, res)
            | c <- fnCases fdef
            , res <- take 1 (query (db st) (caseConditions c) paramBindings) ]
      in case firstMatch of
           ((outs, res) : _) ->
             foldl' (\s o -> performOutcome (groundOutcome o res) s) st outs
           [] -> st
performOutcome (ForEach conds outs) st =
  let bs = query (readView st) conds Map.empty   -- snapshot: all bindings up front
  in foldl' (\s b -> foldl' (\s2 o -> performOutcome (groundOutcome o b) s2) s outs) st bs

-- If inserting @s@ brings a not-yet-existing practice instance into being,
-- return its definition and the role values (so its @init@ can run once).
spawnedInstance :: String -> PraxState -> Maybe (Practice, [String])
spawnedInstance s st =
  case pathNames s of
    ("practice" : pid : rest) -> do
      def <- Map.lookup pid (practiceDefs st)
      let roleVals = take (length (roles def)) rest
          instancePath = "practice." ++ pid ++ "." ++ intercalate "." roleVals
      if length roleVals == length (roles def) && not (existedBefore instancePath)
        then Just (def, roleVals)
        else Nothing
    _ -> Nothing
  where
    -- The instance is newly spawned iff it did not exist before this insert.
    existedBefore path = exists path (db st)

lookupFunction :: String -> PraxState -> Maybe Function
lookupFunction name st =
  case [ f | def <- Map.elems (practiceDefs st)
           , f <- functions def, fnName f == name ] of
    (f : _) -> Just f
    []      -> Nothing
