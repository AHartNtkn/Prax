# v43 — The hygiene bundle: six small holes, closed loudly

Fourth and last of the user-queued foundations passes. Six independent items, each a
guard or extraction with ZERO behavior change for shipped worlds (goldens byte-identical
throughout); bundled because none carries a round's weight alone.

## 1. Function-name collision guard

`Prax.Engine.lookupCookedFn` resolves a `Call` by searching practices first-wins in
alphabetical order; nothing forbids two practices declaring the same `fnName`, so a
collision silently shadows one function (v41 recorded this as the reason its fn-pool
bias flip was unobservable — "v43's guard makes a collision impossible"; this is that
guard). `definePractices` errors loudly on a duplicate `fnName` across OR within
practices, naming both homes.

## 2. Action-name collision guard

`caName` is a lookup key — `Prax.Engine.groundedDeltaAnchors` finds the acted action by
name within its practice (first match), and the v35 intention store serves standing
intentions by `gaActionId`. Two same-named actions in one practice make both silently
wrong. `definePractices` errors loudly on a duplicate `actionName` within a practice.
(Across practices the key is qualified by `gaPracticeId` — no constraint.)

## 3. Clock extraction from Sight

The turn counter lives inside `Prax.Sight`'s perception action: the `turn` family's
name is hardcoded separately in Drift's due-gates, TypeCheck's `ClocklessDrift`, and
Sight's tick — three homes for one concept, and a world cannot have time without
perception. New `Prax.Clock` OWNS the family: the path constant, the tick
condition/outcome fragments (`turn!PraxN` → `turn!PraxM`), the `turn!0` seed, and a
standalone bodiless `clockP`/`clockChar` for worlds that want drift without sight.
`Sight` composes Clock's fragments into its existing action (shipped worlds:
byte-identical — same practice, same action, same conditions); Drift and TypeCheck
import the constant instead of restating `"turn"`. `ClocklessDrift`'s fix-message names
both providers (the sight ticker or the standalone clock).

## 4. Persist version header

`serializeState` writes bare sentences; a file from a future format would deserialize
into silent garbage or a confusing mid-file crash. The output gains a first-line header
(`prax-state v1`); `deserializeState` fails loudly on a missing or unknown header. No
compatibility path: no saved files exist anywhere (established at the v39 review — no
format promise has ever been shipped), so the header simply becomes part of the format.

## 5. Trailing-operator rejection

Probed live (scratch `V43ExclProbe.hs`): `insert "at.bob!"` and `insert "at.bob"`
produce UNEQUAL `Db`s that SERIALIZE IDENTICALLY — a trailing `!` sets the leaf's
exclusion flag, which no query, insert, or serialization ever reads again (write-only
state), so `Eq` distinguishes semantically identical states and serialize→load does not
round-trip `Db` equality. On a node with existing children it is worse: the flag marks
edges exclusive WITHOUT evicting, fabricating an invalid multi-child exclusive node.
The fix is at the authoring boundary: `Prax.Db.tokens` (and thus `internTokens`,
`insert`, every string-side entry) rejects a trailing operator (`.` or `!`) with a loud
error naming the sentence. The Sym-level core (`insertToks`) is untouched — machinery
never generates trailing operators (verified by construction during implementation: all
generated token lists come from `internTokens` or `groundTokens` over authored shapes).

## 6. Splice-point guards (the v40 Lows)

- `driftP`/`sightP` pass `forbiddenSplices = []` today, but both splice authored
  fragments into actions gated `Eq "Actor" <tickerName>` — an authored body using
  `Actor` silently binds the TICKER character, never a mover. Both now pass
  `["Actor"]`. (If any shipped drift/sight fragment turns out to use `Actor`, that is a
  live bug find: BLOCK and surface it, don't accommodate it.)
- `Prax.Rumor.gossip` and `Prax.Deceit.lie` splice authored gates, fabrications, and
  event patterns with NO guard at all — latent capture sites the v40 review flagged.
  Both gain the shared boundary check (`authoredVarClash`/`authoredPatClash`,
  Prax-namespace only; `Hearer`/`Actor`/the pattern's own variables are their authoring
  contract and stay free). `conceal`'s existing variable-free guard already suffices.

## Verification

- Every guard RED-first: a fixture that triggers it (colliding fn names, colliding
  action names, a trailing-`!` sentence, an `Actor`-using drift body, a `PraxD`-using
  gossip gate, a headerless/wrong-header save) errors with the pinned message; the
  legal twin stays quiet.
- Item 5's probe result becomes a pin: the trailing-form insert now errors; the
  round-trip Eq anomaly is unconstructible from strings.
- Item 3 and 4 exactness: goldens byte-identical (Clock extraction is code motion;
  the header changes no in-memory state); `PersistSpec`'s round-trip pins updated for
  the header in the same commit as the format change, itemized.
- Suite green throughout; zero warnings; hlint; `prax check` ×7.

## Out of scope

Sort-inference completion, any new world content, and the banked emotion/chronicler
items — the queue ends here; what comes next is the user's call.
