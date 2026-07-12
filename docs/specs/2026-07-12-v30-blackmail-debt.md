# v30 — Blackmail & debt (`Prax.Debt` + `Prax.Blackmail`)

The backlog's oldest named commitment (v-next since v22), folded with debt per user direction.
The choice structure was **probe-verified live before this spec** (session probe, depth 2):
the extortionist threatens, the victim buys silence, and every step is individually motivated.

## The leverage model (the question v22 parked, answered)

- **A threat is a motive-belief deposit.** Threatening *communicates conditional intent*:
  the action inserts `victim.believes.desires.<extorter>.<punitive-desire>.heard.<extorter>`
  — the same channel confiding and lying already ride (v22/v23). The victim's own round-walk
  then predicts the exposure and weighs it; no new epistemics, no mind-reading.
- **Why the blackmailer withholds rather than gossips**: the debt-want outvalues any
  spread-want. Exposure is the fallback, not the goal.
- **Why the threat is credible**: the extortionist genuinely holds the punitive desire the
  threat professes (`punishes-defiance`: +w per believer-of-the-secret once threatened or
  defied). Probe finding, stated honestly: this desire (via self-recursion — expose is one
  ply away) is what motivates *threatening* in the first place; the blackmailer needn't
  predict compliance (a myopically-unmotivated move `predictMove` correctly won't foresee).
  Character coherence, not accident: one who wouldn't enjoy exposing doesn't threaten.
  *Residual, recorded*: a pure bluffer (deposit without the desire) is expressible but not
  self-motivating — banked with the script layer.
- **A standing threat is exposable** (probe finding: gating exposure on defiance alone makes
  stalling safe forever — the classic hole). With exposure available against silence too,
  waiting ties with defiance and never dominates.
- **The compliance arithmetic, authored not tuned** (probe-measured): the victim complies iff
  `audience × fear-per-head × walk-discount > debt cost × stream accumulation`. One onlooker
  at fear −10 vs favor −4 rationally defies (−5 marginal < −9.6 stream); two onlookers comply.
  The spec states this so world authors price threats deliberately. In the village the
  audience is the whole cast and bob already carries `notorious −15`: the existing weights
  make compliance rational without touching them.

## 1. `Prax.Debt`

A debt is an obligation with a **beneficiary** — the piece `obliged.<who>.<content>` lacks.
New vocabulary `debt.<creditor>.<debtor>.<content>`, thin combinators over `Prax.Deontic`:

- `owe :: String -> String -> String -> [Outcome]` — insert the debt fact AND
  `oblige debtor content` (a debt IS an obligation; both facts, one call).
- `repay`-shaped combinator: the world supplies the transfer action; `settle creditor debtor
  content :: [Outcome]` deletes the debt and `discharge`s the obligation.
- `defaulted`-standing: a creditor's `demand` action (co-present, debt stands, not yet
  demanded) marks refusal via Deontic's `breach` (→ `violated.*`), made `observable`
  (v19) so witnesses believe it; one `standingUnless`-style axiom derives
  `regards.<W>.<debtor>.deadbeat` from the believed default, defeated by eventual repayment
  (`atoned`-shaped: settling inserts `atoned.<debtor>` as the `standingUnless` defeater, per
  `Prax.Repute`'s own idiom — a positive fact whose insertion dissolves every derived regard
  at once; the breach's raw fact and the witness's belief both persist undisturbed, which is
  the point — reputation derives from belief, never the raw fact, so only the defeater's
  presence, not the breach's absence, can dissolve it). Reputation flows from belief, never
  from the raw fact — the K-discipline unchanged.
- Guards: content must be a single sentence (Deontic's stratification rule restated), loud
  errors in the established idiom.

## 2. `Prax.Blackmail`

One combinator, `shakedown`, generating the four-action protocol the probe validated
(shape mirrors `endeavor`: the world slots the actions into its own practice):

```haskell
shakedown :: String        -- id (path segment; loud error)
          -> CoPresence    -- who counts as an audience/co-present
          -> String        -- evidence pattern, e.g. "stole.V.loaf" (V = the victim var)
          -> String        -- the price: debt content, e.g. "favor"
          -> Int           -- the extorter's punitive weight (+w per believer once live)
          -> (Desire, [Action])
```

- **`threaten`**: Actor believes the evidence about V, co-present, not already threatening →
  `Insert threatened.<a>.<v>` + the motive-belief deposit (above).
- **`comply`** (victim): threat stands → `owe extorter victim price` (Debt §1) +
  `Delete threatened` — silence bought, the extorter now holds a debt.
- **`defy`** (victim): threat stands → `Insert defied` + `Delete threatened`.
- **`expose`** (extorter): `Or [threatened, defied]` ∧ evidence ∧ co-present hearer who
  doesn't already believe → the standard sourced-hearsay deposit (`.heard.<extorter>` —
  Rumor's shape, so the whole reputation stack cascades on it).
- The generated `Desire` is the punitive intent (`punishes-<id>`), world-registered; the
  extorter carries it via `charDesires` (named — believable — that is the point).
- **Blackmail leaves a mark** (v25's idiom): `threaten` also inserts
  `<extorter>.extorted.<victim>.<event>` — the extorter's own memory, priceable by traits
  (an `honest`/`decent` bearer won't stoop; a mark future confession/exposure arcs can use).

## 3. Village demo (amended after implementation blocked both drafted arcs)

**The block, recorded**: per-head fear cannot serve two masters — the weight that keeps eve
whispering before a guaranteed witness (≤1/head) contradicts the weight compliance needs
(~10/head); and theft-evidence shakedowns catch the framed (v22's indistinguishability is the
point) while displacing dana's shipped arcs. Both measured, neither tunable away.

**The resolution: threshold fear — bob's own idiom.** Nonlinear fear serves both masters
because its marginal price is zero below the brink and catastrophic at it:

- eve's fear becomes `Want [Match "notorious.eve.slanderer"] (−15)` (mirroring bob's
  notorious −15 — authored meaning: being the village's KNOWN slanderer destroys her),
  wired by `standingUnless "whispered.Culprit.H" "recanted.Culprit" "slanderer"` +
  `notoriety "slanderer" 3`. The whispering ACT becomes observable
  (`witnessed together "whispered.Actor.Hearer"` on the village's lie — content stays
  secret), as originally drafted.
- **Whispering stays rational** (1–2 regards < 3 threshold — asserted, not hoped): eve's
  free-play behavior is unchanged, explicitly tested.
- **The arc** (forced trajectory, the theft tests' own convention): carol walks to the mill
  and witnesses eve's whisper (2 witnesses = 2 regards — still under threshold, eve still
  whispers); carol — already the frame-up's target, now holding `.seen` evidence — shakes
  eve down (`shakedown` with evidence `"whispered.V.H"`, price `favor`); eve stands one
  exposure from the brink, her round-walk sees the third regard land, and she pays.
  carol's additions are additive only: the punitive desire via `charDesires` and a
  favor-debt want; her existing wants and arcs untouched.
- The blocked attempt's real finds ship with the arc: the villageP role `V` renamed (it
  collided with evidence-variable conventions), and `shakedown`'s reserved-variable guard
  extended to `Hearer` and `Actor` (loud, complete).
- dana/bob is retired as an arc (recorded: bob's crimes in this village are either fully
  witnessed or perfectly secret — a Catch-22 that is itself a faithful result).

## 4. Tests (TDD)

- `DebtSpec`: owe/settle/demand/deadbeat-standing lifecycle; breach visibility is
  belief-gated (an unwitnessed default derives no regard); guards.
- `BlackmailSpec` (pins the session probe): threaten deposits the motive-belief; the victim
  complies with a two-head audience and rationally defies with one (BOTH asserted — the
  arithmetic is the mechanic); standing-threat exposure (stalling ties defiance);
  post-compliance the extorter holds the debt and exposure is gone; the extorted mark; a
  trait pricing the mark deters the shakedown (v25 composition).
- `VillageSpec`: the arc end to end (forced trajectory per §3); the village golden
  RE-CAPTURED once, in its own commit, drift itemized (a deliberate world change, never an
  engine change; bar/intrigue must not drift).
- **One sanctioned amendment to a v25 test** (found in implementation): threshold fear makes
  eve a one-shot liar in free play — after her first whisper she sits at two regarders and
  correctly never risks the brink again, so the v25 laundering (her whisper reaching gale,
  who honestly relays) is free-play unreachable. The precedent is v22's retelling: the
  "same spite, different temperaments" test keeps its free-play assertions (the frame-up,
  the mark, gale never lying) and GAINS the sharper one (exactly one whisper, ever — fear
  of the brink made eve prudent); the laundering block moves to a forced-trajectory
  continuation (force the gale whisper — the affordance remains offered — then drive and
  assert gale's honest relay), so the v25 mechanism stays pinned rather than silently
  untested. Task 4's WALKTHROUGH/LEDGER updates retell this honestly.
- Suite green throughout; the usual gates.

## 5. Out of scope

- Bluffing (recorded above), threat expiry/deadlines (needs calendar), counter-blackmail,
  debt transfer/assignment, interest/escalation — banked.
- Any engine change: this round is authored vocabulary + two thin modules over shipped
  machinery. If implementation finds otherwise, BLOCK and amend here first.
