# v31 — Factions & kinship (`Prax.Faction` + `Prax.Kin`), folded

Two backlog rows folded per user direction, because they share one spine: **membership**.
A household is a small faction; kinship *generates* memberships (birth ties, marriage moves
them); faction axioms turn membership into solidarity. One vocabulary, two generators — and
the feud world's bespoke pairwise axioms are the thing being generalized, so v31 refactors
`Prax.Worlds.Feud` onto `Prax.Faction` outright (no dual systems; `FeudSpec` unmodified is
the preservation pin).

## 1. `Prax.Faction`

- **Membership is a base, single-slot fact**: `member.<who>!<faction>` — one primary
  allegiance, and the `!` is the semantics: joining, defecting, and marrying-in are all the
  same exclusion overwrite. (Multi-affiliation guilds are a later extension; the primary
  allegiance is what solidarity keys on.)
- **Axioms** (the feud generalization):
  - `comrades :: Axiom` — `member.X!F ∧ member.Y!F ∧ X≠Y ⇒ allied.X.Y`. The derived fact
    KEEPS the name `allied`, so everything downstream of the feud's old base facts
    (mutuality axiom, "enemy of my ally", `societyP`'s affordances) consumes it unchanged.
  - `solidarity :: String -> Axiom` (raw, feud-era) — the old wronged→resents chain stays
    as-is in feud; solidarity-through-alliance already falls out of `comrades` + the
    existing `resents.A.B ∧ allied.A.C ⇒ resents.C.B`. Nothing new needed for feud beyond
    `comrades` — stated explicitly so the module doesn't grow speculative machinery.
  - `factionStanding :: String -> String -> Axiom` (belief-gated, K-era worlds) —
    `W.believes.<pat> ∧ member.<victim-var>!F ∧ member.W!F ⇒ regards.W.<offender-var>.<label>`
    — an offense against my faction-mate, *that I believe happened*, makes me regard the
    offender. Reputation still flows only from belief (K-discipline); this is
    `standingUnless`'s shape with a membership join. Shipped and spec-tested; wired into a
    K-world demo only if the plan's probe shows it dramatizes cleanly without disturbing
    the village goldens — otherwise FactionSpec-pinned for the next K round (stated
    decision either way).
- Guards: faction names are single path segments (the established loud-error idiom).

## 2. `Prax.Kin`

- **Base vocabulary**: `parent.<parent>.<child>` (multi-valued), `married.<a>.<b>`
  (asserted once; symmetry is derived).
- **`kinAxioms :: [Axiom]`**: marriage symmetry; `sibling.X.Y` (shared parent, X≠Y);
  `grandparent.X.Z`; `inLaw.X.Y` (spouse's parent / sibling's spouse). Pure derivation —
  the closure engine's home turf; retraction-safety for free (a dissolved marriage
  un-derives the in-laws).
- **`wed :: String -> String -> [Outcome]`** — `wed joiner stayer`: insert the marriage
  fact AND `Insert ("member." ++ joiner ++ "!" ++ …stayer's faction…)`. The joining
  direction is the AUTHOR's choice per wedding (who moves households is world content, not
  module policy — haddocked). Membership transfer is the `!` overwrite: the fold's payoff
  in one line. (Reading the stayer's faction requires a value, not a pattern — `wed` takes
  the target faction explicitly: `wed joiner faction spouse`; three arguments, no hidden
  query.)
- **Offices & succession** (exclusion IS succession): `office.<name>!<holder>` +
  `succession :: String -> Action` — a claim action gated on `dead.<holder>` and the
  claimant being the holder's child (`parent.<holder>.Actor`), effect
  `Insert ("office.<name>!Actor")`. Any child may claim; the slot takes one — first
  motivated claimant wins, which is honest exclusion semantics, not primogeniture
  (age doesn't exist; inventing "eldest" would be an unprincipled fact). Inheritance of
  holdings beyond offices: banked (transfer mechanics deserve their own look).

## 3. Demos

- **Feud, refactored and grown** (the main stage): the pairwise `allied.*` setup facts are
  DELETED and replaced by two houses (`member.X!<house>` facts); `comrades` derives what the
  base facts used to assert; `FeudSpec` passes UNMODIFIED (the generalization's proof).
  Then the wedding beat: a marriage across the feud (`wed` moves the bride —
  direction authored) and the derived world flips — her alliances now derive from the new
  house, so the resentment chains she participates in flip sides, asserted at the
  derivation level (readView before/after) plus one driven beat if the probe shows the
  planner dramatizes it (the feud's affordances key off derived enmity, so behavior should
  follow the flip; BLOCK-with-trace, never tune, if not).
- **Succession** (KinSpec fixture, no world drama needed): a holder dies (the `dead.*`
  idiom), two children, a claim resolves the office; the un-derivation sanity checks
  (dissolving a marriage retracts in-laws).
- **The village is untouched** this round — no golden churn; `factionStanding`'s village
  wiring is the stated conditional above.

## 4. Tests (TDD)

- `FactionSpec`: comrades derivation (+ X≠Y, cross-faction negative); membership overwrite
  semantics (defection un-derives old alliances — retraction-safety); `factionStanding`
  belief-gating (an unbelieved offense moves no faction-mate; a believed one moves only
  co-members); guards.
- `KinSpec`: each kinAxiom (positive + a negative each); wed's two facts + membership
  overwrite; dissolution un-derives; succession (gated on death, child-only, exclusion
  resolves competing claims to one holder).
- `FeudSpec`: UNMODIFIED and green — the refactor's contract.
- Goldens: feud is not golden-pinned (village/bar/intrigue are); ViewInvariant covers feud
  drives and must stay green through the axiom changes. Suite green throughout; usual gates.

## 5. Out of scope

- Multi-affiliation, faction offices/leadership wants, inheritance of holdings,
  age/primogeniture, births, divorce-as-action (dissolution is tested via raw retraction),
  village faction wiring (conditional above) — banked.
- Any engine change (pure vocabulary + axiom round; the closure layer already does the work).
