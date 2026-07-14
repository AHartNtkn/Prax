# v35 — Intentions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** NPCs hold standing intentions and re-deliberate only on motive-signature change,
per `docs/specs/2026-07-13-v35-intentions.md` — an ACCEPTED SPEC CHANGE (user-gated):
decisions may in principle differ from always-deliberate; on current content the probes
measured zero drift, and the goldens are the arbiter.

**Architecture:** One new pure function (`motiveSignature`, Planner — beside the machinery
it reuses), one new runtime field (`intentions` on PraxState, the `cursor` precedent), one
consumer rewrite (`npcAct`, Loop). `pickAction` and everything beneath it untouched.

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- The semantics is the spec's, exactly: signature-compare at the character's own turn
  against their LAST DELIBERATION; equal ⇒ act the standing intention with no deliberation;
  else full `pickAction`, store intention + signature. No dual system, no flag — the
  always-deliberate path is replaced.
- The options component is FULL `GroundedAction` equality (not labels): hidden-binding
  drift must re-deliberate, so a stale grounding can never be acted. (Strictly finer than
  the probe's label-set trigger — coverage can only improve.)
- Goldens: expected byte-identical (the probes' 35/35). ANY golden diff = BLOCK with the
  full itemized drift — the controller adjudicates accept-as-semantics (re-capture in its
  own commit) vs defect. Never re-capture unilaterally. ViewInvariant must stay green.
- Zero warnings; hlint "No hints"; grep-gates; no re-exports.
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: The semantics — signature, store, loop

> **Amended mid-round:** the options component shipped as standing-validity + want-bearing templates — see spec §1 as amended (ac7e26a); the code below is the v1 baseline it replaced.

**Files:**
- Modify: `src/Prax/Types.hs` (two types + one `PraxState` field)
- Modify: `src/Prax/Planner.hs` (`motiveSignature`, exported)
- Modify: `src/Prax/Loop.hs` (`npcAct`)
- Test: `test/Prax/PlannerSpec.hs` (signature units), `test/Prax/LoopSpec.hs` (semantics
  pins — follow where npcAct behavior is pinned today; if LoopSpec is thin, the pins go
  there regardless, it is the loop's spec)

**Design.**

`Prax.Types`, beside `GroundedAction` (both types need its `Eq`):

```haskell
-- | The inputs to /wanting to reconsider/ (spec
-- @docs/specs/2026-07-13-v35-intentions.md@): what I can do (FULL grounded
-- candidates — label-invisible binding drift must count as change, so a
-- stale grounding is never acted), how I am doing (per-want satisfaction
-- counts, 'selfWants' order — counts, not their weighted sum, so two
-- profiles cannot mask each other), what is driving me (own live desires:
-- statically improvable and not dead-now), and what motives I know of
-- (believed-motive facts, the family "Prax.Minds" reads). Compared for
-- equality at the character's own turn against their last deliberation.
data MotiveSignature = MotiveSignature
  { msOptions      :: [GroundedAction]
  , msSatisfaction :: [Int]
  , msLiveDesires  :: [String]
  , msKnownMotives :: [(String, String)]  -- (other, desire name)
  }
  deriving (Eq, Show)

-- | A standing intention: the action chosen at the last deliberation (or
-- the choice to do nothing) and the motive signature it was based on.
data Intention = Intention
  { intentAct   :: Maybe GroundedAction
  , intentBasis :: MotiveSignature
  }
  deriving (Eq, Show)
```

`PraxState` gains, beside `cursor` (runtime loop state, NOT retable-maintained — record
updates preserve it):

```haskell
  , intentions :: Map String Intention
    -- ^ Each character's standing intention ('Prax.Loop.npcAct' — the
    -- reconsideration semantics, spec 2026-07-13-v35). Runtime state like
    -- 'cursor': starts empty (every character's first turn deliberates),
    -- never derived, never touched by 'Prax.Engine.retable'.
```

`emptyState` gets `intentions = Map.empty`.

`Prax.Planner` gains (and exports) — new import: nothing (everything is already in scope:
`candidateActions`, `cookedSelfWants`, `deadNow`, `queryCooked`, `childKeys` comes from
`Prax.Db` — extend the existing `Prax.Db` import with `childKeys`):

```haskell
-- | The character's current 'MotiveSignature' — cheap by construction:
-- grounding without scoring, four count\/existence walks against the cached
-- view, no lookahead. This is the whole cost of a quiet turn.
motiveSignature :: PraxState -> Character -> MotiveSignature
motiveSignature st c = MotiveSignature
  { msOptions      = candidateActions st c
  , msSatisfaction = [ length (queryCooked v cs Map.empty)
                     | (cs, _) <- cookedSelfWants st c ]
  , msLiveDesires  = [ desireName d
                     | d <- desires st
                     , desireName d `elem` charDesires c
                     , desireName d `elem` improvables st
                     , not (deadNow st c d) ]
  , msKnownMotives = [ (m, d)
                     | m <- childKeys (charName c ++ ".believes.desires") v
                     , d <- childKeys (charName c ++ ".believes.desires." ++ m) v ]
  }
  where v = readView st
```

`Prax.Loop.npcAct` becomes (new imports: `motiveSignature` from `Prax.Planner`,
`Data.Map.Strict` qualified if not already, `Intention (..)`/`MotiveSignature` come with
`Prax.Types`):

```haskell
-- | Have an NPC act: if their motive signature equals the one their standing
-- intention was based on, act that intention WITHOUT deliberating (spec
-- 2026-07-13-v35 — commitment is the default); otherwise deliberate in full
-- ('pickAction', unchanged), act the result, and store the new intention.
-- A standing action whose grounding is no longer offered cannot be acted:
-- the options component of the signature has necessarily changed.
npcAct :: Int -> Character -> PraxState -> (Maybe GroundedAction, PraxState)
npcAct depth actor st =
  case Map.lookup name (intentions st) of
    Just intent | intentBasis intent == sig -> act (intentAct intent) st
    _ ->
      let chosen = pickAction depth st actor
          st1 = st { intentions =
                       Map.insert name (Intention chosen sig) (intentions st) }
      in act chosen st1
  where
    name = charName actor
    sig  = motiveSignature st actor
    act (Just ga) s = (Just ga, performAction s ga)
    act Nothing   s = (Nothing, s)
```

**Tests.**

- [ ] **Step 1: signature units (RED — names missing), in PlannerSpec** after the deadNow
  group:

```haskell
  , testCase "motiveSignature: options, satisfaction, live desires, known motives" $ do
      let p = practice
            { practiceId = "mess", roles = ["R"]
            , actions =
                [ action "[Actor]: eat lunch" [ Match "hungry.Actor" ]
                    [ Insert "meal.Actor" ]
                , action "[Actor]: idle about" [] []
                ]
            }
          vocab = [ Desire "wants-food"
                      (Want [ Match "hungry.Owner", Match "meal.Owner" ] 5) ]
          beth' = (character "beth")
            { charWants   = [ Want [ Match "meal.beth" ] 10 ]
            , charDesires = ["wants-food"] }
          st0 = setDesires vocab
                  (setCharacters [beth'] (definePractices [p] emptyState))
          st1 = performOutcome (Insert "practice.mess.here") st0
          sigA = motiveSignature st1 beth'
      -- options: only the unconditional action grounds (no hunger yet)
      map gaLabel (msOptions sigA) @?= ["beth: idle about"]
      -- satisfaction: one own want, zero satisfying bindings
      msSatisfaction sigA @?= [0]
      -- live desires: wants-food is improvable but gated shut (no hungry.*
      -- insert in the vocabulary => environment gate; absent => dead-now)
      msLiveDesires sigA @?= []
      -- known motives: none believed yet
      msKnownMotives sigA @?= []
      let st2 = performOutcome (Insert "hungry.beth") st1
          sigB = motiveSignature st2 beth'
      -- the hunger fact opens the gate AND grounds the eat action
      map gaLabel (msOptions sigB) @?= ["beth: eat lunch", "beth: idle about"]
      msLiveDesires sigB @?= ["wants-food"]
      let st3 = performOutcome
                  (Insert "beth.believes.desires.carl.wants-food.heard.gossip") st2
      msKnownMotives (motiveSignature st3 beth') @?= [("carl", "wants-food")]
      -- and each of those three states carries a DIFFERENT signature
      assertBool "A /= B" (sigA /= sigB)
      assertBool "B /= C" (sigB /= motiveSignature st3 beth')
```

  (Adjust the options-order expectation to `possibleActions`' actual deterministic order if
  it differs — assert the exact observed order, both elements named.)

- [ ] **Step 2: the semantics pins (RED — behavior is new), in LoopSpec.** The quiet-keeps-
  intention pin constructs the accepted gap explicitly:

```haskell
  , testCase "a quiet character acts their standing intention — even when fresh deliberation would differ" $ do
      -- priya's goad pays off unless beth retaliates; beth's believed
      -- vengefulness is gated on a grudge FACT. Establish priya's intention
      -- (goad) while beth is harmless; then the grudge lands through an
      -- EXTERNAL event priya has not processed: none of priya's four
      -- signature components move (her options are unconditional, her want
      -- reads slapped.priya which is still absent, she has no own vocabulary
      -- desires, and she already believed beth vengeful before the grudge).
      -- Fresh deliberation now prefers waiting (goad -> predicted slap costs
      -- her more than the goad gains); the standing intention goads anyway —
      -- the spec's accepted one-beat lag, pinned as INTENDED.
      let p = practice
            { practiceId = "spat", roles = ["R"]
            , actions =
                [ action "[Actor]: goad beth" [ Neq "Actor" "beth" ]
                    [ Insert "goaded.beth" ]
                , action "[Actor]: slap priya"
                    [ Match "grudge.Actor", Match "goaded.Actor" ]
                    [ Insert "slapped.priya" ]
                , action "[Actor]: wait about" [] []
                ]
            }
          vocab = [ Desire "vengeful"
                      (Want [ Match "grudge.Owner", Match "slapped.priya" ] 8) ]
          priya = (character "priya")
            { charWants = [ Want [ Match "goaded.beth" ] 5
                          , Want [ Match "slapped.priya" ] (-20) ] }
          beth' = character "beth"
          st0 = setDesires vocab
                  (setCharacters [priya, beth'] (definePractices [p] emptyState))
          st1 = performOutcome (Insert "practice.spat.here") st0
          st  = performOutcome
                  (Insert "priya.believes.desires.beth.vengeful.heard.gossip") st1
      -- First turn: no standing intention -- deliberates, goads (beth
      -- harmless: no grudge, prediction Nothing), intention stored.
      let (a1, stA) = npcAct 2 priya st
      fmap gaLabel a1 @?= Just "priya: goad beth"
      -- Rewind the goad itself but keep the stored intention: rebuild the
      -- pre-goad state and graft the intentions map (the test needs the
      -- external event to be the ONLY difference).
      let stKept = st { intentions = intentions stA }
          grudged = performOutcome (Insert "grudge.beth") stKept
      -- Fresh deliberation WOULD now wait (goad invites a -20 slap):
      fmap gaLabel (pickAction 2 grudged priya) @?= Just "priya: wait about"
      -- ...but priya is quiet: all four components unchanged -- she goads.
      let (a2, _) = npcAct 2 grudged priya
      fmap gaLabel a2 @?= Just "priya: goad beth"

  , testCase "each trigger reconsiders: options, satisfaction, live drive, learned motive" $ do
      -- Four minimal worlds, one per component; in each: establish a standing
      -- intention, move ONLY that component, npcAct must deliberate afresh
      -- (observable: the pick changes to the newly-correct action).
      -- IMPLEMENTER CONTRACT: each arm is SELF-VERIFYING by construction --
      -- it asserts (a) the standing pick by label, (b) the post-change
      -- npcAct pick by label, AND (c) that (b) differs from (a). If an arm's
      -- fixture arithmetic fails to discriminate, assertion (b) or (c) fails
      -- loudly and the fixture must be redesigned (report the redesign);
      -- never weaken an assertion to make an arm pass (the v34 tie-break
      -- lesson: a guard that cannot fail is sabotage, not a test).
```

  For the four-trigger case, build each arm on the Step-1 fixture family (each arm's
  fixture follows the quiet pin's structure — full code written by the implementer under
  the self-verifying contract above; the arms' content is prescribed exactly):
  - **options**: after `Insert "hungry.beth"`, beth's next `npcAct` eats (her standing
    intention was idle; the eat candidate appearing re-deliberates).
  - **satisfaction**: give beth `charWants = [Want [Match "crumbs.C"] (-2)]` and an action
    "sweep" gated on crumbs; externally `Insert "crumbs.floor"` after her idle intention
    stands — the count vector moves 0→1, she re-deliberates and sweeps.
  - **live drive**: the Step-1 gate flip IS this trigger for an own vocabulary desire —
    externally insert the gate fact; she re-deliberates (distinct from options arm: use a
    desire whose gate fact does NOT enable a new action — e.g. eat gated only on a
    practice fact, hunger only in the WANT — so options stay equal and the live-set alone
    moves).
  - **learned motive**: externally insert `beth.believes.desires.carl.<d>...` where
    predicting carl changes beth's best move (reuse the v34 taunt shape: beth benefits
    from carl's predicted eat only if she knows he wants food).
- [ ] **Step 3: implement** exactly the design (Types, Planner, Loop).
- [ ] **Step 4: GREEN** on `-p "Planner"` and `-p "Loop"`; then the FULL suite. GoldenDrive
  and ViewInvariant are inside it. **If any golden differs: BLOCK immediately** — post the
  byte diff, per-turn itemization (first diverging turn, actor, standing-vs-fresh pick),
  and STOP for adjudication. Do not re-capture.
- [ ] **Step 5: the A/B** (uncontended, best-of-3, the exact 31-test filter recorded in
  `.superpowers/sdd/task-2-report.md` of v34): report against 31.11 / 171.64 / 132.75 /
  120.10–123.57. Then the timed full suite.
- [ ] **Step 6: gates** (zero warnings; hlint; grep-gates) and commit
  `"Loop: commitment is the default — NPCs reconsider on motive change, not on a clock"`.

---

### Task 2: Docs

**Files:** `docs/LEDGER.md`, `README.md` (only if a stated behavior is now false —
README's planner description may state that NPCs choose by scoring candidates each turn;
if so, amend to the intention semantics in one sentence).

- [ ] LEDGER: v35 legend row — the user's challenge verbatim-in-spirit, the three probes
  (91.5% ceiling / anchor road structurally exhausted with the chained-zero result / 35-35
  trigger coverage at 70% saved), the semantics (four components, each named as authored
  meaning), the accepted gap (one-beat lag on pure prediction-input change, pinned as
  intended), the goldens verdict as measured, the A/B as measured. The v34 row gains one
  sentence: the "banked levers" (below-instance bounding, per-head cones) were probe-tested
  in the v35 investigation and measured insufficient at the outer loop (chained cache
  served zero picks) — superseded by the semantics change, kept banked only for
  within-pick precision.
- [ ] Full gate; commit `"Docs: v35 — intentions"`.
