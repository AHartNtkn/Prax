# v45 — Protected families: engine-owned facts get one guard, not a whitelist of turn

First of the four audit-queued rounds (v45 protected families → v46 the narrator dies →
v47 function registry → v48 generality bundle; audit inventories at
`.superpowers/sdd/audit-*.md`). The finding (two auditors, independent convergence):
v44 generalized the defect — engine mechanism living in authored/queryable space — but
implemented the enforcement for exactly one family. `clockWriteErrors` guards `turn`
alone, while the structurally identical families sit exposed:

- **`seed!N`** (the die's stream position): an authored `Match "seed!S"` reads fate —
  predicting every future draw; an authored `Insert "seed!7"` rigs it. The
  determinism/replay guarantee v38 sells is unenforced.
- **`sceneEntered!N`** (the scene epoch): authored writes defeat every timed junction;
  authored reads gate on raw machinery time.
- **`contradiction`** (the ⊥ witness): an authored `Insert "contradiction"` fakes a
  permanent logical contradiction.

Each mechanism assumes it is its family's sole writer; an authored touch corrupts it
SILENTLY — the no-silent-failures principle violated at the authoring boundary.

## The design

**The threat model, stated first** [AMENDED after the task review demonstrated a
repl-level counterexample and the amendment adjudicated it]: Prax is a COMPILER. Its
authoring surface is the eDSL combinators, the JSON script format, and the world
sources — each of which carries a Prax-namespace guard (v40/v43/v44 combinator
boundaries; the v44 JSON compile guard; v40's GateSpec literal scanner over
`src/Prax/Worlds/`). Raw Haskell construction against the `Outcome`/`Condition` ADTs
is COMPILER-LEVEL code — the same trust level as editing the engine — and is
definitionally outside any in-language guard's reach: read the unforgeability claim as
applying to raw Haskell and it proves too much, since the compiler's own combinators
could not emit these forms either. The review's counterexample (a hand-built
`Match "seed!PraxS"` action) is compiler-level code doing what that level can do; it
is not an authored-surface forgery.

**One check, one table, one exemption unforgeable AT THE AUTHORING SURFACE.** The
keystone comes from v40: the `Prax` variable namespace is banned in authored fragments
at every door of the surface above, so a pattern whose value positions are all
Prax-namespaced variables is machinery by construction — no AUTHOR can counterfeit it.
The mechanisms' own compiled accesses have exactly that shape (probed: `draw` reads
`seed!PraxS`, writes `seed!PraxS3`; the scene stamp writes `sceneEntered!PraxNow`,
`clockReached` reads `sceneEntered!PraxE`). Therefore:

- `Prax.TypeCheck`'s `ClockWrite` generalizes to a `ReservedFamily` error (rename, no
  dual) driven by a declared table:
  | family | writes | reads |
  |---|---|---|
  | `turn` | forbidden | FREE — the documented authored time interface (`sightedWithin`, gathering gates) |
  | `seed` | machinery-shape only | machinery-shape only — fate is not world-observable |
  | `sceneEntered` | machinery-shape only | machinery-shape only |
  | `contradiction` | forbidden | free (a bare zero-value family; reads cannot corrupt) |
  where "machinery-shape only" = every value position beyond the family head is a
  Prax-namespaced variable; anything else (a literal, a plain variable, a bare subtree
  match on the family) is a loud `ReservedFamily` error naming the site and family.
- **Scan sites**: everything authored — practice outcomes and axiom heads (the existing
  `clockWriteErrors` walk) PLUS authored conditions and schedule-rule bodies (the read
  side is new; a user schedule rule reading `seed!S` is the same leak). Test code is
  untouched (typeCheck scans definitions, not test drivers — RngSpec's direct db unify
  stays legal).
- Family path constants come from their owning modules (`turnPath` in Types, `seedPath`
  in Rng, `sceneEntered` gains a named exported constant in Script); the table lives
  with the check.

## Deferred, with reasons (not omissions)

- **`atSince`** (audit judgment-call D): its stamp value is bound by `Now` — the
  DOCUMENTED contract variable of sighting templates — so the machinery-shape rule
  cannot distinguish the sighting rule's own write from an authored one without
  breaking that contract. Protection waits for a deliberate contract decision; recorded
  as residue in the LEDGER, not silently dropped.
- **`storyAdvanced`** (audit finding 1's family): dies entirely in v46 with the
  narrator; guarding it now would need a practice-id whitelist — a hack for a family
  with one round to live.

## Verification

- RED-first per family and polarity: an authored write of each family flags with the
  pinned message; an authored read of `seed`/`sceneEntered` flags; the mechanisms' own
  compiled shapes do NOT flag (the exemption pin — a world using `draw` and a timed
  junction typeChecks clean); `turn` reads stay free (`sightedWithin` pin); the
  all-shipped-worlds-clean pin extends over the new check.
- Mutation evidence: drop the Prax-var exemption → the exemption pin fails (draw worlds
  flag); drop the read-side scan → the seed-read pin fails.
- Goldens byte-identical; no engine, format, or Persist change — guards on illegal
  input only.

## Out of scope

v46 (narrator → schedule rules), v47 (function registry), v48 (generality bundle) —
queued. Read-guarding `turn` (a documented interface). Moving `seed`/`sceneEntered`
into `PraxState` fields (the fuller fix the audit sketched — it requires the mechanisms
to stop READING them as facts, which is v46-adjacent redesign; the guard closes the
hazard now and loses nothing if that lands later).
