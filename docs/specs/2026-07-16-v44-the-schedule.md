# v44 — The schedule: the engine owns time; authors declare, never script it

A paradigm correction, user-directed, full blast radius. The defect class surfaced by
v38's `feelingsFade` (a global sweep masquerading as per-feeling decay) and generalized
at review: **scheduling is engine semantics, not world content.** Today every temporal
mechanism is scripted — bodiless ticker CHARACTERS (`_sight`, `_drift`, v43's `_time`)
occupy roster slots and are driven through the full planner (`npcAct`/`pickAction`,
intention store and all) to execute unconditional bookkeeping; pulse cadence is
authored `due.*` facts compared against an authored clock fact by compiled gate
machinery; wear-off is an authored mass-delete rule. None of that should be
REPRESENTABLE in user-facing code. The author declares temporal properties (this rule
fires every n rounds; this fact lives n rounds); the engine decides when and how.

## The design

**One engine-owned schedule.** `PraxState` gains a round counter and a schedule: a
due-round-keyed queue of entries —

- **Recurring rules**: the `DriftRule` declaration shape survives (name, period, body
  clauses); firing means grounding each clause's conditions against the world and
  applying its outcomes per binding (the same outcome machinery actions use), then
  re-arming period rounds FROM NOW (the v36 stall semantics, kept verbatim). Seeding
  keeps the v36 convention: first fire one full period in (the world starts sated).
- **One-shot expiries**: an insert declared with a lifetime (`lasts n` shape on the
  authoring side — `feel` and friends grow duration-taking forms) enqueues a retract at
  round + n. Re-inserting the same fact refreshes the expiry (the new onset supersedes;
  being provoked again renews the anger). The retract runs through the normal retract
  path, so closure recompute, intention invalidation, and liveness wakes all follow
  from ordinary delta processing.
- **The clock fact**: the engine maintains `turn!N` itself, advancing it at each round
  boundary — authored conditions (`sightedWithin`, gathering gates) keep reading time
  as a fact; no world seeds or ticks it ever again.

**The round becomes explicit.** Today a round exists only as one cursor rotation. The
loop gains a real boundary: when the rotation wraps, the engine runs the schedule —
clock advance, then due recurring rules in declaration order, then due expiries —
before the next round's first turn. This is the same semantic point as today's
tickers-ride-last roster convention, now structural. Scheduled changes are ENVIRONMENT
by construction: they are not actions, appear in no candidate set, are never simulated
inside imagined lookahead (v37's classification, previously enforced by a pool-deletion
hack in `worldAtomPools`, now true because schedule rules are not in `cookedDefs` at
all).

**Perception rides the schedule.** Sight's sighting rule is a period-1 recurring rule
(the authored sighting template, `ForEach`-shaped, exactly as compiled today) — the
`_sight` character dies with the others. Gatherings re-express as their two recurring
rules (open/close, the phase-offset seeding kept).

## What dies (no dual systems, no wrappers)

The ticker characters and their practices: `driftP`/`driftChar`/`driftName`/
`driftPracticeId`/`driftSetup`, Sight's practice/char/setup, `Prax.Clock`'s
`clockP`/`clockChar`/`clockSetup`/`clockName` (one day old; superseded without
ceremony — `turnPath` and the authored-time reading surface survive). The `due.*` fact
family and its compiled gate machinery. `feelingsFade` and the worlds' fade rules.
`ClocklessDrift` (the engine always has a clock) and its CLI text.
`worldAtomPools`' drift exclusion. Every world's roster and setup entries for tickers.

## Authoring surface

A world declares its schedule at build time (a setter beside `setDesires`/
`definePractices`): the recurring rules (drift-style declarations, the sighting rule,
gathering rules) in one list — declaration order is firing order within a round
boundary. Lifetimes are declared at the insertion site (`lasts n` outcomes; `feel`
duration variants). The v40/v43 splice guards carry over to the rule bodies unchanged
(machinery namespace + `Actor` forbidden — there is no actor at all now).

## Consequences traced

- **Analyses**: schedule-rule outcomes join `producibleAtoms` (the v42 lint must see
  clock-moved and scheduled facts as producible — `marketDay`, `turn`, expiring
  families) and the SeedlessDraw scan (rule bodies may draw); they stay OUT of
  `worldAtomPools` (no mover can take them — the improvability/liveness semantics of
  v37, now structural). Liveness gates on schedule-moved facts keep working (the v35
  wake fires when the schedule flips a gate — signatures are computed from state).
- **Exactness**: NOT byte-identical, deliberately — rosters shrink, step counts change
  meaning, every golden re-captures with itemized drift in its own commit; the
  AnalysisTable pins re-capture where the drift-practice exclusion previously shaped
  pools. Decision CONTENT must be argued equivalent where the fiction is unchanged
  (same mover choices in the same world states, fewer bookkeeping turns between them).
- **Persist**: the round counter and the schedule (per-rule next-due, expiry queue)
  serialize — string-side, no Sym ids; the v43 format header bumps to `prax-state v2`
  (the loud-rejection machinery just landed; this is what it is for).
- **Tests**: DriftSpec/ClockSpec/SightSpec rewrite against the schedule (clock-jump
  fixtures become schedule/counter manipulation or short periods); the v42 lint's
  clockless composition pin dies with ClocklessDrift.
- **CLI**: round boundaries run inside the same `advance` path the CLI and
  `runNpcTicks` share; blank-label suppression loses its ticker case (nothing silent
  is left to suppress).

## Out of scope

Per-emotion default lifetimes and intensity levels (banked); any change to the planner,
the Db, cooked formats, or query semantics; the chronicler; emotion visibility.
Superseded and closed by this round: the banked "per-feeling fade stamps" item (the fix
inside the wrong paradigm).
