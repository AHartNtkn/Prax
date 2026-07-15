# v38 — Chance & feelings: a die for the drama, and the moods that use it

User-directed, reframed by the user at design review: emotions mostly reuse existing
machinery (episodic facts + desires for pricing, v36 pulses for wear-off, Reactions for
event context), so **this is an infrastructure round** — the missing primitives built as
general facilities — **with emotions as the example application**. The user's two calls:
feelings COEXIST (not the Versu single-slot mood), and stochastic onset ships now.

The load-bearing invariant, restated where it governs: **emotions change decision-making,
never what decisions can be made.** The mechanism adds no availability gating anywhere; a
feeling is a plain state fact that author-chosen desires read as utility conditions.
`candidateActions` is identical in every mood. (Feeling-CONTENT preconditions — an act
that *expresses* the feeling, like complaining about someone you are cross with — remain
ordinary authored practice conditions, the eat-requires-hunger carve-out.)

## Infrastructure 1: `Mod` (engine, one operator)

`CalcOp` gains `Mod` (`applyCalc Mod = mod`, Haskell semantics: result carries the
divisor's sign, non-negative for positive modulus — documented). This respects the stated
Praxish rationale for omitting division ("keep the DB integer-valued"): modulo IS integral.
No other engine change in the round.

## Infrastructure 2: `Prax.Rng` — a seeded die in world state

A deterministic random stream as ordinary facts, so reproducibility, goldens, replay, and
persistence all survive for free:

- **State**: one `seed!N` fact. `rngSetup :: Integer -> [Outcome]` seeds it — the initial
  seed is an authored world parameter (it selects the playthrough's fate; goldens pin it).
- **The generator**: Lehmer / Park–Miller MINSTD — `seed' = seed × 16807 mod 2147483647`.
  The constants are MECHANISM with published provenance (Park & Miller 1988), fixed in the
  module, never tuned; the AUTHORED numbers are the odds. This is a drama die, not a
  statistics library — stated in the haddock.
- **The draw combinator** (the authoring surface):

```haskell
-- | "With probability num/den, where conds also hold, apply outs."
-- Compiles to: an unconditional seed advance, then a ForEach whose guard
-- reads the fresh seed, takes it mod den, and requires < num — so every
-- draw consumes EXACTLY one stream step whether or not it hits (failed
-- rolls must not freeze the die: a provocation that failed once must not
-- fail identically forever). Reserved variables guarded loudly.
draw :: Int          -- ^ num   (authored meaning: the odds sentence)
     -> Int          -- ^ den
     -> [Condition]  -- ^ further guards (trait conditions live here)
     -> [Outcome]    -- ^ what happens on a hit
     -> [Outcome]    -- ^ the fragment authors append to an action's outcomes
```

  Guards: `0 < num < den` (a certain or impossible "chance" is authored dishonesty —
  use a plain outcome or nothing); single-segment reserved-variable discipline as in
  `Prax.Drift`. Multiple draws in one action chain independent steps. Worlds without
  `rngSetup` that use `draw` are flagged by `prax check` (the ClocklessDrift precedent).

## Infrastructure 3: coexisting feelings (the mood migration)

The Versu-inherited single-slot `<who>.mood!<feeling>.toward!<target>` is REPLACED —
completely, no legacy — by multi-valued episodic facts:

- `feels.<who>.<emotion>.toward.<target>` (targeted) and `feels.<who>.<emotion>`
  (untargeted); multi-valued throughout — angry at two people while afraid of a third all
  coexist, each fact independent. A want reading `feels.Owner.<emotion>` sees targeted
  instances too (Match sees subtrees).
- `Prax.Emotion` (new home; `Prax.Core`'s mood section is DELETED, its Ekman vocabulary
  moves here): `feel who emotion target`, `feelToward`-style outcome constructors,
  `feeling`/`feelingToward` condition helpers, and the wear-off:
- **Feelings fade** (the v36 principle: episodic state decays, dispositions never): one
  drift rule sweeping `feels.*` at an authored period, shipped test-compressed WITH the
  now-standard truncation label (real authoring reference: a feeling fades in hours,
  ~24-48 rounds). Per-emotion periods are banked until a world needs them.
- **Onset** is authored at the provoking action (the `witnessed`/`conceal` combinator
  idiom): `provokes` fragments append to outcomes, typically wrapping `draw` — a slight
  may anger its victim; a `shortTempered.<who>` trait fact in the draw's guards raises the
  odds arm (two draws: base odds for anyone, a second higher-odds arm gated on the trait —
  each arm's odds an authored sentence). Traits make emotions LIKELIER; they are still
  dispositions and never decay.
- **Pricing is just desires** — the module adds none by force; the doc states the
  authoring guidance: prefer NEGATIVE pricing (a feeling as discomfort driving its own
  discharge — anger priced −k, venting/confronting deletes the feeling) both for the
  psychology and because v33's FloorCheck keeps unfelt negative desires planning-free,
  while positive emotion-desires (revenge-tastes-sweet) are action-insertable and thus
  AlwaysLive — allowed, with the cost named.

## The migration of existing consumers (no dual systems)

Play, Intrigue, Reactions (`disapprovalP`), the Bar, and DirectorSpec consume moods today.
All move to feelings in this round:

- Content-preconditions stay as feels conditions (the Bar's "complain about Subject"
  requires feeling annoyed toward Subject — the act expresses the feeling).
- **Pure availability gates become pricing** (the invariant applied to prior art): the
  Bar's `Not "Actor.mood!annoyed.toward!Other"` guards on buying/greeting are REMOVED from
  conditions; instead annoyance is priced (an authored negative want on
  courtesy-while-cross states, weight with its sentence) so a cross bartender CAN buy the
  round but won't want to. Behavior will shift; the bar goldens are re-captured, itemized.
- `setMood`'s remember-the-previous-mood machinery dies with the slot (coexistence makes
  it meaningless).

## Cargo: the example application, both worlds

- **Bar**: the migration itself is the cargo — disapproval now makes onlookers *feel*
  annoyed (fading, priced), and the bought-round/greeting economics run on valuation
  instead of gates.
- **Village**: one short-tempered villager (carol — her confront want already wants
  targets). Being whispered about (the witnessed whisper ACT, v30's observability) or
  being shunned `provokes` anger with authored odds, doubled-arm on `shortTempered.carol`;
  anger is priced as discomfort (−k) discharged by her existing confrontation affordance
  (plus `Delete` of the feeling on confront); feelings fade on the pulse if never vented.
  A gathering interplay falls out free and is pinned if observed: an angry carol at market
  day confronts in front of everyone (percolation doing drama).

## Verification

- RngSpec: determinism (same seed ⇒ same stream), the exactly-one-step-per-draw law
  (failed rolls advance too — pinned by construction: two consecutive draws with
  impossible-to-satisfy guards still move `seed!`), odds guards loud, check-tool flag for
  seedless draw worlds, reserved-var guards.
- EmotionSpec: coexistence (two feelings, distinct targets, independent wear-off);
  fade-on-pulse; the invariant pin: `candidateActions` IDENTICAL with and without any
  feeling (the no-gating edict as a test); onset arms (base odds vs trait-doubled odds —
  driven across seeds by clock/seed fixtures, exact outcomes per seed).
- Migration: every prior mood consumer's tests updated intent-preserving; bar pins for
  the gate→pricing shift (a cross bartender CAN but does NOT buy the round — both halves
  asserted); goldens re-captured per world in own commits, itemized.
- The suite stays deterministic end-to-end (fixed seeds); ViewInvariant green; the usual
  gates.

## Out of scope

Emotion visibility to other minds (believed feelings, deterrence-by-anger — banked:
own-planning only this round); per-emotion fade periods; intensity levels; mood-congruent
perception; the chronicler noticing feelings; any non-Lehmer generator sophistication.
