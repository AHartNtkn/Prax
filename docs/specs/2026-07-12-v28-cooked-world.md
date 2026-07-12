# v28 — Cook the authored world once (exactness contract carried over)

User directive: continue the constant-factor work. The post-v27 profile: ~48% of a round is
still string splitting/rebuilding (`tokens`/`trim`/`parseNames`/`ground`), all of it on
**strings that never change** — authored condition patterns and outcome templates re-split per
query call and per grounding, and `ground` building strings that the next step immediately
re-splits.

## The rule (unchanged)

Exact only: goldens byte-identical, the ViewInvariant net green after every task, identical
`readView` contents. The nets are the proof; no expected value may be adjusted.

## Design: the cooked pipeline

Authored vocabulary is compiled to token form **once per world**, and the hot paths run on
tokens end to end. Strings remain the authoring surface and the display/serialization surface;
they stop being the computation surface.

1. **`Prax.Cooked`** (new module): mirrors of the hot authored types with patterns pre-split —
   `CookedCondition` (a `Condition` whose `Match`/`Not` carry `[String]` names and whose
   nested lists recurse; non-pattern constructors carried through), `CookedOutcome`
   (`Insert`/`Delete` carry `(tokens, hasBang, evictionShadowNames)`; `ForEach`/`Call`
   recurse), plus `cookCondition`/`cookOutcome`/`groundNames` (substitute bindings into a
   name list — no string rebuild) and `groundCookedCondition`/`groundCookedOutcome`.
2. **The state carries the cooked world**: a `cookedDefs` companion to `practiceDefs`
   (cooked conditions/outcomes per action, cooked initOutcomes, cooked function cases),
   cooked want conditions wherever wants are evaluated (character wants and vocabulary
   desires), and cooked axiom bodies for the closure loop — all rebuilt by the same Engine
   helpers that already maintain `improvables`/`footprint` (`retable` and friends). The
   footprint/negFootprint classification lists are already tokenized (v27).
3. **Consumers switch to the cooked forms**: `possibleActions` (grounding and querying on
   names), `performAction`/`performOutcome` (outcome grounding on names; the
   insert/retract/classification pipeline takes names), `evaluate`/`selfWants`/
   `believedWants` (cooked wants), `closure`/`closureFrom`'s `deltaJoin` (cooked bodies),
   `ForEach` (cooked interior). `Prax.Query` gains the cooked-condition query entry; the
   String `query` remains as cook-then-run for cold/external callers (tests, Inspect, REPL).
4. **No public authoring change**: worlds, tests, and the eDSL keep writing strings.

## Verification

- Goldens + ViewInvariant net + full suite (314) green after every task; zero warnings;
  hlint; `prax check` ×7; the v26/v27 grep-gates.
- Cooked-equivalence unit tests: cooked query/grounding results equal the string path on
  representative fixtures (including `!` paths, variables, ForEach interiors).
- Honest perf report: before/after profiled round and suite times.

## Gated next step (v29, decided by the post-v28 profile)

Interning path segments to `Int` keys (IntMap tries, symbol table in the state) is a deep
representation change through `Db` and `Bindings` — NOT the simple cut this round is. It is
commissioned only if the post-v28 profile still shows segment comparison/allocation dominant;
recorded as not-warranted otherwise.

## Out of scope

Approximate anything (banned); depth/semantics changes; new mechanics (standing directive).
