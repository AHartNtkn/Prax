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

## 1. The mechanism: valuations over conduct-records

The engine evaluates *states*, not actions (Versu's apply-and-evaluate — unchanged). So an
action's conduct-cost is expressed the way every cost in this system is: **the action leaves a
record, and a desire values the record**. A trait is a named bundle of (mostly negative)
desires over the bearer's *own conduct-records*:

```
honest = Trait "honest"
  [ Desire "clean-conscience"
      (Want [ Match "lied.Owner.H.stole.C.loaf" ] (-6)) ]
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

## 2. Conduct-records: the `lied` deed-record

Conduct-valuation requires conduct to leave traces, and mostly it doesn't yet — the lie's only
current trace (`.heard.<liar>` in the hearer) is indistinguishable from truthful gossip. So:

- **`Prax.Deceit.lie` gains one effect**: `Insert ("lied.Actor.Hearer." ++ pat)` — a
  ground-truth deed-record, `lied.<liar>.<hearer>.<full event>` (e.g.
  `lied.eve.dana.stole.carol.loaf`). Fixed arity per lie-declaration, so world-authored trait
  wants count lies exactly (per hearer × per event).
- **This is the banked ground-truth/exculpation item arriving through the front door**: the
  same records that make conscience expressible make frame-ups disprovable later. This round
  ships only the `lied` record (the vocabulary conscience needs *now*); theft-records and
  exculpation actions stay banked, with the pattern established.
- **Privacy by convention, documented**: the trie is public, so conduct-records are readable
  by anyone — conscience stays private only because no shipped action or axiom queries
  `lied.*` except the bearer's own trait want. The same convention `conceal`'s vocabulary
  already lives by. (Believed/witnessed lying — getting *caught* — is exactly what these
  records enable later, deliberately not built now.)
- **Conscience vs shame, expressible and distinct**: a want over `lied.Owner…` is conscience
  (fires seen or unseen); a want over others' *beliefs* about you is shame. Both authorable;
  the difference is now explicit rather than muddled.

## 3. The trait scaffolding (carried over, semantics changed)

The first draft's machinery survives with its meaning corrected — bundles hold
conduct-valuations:

```haskell
data Trait = Trait { traitName :: String        -- single path segment (loud error)
                   , traitDesires :: [Desire] } -- conduct-valuations, typically negative

personaVocabulary :: [Trait] -> [Desire]        -- loud error on duplicate desire names
bearing           :: Trait -> Character -> Character   -- charDesires ++ names
personaSetup      :: [Trait] -> [(String, [Trait])] -> [Outcome]
                     -- trait.<who>.<name> per bearing; traitDesire.<trait>.<desire> data
                     -- facts once per trait; character.<who> for roster members
transparent       :: Axiom
  -- trait.M.T ∧ traitDesire.T.D ∧ character.P ⇒ P.believes.desires.M.D.presumed
  -- (temperament is legible: everyone presumes a bearer's conduct-valuations —
  --  probe-verified through closure and predictMove in the first-draft review)

cast :: [Trait] -> [(String, [Trait])] -> ([Character], [Outcome])
  -- deterministic roster assembly (sampling stays banked with the stress tooling)
```

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

## 5. Tests (TDD)

- `DeceitSpec` additions: the lie deposits `lied.<liar>.<hearer>.<event>` (exact shape);
  truthful `gossip` deposits no such record; existing lie tests unchanged (the record is
  additive).
- `PersonaSpec` (fixture pins the first-draft probe plus the new semantics): `bearing`/
  `personaVocabulary` (+ duplicate/path-segment guards)/`personaSetup`/`cast` mechanics;
  `transparent` derives presumption for bearers only, defeasibly; **the conduct-valuation
  core**: a bearer with a temptation (a want a lie would serve) declines it while an
  unprincipled twin takes it — asserted via `pickAction` on both; the marginal-lie property
  (each additional lie costs again — no fall-from-grace discount); believed-conscience
  prediction (predictMove with planted motive-belief + presumed trait = declines; without the
  trait = the whisper).
- `VillageSpec` additions: gale presumed honest by all from t=0; free-play drive — eve's
  frame-up proceeds, gale never whispers (no `lied.gale.*` record, no `.heard.gale` edges);
  the prediction contrast.
- Regression: full suite green (278 baseline); older village stories intact (eve unchanged;
  cast growth turn-budget raises sanctioned with traces); `prax check` all 7 worlds;
  `cabal build all` zero warnings; hlint clean.

## 6. Out of scope (parked deliberately)

- Theft deed-records + exculpation/getting-caught-lying (the records' pattern is established;
  each is its own round when wanted).
- Script-layer `withTraits` wiring; randomized cast sampling; runtime trait change (character
  transformation belongs with the Arc vocabulary, a future join); hard prohibitions (the
  banked priority-tiers item).
- Goal-bundle "traits" — deliberately not a thing; use plain desires.
