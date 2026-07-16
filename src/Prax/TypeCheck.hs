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
-- touch of an engine-owned fact family ('ReservedFamily', spec v45 — @turn@,
-- @seed@, @sceneEntered@, @contradiction@), and an unseeded die
-- ('SeedlessDraw').
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

import           Data.List (intercalate, nub)
import           Data.Maybe (isJust)
import qualified Data.Map.Strict as Map

import           Prax.Db (isVariable, pathNames, tokens, dbToLabeledSentences, exists)
import           Prax.Query (Condition (..), CookedCondition (..))
import           Prax.Relevance (mayUnifySyms, producibleAtoms)
import           Prax.Rng (seedPath)
import           Prax.Script (sceneEnteredPath)
import           Prax.Sym (symName, symIsVar)
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
    -- ^ a world registers a practice whose outcomes compile a
    -- 'Prax.Rng.draw' (a @ForEach@ guarded on the @seed@ family) but has no
    -- @seed@ fact: every draw would silently fail to ever fire.
  | DeadCondition { teWhere :: String, teSentence :: String }
    -- ^ the positive pattern @teSentence@ (at @teWhere@) may-unifies nothing
    -- the world can ever contain: the site can never fire.
  deriving (Eq, Show)

-- | Every well-formedness problem in a world (empty ⇒ the world is well-formed).
typeCheck :: PraxState -> [TypeError]
typeCheck st =
     concatMap unboundInPractice ps
  ++ concatMap unboundInAxiom (axioms st)
  ++ cardinalityErrors (assertedSentences st)
  ++ refErrors st
  ++ sortErrors st
  ++ reservedFamilyErrors st
  ++ seedlessDrawErrors st
  ++ deadConditionErrors st
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

-- Check 1: unbound variables --------------------------------------------------

unboundInOutcomes :: String -> [String] -> [Outcome] -> [TypeError]
unboundInOutcomes loc bound outs =
  [ UnboundVar loc v s
  | o <- outs, (v, s) <- outcomeUses o, v `notElem` bound ]

unboundInPractice :: Practice -> [TypeError]
unboundInPractice p =
     unboundInOutcomes (practiceId p ++ " (init)") (roles p) (initOutcomes p)
  ++ concatMap action' (actions p)
  ++ concatMap fn' (functions p)
  where
    action' a =
      unboundInOutcomes (practiceId p ++ " / " ++ actionName a)
        ("Actor" : roles p ++ concatMap condVars (actionConditions a))
        (actionOutcomes a)
    fn' f =
      concatMap
        (\c -> unboundInOutcomes (practiceId p ++ " / fn " ++ fnName f)
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
refErrors st = concatMap practiceRefs ps
  where
    ps          = Map.elems (practiceDefs st)
    definedFns  = [ fnName f | p <- ps, f <- functions p ]
    definedPrac = Map.keys (practiceDefs st)
    practiceRefs p = concatMap (outcomeRef (practiceId p)) (allOutcomes p)
    allOutcomes p =
      initOutcomes p ++ concatMap actionOutcomes (actions p)
      ++ [ o | f <- functions p, c <- fnCases f, o <- caseOutcomes c ]
    outcomeRef loc (Call fn _)
      | fn `notElem` definedFns = [ UndefinedRef loc fn ]
    outcomeRef loc (Insert s)
      | ("practice" : pid : _) <- pathNames s
      , pid `notElem` definedPrac = [ UndefinedRef loc ("practice." ++ pid) ]
    outcomeRef loc (InsertFor _ s)
      | ("practice" : pid : _) <- pathNames s
      , pid `notElem` definedPrac = [ UndefinedRef loc ("practice." ++ pid) ]
    outcomeRef loc (ForEach _ subs) = concatMap (outcomeRef loc) subs
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
                ++ concatMap (inserts . actionOutcomes) (actions p)
                ++ concatMap (concatMap (inserts . caseOutcomes) . fnCases) (functions p) ]
  ++ [ h | ax <- axioms st, h <- axiomThen ax ]
  ++ dbToLabeledSentences (db st)
  where
    ps = Map.elems (practiceDefs st)
    inserts os = [ s | Insert s <- os ] ++ [ s | InsertFor _ s <- os ]
              ++ concat [ inserts subs | ForEach _ subs <- os ]

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

-- Only compiled mechanism may touch these families (spec v45). WritesForbidden
-- families (@turn@, @contradiction@) have NO legitimate authored writer at
-- all — reads stay free (turn is the documented time interface; a
-- contradiction read cannot corrupt). MachineryShapeOnly families (@seed@,
-- @sceneEntered@) are machinery in BOTH polarities: the only legal touch is
-- the mechanism's own compiled shape — every name after the family head a
-- Prax-namespaced variable. An authored literal, plain variable, or bare
-- subtree pattern on the family is a loud error: each mechanism assumes it is
-- its family's sole accessor, so an authored touch corrupts it silently
-- otherwise.
data FamilyLaw = WritesForbidden | MachineryShapeOnly

reservedFamilies :: [(String, FamilyLaw)]
reservedFamilies =
  [ (turnPath,         WritesForbidden)
  , (seedPath,         MachineryShapeOnly)
  , (sceneEnteredPath, MachineryShapeOnly)
  , ("contradiction",  WritesForbidden)
  ]

reservedFamilyErrors :: PraxState -> [TypeError]
reservedFamilyErrors st =
     [ ReservedFamily fam loc s
     | (loc, os) <- writeSites st, o <- os, s <- writesOf o
     , Just (fam, law) <- [familyOf s], violatesWrite law s ]
  ++ [ ReservedFamily fam "axiom" h
     | ax <- axioms st, h <- axiomThen ax
     , Just (fam, law) <- [familyOf h], violatesWrite law h ]
  ++ [ ReservedFamily fam loc s
     | (loc, cs) <- readSites st, s <- condSents cs
     , not (machineryShaped s), Just (fam, MachineryShapeOnly) <- [familyOf s] ]
  where
    familyOf s = case pathNames s of
      (h : _) -> (,) h <$> lookup h reservedFamilies
      []      -> Nothing
    violatesWrite WritesForbidden    _ = True
    violatesWrite MachineryShapeOnly s = not (machineryShaped s)
    -- The unforgeable signature: a non-empty tail, every name of which is a
    -- Prax-namespaced VARIABLE (authors cannot write those — v40).
    machineryShaped s = case pathNames s of
      (_ : rest@(_ : _)) -> all isPraxMachineryVar rest
      _                  -> False
    isPraxMachineryVar n = isVariable n && isPraxVar n
    writesOf o = case o of
      Insert s      -> [s]
      InsertFor _ s -> [s]
      Delete s      -> [s]                    -- a delete is a write
      ForEach _ os  -> concatMap writesOf os
      Call _ _      -> []

-- The authored write sites, with labels: practice action/init/function-case
-- outcomes and every schedule rule body's outcomes.
writeSites :: PraxState -> [(String, [Outcome])]
writeSites st =
     [ (practiceId p ++ " (init)", initOutcomes p) | p <- ps ]
  ++ [ (practiceId p ++ " / " ++ actionName a, actionOutcomes a) | p <- ps, a <- actions p ]
  ++ [ (practiceId p ++ " / fn " ++ fnName f, caseOutcomes c)
     | p <- ps, f <- functions p, c <- fnCases f ]
  ++ [ ("schedule " ++ srName r, outs) | r <- schedule st, (_, outs) <- srBody r ]
  where ps = Map.elems (practiceDefs st)

-- The authored read sites, with labels: action/fn-case conditions, ForEach
-- guards nested anywhere in a write site's outcomes, axiom bodies, desires'
-- and characters' want conditions, and schedule rule bodies' conditions.
readSites :: PraxState -> [(String, [Condition])]
readSites st =
     [ (practiceId p ++ " / " ++ actionName a, actionConditions a) | p <- ps, a <- actions p ]
  ++ [ (practiceId p ++ " / fn " ++ fnName f, caseConditions c)
     | p <- ps, f <- functions p, c <- fnCases f ]
  ++ [ (loc ++ " (effect guard)", gs) | (loc, os) <- writeSites st, gs <- outcomeGuards os ]
  ++ [ ("axiom", axiomWhen ax) | ax <- axioms st ]
  ++ [ ("desire " ++ desireName d, wantConditions (desireWant d)) | d <- desires st ]
  ++ [ ("want of " ++ charName c, wantConditions w) | c <- characters st, w <- charWants c ]
  ++ [ ("schedule " ++ srName r, conds) | r <- schedule st, (conds, _) <- srBody r ]
  where ps = Map.elems (practiceDefs st)

-- ForEach guards, recursively, in a list of outcomes (the write-effect side of
-- the read scan — a draw's, or the scene stamp's, own guard is a read too).
outcomeGuards :: [Outcome] -> [[Condition]]
outcomeGuards outs = concat [ conds : outcomeGuards os | ForEach conds os <- outs ]

-- Check 6: draws need a seeded die ------------------------------------------

-- A world whose outcomes compile a 'Prax.Rng.draw' — a @ForEach@ whose
-- guard's first condition is a @Match@ on the @seed@ family (the shape
-- 'Prax.Rng.draw' always compiles to) — but has no @seed@ fact: the die was
-- never seeded, so every draw's guard can never hold.
seedlessDrawErrors :: PraxState -> [TypeError]
seedlessDrawErrors st =
  [ SeedlessDraw
  | any (any outcomeUsesSeed) allOutcomeLists, not (exists seedPath (db st)) ]
  where
    ps = Map.elems (practiceDefs st)
    allOutcomeLists =
      [ initOutcomes p | p <- ps ]
      ++ [ actionOutcomes a | p <- ps, a <- actions p ]
      ++ [ caseOutcomes c | p <- ps, f <- functions p, c <- fnCases f ]
      ++ [ outs | r <- schedule st, (_, outs) <- srBody r ]
    outcomeUsesSeed (ForEach conds outs) =
      any guardReadsSeed conds || any outcomeUsesSeed outs
    outcomeUsesSeed _ = False
    guardReadsSeed (Match s) = case pathNames s of
      (h : _) -> h == seedPath
      []      -> False
    guardReadsSeed _ = False

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
  ++ [ (pid ++ " / fn " ++ fn, cs)
     | (pid, cp) <- defs, (fn, (_, cases)) <- Map.toList (cpFns cp)
     , (cs, _) <- cases ]
  ++ [ (pid ++ " / fn " ++ fn ++ " (effect guard)", gs)
     | (pid, cp) <- defs, (fn, (_, cases)) <- Map.toList (cpFns cp)
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
forEachGuards outs = concat [ conds : forEachGuards os | CForEach conds os <- outs ]

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
  ++ zipWith (\i ax -> ("axiom" ++ show i, condSents (axiomWhen ax) ++ axiomThen ax))
             [0 :: Int ..] (axioms st)
  ++ [ ("<facts>", dbToLabeledSentences (db st)) ]
  where
    practiceSents p =
         concatMap (\a -> condSents (actionConditions a) ++ outcomeSents (actionOutcomes a)) (actions p)
      ++ outcomeSents (initOutcomes p)
      ++ concatMap (concatMap (\c -> condSents (caseConditions c)
                                     ++ outcomeSents (caseOutcomes c)) . fnCases) (functions p)

condSents :: [Condition] -> [String]
condSents = concatMap go
  where
    go (Match s)        = [s]
    go (Not s)          = [s]
    go (Absent cs)      = condSents cs
    go (Exists cs)      = condSents cs
    go (Or clauses)     = concatMap condSents clauses
    go (Subquery _ _ w) = condSents w
    go _                = []

outcomeSents :: [Outcome] -> [String]
outcomeSents = concatMap go
  where
    go (Insert s)          = [s]
    go (Delete s)          = [s]
    go (InsertFor _ s)     = [s]
    go (Call _ _)          = []
    go (ForEach conds outs) = condSents conds ++ outcomeSents outs
