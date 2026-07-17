# prax

A Haskell reimplementation of **Versu** (Richard Evans & Emily Short's simulationist
storytelling system), building up from **Praxish** (Max Kreminski's JavaScript reconstruction
of Versu's *Praxis* language).

The engine models a social world as a set of sentences in **exclusion logic**, with **social
practices** that offer role-based affordances and **autonomous agents** that choose actions by
utility. See the design writeups:

- `docs/research/versu-notes.md` — Versu architecture, from the 2014 IEEE paper.
- `docs/research/praxis-praxish-notes.md` — exclusion logic + the Praxish reference impl.
- `docs/LEDGER.md` — the full feature ledger (what's built, planned, and research-blocked).

## Status

**v1 (faithful engine core) complete.** The engine runs a playable bar storyworld end-to-end.

- `Prax.Db` — exclusion-logic trie: `insert` (with the **corrected `!` semantics**, fixing a
  data-loss bug in Praxish's `db.js`), `retract`, `unify`, `ground`.
- `Prax.Query` — typed condition language (match, negation, eq/neq, numeric compares, calc,
  count, subqueries).
- `Prax.Types` / `Prax.Engine` — practices/actions/outcomes as an eDSL; action discovery,
  execution, practice spawning + `init`, and guarded function calls.
- `Prax.Planner` — utility-based reactive selection: per-character wants, apply-and-evaluate,
  discounted lookahead.
- `Prax.Loop` / `Prax.Worlds.Bar` / CLI — round-robin turns and a menu-driven bar demo.
- `Prax.Core` (v2) — the Versu core model: relationships (numeric, asymmetric role evaluations).
  Feelings (`Prax.Emotion`, v38) coexist as multi-valued episodic facts and fade on a clock,
  replacing the original single-slot mood. Wired into the bar so interaction changes feelings and
  feelings change behaviour through pricing, not gating (a warmth-gated "buy a drink" affordance;
  a snub prices annoyance that makes a cross bartender reluctant to pour, not unable to).
- `Prax.Reactions` (v3) — reactions-as-practices and norms: an action spawns a reaction practice
  offering responses (greet back / rebuff / take offense), and a norm violation (stiffing the
  bartender) spawns disapproval. NPCs avoid violations because the planner scores the
  violation→disapproval future poorly. The mechanism (spawn/consume/violation-marking) ships no
  content of its own; the ready-made `disapproval` reaction lives with its sole consumer,
  `Prax.Worlds.Bar` (**v48** — `Prax.Reactions` no longer holds shipped world content).
- `Prax.Beliefs` (v4) — per-agent beliefs that can diverge from the shared world. A rumour plants a
  possibly-false belief; a character who believes someone resents them won't be friendly to them
  even when their actual warmth is high; evidence can dispel the belief.
- `Prax.Conversation` (v5) — conversations with a selected speaker, turn-taking, and topics. Quips
  are dialogue lines whose effects flow through the core model and beliefs (small talk, compliments,
  and gossip that plants a belief). Friends strike up chats on their own once warm.
- **Story manager (v6)** — the bar's `director` is Versu's Drama Manager: an autonomous agent with
  no body and only *metalevel* desires. It watches for a too-cosy room and injects a falling-out
  between two friends, then lets the autonomous cast play it out — "the DM is just a particular
  type of practice."
- `Prax.Arc` (v7) — character arcs: a character's internal high-level state (hopeful → belonging /
  lonely) that gates its wants, so advancing the arc reshapes what it pursues. The against-desires
  transformation (giving up) is offered to everyone but never taken by the utility planner — so
  "true transformation" is, in practice, the player's alone.
- First-order query connectives (v8) — `Or`/`Absent`/`Exists` + `forAll`/`implies` in `Prax.Query`,
  so preconditions and desires can be disjunctive/quantified.
- Cast removal + `Prax.Worlds.Intrigue` (v9) — a character can die and leave the cast; a branching
  dramatic episode verifies Versu-style drama end-to-end.
- `Prax.Inspect` + `Prax.Stress` (v10) — QA tooling: an inspector that explains why an action is
  unavailable (`explain`), and a seeded stress-tester that plays many random all-AI games and
  reports endings, action coverage, and dead ends (`cabal run prax -- stress [world]`).
- `Prax.Persist` (v11) — save/load a session (the world state is all facts). In play, press `s` to
  save; `cabal run prax -- <world> resume` continues from the save.
- `Prax.Script` + `Prax.Worlds.Play` (v12) — a **Prompter-lite** scene-authoring layer. Drama is
  written as a screenplay — a `CAST` plus a graph of `scene`s, each with a body of `beat`s
  (dialogue/affordances) and `junction`s (labelled routes that end the story or hand off to the
  next scene) — and `compile`d to ordinary practices plus **one silent engine schedule rule**
  that fires junctions/endings at the round boundary (**v46**: replacing a bodiless *narrator*
  character that used to take a fake turn each round to do the same thing — fiction surfaces
  through characters' actions, and scheduling is not one). `flowChart` renders the scene graph
  (`cabal run prax -- flow`), and the stress-tester reports scene coverage. `Prax.Worlds.Play` is
  a *faithful* recasting of `Prax.Worlds.Intrigue` — same cast, affordances (confide, poison,
  warn, self-poison, romance), and endings — as a two-scene play in ~25% fewer authored lines,
  *plus* the scene transition and flow-chart the layer supplies for free. The scene layer also
  compiles **timed junctions** (`after`/`timeout` — a scene transition/ending after N turns, via
  a passive scene clock) and **character sketches** (`concernedWith` turns concerns into desires;
  `withTraits` records personality as queryable facts) from Prompter's authoring constructs
  (v18). (Prompter's **memories** — one-shot exposition fired the first time a trigger holds —
  were also built at v18 but REMOVED at v46: omniscient narration with no speaker is a
  presentation feature, not world content, so it isn't compiled at all rather than re-homed.)
  Scene *bounds* are subsumed by scene-local beats, and the readable text playtext is
  intentionally replaced by JSON. `Prax.Worlds.Audience` (`prax audience`) exercises both
  remaining constructs in one short scene: an ambitious Duke's *concern* for standing makes him
  court the king unbidden, and the audience times out (`dismissed`) if you don't press your
  petition (`granted`) in time.
- `Prax.Script.Json` (v12) — play-scripts round-trip through **readable JSON** (`Prax.Script.Json`),
  the editable authoring/exchange format (chosen over maintaining a bespoke `.prompter` grammar).
  `cabal run prax -- dump-play` prints the built-in play as JSON; `cabal run prax -- play
  examples/play.json` loads and plays an edited one. See `examples/play.json`.
- **Player as DM (v13)** — the human can occupy Versu's drama-manager slot instead of a character.
  In `barDirectorWorld` (`prax dm`) you are the *director*: bound to a metalevel `direct` practice,
  your menu is authorial nudges — stir a rivalry, kindle warmth, cast a pall — over an autonomous
  cast (ada, bex, cai), who then play out the consequences through the ordinary social machinery.
  (A practice-bound player is offered only its practice's affordances, via `candidateActions`.)
- `Prax.Deontic` (v14) — a first-class **deontic "should"/obligation** layer, grounded in Evans'
  *Exclusion Logic as a Deontic Logic* (DEON 2010; distilled in `docs/research/deon-notes.md`). An
  obligation `□φ` is just the fact `obliged.<who>.<φ>` (the paper's `Ob:φ` sugar — no new
  semantics); norm **conflict** is detected via the `!` exclusion (incompatible duties collapse to ⊥),
  resolved *emergently* by the utility planner, and **contrary-to-duty** is the iterated `□□`
  (a reparative duty after a breach). The bar's "settle up" is now a real obligation: serving raises
  it, tipping discharges it, stiffing breaches it and creates a duty to make amends.
- `Prax.EL` + `Prax.Derive` (v15) — a **forward-chaining derivation layer** for emergent worlds.
  Domain rules (`body → head`) are closed to a fixpoint via the DEON paper's `m(X)` construction over
  a faithful Exclusion-Logic lattice (`Prax.EL`: `meet`/`leq`, exact `⊥` on contradiction). Reads go
  through a **defeasible closed view** (`readView`): derivations are recomputed from the base, never
  persisted, so retracting a premise dissolves its conclusions — and it is opt-in (`axioms = []`
  leaves a world untouched). Domain rules **auto-lift under `□`**, giving obligation-closure for free.
  `Prax.Worlds.Feud` (`prax feud`) is the demo: from *one* authored wrong and a handful of rules, a
  whole feud emerges (people who never met come to resent someone through the alliance network) and
  dissolves the moment amends are made. (v31 folds a fourth rule in — see `Prax.Faction` below —
  replacing the demo's hand-authored alliance ties with derived house membership, unmodified-tests
  proving the generalization holds.)
- `Prax.TypeCheck` (v16–17) — a static **type checker** (`prax check [world]`), Versu's implicit type
  system. The declaration-free, sound layer flags **unbound variables** (an outcome/axiom-head variable
  no precondition can bind — a silent no-op), **exclusion-cardinality clashes** (a relation asserted both
  `!` and `.`), and **dangling references** (`Call`/spawn of something undefined). On top, **ML-style
  sort inference**: declare each base sort's members (`sorts` on a world — the bar declares
  `beverage`/`place`/…), and it infers every position's and variable's sort by unification and rejects
  conflicts (a gender where an agent goes). Every shipped world checks clean. Like any type system it is
  conservative — you declare only the monomorphic positions you want checked.
- `ForEach` + `Prax.Witness` (v19) — **quantified outcomes**: `ForEach [Condition] [Outcome]` applies
  its sub-outcomes to *every* binding of its conditions (a snapshot taken at entry, so mutating for
  one binding never changes which others exist) — the dual of v8, which quantified conditions but
  left outcomes singular. `Prax.Witness` compiles it into **authored observability**: `observable`
  declares an action's public appearance, and every co-present non-actor comes to believe it
  happened (`<W>.believes.<event>.seen`, the `.seen` edge recording direct-observation provenance;
  multi-valued from v20, below, so it coexists with hearsay evidence for the same event).
  Observability is a semantic property the author states — undeclared actions (like moving) deposit
  no belief, preserving the looks-like/is gap deception will later exploit. `Prax.Worlds.Village`
  demonstrates it: bob steals a loaf in the square; carol and you, both present, come to believe it
  and can confront him; dana, at the mill, can't.
- `Prax.Rumor` (v20) — **sourced rumor propagation**: `gossip` (authored per event-pattern, like
  `observable`) lets a character tell a co-present hearer what they have evidence for, planting the
  same event-belief with hearsay provenance (`.heard.<teller>`, one edge per source) beside any
  `.seen` edge — provenance is now **multi-valued**, so witnessing and hearsay for the same event
  coexist and evidence accumulates instead of one overwriting the other, and `heard` (a boolean ∃
  over sources) makes corroboration countable. Spreading is want-driven, not automatic: a
  gossip-inclined character is authored with a want that others know what it knows, and the
  ordinary planner carries the news. `Prax.Worlds.Village` grows: carol, having witnessed bob's
  theft, carries the news on her own; hearsay licenses suspicion (`eye … with suspicion`, a milder
  trust hit) but never confrontation — that stays eyewitness-only — and a world-authored
  relationship gate lets distrust close the gossip channel.
- `Prax.Repute` (v21) — **derived reputation**: nobody *stores* a standing. `regards.<observer>.
  <subject>.<label>` is an axiom-derived fact, read straight off an observer's evidence (seen or
  heard alike), so it inherits information asymmetry for free and dissolves the instant its
  support does. `standingUnless` defeats it with a *base-fact* defeater rather than deleting the
  belief — atonement, not amnesia, so the memory of the deed survives even as the standing it once
  supported disappears; committing the deed again simply revokes the defeater, snapping standing
  back from memory nobody lost. `notoriety` turns corroboration into a threshold-gated fact — an
  authored world parameter (`notoriety "thief" 3` means "the whole village knows") — and it
  *drives* behaviour: bob's shame is keyed on being the village's *notorious* thief, not on any
  one person's contempt. `Prax.Worlds.Village` closes its arc on this: theft → witnessing → rumor
  → three regards → notoriety tips bob into returning the loaf → the village relents, memory
  intact throughout — and because the planner can see standing snap back on a repeat, an atoned
  bob is *deterred*, leaving a restocked stall untouched for the rest of the run.
- `Prax.Deceit` (v22) — **secrets and deception**: concealment is authoring, not machinery —
  `conceal` is a want that nobody believe some deed (`Absent [Anyone believes <event>]`); the
  planner's lookahead already simulates the v19 witness deposits, so an agent who values a secret
  avoids being seen *by planning* — waiting for the room to empty falls out of utility, no stealth
  system built or needed. A lie (`lie`) is an assertion without evidence: it mirrors `gossip`
  (v20) and plants the identical `.heard.<liar>` hearsay, so a fabrication is indistinguishable
  from truth to everyone but the liar — the deceived hold real evidence, and if the liar ever
  hears their own lie back, the lie action vanishes and plain gossip takes its place, seamlessly.
  The whole rumor/reputation stack (v20/v21) cascades on a falsehood exactly as it would on the
  truth. `Prax.Worlds.Village` gains a villain: bob conceals his theft, waiting out a genuinely
  watched square (the bread is safe exactly as long as *someone* minds it — a player walking away
  alone isn't enough, since carol keeps her own post); eve, out of authored malice, frames carol,
  and the frame-up settles into regard, shunning, and notoriety exactly as truth would — with an
  honest injustice at the end: framed carol has no recourse (amends needs a loaf she never took,
  and exculpation needs ground-truth event records this vocabulary deliberately doesn't have). The
  player gets the same whisper affordance eve does.
- `Prax.Minds` + `Prax.Sight` + a rewritten `Prax.Planner` (v23). The old lookahead's
  `worldValue` — max over every living character's every
  action, scored by the *planning actor's own wants* — is **deleted**: it turned out to be
  speculative (credited others with actions they'd never take — carol's top move became an
  accusation she had no evidence for), omniscient (used movers' *true* wants, so a
  secretly-planned murder was foreseeable by anyone), and combinatorially explosive (the village
  stress suite regressed 8.7s → 621s). It's replaced by a round-walk: each other character
  within the actor's epistemic *scope* (`predictionScope`, world-authored, default everyone)
  gets one myopic, motivated move predicted from the actor's *believed* model of them
  (`predictMove`) — never their true one — in cast order, before the actor recurses on its own
  next choice. A mind the actor holds no belief about, or a mover it can't currently place, sits
  still, never helpfully. Desires are now nameable and believable: `Prax.Minds` gives worlds
  owner-parameterized `Desire` vocabularies (`charDesires`), and motive-beliefs reuse the v20
  provenance shape unchanged — gossip, lying, confiding, and forgetting all work on
  `desires.<owner>.<name>` with zero new machinery; "public" and "secret" fall out as *derived*,
  defeasible common knowledge (a professed desire, or a conventionally-assumed one) rather than a
  flag. `Prax.Sight` makes location itself a belief: a period-1 schedule rule, run by the engine
  at each round boundary (v44), refreshes co-present sightings against the engine's clock, so
  predicting someone who has stepped out of
  the room means predicting from where you last saw them, with an authored horizon for how long
  that guess stays good. `Prax.Worlds.Intrigue`'s plot is now a believed mind: a confidant's
  prediction of cassia foresees the poisoning; the uninformed victim's does not; and a leaked
  rumor of her motive flips that. Master's own test suite dropped 5.5s → 0.8s as a side effect of
  the redesign. The true referee — v22's village suite, landed on top — bears it out: ~19s (down
  from the 621s blowup, a >30× recovery, back to the original pre-blowup order of magnitude).
- `Prax.Project` (v24) — **endeavors**: authored project *types* (`endeavor`/`Stage`) compile to
  an undertake action, a one-instance-per-owner staged practice, and a named pursuit desire that
  rewards each completed stage directly, so a long project needs no planner change — every next
  stage is ordinary local utility, and the desire sits *dormant* (zero bindings, zero utility)
  for any disposed character until undertaking switches it on, a worked instance of "a want
  gated on a fact is injectable by inserting the fact." Because the pursuit is a named,
  believable desire, a witnessed stage becomes theory-of-mind content for free. bob's redemption
  closes the village's arc on it: deterred since v21 and concealing since v22, he takes up honest
  work unprompted, sweeps the square in the open, and earns — rather than steals — his loaf,
  while the village learns his purpose just by watching him work.
- `Prax.Persona` (v25) — **temperament as conduct-valuations**: a `Trait` bundles desires over the
  bearer's *own* conduct-marks, not goals — `honest` costs a lie-mark rather than forbidding the
  lie, so a bearer can still lie if the arithmetic ever favored it. Conduct needed something to
  value, so `Prax.Deceit.lie` gained one outcome: the liar's own memory of the deed
  (`Actor.lied.Hearer.<pat>`) — a mark on the liar, never a world-rooted ground-truth record.
  `transparent` derives that everyone presumes a bearer's valuations from the start, so a believed
  temperament nets against a believed motive in prediction. `Prax.Worlds.Village` gains gale, eve's
  contrast: the identical named spite, but gale's conscience outprices it, so eve whispers and gale
  never does — and, found live rather than authored, eve's whisper deceives gale too, who then
  spreads the falsehood she genuinely believes by ordinary gossip, no lie, no mark, carrying it
  back to eve as "evidence" for her own fabrication. The honest villager launders the lie.
- `Prax.Debt` + `Prax.Blackmail` (v30) — **leverage, priced**: a debt is an obligation with a
  beneficiary (`owe`/`settle`), default is belief-gated deadbeat standing, and a threat is a
  motive-belief deposit (`shakedown`'s threaten/comply/defy/expose) the victim's own round-walk
  prices — the extortionist is credible because the punitive desire the threat professes is
  genuinely held, not because it predicts compliance. `Prax.Worlds.Village`'s carol shakes eve down
  once threshold fear (a nonlinear want mirroring bob's `notorious.bob.thief`) makes a single
  witnessed whisper land two of the three regards notoriety needs; the same fear makes eve a
  one-shot liar, retelling v25's "honest villager launders the lie" finding under a forced
  continuation once free play alone can no longer reach it. Found by the planner's own lookahead,
  not designed for: an unguarded repeat threat could re-extract silence indefinitely, banking
  serial extortion as a real future mechanic once guarded shut.
- `Prax.Faction` + `Prax.Kin` (v31) — **one membership spine, two generators**: two backlog rows
  folded because they share one primitive. `member.<who>!<faction>` is a base, single-slot fact —
  joining, defecting, and marrying-in are all the same `!` exclusion overwrite. `comrades` derives
  `allied.X.Y` from shared membership, keeping the `allied` name so every existing consumer needs
  no change — proved by refactoring `Prax.Worlds.Feud` onto it: the two hand-authored pairwise
  `allied.*` setup facts are gone, replaced by three house memberships, and `FeudSpec`'s 5 original
  tests pass byte-unmodified. `kinAxioms` is pure derivation (marriage symmetry, sibling,
  grandparent, one-directional in-laws), retraction-safe with a designed asymmetry: dissolving a
  marriage un-derives its in-laws, but membership itself does not, since `wed`'s transfer is a base
  move, not a derivation. `wed joiner faction spouse` compiles a wedding to the marriage fact plus
  one membership overwrite; succession reuses the same exclusion idiom for offices, any child of a
  dead holder may claim the single slot. `Prax.Worlds.Feud`'s wedding beat: esme, inert in her own
  single-member house, weds into the feud's `kestrel` house and inherits its grudge against alice —
  the planner has her shunning alice, unprompted, on the first try. The village is untouched this
  round (goldens byte-identical); `factionStanding` (belief-gated regard through a faction-mate)
  ships spec-tested but unwired into any world, a stated and deferred decision.
- `Prax.Confession` (v32) — **the road back is real, and it narrows**: a lied-mark converts to a
  confessed one (never deleted, so a trait can still price the residue; the discharge verb is an
  authored parameter since **v48** — shipped worlds pass `"confessed"`, but recant/boast/admit fit
  the same machinery), confessing self-incriminates through the same sourced-hearsay channel
  gossip already rides, and absolution is a separate, refusable second-party act that inserts the
  world's own standing-defeater. An
  absolver's patience (`incorrigible`) points `Prax.Repute.notoriety`'s own counting idiom inward —
  fed-up-ness is what she *believes*, not a bookkept tally. `Prax.Worlds.Village`'s eve confesses to
  gale, who already regards her a slanderer from witnessing the whisper directly, and it costs
  nothing; confessing to carol, the party actually wronged, was probed and measured never to beat
  eve's baseline at any authored generosity — the planner's own discount on a *predicted* absolution
  caps the achievable relief regardless. The road back closes the liar's own conscience and standing,
  never a third party's frame-up: carol still has no recourse.
- **v26 — planner work elimination**: an exactness-contract performance round (golden
  decision-sequence tests pin every planner choice, byte-identical before/after). Cached
  per-state closed views, a conservative relevance pre-filter (`Prax.Relevance`) that skips
  predictions no authored action could motivate, tokenize-once query/closure internals, and
  shared test trajectories: the full suite dropped ~726s → ~114s with zero behavior change.
- **v27 — incremental view maintenance**: the cached world-view keeps itself, provably — a
  per-turn invariant suite pins `readView` to a from-scratch closure while three tiers build
  it (lockstep application for deltas the axioms can't see, in-place growth of the closed
  view for inserts that defeat nothing, full re-derivation for the rest). A profiled village
  round fell 7.07s → 1.32s across v26+v27 with bit-for-bit identical decisions throughout.
- **v28 — the world compiles once**: authored conditions and outcomes cook to token form at
  world construction and the hot paths (candidate enumeration, outcome application, want
  scoring, the closure loop) run on names end to end, with the string evaluator retained as
  the independently-verified reference. Profiled round 1.32s → 0.69s; suite ~22–30s.
- **v29 — segment interning**: path segments intern to machine integers (variable-ness in
  the parity bit) and the engine computes in symbols end to end, strings only at the
  authoring/display boundary. Honestly reported: a wash on wall time (~10% within noise) —
  kept as the consistent endpoint of v28's design, with the attribution lesson recorded.
- **v33 — state-conditioned relevance**: v26's relevance skip asks vocabulary alone
  ("could ANY action ever improve this want-kind?"); this round adds "could it matter NOW,"
  a per-state floor/gate check consulted alongside it. A 31-test village A/B recovers ≈39s of
  a prior round's regression (171.64s → 132.75s) — a real but partial reclaim, not a return to
  that round's own pre-regression 31.11s, the residual being the world's own accumulated
  richness, not the filter.

See `docs/LEDGER.md` for what's next (character prose-sketches, timed junctions, the
player as DM, …).

## Build, test, play

Requires GHC 9.x + Cabal.

```sh
cabal build       # compile everything
cabal test        # run the test suite (tasty)
cabal run prax             # play the bar demo — you are 'you'; pick from the menu
cabal run prax -- intrigue  # play the dramatic episode (a Roman conspiracy)
cabal run prax -- play      # play the same drama authored as a Prompter-lite play-script
cabal run prax -- dm        # you are the drama manager — steer an autonomous cast
cabal run prax -- feud      # emergent sandbox: a feud derived from one wrong + a handful of rules
cabal run prax -- audience  # a Prompter demo: timed junction + character-sketch in one scene
cabal run prax -- village   # witnessing + rumor + reputation + deception + endeavors: what you see or hear settles into standing, an atoned thief is deterred from stealing again, a concealed secret stays kept while it's worth keeping, an unproven whisper cascades into reputation exactly like the truth would, and — given a lawful way to earn what he wanted — the thief takes up honest work instead
cabal run prax -- flow      # print the play's scene-flow chart (Mermaid)
cabal run prax -- check feud   # static well-formedness check of a world
```

```sh
cabal run prax -- dump-play         # print the play-script as JSON
cabal run prax -- play examples/play.json  # load and play a play-script from JSON
```

In the bar demo, NPCs act autonomously: order a drink and the bartender (ada) will serve you,
while the patron (bex) pursues a beer of their own. In **Intrigue** you are Marcus: a conspirator
means to poison your patron — do nothing and the plot runs its course (a character dies), or warn
him, do the deed yourself, or romance the conspirator. It reaches distinct endings and demonstrates
Blood & Laurels-style drama (murder, death, betrayal, branching) on the same engine.

**New here? Read `docs/WALKTHROUGH.md`** — a guided playthrough that names each thing to try and
explains which feature it demonstrates. Part I tours the whole engine core by playing the bar;
Part II walks the rest (intrigue, stress, save/resume, play/flow, dm, feud, check, audience, village).

## References

Primary source material (the Versu paper, the Praxish checkout, etc.) is downloaded into
`references/`, which is git-ignored — kept locally, never distributed.
