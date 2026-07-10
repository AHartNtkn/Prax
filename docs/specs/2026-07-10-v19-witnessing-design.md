# v19 — Quantified outcomes + witnessing (`ForEach`, `Prax.Witness`)

Backlog item **K** (`docs/LEDGER.md`, "Sandbox extension backlog"): the keystone of the
large-cast-sandbox arc. When an action is performed, co-present characters acquire a persistent
*belief that it happened*; characters elsewhere don't. This creates general information asymmetry —
the substrate for rumor (v20), reputation (v21), and later secrets/deception.

Decisions taken during brainstorm (2026-07-10):

- **Observability is authored, not automatic.** An action's public appearance is a semantic
  property the author states (`observable`); undeclared actions are not events. No auto-derived
  action-key vocabulary. This preserves the looks-like/is distinction deception later needs.
- **v19 is the keystone alone**: `ForEach` + `Prax.Witness` + the village seed world + tests.
  Rumor and repute are v20/v21, stacking on the same village.

## 1. Foundational: the `ForEach` outcome

### Motivation

Witnessing needs "for **every** co-present W, insert `W.believes.…`". The outcome language cannot
express it: `Insert`/`Delete` are singular, and `Call` applies its first case's **first** binding
only (`take 1`, `Engine.performOutcome`). This is the dual gap of what v8 closed for conditions
(conditions got `Exists`/`Absent`/`forAll`; outcomes never got their quantifier). Also needed by
much of the backlog (festival-wide effects, faction-wide effects, inheritance to all heirs).

### Semantics

```haskell
data Outcome = … | ForEach [Condition] [Outcome]
```

`performOutcome (ForEach conds outs) st`:

1. Evaluate `query view conds Map.empty` where `view = readView st` — condition reads go through
   the defeasible closed view, consistent with action preconditions (so a `ForEach` can quantify
   over *derived* facts, e.g. "everyone who resents X").
2. **Snapshot semantics**: collect *all* bindings first, then fold the sub-outcomes over each
   binding in order. Mutations performed for binding 1 must not affect which bindings exist —
   quantification is over the state at entry. (This removes order-dependence by construction; no
   re-query mid-fold.)
3. For each binding `b`, apply `performOutcome (groundOutcome o b)` for each `o ∈ outs`, in order.
4. Zero bindings ⇒ no-op.

`groundOutcome (ForEach conds outs) b` grounds the enclosing action's bindings into both the
conditions and the sub-outcomes (sentence fields ground via the existing `ground`; a small
`groundCondition` traversal is added — mechanical, one case per `Condition` constructor). Variables
left free after that grounding are the quantified ones.

Nesting is permitted and needs no special handling (`ForEach` inside `ForEach` grounds outer
bindings first, then quantifies the rest).

### Integration

- **TypeCheck**: `ForEach`'s conditions *bind* variables for its sub-outcomes — extend the
  unbound-variable analysis so a sub-outcome variable bound by the `ForEach` conditions (or the
  enclosing action's conditions) is not flagged; a genuinely unbound one still is. Sub-outcome
  `Insert`s join the asserting-sentence corpus for cardinality/sort analysis; the conditions join
  the condition corpus; and `ForEach` sub-outcomes' `Call`s and spawns join the dangling-reference
  analysis, so an undefined function or practice named inside a `ForEach` is caught too.
- **JSON** (`Prax.Script.Json`): `{"forEach": {"when": [...conds], "do": [...outcomes]}}`,
  round-tripping like the existing outcome encodings.
- **Inspect/Stress/Persist**: no changes — outcomes are opaque to them; state stays a fact DB.

## 2. Compiled: `Prax.Witness`

### Vocabulary

A witnessed event is an ordinary belief (`Prax.Beliefs` unchanged):

```
<W>.believes.<event>!seen
```

The `!seen` value records **provenance** (direct observation). v20's rumor layer will plant the
same issue with `!heard`, giving an evidential distinction for free (a court scene can weigh an
eyewitness over hearsay) while all existing belief machinery (gating, revision) works on both.

### API

```haskell
-- | Co-presence template: conditions relating the fixed variables "Witness" and "Actor"
-- in this world's vocabulary (location facts, current scene, …).
type CoPresence = [Condition]

-- | Declare an action's public appearance: what a co-present character comes to believe.
-- The event sentence may use the action's own variables (e.g. "stole.Actor.Loaf").
observable :: CoPresence -> String -> Action -> Action
```

`observable copresence event act` appends to `act`'s outcomes:

```haskell
ForEach (copresence ++ [ Neq "Witness" "Actor" ])
        [ Insert "Witness.believes.<event>!seen" ]
```

The combinator adds only the actor-exclusion; everything else that constrains who can witness
(being a character, having a location) is the co-presence template's job, since only the world
knows its own vocabulary.

Notes:

- The actor is excluded — they *did* it; the world fact is their knowledge.
- Co-presence is **world vocabulary**, supplied by the world (the village exports its own
  `together :: CoPresence` over its location facts; a scene-world equivalent can come later).
  The engine has no built-in notion of place and gains none.
- Bodiless characters (narrator-style) are excluded naturally: they have no location, so
  co-presence conditions never bind them.
- The event sentence is authored per action and may deliberately differ from what the action
  *does* — that gap is the deception hook (out of scope until the secrets tier; nothing in v19
  forecloses it).

## 3. Demo: `Prax.Worlds.Village` (seed), CLI `prax village`

The proving ground the arc grows in (as the bar grew v2–v8). v19 keeps it minimal:

- **Places**: `square`, `mill` (movement as in the bar's `world` practice).
- **Cast**: `you` (the player, a villager — one agent among many), `bob` (will steal when hungry:
  a want-driven affordance `observable together "stole.Actor.Loaf"`), `carol`, `dana`.
- **The scene**: carol is in the square, dana at the mill. Bob steals the loaf. Result:
  `carol.believes.stole.bob.loaf!seen` exists; dana holds no such belief.
- **A belief-gated consequence** (so asymmetry is *visible in play*): a witness who believes the
  theft cools toward bob (`adjustScore` via a one-shot reaction-style action gated on the belief)
  and gains a "confront bob" affordance dana never gets.
- The player can walk between square and mill and observe that who-saw-what differs.

## 4. Tests (TDD; extend suite as each piece lands)

- `EngineSpec` — `ForEach`: zero bindings (no-op), one, many; enclosing-binding grounding;
  **snapshot semantics** (a sub-outcome that would falsify a later binding's conditions still
  applies to all originally-matched bindings); nested `ForEach`.
- `TypeCheckSpec` — a variable bound only by `ForEach` conditions is not flagged unbound; a
  genuinely unbound sub-outcome variable is; cardinality analysis sees sub-outcome inserts.
- `Script.JsonSpec` — `ForEach` round-trip.
- `VillageSpec` — witness believes (`!seen`), absent character doesn't, actor excluded,
  an *undeclared* action deposits nothing, the belief-gated consequence fires for carol only.
- Regression: full suite green; every world (incl. village) passes `prax check`; bar/play goldens
  untouched (no existing action becomes `observable` in v19).

## 5. Out of scope (parked deliberately)

- Rumor propagation (v20) and reputation axioms (v21) — same village, own rounds.
- Any automatic event deposit, action-identity vocabulary, or engine event log.
- Scene-layer `observable` sugar (add when a script world first needs it).
- Forgetting/decay of witnessed beliefs (the decay backlog item).
