# v40 — Hygienic machinery variables: one namespace, one guard

First of the user-queued foundations passes (strict improvements: elegance/usability up,
complexity flat or down). The defect: every combinator that generates conditions splices
its internal variables into the same flat namespace worlds author in, so each ships a
bespoke reserved-name list and guard walker — `Drift` (D/D2/Now), `Rng` (S/S2/S3/R),
`Faction` (W/F via `reservedClash`), `Confession` (its Count-idiom internals via
`reservedIn`), `Blackmail` — five copies of one defense, and the v30/v31 capture bugs
happened in the gaps between them.

## The two tiers (the probe's finding, load-bearing)

- **Interface variables are the authoring contract, not the defect**: `Actor`, `Owner`,
  `Witness`, `Hearer`, `Seer`/`Seen`/`Spot`, `Anyone` — worlds write these deliberately
  to mean what the engine grounds them to. They keep their names and their documentation.
- **Machinery variables are combinator internals** the author should never see or be able
  to collide with: the due-gate's, the die's, the generated axioms'. These are the round's
  subject.

## Design

1. **The namespace**: every machinery variable is renamed `Prax<Name>` (`PraxD`,
   `PraxNow`, `PraxS`, `PraxW`, `PraxEvicted`, …) — uppercase-first so `symIsVar`'s
   parity rule is untouched (underscore is unavailable: it belongs to the pseudo-character
   naming convention), unmistakable in any trace, and impossible to write by accident.
   Pure alpha-renaming of generated conditions: variable names never reach facts, labels,
   or serialized state, so goldens are expected byte-identical (the nets gate it).
2. **One guard, one home**: a shared boundary check beside the v38 walkers —
   `assertAuthored :: String -> [Condition] -> [Outcome] -> ()`-shaped (name it per the
   house style; loud error naming the offending variable and the combinator) — that every
   combinator calls on its AUTHOR-SUPPLIED arguments. The five bespoke lists and their
   private walkers (`reservedClash`, `reservedIn`, Drift's and Rng's lambda-guards)
   are DELETED; each site becomes one call. The rule the guard enforces is single:
   authored fragments may not contain `Prax`-prefixed variables.
3. **The world-source gate**: one grep-gate line (the repo's existing gate discipline)
   over `src/Prax/Worlds/` and authored test fixtures — no `Prax`-prefixed variable
   appears in hand-authored pattern strings. Belt for the braces.
4. **Documentation truth**: each combinator's haddock drops its bespoke reserved-name
   sentence for a pointer to the one rule; the interface-variable contract (tier one)
   gets stated once, where authors look (the `Prax.Types.practice`/`action` area or the
   module that owns authoring, per the plan's probe).

Explicitly NOT in scope: gensym/true hygiene machinery (provenance-tracked variable
allocation would be complexity up for no additional safety once the namespace is
unwritable-by-accident); any change to `symIsVar`, interning, cooked formats, or engine
semantics; renaming any interface variable.

## Verification

- Alpha-invariance is the exactness claim: goldens byte-identical, ViewInvariant green,
  full suite green. Any golden movement means a variable name leaked into semantics
  somewhere — BLOCK and trace (that would be its own discovered defect).
- The consolidated guard pinned per combinator (each old guard's spec test updates to
  the new error message; one new shared-guard unit pins the walker coverage — conditions
  AND outcomes, subquery internals included, per the v38 walkers it reuses).
- The grep-gate runs in the suite (or as a check-tool rule — plan decides with the
  existing gate mechanism).
- Mutation evidence where a pin's discriminating power isn't self-evident (a combinator
  called WITHOUT its guard must fail the guard pin).

## Out of scope

v41 (analysis unification), v42 (dead-condition lint), v43 (the hygiene bundle) — queued
separately, in order.
