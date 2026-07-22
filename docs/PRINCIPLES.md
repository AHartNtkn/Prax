# Prax — Principles

Cross-cutting design intents that govern the whole system and every game built on it.
These are easy to lose: they live implicitly across many specs and plans (and, in at least
one case below, survived only as a transliterated code comment). This document is the place
to consolidate them so they stop being reinvented. Lift intents out of individual specs into
here as they prove to be system-wide rather than local.

---

## Time: a turn is ~5 minutes

**A turn represents ~5 minutes of fiction, by default.** The mapping is loose — some actions
are a few seconds, some run a bit longer than five minutes — but **~5 minutes per turn is the
default assumption, and every real game built on this system MUST be designed under it.**

This is foundational, present from the system's inception. It was never recorded in a design
document; its only surviving written trace is a comment on the village hunger-drift rule
(origin: [`docs/specs/2026-07-14-v36-drift.md`](specs/2026-07-14-v36-drift.md)):

> real authoring is ~72 rounds — two meals a waking day at ~5 minutes a round

### What this scale implies

At ~5 minutes per turn:

- **1 hour ≈ 12 turns; 24 hours ≈ 288 turns.**
- A full **day/night cycle is hundreds of turns** — roughly 288 turns spanning a whole day
  (its day/night split is an authoring choice, not one turn each).
- A game covering **~20–30 days is on the order of thousands of turns**, not dozens.

### Test compression is allowed — but only when labelled as compression

Real games run long at the true scale, which is fine for authoring and play but expensive for
test drives and mass runs. Compressing a cadence (a short period, a short day) to keep a test
tractable is legitimate and has precedent (v36's hunger fires every 3 rounds in tests versus
~72 rounds in real authoring). The rule is:

- A compressed constant must be **named as compression**, with the real-scale value stated
  beside it. Never assert a compressed value as the real time semantics.
- The real ~5-minute scale is the semantic baseline the fiction is designed against; the
  compressed value is a test/stress convenience layered on top of it.

### Recorded violation (to be corrected)

The vampire village skeleton (`rust/prax-worlds/src/vampire.rs`) violated this: its
`TURN_DELAY`/`FEED_COOLDOWN = 2` were justified by a comment claiming "two turns is a full
day-night cycle — the design's 24h window," i.e. a turn = 12 hours — a value invented to fit
the design's "24h" timers because this principle was unrecorded. Its `phase_clock` also flips
day↔night every single turn (every ~5 minutes). The vampire game's time model is being
redesigned under this principle: real-scale day/night spans and timers, with any
test-compressed constants clearly marked as such.
