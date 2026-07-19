# v54 — Coercion joins the schedule's paradigm: the permanences become authored lifetimes; the bank empties

The user's challenge, which this round executes: if the coercion primitive were
properly scoped, the v30-era banked fictions should mostly already work — and
where they don't, the mechanism is too narrow. The assessment confirmed it with
an exact diagnosis: the narrowness is TIME (two bare `Insert`s hardcoding
temporal model decisions v44 assigns to authors), plus one credibility semantics
that was already half-present and needed its accident caught.

REWRITTEN after the three-lens pre-gate panel (`.superpowers/sdd/v54-spec-review-*`):
the design lens's Critical [D-C1] rejected the first draft's bluff framing as a
hazard retconned into a feature (the exact call v51 made the OTHER way for the
isomorphic deontic case); the resolution below came out of the soundness lens's
own trace [S-I1] and is cleaner than the flag the design lens proposed. All
findings folded with [S]/[D]/[C] citations.

## 1. Two lifetime fields (the marker-permanence half)

```haskell
data Coercion = Coercion
  { ...                            -- every shipped field unchanged
  , coThreatLasts     :: Maybe Int -- Nothing = a standing threat (today, verbatim)
  , coComplianceLasts :: Maybe Int -- Nothing = bought silence stays bought (today)
  }
```

`coerce` compiles each marker as `Insert` (Nothing — byte-identical to shipped:
`Action` derives Eq, and the Nothing/Nothing compilation is pinned structurally
[S-verified]) or `InsertFor n` (Just n — v44's public machinery: boundary-exact
retraction, delete-purges-pending, persistence via dues, all inherited).

- **Threat expiry** (`coThreatLasts = Just n`): the threat marker retracts n
  boundaries after threaten. The DEFIED arm of punish and of the punitive want
  is UNTOUCHED — punishment for defiance does not expire with the threat; only
  the standing-threat arm and comply's pressure end with the marker. Property
  1 (stalling never dominates) re-pins TIME-SCOPED: while the threat stands.
- **Serial extortion** (`coComplianceLasts = Just n`): the `complied` marker
  expires and the racket cycles with NO new logic — comply deletes the threat,
  threaten re-arms on no-standing-threat, and only the permanent `complied`
  blocked the second purchase. Property 3 generalizes honestly: repeat
  EXTRACTION impossible within the bought period; permanent is the Nothing
  case. **Scoped precisely [S-I2]: the bought period protects the victim's
  PURSE, not their person — a still-motivated extorter can re-threaten and
  punish during it, exactly as under today's permanent marker (the lifetime
  introduces NO new exposure; the fixture observes and records the shipped
  behavior rather than asserting protection the mechanism never gave).**
- The motive-belief deposit and the extorted mark stay PERMANENT, stated
  [C-M1]: they are records of the attempt, not currency of the threat.
- Mid-life renewal is structurally impossible (threaten gates on no standing
  threat), so no "refresh law" is claimed [D-I1: the first draft's property 7
  was a v44 engine pin unreachable through this surface — dropped; its
  corollary stands: the renew-before-expiry strategy question closes by
  construction, and no new bank item exists].

`Maybe Int` is not a heuristic: n is the author's fictional decision, exactly
like every `InsertFor` lifetime since v44 [D-M1: fields confirmed the right
surface — the markers are mechanism-inserted, so lifetime must be a record
parameter, not an authored wrapper].

## 2. Credibility as world state: genuine, bluff, and the accident (the [D-C1] resolution)

The probe's discovery stands: `threaten` itself plants the fear
(`victim.believes.desires.<extorter>.punishes-<sid>`, mechanism-owned). What the
panel corrected is what may be SILENT about the other side. The three cases are
now distinct WORLD STATES, two authored and one caught loudly:

- **GENUINE**: `punishes-<sid>` registered (`setDesires`) AND held by the
  extorter (`charDesires`). The threat is self-motivated (v49's credibility,
  unchanged).
- **BLUFF**: registered but NOT held. The vocabulary exists, so the victim's
  believed-desire machinery resolves and their fear is REAL and identical to
  the genuine case [S-I1: `believedDesires` filters on `desires st` — this
  split, not full omission, is what makes bluff-parity true]; the extorter,
  holding no punitive want, never chooses punish. Holding is the POSITIVE
  authoring act that distinguishes the two settings — no flag needed, and
  both settings are declared in inspectable world state [D-C1's principle,
  satisfied by a cleaner mechanism than its proposed `coBluff`].
- **THE ACCIDENT — a loud TypeCheck net, the `DeonticUnclosed` precedent
  applied to its isomorphic case [D-C1]**: an UNREGISTERED punitive name is
  neither setting — believed-desire resolution dangles, threats are silently
  inert, and that is the silent failure v49 documented as a hazard. New check:
  a world whose authored outcomes deposit a `believes.desires.<E>.punishes-*`
  belief (the threaten shape — detectable by the existing outcome walks) for
  a name absent from the registered desire vocabulary flags
  `CoercionUnmotivated`, naming the name and the fix (register it; hold it or
  not is the genuine/bluff choice). The v49 registration-contract prose
  re-frames to this three-state semantics — the two-setting claim is now
  EARNED by the net, not asserted over an unguarded omission.

## 3. Counter-coercion is composition (proven, not asserted)

A second `Coercion` whose `coTrigger` reads the first's marks (deliberately
unreserved, gateable fiction per v49). The fixture needs its OWN small world
[C-I2 — the racket world gives the victim no leverage material]: the victim
holds something over the extorter (the extortion's own mark is the natural
material: `Actor.extorted.V.<sid1>` — being known as an extorter is exposure),
a registered-and-held `punishes-<sid2>` desire, and a kernel. Both threats
stand; the table turns on ordinary scoring. If composition is blocked anywhere,
that is a BLOCK-and-surface finding.

## The shakedown decision, stated [C-I1]

`shakedown` keeps its signature and passes `Nothing`/`Nothing` — blackmail the
INSTANCE is a fixed fiction (permanent threats, permanent purchase), and the
consequence is accepted and stated: a world wanting expiring or serial
blackmail authors its own `Coercion` directly — the primitive, not the
instance, is the expressiveness home. `Blackmail.hs:17`'s permanence prose
stays true BECAUSE of this hardcode and says so.

## Construction sites and the clock (the mechanical estate)

Every existing `Coercion` literal gains the two fields explicitly — enumerated,
not discovered [C-C1, S-I3]: `Blackmail.hs:107` (shakedown's record), and
CoerceSpec's fixtures (the racket at :26, `blackmailShaped` at :104, and any
sibling literal the implementer's grep finds — `-Wmissing-fields` makes an
omission ⊥ at coerce's read, taking down every coercion pin, so this is
checked BY NAME). The expiry fixtures DRIVE THE CLOCK [C-C2]: `InsertFor`
retracts only at `roundBoundary`, which the shipped racket harness never
calls — the racket-cycle and stale-threat pins tick boundaries explicitly
(the ScheduleSpec idiom), or they are vacuous.

## Classification

Expressiveness extension with a byte-identical default: Nothing/Nothing and
registered-and-held reproduce the shipped compilation and semantics exactly;
all v49 property pins, goldens, and analyses UNMOVED — observed, not asserted.
New behavior only where a fixture authors `Just n`, a bluff, a counter, or the
lint's accident case.

## The property contract (violating any is the BLOCK)

1. **Nothing is today**: the Nothing/Nothing compilation structurally equals
   the shipped one (Eq over the generated actions).
2. **The racket cycles**: with `coComplianceLasts = Just n` and the clock
   driven — threaten→comply extracts; the re-threat's comply stays blocked for
   n boundaries; at expiry the extorter re-threatens and EXTRACTS again. One
   PURCHASE per bought period (the purse property; the person property is
   observed and recorded as shipped-identical [S-I2]).
3. **A stale threat is spent**: with `coThreatLasts = Just n`, after n
   boundaries the un-answered threat stops offering punish's standing arm and
   stops pressuring comply; the defied arm survives expiry.
4. **Stalling never dominates while the threat stands** (property 1,
   time-scoped).
5. **The bluff pair**: registered-not-held — the victim's comply/defy decision
   is IDENTICAL to the genuine case; the bluffing extorter, defied, declines
   punish where the genuine one chooses it (pickAction both sides).
6. **The accident is loud**: a threaten-shaped deposit for an unregistered
   punitive name flags `CoercionUnmotivated`; the genuine, bluff, and
   no-coercion worlds are all clean; every shipped world stays
   `typeCheck == []`.
7. **The table turns by composition**: the counter-coercion world reaches a
   standing counter-threat from pure content over the shipped surface.

## Verification

CoerceSpec grows the fixtures (RED-first each; the lint RED via the
pre-wire/neuter path); BlackmailSpec untouched and re-observed; the v49
six-property pins re-observed green; one mid-racket save/resume pin (save with
the complied-expiry due pending, reload, drive boundaries, the cycle resumes on
schedule [C-M2]). Docs, enumerated [C-I3..I5]: `Coerce.hs:165-167`'s PERMANENT
comply comment (false under Just n — rewritten to the two-case truth);
`Blackmail.hs:17` (true via the hardcode — says so); the Coerce module haddock
(three-state credibility; lifetime fiction); WALKTHROUGH:1510-1519 (the serial
extortion bank paragraph — discharged) and :1656; LEDGER: the v54 row PLUS
status updates at the existing bank sites per the DONE-banked-vN convention
(:2695-2704 the serial-extortion entry and v30 forward pointer, :1924-1934 the
v49 cross-reference). Pre-gate: the panel ran; verdicts SOUND / FLAWED / GAPS,
folded; the amended spec is what gates.

## Out of scope

Nothing carried: this round empties the coercion bank — threat expiry and
serial extortion ship as fields; bluffing ships as the registered-not-held
setting with its accident netted; counter-coercion ships as a proven
composition with a standing fixture.
