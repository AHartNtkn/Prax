# v25 — Persona (`Prax.Persona`): traits as volition

Backlog item **Personality → volition** (`docs/LEDGER.md`, "Sandbox extension backlog", Tier 2),
and the honest closure of v18's recorded gap: *"traits are NOT compiled to behaviour: no source
gives a trait→desire mapping, so inventing one would be a heuristic."* The objection dissolves
the right way — the mapping becomes **authored world vocabulary**, per trait, per world, built
on `Prax.Minds`: a trait is a *named bundle of vocabulary desires*, and which desires a trait
bundles is as much authored content as a practice's affordances.

The load-bearing mechanism was verified against the live engine before this spec was written:
one **transparency axiom** (a three-way join over `trait.M.T` ∧ `traitDesire.T.D` ∧
`character.P`) derives presumed motive-beliefs across the cast, `predictMove` fires from
temperament alone (a stranger anticipates the industrious character's work having never seen
it), absent trait facts derive nothing, and striking the trait fact dissolves the presumption.

## 1. The model

A **trait** is three things at once, from one declaration:

1. **Volition** — its bundled desires join the world vocabulary, and a bearer carries their
   names in `charDesires` (so the bearer *acts* on them: `selfWants`).
2. **A fact** — `trait.<who>.<name>`, queryable by conditions and axioms like any world state
   (v18's declarative traits, unchanged).
3. **Legible temperament** — traits are *visible* folk psychology: everyone can see what sort
   of person you are, so everyone presumes the bundled motives. One axiom serves every trait,
   because the bundle itself is data (`traitDesire.<trait>.<desire>` facts).

The contrast with v24 is the point, and becomes a shipped demo: **bob's pursuit had to be
learned by observation** (someone watched him sweep); **a trait-borne disposition is presumed
from temperament alone**. Between them: `professed`/`conventional` (v23). Four presumption
sources, one belief shape, one prediction machinery.

**Hidden dispositions need no machinery**: a trait *is* visible temperament by definition here;
a secret inclination is just a plain desire that is neither trait-borne, professed, nor
conventional — already expressible since v23.

## 2. API (`Prax.Persona`)

```haskell
data Trait = Trait
  { traitName    :: String     -- a single path segment (loud error otherwise)
  , traitDesires :: [Desire]   -- the bundle (names should be distinctive; they join the
  }                            --  world vocabulary as-is)

-- | All bundled desires of a trait vocabulary, for PraxState's `desires`.
--   Loud error on duplicate desire names across traits (a name is identity).
personaVocabulary :: [Trait] -> [Desire]

-- | A character bears a trait: its desire names join their charDesires.
bearing :: Trait -> Character -> Character

-- | Setup facts for a roster: trait.<who>.<name> per bearing, plus the
--   traitDesire.<trait>.<desire> data facts (once per trait).
personaSetup :: [Trait] -> [(String, [Trait])] -> [Outcome]

-- | Temperament is legible: one axiom for every trait.
--   trait.M.T ∧ traitDesire.T.D ∧ character.P ⇒ P.believes.desires.M.D.presumed
transparent :: Axiom

-- | The cast generator: characters from a roster of (name, traits), with the
--   setup facts to match — the v18 sketch, generating volition. Deterministic
--   assembly (sampling/randomized casts belong to the stress tooling, banked).
cast :: [Trait] -> [(String, [Trait])] -> ([Character], [Outcome])
-- cast vocab roster = ( [ foldr bearing (character n) ts | (n, ts) <- roster ]
--                     , personaSetup vocab roster )   -- (plus character.<n> facts)
```

Worlds using personas add `personaVocabulary traits` to `desires`, `transparent` to `axioms`,
and fold `personaSetup`/`cast` into world assembly. `cast`/`personaSetup` emit `character.<n>`
facts for roster members (the axiom quantifies over them); the world asserts them for any
non-roster cast it wants inside the presumption audience. **No duplicate desire names may reach
`desires`**: `personaVocabulary` guards within the trait vocabulary, and a desire that enters
through a trait bundle must not ALSO be listed standalone — a duplicate entry would double-count
the believed model's utility.

## 3. Demo: the village gains finn — and a legibility contrast

- **Trait vocabulary**: `industrious = Trait "industrious" [ earnBreadPursuit ]` — the v24
  synergy cashed: a trait bundles a *project disposition*, so temperament explains undertaking.
  The pursuit now enters the village vocabulary **through the bundle**: the standalone
  `desires = [earnBreadPursuit]` entry from v24 is replaced by
  `desires = personaVocabulary [industrious]` (same one desire, no duplicate). bob keeps his
  `charDesires = ["pursues-earnBread"]` **without** the trait fact — his disposition is
  unheralded, which is exactly what makes the legibility contrast below possible.
- **finn joins via the generator**: `cast` builds him bearing `industrious`, starting at the
  mill. He undertakes his own `earnBread` instance (one per owner — two honest bakers don't
  collide) and works it exactly as bob does.
- **The legibility contrast, as tests**: from t=0, *before finn has done anything*, everyone
  presumes his industry (`…presumed` in the view) and `predictMove` anticipates his next stage
  once one is available — **temperament read at a glance**. bob, whose disposition is a plain
  `charDesires` entry with no trait fact, still requires the v24 observation chain (the
  witnessed sweep) before anyone can predict him. Same machinery, two epistemics, both honest.
- The dormancy story composes: finn's presumed pursuit predicts nothing until he undertakes
  (the believed model gains from no available move) — temperament tells you *what he's like*,
  not *what he's doing*, until he's doing it.

## 4. Tests (TDD)

- `PersonaSpec` (minimal inline fixture, the probe pinned): `bearing` extends `charDesires`;
  `personaVocabulary` collects bundles and errors loudly on duplicate desire names; trait-name
  path-segment guard; `personaSetup` emits trait + traitDesire facts; `transparent` derives
  presumption for bearers only, across the cast, defeasibly (strike the trait fact, the
  presumption dissolves — while a `.heard`-sourced belief about the same desire would survive);
  temperament-alone prediction (the probe's `Just`), and dormancy composition (presumed pursuit
  of an instanceless project predicts `Nothing`); `cast` assembles characters + facts
  deterministically.
- `VillageSpec` additions: finn presumed industrious by all from t=0; finn undertakes and works
  his own instance in free play alongside bob's (both complete; instances distinct); the
  contrast — before any observation, `predictMove` works for finn (temperament) and not for bob
  (needs the sweep); the older observation chain for bob still passes unchanged.
- Regression: full suite green (278 baseline); `prax check` all 7 worlds (the new axiom passes
  the axiom analysis); `cabal build all` zero warnings; hlint clean. Cast-size effects on drive
  turn budgets are sanctioned parameters (state + trace), as established.

## 5. Out of scope (parked deliberately)

- **Script-layer wiring**: v18's `withTraits` still records declarative facts; compiling
  `castTraits` through a world-supplied Trait vocabulary needs a `compile` signature change and
  JSON schema thought — banked (the principled mapping now *exists*; the script layer can adopt
  it in its own round).
- Randomized/sampled cast generation (a stress-tooling extension); trait acquisition/change at
  runtime; anti-traits/aversions (negative bundles work today — a `Desire` may carry negative
  weight — but a designed idiom for them is future work).
- Hidden temperaments (see §1 — already expressible as plain desires; no machinery wanted).
