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
- `Prax.Core` (v2) — the Versu core model: emotions (single-slot moods with target/cause and a
  remembered prior) and relationships (numeric, asymmetric role evaluations). Wired into the bar
  so interaction changes feelings and feelings change behaviour (a warmth-gated "buy a drink"
  affordance; a snub breeds an `annoyed` mood that withholds it).
- `Prax.Reactions` (v3) — reactions-as-practices and norms: an action spawns a reaction practice
  offering responses (greet back / rebuff / take offense), and a norm violation (stiffing the
  bartender) spawns disapproval. NPCs avoid violations because the planner scores the
  violation→disapproval future poorly.
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
  next scene) — and `compile`d to ordinary practices. A bodiless *narrator* (Versu's story manager)
  fires junctions automatically; `flowChart` renders the scene graph (`cabal run prax -- flow`), and
  the stress-tester reports scene coverage. `Prax.Worlds.Play` is a *faithful* recasting of
  `Prax.Worlds.Intrigue` — same cast, affordances (confide, poison, warn, self-poison, romance),
  and endings — as a two-scene play in ~25% fewer authored lines, *plus* the scene transition,
  flow-chart, and story manager the layer supplies for free. The scene layer also compiles the rest
  of Prompter's authoring constructs (v18): **memories** (`memory` — one-shot exposition fired the
  first time a trigger holds), **timed junctions** (`after`/`timeout` — a scene transition/ending
  after N turns, via a passive scene clock), and **character sketches** (`concernedWith` turns
  concerns into desires; `withTraits` records personality as queryable facts). Scene *bounds* are
  subsumed by scene-local beats, and the readable text playtext is intentionally replaced by JSON.
  `Prax.Worlds.Audience` (`prax audience`) exercises all three in one short scene: a royal audience
  where a memory recalls a warning as you enter, an ambitious Duke's *concern* for standing makes him
  court the king unbidden, and the audience times out (`dismissed`) if you don't press your petition
  (`granted`) in time.
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
  `Prax.Worlds.Feud` (`prax feud`) is the demo: from *one* authored wrong and three rules, a whole
  feud emerges (people who never met come to resent someone through the alliance network) and
  dissolves the moment amends are made.
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

See `docs/LEDGER.md` for what's next (character prose-sketches, timed junctions, memories, the
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
cabal run prax -- feud      # emergent sandbox: a feud derived from one wrong + three rules
cabal run prax -- audience  # a Prompter demo: memory + timed junction + character-sketch in one scene
cabal run prax -- village   # witnessing + rumor + reputation: what you see or hear settles into standing, notoriety tips the thief into atoning, and an atoned thief is deterred from stealing again
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
