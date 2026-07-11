# v25 — Persona (`Prax.Persona`): traits as conduct, not goals

Backlog item **Personality → volition** (Tier 2), and the honest closure of v18's recorded gap
("traits are NOT compiled to behaviour"). This spec was **rewritten after user review** — see
Design history — from a goal-bundle model to a conduct-valuation model.

## Design history (why the first draft was wrong)

- **Rejected: traits as goal-bundles** (first draft: `industrious` bundles `pursues-earnBread`).
  Pressed, the layer added nothing real: a bearer behaves identically to a character assigned
  the desires directly; the trait fact is just a fact; transparency ≈ auto-professing the
  bundle. Authoring sugar and one presumption variant — not a round.
- **Adopted: traits as conduct-valuations** (user's reframe, CK3-inspired, minus the stress
  bookkeeping): a trait says how you're *willing to act*, orthogonal to what you're after. A
  character can do anything, but **trait-contrary conduct carries negative utility** — costs,
  not prohibitions, in keeping with the soft planner (hard tiers stay banked, separate). Traits
  make kinds of action more or less likely; goal-bundles are demoted to what they always were —
  plain desires needing no trait.
- **Rejected on second review: objective deed-records.** The first conduct draft had `lie`
  deposit a ground-truth ledger entry, framed as the banked exculpation item "arriving through
  the front door." User review overturned it, and rightly: **history persists only through the
  marks it makes** — beliefs, memories, consequences — and the game must be able to reach
  states where the truth is impossible to recover. v22 already shipped this as a commitment
  ("nobody in-world holds ground truth"); an objective record is an oracle, and anything built
  on it would be omniscience through the archive. The conduct-residue is therefore a **mark on
  the liar** — their own memory, owned, forgettable, perishable — and the banked
  exculpation-via-records idea is unbanked as mistaken (truth recovery, if ever built, flows
  through mark-bearers: confession and testimony, not consultation).

## 1. The mechanism: valuations over conduct-records

The engine evaluates *states*, not actions (Versu's apply-and-evaluate — unchanged). So an
action's conduct-cost is expressed the way every cost in this system is: **the action leaves a
record, and a desire values the record**. A trait is a named bundle of (mostly negative)
desires over the bearer's *own conduct-records*:

```
honest = Trait "honest"
  [ Desire "clean-conscience"
      (Want [ Match "Owner.lied.H.stole.C.loaf" ] (-6)) ]
      -- −6 per lie told (per hearer × per fabricated subject), matching the
      -- village's lie-record shape; the weight is authored: what a lie costs
      -- HER, netted against whatever the lie would buy.
```

- **Deterrence at depth 0**: the record appears in the very state the planner evaluates
  (the concealment-want pattern, already proven in v22/v24). Each *additional* lie adds its
  own record binding, so the marginal lie always costs — a conscience with a memory, not a
  one-time fall from grace.
- **Prediction for free**: a *believed* conscience nets against believed motives inside
  `predictMove` — someone who knows both gale's spite and her honesty predicts she declines
  the whisper. Temperament changes what others expect you to do, not just what you do.
- Trait wants are **world-authored against world record shapes**, the same discipline as all
  vocabulary (the trait lives with the world that defines its conduct-records).

## 2. Conduct-marks: the liar's own memory

Conduct-valuation requires conduct to leave traces, and mostly it doesn't yet — the lie's only
current trace (`.heard.<liar>` in the hearer) is indistinguishable from truthful gossip. The
residue is a **mark on the liar**, not a record in the world:

- **`Prax.Deceit.lie` gains one effect**: `Insert ("Actor.lied.Hearer." ++ pat)` — the liar's
  own memory of the deed, rooted under their name like all mental state (e.g.
  `eve.lied.dana.stole.carol.loaf`). Fixed arity per lie-declaration, so world-authored trait
  wants count the bearer's lies exactly (per hearer × per event).
- **Marks, not records — a design commitment**: history persists only through the marks it
  makes, and the truth must be able to become unrecoverable. The mark is owned (the liar's
  psyche), forgettable (a `Delete` on its root retracts it), and perishes with its bearer. Nothing can
  consult it as an oracle: a frame-up stays undisprovable unless a mark-bearer acts — the liar
  confesses, or someone who witnessed the whispering speaks. Truth recovery, wherever it is
  ever built, flows through people.
- **Privacy inherits the existing convention**: psyche-rooted marks have exactly the status of
  `X.believes.…` — public in the trie, private by the same convention every mental-state
  vocabulary already lives by. No new namespace, no new convention.
- **Conscience vs shame, expressible and distinct**: a want over `Owner.lied.…` is conscience
  (fires seen or unseen — it is your own memory); a want over others' *beliefs* about you is
  shame. Both authorable; the difference is now explicit rather than muddled. (A coarser
  gainable attribute — a single `guilty` mood in the v2 idiom — is also authorable where
  per-deed granularity is unwanted.)

## 3. The trait scaffolding (carried over, semantics changed)

The first draft's machinery survives with its meaning corrected — bundles hold
conduct-valuations:

```haskell
data Trait = Trait { traitName :: String        -- single path segment (loud error)
                   , traitDesires :: [Desire] } -- conduct-valuations, typically negative

personaVocabulary :: [Trait] -> [Desire]        -- loud error on duplicate desire names
bearing           :: Trait -> Character -> Character   -- charDesires ++ names
transparent       :: Axiom
  -- trait.M.T ∧ traitDesire.T.D ∧ character.P ⇒ P.believes.desires.M.D.presumed
  -- (temperament is legible: everyone presumes a bearer's conduct-valuations —
  --  probe-verified through closure and predictMove in the first-draft review)

cast :: [Trait] -> [(Character, [Trait])] -> ([Character], [Outcome])
  -- deterministic roster assembly over hand-authored base characters: each
  -- member `bearing` their traits, plus the facts transparent reads
  -- (trait.<who>.<t> per bearing, traitDesire.<t>.<d> once per trait,
  -- character.<who> per member). Loud errors: a non-segment trait name; a
  -- borne trait missing from the vocabulary list (silently illegible
  -- valuations). Sampling stays banked with the stress tooling.
```

The first draft's separate `personaSetup` is folded into `cast`: a roster of hand-authored
characters is the general case (the village's members carry wants and anchors a String-keyed
roster couldn't), and one entry point means the facts and the bearings can never drift apart.

No duplicate desire names may reach `desires` (the guard, plus the world's own care where a
bundle overlaps standalone entries).

## 4. Demo: gale and eve — same spite, different temperaments

The village gains **gale** (via `cast`), the contrast pair to eve:

- **Same motive, made nameable**: the malice becomes a vocabulary desire —
  `Desire "spites-carol" (Want [Match "regards.W.carol.thief"] 4)` — carried by BOTH eve and
  gale via `charDesires`, professed by no one (named-but-unheralded, like bob's pursuit before
  anyone watched him work: spite is not temperament, and stays unreadable until someone is
  told). eve's behavior is unchanged (`selfWants` is identical to her old unnamed want); the
  naming is what makes the prediction test below possible at all — unnamed wants cannot be
  believed.
- **Different temperament**: gale bears `honest` (the trait above). The whisper affordance is
  as available to her as to eve, and would serve her spite (+4 per deceived head) — but the
  lie-record's −6 nets it negative. **Eve whispers; gale never does.** Authored meaning: her
  honesty outweighs her spite, by exactly the margin written.
- **Legibility**: from t=0, everyone presumes gale's conscience (`transparent`); a test plants
  a `spites-carol` motive-belief about each woman in a predictor's head — with gale's presumed
  conscience netted in, `predictMove` has her declining; the same planted belief about eve
  (no conscience) predicts the whisper.
- The CK3 property, kept: gale *can* lie — nothing forbids it. If her spite were authored
  larger than her conscience, she would. Costs, not prohibitions.
- **The laundering (found in implementation, kept honest)**: eve's one-shot-per-hearer whisper
  reaches gale too — and an honest believer is the *perfect* vector. Once deceived, gale
  spreads the falsehood she now genuinely holds by ordinary `gossip`: not a lie, no mark, no
  conscience cost — her spite is served by telling a truth-as-she-knows-it. She even carries
  it back to eve, handing the liar "evidence" for her own fabrication. The first spec draft
  wrongly asserted "no `.heard.gale` edges"; that assertion confused *never lying* with
  *never spreading*, and the demo pins the corrected claim instead: gale's psyche stays
  unmarked, the lie still travels through her, and everything traced to her is something she
  honestly believes.

## 5. Tests (TDD)

- `DeceitSpec` additions: the lie deposits the liar's mark `<liar>.lied.<hearer>.<event>`
  (exact shape); truthful `gossip` deposits no such mark; the mark is forgettable (a `Delete`
  on the `lied` root clears it — and PersonaSpec shows the conscience-cost clearing with it);
  existing lie tests unchanged (the mark is additive).
- `PersonaSpec` (fixture pins the first-draft probe plus the new semantics): `bearing`/
  `personaVocabulary`/`cast` mechanics (+ duplicate/path-segment/stray-borne-trait guards);
  `transparent` derives presumption for bearers only, defeasibly; **the conduct-valuation
  core**: a bearer with a temptation (a want a lie would serve) declines it while an
  unprincipled twin takes it — asserted via `pickAction` on both; the marginal-lie property
  (each additional lie costs again — no fall-from-grace discount); believed-conscience
  prediction (predictMove with planted motive-belief + presumed trait = declines; without the
  trait = the whisper).
- `VillageSpec` additions: gale presumed honest by all from t=0; free-play drive — eve's
  frame-up proceeds, gale never whispers (no `gale.lied.*` mark), yet the lie travels through
  her honestly (`.heard.gale` edges exist and everything behind them is something gale
  believes — the laundering, §4); the prediction contrast.
- Regression: full suite green (278 baseline); older village stories intact (eve unchanged;
  cast growth turn-budget raises sanctioned with traces); `prax check` all 7 worlds;
  `cabal build all` zero warnings; hlint clean.

## 6. Out of scope (parked deliberately)

- Theft conduct-marks and getting-caught-lying (the mark pattern is established; each is its
  own round when wanted). Exculpation-via-records is not parked — it is rejected (§2).
- Script-layer `withTraits` wiring; randomized cast sampling; runtime trait change (character
  transformation belongs with the Arc vocabulary, a future join); hard prohibitions (the
  banked priority-tiers item).
- Goal-bundle "traits" — deliberately not a thing; use plain desires.
