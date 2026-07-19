# v53 — Engine-rule provenance: the mixed-layer door closes, and the compiler's families join the reserved table

AMENDED after the three-lens pre-gate panel (`.superpowers/sdd/v53-spec-review-*`):
the scope grew on the design lens's charge — `currentScene` and `ending` sit in
STRUCTURALLY the same position as `scenePatience` (compiler-emitted,
literal-tailed, single-legitimate-writer, corruptible through the same raw doors),
and my "out of scope: any other family" was means-testing by which bank item
happened to be filed [D-Q4]. All three families reserve; the blast-radius
directive (v44's "do not limit blast radius — segregating what can use the same
mechanism is asking for cleanup later") decides the fork the panel left open.
Remaining findings folded with [S]/[D]/[C] citations.

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

- **`PraxState` gains `engineRuleNames :: [String]`** [D-Q5: it holds names,
  so the name says so] — the schedule rules installed through
  `registerEngineRules` (name-keyed, kept as a list in the state's own list
  style; dup-free and unambiguous BECAUSE `addScheduleRules` enforces rule-name
  uniqueness ACROSS BOTH DOORS — that invariant is the exemption's entire
  safety condition and gets its own pin [S-I1]: a `setSchedule` rule named
  "story" onto compiled Audience is a loud duplicate-name error, never a
  silent exemption). Written ONLY by the engine door — `registerEngineRules`
  stops being a bare alias of `addScheduleRules` and records what it installs
  [S, C-M]. Unforgeable at the authoring surface: provenance is stamped by the
  DOOR, not carried on the rule, precisely because `ScheduleRule` values cross
  the authoring boundary and a rule-borne field could be forged [D-Q1]. Raw
  calls to the compiler door itself remain outside the guards' charter (v45,
  reaffirmed v50). `emptyState` lists it explicitly (the one full-record
  construction site [C]); Persist untouched — schedule rules are code,
  re-supplied at build, and provenance re-establishes with them
  (`deserializeState` keeps the base state's field [C-verified]).
- **`writeSites` polices the AUTHORED schedule only** [S-I2: the exemption
  lives HERE, a whole-rule drop — `writeSites` has exactly one consumer, the
  reserved scan, while `seedlessDrawErrors` and the dead-condition lint scan
  `schedule st` directly and correctly still see engine rules]: its schedule
  leg drops rules whose name is in `engineRuleNames st`. The exemption is
  WHOLESALE — engine-door rules leave the reserved scan for every family,
  `turn` included: the scan's charter is authored definitions ("no legitimate
  AUTHORED writer"), and the engine door is the sanctioned mechanism writer by
  the v44/v45 threat model [D-Q2]. The scan's axiom leg stays UNFILTERED —
  there is no engine door for axioms, and that asymmetry is intentional, not
  an incomplete scoping [C-I5].
- **THREE families join `reservedFamilies`** [D-Q4]: `scenePatience`,
  `currentScene`, and `ending` — each compiler-emitted, literal-tailed, with
  exactly one legitimate writer (the story rule / compile's setup, both
  engine-side: transitions and endings ride the engine-door rule, and the
  start scene's inserts are PERFORMED into the db at compile, which is state,
  not a definition surface [S — the highest-risk trace: no second self-trip
  door exists]). Same law as `turn`/`contradiction`: WRITES forbidden at every
  authored site (practices, functions, authored schedule rules, axiom heads;
  a delete is a write). READS stay free at the raw layer (the `turn`
  precedent: reads couple but cannot corrupt, and gating a beat on the current
  scene or a reached ending is legitimate authoring; the read-scan machinery
  died with v50's zero-member law and is not resurrected). TypeCheck imports
  the family names from `Prax.Script` — which must EXPORT them
  ([C-C1]: `scenePatienceFamily` is defined but unexported today, and
  `currentScene`/`ending` get named constants as their one home). Plan-time
  verification: no shipped authored definition writes any of the three — if
  one does, that is a bug this round FIXES, not accommodates.
- **The Script-layer guard is UNCHANGED and stricter** (both polarities
  rejected in scripted content, site-named errors at the surface that can see
  who's asking) — two layers, each policing the surface it owns, neither a
  dual system: they guard different doors [D-Q3 affirmed].
- **The v50 bank closes SHIPPED — scoped honestly** [D-Q5]: the filed item
  (the scenePatience door) closes, AND the structurally identical doors the
  panel found (`currentScene`/`ending`) close with it, so the headline does
  not silently leave the class open while claiming the instance.

## What this deliberately does not do

No general "machinery family" registry (one recorded provenance fact + one
table row is the whole need; a registry is speculative generality). No
resurrection of read scanning. No blunt rejection of mixed composition — the
composition is now guarded where it was hazardous, which is strictly better
than forbidding it.

## Exactness

Behavior-identical everywhere (a checker-and-provenance round): no fiction
transcript, score, or analysis row moves; `typeCheck` output changes ONLY for
worlds that write a reserved compiler family from raw authored surfaces (none
shipped — plan-verified). The scan-scope change is observable only as the
ABSENCE of self-tripping — BOTH compiled story-rule worlds, Audience AND Play
[C-I4], staying `typeCheck == []` are the standing pins. Three TypeCheck doc
surfaces are rewritten with the code they describe [C-I1..I3]: the
`ReservedFamily` constructor haddock (its v40-namespace grounding is FALSE for
literal-tailed families — provenance, not namespace, protects them), the
`reservedFamilies` comment block (which today argues scenePatience's EXCLUSION
— exactly the reasoning this round reverses), and the module haddock's check
enumeration (also stale on count since v51).

## Verification (RED-first per behavior)

- A raw practice action inserting `scenePatience.x.y` flags `ReservedFamily`;
  same for a function case, an authored `setSchedule` rule body, an axiom
  head, an `InsertFor`, and a `Delete` (a delete is a write); `currentScene`
  and `ending` writes each pinned at a representative site.
- The mixed-composition repro from the bank, pinned end-to-end: a practice
  added via `definePractices` ONTO COMPILED AUDIENCE writing a `scenePatience`
  path flags loudly (the exact door v50 left open).
- No self-trip: Audience AND Play and every shipped world stay
  `typeCheck == []` with the families reserved (the story rule is engine-door,
  exempt; compile's performed setup is db state, not a definition).
- Provenance mechanics (home: ScheduleRuleSpec [C-M]): `registerEngineRules`
  records names; `setSchedule` does not; the scan polices exactly the
  difference (a same-shaped rule through each door — authored flags, engine
  does not); a `setSchedule` rule named "story" onto compiled Audience is a
  LOUD duplicate-name error [S-I1 — the exemption's safety condition, pinned].
- The v50-era Script-layer guard pins stay green (both polarities, unchanged,
  disjoint from the new TypeCheck pins [C-verified]).
- Deaths: none (this round only adds); death-grep n/a.
- Pre-gate: the three-lens panel ran; verdicts SOUND / PRINCIPLED / GAPS,
  folded above; the amended spec is what gates.

## Out of scope

Read scanning (the raw layer keeps the turn precedent). Persist changes (none
needed — verified). Reserving non-compiler families or building a
machinery-family registry (three table rows + one provenance fact is the whole
need; a registry is speculative generality [D-Q4's mechanism verdict]). The
remaining banks (coercion set, part parks, Deontic priorities) — untouched.
