# v44 — The schedule: the engine owns time; authors declare, never script it

A paradigm correction, user-directed, full blast radius. The defect class surfaced by
v38's `feelingsFade` (a global sweep masquerading as per-feeling decay) and generalized
at review: **scheduling is engine semantics, not world content.** Today every temporal
mechanism is scripted — bodiless ticker CHARACTERS (`_sight`, `_drift`, v43's `_time`,
Script's `_clock`) occupy roster slots and are driven through the full planner
(`npcAct`/`pickAction`, intention store and all) to execute unconditional bookkeeping;
pulse cadence is authored `due.*` facts compared against an authored clock fact by
compiled gate machinery; wear-off is an authored mass-delete rule. None of that should
be REPRESENTABLE in user-facing code. The author declares temporal properties (this
rule fires every n rounds; this fact lives n rounds); the engine decides when and how.

This spec was reviewed pre-gate by three isolated fresh-context reviewers (soundness /
architecture / completeness; reviews in `.superpowers/sdd/v44-spec-review-*.md`) and
amended throughout; the section markers [S], [D], [C] cite which review forced a
statement.

## The design

**One engine-owned schedule, one clock.** `PraxState` gains a schedule with two entry
kinds over one due-turn dispatch — genuinely different lifecycles, deliberately not
collapsed into one primitive [D]:

- **Recurring rules**: the `DriftRule` declaration shape survives (name, period, body
  clauses); firing means grounding each clause's conditions against the world and
  applying its outcomes per binding (the same outcome machinery actions use — verified
  actor-free [S]), then re-arming period rounds FROM NOW (the v36 stall semantics).
  Seeding: first fire one full period in (the start-sated convention) — uniform, since
  phase-offset seeding's only client dies with the gathering close rule (below). Rule
  bodies may read the clock as a fact (`Match "turn!Now"`) — that is how the sighting
  rule stamps `atSince` [C]. Persist re-associates next-dues to rules BY NAME on load;
  an unknown name is a loud error [S].
- **One-shot expiries**: an insert declared with a lifetime enqueues a retract at
  turn + n. REPRESENTATION [S — this forces a format change, accepted]: a new lifetime
  insert `Outcome` constructor with its cooked mirror, threaded through
  `cookOutcome`/`groundCookedOutcome`/`performCooked` (insert + enqueue) — lifetime
  inserts appear in ordinary ACTION outcomes (`feel`), not only schedule rules, so they
  ride the whole cooked pipeline. Every analysis (`worldAtomPools`, `producibleAtoms`,
  `groundedDeltaAnchors`) treats it exactly as its plain insert: the deferred retract
  is ENVIRONMENT, not a mover effect, and lookahead discards speculative enqueues with
  the rest of the imagined state [S].
- **The clock fact**: the engine maintains `turn!N`, seeded `turn!0` in `emptyState`
  itself — construction, not world setup, so every path (worlds, Script compile,
  fixtures) has a clock before anything reads it [C][D]. ENFORCED boundary: a new
  `typeCheck` rule flags any authored outcome or axiom head writing the `turn` family;
  the clock-jump test idiom is untouched (fixtures jump time through direct
  `performOutcome` calls, which scan no authored definition). `Prax.Clock` DIES
  ENTIRELY — `turnPath` moves to the one home every consumer already imports
  (`Prax.Types`); `tickConditions`/`tickOutcome`/`clockSeed`/`clockP`/`clockChar`/
  `clockSetup`/`clockName` have no surviving caller [D].

**The expiry queue's law** [S][D] — stated fully because "same fact" is subtle under
exclusion:

- Keyed by EXACT LABELED PATH (a path-keyed map, not a bare due-turn multiset — refresh
  and cancel must find entries by path; a multiset would let a stale earlier due kill a
  refreshed fact prematurely).
- Insertion supersedes wholesale: re-insert WITH a lifetime refreshes the due; WITHOUT
  one, CANCELS it (a permanent assertion never dies on a stale timer). This cancel is
  action-at-a-distance by design and gets its own tests [D].
- Any retract of the exact path — authored `Delete` (a vented feeling) or `!`-eviction
  (a displaced value) — PURGES the pending entry: no timer outlives its fact, no
  spurious no-op retract drives a later closure recompute [C][S]. A different value
  arriving in a `!` family is therefore eviction-purge, not "refresh": `mood!calm`
  displacing `mood!angry` kills angry's timer with angry.
- Expiry fires as a normal retract (then purges itself), so closure recompute and
  liveness wakes follow from ordinary delta processing. NOTE the retract takes the
  path's whole subtree (retract semantics): lifetimes belong on leaf facts — authoring
  guidance, stated in the `lasts` haddock [S].

**The round becomes explicit** [S][D] — precisely, since a vague boundary deranges
every due:

- Wrap predicate: `advance` computes the next living index i; **i ≤ cursor is a wrap**
  (equality included — a single-survivor cast wraps every turn; strict `<` would freeze
  time for the rest of the game). Initial `cursor = -1` means no boundary fires before
  round 1.
- Boundary order: detect wrap (pre-schedule cursor) → run the schedule → RE-select the
  actor from post-schedule aliveness (a rule may kill; the dead take no turn) → commit.
  Within the schedule: clock advance, then due EXPIRIES, then due rules in declaration
  order. Expiries first for a stated reason: a fact with lifetime n is present rounds
  onset..onset+n−1 and GONE at the boundary — rules-first would let the period-1
  sighting rule stamp a belief about a fact expiring that instant (a ghost
  observation). This is a global, un-authorable ordering law, chosen not forced;
  flagged as such [D].
- The boundary is a PURE function of `PraxState` — the schedule field (per-rule
  next-due + expiry queue) serializes, wrap detection reads only `cursor`, no
  out-of-state handles — so the CLI's save-before-advance/replay-on-resume idiom stays
  coherent; PersistSpec gains a schedule round-trip pin and a v1-rejection pin [C].
  Format header bumps to `prax-state v2`.

**Environment by construction.** Schedule rules live on their own cooked surface
(`cookedScheduleRules`, cooked once by `retable`'s machinery), NOT in `cookedDefs`:
movers cannot take them, they appear in no candidate set. The analyses split cleanly
[S][D]: `producibleAtoms` and `seedlessDrawErrors` (and GateSpec's source scan) fold
over practices PLUS the schedule surface — walked by the SAME `cookedOutcomeAtoms`
machinery, no parallel walker — so the v42 lint sees `marketDay`/`turn`/expiring
families as producible; `worldAtomPools` folds over practices alone (the v37
`Map.delete driftPracticeId` hack dies as dead code). Lookahead never simulates the
schedule for the REAL reason [D]: the boundary lives in the loop, outside
`performAction`/`scoreActions`, and the tickers leave the roster (hence `othersAfter`);
the `cookedDefs` absence governs improvability/liveness, a separate consequence. The
v35 wake for schedule-moved facts runs entirely through the state half of the motive
signature (satisfaction + liveness gates — the v37 mechanism); the bearing half loses
nothing real (there is no action to bear on), and the VillageSpec onset/fade wake pins
re-express against the schedule to keep this covered [C].

**Perception rides the schedule.** Sight's sighting rule is a period-1 recurring rule:
the authored sighting template, `ForEach`-shaped, with its `atSince` stamp re-expressed
to bind the clock fact (`Match "turn!Now"` in the rule conditions, `Now` in the stamp)
— the old form's `PraxM` came from tick machinery that no longer exists [C]. The
`_sight` character dies.

**Gatherings collapse onto expiry** [D]. A gathering is definitionally "assert X, and
it stops holding duration rounds later" — the open rule's inserts carry
`lasts duration`; the close rule, its phase-offset seed, and the `duration < period`
guard ALL die. One mechanism for "a temporary fact," not two. (Village's market: one
open rule inserting `practice.market.fair` and `marketDay.square`, both `lasts 1`.)

**Feelings migrate onsite, and the fiction changes deliberately** [C — this was the
round's origin and the spec must commit to it]: `feelingsFade`/`villageFade`/`barFade`
die; EVERY shipped onset converts — Bar's ~20 `feelToward` sites and Village's two
draw-nested anger onsets gain explicit authored lifetimes (`feelToward` gets the
duration variant; `feel` likewise; both compile to the ONE lifetime-insert mechanism
[D], values authored at migration with the standard test-compression label). This is a
semantic change beyond bookkeeping: per-onset spans replace synchronized mass-wipes —
each feeling now lives its own n rounds, which is the v36/v38 episodic principle
actually implemented. Play/Intrigue/Feud/Audience feelings are non-fading today (no
fade rule registered) and STAY non-fading — an authored choice preserved, not an
oversight [C].

**The scene clock dies too (no segregation).** `Prax.Script`'s `_clock` character and
the `sceneClock` family go; scene entry stamps the live clock via a bound insert
(`ForEach [Match "turn!Now"] [Insert "sceneEntered!Now"]` — a plain stamp cannot
capture a live value [C]) at BOTH the initial scene's setup and every junction
transition; a timed junction's condition compares `turn − entered ≥ n` (`Sub` exists in
`CalcOp`). Consumers itemized: shipped Audience's `timeout "dismissed" 5` and
ScriptSpec's two timed-junction tests. Every script world now carries the universal
clock (previously `usesClock`-gated) — intended [C][D].

## What dies (no dual systems, no wrappers, no dead code)

The ticker characters and their machinery: `driftP`/`driftChar`/`driftName`/
`driftPracticeId`/`driftSetup`; Sight's practice/char/setup; ALL of `Prax.Clock`
(`turnPath` relocates to `Prax.Types`); Script's `_clock` + `sceneClock`. The `due.*`
family and its compiled gates. `feelingsFade` and both world fade rules. The gathering
close rule/offset seed/duration guard. `ClocklessDrift` and its CLI text (the engine
always has a clock). `worldAtomPools`' drift exclusion. Every world's roster/setup
ticker entries. `Loop.narrate`'s blank-label suppression and Main's mirror (no silent
action remains — dead guards go, per the no-dead-code edict) [C]. Stale doc prose
describing the dying machinery as live (Drift/Clock module headers, `bearingTemplates`'
no-drifter-exclusion note, `Emotion`'s wear-off cross-reference, Sight cross-refs) is
updated with the code, fix-don't-confess [C].

## Authoring surface

A world declares its schedule at build time (a setter beside
`setDesires`/`definePractices`): recurring rules in one list — declaration order is
firing order. Lifetimes are declared at the insertion site (the lifetime-insert
outcome; `feel`/`feelToward` duration variants compiling to it). The v40/v43 splice
guards carry over to rule bodies unchanged (machinery namespace + `Actor` forbidden —
there is no actor at all now).

## Verification (beyond the usual gates)

- NEW boundary coverage [C]: wrap detection under dead-skipping (including the
  single-survivor equality case), boundary order (expiries before rules — a ghost-
  observation repro), no boundary before round 1, a rule killing a character mid-wrap.
- Expiry-law pins: refresh, cancel-on-bare-insert, delete-purge, eviction-purge, the
  `!`-family different-value case, subtree-retract semantics.
- Re-expressed unit pins keep their coverage [C]: EmotionSpec's fade group (schedule-
  driven), RelevanceSpec's three liveness-gate pins (schedule as the fact-mover — the
  unit coverage of "clock-moved facts gate; action-insertable facts don't" must
  survive), VillageSpec's onset/fade v35-wake pins, BarSpec/VillageSpec tick helpers,
  LoopSpec's round arithmetic re-captured plus the new boundary tests, PersistSpec
  schedule round-trip + v1 rejection.
- Exactness: NOT byte-identical, deliberately — rosters shrink, fade semantics change
  as specified; every golden re-captures with itemized drift in its own commit.
  Decision-CONTENT equivalence is argued where the fiction is unchanged, and the
  argument must cover sighting/pulse TIMING relative to mover turns (boundary-at-wrap
  vs tickers-in-roster), not only the roster shrink [S]. Time-free worlds
  (Intrigue/Play/Feud/Audience) recapture too — they now carry a ticking clock [D].

## Out of scope

Per-emotion DEFAULT lifetimes and intensity levels (banked; explicit per-site lifetimes
are this round). Any change to the planner's scoring, the Db, or query semantics — the
cooked OUTCOME format change (the lifetime insert) is explicitly IN scope [S]. The
chronicler; emotion visibility. Superseded and closed: the banked "per-feeling fade
stamps" item.
