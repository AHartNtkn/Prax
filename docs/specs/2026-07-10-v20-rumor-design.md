# v20 — Rumor propagation (`Prax.Rumor`, provenance vocabulary)

Backlog item **⤷K Gossip / rumor propagation** (`docs/LEDGER.md`, "Sandbox extension backlog"):
a held belief about an event can be *told* to a co-present hearer, planting the same belief with
hearsay provenance. With v19's witnessing this closes the loop: what happens in front of people
travels beyond them. Reputation (v21) will derive standing from the resulting belief sets.

## 1. The provenance decision (resolves the banked v19 review note)

**Provenance becomes multi-valued, with named sources.** The v19 shape `believes.<event>!seen`
is replaced by:

```
<W>.believes.<event>.seen             -- direct witness (leaf)
<W>.believes.<event>.heard.<source>   -- hearsay, one edge per teller
```

Grounds (each independently sufficient):

1. **Semantics.** Evidence accumulates — one can see an event *and* hear of it from several
   people. An exclusive slot forces later evidence to destroy earlier evidence.
2. **The type checker rejects the alternative.** Keeping `!seen` while adding `.heard` uses the
   same `believes.<event>` slot both `!` and `.` — a `CardinalityClash` by v16's own analysis.
3. **v21 needs source-counting.** Corroboration ("three distinct people say it") requires one
   edge per source.

Consequences:

- Believing an event *is* holding at least one provenance edge under it. `forget` (subtree
  retract) still erases the whole belief, evidence and all.
- "Don't retell to someone who already heard it *from me*" is just `Not "…heard.<teller>"` — the
  one-shot marker falls out of the vocabulary.
- **Migration is a complete replacement** (no compatibility shim): `Prax.Witness.observable`
  deposits `….seen` (dot, not bang); `saw who event` becomes
  `Match (who ++ ".believes." ++ event ++ ".seen")`; the `beliefSentence`/`believesThat`
  `!value` helpers remain for *valued* issues (e.g. `resentedBy!yes`) and are simply no longer
  used for event beliefs. `WitnessSpec`/`VillageSpec` assertions update accordingly.

## 2. Compiled: `Prax.Rumor`

### Why authored per event-pattern

A single generic "share any belief" action is impossible (a query variable binds one path
segment; events like `stole.bob.loaf` are multi-segment) and would be wrong anyway, for the
same reason v19 rejected automatic observability: *what is tellable is authored vocabulary*.
`gossip` therefore mirrors `observable` — one declaration per event pattern.

### API

```haskell
-- | An action: tell a co-present hearer about an event you have evidence for.
-- The event pattern may use variables (e.g. "stole.Culprit.Item"); the FIRST
-- variable in the pattern is the event's subject (the person the rumor is
-- about), who is never offered as a hearer.
gossip :: CoPresence   -- who can be told (world vocabulary, as in Prax.Witness)
       -> [Condition]  -- the world's own extra gate (may be [])
       -> String       -- event pattern, e.g. "stole.Culprit.Item"
       -> String       -- action label, e.g. "[Actor]: tell [Hearer] that [Culprit] stole the [Item]"
       -> Action

-- | Condition: @who@ has hearsay evidence of @event@ (from anyone). A boolean
-- ∃ — 'Exists' — so multiple sources yield ONE row, not one binding per teller.
heard :: String -> String -> Condition   -- Exists [ Match (who .believes. event .heard.Src) ]
```

`gossip copresence gate pattern label` builds `action label conds [effect]` where, with
`subject` = the first variable segment of `pattern` (it is an error — `error`, loudly — to call
`gossip` on a pattern with no variable segment, since a rumor is about someone):

```haskell
conds =
  [ Match ("Actor.believes." ++ pattern) ]   -- any evidence; binds the pattern's variables
  ++ copresenceForHearer          -- copresence with "Witness" renamed to "Hearer"
  ++ [ Neq "Hearer" "Actor"
     , Neq "Hearer" subject       -- you don't tell bob about bob's theft
     , Absent [ Match ("Hearer.believes." ++ pattern ++ ".seen") ]      -- no news value
     , Not ("Hearer.believes." ++ pattern ++ ".heard.Actor") ]          -- one-shot per teller
  ++ gate

effect = Insert ("Hearer.believes." ++ pattern ++ ".heard.Actor")
```

The evidence condition is a **prefix match**: `believes.<event>` nodes exist iff some provenance
edge was deposited beneath them (witness and rumor deposits are the only writers, both write a
provenance leaf; `forget` retracts the whole subtree), and matching the prefix binds the
pattern's variables exactly once per known event regardless of how many provenance edges sit
below — no duplicate menu rows for a teller who both saw and heard. (An `Or` over the two
provenance shapes was considered and rejected: it unions bindings, so a teller with two evidence
paths — or two `heard` sources — would be offered duplicate rows of the same tell.)

`CoPresence` templates are written over the fixed variables `Witness`/`Actor` (v19). `gossip`
reuses them for the hearer by substituting the variable name `Witness` → `Hearer` in the
template's sentences (a mechanical rename; the template stays single-sourced in the world).

Spreading is **want-driven, not automatic**: a gossip-inclined character is authored with a want
that others know what it knows, and the ordinary planner does the rest — news travels, and stops
when no tellable hearer remains (all conditions exhausted).

### Not in scope

- Distortion/mutation in transit (the belief planted is the belief held).
- Lying (planting a belief the speaker holds no evidence for) — the secrets/deception tier.
- Automatic newsworthiness or decay of rumors.

## 3. Demo: the village grows

- **Carol becomes the gossip**: an authored want — for each other villager to have heard of the
  theft from her — so after witnessing she walks to the mill and tells dana *on her own*.
- **The evidential distinction is visible in play**: dana (hearsay only) gains
  `[Actor]: eye [Thief] with suspicion` — gated on `heard` *and* `Absent [seen]` (an eyewitness
  confronts instead; seen subsumes heard for the milder act) — with a milder trust hit (−5,
  reason `heardOfTheft`) — but **not** `confront`, which stays `saw`-gated (−10): "I saw you!"
  is not something hearsay licenses.
- **The relationship gate is demonstrated**: the village's gossip gate is "you don't gossip with
  someone you distrust" — `Absent [ Match "Actor.relationship.Hearer.trust.score!V", Cmp Lt "V" "0" ]`
  — so a sparse world (no scores) gossips freely and hostility closes the channel.
- The player shares the same affordance (tell whom you like; withhold what you know).

## 4. Tests (TDD)

- `RumorSpec` (minimal inline fixture, like `WitnessSpec`): telling plants
  `Hearer.believes.<event>.heard.<teller>`; the subject of the rumor is never offered as hearer;
  retelling to the same hearer is not offered (one-shot per teller); a second teller adds a second
  `heard` edge (corroboration); a hearer who `saw` the event is not offered; the distrust gate
  closes the channel; `gossip` on a variable-free pattern errors loudly.
- `WitnessSpec`/`VillageSpec` migration: deposits/assertions use `.seen`.
- `VillageSpec` additions: carol autonomously spreads (driveIdle-style: after the theft, dana
  eventually holds `dana.believes.stole.bob.loaf.heard.carol`); dana can *eye with suspicion* but
  not *confront*; carol (eyewitness) can confront but is not offered *eye with suspicion*
  (seen subsumes heard for the milder act — gate suspicion on `heard` + `Absent [seen]`).
- Regression: full suite green; all worlds `prax check` clean (the cardinality checker now guards
  the all-`.` provenance scheme); bar/play goldens untouched.

## 5. Out of scope (parked)

- Reputation axioms over evidence sets (v21, next).
- Gatherings/calendar as the mixing dynamic (backlog).
- Belief revision on contradiction (a heard rumor vs. seen counter-evidence).
