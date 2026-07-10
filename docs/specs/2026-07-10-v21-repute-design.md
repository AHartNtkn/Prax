# v21 — Reputation (`Prax.Repute`)

Backlog item **⤷K Reputation** (`docs/LEDGER.md`, "Sandbox extension backlog"): derivation axioms
from evidence to standing, defeasible like the feud, with corroboration counting v20's sourced
edges. Completes the K → rumor → repute arc: what happens in front of people travels beyond them
and *settles into what people think of you* — and materially changes how they treat you.

All mechanisms below were verified empirically through `Prax.Derive.closure` before this spec was
written: a `Not`-guarded axiom (the first negation-guarded axiom in the codebase), defeater
dissolution, and a Count-threshold axiom deriving from *derived* facts across fixpoint rounds.

## 1. Standing is per-observer and derived

```
regards.<observer>.<subject>.<label>     -- derived, lives in the defeasible view only
```

Derived by axiom from evidence: any observer holding `believes.<deed>` (seen or heard — the
prefix match works in axiom bodies exactly as in `gossip`) comes to regard the deed's subject
under the label. Nobody *stores* reputation; it is a way of reading the evidence, so it inherits
information asymmetry (only those the news reached hold the regard) and defeasibility (retract
the support and the standing is gone on the next read).

## 2. Defeaters: atonement, not amnesia

The feud dissolves by deleting the base wrong. Reputation cannot: **the deed happened and people
saw it** — deleting their beliefs would be forced amnesia. Instead the axiom carries a negation
guard over a base fact:

- `standingUnless "stole.Culprit.loaf" "atoned.Culprit" "thief"` derives regard only while
  `atoned.<culprit>` is absent. One base insertion dissolves every derived regard at once, while
  every belief (the memory of the deed) persists untouched. Standing = *unatoned* believed deeds
  — forgiveness without forgetting.
- **Constraint (documented in the haddock):** the defeater pattern must name only *base* facts
  (never derived heads), keeping the closure stratified. Village obeys it (`atoned.*` is written
  only by the amends action).

## 3. Notoriety: corroboration cashed

```haskell
notoriety :: String -> Int -> Axiom
-- notoriety "thief" 3 ⇒ axiom
--   [ Match "regards.W0.T.thief"
--   , Subquery "Rs" ["W"] [ Match "regards.W.T.thief" ], Count "N" "Rs", Cmp Gte "N" "3" ]
--   [ "notorious.T.thief" ]
```

`notorious.<subject>.<label>` is a *global* derived fact holding when at least `k` distinct
observers hold the regard — counting **derived** facts (second fixpoint round; verified). The
threshold is an authored world parameter with stated meaning; the village uses **3** = every
villager except the thief ("the whole village knows"). Below threshold, no notoriety.

## 4. API (`Prax.Repute`)

```haskell
standing       :: String -> String -> Axiom            -- deed pattern → label (no defeater)
standingUnless :: String -> String -> String -> Axiom  -- deed pattern → defeater pattern → label
regardedAs     :: String -> String -> String -> Condition
                  -- regardedAs observer subject label = Match "regards.<observer>.<subject>.<label>"
notoriety      :: String -> Int -> Axiom               -- label → threshold
```

- `standing pat label` = `axiom [ Match ("Regarder.believes." ++ pat) ]
  [ "regards.Regarder." ++ subject ++ "." ++ label ]` where `subject` is the pattern's **first
  variable** (the same subject convention as `gossip`; loud `error` if the pattern has none).
- `standingUnless pat defeater label` adds `Not defeater` to the body (the defeater may use the
  pattern's variables, e.g. `"atoned.Culprit"`).
- Reserved variable: **`Regarder`** (the `Witness`/`Hearer`/`Actor` convention) — deed patterns
  must not use it. `notoriety` uses its own internal variables (`W0`/`W`/`T`/`Rs`/`N`).

## 5. Demo: the village closes its arc

Additions to `Prax.Worlds.Village` (this is the first world with **both** `axioms` and the full
information stack):

- **Axioms**: `[ standingUnless "stole.Culprit.loaf" "atoned.Culprit" "thief", notoriety "thief" 3 ]`.
- **Shun** (feud's move, regard-gated): `[Actor]: shun [T]` gated on
  `regardedAs "Actor" "T" "thief"` + `Neq "T" "Actor"` + one-shot (`Not "shunned.Actor.T"`);
  effect `Insert "shunned.Actor.T"`. Villagers (you excluded — the player decides) want the
  *condemned* shunned — the want is regard-conditioned:
  `Want [ Match "shunned.<n>.T", Match "regards.<n>.T.thief" ] 5` for carol and dana (feud's
  weight). The conditioning matters: with an unconditional shun-want, a post-atonement relent
  would be a net-zero tie (+5 shun-want vs −5 stale-shun) decided by tie-break; conditioned, the
  shun-want evaporates with the regard and relent wins by a clear −5 → 0.
- **Amends**: `[Actor]: return the loaf with apologies`, gated on `Match "holding.Actor.loaf"` +
  `Exists [ Match "regards.W.Actor.thief" ]` (you only make amends when someone thinks ill of
  you); effect `Delete "holding.Actor.loaf"`, `Insert "stall.loaf"`, `Insert "atoned.Actor"`.
  **Bob's motivation is authored**: he values his standing at `Want [ Match
  "regards.Other.bob.thief" ] (-10)` per regarder against the loaf's `+10` — one person knowing
  is a wash; **the whole village knowing is what tips him** into giving the bread back.
- **Relent** (the forgiveness beat): `[Actor]: relent toward [T]`, gated on
  `Match "shunned.Actor.T"` + `Absent [ Match "regards.Actor.T.thief" ]`; effect
  `Delete "shunned.Actor.T"`. Driven by a want *against* stale shuns: carol and dana get
  `Want [ Match "shunned.<n>.T", Absent [ Match "regards.<n>.T.thief" ] ] (-5)` — you don't keep
  shunning someone you no longer condemn.
- **Scene lines** (`app/Main.hs`, the village needs its own lines like the feud's): render
  `regards.W.T.thief` ("W regards T as a thief"), `notorious.T.thief` ("T is notorious as a
  thief"), `shunned.A.T` ("A is shunning T"), `atoned.A` ("A has made amends") — all read via
  `readView` (derived facts included), following the existing feud rendering idiom.

The full autonomous arc, no scripting: theft → witnessing → rumor → three regards → notoriety →
shunning → bob atones → regards and notoriety dissolve → villagers relent. Memory persists
throughout (beliefs are never deleted).

## 6. Tests (TDD)

- `ReputeSpec` (minimal inline fixture, beliefs inserted directly): `standing` derives per-observer
  regard from seen AND from heard evidence; a non-believer holds no regard; `standingUnless`'s
  defeater dissolves regard on the next read while beliefs persist; `notoriety` absent below
  threshold, present at threshold (counting derived regards); `regardedAs` shape; first-variable
  subject convention (two-variable deed pattern); loud `error` on a variable-free pattern;
  a `regardedAs`-gated *action* sees the derived fact (preconditions read the view).
- `VillageSpec` additions: after the rumor spreads, exactly you/carol/dana regard bob (bob holds
  no self-regard); `notorious.bob.thief` holds at three; shun offered only to regarders; the full
  autonomous arc reaches `atoned.bob` and ends with **no** `shunned` facts and **no** derived
  regards — while all three beliefs still exist (forgiveness without forgetting); amends is not
  offered before anyone regards bob.
- Regression: full suite green; all worlds `prax check` clean (the type checker's axiom analysis
  covers the new axioms — unbound head variables would be caught); bar/play goldens untouched.

## 7. Out of scope (parked)

- Score effects from standing (numeric adjustments are outcomes, not axioms — a standing-gated
  reaction could apply them; not needed for the arc).
- Multiple standings interacting (thief vs. hero), standing decay, and faction-scoped standings
  (the factions backlog item).
- Lying/false rumors feeding reputation — already possible mechanically (a false belief derives
  real regard), deliberately not demoed until the secrets tier.
