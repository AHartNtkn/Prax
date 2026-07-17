# v46 — The narrator dies, and takes the narration with it

Second of the four audit-queued rounds (v45 protected families → **v46** → v47 function
registry → v48 generality bundle). Twice rewritten: once after the three-lens panel
(`.superpowers/sdd/v46-spec-review-*.md`) showed the first draft unsound, and once
after the user removed the constraint the whole premium hung on — the scene layer's
omniscient narration was never a required feature, and "the most principled option,
including not supporting any of this, is the right option."

## The problem

The scene layer (v12) needed story events — scene transitions, endings, one-shot
narration — to happen, and the only thing that happens in Prax is a character
performing an action. So the compiler invented `_narrator`: a hidden bodiless cast
member whose one desire is "advance the story," bribed by fabricated
`storyAdvanced.<key>` facts that every junction and memory inserts purely to raise its
utility. A fake person takes a real turn each round, driven through the full planner,
to execute what is actually scheduling. The audit rated it HIGH/category-2: v44's own
named defect, surviving.

## The principle that sizes the fix

Fiction surfaces through CHARACTERS' actions; the world's own dynamics (v44: the
schedule) fire silently — hunger does not announce itself. Under that principle:

- **Memories** ("(You recall the last envoy…)") are omniscient narration with no
  speaker — a presentation feature wearing world-content clothes. They are REMOVED as
  a feature: the construct, its compilation, its JSON field, the two shipped
  one-liners (Play, Audience), and their tests. The Prompter-parity charter yields to
  the engine's own model; recorded in the LEDGER as removed-by-design, not lost.
- **Junction labels** (`"(story) toBanquet"`) are log markers, not fiction. Gone with
  the actor; transitions fire silently like every other schedule rule.
- With no words to carry, there is NO narration channel, no clause labels, no
  boundary-signature change, no save-point move, and no `FirstMatch` mode — the whole
  apparatus of the previous draft existed to preserve narration parity, and the first
  panel's Criticals were all findings against that apparatus.

## The design (small)

**1. Junctions and endings compile to plain `AllClauses` period-1 schedule rules**
(one rule, `"story"`, clauses in authored order: scenes in declaration order, a
scene's junctions in declaration order). The firing law is the EXISTING machinery,
stated: each clause's own gates (`currentScene!sid`, `Absent ending`) self-mask —
same-scene doubles are masked by the transition's `currentScene` eviction, everything
is masked after an ending, and declaration order resolves simultaneous enables (the
old tiebreak was alphabetical-by-label, an ACCIDENT of the planner's sort; authored
order is a statement). Cross-scene cascade within one boundary is PERMITTED and
stated as eager semantics: a scene whose exit condition already holds on entry was
authored as pass-through — the executor threads state between clauses, so the gates
decide, not a mode. (No shipped script cascades; a pin documents the semantics.)

**2. One internal registration door.** The compiled story rule carries Prax-namespaced
machinery (the timed junction's clock arithmetic, the `sceneEntered` stamp), which
`setSchedule`'s authoring guard rightly rejects. `Script.compile` — compiler-level
code, squarely inside v45's threat model — registers its rule through an internal
`Prax.Engine` function carrying no authoring guard, documented as exactly that. The
sighting rule is Prax-var-free and REMAINS on `setSchedule` as today: no `withSighting`
setter, no door migration, no engine-vs-authored ordering law beyond what exists.
The rule-name table stays globally keyed; registration gains a duplicate-name guard
across both entry points (an authored rule named `story` in a script world errors at
build).

**3. Timed junctions keep their fiction** (Audience's `timeout "dismissed" 5`): the
`junctionAfter` expansion (`sceneEntered`/clock comparison) moves into the story
rule's clause conditions unchanged in meaning; dawdling still dismisses.

## What dies

`_narrator` + `narratorName`, the `junctionsP` practice, the `storyAdvanced.*` family
(closing v45's deferral), the memory construct end-to-end (AST, compile, JSON field,
shipped content, tests), the `"(story)"` label convention, every roster/setup entry
for the narrator. Script casts contain only characters again. Beats are real cast
affordances and are untouched.

## Also forced

- **`prax-state v3`**: a v45-era script save carries `storyAdvanced.*`/`memoryFired`
  facts and a `junctions`-practice instance but no `story` due — format-identical to
  v46 under v2 yet semantically dead. The header machinery exists for this.
- **StressSpec's play-world coverage is re-argued, not re-pinned**: the narrator
  leaves the random-mover pool and story firing becomes deterministic at boundaries;
  if scenes/endings coverage or zero-dead-ends fails under the new dynamics, that is
  a finding for adjudication, not a number to update.

## Semantics shifts, stated honestly

(a) Story events fire at round boundaries, not on a roster turn. (b) The
simultaneous-enable tiebreak becomes authored order (was alphabetical label). (c)
Cross-scene cascade in one boundary is now possible where conditions author it. (d)
Memories and story-marker lines vanish from traces. Script goldens and AnalysisTable
pins re-capture with per-class itemization; real characters' decision content argued
equivalent.

## Verification

- One-boundary story law pinned: same-scene co-enabled junctions → first in authored
  order fires, second masked; ending masks everything after; cascade pinned as the
  documented semantics (a pass-through scene traverses in one boundary).
- Timed junction fiction preserved (dismissal pin); memories grep-proof gone;
  storyAdvanced grep-proof gone; the duplicate-name guard pinned both directions.
- Persist v3 pins (v2 rejected); Stress re-argued; time-free non-Script worlds
  byte-identical; Script re-captures itemized.

## Out of scope

v47, v48. Any narration/presentation channel (removed from the design; if a future
need for authored narration arises it is a presentation-layer question, banked as
such). Mid-path reserved families (the atSince residue stands as annotated in v45).
