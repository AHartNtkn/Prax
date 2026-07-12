# v32 — Confession & absolution (`Prax.Confession`)

A deliberately small round (user-sized): the mark-discharge arc v25 parked, with the
recidivism design settled in review with the user. If the probe shows the two-party
structure degenerating into trivia, the honest fallback is authored content with no new
module — stated up front.

## The design decisions (the round's actual content)

1. **Marks convert, never delete** (atonement-not-amnesia, aimed at the psyche):
   confession turns `<who>.lied.<hearer>.<event>` into `<who>.confessed.<hearer>.<event>`.
   The memory persists; its *valence* changes — traits price `lied` marks negatively and
   `confessed` marks at 0 or a mild authored residue. The conversion shape is the fixed
   vocabulary convention (per-event arity preserved) so traits price it consistently.
2. **Confession is self-incriminating by design**: the act deposits the deed into the
   hearer's beliefs through the normal sourced-hearsay channel
   (`<hearer>.believes.<event>.heard.<confessor>`) — the marks-not-records commitment's
   front door (truth recovery flows through mark-bearers). Consequences follow from what
   hearers now KNOW: the whole rumor/reputation stack cascades on a confession exactly as
   on gossip.
3. **Absolution is a separate, refusable, second-party act** — the cheap-grace loop's
   structural close. Confessing clears your conscience (the mark converts); only an
   absolver's grant clears your standing (inserts the world's defeater, e.g.
   `recanted.<who>` — v30's dangling hook finally wired). You can confess and be refused:
   conscience clean, standing dirty.
4. **Fed-up-ness is knowledge, not bookkeeping** (the recidivism mechanism, settled with
   the user): an `incorrigible` axiom (notoriety's Count idiom pointed inward) derives
   `regards.<W>.<who>.<label>` once W *believes* ≥k distinct past deeds by `<who>` — and
   `absolve` is gated on the absolver not so regarding the confessor. Properties that fall
   out rather than being bolted on: permanence by memory (beliefs never die, so a
   made-up mind stays made up); per-absolver patience (regards are per-regarder); warnings
   count (gossiped deeds feed the same threshold); confession-shopping works until word
   spreads — true to life, not a hole.
5. **Re-offense deletes the defeater** (v21's re-steal idiom): the village's whisper gains
   `Delete "recanted.Actor"` — lie again and standing snaps back from the beliefs nobody
   lost, before a now-less-patient audience.
6. **Recidivism-into-character (trait acquisition) is BANKED**, per the user: becoming a
   liar by lying needs bearer-side desires to be fact-driven (`charDesires` is a static
   character field), a Minds/engine change belonging with the Arc vocabulary. Ledgered
   with that obstacle stated.

## The module (thin by intent)

```haskell
confess :: String       -- mark kind, e.g. "lied" (single segment; loud error)
        -> CoPresence   -- who can be confessed to
        -> String       -- MARK pattern (the mark's event arity, e.g. "stole.C.loaf")
        -> String       -- DEPOSIT pattern (what the confession reveals — may
                        --   reference Actor/H, e.g. "whispered.Actor.H"; must be
                        --   subject-first for absolve's convention)
        -> String       -- action label
        -> Action
  -- AMENDED after implementation blocked (the anticipated shape finding): a
  -- content-shaped mark (conscience: "stole.C.loaf") and an act-shaped standing
  -- ("whispered.V.H") cannot share one pattern — the deposit decouples them.
  -- What a confessed lie reveals is the ACT and its falsity, not a re-assertion
  -- of the content; the deposit pattern is that act-truth, grounded from the
  -- mark's own bindings (Actor, H, and the mark's event variables). Worlds with
  -- self-shaped deeds pass the same pattern twice — explicitly, no default
  -- (one signature, no dual). Un-deceiving the original hearer (retracting the
  -- planted content-belief) is BANKED: it needs belief-retraction semantics.
  -- conditions: own mark exists (Match "Actor.<kind>.H.<pat>"), hearer co-present
  --   (asRole), hearer ≠ Actor; one confession per (hearer, deed) via the mark
  --   conversion itself (the lied-mark is the precondition and it converts away).
  -- outcomes: Delete the lied-mark path, Insert the confessed-mark path,
  --   Insert the hearer's sourced belief.

absolve :: String       -- the world's defeater fact prefix, e.g. "recanted"
        -> String       -- event pattern (binds the confessor from the believed deed)
        -> String       -- the incorrigibility label this absolver's patience checks
        -> String       -- action label
        -> Action
  -- conditions: Actor believes the deed heard from its own doer (the confession's
  --   deposit shape — you absolve what was confessed TO YOU, not gossip),
  --   Not (regards.Actor.<confessor>.<incorrigible-label>), Actor ≠ confessor.
  -- outcomes: Insert "<defeater>.<confessor>".

incorrigible :: String -> Int -> String -> Axiom
  -- deed pattern (FIRST variable = the offender; reserved-var guards per the
  -- v31 lesson), threshold k, label:
  -- Subquery/Count over W.believes.<pat> instances ≥ k ⇒ regards.W.<offender>.<label>.
```

Reserved-variable guards on every pattern argument (the v30/v31 class, guarded at birth).

## Arithmetic to probe BEFORE pinning (the plan's Task-1 probe, v30's discipline)

- **Spontaneous confession**: a conscience-bearer's mark conversion is utility-improving at
  depth 0 (−k relieved), against the social cost the deposit realizes (concealment fears,
  standing). Cornered/cheap-secret confessions should fire; expensive secrets shouldn't.
  Both sides pinned like v30's compliance arithmetic.
- **Confession as blackmail defense** (the signature composition): a confessed secret is
  spent leverage — after confession the extorter's expose deposits nothing new, so threats
  collapse. Rational exactly when the demanded price outweighs the realized fear — the
  fixture constructs that case (high price, mild secret) and the converse.
- **Eve's redemption** (village arc, forced trajectory per the theft/wedding precedent):
  confession's deposit risks tripping her own slanderer threshold BEFORE absolution
  dissolves it — whether depth-2 sees through confess→absolve (needs a believed absolver
  desire: a named, professed `merciful` desire on the absolver) is exactly what the probe
  must measure. Fallback if it doesn't drive: eve confesses to GALE (who already believes —
  a free confession, no new regard) and gale absolves; stated as the fallback arc now so
  the plan probes both and ships one.

## Tests / gates

ConfessionSpec pins: conversion (mark persists in confessed form, priced by a trait at the
residue), the deposit, absolve's grant + refusal gate, incorrigibility (k−1 vs k, gossip
feeding the threshold, per-absolver independence), re-offense snapping the defeater, both
sides of the spontaneous-confession arithmetic, the blackmail-defense composition both ways.
Village: the chosen redemption arc; whisper gains the defeater-delete; existing tests
unmodified except sanctioned additions; goldens byte-identical or one itemized re-capture
if the village vocabulary change shifts free play (expected NOT to — the new affordances
are forced-trajectory only and eve's free play never confesses unprompted unless the probe
says otherwise; any drift is itemized per the v30 discipline). Suite green; usual gates.

## Out of scope

Trait acquisition (banked, obstacle stated); confessor-side penance obligations; public
confession (one-to-many); priest-like roles; village faction wiring (still deferred).
