# v35 — Intentions: reconsideration semantics for NPC deliberation

User-directed, and unlike every round since v26, **an accepted spec change**, not an exact
optimization. The user's challenge: "agents would not plan potentially whispering secrets
every few seconds — always considering every possibility every step is not realistic and
very wasteful." Three probes ground the redesign:

1. **Stability ceiling.** Over a 70-turn village drive, computing every character's
   hypothetical pick every turn: **91.5% unchanged turn-over-turn** (379/414); carol — one
   of the most expensive deliberators — never changed her pick once in 70 turns. The
   volatility sits exactly where the story is (eve 14 changes, gale 10).
2. **The anchor road is structurally exhausted.** A chained cache simulation using v34's
   full machinery upgraded with per-reachable-head cones (the banked precision lever,
   probe-side) served **zero** picks from cache: every non-wait action's cone reaches all
   six characters, because the village's axiom graph deliberately chains co-presence into
   reputation (movement → togetherness → witnessing → belief → notoriety → regard). Anchor
   read-sets are state-independent; they cannot see that gale walking to the mill is
   irrelevant to carol in the square. No refinement of the v26–v34 proof family fixes this;
   it is that family's precision limit on socially-entangled worlds.
3. **Motivational triggers discriminate.** Three cheap state-level signatures — the
   character's candidate set, own-want satisfaction, live-desire set — caught **all 35**
   real pick changes while licensing **290/414 (70%)** skips, at roughly depth-0 cost
   (milliseconds) against ~700ms average full picks. Zero misses **on this trajectory** —
   measured, not proven, which is exactly why this round is a semantics change and was
   gated as one.

## The semantics (the round's actual content)

A character holds a **standing intention**: the grounded action (possibly none) chosen at
their last deliberation, together with the **motive signature** that deliberation was based
on. On their turn:

- compute the current motive signature (cheap);
- if it equals the stored one, **act the standing intention without deliberating** —
  commitment is the default, per Bratman: deliberation is expensive, so rational agents
  reconsider on relevant change, not on a clock;
- otherwise deliberate in full (the existing depth-2 `pickAction`, unchanged in every
  detail), act the result, and store the new intention + signature.

**The motive signature** is the answer to "what are the inputs to *wanting to reconsider*?"
— four components, each authored meaning, none a tuned number:

1. **What I can do — refined by measurement to what I can do THAT I CARE ABOUT** (amended
   mid-round: the original grain, full grounded-candidate equality, made the mechanism
   inert — 120/120 real turns re-deliberated, because villagers move every round and
   movement churns co-presence groundings; per-turn probes had measured the wrong
   interval). Two sub-components, probed at the real own-turn interval (140 village
   turns): **(a) my standing action is still offered** (membership by full grounded
   equality — a stale grounding can never be acted); **(b) the set of want-bearing
   affordance templates** — action identities whose authored outcome atoms may-unify some
   pattern of my own wants or desires (the v26 improvability test, scoped per character
   per template; an unresolvable `Call` conservatively bears). Opportunities that touch
   what I care about interrupt me; irrelevant comings and goings do not. The probe ladder
   that fixed this grain: full groundings = 0% served; templates unfiltered = 26% served,
   0 divergences; templates dropped entirely ("bold agent") = catastrophic — dana served
   "Wait" all drive while fresh deliberation wanted "shun carol" every round (opportunity
   appearance carries real dramatic signal); want-bearing templates = **38% served, 0
   divergences**, remaining deliberations all with defensible causes (arrivals expiring
   movement picks, genuine satisfaction moves, first turns).
2. **How I am doing**: the per-want satisfaction count vector over `selfWants` (counts, not
   their utility-weighted sum — two different profiles must not mask each other by summing
   equal).
3. **What is driving me**: the live-desire set — own named desires that are statically
   improvable AND not dead-now (v33's floor/gate machinery, pointed at oneself).
4. **What motives I know of**: the believed-motive facts the character holds
   (`<self>.believes.desires.<other>.<desire>` — the family `believedDesires` reads),
   so learning through gossip that someone wants something is grounds to re-plan around
   them.

Signatures are compared against the character's **last deliberation**, not the previous
turn: a change that appears and fully reverts between two of the character's turns leaves
the signature equal and the intention standing — "nothing differs now from when I decided."

**What this deliberately gives up:** reconsideration driven purely through predicted
OTHERS' responses to newly-available actions lags one beat (the bearing test is
first-order: my outcomes touching my wants — a payoff reachable only via another's
predicted reply to a fresh affordance is invisible until a signature-visible consequence
lands); and a pick can in principle flip on a pure prediction-input change — another character's private state shifts what the mover would be
predicted to do, while all four signature components hold still. Under these semantics the
character keeps their intention one beat longer, until a signature-visible consequence
lands. That is the accepted trade (arguably the more realistic fiction: she had not
re-thought it yet), it is pinned by a dedicated test as INTENDED behavior, and the current
village content never exercises it (the probe's zero misses).

**No dual system.** The always-deliberate loop is replaced, not flagged: `npcAct` consults
the intention store, period. Forced-trajectory helpers (`performAction`/`performOutcome`
driving) are unaffected — they never deliberated. The player never deliberates. Dead
characters neither act nor retain live intentions of consequence.

## Mechanics

- `Prax.Types`: `Intention` (standing action + signature) and an `intentions :: Map String
  Intention` runtime field on `PraxState` (the `cursor` precedent: loop state, not derived
  state; untouched by `retable`).
- `Prax.Planner`: `motiveSignature :: PraxState -> Character -> MotiveSignature` — the four
  components, one home beside the machinery they reuse (`candidateActions`,
  `cookedSelfWants`, `deadNow`, the believes walk).
- `Prax.Loop.npcAct`: signature-compare → act standing intention, or deliberate + store.
  `pickAction` and everything beneath it unchanged.

## Verification

- **Goldens are re-captured under the new semantics** — a deliberate spec change, own
  commit, drift itemized line by line. Expected: byte-identical on current content (the
  probe's 35/35 with zero misses); any observed drift is examined and either accepted as
  the semantics working (itemized with the trigger trace) or diagnosed as a defect.
- ViewInvariant untouched (views are not affected by who deliberates when).
- LoopSpec/PlannerSpec additions, RED-first where the behavior is new:
  - quiet character does not deliberate (observable: the standing intention is acted even
    though a fresh pick WOULD differ — the in-principle gap constructed explicitly, pinning
    the semantics as intended);
  - each trigger fires alone: an options change, a satisfaction change, a live-set flip
    (hunger gate), a learned motive — each causes re-deliberation (the pick updates);
  - a cached action invalidated by its own conditions (left the candidate set) is never
    acted;
  - persistence across the character's own action when nothing signature-visible moved.
- **Perf acceptance**: the 31-test village A/B (uncontended, best-of-3) against the
  recorded epochs 31.11 / 171.64 / 132.75 / 120.10–123.57; the full suite timed. Reported
  as measured, wherever it lands.

## Out of scope

Plan-shaped intentions (action queues — practices already carry multi-step structure);
interruptibility as a personality trait (per-character commitment styles — natural
follow-on, needs its own design); anything approximate inside deliberation itself;
invalidation bookkeeping (the signature-compare design needs none).
