# v34 — Prediction Reuse Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `scoreActions` stops re-deriving identical predictions at every imagination-tree
node, per `docs/specs/2026-07-13-v34-prediction-reuse.md` — bit-for-bit identical decisions,
the recursion's redundancy reclaimed.

**Architecture:** Three static enumerations (what an action's outcomes can touch, what a
prediction reads, which derived families axioms can write) feed one runtime test: a node
whose accumulated path delta — derived-fact cone included — cannot may-unify anything a
(actor, mover) prediction reads must yield the root state's prediction, so the pick-scoped
memo answers instead of `predictMove`. Effects knowledge lives in `Prax.Engine` (it mirrors
`performAction`), read knowledge in `Prax.Relevance`, the memo threading in `Prax.Planner`.

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- Exact only: goldens byte-identical (village/bar/intrigue/feud), ViewInvariant green,
  suite green (441 baseline @ ~175s) after every task. Net failure = BLOCK with the trace.
- Conservativity is one-directional: every uncertainty (opaque delta, unknown mover, any
  cone/read intersection) computes the prediction live. An unsound reuse is Critical.
- Reuse never changes WHAT is scored: no candidate pruned, no score approximated, no
  frequency consulted.
- Zero warnings; hlint "No hints"; grep-gates unchanged; no re-exports (Liveness precedent,
  d950f9b).
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: The three static enumerations

**Files:**
- Modify: `src/Prax/Derive.hs` (export list + one function)
- Modify: `src/Prax/Relevance.hs` (two exports)
- Modify: `src/Prax/Engine.hs` (one export + helper, `retable`)
- Modify: `src/Prax/Types.hs` (one `PraxState` field)
- Modify: `src/Prax/Query.hs` (export `groundNames` if not already exported)
- Test: `test/Prax/RelevanceSpec.hs`, `test/Prax/EngineSpec.hs`

**Interfaces produced (Task 2 consumes exactly these):**
- `Prax.Relevance.moverReadAnchors :: PraxState -> Character -> Character -> [[Sym]]`
- `Prax.Engine.groundedDeltaAnchors :: PraxState -> GroundedAction -> Maybe [[Sym]]`
- `PraxState`'s `axiomHeads :: [[Sym]]` (retable-maintained)
- `Prax.Relevance.mayUnifySyms` (existing, unchanged)

**Design.**

`Prax.Derive` gains (and exports) the head enumeration, beside `axiomFootprint` whose
lift-notion it shares:

```haskell
-- | Every head template the axioms can write — 'axiomThen' plus the □-lifted
-- forms of liftable rules ('liftObliged', the same notion 'axiomFootprint'
-- uses: heads of rules that can actually fire). A delta that feeds some
-- axiom can change derived facts only in these families.
axiomHeadPatterns :: [Axiom] -> [String]
axiomHeadPatterns axs = concat [ hs | Axiom _ hs <- axs ++ mapMaybe liftObliged axs ]
```

Do NOT touch `Prax.Relevance.axiomDerivable`: its unconditional `"obliged.W." ++ h` lift is
deliberately more conservative (it answers "could an axiom-shaped thing derive this?" for
the improvability screen); `axiomHeadPatterns` answers "which heads can fire?" for the cone.
Different questions, each with one home.

`Prax.Types.PraxState` gains, in the derived-field block beside `footprint`:

```haskell
  , axiomHeads :: [[Sym]]
    -- ^ Interned anchors of every axiom head that can fire
    -- ('Prax.Derive.axiomHeadPatterns') plus the @contradiction@ witness
    -- ('Prax.Engine.reclose' inserts it when a delta trips ⊥). Maintained by
    -- 'Prax.Engine.retable'; consumed by the planner's prediction-reuse cone:
    -- a path delta that feeds any axiom ('footprint') can change derived
    -- facts only in these families.
```

`emptyState` gets `axiomHeads = [[intern "contradiction"]]` (retable's value for zero
axioms). `Prax.Engine.retable` adds (importing `axiomHeadPatterns` from `Prax.Derive`):

```haskell
  , axiomHeads = [ map intern (pathNames h) | h <- axiomHeadPatterns (axioms st) ]
                 ++ [[intern "contradiction"]]
```

`Prax.Relevance` gains and exports `cookedReadAnchors` and `moverReadAnchors` (new imports:
`CookedCondition (..)`, `groundCookedCondition`, `groundNames` from `Prax.Query` — extend
`Prax.Query`'s export list with `groundNames` if it is not already there; `Val (..)` from
`Prax.Db`; `deadSentence` comes with `Prax.Types`):

```haskell
-- | Every DB path a cooked-condition query can consult, at any polarity —
-- including inside Or\/Absent\/Exists\/Subquery. Complete by construction:
-- CEq\/CNeq\/CCmp\/CCalc compare already-bound values and CCount measures a
-- bound set (produced by a CSubquery, whose inner conditions ARE walked), so
-- none of them reads a path this walk misses.
cookedReadAnchors :: [CookedCondition] -> [[Sym]]
cookedReadAnchors = concatMap go
  where
    go c = case c of
      CMatch p         -> [p]
      CNot p           -> [p]
      COr clauses      -> concatMap cookedReadAnchors clauses
      CAbsent cs       -> cookedReadAnchors cs
      CExists cs       -> cookedReadAnchors cs
      CSubquery _ _ ws -> cookedReadAnchors ws
      CEq {}           -> []
      CNeq {}          -> []
      CCmp {}          -> []
      CCalc {}         -> []
      CCount {}        -> []

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
  scopeReads ++ [believesRead, deadRead] ++ affordanceReads ++ desireReads
  where
    mSym   = intern (charName m)
    actorB = Map.singleton (intern "Actor") (VSym mSym)
    ownerB = Map.singleton (intern "Owner") (VSym mSym)
    scopeB = Map.fromList [ (intern "Actor",   VSym (intern (charName actor)))
                          , (intern "Witness", VSym mSym) ]
    readsOf b conds = cookedReadAnchors (map (groundCookedCondition b) conds)
    scopeReads   = readsOf scopeB (map cookCondition (predictionScope st))
    believesRead = [ intern (charName actor), intern "believes"
                   , intern "desires", mSym, intern "D" ]
    deadRead     = map intern (pathNames (deadSentence (charName m)))
    affordanceReads = concat
      [ groundNames actorB (cpInstanceNames cp)
        : concatMap (\ca -> readsOf actorB (caConds ca)
                            ++ outcomeCondReads actorB (caOuts ca))
                    (cpActions cp)
        ++ outcomeCondReads Map.empty (cpInits cp)
        ++ concat [ readsOf Map.empty cs
                  | (_, cases) <- Map.elems (cpFns cp), (cs, _) <- cases ]
      | cp <- Map.elems (cookedDefs st) ]
    desireReads = concat [ readsOf ownerB conds | conds <- Map.elems (cookedDesires st) ]

-- Conditions embedded in outcomes ('CForEach' guards, recursively) — the
-- imagined apply queries these against the node's view.
outcomeCondReads :: Bindings -> [CookedOutcome] -> [[Sym]]
outcomeCondReads b outs = concat
  [ cookedReadAnchors (map (groundCookedCondition b) cs) ++ outcomeCondReads b os
  | CForEach cs os <- outs ]
```

`Prax.Engine` gains and exports `groundedDeltaAnchors` (new import: `symIsVar` from
`Prax.Sym`; `listToMaybe` from `Data.Maybe`; `evictionShadowNames` is already imported):

```haskell
-- | The insert\/delete anchor families one grounded action's outcomes can
-- touch — 'performAction''s effects, bounded statically per call by walking
-- the same cooked outcomes 'performAction' itself executes. @Nothing@ when
-- the effects cannot be bounded: an unresolvable 'CCall', or an insert whose
-- first segment is (or could ground to) @practice@ — such an insert may
-- bring an instance into being ('spawnedInstanceNames') and run that
-- practice's 'cpInits', arbitrary further outcomes this walk does not model.
-- The caller (the planner's prediction reuse) treats @Nothing@ as opaque: no
-- reuse at or below the node. Conservative by construction: 'CForEach'
-- bodies are included whether or not their guards would fire, with unbound
-- variables left as 'mayUnifySyms' wildcards; 'CCall' includes every case of
-- the resolved function, cycle-guarded like 'Prax.Relevance''s string-side
-- atom walk.
groundedDeltaAnchors :: PraxState -> GroundedAction -> Maybe [[Sym]]
groundedDeltaAnchors st ga = do
  cp <- Map.lookup (gaPracticeId ga) (cookedDefs st)
  ca <- listToMaybe [ a | a <- cpActions cp, caName a == gaActionId ga ]
  outcomeDeltaAnchors st [] (map (groundCookedOutcome (gaBindings ga)) (caOuts ca))

outcomeDeltaAnchors :: PraxState -> [String] -> [CookedOutcome] -> Maybe [[Sym]]
outcomeDeltaAnchors st visited = fmap concat . traverse go
  where
    go o = case o of
      CInsert toks
        | mightSpawn (map fst toks) -> Nothing
        | otherwise -> Just (map fst toks : evictionShadowNames toks)
      CDelete toks  -> Just (map fst toks : evictionShadowNames toks)
      CForEach _ os -> outcomeDeltaAnchors st visited os
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
    mightSpawn (n : _) = symIsVar n || n == intern "practice"
    mightSpawn []      = False
```

(`groundCookedOutcome` comes from `Prax.Cooked` — check Engine's existing imports and add
if missing. If `CookedOutcome`'s constructors are not in scope in Engine, they come with
`Prax.Types`.)

**Tests (RED-first — the new names don't exist yet, so compilation is the RED):**

- [ ] **Step 1: Write the failing tests.** In `test/Prax/RelevanceSpec.hs` add to the
  existing group:

```haskell
  , testCase "cookedReadAnchors walks every polarity, including subquery internals" $ do
      let conds = map cookCondition
            [ Match "a.X", Not "b.X"
            , Subquery "S" ["W"] [ Match "c.W.deed", Cmp Gte "N" "2" ]
            , Count "N" "S", Calc "M" Add "N" "1", Eq "X" "y"
            , Or [ [ Match "d.X" ], [ Absent [ Match "e.X" ] ] ]
            ]
          anchors = cookedReadAnchors conds
          want p = map intern (pathNames p) `elem` anchors
      assertBool "a.X read"        (want "a.X")
      assertBool "b.X (Not) read"  (want "b.X")
      assertBool "subquery inner read" (want "c.W.deed")
      assertBool "Or branch read"  (want "d.X")
      assertBool "Absent-in-Or read" (want "e.X")
      length anchors @?= 5

  , testCase "moverReadAnchors: scope, believes, death, affordances, desires — grounded to the pair" $ do
      let p = practice
            { practiceId = "eatery", roles = ["R"]
            , actions = [ action "[Actor]: eat"
                            [ Match "hungry.Actor" ]
                            [ ForEach [ Match "crumb.C" ] [ Delete "crumb.C" ]
                            , Insert "meal.Actor" ] ]
            }
          vocab = [ Desire "wants-food" (Want [ Match "hungry.Owner" ] 5) ]
          priya = character "priya"
          beth' = character "beth"
          st = setDesires vocab
                 (setCharacters [priya, beth'] (definePractices [p] emptyState))
          anchors = moverReadAnchors st priya beth'
          has s = map intern (pathNames s) `elem` anchors
      assertBool "believes family, actor+mover grounded"
        (has "priya.believes.desires.beth.D")
      assertBool "death mark" (has "dead.beth")
      assertBool "affordance condition, Actor:=beth" (has "hungry.beth")
      assertBool "ForEach guard read" (has "crumb.C")
      assertBool "desire condition, Owner:=beth" (has "hungry.beth")
      assertBool "NOT grounded to the predictor" (not (has "hungry.priya"))
```

  In `test/Prax/EngineSpec.hs` add:

```haskell
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
            , functions =
                [ Function "bless" ["Who"]
                    [ FnCase [] [ Insert "blessed.Who" ] ] ]
            }
          st = definePractices [p] emptyState
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
          assertBool "eviction shadow"      (has "coin.ada.Evicted" as)
          assertBool "grounded delete path" (has "stock.ada" as)
      anchorsOf "ada: enroll" @?= Nothing           -- spawn opacity
      case anchorsOf "ada: ritual" of
        Nothing -> assertFailure "resolvable Call must be bounded"
        Just as -> assertBool "Call-resolved insert, arg-grounded" (has "blessed.ada" as)
      anchorsOf "ada: chant" @?= Nothing            -- unresolvable Call

  , testCase "axiomHeads: fireable heads, lifted forms, the contradiction witness" $ do
      let axs = [ axiom [ Match "starving.X" ] [ "hungry.X" ] ]
          st = setAxioms axs emptyState
          has s = map intern (pathNames s) `elem` axiomHeads st
      assertBool "the head"        (has "hungry.X")
      assertBool "the lifted head" (has "obliged.Obligor.hungry.X")
      assertBool "the ⊥ witness"   (has "contradiction")
```

  (The eviction-shadow assertion matches `evictionShadowNames`' actual variable segment —
  it appends `intern "Evicted"`, so `has "coin.ada.Evicted"` is exact. `Function`/`FnCase`
  are plain record constructors from `Prax.Types`.)

- [ ] **Step 2: Run to verify RED.** `cabal test --test-options='-p "Relevance"'` and
  `-p "Engine"` must FAIL to compile (names not in scope).
- [ ] **Step 3: Implement** exactly the code above (Derive, Types, Engine, Relevance, plus
  the `groundNames` export if needed).
- [ ] **Step 4: GREEN.** `-p "Relevance"`, `-p "Engine"`, then the full suite once (count
  reported — 441 + new). Goldens and ViewInvariant are inside the suite; confirm green.
- [ ] **Step 5: Gates.** `cabal build 2>&1 | grep -i warning` empty; `hlint src test` "No
  hints"; existing grep-gates.
- [ ] **Step 6: Commit** `"Relevance/Engine: what a prediction reads, what an action touches, what axioms can write"`.

---

### Task 2: The planner reuses — and the A/B answers

**Files:**
- Modify: `src/Prax/Planner.hs`
- Test: `test/Prax/PlannerSpec.hs`
- Measurement: scratchpad only.

**Interfaces consumed:** Task 1's three names, verbatim. **Exports unchanged** —
`predictMove` keeps its exact signature and behavior (it IS the live path); only
`scoreActions`' internals change.

**Design.** New imports in Planner: `moverReadAnchors`, `mayUnifySyms` from
`Prax.Relevance`; `groundedDeltaAnchors` from `Prax.Engine`; `Sym` from `Prax.Sym`.
Replace `scoreActions` with:

```haskell
-- | One imagined path's accumulated effect on the pick's root state, as
-- anchor families with the derived-fact cone folded in: the moment any
-- extension feeds an axiom ('footprint'), every fireable head ('axiomHeads')
-- joins the delta — and stays, because heads are themselves in the
-- footprint. 'Nothing' is the opaque path: some applied outcome could not
-- be bounded ('Prax.Engine.groundedDeltaAnchors'), so nothing at or below
-- it may reuse. Spec: docs/specs/2026-07-13-v34-prediction-reuse.md.
type PathDelta = Maybe [[Sym]]

extendDelta :: PraxState -> PathDelta -> Maybe [[Sym]] -> PathDelta
extendDelta st (Just old) (Just new) =
  Just (old ++ new ++ [ h | feeds, h <- axiomHeads st, h `notElem` old ])
  where feeds = any (\n -> any (mayUnifySyms n) (footprint st)) new
extendDelta _ _ _ = Nothing

-- | Score each candidate by the imagined round it opens (best first; ties
-- broken by label for determinism). Within one pick, every prediction is
-- either the root state's — reused when the path delta provably cannot
-- reach anything that (actor, mover) prediction reads — or computed live,
-- exactly as before; the reused value is EQUAL to the live one (the spec's
-- soundness argument), so decisions are bit-for-bit unchanged.
scoreActions :: Int -> PraxState -> Character -> [(GroundedAction, Double)]
scoreActions depth st0 actor = go depth (Just []) st0
  where
    -- The root memo: each mover's step decision (scope gate + prediction)
    -- at the PICK's root state. Map values are lazy — a mover whose pairs
    -- never reuse never computes its root prediction.
    rootStep = Map.fromList
      [ (charName m, stepPredict st0 m) | m <- othersAfter st0 actor ]
    rootReads = Map.fromList
      [ (charName m, moverReadAnchors st0 actor m) | m <- othersAfter st0 actor ]
    stepPredict s m
      | inScope s actor m = predictMove s actor m
      | otherwise         = Nothing

    -- Reuse the root's decision when sound; live otherwise (opaque path,
    -- a mover the root never enumerated, or a delta/read intersection).
    predictAt :: PathDelta -> PraxState -> Character -> Maybe GroundedAction
    predictAt (Just delta) s m
      | Just rs <- Map.lookup (charName m) rootReads
      , not (any (\d -> any (mayUnifySyms d) rs) delta)
      = Map.findWithDefault (stepPredict s m) (charName m) rootStep
    predictAt _ s m = stepPredict s m

    go d delta st =
      sortOn (\(ga, s) -> (Down s, gaLabel ga))
        [ (a, valueAfter d
                (extendDelta st0 delta (groundedDeltaAnchors st a))
                (performAction st a))
        | a <- candidateActions st actor ]

    valueAfter d delta st1 = base + rest
      where
        base = fromIntegral (evaluateCooked st1 (cookedSelfWants st1 actor))
        rest
          | d <= 0    = 0
          | otherwise = othersScore + selfNext
          where
            (afterRound, afterDelta, othersScore) =
              foldl step (st1, delta, 0) (othersAfter st1 actor)
            step (s, dlt, acc) m = case predictAt dlt s m of
              Nothing -> (s, dlt, acc)
              Just ga ->
                let s'   = performAction s ga
                    dlt' = extendDelta st0 dlt (groundedDeltaAnchors s ga)
                in (s', dlt', acc + 0.5 * fromIntegral (evaluateCooked s' (cookedSelfWants s' actor)))
            selfNext = case go (d - 1) afterDelta afterRound of
              ((_, v) : _) -> 0.9 * v
              []           -> 0
```

Behavior identities to preserve, exactly: the old `step`'s
`| not (inScope s actor m) = (s, acc)` guard is `stepPredict`'s `otherwise` arm; the sort
key, discounts (0.5, 0.9), fold order (`othersAfter st1 actor`, the NODE's living cast),
and `pickAction` are untouched. A mover alive at the root but dead along the path is caught
by the read set (its `dead.<name>` anchor is in `moverReadAnchors`) — the death insert
intersects, so the pair recomputes live and `candidateActions` returns `[]`.

**Tests (in PlannerSpec, after the deadNow group):**

- [ ] **Step 1: Write the guard tests.** These pass BEFORE the change (today's planner
  computes everything live) and must still pass AFTER — they are the reuse mechanism's
  falsifiability net, and Step 4 verifies each one actually discriminates by mutation.

```haskell
  , testCase "prediction reuse: a base-fact delta that enables the mover is recomputed, not reused" $ do
      -- priya's "taunt" inserts beth's hunger — the gate fact of beth's
      -- believed desire. At the pick's ROOT beth has no motivated move
      -- (Nothing); after taunt she is predicted to eat, and the imagined
      -- meal is priya's own payoff. If the planner wrongly reused the
      -- root's Nothing (the taunt delta unifies beth's read set — both her
      -- affordance condition and her desire condition read hungry.beth —
      -- so it must NOT), taunting would score no better than idling and
      -- lose the label tie-break to "idle about". The pick is the witness.
      let p = practice
            { practiceId = "mess", roles = ["R"]
            , actions =
                [ action "[Actor]: taunt beth" [ Neq "Actor" "beth" ]
                    [ Insert "hungry.beth" ]
                , action "[Actor]: eat lunch" [ Match "hungry.Actor" ]
                    [ Insert "meal.Actor" ]
                , action "[Actor]: idle about" [] []
                ]
            }
          vocab = [ Desire "wants-food"
                      (Want [ Match "hungry.Owner", Match "meal.Owner" ] 5) ]
          priya = (character "priya")
            { charWants = [ Want [ Match "meal.beth" ] 10 ] }
          beth' = character "beth"
          st0 = setDesires vocab
                  (setCharacters [priya, beth'] (definePractices [p] emptyState))
          st1 = performOutcome (Insert "practice.mess.here") st0
          st  = performOutcome
                  (Insert "priya.believes.desires.beth.wants-food.heard.gossip") st1
      -- Sanity: the root prediction really is Nothing (beth unmotivated).
      predictMove st priya beth' @?= Nothing
      -- The pick sees through the taunt: enabling beth's meal beats idling.
      fmap gaLabel (pickAction 2 st priya) @?= Just "priya: taunt beth"

  , testCase "prediction reuse: a DERIVED-fact flip (the cone) is recomputed, not reused" $ do
      -- priya's "denounce" inserts only a believes fact; an axiom derives
      -- the regard beth fears; beth's amends (gated on the DERIVED fact
      -- only) is her motivated answer, and the apology is priya's payoff.
      -- The raw taunt... the raw denounce delta (priya.believes.beth.thief)
      -- unifies NOTHING beth's prediction reads directly — only the cone
      -- (delta feeds the axiom => its head regards.W.C.thief joins) reaches
      -- her fear and her amends gate. A cone-less implementation reuses the
      -- stale Nothing and bides; the correct one denounces. (The do-nothing
      -- label must sort BEFORE "denounce beth" — a mutation's score-tie
      -- falls back to the label order, and a do-nothing that sorted after
      -- would hand the tie to the very label the test asserts, making the
      -- guard vacuous. "bide time" < "denounce beth"; "idle about" is not.)
      let p = practice
            { practiceId = "court", roles = ["R"]
            , actions =
                [ action "[Actor]: denounce beth" [ Neq "Actor" "beth" ]
                    [ Insert "Actor.believes.beth.thief" ]
                , action "[Actor]: make amends"
                    [ Match "regards.V.Actor.thief" ]
                    [ Insert "recanted.Actor", Insert "apology.Actor" ]
                , action "[Actor]: bide time" [] []
                ]
            }
          axs = [ axiom [ Match "W.believes.C.thief", Not "recanted.C" ]
                        [ "regards.W.C.thief" ] ]
          vocab = [ Desire "hates-infamy"
                      (Want [ Match "regards.V.Owner.thief" ] (-8)) ]
          priya = (character "priya")
            { charWants = [ Want [ Match "apology.beth" ] 10 ] }
          beth' = character "beth"
          st0 = setDesires vocab
                  (setAxioms axs
                     (setCharacters [priya, beth'] (definePractices [p] emptyState)))
          st1 = performOutcome (Insert "practice.court.here") st0
          st  = performOutcome
                  (Insert "priya.believes.desires.beth.hates-infamy.heard.gossip") st1
      predictMove st priya beth' @?= Nothing
      fmap gaLabel (pickAction 2 st priya) @?= Just "priya: denounce beth"

  , testCase "prediction reuse: a mid-path death is recomputed (the dead anchor)" $ do
      -- The existing "mid-round death silences the rest of the imagined
      -- round" test already pins this behavior end-to-end; this case pins
      -- the read-set half directly: the death mark is in every mover's
      -- read anchors, so no reuse can survive a kill on the path.
      let priya = character "priya"
          beth' = character "beth"
          st = setCharacters [priya, beth'] emptyState
      assertBool "dead.beth is read"
        (map intern (pathNames "dead.beth") `elem` moverReadAnchors st priya beth')
```

  (For the third case import `moverReadAnchors`/`pathNames`/`intern` in PlannerSpec the
  same way RelevanceSpec does.)

- [ ] **Step 2: Run them against the UNCHANGED planner.** All three must PASS (today
  everything is computed live). This is the both-directions baseline, not the RED; the
  RED for this task is Step 4's mutations.
- [ ] **Step 3: Implement** the Planner change exactly as designed. Suite green, nets
  green (`-p "GoldenDrive"` byte-identical all four worlds, `-p "ViewInvariant"`).
- [ ] **Step 4: Mutation-verify the guards (the RED evidence).** Temporarily, in a scratch
  build (never committed):
  1. Make `predictAt` always reuse (`predictAt _ s m = Map.findWithDefault (stepPredict s m) (charName m) rootStep` with the guard dropped) — the taunt test AND the denounce test
     must FAIL (stale `Nothing` reused).
  2. Restore the guard but drop the cone (make `extendDelta`'s `feeds` always `False`) —
     the denounce test alone must FAIL (raw delta misses the derived flip), the taunt test
     still passes.
  Record both observed failures (test name + wrong pick) in the report, then revert to the
  real implementation and re-run to GREEN. This is the observed-RED for a change whose
  correct behavior is "nothing changes".
- [ ] **Step 5: The A/B (the round's acceptance).** Uncontended (no other `cabal`/`ghc`
  processes), best-of-3, the exact recorded 31-test filter:

```
cabal test prax-test --test-options='-p "/Prax.Worlds.Village/ && !/confessing to gale converts the mark/ && !/absolution inserts the defeater/ && !/a fresh whisper snaps the defeater away/ && !/incorrigibility: gale, now knowing two distinct instances/ && !/free-play preservation: eve does not confess/"'
```

  Report against the recorded epochs: 31.11s (pre-v32) / 171.64s (post-v32) / 132.75s
  (post-v33). Then the full suite once, timed (441+ tests, ~175s baseline). All numbers as
  measured, wherever they land.
- [ ] **Step 6: Gates** (warnings, hlint, grep-gates) and commit
  `"Planner: one pick, one set of predictions — reuse what the path provably cannot change"`.

---

### Task 3: Docs

**Files:**
- Modify: `docs/LEDGER.md`
- Modify: `README.md` only if it states planner cost characteristics that changed.

- [ ] LEDGER: v34 legend row — the branch statistics that motivated it (89ms/2.3s/44.5s
  depth split; 458-considered-1-taken; 4,014/4,014 observed prediction equality), the
  mechanism (root memo, path-delta anchors, the axiom-head cone, opacity rules), the
  soundness argument's one-line form, and the measured A/B (all four epochs). The v33 row
  gains a pointer ("the residual ~100s was the recursion re-deriving identical
  predictions; resolved in v34" — phrased to match the actual Task 2 measurement).
- [ ] Full gate recorded; commit `"Docs: v34 — the imagination stops repeating itself"`.
