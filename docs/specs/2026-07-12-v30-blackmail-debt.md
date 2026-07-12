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
  (`atoned`-shaped: settling deletes the breach base). Reputation flows from belief, never
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

## 3. Village demo

- **carol blackmails eve** — the poetic close of the v25 arc: gale (honest, at the mill)
  *witnesses eve's whisper* — the v25-parked getting-caught piece, minimally: the village's
  `lie` action becomes observable-as-an-act (`whispered.<liar>.<hearer>` seen by co-present
  witnesses via the existing `witnessed` outcome — the CONTENT stays secret, the ACT does
  not). gale, honest, won't lie but will *tell* — carol (already motivated against her
  framer) hears of the whispering, holds the evidence-belief, and shakes eve down: silence
  about the whispering for a favor. eve — whose malice-want loses everything if the village
  learns her frame-up was a lie — pays.
- Alternatively simplest-path if the above needs new observability plumbing beyond the one
  `witnessed` line: dana blackmails bob post-theft (forced-theft trajectory, the standard
  test start). The plan probes WHICH arc drives cleanly and ships the one that does; both
  are asserted intents, not both shipped.
- Debt demo rides the same arc: the extracted favor is a real `debt.*` +
  `obliged.*` pair; a later `settle` or a `demand`→`deadbeat` branch exercises Debt fully.

## 4. Tests (TDD)

- `DebtSpec`: owe/settle/demand/deadbeat-standing lifecycle; breach visibility is
  belief-gated (an unwitnessed default derives no regard); guards.
- `BlackmailSpec` (pins the session probe): threaten deposits the motive-belief; the victim
  complies with a two-head audience and rationally defies with one (BOTH asserted — the
  arithmetic is the mechanic); standing-threat exposure (stalling ties defiance);
  post-compliance the extorter holds the debt and exposure is gone; the extorted mark; a
  trait pricing the mark deters the shakedown (v25 composition).
- `VillageSpec`: the chosen arc end to end in free play; goldens/ViewInvariant untouched
  and green (cast/vocabulary changes will shift golden sequences — NO: goldens pin the
  villageWorld; adding vocabulary CHANGES the world, so the village golden is RE-CAPTURED
  once, in its own commit, with the decision drift itemized and justified line by line —
  the one sanctioned re-capture form: a deliberate world change, not an engine change).
- Suite green throughout; the usual gates.

## 5. Out of scope

- Bluffing (recorded above), threat expiry/deadlines (needs calendar), counter-blackmail,
  debt transfer/assignment, interest/escalation — banked.
- Any engine change: this round is authored vocabulary + two thin modules over shipped
  machinery. If implementation finds otherwise, BLOCK and amend here first.
