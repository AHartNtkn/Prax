# v46 — The narrator dies: story flow is scheduling, and the boundary learns to speak

Second of the four audit-queued rounds (v45 protected families → **v46** → v47 function
registry → v48 generality bundle). The audit's HIGH finding: `_narrator` is the
surviving instance of v44's own named defect — a bodiless character occupying a roster
slot, driven through the full planner (intention store and all), whose single Want
(`Match "storyAdvanced.J"` at weight 100) exists so that junctions and memories can
bribe the planner into firing them: every compiled junction/memory asserts a
`storyAdvanced.<key>` marker purely to raise the narrator's utility. "Fire the moment
the condition holds" is a SCHEDULING statement; v44 built the home for it and routed
perception there (`sightRule`), but left story flow on the planner detour.

## The design

**1. Junctions and memories compile to engine schedule rules.** Each junction becomes a
condition-gated period-1 `ScheduleRule`: fire at the round boundary when
`currentScene!sid` holds, no ending stands, `junctionWhen` (pure author content since
v44) holds, and — for `junctionAfter = Just n` — `clockReached n`. Outcomes are exactly
today's: the transition (scene eviction, `sceneEntered` stamp, destination setup) or
the ending insert. Memories: the same shape with the `memoryFired.<key>` latch, minus
the `storyAdvanced` pump. The `_narrator`, `narratorName`, the `junctionsP` practice,
and the entire `storyAdvanced` family DIE (closing v45's deferral). Beats are real cast
affordances and stay exactly where they are.

**2. The boundary learns to speak — the round's one new engine surface.** The
narrator's labels were the story's narration: a memory's text IS its label; junction
labels render transitions. Schedule rules are silent today, so the boundary gains a
narration channel: `ScheduleRule` gains a display label (a template, like
`actionName` — blank means silent, today's convention), and `roundBoundary` returns
the rendered labels of rules that FIRED (with their bindings) alongside the state; the
loop threads them into the same narration trace action labels flow through. Narration
is PRESENTATION, not world state — no fact is written, Persist is untouched, exactly
as action labels are not facts. This is deliberately general, not a Script special
case: any schedule rule may narrate (a market that announces its opening is now one
authored label away — the mechanism Script needs is the mechanism everyone gets).

**3. The sighting contract sheds `Now`, and `atSince` joins the reserved table**
(closing v45's other deferral). `sightRule`'s generated conjunct and stamps rename
`Now` → `PraxNow` (machinery, per v40); the sighting-template contract shrinks to
`Seer`/`Seen`/`Spot` — no shipped template uses `Now` (probed: village and bar
templates are two location `Match`es each), and a template wanting time-gated sighting
reads the clock with its OWN variable (`turn` reads are free, v45). With the stamp
machinery-shaped (`atSince.Seen!PraxNow`), `atSince` enters `reservedFamilies` as
`MachineryShapeOnly` — the v45 residue resolved by contract decision, not workaround.

**4. `sceneEntered` needs no migration**: already `MachineryShapeOnly` (v45); its
writers become the junction rules' outcomes, which are machinery-shaped and
engine-fired — strictly less exposed than today.

## What dies

`_narrator` + `narratorName`, the `junctionsP` practice, the `storyAdvanced.*` family,
every roster/setup entry for them. Script worlds' casts contain only characters again.

## Semantics shift, stated honestly

Junctions fired on the narrator's ROSTER TURN; they now fire at ROUND BOUNDARIES.
Within-round timing of transitions/endings shifts; the fiction (the story advances the
moment its condition holds, once per round) is unchanged. Script goldens re-capture
with itemized drift (narrator lines become boundary narration lines; timing shifts by
roster position only); decision content of real characters must be argued equivalent.

## Verification

- RED-first: a junction/memory fixture asserting schedule-fired behavior against the
  pre-round tree; the narration channel pinned (a labeled rule's text appears in the
  trace at the boundary; a blank-labeled rule is silent); the storyAdvanced family
  gone (grep-proof + the v45 reserved-table stopgap never needed).
- ScriptSpec's junction/memory/timed-junction pins re-expressed with fiction
  preserved (dawdling still dismisses; memories still fire exactly once).
- `atSince` reserved-table pins (authored read/write flag; the sighting rule's own
  shapes exempt — the all-shipped-worlds pin carries it).
- Goldens re-captured deliberately, itemized; time-free non-Script worlds
  byte-identical.

## Out of scope

v47 (function registry), v48 (generality bundle). Rule-narration TEMPLATES beyond the
`[Var]` rendering actions already use. The chronicler (banked — this round gives it
its natural future home, the boundary's narration channel, but builds none of it).
