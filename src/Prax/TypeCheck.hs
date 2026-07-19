-- | A static well-formedness checker for a world (LEDGER #8, first cut).
--
-- Versu ships a type checker to "find errors early" (Evans & Short 2014, §VII-E):
-- it is strongly but /implicitly/ typed — the author declares nothing and the
-- system complains when a consistent assignment cannot be found. This module is
-- the __sound, declaration-free subset__ of that: static checks over every
-- sentence a world authors ('practiceDefs' + 'axioms' + facts), each of which
-- flags only unambiguous bugs (no false positives, so the report is trustworthy).
-- It adds no logic engine — just a pass over the existing sentence structure.
-- One 'TypeError' constructor per check, seven in all — the four below plus
-- declared-sort conflicts ('SortConflict', the opt-in sort pass), an authored
-- touch of an engine-owned fact family ('ReservedFamily', spec v45 — @turn@
-- and @contradiction@), and an unseeded die ('SeedlessDraw').
--
--   * __Unbound variables__ — a variable used in an outcome (or an axiom head) that
--     no precondition, role, or @Actor@ can bind is ungroundable: it silently
--     inserts a literal @\"X\"@ or a no-op. (The most common real bug.)
--   * __Exclusion-cardinality clashes__ — a relation used both single-valued (@!@)
--     and multi-valued (@.@); the paper's exclusion-information check, done
--     statically.
--   * __Dangling references__ — a @Call@ to an undefined function, or a spawn of an
--     undefined practice.
--   * __Dead conditions__ (v41 cooked surface, first new check on it) — a positive
--     conjunct that may-unifies nothing the world can ever produce, so its site
--     (an action, function case, @ForEach@ guard, or want) can never fire.
--
-- Full ML-style /sort/ inference (agent-vs-gender) needs a declaration layer and
-- is the documented next step; this checker does not attempt it.
module Prax.TypeCheck
  ( TypeError(..)
  , typeCheck
  ) where

import           Data.List (intercalate, isPrefixOf, nub)
import           Data.Maybe (isJust, isNothing)
import qualified Data.Map.Strict as Map

import           Prax.Db (isVariable, pathNames, tokens, dbToLabeledSentences, dbToSentences)
import           Prax.Query (Condition (..), CookedCondition (..), condSents)
import           Prax.Relevance (mayUnifySyms, producibleAtoms, cookedFnPool, cookedOutcomeAtoms)
import           Prax.Deontic (obligedHead, obligedLift)
import           Prax.Sym (intern, symName, symIsVar)
import           Prax.Types
import           Prax.Derive (Axiom (..))

-- | A well-formedness problem found in a world.
data TypeError
  = UnboundVar { teWhere :: String, teVar :: String, teSentence :: String }
    -- ^ @teVar@ in @teSentence@ (at @teWhere@) is bound by nothing.
  | CardinalityClash { teSlot :: String }
    -- ^ the (variable-normalized) slot @teSlot@ is used both @!@ and @.@.
  | UndefinedRef { teWhere :: String, teName :: String }
    -- ^ a @Call@/spawn (at @teWhere@) names something never defined.
  | SortConflict { teWhere :: String, teDetail :: String }
    -- ^ a position/variable (@teWhere@) is inferred to have two sorts (@teDetail@).
  | ReservedFamily { teFamily :: String, teWhere :: String, teSentence :: String }
    -- ^ an authored definition touches the engine-owned family @teFamily@
    -- (spec v45): its facts are machinery — written (and for some families
    -- read) only by compiled mechanism, whose accesses carry Prax-namespaced
    -- value variables no author can write (the v40 namespace ban makes the
    -- shape unforgeable).
  | SeedlessDraw
    -- ^ a world's authored outcomes contain a 'Roll' (a compiled
    -- 'Prax.Rng.draw') reachable anywhere — practices or schedule, nested
    -- under 'ForEach' — but 'rngSeed' is 'Nothing': executing the draw would
    -- be a loud unseeded-die error at runtime, so the die must be seeded
    -- ('Prax.Engine.seedDie').
  | DeadCondition { teWhere :: String, teSentence :: String }
    -- ^ the positive pattern @teSentence@ (at @teWhere@) may-unifies nothing
    -- the world can ever contain: the site can never fire.
  | DeonticUnclosed { teSentence :: String }
    -- ^ the world can invoke an obligation (it can produce an @obliged.*@ fact)
    -- yet its axiom list omits the □-lifted twin of the liftable rule whose
    -- first head is @teSentence@: DEON property 1 would silently fail. Declare
    -- the closure with 'Prax.Deontic.obligedClose' (spec v51).
  deriving (Eq, Show)

-- | Every well-formedness problem in a world (empty ⇒ the world is well-formed).
typeCheck :: PraxState -> [TypeError]
typeCheck st =
     concatMap unboundInPractice ps
  ++ concatMap unboundInFunction (worldFns st)
  ++ concatMap unboundInAxiom (axioms st)
  ++ cardinalityErrors (assertedSentences st)
  ++ refErrors st
  ++ sortErrors st
  ++ reservedFamilyErrors st
  ++ seedlessDrawErrors st
  ++ deadConditionErrors st
  ++ deonticUnclosedErrors st
  where ps = Map.elems (practiceDefs st)

-- Variables mentioned in a sentence / condition -------------------------------

varsOf :: String -> [String]
varsOf = filter isVariable . pathNames

-- Every variable a condition /mentions/ (an over-approximation of what it can
-- bind — sound for the unbound check: a var mentioned by no condition truly
-- cannot be bound).
condVars :: Condition -> [String]
condVars c = case c of
  Match s          -> varsOf s
  Not s            -> varsOf s
  Eq a b           -> vs [a, b]
  Neq a b          -> vs [a, b]
  Cmp _ a b        -> vs [a, b]
  Calc r _ a b     -> vs [r, a, b]
  Count r s        -> vs [r, s]
  Subquery set f w -> vs (set : f) ++ concatMap condVars w
  Or clauses       -> concatMap (concatMap condVars) clauses
  Absent cs        -> concatMap condVars cs
  Exists cs        -> concatMap condVars cs
  where vs = filter isVariable

-- The variables an outcome /uses/, paired with the text they appear in.
outcomeUses :: Outcome -> [(String, String)]
outcomeUses (Insert s)           = [ (v, s) | v <- varsOf s ]
outcomeUses (Delete s)           = [ (v, s) | v <- varsOf s ]
outcomeUses (InsertFor _ s)      = [ (v, s) | v <- varsOf s ]
outcomeUses (Call fn args)       = [ (v, fn) | a <- args, v <- varsOf a ]
outcomeUses (ForEach conds outs) =
  [ (v, s) | (v, s) <- concatMap outcomeUses outs
           , v `notElem` concatMap condVars conds ]
outcomeUses (Roll _ _ conds outs) =
  [ (v, s) | (v, s) <- concatMap outcomeUses outs
           , v `notElem` concatMap condVars conds ]

-- Check 1: unbound variables --------------------------------------------------

unboundInOutcomes :: String -> [String] -> [Outcome] -> [TypeError]
unboundInOutcomes loc bound outs =
  [ UnboundVar loc v s
  | o <- outs, (v, s) <- outcomeUses o, v `notElem` bound ]

unboundInPractice :: Practice -> [TypeError]
unboundInPractice p =
     unboundInOutcomes (practiceId p ++ " (init)") (roles p) (initOutcomes p)
  ++ concatMap action' (actions p)
  where
    action' a =
      unboundInOutcomes (practiceId p ++ " / " ++ actionName a)
        ("Actor" : roles p ++ concatMap condVars (actionConditions a))
        (actionOutcomes a)

-- Each function case's outcomes may only use the function's params and the
-- case's own condition-bound variables (spec v47: functions are registry-level,
-- so the site label drops the practice prefix).
unboundInFunction :: Function -> [TypeError]
unboundInFunction f =
  concatMap
    (\c -> unboundInOutcomes ("fn " ++ fnName f)
             (fnParams f ++ concatMap condVars (caseConditions c))
             (caseOutcomes c))
    (fnCases f)

unboundInAxiom :: Axiom -> [TypeError]
unboundInAxiom ax =
  [ UnboundVar "axiom" v h | h <- axiomThen ax, v <- varsOf h, v `notElem` bound ]
  where bound = concatMap condVars (axiomWhen ax)

-- Check 2: exclusion-cardinality consistency ----------------------------------

-- Each edge of a sentence, keyed by the variable-normalized path to its parent,
-- paired with whether that edge is exclusive (@!@).
edgesOf :: String -> [(String, Bool)]
edgesOf s =
  [ (intercalate "." (take (i + 1) names), op == Just '!')
  | (i, (_, op)) <- zip [0 :: Int ..] ts, isJust op ]
  where
    ts    = tokens s
    names = map (\(n, _) -> if isVariable n then "_" else n) ts

cardinalityErrors :: [String] -> [TypeError]
cardinalityErrors sentences =
  [ CardinalityClash slot
  | (slot, labels) <- Map.toList byslot, length (nub labels) > 1 ]
  where
    byslot = Map.fromListWith (++)
               [ (slot, [excl]) | s <- sentences, (slot, excl) <- edgesOf s ]

-- Check 3: dangling references ------------------------------------------------

refErrors :: PraxState -> [TypeError]
refErrors st = concatMap practiceRefs ps ++ concatMap functionRefs (worldFns st)
  where
    ps          = Map.elems (practiceDefs st)
    definedFns  = map fnName (worldFns st)
    definedPrac = Map.keys (practiceDefs st)
    practiceRefs p = concatMap (outcomeRef (practiceId p)) (allOutcomes p)
    allOutcomes p = initOutcomes p ++ concatMap actionOutcomes (actions p)
    functionRefs f =
      concatMap (outcomeRef ("fn " ++ fnName f)) [ o | c <- fnCases f, o <- caseOutcomes c ]
    outcomeRef loc (Call fn _)
      | fn `notElem` definedFns = [ UndefinedRef loc fn ]
    outcomeRef loc (Insert s)
      | ("practice" : pid : _) <- pathNames s
      , pid `notElem` definedPrac = [ UndefinedRef loc ("practice." ++ pid) ]
    outcomeRef loc (InsertFor _ s)
      | ("practice" : pid : _) <- pathNames s
      , pid `notElem` definedPrac = [ UndefinedRef loc ("practice." ++ pid) ]
    outcomeRef loc (ForEach _ subs) = concatMap (outcomeRef loc) subs
    outcomeRef loc (Roll _ _ _ subs) = concatMap (outcomeRef loc) subs
    outcomeRef _ _ = []

-- The world's __asserting__ sentences, for the cardinality pass. Only inserts,
-- static data facts, axiom heads, and live facts /assert/ a slot's cardinality;
-- a query condition's @!@/@.@ is mere path syntax (unify ignores it), so
-- conditions are deliberately excluded — otherwise a legitimate @Match "ending.E"@
-- would falsely clash with an @Insert "ending!betrayal"@.
assertedSentences :: PraxState -> [String]
assertedSentences st =
     [ s | p <- ps
         , s <- dataFacts p
                ++ inserts (initOutcomes p)
                ++ concatMap (inserts . actionOutcomes) (actions p) ]
  ++ [ s | f <- worldFns st, s <- concatMap (inserts . caseOutcomes) (fnCases f) ]
  ++ [ h | ax <- axioms st, h <- axiomThen ax ]
  ++ dbToLabeledSentences (db st)
  where
    ps = Map.elems (practiceDefs st)
    inserts os = [ s | Insert s <- os ] ++ [ s | InsertFor _ s <- os ]
              ++ concat [ inserts subs | ForEach _ subs <- os ]
              ++ concat [ inserts subs | Roll _ _ _ subs <- os ]

-- Check 4: ML-style sort inference (only when sorts are declared) -------------

-- Infer a sort for every predicate position and variable by unification, and
-- report positions/variables forced to two sorts. Sorts are declared by
-- membership; a segment that is a variable or a declared member is an
-- /object\/value/ (normalized to @_@ in a position key), everything else is a
-- /structural field-name/ (kept), which is how the checker tells them apart.
sortErrors :: PraxState -> [TypeError]
sortErrors st
  | null (sorts st) = []
  | otherwise       = dupErrors ++ conflictErrors
  where
    memberPairs = [ (c, s) | (s, cs) <- sorts st, c <- cs ]
    memberSort  = Map.fromList memberPairs
    isMember c  = c `Map.member` memberSort

    -- a constant declared in two sorts is itself a conflict
    dupErrors =
      [ SortConflict c ("declared in " ++ intercalate ", " (nub ss))
      | (c, ss) <- Map.toList byConst, length (nub ss) > 1 ]
      where byConst = Map.fromListWith (++) [ (c, [s]) | (c, s) <- memberPairs ]

    -- object/value occurrences of a sentence: (segment, position key)
    occs sentence =
      [ (seg, intercalate "." (map norm (take (i + 1) segs)))
      | let segs = pathNames sentence
      , (i, seg) <- zip [0 :: Int ..] segs, isVariable seg || isMember seg ]
      where norm seg = if isVariable seg || isMember seg then "_" else seg

    scoped = [ (sc, s) | (sc, ss) <- sentencesByScope st, s <- ss ]

    -- per scope, the positions each variable occupies (drives the unions)
    varPositions =
      Map.elems $ Map.fromListWith (++)
        [ ((sc, seg), [key]) | (sc, s) <- scoped, (seg, key) <- occs s, isVariable seg ]

    -- positions labelled by a member constant landing there (global)
    posLabels =
      Map.fromListWith (++)
        [ (key, [memberSort Map.! seg]) | (_, s) <- scoped, (seg, key) <- occs s, isMember seg ]

    -- union positions that a variable connects, then group labels by class
    uf     = foldl unionAll Map.empty varPositions
    byRep  = Map.fromListWith (++) [ (find uf key, labs) | (key, labs) <- Map.toList posLabels ]
    conflictErrors =
      [ SortConflict (readable rep) (intercalate " vs " ss)
      | (rep, labs) <- Map.toList byRep, let ss = nub labs, length ss > 1 ]
    readable key = case filter (/= "_") (pathNames key) of
      [] -> key
      ns -> intercalate "." ns

-- Check 5 (generalized, v45): engine-owned fact families ---------------------

-- Only compiled mechanism may write these families (spec v45): @turn@ and
-- @contradiction@ have NO legitimate authored writer at all — reads stay free
-- (turn is the documented time interface; a contradiction read cannot
-- corrupt). Engine-owned families that are machinery in BOTH polarities do
-- not appear here: they are unrepresentable outright (the die's stream, v50)
-- or rejected by their own compiler ('Prax.Script.compile' rejects an
-- authored @scenePatience@ touch — a literal-tailed compiler fact this
-- table's write scan cannot distinguish from the compiler's own insert).
reservedFamilies :: [String]
reservedFamilies = [ turnPath, "contradiction" ]

reservedFamilyErrors :: PraxState -> [TypeError]
reservedFamilyErrors st =
     [ ReservedFamily fam loc s
     | (loc, os) <- writeSites st, o <- os, s <- writesOf o
     , Just fam <- [familyOf s] ]
  ++ [ ReservedFamily fam "axiom" h
     | ax <- axioms st, h <- axiomThen ax
     , Just fam <- [familyOf h] ]
  where
    familyOf s = case pathNames s of
      (h : _) | h `elem` reservedFamilies -> Just h
      _                                   -> Nothing
    writesOf o = case o of
      Insert s      -> [s]
      InsertFor _ s -> [s]
      Delete s      -> [s]                    -- a delete is a write
      ForEach _ os  -> concatMap writesOf os
      Roll _ _ _ os -> concatMap writesOf os
      Call _ _      -> []

-- The authored write sites, with labels: practice action/init/function-case
-- outcomes and every schedule rule body's outcomes.
writeSites :: PraxState -> [(String, [Outcome])]
writeSites st =
     [ (practiceId p ++ " (init)", initOutcomes p) | p <- ps ]
  ++ [ (practiceId p ++ " / " ++ actionName a, actionOutcomes a) | p <- ps, a <- actions p ]
  ++ [ ("fn " ++ fnName f, caseOutcomes c)
     | f <- worldFns st, c <- fnCases f ]
  ++ [ ("schedule " ++ srName r, outs) | r <- schedule st, (_, outs) <- srBody r ]
  where ps = Map.elems (practiceDefs st)

-- Check 6: draws need a seeded die ------------------------------------------

-- A world whose authored outcomes contain a 'Roll' (a compiled
-- 'Prax.Rng.draw') anywhere — a practice's init/action outcomes, a function
-- case, a schedule rule body, or nested under a @ForEach@ — while 'rngSeed' is
-- 'Nothing': executing that draw is a loud unseeded-die error at runtime, so
-- the die must be seeded ('Prax.Engine.seedDie'). Structural — it detects the
-- draw by its constructor, not by sniffing a seed-family guard.
seedlessDrawErrors :: PraxState -> [TypeError]
seedlessDrawErrors st =
  [ SeedlessDraw
  | isNothing (rngSeed st), any (any hasRoll) allOutcomeLists ]
  where
    ps = Map.elems (practiceDefs st)
    allOutcomeLists =
      [ initOutcomes p | p <- ps ]
      ++ [ actionOutcomes a | p <- ps, a <- actions p ]
      ++ [ caseOutcomes c | f <- worldFns st, c <- fnCases f ]
      ++ [ outs | r <- schedule st, (_, outs) <- srBody r ]
    hasRoll o = case o of
      Roll{}         -> True
      ForEach _ outs -> any hasRoll outs
      _              -> False

-- Check 7: dead conditions ----------------------------------------------------

-- A positive Match conjunct whose pattern can never match anything the world
-- can ever contain ('Prax.Relevance.producibleAtoms') makes its site
-- unreachable: the action, function case, ForEach effect, or want silently
-- never fires — the unambiguous-bug class. Scanned sites are affordances and
-- motives only; axiom bodies are deliberately OUT of scope (a rule library
-- included wholesale routinely leaves some rules inert — feud's kinAxioms,
-- documented deliberate in the fixture — and an unfireable rule is
-- harmless). Flagged positions are conjunctive positives only: top level and
-- inside Exists (a dead positive there kills the Exists, killing the site).
-- NOT flagged: negations (vacuously true is plausible defensive authoring),
-- Or clauses (a dead clause doesn't kill the Or), Subquery interiors (an
-- always-empty set can be a Count comparison's intended meaning), and
-- unanchored patterns (every segment a variable: matches everything —
-- 'mayUnifySyms' would discard the evidence-free overlap, so without this
-- exemption the lint would flag exactly the patterns that can never be
-- dead). A wild world (unresolvable Call) silences the lint entirely.
deadConditionErrors :: PraxState -> [TypeError]
deadConditionErrors st = case producibleAtoms st of
  Nothing   -> []
  Just pool ->
    [ DeadCondition loc (intercalate "." (map symName p))
    | (loc, conds) <- lintSites st
    , p <- concatMap positives conds
    , not (all symIsVar p)
    , not (any (mayUnifySyms p) pool) ]
  where
    positives c = case c of
      CMatch p   -> [p]
      CExists cs -> concatMap positives cs
      _          -> []

-- The affordance/motive sites the lint scans, with author-legible labels:
-- action conditions, ForEach guards (recursively — action outcomes, init
-- outcomes, function-case outcomes), function-case conditions, desires,
-- character wants.
lintSites :: PraxState -> [(String, [CookedCondition])]
lintSites st =
     [ (pid ++ " / " ++ caName a, caConds a)
     | (pid, cp) <- defs, a <- cpActions cp ]
  ++ [ (pid ++ " / " ++ caName a ++ " (effect guard)", gs)
     | (pid, cp) <- defs, a <- cpActions cp, gs <- forEachGuards (caOuts a) ]
  ++ [ (pid ++ " (init guard)", gs)
     | (pid, cp) <- defs, gs <- forEachGuards (cpInits cp) ]
  ++ [ ("fn " ++ fn, cs)
     | (fn, (_, cases)) <- Map.toList (cookedFns st), (cs, _) <- cases ]
  ++ [ ("fn " ++ fn ++ " (effect guard)", gs)
     | (fn, (_, cases)) <- Map.toList (cookedFns st)
     , (_, os) <- cases, gs <- forEachGuards os ]
  ++ [ ("schedule " ++ csrName r, cs)
     | r <- cookedSchedule st, (cs, _) <- csrBody r ]
  ++ [ ("schedule " ++ csrName r ++ " (effect guard)", gs)
     | r <- cookedSchedule st, (_, os) <- csrBody r, gs <- forEachGuards os ]
  ++ [ ("desire " ++ n, cs) | (n, cs) <- Map.toList (cookedDesires st) ]
  ++ [ ("want of " ++ n, cs)
     | (n, css) <- Map.toList (cookedWants st), cs <- css ]
  where defs = Map.toList (cookedDefs st)

forEachGuards :: [CookedOutcome] -> [[CookedCondition]]
forEachGuards outs = concat $
     [ conds : forEachGuards os | CForEach conds os <- outs ]
  ++ [ conds : forEachGuards os | CRoll _ _ conds os <- outs ]

-- Check 8: a world that can invoke obligation declares its closure ------------

-- A world that CAN produce an @obliged.*@ fact, yet whose axiom list contains a
-- liftable rule whose □-lifted twin ('Prax.Deontic.obligedLift') is absent,
-- would silently fail DEON property 1 (an obligation would not close over the
-- consequences of its content). Flag each such rule, naming its first head; the
-- fix is to declare the closure with 'Prax.Deontic.obligedClose' (spec v51 —
-- the check the v48 producer census became once lifting left the engine). The
-- lint runs on the FINISHED world, so no setter-order sensitivity remains.
deonticUnclosedErrors :: PraxState -> [TypeError]
deonticUnclosedErrors st =
  [ DeonticUnclosed (renderHead a)
  | deonticInvokable st
  , a <- axioms st, Just twin <- [obligedLift a]
  , not (alreadyLifted a), twin `notElem` axioms st ]
  where
    renderHead a = case axiomThen a of
      (h : _) -> h
      []      -> ""

-- An axiom that IS already a □-form owes no twin: every body condition a Match
-- and every body/head sentence starting @obliged.Obligor.@. This stops the
-- check demanding lifts-of-lifts (a lifted twin is itself all-Match liftable).
alreadyLifted :: Axiom -> Bool
alreadyLifted (Axiom body heads) = all bodyLifted body && all lifted heads
  where
    lifted s            = (obligedHead ++ ".Obligor.") `isPrefixOf` s
    bodyLifted (Match s) = lifted s
    bodyLifted _         = False

-- Can this world ever contain an @obliged.*@ fact? The producer census (the
-- input the v48 □-lift gate used, relocated here). Reads the producers —
-- practice and schedule insert atoms, db facts as of now, and axiom heads.
-- Conservative: a variable-headed producer counts (it could ground to
-- @obliged@), and an unresolvable 'CCall' (wild) counts too.
deonticInvokable :: PraxState -> Bool
deonticInvokable st = wild || any headProduces producerHeads
  where
    fns = cookedFnPool (cookedFns st)
    outcomeAtoms =
      [ cookedOutcomeAtoms fns [] o
      | cp <- Map.elems (cookedDefs st), a <- cpActions cp, o <- caOuts a ]
      ++ [ cookedOutcomeAtoms fns [] o | cp <- Map.elems (cookedDefs st), o <- cpInits cp ]
      ++ [ cookedOutcomeAtoms fns [] o
         | csr <- cookedSchedule st, (_, outs) <- csrBody csr, o <- outs ]
    -- An unresolvable Call could produce anything: treat the world as invoking.
    wild = Nothing `elem` outcomeAtoms
    insertHeads = [ h | Just (ins, _) <- outcomeAtoms, (h : _) <- ins ]
    -- An @obliged.*@ fact already in the db means the world invokes obligation,
    -- so its propagation (DEON property 1) must be declared.
    dbHeads     = [ h | s <- dbToSentences (db st), (h : _) <- [map intern (pathNames s)] ]
    axiomHeadsU = [ h | ax <- axioms st, s <- axiomThen ax, (h : _) <- [map intern (pathNames s)] ]
    producerHeads = insertHeads ++ dbHeads ++ axiomHeadsU
    headProduces h = h == intern obligedHead || symIsVar h

-- A tiny union-find over position-key strings.
find :: Map.Map String String -> String -> String
find uf x = case Map.lookup x uf of
  Just p | p /= x -> find uf p
  _               -> x

unionAll :: Map.Map String String -> [String] -> Map.Map String String
unionAll uf []       = uf
unionAll uf (x : xs) = foldl (`link` x) uf xs
  where link u a b = let ra = find u a; rb = find u b
                     in if ra == rb then u else Map.insert ra rb u

-- Every sentence, grouped by the scope its variables belong to (a practice; each
-- axiom; the live facts). Used for the sort pass — conditions and outcomes both
-- constrain a variable's sort.
sentencesByScope :: PraxState -> [(String, [String])]
sentencesByScope st =
     [ (practiceId p, practiceSents p) | p <- Map.elems (practiceDefs st) ]
  ++ [ ("fn " ++ fnName f, fnSents f) | f <- worldFns st ]
  ++ zipWith (\i ax -> ("axiom" ++ show i, condSents (axiomWhen ax) ++ axiomThen ax))
             [0 :: Int ..] (axioms st)
  ++ [ ("<facts>", dbToLabeledSentences (db st)) ]
  where
    practiceSents p =
         concatMap (\a -> condSents (actionConditions a) ++ outcomeSents (actionOutcomes a)) (actions p)
      ++ outcomeSents (initOutcomes p)
    -- Each function is its own binding scope: its params are call-scoped, not
    -- shared with any practice (spec v47). The old host-practice scoping
    -- incidentally union-linked sort constraints across functions of one host
    -- (Bar's recordDrink/checkTipsy shared @P@ by name+host coincidence, never
    -- by a modeled Call binding) — that accidental linkage is deliberately
    -- dropped; sort scoping that models Call bindings would be its principled
    -- successor if ever needed.
    fnSents f =
      concatMap (\c -> condSents (caseConditions c) ++ outcomeSents (caseOutcomes c)) (fnCases f)

