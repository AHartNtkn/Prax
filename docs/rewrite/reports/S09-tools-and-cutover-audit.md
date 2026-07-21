# S9 — TypeCheck + AnalysisTable + Persist + Stress + Inspect + CLI     2026-07-21

CONTRACT     92 new `// H:` labels (TypeCheckSpec 56 · AnalysisTableSpec 8 ·
             PersistSpec 22 · StressSpec 6), each accounted exactly once; 15
             `owed:S9` KILLED rows discharged and REMOVED. **owed:S9 == 0, and
             nothing is owed to any later stage** — the whole ledger owes
             nothing entering the cut-over. Meta-gate green (exactly-once holds
             after 15 removals + 4 allowlist additions).
THE HEADLINE **The last stage before deletion, and the tooling layer is whole:**
             the static checker (`type_check`, 9 constructors / 11 passes in
             frozen order), the serializer (`prax-rs-state v1`, six-field
             round-trip), Stress, Inspect, and a real `prax-cli` all land, and
             the program-wide CUT-OVER AUDIT below is finished — every net that
             only compares to the frozen tree now has a native successor or a
             written contract, so S10 can delete the reference without a silent
             hole. Built across a two-implementer session-limit handoff;
             incremental commits meant nothing was lost.
PANEL        SOUND + COMPLETE-WITH-GAPS, 0 critical. §11 [P1]–[P9] fold the
             corrections in: [P1] worldshape localizes `axiom_heads` ONLY —
             the native AnalysisTableSpec literals are the SOLE durable net for
             improvables/liveness/caresAbout/footprint/negFootprint (5 fields,
             not 2); [P2] the meta-gate gap was never "commit the manifest"
             (already committed, read-as-committed) but the missing `read_pins`
             anti-vacuity FLOOR — now a ≥849/non-empty guard; [P6] the check-
             order pin honours the three-way UnboundVar sub-order (11 passes,
             not 9); [P8] `run_random` factored out behavior-preserving.
RULINGS      [R1] `producible_atoms` verbatim from the frozen: insert-half +
             SCHEDULE-rule inserts (the term movers-only `world_atom_pools`
             lacks) + db facts + axiom heads + turn + contradiction; None-is-
             wild silences the dead lint. The `&State` signature kept — the
             checks clone the interner locally to mint eviction shadows, so
             they stay side-effect-free (no `&mut State`). [R6] the persist
             re-intern hazard [S6 S-I1] discharges HERE end-to-end: a reloaded
             intention is REUSED by `npc_act` (basis==sig AND still_offered,
             after a fresh interner), not re-deliberated.
DIFFERENTIAL oracle `check` GREEN (intrigue/bar/feud/play) · oracle `stress`
             GREEN (intrigue/bar/play) · 100-seed randtrace clean post-factor
             AND post-relocation (6189 records, 0 divergent) · CLI stdout
             BYTE-IDENTICAL to frozen `prax` for check/flow/dump-play/stress ·
             `dump-play == examples/play.json`. The `check` describe golden is
             byte-exact and serves BOTH the oracle Value-vs-Value channel and
             the CLI check golden (the two frozen `describe`s are identical,
             [S-OK11]).
REVIEW       APPROVE-WITH-MINORS (isolated, mutation-injection, baseline
             reproduced). The one high-suspicion check — the implementer
             RETARGETED a meta-gate guard so its own `owed==0` terminal state
             passes — cleared: the guard moved from "≥1 owed row" (which would
             false-fire at the legitimate empty-ledger success state) to "KILLED
             table substantive", and STILL reddens on an emptied table. A
             principled retarget, not a neutered net. Every net stress-tested
             bit: unaccounted label → red; manifest floor → red; producible
             schedule-drop → red (false DeadCondition on play_world); persist
             re-intern mangle → red at still_offered; [P7] raw-string drop →
             red. No wrong semantic, no weakened net, no unaccounted label.

## §8 PROGRAM-WIDE CUT-OVER AUDIT — every S1–S9 net that compares to the frozen tree

The stage's major deliverable. A net that only asserts equal-to-frozen
EVAPORATES at the S10 deletion; each below has a native successor or a written
S10 contract. (Conformance tests that read COMMITTED baselines —
`fixtures/*.json`, `goldens/*`, `cg1_supersession.json` — are Rust-vs-committed,
SURVIVE untouched, and are correctly out of scope.)

| net | compares to frozen | survives? | successor / contract |
|---|---|---|---|
| `prax-oracle compare`/`matrix` (worlds × seeds) | frozen JSONL, record-by-record | no | **[P4] S10:** capture the clean 500-seed run's Rust-side JSONL as a committed baseline under `conformance/oracle-baselines/`; DELETE `drive_frozen.rs` + the `freeze_check`/`freeze_rev` gates; add `compare --baseline` (Rust-now vs Rust-committed) |
| `die-seed-sweep.sh` | frozen oracle via cabal | no | **[P3]** folds into the row above — retargets to `--baseline` |
| `worldshape --check` (all worlds) | frozen worldshape JSON | no | **[P1]** localizes `axiom_heads` + shape/bodies ONLY; the 5 analysis-table fields are covered by the native AnalysisTableSpec literals (their SOLE net); worldshape stays a slice-time localizer |
| oracle `check` differential | frozen `typeCheck` describe array | no | 9 SHOULD-flag native fixtures (one per constructor) + the `type_check==[]` standing net over the shipped worlds + the byte-exact describe golden |
| oracle `stress` differential | frozen `StressReport` | no | the StressReport native pins + the committed CLI stress format golden |
| `golden-check.sh` (village/bar/intrigue/loop-bar) | frozen spec literals | no | `conformance/goldens/SHA256SUMS` (S7 [D-C3]); check retargets to the hashes at deletion |
| `examples/play.json` (dump-play) | frozen encoder + the file | partly | committed SHA256 (S8 [R7]); the FILE survives, the pin is file-driven (encoder == its own re-emission) |
| meta-gate manifest (849 labels) | labels from the frozen `test/` tree | no | **[P2]** `HASKELL_PINS.txt` is ALREADY committed & read-as-committed; the S9 deliverable was the `read_pins` anti-vacuity FLOOR (≥849, non-empty) + an emptied-manifest pin — DONE |
| CLI stdout equality (check/stress/flow/dump-play) | frozen `prax` stdout | no | **[P5]** committed exact-byte stdout goldens (durable); the structural oracle differential is PRIMARY pre-deletion |
| `freeze-check.sh` | `git diff haskell-freeze -- src app test examples` | n/a — subject deleted | dies at cut-over; removed in the deletion commit |

Nothing in S1–S9 silently compares to a deleted thing and escapes this table
(reviewer swept `conformance/src/*.rs`, `scripts/*.sh`, `oracle/`
independently). The two rows the design flagged most-at-risk — the meta-gate
manifest and the prax-oracle retarget — are closed: the first by the [P2]
floor (built), the second by the [P4] contract (written into PROGRAM.md's S10
row, built at the deletion commit that makes it possible).

SUITES       Rust: prax-core lib 247 · prax-worlds 44 · conformance lib 290 ·
             prax-oracle non-cabal 59 · workspace exit 0 · clippy
             `--all-targets -D warnings` clean · no `unsafe` in the S9 diff ·
             freeze (src app test examples) byte-identical.
MINORS       A best-effort `stdout().flush()` and a defensive `unwrap_or_default`
             on an unreachable path (both on the interactive `play` surface
             S10 finalizes); check/stress differentials cover 4/3 shipped
             worlds, with village/audience `type_check==[]` held by the durable
             native pin over all 7. `renderScene` is demo-verified-not-pinned
             ([R11]) — no frozen label exists for the interactive loop, so
             nothing is dropped from the meta-gate.
FORKS        none. DIVERGENCES: unchanged (DIV-1/2/4). CARRIED GAPS: NONE.
