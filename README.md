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

See `docs/LEDGER.md` for what's next (public bonds in play, richer norms, a generic event bus,
multi-party conversation, the player as DM, a text authoring language, …).

## Build, test, play

Requires GHC 9.x + Cabal.

```sh
cabal build       # compile everything
cabal test        # run the test suite (tasty)
cabal run prax             # play the bar demo — you are 'you'; pick from the menu
cabal run prax -- intrigue  # play the dramatic episode (a Roman conspiracy)
```

In the bar demo, NPCs act autonomously: order a drink and the bartender (ada) will serve you,
while the patron (bex) pursues a beer of their own. In **Intrigue** you are Marcus: a conspirator
means to poison your patron — do nothing and the plot runs its course (a character dies), or warn
him, do the deed yourself, or romance the conspirator. It reaches distinct endings and demonstrates
Blood & Laurels-style drama (murder, death, betrayal, branching) on the same engine.

**New here? Read `docs/WALKTHROUGH.md`** — a guided playthrough that names each thing to try and
explains which engine feature it demonstrates. The bar world exercises every v1 feature.

## References

Primary source material (the Versu paper, the Praxish checkout, etc.) is downloaded into
`references/`, which is git-ignored — kept locally, never distributed.
