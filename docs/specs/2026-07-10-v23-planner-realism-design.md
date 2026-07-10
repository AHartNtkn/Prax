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

One redesign addresses all three: **predict each other character's single next move, from the
part of their mind you can see, in turn order.** It is a pruning that makes the model *more*
realistic.

### Philosophical continuity (Rawls, via the paper)

The paper's own answer to combinatorial overwhelm is §VIII-A's **constitutive view of social
practices** — "first articulated explicitly by Rawls" (*Two Concepts of Rules*, Philosophical
Review LXIV, 1955; the paper's ref [20]): actions do not exist outside the practice (Rawls'
chess example), so "the agent is not overwhelmed by an infinite number of choices because he
only sees the affordances that are provided by the social practices he is in." We implement
that literally already (LEDGER #13/#14) — it is pruning layer one, and the v22 blowup happened
*inside* it. This round adds layer two in the same spirit: when predicting others, model them as
the paper's goal-pursuers within their practice-bounded options — one motivated move each, from
the mind you can actually see — where "the mind you can see" is itself relational (§2). (A
third, banked layer is also continuous with the constitutive view: limiting prediction to
practices the predictor *shares* — you cannot anticipate moves in a game you cannot see.)

## 1. Semantics: the round-walk

`scoreActions depth st actor` scores each candidate action `a` of the actor as follows:

1. `st₁ = performAction st a`; `score = evaluate st₁ (allWants actor)`.
2. If `depth > 0`: walk the **other living characters once each, in the loop's turn order
   starting after the actor** (the round-robin `cursor` order `Prax.Loop` uses, skipping the
   dead). For each other character `m` in sequence:
   - **Predict** `m`'s move: `m`'s best candidate action in the *current simulated state*,
     chosen myopically (depth 0) against **the part of `m`'s mind readable by the actor** —
     `m`'s public wants plus any secrets the actor is in on (§2) — and only if that move
     **strictly improves** that readable evaluation over doing nothing (doing nothing is always
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
  `predictMove :: PraxState -> Character -> Character -> Maybe GroundedAction`
  (predictor → mover; the §2 prediction, testable in isolation).

## 2. Public and secret minds — and who is in on the secret

A flat secret/public split is insufficient: it would make every secret universally opaque, so
two NPCs could never **coordinate in secret** (an accomplice must anticipate the conspirator's
move; the victim must not). Secrecy is *relational* — secret from most, shared with some — so
the model is **secret-unless-in-on-it**:

```haskell
data Character = Character
  { charName        :: String
  , charWants       :: [Want]         -- public: what anyone can sensibly expect of you
  , charSecretWants :: [SecretWant]   -- motives readable only by those in on them
  , charBoundTo     :: Maybe String
  }

data SecretWant = SecretWant
  { secretKnownBy :: Maybe String   -- ^ unlock sentence over the reserved variable
                                    --   @Knower@ (e.g. @"confided.cassia.Knower"@,
                                    --   @"conspirator.Knower"@); 'Nothing' = nobody
  , secretWant    :: Want
  }
```

- **Planning yourself** uses `charWants ++ map secretWant charSecretWants` (you know your own
  mind, unlocks irrelevant).
- **Predictor P predicting mover M** uses M's `charWants` **plus** every `SecretWant` of M whose
  unlock sentence, with `Knower` bound to P's name, holds in the current (view of the) state.
  `predictMove` therefore takes the predictor: `predictMove :: PraxState -> Character{-predictor-}
  -> Character{-mover-} -> Maybe GroundedAction`.
- **Default is public** (`charSecretWants = []`; the `character` constructor unchanged in
  shape) — the source's epistemics ("shared by default; divergence opt-in"). Norms stay
  predictable *because they are public*: bex still avoids stiffing ada because ada's
  disapproval-want is common knowledge — which is what makes it a norm.
- **The unlock is an ordinary sentence, so being in on a secret is world state** — insertable by
  a confide action, a faction membership (`conspirator.Knower` gives *group* secrets), or even
  the information stack itself: if the plot is witnessed or *rumored to you* (v19/v20 facts),
  an unlock written over those sentences means a leak genuinely changes who can see the plan —
  preemption becomes possible exactly when, and only when, the preemptor has a clue.
- Authored in shipped worlds, this round: **cassia's murder motive** becomes
  `SecretWant (Just "confided.cassia.Knower") (Want [Match (deadSentence "artus")] 100)` (the
  unlock written over Intrigue's actual confide vocabulary — exact sentence pinned in the plan):
  her confidant can anticipate the poisoning; the victim, with no clue, cannot. (The village's
  eve/bob secrets belong to the parked v22 content and are marked when v22 resumes.)

### The honest residual

Base **facts** remain common knowledge inside predictions, exactly as the shared-by-default
model keeps them: a predicted move that is *publicly motivated* but gated on a secret fact can
still leak. Documented, not hidden; per-agent world-views are the machinery Versu itself
declined to build.

## 3. Expected consequences (each becomes a test)

- **Speculative lying dies**: predicted moves are moves the mover would pick for their own
  (public) reasons — carol's whisper scores below honest options; the v22-parked stealth and
  frame-up beats become implementable as designed.
- **The murder is a surprise to the victim, not the accomplice**: `predictMove` of cassia is
  the poison move for a predictor the unlock admits (the confidant) and NOT for one it doesn't
  (the victim) — the minimal relational pair. And **NPCs can coordinate in secret**: an
  accomplice whose own payoff depends on the conspirator's next (secret) move takes the enabling
  action iff they are in on the secret.
- **Norm avoidance survives**: bar/deontic/feud/village behavioral suites green.
- **Performance collapses to linear-per-ply**: one predicted move per other character instead of
  all moves of everyone. Master's suite must stay in its current time class; the real referee is
  the parked v22 village suite (621s → expected seconds) measured when v22 resumes.

## 4. Tests (TDD)

- `PlannerSpec` (rewritten where it encoded the old arithmetic, e.g. the `worldValue 1 … @?= 9.0`
  case): evaluate unchanged; the walk-up-to-order story still chosen at depth 1 under round-walk
  scoring (with the new expected numbers, derived in the test's comments); `predictMove` is
  relational (the confidant/victim pair over one secret want; an unlock over a group sentence
  admits several); secret coordination (accomplice enables iff in on it); prediction is myopic;
  the round advances
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
