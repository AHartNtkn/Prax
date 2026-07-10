# v23 — Realistic lookahead: round-walk prediction over believed minds

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
predictor's *beliefs about* that character's mind, in turn order.** A pruning that makes the
model *more* realistic.

### Design history (second-round review)

Two simpler models were considered and rejected:

- A flat **secret/public flag** on wants — rejected because it makes every secret universally
  opaque: two NPCs could never coordinate in secret (the accomplice must anticipate the
  conspirator's move; the victim must not).
- **SecretWant with unlock sentences** (relational secrecy as bespoke machinery) — rejected on
  second-round review as a special case of something the system already has: *beliefs*. "Public
  want" just means "commonly believed want," so the general mechanism is to make others' minds
  an object of belief like everything else — witnessable, gossipable, liable to be lied about,
  derivable, defeasible, and sometimes wrong.

### Philosophical continuity (Rawls, via the paper)

The paper's own answer to combinatorial overwhelm is §VIII-A's **constitutive view of social
practices** — "first articulated explicitly by Rawls" (*Two Concepts of Rules*, Philosophical
Review LXIV, 1955; the paper's ref [20]): actions do not exist outside the practice (Rawls'
chess example), so "the agent is not overwhelmed by an infinite number of choices because he
only sees the affordances that are provided by the social practices he is in." We implement that
literally already (LEDGER #13/#14) — pruning layer one, and the v22 blowup happened *inside*
it. This round adds layer two in the same spirit: when predicting others, model them as the
paper's goal-pursuers within their practice-bounded options — one motivated move each, from the
mind you *believe* they have. (A third, banked layer is also continuous with the constitutive
view: limiting prediction to practices the predictor *shares* — you cannot anticipate moves in a
game you cannot see.)

## 1. The desire vocabulary (`Prax.Minds`)

To believe something about a mind, minds must be nameable. A world declares **named,
owner-parameterized desire templates**:

```haskell
-- | A nameable desire: a Want whose conditions may use the reserved variable
-- @Owner@, instantiated per character.
data Desire = Desire
  { desireName :: String
  , desireWant :: Want        -- conditions over the reserved variable Owner
  } deriving (Eq, Show)

wantFor :: String -> Desire -> Want   -- ground Owner := the character's name
```

- `PraxState` gains `desires :: [Desire]` (default `[]` — a world without a vocabulary is
  unchanged).
- `Character` gains `charDesires :: [String]` — the names of the vocabulary desires this
  character actually has. **Self-planning uses** `charWants ++ [wantFor me d | d named in
  charDesires]`.
- `charWants` (unnamed wants) remain, with new semantics falling out for free: **an unnamed
  want has no name to believe, so it is inherently unreadable** — never used in prediction.
  This is exactly right for the story manager's metalevel desires (nobody should foresee the
  director stirring a rivalry) and for idiosyncratic interior wants. Verified against the
  shipped worlds: their load-bearing behaviors are self-wants and self-chains (e.g. bex's norm
  avoidance is his own `Want [violationOf …] (-40)`, not a prediction of ada), so existing
  worlds keep their `charWants` unmigrated and behave identically — the vocabulary is adopted
  where theory-of-mind is the point.

## 2. Beliefs about minds

A motive-belief is an ordinary belief in the v20 provenance shape, over the issue
`desires.<owner>.<name>`:

```
P.believes.desires.cassia.kill-artus.seen          -- P inferred it from what P saw
P.believes.desires.cassia.kill-artus.heard.marcus  -- P was told, sourced
P.believes.desires.cassia.kill-artus.presumed      -- derived: convention/profession (§3)
```

Because the shape is the standard event-belief shape, **the whole information stack works on
minds with zero new machinery**:

- `gossip`/`lie` (v20/v22) take `desires.M.<name>` patterns as-is — motives can be told,
  corroborated, one-shot-per-teller, **and lied about** (framing someone's *intentions*).
- A **confide** is just an action inserting the sourced belief in the confidant.
- **Learning by observation** is the v21 pattern aimed at minds: *authored* inference axioms
  from evidence to believed motive (e.g. `Regarder.believes.stole.bob.loaf ⇒
  Regarder.believes.desires.bob.covets-bread.presumed`). Automatic inverse planning — inferring
  motives from actions without authored rules — is explicitly rejected as a heuristic swamp.
- `forget` retracts a motive-belief subtree like any other.

## 3. Common knowledge, recovered by derivation (not by flag)

"Public" was a flag pretending to be a primitive. It is recovered as **derived common
knowledge**, defeasible, via two axiom builders in `Prax.Minds` (worlds assert `character.<n>`
facts for their cast so the axioms can quantify — script worlds already do):

- `professed`: the fact `professes.<owner>.<name>` (this character's desire is openly held)
  derives `P.believes.desires.<owner>.<name>.presumed` for every character P.
- `conventional`: the fact `conventional.<name>` (a desire everyone is presumed to have —
  norm-respect, ordinary appetites) derives `P.believes.desires.M.<name>.presumed` for every
  pair of characters — **even where M does not actually have it**: you expect strangers to be
  conventional, and can be wrong. Mispredicting the unconventional is a feature of the model,
  not a bug.

Secrecy is now nothing at all: a desire that is neither professed nor conventional nor yet
believed by anyone is simply unknown. Sharing it, witnessing its exercise, or hearing of it are
all just facts arriving.

## 4. Semantics: the round-walk

`scoreActions depth st actor` scores each candidate action `a` of the actor:

1. `st₁ = performAction st a`; `score = evaluate st₁ (selfWants actor)`.
2. If `depth > 0`: walk the **other living characters within the actor's prediction scope,
   once each, in the loop's turn order starting after the actor** (the round-robin `cursor`
   order, skipping the dead). The scope is epistemic and world-supplied: `PraxState` gains
   `predictionScope :: CoPresence` (the v19 template vocabulary, over `Actor`/`Witness`;
   default `[]` = vacuously true = everyone, so scopeless worlds are unchanged). A mover `m`
   is in scope iff the template holds with `Actor` := the actor and `Witness` := `m` in the
   current simulated state. Worlds with places author the template as **"co-present now, or
   sighted within the world's horizon"** (§4a): you deliberate about the people you know are
   around — the room you are in, the colleague who just stepped out — while someone whose
   location you do not know **simply does not participate in your predictions**, and distant
   events reach you as arriving information to react to. This is the banked Rawlsian third
   layer in epistemic form, and it bounds per-node cost to local density instead of cast
   size. For each in-scope character `m` in sequence:
   - **Predict** `m`'s move with `predictMove st' actor m`: `m`'s best candidate action in the
     current simulated state, chosen myopically (depth 0) against **the actor's believed model
     of `m`** — `wantFor m d` for every vocabulary desire `d` such that the view satisfies the
     prefix `actor.believes.desires.m.<name>` (any provenance) — and only if that move
     **strictly improves** the believed model's evaluation over doing nothing (an unmotivated
     move is not a prediction: tie-break wandering is noise, not plan — a deliberate divergence
     from Praxish's always-predict). A mind the actor holds no beliefs about is modeled as
     still, not as helpful. Note the model can be **wrong**: prediction uses the actor's
     beliefs, not `m`'s true wants.
   - If a move is predicted, **perform it** (later predictions see earlier ones, as a real
     round would) and add `0.5 × evaluate (selfWants actor)` of the resulting state to `score`.
3. After the full round of others, recurse for the actor's own next move:
   `score += 0.9 × best (scoreActions (depth − 1))` over the actor's candidates in the
   simulated state (0 if none).

Notes:
- Accumulation is over **absolute momentary utilities** (a discounted utility stream over the
  simulated round), the reference's shape; candidates share the structure so the ranking is
  well-defined. Ties keep the deterministic label order.
- `depth` counts the actor's *own* future choices. The CLI/loop keep `lookaheadDepth = 2`.
- Others are predicted **myopically** (depth 0); deeper nested opponent models are out of scope.
- `worldValue` in its current form is **deleted** (no wrappers); API: `evaluate`,
  `candidateActions`, `scoreActions`, `pickAction`, plus
  `predictMove :: PraxState -> Character{-predictor-} -> Character{-mover-} -> Maybe GroundedAction`.

### 4a. Sightings: knowing where people are is itself information (`Prax.Sight`)

Locations are learned, not known. A **sighting** is an ordinary location-belief:

```
P.believes.at.M!<place>        -- best guess, single-slot (a new sighting overwrites)
P.believes.atSince.M!<turn>    -- when it was formed
```

maintained by a compiled, v18-`_clock`-idiom **ticker practice** (`Prax.Sight`): a bodiless
per-round character whose one silent action (i) advances a global turn counter `turn!N`
(single-slot; the first brick of the banked calendar/decay tier) and (ii) refreshes sightings
for every pair satisfying the world's co-presence template, via `ForEach` — deposits are
world-vocabulary, so the engine still knows nothing about places. Sightings persist after
separation: "last known location" stays queryable (the substrate future *search* behavior —
hunting an identified murderer toward where you last saw them — will be authored against).

The **horizon** — how long you assume people stay put — is an authored world parameter with
stated meaning, written directly into the world's `predictionScope` template in the existing
condition language (`Or` of co-present-now and
`believes.at`-fresh-within-horizon via `Calc`/`Cmp` over `turn!N`). It is not an engine
constant; a village might set it near a square↔mill round trip.

**Honest residual (banked):** scope participation is belief-limited, but an in-scope mover is
still *simulated at their true position*. Imagining them at your believed position requires
counterfactual placement — per-agent world-view machinery — banked alongside it.

## 5. Authored in shipped worlds, this round

- **Intrigue**: a one-entry vocabulary — `Desire "kill-artus" (Want [Match (deadSentence
  "artus")] 100)`... generalized as authored (`Owner`-parameterized where sensible). Cassia's
  motive moves from `charWants` to `charDesires = ["kill-artus"]`, neither professed nor
  conventional; the existing confide action additionally inserts
  `<Target>.believes.desires.cassia.kill-artus.heard.cassia`. Result, as tests: the confidant's
  `predictMove` of cassia is the poisoning; the victim's is not — and if the plot is *rumored*
  to artus (motive-gossip over the same pattern), his prediction changes, because a leak
  genuinely changes who can see the plan.
- **Prediction scopes wired** where worlds have place vocabularies: the village and the bar
  each run the `Prax.Sight` ticker and author their scope as co-present-or-recently-sighted
  (horizon an authored per-world parameter). Locationless worlds (feud, deontic fixtures,
  script worlds) keep the default everyone-scope and no ticker.
- Other worlds: otherwise unchanged this round (their behaviors are self-want-driven; verified
  above). The village's eve/bob secrets are marked when the parked v22 resumes on top.

### The honest residual

Base **facts** remain common knowledge inside predictions, exactly as the shared-by-default
model keeps them: a predicted move that is *believably motivated* but gated on a secret fact can
still leak. Documented, not hidden; per-agent world-views are the machinery Versu itself
declined to build. Believed *weights* are also template-fixed (you believe *that* bob covets
bread; the intensity comes with the concept) — per-observer intensities are out of scope.

## 6. Expected consequences (each becomes a test)

- **Speculative lying dies**: predicted moves are moves the predictor believes the mover wants —
  carol's whisper scores below honest options; the parked v22 stealth/frame-up beats become
  implementable as designed.
- **The murder is a surprise to the victim, not the accomplice** — and **NPCs coordinate in
  secret**: an accomplice whose payoff depends on the conspirator's next move takes the enabling
  action iff they hold the motive-belief.
- **Mispredictions are real**: a false planted motive-belief produces a predicted move the mover
  never takes.
- **Norms and the bar unchanged**: full behavioral suite green with no world edits outside
  Intrigue's §5 change.
- **Performance collapses to linear-per-ply**: one predicted move per other character. Master's
  suite stays in its time class; the true referee is the parked v22 village suite (621s →
  expected seconds) measured when v22 resumes.

## 7. Tests (TDD)

- `MindsSpec` (new): `wantFor` grounds `Owner`; `professed`/`conventional` axioms derive
  `.presumed` beliefs for the cast (and are defeasible — retract the profession, the presumption
  dissolves); the vocabulary is inert when empty.
- `PlannerSpec` (rewritten where it encoded the old arithmetic, e.g. `worldValue 1 … @?= 9.0`):
  the walk-up-to-order story still chosen at depth 1 (new expected numbers derived in comments);
  `predictMove` is belief-relative (confidant/victim pair); myopic; motivated-only (a believed
  mind with nothing to gain predicts still); sequential round (second prediction sees the
  first's effects); unnamed wants are unreadable (a director-like mover predicts still);
  misprediction (false belief → predicted move ≠ mover's actual pick); secret coordination
  (accomplice enables iff believing); **scope**: an out-of-scope mover is not predicted even
  under a held motive-belief, an in-scope one is; the empty scope predicts everyone.
- `SightSpec` (new): the ticker advances `turn!N`; co-presence refreshes `believes.at`/
  `atSince` (and a new sighting overwrites the old); after separation the sighting persists;
  a mover sighted within the horizon still participates in prediction from the next room;
  past the horizon they drop out ("anyone whose location is unknown simply doesn't participate");
  a never-sighted mover never participates.
- Motive-information stack smoke: `gossip` over a `desires.…` pattern spreads a motive-belief
  that flips a third party's `predictMove`; a `lie` plants a false one.
- Behavioral regression: full suite green — bar (norm avoidance, director), intrigue (plot still
  runs to betrayal unimpeded; new confide-belief wiring), play/audience goldens, feud, village
  v19–21 arcs. Any story change must be argued from this spec's semantics, never tuned into
  place. `prax check` clean everywhere (the new axioms pass the axiom analysis); `-Wall`/hlint
  clean.

## 8. Sequencing

v23 lands on committed master (v22 Task 1 is in; Task 2's working-tree content is parked on a
side branch first, and v22 resumes on top — its village suite then validates the exploit's death
and the runtime).
