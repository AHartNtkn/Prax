# v53 — Engine-rule provenance: the mixed-layer door closes, and scenePatience joins the reserved table

The v50 bank item opens: the `scenePatience` rejection lives only in
`Prax.Script.compile`, so a write injected through the RAW doors
(`definePractices`/`setSchedule`/`defineFunctions`) onto a compiled-Script state
silently corrupts a live timeout. v50 adjudicated the gap as structurally forced:
the family could not enter the v45 reserved table because the story rule — the
compiler's OWN machinery — lives in the same flat `schedule` list the reserved
scan polices, so reserving the family would trip the compiler's own `InsertFor`.

## The insight: the blocker is missing PROVENANCE, not the table's shape

The engine already has two doors with different contracts — `setSchedule`
(authored; v40-guarded) and `registerEngineRules` (the compiler door; unguarded
by design, v44) — but the distinction is spent at call time and never recorded:
both append to one list, and `writeSites` cannot tell machinery from content.
One recorded fact dissolves the whole v50 impasse: if the engine door REMEMBERS
what it installed, the reserved scan polices exactly the authored remainder.

## The design

- **`PraxState` gains `engineRules :: [String]`** — the names of schedule rules
  installed through `registerEngineRules` (name-keyed; rule names are already
  globally unique, enforced loudly by `addScheduleRules`). Written ONLY by the
  engine door: unforgeable at the authoring surface under the standing v45
  threat model (raw Haskell can call the compiler door directly; compiler-level
  construction has been outside the guards' charter since v45, reaffirmed at
  v50 — nothing new opens here). `emptyState` starts empty; Persist is
  untouched (schedule rules are code, re-supplied at build — their provenance
  re-establishes with them; verify at plan).
- **`writeSites` polices the AUTHORED schedule only**: its schedule leg drops
  rules whose name is in `engineRules st`. The reserved-family scan's charter
  was always "authored definitions" — the engine door exists precisely because
  engine rules are machinery; today's scan catches them only because nothing
  recorded the difference.
- **`scenePatience` joins `reservedFamilies`** (TypeCheck imports
  `Prax.Script.scenePatienceFamily` — the checker importing content vocabulary,
  the v51 `DeonticUnclosed` precedent). Same law as `turn`/`contradiction`:
  WRITES forbidden at every authored site (practices, functions, authored
  schedule rules, axiom heads; a delete is a write) — the corruption hazard the
  bank named. READS stay free at the raw layer, stated honestly with the `turn`
  precedent (a read couples but cannot corrupt; the read-scan machinery died
  with v50's zero-member law and one family does not resurrect it). The
  Script-layer guard is UNCHANGED and stricter (both polarities rejected in
  scripted content, with site-named errors at the surface that can see who's
  asking) — two layers, each policing the surface it owns, neither a dual
  system: they guard different doors.
- **The v50 bank closes SHIPPED**: mixed compiled-Script + raw composition is
  no longer unsupported-and-silent — the raw write that corrupted a timeout is
  now a loud `ReservedFamily` error, and the story rule no longer needs an
  exemption because provenance, not luck, keeps it out of the scan.

## What this deliberately does not do

No general "machinery family" registry (one recorded provenance fact + one
table row is the whole need; a registry is speculative generality). No
resurrection of read scanning. No blunt rejection of mixed composition — the
composition is now guarded where it was hazardous, which is strictly better
than forbidding it.

## Exactness

Behavior-identical everywhere (a checker-and-provenance round): no fiction
transcript, score, or analysis row moves; `typeCheck` output changes ONLY for
worlds that write `scenePatience` from raw authored surfaces (none shipped).
The scan-scope change is observable only as the ABSENCE of self-tripping when
the family is reserved — Audience (the story-rule world) staying
`typeCheck == []` is the standing pin.

## Verification (RED-first per behavior)

- A raw practice action inserting `scenePatience.x.y` flags `ReservedFamily`;
  same for a function case, an authored `setSchedule` rule body, an axiom
  head, an `InsertFor`, and a `Delete` (a delete is a write).
- The mixed-composition repro from the bank, pinned end-to-end: a practice
  added via `definePractices` ONTO COMPILED AUDIENCE writing a `scenePatience`
  path flags loudly (the exact door v50 left open).
- No self-trip: Audience and every shipped world stay `typeCheck == []` with
  the family reserved (the story rule's `InsertFor` is engine-door, exempt).
- Provenance mechanics: `registerEngineRules` records names; `setSchedule`
  does not; the scan polices exactly the difference (a same-shaped rule
  through each door — authored flags, engine does not).
- The v50-era Script-layer guard pins stay green (both polarities, unchanged).
- Deaths: none (this round only adds); death-grep n/a.
- Pre-gate: the three-lens panel runs on this document (engine-surface: a
  PraxState field + door semantics).

## Out of scope

Reserving any other family; read scanning; Persist changes (none needed —
verified at plan); the remaining banks (coercion set, part parks, Deontic
priorities) — untouched.
