# Praxis & Praxish — implementation notes

Reference for the Haskell port. Praxish source is checked out (git-ignored) at
`references/praxish/` (`db.js`, `praxish.js`, `planner.js`, `demos/`). Verbatim excerpts below
were read directly from those files this session.

- Praxish repo: https://github.com/mkremins/praxish ("Partial reconstruction of Versu's Praxis")
- RePraxis (C# port, states the corrected `!` rule): https://github.com/ShiJbey/RePraxis
- Praxis is Evans's exclusion-logic DSL underlying Versu (paper §VI–VII); see `versu-notes.md`.

## Database (`db.js`)

State is a nested JS object (trie); leaves are `{}`; sentences are `.`/`!`-delimited paths.

`DB.unify(sentence, db, bindings)` — splits the pattern on **both** `.` and `!` (they are
identical for *matching*; the distinction only matters on insert). For each segment: if it's a
variable (capitalized first char, `DB.isVariable`) and unbound, branch over every key of the
current subtree (fork a "possible world" per value, extending bindings); if bound or constant,
descend into that key or drop the world. Returns the list of consistent bindings maps. This is
the core nondeterministic list-monad match. `DB.unifyAll` threads bindings through a list of
sentences (conjunction).

`DB.insert(db, sentence)` — tokenizes keeping trailing punctuation (`/[^\.\!]+.?/g`). **Its `!`
handling is buggy** (author's own TODO): it does `subtree[key] = {}` — always overwriting the
child's subtree. This drops data: after `foo!bar.baz`, inserting `foo!bar.meow` wipes `baz`.

**Corrected semantics we implement (paper §VII + RePraxis):** `P!K` asserts `P` has *exactly
one child*. Inserting it should remove any **sibling** of `K` under `P`, but **preserve** `K`
and its existing subtree. So `x.age!32` then `x.age!33` ⇒ `age` has the single child `33`;
`foo!bar.baz` then `foo!bar.meow` ⇒ `bar` retains both `baz` and `meow` (only siblings of `bar`
would be excluded). This is the single most important divergence from Praxish.

`DB.retract` deletes the leaf key of a path. `DB.ground(sentence, bindings)` substitutes bound
variables back into a path (preserving `.`/`!`), warns on arrays (set values).

## Query DSL (`praxish.js` `Praxish.query`)

Conditions are strings; `parts = split on whitespace`:
- **1 token** → a logic sentence: `DB.unify` (conjunctive; extends bindings).
- `not S` → negation as failure: keep a match iff `S` has zero unifications.
- `eq A B` → if one side bound/constant and the other an unbound var, **bind** it (assignment);
  if both bound, keep iff string-equal; both unbound = author error.
- `neq A B` → both must be bound; keep iff unequal.
- `lt|lte|gt|gte A B` → both bound; numeric compare via `Number()`.
- `calc R op A B` → `op ∈ {add,sub,mul}` writes result into new lvar `R`; `calc R count Xs`
  counts an array-valued binding (arrays only come from `set` subqueries). No `div`.
- Subquery object `{set: R, find: [lvars], where: [conds]}` → runs a nested query, projects each
  result over `find`, stores the list under `R`. **Subqueries cannot nest.**
- `killsPerStep` metadata records which condition eliminated all bindings (impossible-action
  debugging). Worth reproducing for our "why did preconditions fail" inspector later.

In Haskell we model conditions as a typed `Condition` ADT (no string-splitting), preserving
these semantics exactly. Praxish's DSL lacks the paper's full FOL (`∀`/`∃`/`∨`/`→`); those are
post-v1 (see LEDGER).

## Practices, actions, outcomes (`praxish.js`)

- `definePractice` registers a `practiceDef` and inserts its static `data` sentences under
  `practiceData.<id>.`.
- Practice fields: `id`, `name` (template), `roles` (array; the instance key), `actions`,
  optional `data`, `init` (outcomes run once when an instance first spawns), `functions`
  (named, params, `cases` of `{conditions, outcomes}` — guarded conditional effects).
- Action fields: `name` (template with `[Var]`, doubles as actionID), `conditions`, `outcomes`
  (strings: `insert …` / `delete …` / `call fn args…`).
- `getAllPossibleActions(state, actor)` — for each instantiated practice
  (`db.practice.<id>.<roles…>`), unify roles to enumerate instances, seed `{Actor: actor}` +
  role bindings, run each action's `conditions` via `query`; each surviving binding is a
  grounded action (name via `renderText`, `[Var]`→value).
- `performOutcome` — `insert`: detect **spawning** (an `insert practice.<id>.<roles>` whose
  instance didn't exist) and, after inserting, run that practice's `init` once with role
  bindings. `delete`: `DB.retract`. `call`: find the named function across practiceDefs, bind
  params, run the **first** matching case's outcomes.
- `renderText` = literal `[key]`→value substitution.

## Action selection (`planner.js` / `demos/pwim/app.js`)

- Main loop is **round-robin, one actor per tick** (1000 ms interval in the demo); the player's
  turn pauses for input, NPCs use the planner.
- `Planner.scoreActions(state, actor, depth)` — for each possible action: clone DB, speculatively
  `performAction`, `score = evaluate(db, goals)`, recurse to `depth` with discount **0.9** for
  the actor's own future and **0.5** for others, then restore the DB. `pickAction` = argmax.
- **Our change (Versu-faithful):** replace Praxish's single global `goals`/`evaluate` with
  **per-character `[Want]`**; `evaluate db wants = Σ modifier × (#satisfying bindings)` (paper
  §IX-A). Immutability means "clone + undo" is just scoring `apply action db` and discarding it.

## Example content (`demos/`, our port targets)

- `demos/test/tests.js` — smoke tests + `greet`, `tendBar`, `ticTacToe`, `jukebox` practices.
- `demos/pwim/domain.js` — small storyworld: `world` (locations via `connected`, `Go to`),
  `greet`, `tendBar` (order/fulfill). Best concrete syntax reference; basis for `Prax.Worlds.Bar`.
- `demos/sway` — Swaygent (Ensemble-style volition/influence) selector — post-v1.
- PWIM embedding-based free-text player input (arXiv 2406.00942) — post-v1, external model dep.
