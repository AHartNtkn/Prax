# Feature Ledger

Every capability we intend `prax` to support, derived from the Versu paper and Praxish. Status:
- **v1** — in the first milestone (faithful engine core).
- **v2** — the core-model round (emotions & relationships), built as `Prax.Core`.
- **v3** — reactions-as-practices & a first norm, built as `Prax.Reactions`.
- **v4** — per-agent beliefs (incl. false beliefs), built as `Prax.Beliefs`.
- **v5** — conversation (speakers, topics, quips), built as `Prax.Conversation`.
- **v6** — a story manager (DM) as a metalevel agent (the bar `director`).
- **v7** — character arcs (internal high-level state), built as `Prax.Arc`.
- **v8** — first-order connectives in the query language (∀/∃/∨/→).
- **v9** — cast removal + a dramatic vertical slice (`Prax.Worlds.Intrigue`).
- **v10** — QA tooling: the inspector (`Prax.Inspect`) and stress-test (`Prax.Stress`).
- **v11** — persistence: save/load a session (`Prax.Persist`), CLI save + resume.
- **v12** — a Prompter-lite scene-authoring layer (`Prax.Script`) that compiles a
  CAST + scene-graph play-script to practices, with an auto flow-chart + scene coverage;
  demonstrated by `Prax.Worlds.Play`. Play-scripts round-trip through readable JSON
  (`Prax.Script.Json`) — the editable authoring format, chosen over a bespoke parser.
- **v13** — player-as-DM: the human occupies the drama-manager slot, steering an
  autonomous cast with metalevel nudges (`Prax.Worlds.Bar` `barDirectorWorld`, `prax dm`).
- **v14** — a first-class deontic `should`/obligation layer (`Prax.Deontic`): `□φ` as the
  fact `obliged.<who>.<φ>`, conflict detection via the `!` exclusion, and contrary-to-duty
  (`□□`); the bar's settle-up is now a real obligation.
- **v15** — a forward-chaining **derivation layer** (`Prax.EL` + `Prax.Derive`): domain rules
  closed to a fixpoint via the paper's `m(X)`, exact `⊥` detection, auto-`□`-lift (obligation
  closure), a defeasible closed *view* on the read path; `Prax.Worlds.Feud` is the emergent demo.
- **v16** — a static **well-formedness checker** (`Prax.TypeCheck`): unbound-variable,
  exclusion-cardinality, and dangling-reference checks over a world's authored sentences (`prax check`).
- **v17** — **ML-style sort inference** completing #8: sorts declared by membership, every
  position/variable sort inferred by unification, conflicts reported (`Prax.TypeCheck`).
- **v18** — the remaining **Prompter compilation features** in `Prax.Script`: memories (one-shot
  exposition), timed junctions (a scene clock), and character sketches (concerns→wants, traits→facts).
- **v19** — **quantified outcomes** (`ForEach`, the dual of v8's condition quantifiers) and
  **authored witnessing** (`Prax.Witness`): co-present characters come to believe an action
  happened, with `.seen` provenance (multi-valued from v20 — see below); observability is a
  semantic property the author states, not an automatic event log. `Prax.Worlds.Village` seeds the
  sandbox arc (`prax village`).
- **v20** — **sourced rumor propagation** (`Prax.Rumor` `gossip`/`heard`): a character tells a
  co-present hearer what they have evidence for, planting the same event-belief with hearsay
  provenance. Provenance becomes **multi-valued** (`.seen`/`.heard.<source>`, replacing v19's
  exclusive `!seen`), so witnessing and hearsay for the same event coexist instead of one
  overwriting the other, and corroboration (multiple named `.heard.<source>` edges) is countable.
  `Prax.Worlds.Village` grows: carol spreads the theft on her own; hearsay licenses
  `eye … with suspicion`, never `confront` (eyewitness-only); a world-authored relationship gate
  lets distrust close the gossip channel.
- **v21** — **derived reputation** (`Prax.Repute` `standing`/`standingUnless`/`regardedAs`/
  `notoriety`): `regards.<observer>.<subject>.<label>` is never stored, only *derived* from an
  observer's evidence (seen or heard alike) — so it inherits information asymmetry and
  defeasibility for free. Standing is defeated by **atonement, not amnesia**: `standingUnless`
  guards the derivation with a *base-fact* defeater, dissolving every regard on one insertion while
  every belief (the memory of the deed) persists untouched — and because the belief never went
  away, **re-offense revokes the defeater**, so standing and notoriety snap back instantly on a
  repeat. `notoriety` turns corroboration into a threshold-gated global fact (an authored world
  parameter). `Prax.Worlds.Village` completes its arc: theft → witnessing → rumor → three regards
  → notoriety tips bob into atonement → the village relents — and, because the planner can see the
  snap-back, an atoned thief facing a restocked stall is *deterred*, never touching it again.
- **v22** — **secrets & deception** (`Prax.Deceit` `conceal`/`lie`): a concealment want — that
  nobody believe some deed (`Absent [Anyone.believes.<event>]`) — makes an agent avoid witnesses
  *by planning*: the lookahead already simulates v19's witness deposits, so waiting for privacy
  falls out of ordinary utility, no stealth system needed. A lie is an assertion without evidence:
  `lie` mirrors v20's `gossip` and plants the identical `.heard.<liar>` hearsay, so a fabrication
  is indistinguishable from truth to everyone but the liar, and the whole rumor/reputation stack
  (v20/v21) cascades on it unmodified — hearing your own lie back replaces the lie action with
  plain gossip, seamlessly. `Prax.Worlds.Village` gains a villain: bob conceals his theft, waiting
  out a genuinely watched square (walking away alone isn't enough — carol keeps her own post; the
  bread is safe exactly as long as *someone* is watching); eve, out of authored malice, frames
  carol, and the frame-up settles into regard, shunning, and notoriety exactly as truth would —
  with an honest injustice at the end: framed carol has no recourse (amends needs a loaf she never
  took; exculpation needs ground-truth event records, banked below, not faked).
- **v23** — **realistic lookahead: round-walk over believed minds** (`Prax.Minds`, `Prax.Sight`,
  a rewritten `Prax.Planner`; spec `docs/specs/2026-07-10-v23-planner-realism-design.md`). The old
  lookahead's `worldValue` (now **deleted**) maxed over every living character's every action,
  scored by the *planning actor's own wants* — three demonstrated failures: speculative (credited
  others with actions they would never choose, e.g. carol's top move became an unevidenced
  accusation), omniscient (used movers' *true* wants, so a secret plot was foreseeable by anyone),
  and combinatorially explosive (v22's village suite: 8.7s → 621s). Replaced by a round-walk: each
  other character within the actor's **epistemic scope** (`predictionScope`, the v19 co-presence
  template — default everyone) gets one myopic, *motivated-only* move predicted from the actor's
  **believed** model of them (`predictMove`), in cast order, before the actor recurses on its own
  next choice (`scoreActions`, `pickAction`). A mind the actor holds no belief about, or a mover
  out of scope, is modeled as still, never as helpful — and the model can be *wrong* (prediction
  uses the actor's beliefs, not the mover's true wants). Desires become nameable and believable
  (`Prax.Minds`: owner-parameterized `Desire` templates, `charDesires`; motive-beliefs reuse the
  v20 provenance shape unchanged, so gossip/lie/confide/forget all work on
  `desires.<owner>.<name>` for free); "public"/"secret" is recovered as derived, defeasible common
  knowledge (`professed`/`conventional` → `.presumed`) rather than a flag. `Prax.Sight` adds
  sightings as ordinary location-beliefs (`believes.at`/`atSince`), maintained by a compiled
  per-round ticker (`turn!N`, the v18 `_clock` idiom) — "who's around" is itself information with
  an authored horizon, not global. Intrigue's plot is now a believed mind (a confidant's
  `predictMove` of cassia foresees the poisoning; the victim's does not; a leak flips it). Two
  world edits were needed outside Intrigue and are recorded honestly in the spec's §6: the
  village's `dana` gets a sanctioned mill-anchoring want (the same idiom `bob`'s stall-anchor
  already used), and the bar's `LoopSpec` golden-trace turn budget is re-derived from 20 to 25
  (5 rounds × a cast grown by one for the sight ticker) — the narration itself is unchanged.
  Master suite: 5.5s → 0.8s (the rewrite's own speedup). The true referee — v22's village suite,
  once landed on top — bears this out directly: ~19s (`cabal test --test-options='--pattern
  "Prax.Worlds.Village"'`), down from the 621s blowup, a >30× recovery, at the *original* pre-blowup
  order of magnitude (8.7s).
- **v24** — **endeavors: staged practices with dormant pursuits** (`Prax.Project` `endeavor`/
  `Stage`; spec `docs/specs/2026-07-11-v24-project-design.md`). A project *type* is authored
  vocabulary, like a practice or a desire; `endeavor pid weight undertakeLabel gate stages`
  compiles it to three things a world wires in: the undertake `Action`, a staged `Practice`
  (one instance per owner — undertaking twice is never offered again), and a named pursuit
  `Desire` that counts completed stages (`practice.<pid>.Owner.done.S`) at `+weight` each.
  Progress itself is the reward, so horizon length stops mattering — every next stage is
  locally visible to the ordinary planner, no lookahead change needed; this is the concrete
  case that finally **closes #21's runtime-want-injection question by worked pattern**, not
  just by claim (see #21's updated note, below): the pursuit desire is *dormant* — zero
  bindings, zero utility — for any disposed character with no instance yet, and undertaking
  (an ordinary planner choice) is exactly "injecting a want by inserting a fact." Because the
  pursuit is a named, nameable desire, an endeavor is automatically theory-of-mind content
  (`Prax.Minds`): whoever comes to believe a character pursues one gets it fed straight into
  `predictMove`. `Prax.Witness`'s `witnessed` outcome-builder is extracted as a first-class,
  exported combinator (previously folded only inside `observable`) so a generated stage can
  carry public observability in its own effects — `Prax.Project` needed this, not a new
  primitive. `Prax.Worlds.Village` closes its own moral arc on this: bob — deterred since v21,
  concealing since v22 — takes up `earnBread` from a clean t=0 free-play start: undertakes at
  the stall, sweeps the square in public, walks to the mill, fetches flour, returns, and bakes
  the loaf he could no longer safely steal, done by turn 32 of the same running world every
  earlier round used. Watching him sweep is enough for the village to learn his purpose (a
  one-line inference axiom presumes the pursuit for anyone who believed the sweep), and once he
  stands at the mill, `predictMove` anticipates the flour trip specifically for whoever holds
  that belief — myopically (no prediction at the square, where no stage is yet available) and
  belief-relative (co-present, unbelieving dana predicts nothing, proving prediction reads the
  predictor's beliefs, not the mover's true state). The opportunism stays honest: mid-project,
  with the square genuinely empty, stealing (71.18) still beats pressing on to the next stage
  (50.46) — industry is chosen because it's watched and safe, not because temptation stopped
  scoring. One sanctioned test amendment (spec §3): "an atoned thief is deterred" swaps its
  stale "bob holds no loaf" proxy for a direct non-re-offense assertion
  (`practice.earnBread.bob.done.s3`), since the endeavor now gives bob a lawful loaf the
  original proxy couldn't anticipate.
- **planned** — committed for later; well-understood from sources.
- **research-needed** — blocked on an external dependency (an embedding model, #42) or an unsettled
  design question (#8). The DEON 2010 exclusion-logic paper that formerly blocked #34/#8 is now
  obtained and distilled (`docs/research/deon-notes.md`).

Paper = Evans & Short 2014 (see `docs/research/versu-notes.md`). "P§" = its section/page.

## Core logic engine

| # | Feature | Status | Source | Notes |
|---|---------|--------|--------|-------|
| 1 | Trie world-state DB (all state = sentences) | v1 | P§VI | `newtype Db = Db (Map String Db)` |
| 2 | `.` multi-valued descent | v1 | P§VII | |
| 3 | `!` exclusion (single-valued, sibling-clearing) | v1 | P§VII | **fixes Praxish bug** — clears siblings, preserves child subtree |
| 4 | Prefix = object; delete subtree by prefix | v1 | P§VII | `retract` |
| 5 | Unification / pattern match (vars = Capitalized) | v1 | Praxish `db.js` | list-monad over bindings |
| 6 | Query ops: not / eq(assign) / neq / lt·lte·gt·gte / calc / subquery | v1 | Praxish `praxish.js` | typed `Condition` ADT |
| 7 | Full FOL queries: ∀, ∃, ∨, → | v8 | P§VII | `Prax.Query` `Or`/`Absent`/`Exists` + `forAll`/`implies`; nests freely |
| 8 | Static type inference / checker (ML-style) | v16–17 | P§VII p.120 | `Prax.TypeCheck` `typeCheck`. **v16** (declaration-free, sound): unbound variables, exclusion-cardinality consistency, dangling `Call`/spawn refs. **v17** (ML-style *sort* inference): sorts declared by membership (`sorts` on `PraxState`), every position/variable sort inferred by union-find and conflicts reported (agent-vs-gender). Every shipped world checks clean; the bar declares `beverage`/`place`/…; `prax check`. Sort-checking is a conservative type system (may reject genuinely-polymorphic positions; declare only monomorphic ones) |

## Practices & actions

| # | Feature | Status | Source | Notes |
|---|---------|--------|--------|-------|
| 9 | Practices as first-class instantiable objects | v1 | P§VIII | `process.<id>.<roles>` |
| 10 | Role-agnostic practices (any cast fills roles) | v1 | P§VIII | key to replayability |
| 11 | Actions: conditions + outcomes (insert/delete/call) | v1 | Praxish | |
| 12 | Practice `data`, `init`-on-spawn, `functions`/`cases` | v1 | Praxish | |
| 13 | Concurrent practices; options = union of affordances | v1 | P§V | falls out of the loop |
| 14 | Constitutive affordances (only available in-practice) | v1 | P§VIII | |
| 15 | Norms: violation-marking postconditions + norm desires | v3 | P§VIII-D | `Prax.Reactions` `markViolation`/`violationOf`; strong-negative want ⇒ planner avoids |
| 16 | Reactions as practices (spawned by an action's outcomes) | v3 | P§X | `Prax.Reactions` `spawnReaction`/`endReaction`; `disapprovalP`; response chains |
| 17 | Conditional effects / domain axioms in the action language | v15 | P§VIII | `Prax.Derive`: domain rules `body → head` forward-chained to a fixpoint (the paper's `m(X)`) over `Prax.EL`, by **semi-naive** evaluation (fire only on newly-derived facts — ~8× faster than naive at scale); reads see the closed **view** (`readView`), which is defeasible (derivations recompute from the base) and opt-in (`axioms=[]` ⇒ unchanged). Auto-`□`-lift gives obligation-closure (DEON property 1). Exact `⊥` detection. Demo: `Prax.Worlds.Feud` (`bigFeud n` scales it) |

## Agents & action selection

| # | Feature | Status | Source | Notes |
|---|---------|--------|--------|-------|
| 18 | Utility-based reactive selection (apply-evaluate-undo) | v1 | P§IX | immutability ⇒ no explicit undo |
| 19 | Per-character wants; utility = Σ modifier × #bindings | v1 | P§IX-A | Versu-faithful; supersedes Praxish global goals |
| 20 | Round-walk lookahead over believed minds, w/ discounts (0.9 self / 0.5 other) | v1, v23 | Praxish `planner.js` (v1); redesigned (v23, spec `docs/specs/2026-07-10-v23-planner-realism-design.md` §4) | `Prax.Planner`. **v1**'s `worldValue` — max over every living character's every available action, scored by the *planning actor's* own wants — is **deleted**: it was speculative (credited others with actions they'd never choose), omniscient (used movers' *true* wants), and combinatorially explosive. **v23**: `scoreActions` predicts each other character within the actor's epistemic `predictionScope` (default everyone) exactly once, myopically, from the actor's **believed** model of them (`predictMove`, `Prax.Minds`) — only if the move strictly improves that belief over doing nothing (unmotivated moves are not predicted) — in cast round-robin order after the actor, before the actor recurses on its own next choice. `depth` still counts only the actor's own future plies; the CLI/loop keep depth 2 |
| 21 | Wants as arbitrary logic sentences (∃/∀ desires) | v8\* | P§IX-A | unblocked by #7 — a want is now any FOL formula; runtime want injection needs no separate mechanism (a want gated on a fact is injectable by inserting the fact). **Closed by worked pattern in v24**: `Prax.Project`'s pursuit `Desire` is dormant (zero bindings, zero utility) for any disposed character with no project instance, and undertaking — an ordinary planner choice — inserts the very fact that switches it on; bob's `charDesires = ["pursues-earnBread"]` carries the disposition permanently, live only once he acts on it |
| 22 | Character arcs / interiority (high-level internal choices) | v7 | P§X | `Prax.Arc`; bex's hopeful→belonging/lonely arc gates its wants; against-desires transformation is player-only |
| 23 | Swaygent-style volition/influence selection | research-needed | Praxish `swaygent.js` | Ensemble-inspired alt selector |

## Core model (emotion / relationship / belief)

| # | Feature | Status | Source | Notes |
|---|---------|--------|--------|-------|
| 24 | Emotions (Ekman, single-slot, remembers target+cause+prev) | v2 | P§X | `Prax.Core` `setMood`; mood `!`-override + `priorMood` |
| 25 | Role-evaluation relationships (multiple, asymmetric, w/ reason) | v2 | P§X | `Prax.Core` `adjustScore`; `A.relationship.B.role.score!N`/`reason!Why` |
| 26 | Public symmetric relationship state | v2 | P§X | `Prax.Core` `setBond` writes both orderings |
| 27 | Beliefs: shared world + per-issue divergence | v4 | P§X | `Prax.Beliefs` `believe`/`believesThat`/`forget`; `X.believes.<issue>!V` |
| 28 | Quantified / nested beliefs | research-needed | P§XI | Versu itself couldn't do this |
| 29 | Conversation: speakers, topics, quips (template + effects) | v5 | P§X / ES blog | `Prax.Conversation` `quip`/`changeSubject`; speaker turn-taking; quips shift core model & beliefs |

## Story management & authoring

| # | Feature | Status | Source | Notes |
|---|---------|--------|--------|-------|
| 30 | DM / story manager as a special practice | v6 | P§VI, XI | bar `director`: a bound metalevel agent with story-level wants; injects a rivalry |
| 31 | Player as DM | v13 | P§XI | `Prax.Worlds.Bar` `barDirectorWorld`: the human is bound to the metalevel `direct` practice (stir a rivalry / kindle warmth / cast a pall) and steers an autonomous cast; the CLI offers a bound player only its practice's affordances (via `candidateActions`). `prax dm` |
| 32 | Readable serialization for play-scripts (JSON) | v12 | P§VII-VIII | `Prax.Script.Json`: round-trips a `Script` to/from JSON — an editable authoring/exchange format with no bespoke grammar to maintain; `prax play <file.json>`, `prax dump-play`, `examples/play.json`. (Chosen over a custom `.prompter` parser.) |
| 33 | Prompter-style play-script front end (scene/beat/junction → practices) | v12,18 | P§XII | `Prax.Script`: CAST + scene-graph eDSL, `compile`, auto `flowChart`; a bodiless narrator fires junctions. **v18** adds the deferred compilation features: **memories** (`memory` — one-shot exposition on first-trigger), **timed junctions** (`after`/`timeout` — a passive scene clock), and **character sketches** (`concernedWith` → wants, `withTraits` → facts). Scene *parameters/bounds* are subsumed (affordances are already scene-local). The readable text surface is deliberately omitted — JSON (#32) stands in. |
| 34 | Deontic `should` / obligation operator; norm-conflict resolution | v14 | DEON 2010 | `Prax.Deontic`: `□φ` = fact `obliged.<who>.<φ>` (the paper's `Ob:φ` sugar, no semantic change); conflict *detection* via `!`-exclusion collapse (property 2); breach reuses `violated.…`; contrary-to-duty (`□□`) via nested obligations; behavioural coupling by Wants, planner unchanged. Resolution is *emergent* (utility) — explicit priority is a documented extension. Gaps: no entailment-closure (property 1), no `m(X)`/LRT (that's #8). Grounding: `docs/research/deon-notes.md` |

## Runtime, tooling, UX

| # | Feature | Status | Source | Notes |
|---|---------|--------|--------|-------|
| 35 | CLI menu loop (act / more), narration | v1 | P§V UI | |
| 36 | Round-robin turn loop | v1 | Praxish `app.js` | |
| 37 | Deterministic playback / replay | v10 | P§VI | pure loop ⇒ reproducible traces (golden replay); mid-session save/resume-to-file via `Prax.Persist` (v11) |
| 38 | Runtime inspector ("why is X true / why did preconds fail") | v10 | P§VI | `Prax.Inspect` `explain`/`firstFailing` (revives `killsPerStep`) |
| 39 | Stress-test harness (many auto-played runs) | v10 | P§VI | `Prax.Stress` — seeded random all-AI runs; endings + action coverage + dead-ends; **scene coverage** (which scenes random play reaches — Prompter's report) added in v12; CLI `prax stress` |
| 40 | Serializable world state (save/load) | v11 | P§VI | `Prax.Persist` (facts + cursor); exact round-trip; CLI in-game `s` save + `resume` |
| 41 | Rich branching multi-character episode (content) | v9 | P§XII | `Prax.Worlds.Intrigue`: murder, character death (cast-removal), betrayal/loyalty/complicity endings, romance. `Prax.Worlds.Play` (v12) recasts it as a 2-scene play-script |
| 43 | Cast removal (death / eviction) | v9 | P§VIII-D | `dead.<name>` fact; `Prax.Types.livingCharacters`; loop/planner skip the dead |
| 42 | PWIM embedding-based free-text player input | research-needed | arXiv 2406.00942 | external model dependency |

## Open research to close
- Only **#42** (PWIM free-text input) remains research-needed, and it is an external-model dependency,
  not a paper to obtain.
- The **DEON 2010 paper** (`references/papers/EVAIEL.pdf`, distilled in `docs/research/deon-notes.md`)
  is obtained and fully applied: it grounded the deontic layer #34 (v14) and the `m(X)` derivation
  closure #17 (v15). #8's type checker (v16–17) turned out to need only sentence-structure analysis,
  not the full LRT decision procedure.

## Future ideas to investigate
- **Incremental view maintenance for the derivation layer (#17).** Closure is now semi-naive and
  cheap per call, but the planner's lookahead still re-closes each distinct future state from scratch.
  Since a child state differs from its parent by a small delta (one action's outcomes), the closed
  view could be *maintained incrementally* — add/retract the delta's consequences rather than
  recompute — which would cut the lookahead's dominant cost. Bigger change than semi-naive (needs
  delta-retraction / provenance to un-derive facts whose support is gone); worth it only if a large
  axiom set + deep lookahead proves to be the bottleneck in a real sandbox.
- **Hard priority tiers for action selection (from Praxish's `swaygent.js`).** Ensemble/CiF-style
  selection tags actions with a symbolic tier — `forbidden` / `required` / `normal` — that sorts
  *above* numeric utility, giving categorical "you must / may not" rules. Our planner and norms are
  all *soft* (a strong-negative want steers away, but nothing is inviolable). Borrowing tiers would
  give the deontic layer (#34, v14) **hard** norm enforcement: an obligation ⇒ `required`, a
  prohibition ⇒ `forbidden`. It is a selection-paradigm change, not a Versu feature — Swaygent is
  Praxish's alt selector, whereas we (faithfully) use Versu's utility planner — and combining hard
  tiers with N-ply lookahead (prune forbidden branches, propagate required) is the non-trivial part.
  A "beyond Versu" enhancement, not a parity gap.

## Sandbox extension backlog (brainstormed 2026-07-10)

Target frame: a **large-cast, long-time sandbox** where the player is one agent among many;
**symbolic only** (no external models — PWIM #42 stays parked). All of these are *beyond Versu*.
Unless marked foundational, each compiles onto the existing layers and can be taken or left in
isolation. **K is the keystone**: most of Tier 1–3 stacks on it (marked ⤷K).

**K. Witnessing / event deposits** *(done — v19: `ForEach` quantified outcomes in the engine;
`Prax.Witness` authored observability; `Prax.Worlds.Village` seed, CLI `prax village`)*. When an
action is performed, co-located characters acquire a persistent *belief that it happened*;
characters elsewhere don't. Generalizes v3's per-action authored reactions into "react to any
action" (the old event-bus idea). Resolved as a small engine primitive (`ForEach`, quantifying an
outcome over every binding) plus a compiled-per-action combinator (`observable`) built on it — not
a separate hook. Unlocks information asymmetry — the root of reputation, rumor, secrets, alibis.

Tier 1 — compiled social structures:
- **⤷K Gossip / rumor propagation** (`Prax.Rumor`) *(done — v20: `gossip`/`heard`, authored per
  event-pattern like `observable`; evidence is a prefix match on `believes.<event>`; spreading is
  want-driven)*: share a held belief with a co-located, relationship-gated hearer, planting the
  same belief. Reuses `Prax.Beliefs`; false rumors already work. Reputation travels. *Resolved in
  v20:* provenance is multi-valued (`.seen`/`.heard.<source>`), so a `.heard.<source>` edge for
  someone who witnessed **and** later hears the same event sits *beside* their `.seen` edge instead
  of overwriting it — evidence accumulates and corroboration (multiple named sources) is countable.
- **⤷K Reputation** (`Prax.Repute`) *(done — v21: `standing`/`standingUnless`/`regardedAs`/
  `notoriety` — per-observer standing derived from believed deeds
  (`believes.X.(stole.Y._) ⇒ regards.X.Y.thief`); defeated by a *base-fact* defeater, not by
  deleting the belief — atonement, not amnesia, so re-offense (which revokes the defeater fact)
  makes standing snap back from memory that was never lost; `notoriety` counts derived regards at
  an authored threshold)*. Score effects from standing (a reaction, not an axiom) remain unbuilt —
  not needed for the village's arc.
- **Factions & membership** (`Prax.Faction`): membership facts + the feud axioms generalized
  ("my faction's enemy is my enemy"), join/leave/exile practices, and faction-/place-scoped deontic
  norm-sets (what's obligatory in the temple isn't at the tavern). Composes Derive + Deontic.
- **Debt & favors** (`Prax.Debt`): a debt *is* an obligation — `oblige` on borrowing, `discharge`
  on repayment, `breach` → reputation damage. Zero new semantics.
- **Kinship & households** (`Prax.Kin`): family relations + axioms (sibling symmetry, in-law
  derivation), marriage as bond+obligations, inheritance on death (cast removal exists). Offices as
  single-slot `!` facts (`mayor!bob`) — exclusion semantics are succession semantics.

Tier 2 — agent interiority for long time-spans:
- **Projects / endeavors** (`Prax.Project`) *(done — v24: `endeavor`/`Stage` — authored project
  types compile to an undertake action, a staged one-instance-per-owner practice, and a named,
  dormant pursuit desire that rewards completed stages directly, so horizon length never enters
  the planner's lookahead; a witnessed stage is theory-of-mind content the moment `Prax.Minds`
  believes it. `Prax.Worlds.Village`'s `earnBread` closes the village's moral arc: deterred,
  concealing bob is given honest work, takes it up unprompted, and the village learns his
  purpose by watching)*. Banked residuals, not attempted this round: **abandonment** (walking
  away from an in-progress instance mid-stage — the current model has no "give up" outcome, only
  completion); **cooperative projects** (multiple owners on one instance — `roles = ["Owner"]`
  is deliberately single-slot); **type synthesis** (authoring a *family* of endeavors from a
  higher-order description rather than one `endeavor` call per project type).
- **Personality → volition** (`Prax.Persona`): define our own trait semantics as documented
  want-packages (`vengeful` ≡ want [my grudges avenged] +k); turns v18 sketches into a cast
  generator. Principled because the mapping is a stated model, not per-world tuning.
- **⤷K Secrets & deception** (`Prax.Deceit`) *(done — v22: `conceal`/`lie` — a concealment want
  (`Absent [Anyone believes <deed>]`) makes the planner avoid witnesses automatically, lookahead
  already simulating the v19 witness deposits; `lie` plants the same `.heard.<liar>` hearsay as
  `gossip`, so a fabrication is indistinguishable from truth once heard, and hearing your own lie
  back turns it right back into gossip — the lie/gossip duality that makes the whole v20/v21 stack
  run on a falsehood unmodified)*. `Prax.Worlds.Village` gains a villain on this: bob conceals his
  theft; eve frames carol out of authored malice, and the frame-up cascades through the unmodified
  v20/v21 machinery to real shunning and notoriety, with an honest injustice — the framed have no
  recourse (amends needs a loaf never taken).
- **Ground-truth event records & exculpation** *(banked — v22 spec §5)*: v22's `lie` leaves no
  ground truth in the vocabulary — nobody, including the narrator, can check a fabrication against
  what actually happened, so the framed have no way to clear their name. Needs an event record
  (deed tokens / a calendar) actions could be checked against; deferred wholesale, alongside the
  banked counterfactual-placement and sighting-salience residuals below.
- **Blackmail** *(v-next candidate, split out from v22)*: obligation extracted under threat of
  gossip composes Deontic + Rumor, but its leverage model (exclusivity of knowledge; why the
  blackmailer withholds rather than gossips) deserves its own design round — parked deliberately,
  not attempted this round.
- **Counterfactual placement (per-agent world-views)** *(banked — v23 spec §4a "honest residual")*:
  a predicted in-scope mover is still simulated at their *true* position, not the predictor's
  *believed* one — imagining them where the predictor thinks they are requires giving every
  predictor its own simulable view of the world, the per-agent-world-view machinery Versu itself
  declined to build. Base facts leaking into predictions and template-fixed believed weights (no
  per-observer intensities) are the same residual: full per-agent world-views, deferred wholesale.
- **Sighting recency-salience** *(banked — v23)*: `Prax.Sight` sightings are single-slot (a new
  sighting overwrites the old) and `sightedWithin` gates prediction scope with a hard
  ticks-since-sighted threshold — a sighting one tick old and one at the horizon's edge are
  credited identically. A recency-weighted salience model (confidence decaying smoothly with
  elapsed ticks rather than a boolean cutoff) is banked, not built; needs a principled decay
  function before it's more than a heuristic.
- **Decay & drift**: scores cool toward baseline via a bodiless ticker (the v18 `_clock` pattern);
  rates must be authored world parameters with stated semantics, not tuned constants.
- **Calendar & gatherings** *(partially seeded — v23: `Prax.Sight`'s ticker already advances a
  global `turn!N` every round, the first brick of the clock; what's missing is authored
  clock-gated scene spawns keyed off it, not the clock itself)*: recurring clock-gated scene
  spawns (market day, festival) — the mixing dynamic that makes gossip percolate.

Tier 3 — host-game boundary:
- **⤷K Chronicler / salience queries** (`Prax.Chronicle`): derived summaries over the event stream
  ("a feud started", "the mayorship changed hands") — quest-hook generation, and the answer to
  emergence nobody can see.
- **Embedding API**: a `step / inject / query` surface for a host engine; design once a host exists.

Foundational watchlist (high bar; none currently urgent): hard priority tiers (above) — wait for a
demonstrated soft-norm failure; incremental view maintenance (above) — Tier 1 multiplies axioms ×
cast, so measure then decide; locality-scoped action discovery / level-of-detail — premature before
a large world exists to profile. Notably *not* foundational: runtime want injection (#21) — a want
gated on a fact is injectable by inserting the fact.
