# v37 — Calendar & gatherings: the clock convenes, the town shows up

User-directed. The banked item: recurring clock-gated scene spawns (market day, festival) —
the mixing dynamic that makes gossip percolate. Probed live before speccing (the round
protocol), with one clean success and one real discovery:

- **The calendar already works.** A pair of v36 pulse rules — open (inserts the gathering's
  practice instance + its `marketDay.now`-style event fact) and close (deletes both), the
  close due seeded one duration past the open — ran a 26-round probe opening the market at
  round 6 for 3 rounds, recurring every 6, self-sustaining, exactly on schedule. Spawning a
  practice instance from a pulse body works today (`spawnedInstanceNames` fires, inits run);
  teardown closes it (the v36 eat-teardown precedent).
- **But nobody attended — and the diagnosis is a v36 regression, not a v37 gap.** Fresh
  deliberation at the open market picks "Go to square" (observed; the attendance desire's
  arithmetic works). The characters' motive signatures, however, are byte-equal before and
  after the market opens (observed): the attendance want is a conjunction (event ∧
  presence) whose satisfaction count stays 0 at opening, so the wake must come from the
  live-desire set — and `loves-the-fair` classifies `AlwaysLive`, not `GateCheck`, because
  **v36 made the ticker an ordinary practice action, polluting the insert pools**: every
  clock-moved fact (`marketDay.now`, `hungry.*`) now looks "action-insertable" to the v33
  gate classifier, which therefore refuses to gate on it. Conservative direction, so v36
  stayed exact — but it silently degraded gate precision for exactly the fact family the
  clock exists to move, and here it masks the entire point of gatherings.

## Design 1: the ticker is the environment, not an author (the fix)

The user's position, adopted as the semantics: tickers change motives — NPCs must make
different decisions when things relevant to them become true. Mechanically: the relevance
analysis's world atom pools (`Prax.Relevance.worldAtomPools`, feeding `improvableDesires`
and `livenessOf`) EXCLUDE the drift practice's outcomes. The v33 environment-gate concept
— "a fact family no authored outcome inserts and no axiom derives" — regains its defining
example: clock-moved facts are what the environment does.

Exactness, both consumers: the static improvability screen feeds `predictMove`'s
pair-skip, which is about MOVER ACTIONS — a desire improvable only by the clock has no
improving mover-action for the strict-improvement scan to find, so skipping is exact.
`deadNow`'s gate check reads the fact's presence at evaluation time either way;
conservativity direction unchanged. Consequences, all wanted:

- `loves-the-fair` (event ∧ presence, positive) ⇒ `GateCheck [event]`: dead between
  gatherings (zero planning cost town-wide), LIVE the round the market opens ⇒ the
  live-desire component of every attendee's v35 signature flips ⇒ they re-deliberate and
  converge; the gate shuts at close ⇒ they re-deliberate and disperse. The event starting
  IS a motivational change, and now the machinery says so.
- v36's hunger-shaped positive desires regain their gates (the silent precision regression
  repaired); FloorCheck negatives were never affected.
- `Prax.Drift` exports its practice id; `Prax.Relevance` consumes it (no cycle — Drift
  imports only Types/Query/Db).

## Design 2: the `gathering` combinator (formalizing the probe)

```haskell
gathering :: String        -- name (single segment; loud error)
          -> Int           -- period: rounds between openings (authored meaning)
          -> Int           -- duration: rounds it stays open; 0 < duration < period
                           --   (loud error otherwise — no overlap, no null event)
          -> [Outcome]     -- open effects (the instance spawn + the event fact)
          -> [Outcome]     -- close effects (their teardown)
          -> ([DriftRule], [Outcome])   -- (two pulse rules, their due seeds)
```

Two rules named `<name>Open`/`<name>Close`, both `period`-cadenced; the seeds place open
at `period` and close at `period + duration` (v36's start-sated convention: the first
gathering convenes one full period in). Home: `Prax.Drift` (it composes pulse rules; a
separate calendar module would be a wrapper). `driftSetup` keeps working for plain rules;
`gathering`'s seeds ride the same setup list.

## Cargo: village market day

Every `marketPeriod` rounds the market convenes in the square for `marketDuration` rounds:
the open spawns a `market` practice instance and `marketDay.<square>`-shaped event fact;
villagers hold an attendance desire (event ∧ at-square, positive, authored weight with a
stated sentence — strong enough to outweigh anchoring wants, weaker than arcs' stakes so
drama still outranks festivity). No market-only affordances are REQUIRED for the round's
payoff — co-presence density is the point: sightings refresh town-wide, witnessed acts
reach everyone at once, whisper hearer fans widen (the percolation the bank entry named).
One market-flavored affordance MAY ship if the plan finds a cheap one with story stakes
(browsing is set dressing; skip it unless it earns its place).

Free play will change substantially (a synchronized town rhythm): the village golden
re-capture is expected LARGE, its own commit, itemized line by line with causes — and the
drama pins (theft arc, whisper arc, hunger cycle) must still complete, re-indexed as
needed with each re-index traced (the v36 postTheftAt discipline).

## Verification

- RelevanceSpec: the reclassification pinned both ways — a clock-moved fact IS a gate
  (fixture drift rule inserting the want's conjunct ⇒ GateCheck, was AlwaysLive); an
  action-insertable fact still is NOT; hunger's `wants-food` shape regains GateCheck;
  negatives untouched. The v35 wake pinned end-to-end in LoopSpec: a standing intention
  holds while the gathering is closed, the open flips the live set and the character
  re-deliberates TO the gathering, the close disperses them (the probe's trajectory, as a
  test).
- DriftSpec: `gathering` cadence (open at period, close at period+duration, recurrence),
  the duration guards, name guards inherited.
- VillageSpec: convergence (attendees at the square during market, not between), the
  percolation observable (a fact witnessed at market reaches more believers than the same
  fact witnessed on a quiet day — pinned with counts), arcs re-verified.
- Goldens: village re-captured (own commit, itemized); bar/intrigue/feud must not move.
  ViewInvariant green; suite green; paired drive bench re-run (the market rounds will wake
  attendees — expected, bounded; report the serve-rate as measured).

## Out of scope

Multiple simultaneous gatherings and venue conflicts (one market first); invitations/
exclusivity (who MAY attend is co-presence + desire, not access control); festival content
beyond the market instance; the chronicler noticing gatherings (banked with it).
