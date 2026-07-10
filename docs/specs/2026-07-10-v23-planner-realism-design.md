# v23 — Realistic lookahead: round-walk prediction over public minds

## Provenance, stated honestly

The original research has **no multi-ply lookahead**: Versu's published selection is reactive —
apply the candidate, evaluate the resulting state against desires, undo (paper §IX; notes p.113,
p.122). Lookahead is a *beyond-source extension* our v1 adopted from Praxish, and our port
diverged from even Praxish's design: `worldValue` maxes over **every living character's every
available action, scored by the planning actor's own wants**. That model is indefensible on the
merits and now demonstrably harmful, so this round **redesigns our extension** (it does not
"restore faithfulness" — there is no source lookahead to be faithful to). Praxish's round-walk is
adopted as corroborating precedent on the merits, not as authority.

Three observed failures of the current model (all first-hand, in the v22 record —
`.superpowers/sdd/task-2-report.md` and the session ledger):

1. **Speculative lying.** Carol's top depth-2 action was whispering an accusation she had no
   evidence for (4.75 vs 2.25 for honesty), because the lookahead credited her with futures in
   which *other* characters take actions they would never choose (bob stealing in front of her —
   forbidden by his own concealment want; strangers lying to her for no gain).
2. **Omniscient prediction.** Lookahead simulates others' moves from their *true* wants, so a
   secretly-planned murder is foreseeable by characters who could have no clue of the plan — a
   victim could preemptively act against a murderer nobody suspects.
3. **Combinatorial cost.** Max-over-everyone's-everything explodes with the action space: the
   v22 village suite regressed 8.7s → 621s. Praxish's own comments name this exact hazard and
   chose single-prediction-per-actor to avoid it.

One redesign addresses all three: **predict each other character's single next move, from their
public mind, in turn order.** It is a pruning that makes the model *more* realistic.

## 1. Semantics: the round-walk

`scoreActions depth st actor` scores each candidate action `a` of the actor as follows:

1. `st₁ = performAction st a`; `score = evaluate st₁ (allWants actor)`.
2. If `depth > 0`: walk the **other living characters once each, in the loop's turn order
   starting after the actor** (the round-robin `cursor` order `Prax.Loop` uses, skipping the
   dead). For each other character `m` in sequence:
   - **Predict** `m`'s move: `m`'s best candidate action in the *current simulated state*,
     chosen myopically (depth 0) against **`m`'s public wants** (§2) — and only if that move
     **strictly improves** `m`'s public evaluation over doing nothing (doing nothing is always
     available, so an unmotivated move is not a prediction; this deliberately refuses to treat
     tie-break wandering as intent, a divergence from Praxish's always-predict which would
     model noise as plan). An unreadable mind — no candidates, or only secret motives — is
     modeled as still, not as helpful.
   - If a move is predicted, **perform it** (the simulation advances — later predictions see
     earlier ones, as a real round would) and add `0.5 × evaluate (allWants actor)` of the
     resulting state to `score`.
3. After the full round of others, recurse for the actor's own next move:
   `score += 0.9 × best (scoreActions (depth − 1))` over the actor's candidates in the
   simulated state (0 if none).

Notes:
- Accumulation is over **absolute momentary utilities** (a discounted utility stream over the
  simulated round), the reference's shape; candidates are compared under the identical structure
  so the ranking is well-defined. Ties keep the deterministic label order.
- `depth` counts the actor's *own* future choices (each own-ply includes one full predicted
  round of others). The CLI/loop keep `lookaheadDepth = 2`.
- Others are predicted **myopically** (depth 0). Deeper opponent models are out of scope.
- `worldValue` in its current form is deleted (no wrappers, no compatibility); `evaluate`,
  `candidateActions`, `scoreActions`, `pickAction` remain the API, plus a new exported
  `predictMove :: PraxState -> Character -> Maybe GroundedAction` (the §2 prediction, testable
  in isolation).

## 2. Public and secret minds

`Character` gains a second want list:

```haskell
data Character = Character
  { charName        :: String
  , charWants       :: [Want]   -- public: what others can sensibly expect of you
  , charSecretWants :: [Want]   -- private: motives nobody else can see
  , charBoundTo     :: Maybe String
  }
```

- **Planning yourself** uses `charWants ++ charSecretWants` (you know your own mind).
- **Being predicted** uses `charWants` only.
- **Default is public** (`charSecretWants = []`, the `character` constructor unchanged in
  shape) — matching the source's epistemics ("world state shared by default; divergence
  opt-in"). Norms and conventional desires stay predictable *because they are public*: bex still
  avoids stiffing ada because ada's disapproval-want is common knowledge — which is what makes
  it a norm.
- Authored secrecy in shipped worlds, this round: **cassia's murder motive**
  (`Want [Match (deadSentence "artus")] 100` moves to `charSecretWants`) — the user's example,
  made real: nobody's lookahead foresees the poisoning, so nobody preempts a murderer they have
  no clue about. (The village's eve/bob secrets belong to the parked v22 content and are marked
  when v22 resumes.)

### The honest residual

Base **facts** remain common knowledge inside predictions, exactly as the shared-by-default
model keeps them: a predicted move that is *publicly motivated* but gated on a secret fact can
still leak. Documented, not hidden; per-agent world-views are the machinery Versu itself
declined to build.

### Banked (explicitly future)

**Relational secrecy** — a confidant *should* predict the plot (marcus, once confided, knows
cassia's intent). "Secret-unless-believed": prediction of a secret want unlocked when the
predictor holds a corresponding belief. Joins naturally with the belief vocabulary; its own
design round. Recorded in the LEDGER backlog.

## 3. Expected consequences (each becomes a test)

- **Speculative lying dies**: predicted moves are moves the mover would pick for their own
  (public) reasons — carol's whisper scores below honest options; the v22-parked stealth and
  frame-up beats become implementable as designed.
- **The murder stays a surprise**: `predictMove` of cassia with the motive secret ≠ the poison
  move; made public, it is the poison move (the minimal bug-demonstrating pair).
- **Norm avoidance survives**: bar/deontic/feud/village behavioral suites green.
- **Performance collapses to linear-per-ply**: one predicted move per other character instead of
  all moves of everyone. Master's suite must stay in its current time class; the real referee is
  the parked v22 village suite (621s → expected seconds) measured when v22 resumes.

## 4. Tests (TDD)

- `PlannerSpec` (rewritten where it encoded the old arithmetic, e.g. the `worldValue 1 … @?= 9.0`
  case): evaluate unchanged; the walk-up-to-order story still chosen at depth 1 under round-walk
  scoring (with the new expected numbers, derived in the test's comments); `predictMove` uses
  public wants only (secret/public murder pair); prediction is myopic; the round advances
  sequentially (a fixture where the second predicted mover's options depend on the first's move);
  unreadable minds are inert (a character with only secret wants is predicted to do nothing).
- Behavioral regression: the full suite green — bar (norm avoidance, director), intrigue (plot
  runs to betrayal when unimpeded), play/audience goldens, feud, village (v21 arc: spread,
  notoriety, atonement, deterrence, relent). Where a spec-level story changes, the change must be
  argued from this spec's semantics — never tuned into place.
- `prax check` clean everywhere; `-Wall`/hlint clean.

## 5. Sequencing

v23 lands on committed master (v22 Task 1 is in; Task 2's working-tree content is parked on a
side branch first, and v22 resumes on top of v23 — its own tests then validate the exploit's
death and the runtime).
