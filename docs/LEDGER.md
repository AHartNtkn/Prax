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
| 20 | Forward-chaining lookahead w/ discounts (0.9 self / 0.5 other) | v1 | Praxish `planner.js` | depth configurable |
| 21 | Wants as arbitrary logic sentences (∃/∀ desires) | v8\* | P§IX-A | unblocked by #7 — a want is now any FOL formula; *runtime want injection still open* |
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
- **⤷K Reputation** (`Prax.Repute`): derivation axioms from believed deeds to standing
  (`believes.X.(stole.Y._) ⇒ regards.X.Y.thief`) + score effects. Defeasible for free — clear the
  belief and the standing dissolves (the feud pattern).
- **Factions & membership** (`Prax.Faction`): membership facts + the feud axioms generalized
  ("my faction's enemy is my enemy"), join/leave/exile practices, and faction-/place-scoped deontic
  norm-sets (what's obligatory in the temple isn't at the tavern). Composes Derive + Deontic.
- **Debt & favors** (`Prax.Debt`): a debt *is* an obligation — `oblige` on borrowing, `discharge`
  on repayment, `breach` → reputation damage. Zero new semantics.
- **Kinship & households** (`Prax.Kin`): family relations + axioms (sibling symmetry, in-law
  derivation), marriage as bond+obligations, inheritance on death (cast removal exists). Offices as
  single-slot `!` facts (`mayor!bob`) — exclusion semantics are succession semantics.

Tier 2 — agent interiority for long time-spans:
- **Projects / endeavors** (`Prax.Project`): staged external arcs (build a house = timber → plot →
  build), each stage gating wants — long horizons become chains of local utility, no planner change.
  The symbolic answer to bounded lookahead.
- **Personality → volition** (`Prax.Persona`): define our own trait semantics as documented
  want-packages (`vengeful` ≡ want [my grudges avenged] +k); turns v18 sketches into a cast
  generator. Principled because the mapping is a stated model, not per-world tuning.
- **⤷K Secrets & deception**: a secret = fact + concealment want (`Want [Absent [anyone believes
  X]]`) — the planner then avoids witnesses automatically; lying = planting a belief the speaker
  doesn't hold; blackmail = obligation extracted under threat of gossip.
- **Decay & drift**: scores cool toward baseline via a bodiless ticker (the v18 `_clock` pattern);
  rates must be authored world parameters with stated semantics, not tuned constants.
- **Calendar & gatherings**: recurring clock-gated scene spawns (market day, festival) — the mixing
  dynamic that makes gossip percolate.

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
