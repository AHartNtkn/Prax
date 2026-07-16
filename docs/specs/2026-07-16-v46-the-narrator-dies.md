# v46 — The narrator dies: story flow is scheduling, and the boundary learns to speak

Second of the four audit-queued rounds (v45 protected families → **v46** → v47 function
registry → v48 generality bundle). The audit's HIGH finding: `_narrator` is the
surviving instance of v44's own named defect — a bodiless character occupying a roster
slot, driven through the full planner, whose single Want (`Match "storyAdvanced.J"` at
weight 100) exists so junctions and memories can bribe the planner into firing them.
"Fire the moment the condition holds" is a scheduling statement; v44 built the home.

This spec was rewritten after the three-lens isolated pre-gate review
(`.superpowers/sdd/v46-spec-review-*.md`) — the panel's findings forced the three
design moves below ([S]/[D]/[C] cite the forcing lens); the first draft's
junction-as-plain-rules and atSince-reservation designs were UNSOUND as written.

## The design

**1. The schedule gains first-match clauses, and junctions become ONE rule.** The
first draft compiled each junction to its own period-1 rule — the panel showed that
reproduces neither the narrator's one-per-round selection nor its tiebreak, and admits
intra-boundary transition CASCADES (S→X→Y in one boundary, skipping X's beats) and
ending-vs-transition order accidents [S][D]. The fix is a precedented primitive:
`ScheduleRule` gains a clause mode — `AllClauses` (today's semantics, the default) or
`FirstMatch` (the `Function`/`FnCase` semantics the engine already owns: the first
clause whose conditions hold fires, the rest are skipped). A script's junctions AND
memories compile to ONE `FirstMatch` period-1 rule, clauses in AUTHORED scene order:
at most one story event per boundary, no cascade (a new scene's junctions evaluate at
the NEXT boundary), and multi-binding memories fire once ([S-I4] resolved by
first-match). Within a scene, MEMORIES precede junctions in the clause order (stated
law: exposition fires before the story can leave the scene that holds it — a junction
firing first would strand a reachable memory unfired); among themselves, each keeps
authored order. The single rule carries ONE name (`story`); `scheduleDues` and
Persist's `due` lines key globally by rule name across BOTH doors [C-I2], so
registration gains a duplicate-rule-name guard spanning doors — an authored rule named
`story` in a script world errors loudly at build. THE TIEBREAK LAW CHANGES, deliberately and stated: today simultaneous
enabled junctions resolve by the planner's alphabetical-label sort — an accident of
`sortOn (Down score, gaLabel)`; henceforth AUTHORED ORDER resolves — scenes in
declaration order, a scene's junctions in declaration order, endings/transitions
exactly where the author put them. Authored order is a statement; alphabetical labels
never were. Goldens/re-pins that shift because of the tiebreak change are itemized as
exactly that.

**2. Two DOORS to the schedule, one type — machinery is unrepresentable in the
authored door.** The first draft would have failed to build at all: `setSchedule`'s
guard forbids Prax variables in every clause, and compiler-emitted rules (the sighting
rule, junction machinery: `clockReached`'s `PraxE`/`PraxNow`, the `sceneEntered`
stamp) legitimately carry them [S-C2]. The T3 lesson applies (fix the representation,
not the guard): `setSchedule` remains the AUTHORED door, fully guarded, where
machinery stays unrepresentable; compiler modules (`Prax.Schedule.sightRule`,
`Prax.Script.compile`) register through an internal engine door (`schedulePart` /
unexported registration) that carries no authoring guard — the same
definitions-vs-engine-calls boundary as `performOutcome`, squarely inside v45's stated
threat model (compiler-level code is trusted by definition). One `ScheduleRule` type,
one boundary executor; the doors differ in guarding, not in kind.

**3. The boundary learns to speak — narration renders per fired clause.** The
narrator's labels were the story's narration, so the schedule gains a display surface:
each clause may carry a label template (rendered with the firing binding, like
`actionName`); the engine's rule-firing loop — which owns the query-then-apply per
clause under `FirstMatch` anyway — renders labels per firing [D-1: the old
delegate-to-CForEach path hid the bindings; the executor now grounds clauses itself,
one implementation for both modes]. `roundBoundary` returns `(PraxState, [String])`;
`advance` grows the narration output and every caller changes BY TYPE, loudly [C-C1]:
`runNpcTicks` threads boundary lines into the trace it already returns, the CLI
prints them, `Stress` discards them explicitly. Blank labels are silent — stated as a
NEW rule for this channel [D-2] (the action trace never filtered; nothing blank
remains there since v44). Narration is presentation: no fact, no Persist impact from
the channel itself. The FORMAT HEADER bumps to `prax-state v3` regardless [C-I3]: a
v45-era script save carries `storyAdvanced.*`/`memoryFired` facts and
`junctions`-practice instances but no `story` due — format-indistinguishable from v46
under the v2 header, it would load as inert stale state with the story rule unseeded.
The header machinery exists for exactly this; the bump makes the incompatibility loud.

**4. The CLI save point moves past the boundary** [S-I5]: today `savePoint` is
pre-advance, so a resume would replay the boundary and re-print narration the player
already saw. The boundary belongs to the completed round: the CLI saves the
POST-boundary, pre-player-turn state — resume never replays a narrated boundary. (The
boundary stays a pure state function; this is only where the CLI snapshots.)

**5. `atSince` reservation is DROPPED from this round** [S-C3b][D]: `atSince` is a
mid-path segment (`<seer>.believes.atSince.<seen>!<turn>`), and the v45 reserved-table
mechanism is head-keyed with whole-tail shape — admitting mid-path families is a
genuine mechanism redesign, not a table entry. The first draft underspecified it by an
order of magnitude. The v45 residue stands with its pointer, now annotated: the cost
is mid-path family matching; the datum is half-fiction (a memory of when you saw
someone); revisit only if evidence of real corruption appears. The `Now→PraxNow`
sighting rename is also dropped — under the two-door design the sighting rule enters
by the engine door and meets no authored guard, so the rename buys nothing [S-3a
dissolved].

## What dies

`_narrator` + `narratorName`, the `junctionsP` practice, the `storyAdvanced.*` family
(closing v45's deferral), every roster/setup entry for them. Script casts contain only
characters again.

## Semantics shifts, stated honestly

(a) Junctions fire at round boundaries, not on a roster turn — within-round timing
shifts. (b) The simultaneous-junction tiebreak becomes authored order (was
alphabetical label, an accident). (c) At most one story event per boundary — the
cascade the plain-rules design would have introduced is explicitly rejected. Script
goldens and AnalysisTable pins re-capture with per-class itemization (removed
`_narrator` rows/lines [C-I1], boundary-timing shifts, tiebreak-law changes — and
nothing else); real characters' decision content argued equivalent.

## Verification

- The FirstMatch primitive pinned on its own (first clause wins; later clauses
  skipped; next boundary re-evaluates) and via Function-parity (same semantics the
  engine's `FnCase` already has).
- One-story-event-per-boundary pinned (two co-enabled junctions → one fires, authored
  order); no-cascade pinned (transition enables the next scene's junction → it fires
  at the NEXT boundary); ending-vs-transition order pinned as authored.
- Narration channel: labeled clause renders with its binding at the boundary; blank is
  silent; the CLI resume does NOT re-print (the moved save point pinned).
- Memories: fire exactly once (latch), one binding (first-match), text in the trace.
- storyAdvanced grep-proof; ScriptSpec/JsonSpec/DirectorSpec re-expressed with fiction
  preserved; time-free non-Script worlds byte-identical.
- StressSpec's play-world coverage assertions [C-I4] are re-argued, not assumed: the
  narrator leaves the random-mover pool and junction firing becomes deterministic at
  boundaries, changing reach/dead-end dynamics — the scenes-and-endings coverage and
  zero-dead-ends assertions must be re-verified against the new dynamics and any
  change adjudicated (a coverage loss would be a real finding, not a re-pin).

## Out of scope

v47, v48. Mid-path reserved families (the atSince mechanism — residue, annotated).
Narration templates beyond the `[Var]` rendering actions already use. The chronicler
(banked; the narration channel is its future home, none of it built here).
