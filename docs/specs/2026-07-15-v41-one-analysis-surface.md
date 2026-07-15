# v41 — One analysis surface: the world-model analyses read the cooked form

Second of the user-queued foundations passes. The defect: the static analyses are split
across parallel walker families that must be kept mentally in sync — `wantPatterns`
(string, polarity+taint; feeds `improvableDesires`/`livenessOf`), `outcomeAtoms` (string,
Call-resolving; feeds `worldAtomPools`/`bearingTemplates`), `condPatterns` (string, in
Derive; feeds `axiomFootprint`/`axiomNegPatterns`), `mayUnify` (string) beside
`mayUnifySyms` (cooked), and `cookedReadAnchors` (cooked, v34). Each new analysis picks a
side; the sides have already drifted once in a way we had to carry in our heads
(`wantPatterns` does not extract subquery internals; `cookedReadAnchors` does — both
correct for their consumers, invisible from either alone).

## Design

**Cook first; analyze only the cooked form.** `retable` already cooks everything before
any analysis runs (`cookedDefs`, `cookedRules` — lifted forms included — and
`cookedDesires`); the analyses simply stop re-walking the string ASTs:

- `improvableDesires` and `livenessOf` consume `cookedDesires` + a cooked
  `worldAtomPools` (over `cookedDefs`'s `caOuts`/`cpInits`/`cpFns`); a cooked polarity
  walk (`cookedWantPatterns :: [CookedCondition] -> ([[Sym]], [[Sym]], Bool)` — positive
  anchors, negative anchors, taint) replaces `wantPatterns` with IDENTICAL classification
  semantics (the taint rules, the `Or`/`Absent`/`Exists` polarity flips, transcribed
  case-for-case and pinned).
- The cooked atom pools share their recursion skeleton with the one cooked outcome walker
  that already exists (`Prax.Engine`'s delta-anchor family) where that is genuinely one
  shape — one walker skeleton, two leaf policies (static all-cases pools vs grounded
  spawn-guarded deltas) — rather than a third copy; the plan decides the exact factoring
  against the sources, with no behavioral drift.
- `axiomFootprint`/`axiomNegPatterns`/`axiomHeadPatterns` read `cookedRules` (`crBody`
  via `cookedReadAnchors`-style walks, `crHeads` directly — the lifting comes free since
  cooked rules already include the lifted forms; the string-side lift duplication in the
  head enumeration is deleted with it).
- `mayUnify` (string) is DELETED; every consumer moves to `mayUnifySyms` over cooked
  anchors. The entity-names-vs-predicate-literals invariant documentation (currently in
  `mayUnify`'s orbit) moves to `mayUnifySyms`, its one home.
- `PraxState`'s derived analysis fields and their meanings are unchanged — this is a
  re-plumbing of how they are computed, not what they say.

**Explicitly staying string-side** (and said so in their haddocks): the v38/v40 boundary
walkers (`conditionVars`/`outcomeVars`, `authoredVarClash`/`authoredPatClash`) — they
guard AUTHOR-SUPPLIED fragments at combinator boundaries, before any cooking exists; and
all parsing/grounding machinery. The line the round draws: **authoring boundary = string;
world model = cooked.** One sentence, stated once, in the module docs of both sides.

## Exactness

The analyses' outputs must be classification-identical: `improvables`, `liveness`,
`footprint`, `negFootprint`, `axiomHeads`, `caresAbout` per world. Gates:

- BEFORE the switch, extend the per-world table pins to cover every analysis field for
  every shipped world (the liveness table pins exist; add improvables/footprint/
  caresAbout snapshots — exact expected values derived from the CURRENT code by
  observation). These pins are the equivalence net; they must pass unchanged across the
  switch. Any divergence = BLOCK and trace (either a transcription bug or a discovered
  drift between the old walkers — both must be understood, not absorbed).
- Goldens byte-identical; ViewInvariant green; the v34/v35 reuse and signature pins
  unchanged (they consume the same fields).
- The deletions are real: `mayUnify`, `wantPatterns`, `condPatterns`, `outcomeAtoms`
  gone from the tree; no wrapper, no re-export, no dual path.

## Out of scope

The dead-condition lint (v42 — it will be the first NEW analysis written against the
unified surface, which is the point of doing this round before it); the hygiene bundle
(v43); any change to cooking itself, query evaluation, or the cooked data types.
