# S10 hardening — RAW CAPTURED MACHINE OUTPUT

This file holds the verbatim machine output the S10 evidence report embeds. It is
NOT prose: every block below is captured stdout, reproduced byte-for-byte from the
run that produced it. Do not hand-edit the numbers — the matrix block carries the
`provenance` guard (`matrix.rs:provenance_violations`) that forbids a retyped
figure, and this file is where the controller reads the captured evidence from.

Machine: XPG — 12th Gen Intel(R) Core(TM) i7-1260P — 16 hardware threads (nproc=16).

---

## §1 — the full 500-seed hardening matrix (frozen-vs-Rust differential)

Run while the frozen Haskell oracle still lives; its Rust side captured as the
[P4] baseline (`conformance/oracle-baselines/`, 3006 files = 6 worlds × (1 trace +
500 randtrace)). Six shipped worlds, 500 seeds each, cap 50, state mode, one
process, `--jobs 16`. Wall clock 04:10:36 → 04:13:11 UTC (~2m35s; warm
freeze-rev-keyed oracle cache from prior stages). Exit 0.

**VERDICT: CLEAN.** Every world: `DIVERGENT = 0` and `SHAPE-DIVERGENT = 0`; every
one of the 3006 cells `clean` (register empty, so plain-clean, no
clean-mod-adjudicated). 3006/3006 cells agreed with the frozen record-for-record.

```
invocation (chosen, verbatim): prax-oracle matrix --worlds village,bar,intrigue,feud,audience,play --seeds 0..499 --cap 50 --jobs 16 --format report --capture-baseline conformance/oracle-baselines

| world | randtrace seeds (measured) | cells (measured) | clean (measured) | clean-mod-adjudicated (measured) | DIVERGENT (measured) | SHAPE-DIVERGENT (measured) | records compared (measured) | distinct walks (measured) | budget stop |
|---|---|---|---|---|---|---|---|---|---|
| audience | 500 | 501 | 501 | 0 | 0 | 0 | 4775 | 2 | --seeds as requested; no record floor was asked for |
| bar | 500 | 501 | 501 | 0 | 0 | 0 | 33250 | 500 | --seeds as requested; no record floor was asked for |
| feud | 500 | 501 | 501 | 0 | 0 | 0 | 4025 | 1 | --seeds as requested; no record floor was asked for |
| intrigue | 500 | 501 | 501 | 0 | 0 | 0 | 4025 | 4 | --seeds as requested; no record floor was asked for |
| play | 500 | 501 | 501 | 0 | 0 | 0 | 3857 | 3 | --seeds as requested; no record floor was asked for |
| village | 500 | 501 | 501 | 0 | 0 | 0 | 25525 | 500 | --seeds as requested; no record floor was asked for |
```

---

## §2 — the perf table (Rust ≥ Haskell)

_(captured in the next step; see below)_
</content>
