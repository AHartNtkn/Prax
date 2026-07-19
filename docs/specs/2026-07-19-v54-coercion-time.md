# v54 — Coercion joins the schedule's paradigm: the two permanences become authored lifetimes; two banked fictions proven free

The user's challenge, which this round executes: if the coercion primitive were
properly scoped, the v30-era banked fictions should mostly already work — and
where they don't, the mechanism is too narrow. The assessment against the shipped
`Prax.Coerce` confirmed it with an exact diagnosis: the narrowness is TIME, and
only time. Two bare `Insert`s hardcode temporal model decisions (the threat
marker's permanence; the `complied` marker's permanence) that v44's paradigm —
the engine owns scheduling, lifetimes are authored content, `InsertFor` is the
public primitive — already assigns to the author. The other two banked fictions
need NO mechanism change at all, and this round proves it with fixtures instead
of asserting it.

## The design

### 1. Two lifetime fields (the only mechanism change)

```haskell
data Coercion = Coercion
  { ...                            -- every shipped field unchanged
  , coThreatLasts     :: Maybe Int -- Nothing = a standing threat (today, verbatim)
  , coComplianceLasts :: Maybe Int -- Nothing = bought silence stays bought (today)
  }
```

`coerce` compiles the threat marker as `Insert` (Nothing) or `InsertFor n`
(Just n), and the `complied` marker likewise. Nothing = the shipped bytes —
every existing instance (Blackmail's shakedown, the racket fixture) passes
Nothing and is untouched; goldens and all v49 pins byte-identical. `Just n`
hands the marker to v44's expiry machinery, which already owns everything else:
supersession (a re-threat refreshes the clock), persistence (dues serialize),
boundary-exact retraction.

- **Threat expiry** (`coThreatLasts = Just n`): the threat marker retracts n
  boundaries after threaten — a stale threat stops gating punish's
  standing-threat arm and stops pressuring comply. The DEON-property-1-style
  guarantee ("stalling never dominates", property 1) is TIME-SCOPED by the
  author's own declaration: within the lifetime stalling still never wins;
  after expiry there is no threat to stall against. The property re-pins with
  the scope stated.
- **Serial extortion** (`coComplianceLasts = Just n`): the `complied` marker
  expires, and the racket CYCLES with no new logic — the shipped gates already
  compose: comply deletes the threat; threaten re-arms on no-standing-threat;
  only the permanent `complied` blocked the second extraction. Property 3
  ("repeat extraction impossible") generalizes to its honest form: impossible
  WITHIN THE BOUGHT PERIOD — permanent is the `Nothing` case, exactly as
  shipped, and the v49 permanence paragraph's "until serial extortion is
  deliberately designed" clause discharges: this is that design, and it is one
  field riding v44.

No new markers, no new gates, no new wants. The `Maybe Int` is not a heuristic:
n is the author's fictional decision (how long a threat hangs; how long silence
stays bought), exactly like every `InsertFor` lifetime since v44.

### 2. Two fictions proven free (fixtures, not mechanism)

- **Bluffing IS the registration contract, re-read.** The probe's discovery:
  the motive-belief deposit is MECHANISM-owned — `threaten` itself plants
  `victim.believes.desires.<extorter>.punishes-<sid>` (Coerce.hs:161). The
  victim always believes the punitive want exists; whether it DOES exist is
  the author's registration act (`setDesires`/`charDesires` — v49's documented
  contract). Omission, which v49's docs framed as a silent hazard, is the
  bluff: the threat lands (deposit), the victim's fear is real (belief-driven),
  and the extorter — holding no punitive want — never chooses punish. A
  spec-only fixture pins the pair: the bluffed victim complies exactly as
  against a genuine threat (the deposit is identical); the bluffing extorter,
  DEFIED, declines to punish where the genuine one punishes (pickAction, both
  sides — the bluff is real in the fiction and empty in the mind). The v49
  registration-contract prose re-frames: registration is the CREDIBILITY
  DECISION — registering authors a genuine coercer, omitting authors a bluffer
  — a semantics with two intended settings, not a contract with a failure
  mode.
- **Counter-coercion is composition.** A second `Coercion` whose `coTrigger`
  reads the first's marks (`threatened.<sid1>.E.V` / `Actor.extorted.V.<sid1>`
  — deliberately unreserved, gateable fiction per v49). Fixture: the victim of
  coercion 1, holding leverage over the extorter's extortion itself, threatens
  back; both threats stand; the table turns on ordinary scoring. Zero
  mechanism change — the fixture is the proof, and if composition is blocked
  anywhere, that blocker is a BLOCK-and-surface finding, not something to
  patch around.

## What dies

The v49 spec's permanence paragraph's forward reference discharges (recorded in
the LEDGER row, the spec stays as record). The v49 haddock/doc framing of an
omitted registration as a pure hazard is REPLACED by the two-setting semantics.
Nothing else: no shipped field, gate, marker, or pin changes meaning under
`Nothing`/registered.

## Classification

Expressiveness extension with a byte-identical default: every shipped instance
compiles to the same cooked actions (Nothing = the same `Insert`), so all v49
property pins, goldens, and analyses are UNMOVED — verified, not asserted. New
behavior exists only where a fixture authors `Just n` or omits a registration.

## The property contract (new pins; violating any is the BLOCK)

1. **Nothing is today**: a Nothing/Nothing coercion's compiled actions are
   byte-identical to the shipped compilation (the racket world re-cooked and
   compared structurally).
2. **The racket cycles**: with `coComplianceLasts = Just n`, threaten→comply
   extracts; re-threats extract nothing for n boundaries (property 3, scoped);
   at expiry the extorter re-threatens and extracts AGAIN — the monthly racket,
   end-to-end, exactly one extraction per bought period.
3. **A stale threat is spent**: with `coThreatLasts = Just n`, the un-complied,
   un-defied threat stops offering punish's standing-threat arm after n (the
   defied arm is untouched — punishment for defiance does not expire with the
   threat); comply's pressure ends with the marker.
4. **Stalling never dominates within a live threat** (property 1, time-scoped):
   while the threat stands, waiting never strictly beats both comply and defy.
5. **The bluff pair**: the bluffed victim's comply/defy decision is IDENTICAL
   to the genuine case (same deposit, same fear); the bluffing extorter
   declines punish on defiance where the genuine extorter chooses it
   (pickAction both).
6. **The table turns by composition**: the counter-coercion fixture reaches a
   standing counter-threat from pure content over the shipped surface.
7. **Refresh law**: a re-threat under `Just n` refreshes the threat's clock
   (v44 supersession, inherited — pinned at the instance).

## Verification

CoerceSpec grows the fixtures (RED-first each); BlackmailSpec untouched
(shakedown passes Nothing — byte-identity observed); the v49 six-property pins
re-observed green unchanged; Persist inherits (expiring markers are ordinary
v44 facts — the mid-scene precedent covers it; one save/resume pin mid-racket).
Docs: Coerce haddock (the two-setting registration semantics; the lifetime
fields' fiction), LEDGER v54 row (the banked-fictions disposition: two shipped
as fields, two proven free, the bank emptied), WALKTHROUGH's leverage prose if
it mentions permanence. Pre-gate: the three-lens panel runs on this document.

## Out of scope

Nothing carried: this round empties the coercion bank. (Threat expiry and
serial extortion ship as fields; bluffing and counter-coercion ship as proven
compositions with standing fixtures.)
