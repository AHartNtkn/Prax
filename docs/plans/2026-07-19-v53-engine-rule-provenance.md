# Plan v53 — engine-rule provenance

Governing spec: `docs/specs/2026-07-19-v53-engine-rule-provenance.md`
(panel-amended, then probe-corrected: TWO families reserve; `ending` excluded on
the Intrigue evidence). One code task + docs folded into it (the round is small;
the doc surfaces are three haddock blocks in the same file the code changes).
RED-first per behavior; every shipped world's `typeCheck == []` pins are the
no-regression net.

## T1 — Provenance, the scan scope, and the two new rows

**Why each piece exists:** the corruption door is real (a raw write to a
compiler family passes every guard today); the reserved table couldn't hold the
families only because nothing recorded which schedule rules are machinery; one
door-stamped name list closes that; and `ending` stays out because Intrigue's
authored endings prove it is shared vocabulary, not machinery.

- `Prax.Types`: `PraxState` gains `engineRuleNames :: [String]` — haddock: the
  schedule rules installed through the compiler door
  (`Prax.Engine.registerEngineRules`); the reserved-family scan exempts them
  (machinery may write reserved families — v45's charter); dup-free because
  rule names are globally unique across both doors. `emptyState` lists
  `engineRuleNames = []` explicitly [S].
- `Prax.Engine`:

  ```haskell
  registerEngineRules :: [ScheduleRule] -> PraxState -> PraxState
  registerEngineRules rules st =
    (addScheduleRules rules st)
      { engineRuleNames = engineRuleNames st ++ map srName rules }
  ```

  (no longer a bare alias [S, C-M]; `addScheduleRules` unchanged — its
  cross-door duplicate-name guard is the exemption's safety condition and is
  pinned, not modified). `setSchedule` unchanged.
- `Prax.Script`: named constants become the one home and are EXPORTED
  [C-C1]: `scenePatienceFamily` (exists, unexported — joins the export list)
  and new `currentScenePath :: String` (`"currentScene"`), with the module's
  literals rewired through it (`currentSceneOf`'s unify pattern :239,
  `storyClause`'s Match/Insert :346/:351, the beats gate :362, compile's
  setup insert). The Script-layer guard is untouched.
- `Prax.TypeCheck`:

  ```haskell
  import           Prax.Script (scenePatienceFamily, currentScenePath)

  reservedFamilies :: [String]
  reservedFamilies =
    [ turnPath, "contradiction", scenePatienceFamily, currentScenePath ]
  ```

  `writeSites`'s schedule leg becomes
  `[ ("schedule " ++ srName r, outs)
   | r <- schedule st, srName r `notElem` engineRuleNames st
   , (_, outs) <- srBody r ]` — the exemption lives HERE, whole-rule [S-I2];
  `seedlessDrawErrors` and the dead-condition lint scan `schedule st` directly
  and deliberately still see engine rules; the axiom leg stays unfiltered
  (no engine door for axioms — intentional [C-I5]).
  THREE doc surfaces rewritten with the code [C-I1..I3]: the `ReservedFamily`
  constructor haddock (v40-namespace grounding is false for literal-tailed
  families — provenance protects them), the `reservedFamilies` comment block
  (today it argues scenePatience's EXCLUSION — replaced by the provenance
  story and `ending`'s evidence-based exclusion), and the module haddock's
  check enumeration (also fixing the count stale since v51).

**Tests (RED observed per behavior; reserved pins in TypeCheckSpec, provenance
pins in ScheduleRuleSpec [C-M]):**
- ReservedFamily pins, one per authored site: a practice action's `Insert
  "scenePatience.x.y"`; a practice init's `InsertFor`; a function case's
  write; an authored `setSchedule` rule body's write; an axiom head; a
  `Delete` (a delete is a write); a `currentScene!x` write at one
  representative site. RED: wire the two rows in AFTER writing the pins
  (observe absent-flag), or neuter the rows — either way observed.
- The mixed-composition repro, end-to-end [the bank's exact door]: a practice
  added via `definePractices` onto COMPILED AUDIENCE writing
  `scenePatience.<sid>.<j>` flags loudly. RED: observed before the rows land.
- No self-trip: Audience AND Play [C-I4] and every shipped world stay
  `typeCheck == []` (the existing all-worlds pin is the net; observed with
  the rows live).
- Intrigue stays `typeCheck == []` — the `ending`-exclusion's standing pin
  (it raw-authors `ending!` at Intrigue.hs:71/:83/:93).
- Provenance mechanics (ScheduleRuleSpec): the same-shaped rule through each
  door — authored flags, engine-door does not (RED: neuter the writeSites
  filter and observe the engine-door rule flagging); `registerEngineRules`
  records names, `setSchedule` does not; a `setSchedule` rule named "story"
  onto compiled Audience is a LOUD duplicate-name error [S-I1].
- The v50 Script-layer guard pins: untouched, re-observed green.

Suite green at end (baseline 688; report the delta). `-Wall` clean. Commit
"v53 T1: " then a second commit "Docs: v53 — " for the LEDGER row (the v50
bank closes SHIPPED — scoped honestly: the filed door plus the structurally
identical `currentScene` door close; `ending` excluded on evidence; record the
panel's Q4 catch — the first draft's scope was means-tested — and the probe's
`ending` correction) plus any README/WALKTHROUGH mention of the mixed-layer
limitation (grep; the v50 row's bank paragraph stays as record).

## Exactness ledger

Nothing moves: no fiction transcript, score, analysis row, or existing pin.
The suite grows by the new pins only. Anything else = BLOCK and trace.
