-- | Which desires can authored action even in principle improve? Computed once
-- per world from the vocabulary (spec
-- @docs/specs/2026-07-11-v26-planner-work.md@ §2) and consulted by the planner
-- to skip predictions that are provably fruitless: a believed model none of
-- whose desires any available action can improve admits no motivated move.
--
-- The analysis is __conservative__: it answers "not improvable" only when
-- that is provable from the authored patterns. Anything uncertain — outcomes
-- behind unresolvable 'Call's, wants over facts an axiom may derive, wants
-- gated by 'Subquery'\/'Count'\/'Calc' — counts as improvable. An unsound
-- "not improvable" is a planner behavior change and a defect; a spurious
-- "improvable" merely costs the evaluation we would have done anyway.
--
-- One stated invariant carries the conservativity (an assumption about
-- authored worlds, not a construction guarantee): __entity names never
-- collide with predicate-name literals__ — no character, place, or value is
-- named @lied@, @believes@, @regards@, and so on. 'mayUnifySyms' spends it in
-- exactly one place (the anchor below). A world that named a character after
-- a predicate segment would void the analysis; every shipped world satisfies
-- the invariant, and the golden decision-sequence tests would surface a
-- violation as a dropped prediction.
--
-- __Authoring boundary = string, world model = cooked.__ The string surface
-- ('Prax.Types.conditionVars' and the v40 authoring guards) runs before
-- cooking exists — it validates what the author wrote. Everything here is a
-- world-model analysis and reads the cooked form: the state's precooked
-- tables ('cookedDefs'\/'cookedRules'\/'cookedDesires'\/'cookedWants'), never
-- the authored strings.
--
-- 'livenessOf' (spec @docs/specs/2026-07-13-v33-live-relevance.md@) adds the
-- state-conditioned dimension over the same vocabulary: not just "could
-- anything ever improve this want-kind?" but "is that improvability LIVE
-- right now?" Each named desire's per-'Prax.Types.Liveness' recipe — a
-- negative want-kind's floor check, a positive want-kind's environment
-- gates, or 'Prax.Types.AlwaysLive' when no cheap state test applies — is
-- computed once per world and cached on 'Prax.Types.PraxState' (like
-- 'improvableDesires' itself), for the planner's per-state dead-now test to
-- consult cheaply. Conservativity runs the same direction as above: a gate
-- only ever removes work when provably safe (axiom-derivable or otherwise
-- uncertain candidates are never gates), and every uncertainty — an
-- unresolvable outcome, a @Subquery@\/@Count@\/@Calc@-tainted want — keeps
-- the desire 'Prax.Types.AlwaysLive'.
module Prax.Relevance
  ( mayUnifySyms
  , improvableDesires
  , livenessOf
  , bearingTemplates
  , evictionShadowNames
  , moverReadAnchors
  , producibleAtoms
  , cookedFnPool
  , cookedOutcomeAtoms
  ) where

import           Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map

import           Prax.Db (Bindings, Val (..), pathNames, dbToSentences)
import           Prax.Derive (CookedRule (..))
import           Prax.Query (CookedCondition (..), cookCondition, cookedReadAnchors, groundCookedCondition, groundNames)
import           Prax.Sym (Sym, intern, symIsVar)
import           Prax.Types

-- | Could a grounded instance of one path pattern be an instance (or a
-- prefix\/extension) of the other, on pre-split, pre-interned paths — the
-- planner-hot classification ('relevantDelta' runs it for every primitive
-- delta against every footprint pattern). Segments unify when either is a
-- variable or they are equal; length mismatch is prefix-compatible (a
-- 'Match' sees subtrees). A pair unifies only if some overlapping segment is
-- a shared /literal/ (both sides constant and equal) — Prax facts are
-- identified by their literal predicate-name segments, so an overlap covered
-- entirely by variables carries no evidence the two patterns denote the same
-- predicate at all (any string could occupy a variable slot, including
-- another pattern's unrelated literal, e.g. a role variable coincidentally
-- lining up against someone else's "lied"). Discarding such evidence-free
-- overlaps is where the module header's entity-names-vs-predicate-literals
-- invariant is spent: under it, a genuine correspondence between an authored
-- effect and a want always shares an aligned literal, so the anchor removes
-- only coincidence. Variable-ness is the parity bit ('Prax.Sym.symIsVar')
-- and a shared literal segment is Int equality — the hottest classification
-- in the engine, Int equality rather than String equality.
mayUnifySyms :: [Sym] -> [Sym] -> Bool
mayUnifySyms as bs = anchored && and (zipWith seg as bs)
  where
    seg x y = symIsVar x || symIsVar y || x == y
    anchored = or (zipWith literalMatch as bs)
    literalMatch x y = not (symIsVar x) && not (symIsVar y) && x == y

-- | The eviction shadows of an exclusion insert, computed directly on
-- already-split, interned tokens — the single name-shape every consumer
-- ('Prax.Engine.performCooked'\'s cooked hot path, 'Prax.Engine.relevantDelta',
-- and the atom-pool walk below) goes through, so there is one implementation.
-- One shadow per @!@ operator: the names up to and including that point,
-- followed by a fresh @"PraxEvicted"@ segment (interns as a variable —
-- uppercase initial, Prax-namespaced machinery — so 'mayUnifySyms' treats it
-- as the wildcard it denotes and no authored name can collide with it). Each
-- exclusion clears the displaced sibling's entire subtree — arbitrary depth
-- and shape — and 'mayUnifySyms' compares only up to the shorter path, so
-- the truncated shadow covers every want under it.
evictionShadowNames :: [(Sym, Maybe Char)] -> [[Sym]]
evictionShadowNames toks =
  [ map fst (take i toks) ++ [intern "PraxEvicted"]
  | (i, (_, op)) <- zip [1 ..] toks, op == Just '!' ]

-- The Call-resolution pool: every registered function's cooked case outcomes,
-- guards ignored (conservatively: all cases), keyed by 'fnName'. Reads the one
-- registry ('Prax.Types.cookedFns') directly — since v47 functions have a
-- single home, so there is no practice fold and no resolution-order subtlety
-- (both this pool and 'Prax.Engine.lookupCookedFn' read the same Map, exactly).
cookedFnPool :: Map String ([String], [([CookedCondition], [CookedOutcome])])
             -> Map String [CookedOutcome]
cookedFnPool = Map.map (concatMap snd . snd)

-- The insert- and delete-shaped atoms an outcome can produce, resolving
-- 'CCall's through the pool (conservatively: all cases). An exclusion insert
-- both asserts its path and evicts that value's __siblings__ — whose names
-- appear nowhere in the outcome — so the delete side carries the path itself
-- plus its eviction shadows ('evictionShadowNames': conservatively, any
-- sibling). Returns Nothing for "unknown effects" (unresolvable 'CCall'):
-- the caller must treat that as improves-everything.
cookedOutcomeAtoms :: Map String [CookedOutcome] -> [String] -> CookedOutcome
                   -> Maybe ([[Sym]], [[Sym]])
cookedOutcomeAtoms fns visited o = case o of
  CInsert toks
    | any ((== Just '!') . snd) toks
                  -> Just ([map fst toks], map fst toks : evictionShadowNames toks)
    | otherwise   -> Just ([map fst toks], [])
  CInsertFor _ toks -> cookedOutcomeAtoms fns visited (CInsert toks)  -- deferred
                                                     -- retract is environment: same atoms
  CDelete toks    -> Just ([], [map fst toks])
  CForEach _ outs -> mconcat' (map (cookedOutcomeAtoms fns visited) outs)
  CRoll _ _ _ outs -> mconcat' (map (cookedOutcomeAtoms fns visited) outs)  -- body may hit
  CCall fn _
    | fn `elem` visited -> Just ([], [])           -- cycle: already counted
    | otherwise -> case Map.lookup fn fns of
        Nothing   -> Nothing                       -- unknown function: wild
        Just outs -> mconcat' (map (cookedOutcomeAtoms fns (fn : visited)) outs)
  where
    mconcat' ms = do
      pairs <- sequence ms
      pure (concatMap fst pairs, concatMap snd pairs)

-- Positive and negated path patterns of a want's cooked conditions. The Bool
-- is "uncertain": the want's satisfaction depends on machinery (numeric
-- binds, counts, subqueries) beyond pattern presence. Deliberately NOT
-- 'Prax.Query.cookedReadAnchors': subquery internals are machinery here
-- (taint), not read anchors — the two walks answer different questions.
cookedWantPatterns :: [CookedCondition] -> ([[Sym]], [[Sym]], Bool)
cookedWantPatterns = foldr step ([], [], False)
  where
    step c (pos, neg, unc) = case c of
      CMatch p     -> (p : pos, neg, unc)
      CNot p       -> (pos, p : neg, unc)
      CAbsent cs   -> let (p', n', u') = cookedWantPatterns cs
                      in (pos ++ n', neg ++ p', unc || u')
      CExists cs   -> let (p', n', u') = cookedWantPatterns cs
                      in (pos ++ p', neg ++ n', unc || u')
      COr clauses  -> let parts = map cookedWantPatterns clauses
                      in ( pos ++ concatMap (\(p', _, _) -> p') parts
                         , neg ++ concatMap (\(_, n', _) -> n') parts
                         , unc || any (\(_, _, u') -> u') parts )
      CEq {}       -> (pos, neg, unc)
      CNeq {}      -> (pos, neg, unc)
      CCmp {}      -> (pos, neg, unc)
      CCalc {}     -> (pos, neg, True)
      CCount {}    -> (pos, neg, True)
      CSubquery {} -> (pos, neg, True)

-- Every effect an authored MOVER action can cause, resolved once per world:
-- the insert- and delete-shaped atom pools ('cookedOutcomeAtoms' over every
-- action's declared outcomes, plus every practice's 'cpInits' — spawning runs
-- those too), and whether any of them is "wild" (an unresolvable 'CCall',
-- conservatively improves-everything). Ranges over 'cookedDefs' — the
-- practices movers can take — and NOT the schedule surface ('cookedSchedule'
-- lives off 'cookedDefs' by construction, spec v44): a desire only the engine
-- schedule can improve (@hungry.*@, @marketDay.*@) has no improving MOVER
-- action, so the static screen stays exact and its liveness becomes a
-- GateCheck the pulse flips (the v35 wake). Shared by 'improvableDesires' and
-- 'livenessOf' — one atom-pool computation, not two.
data AtomPools = AtomPools
  { poolInserted :: [[Sym]]
  , poolDeleted  :: [[Sym]]
  , poolWild     :: Bool
  }

worldAtomPools :: Map String CookedPractice -> Map String [CookedOutcome] -> AtomPools
worldAtomPools defs fns = AtomPools
  { poolInserted = concatMap (maybe [] fst) atoms
  , poolDeleted  = concatMap (maybe [] snd) atoms
  , poolWild     = Nothing `elem` atoms
  }
  where
    practices = Map.elems defs
    atoms = [ cookedOutcomeAtoms fns [] o
            | cp <- practices, a <- cpActions cp, o <- caOuts a ]
         ++ [ cookedOutcomeAtoms fns [] o | cp <- practices, o <- cpInits cp ]

-- Axiom heads count as derivable: a want (or gate candidate) over a
-- derivable pattern is conservatively improvable\/never a gate. □-lifted
-- heads arrive as rules of their own ('Prax.Derive.cookAxioms'). Shared by
-- 'improvableDesires' and 'livenessOf'.
axiomDerivable :: [CookedRule] -> [Sym] -> Bool
axiomDerivable rules p = any (mayUnifySyms p) heads
  where heads = [ map fst h | r <- rules, h <- crHeads r ]

-- | The names of the desires some authored action might improve. Reads the
-- state's cooked tables ('cookedDefs'\/'cookedRules'\/'cookedDesires' —
-- 'Prax.Engine.retable' cooks before it analyzes). See the module header
-- for the conservativity contract.
improvableDesires :: PraxState -> [String]
improvableDesires st =
  [ desireName d | d <- desires st, improvable d ]
  where
    pools = worldAtomPools (cookedDefs st) (cookedFnPool (cookedFns st))
    wild = poolWild pools
    inserted = poolInserted pools
    deleted  = poolDeleted pools
    derivable = axiomDerivable (cookedRules st)
    improvable d@(Desire _ (Want _ u))
      | u == 0    = False
      | wild      = True
      | unc       = True
      | any derivable (pos ++ neg) = True
      | u > 0     = any (\i -> any (mayUnifySyms i) pos) inserted
                    || any (\dl -> any (mayUnifySyms dl) neg) deleted
      | otherwise = any (\dl -> any (mayUnifySyms dl) pos) deleted
                    || any (\i -> any (mayUnifySyms i) neg) inserted
      where (pos, neg, unc) = cookedWantPatterns (cookedDesires st Map.! desireName d)

-- | Classify every named desire's dead-now recipe (see 'Liveness'): a
-- negative want-kind is unconditionally 'FloorCheck'; a positive want-kind
-- gates on its top-level positive @Match@ conjuncts that are neither
-- action-insertable nor axiom-derivable (each such conjunct, cooked, is one
-- gate — 'null' qualifying conjuncts, an unresolvable\/wild outcome, or a
-- @Subquery@\/@Count@\/@Calc@-tainted want all fall back to 'AlwaysLive');
-- weight 0 is statically never-improvable already and is mapped 'AlwaysLive'
-- defensively (the static filter, 'improvableDesires', screens it first).
livenessOf :: PraxState -> Map String Liveness
livenessOf st =
  Map.fromList [ (desireName d, classify d) | d <- desires st ]
  where
    pools = worldAtomPools (cookedDefs st) (cookedFnPool (cookedFns st))
    derivable = axiomDerivable (cookedRules st)
    classify d@(Desire _ (Want _ u))
      | u < 0     = FloorCheck
      | u > 0     = positive (cookedDesires st Map.! desireName d)
      | otherwise = AlwaysLive
    positive conds
      | unc || poolWild pools || null gates = AlwaysLive
      | otherwise = GateCheck gates
      where
        (_, _, unc) = cookedWantPatterns conds
        candidates = [ p | CMatch p <- conds ]
        qualifies p = not (any (mayUnifySyms p) (poolInserted pools))
                    && not (derivable p)
        gates = [ [CMatch p] | p <- candidates, qualifies p ]

-- | Everything 'Prax.Planner.predictMove' (scope gate included) can read when
-- the pick's actor predicts mover @m@, as pattern anchors grounded to the
-- pair: the prediction-scope template (Actor:=actor, Witness:=m); the
-- believed-model source family (@\<actor\>.believes.desires.\<m\>.*@ — the
-- exact family "Prax.Minds" consults); the mover's death mark; every
-- practice's instance pattern, action conditions, and outcome-embedded
-- conditions (ForEach guards recursively, every function case — the imagined
-- apply queries these) with Actor:=m; and every vocabulary desire's
-- conditions with Owner:=m (model evaluation and the dead-now checks).
-- Ungrounded variables stay variables ('mayUnifySyms' wildcards): partial
-- grounding only ever widens the set, never narrows it — 'cpInits' and
-- function cases are left fully wild because their call-time bindings are
-- not the mover's.
moverReadAnchors :: PraxState -> Character -> Character -> [[Sym]]
moverReadAnchors st actor m =
  scopeReads ++ [believesRead, deadRead] ++ affordanceReads ++ functionReads ++ desireReads
  where
    mSym   = intern (charName m)
    actorB = Map.singleton (intern "Actor") (VSym mSym)
    ownerB = Map.singleton (intern "Owner") (VSym mSym)
    scopeB = Map.fromList [ (intern "Actor",   VSym (intern (charName actor)))
                          , (intern "Witness", VSym mSym) ]
    readsOf b conds = cookedReadAnchors (map (groundCookedCondition b) conds)
    scopeReads   = readsOf scopeB (map cookCondition (predictionScope st))
    believesRead = [ intern (charName actor), intern "believes"
                   , intern "desires", mSym, intern "PraxD" ]
    deadRead     = map intern (pathNames (deadSentence (charName m)))
    affordanceReads = concat
      [ groundNames actorB (cpInstanceNames cp)
        : concatMap (\ca -> readsOf actorB (caConds ca)
                            ++ outcomeCondReads actorB (caOuts ca))
                    (cpActions cp)
        ++ outcomeCondReads Map.empty (cpInits cp)
      | cp <- Map.elems (cookedDefs st) ]
    -- The one registry ('cookedFns'), read once (not per practice, since
    -- functions have a single home since v47).
    functionReads = concat [ readsOf Map.empty cs ++ outcomeCondReads Map.empty os
                           | (_, cases) <- Map.elems (cookedFns st), (cs, os) <- cases ]
    desireReads = concat [ readsOf ownerB conds | conds <- Map.elems (cookedDesires st) ]

-- Conditions embedded in outcomes ('CForEach' guards, recursively) — the
-- imagined apply queries these against the node's view.
outcomeCondReads :: Bindings -> [CookedOutcome] -> [[Sym]]
outcomeCondReads b outs = concat
  [ case o of
      CForEach cs os    -> reads' cs os
      CRoll _ _ cs os   -> reads' cs os   -- a roll's guard reads like a CForEach's
      _                 -> []
  | o <- outs ]
  where reads' cs os = cookedReadAnchors (map (groundCookedCondition b) cs)
                         ++ outcomeCondReads b os

-- | Per character, the affordance templates whose authored outcomes could
-- touch their own wants or desires — the opportunity-relevance half of the
-- v35 motive signature (spec @docs/specs/2026-07-13-v35-intentions.md@): a
-- newly available or vanished action interrupts a standing intention only
-- when some insert- or delete-shaped atom of its outcomes ('cookedOutcomeAtoms',
-- 'CCall's resolved through the declared functions) may-unify some pattern
-- the character's wants or held desires read ('cookedReadAnchors' — total,
-- subquery internals included). Conservative like everything here: an
-- unresolvable 'CCall' bears on everyone; over-bearing merely re-deliberates,
-- under-bearing would sleep through an arc (the probed "bold agent" failure:
-- dana serving Wait all drive while fresh deliberation wanted shun).
bearingTemplates :: PraxState -> Map String [String]
bearingTemplates st =
  Map.fromList [ (charName c, bearing (charPats c)) | c <- characters st ]
  where
    defs = cookedDefs st
    fns = cookedFnPool (cookedFns st)
    actionAtoms = [ (caName a, atoms a)
                  | cp <- Map.elems defs, a <- cpActions cp ]
    atoms a = do
      pairs <- traverse (cookedOutcomeAtoms fns []) (caOuts a)
      pure (concatMap fst pairs ++ concatMap snd pairs)
    charPats c = cookedReadAnchors
      (  concat (Map.findWithDefault [] (charName c) (cookedWants st))
      ++ [ cc | d <- desires st, desireName d `elem` charDesires c
              , cc <- cookedDesires st Map.! desireName d ] )
    bearing pats =
      [ n | (n, m) <- actionAtoms
          , case m of
              Nothing -> True
              Just as -> any (\atom -> any (mayUnifySyms atom) pats) as ]

-- | Everything the registered world can ever contain, as pattern anchors:
-- the initial db's facts, every practice's insert-side atoms
-- ('cookedOutcomeAtoms'\'s insert half over ALL practices), every SCHEDULE
-- rule's insert-side atoms (unlike 'worldAtomPools', which ranges over movers
-- only: the consumer asks "can this fact ever exist", and schedule-moved facts
-- — @marketDay.*@, expiring feelings — exist), every axiom head
-- ('cookedRules'\' 'crHeads', □-lifted forms included — heads count regardless
-- of whether their rules can fire; conservative, which here only ever silences
-- the consumer), the engine's own @turn@ clock, and its @contradiction@ witness
-- ('Prax.Engine.reclose' inserts it at ⊥). @Nothing@ = wild (an
-- unresolvable 'CCall'): the caller must go silent. First consumer:
-- "Prax.TypeCheck"'s dead-condition lint
-- (spec @docs/specs/2026-07-15-v42-dead-condition-lint.md@).
producibleAtoms :: PraxState -> Maybe [[Sym]]
producibleAtoms st = do
  pairs <- sequence ( [ cookedOutcomeAtoms fns [] o
                      | cp <- practices, a <- cpActions cp, o <- caOuts a ]
                   ++ [ cookedOutcomeAtoms fns [] o
                      | cp <- practices, o <- cpInits cp ]
                   ++ [ cookedOutcomeAtoms fns [] o
                      | csr <- cookedSchedule st, (_, outs) <- csrBody csr, o <- outs ] )
  pure ( concatMap fst pairs
      ++ [ map intern (pathNames s) | s <- dbToSentences (db st) ]
      ++ [ map fst h | r <- cookedRules st, h <- crHeads r ]
      ++ [[intern turnPath]]        -- the engine produces the clock
      ++ [[intern "contradiction"]] )
  where
    practices = Map.elems (cookedDefs st)
    fns = cookedFnPool (cookedFns st)
