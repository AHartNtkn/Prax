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

## §2 — the perf table (Rust ≥ Haskell) — **VERDICT: RED (village fails; bar fails on the planner walk)**

Engine vs engine, both BUILT binaries. Rust `cargo build --release -p prax-oracle`
(in-process walk). Haskell `cabal build -O2 prax-oracle`, binary invoked DIRECTLY
via `cabal list-bin -O2 prax-oracle` (the `.../opt/build/...` path — NOT the
default `-O1` `list-bin`), so cabal's dependency-resolution latency is excluded.
Reproducible from the committed invocation `scripts/perf-table.sh`.

`ratio = haskell_wall / rust_wall`; `ratio ≥ 1.0` PASS (Rust ≥ Haskell), `< 1.0`
RED. Machine: XPG — 12th Gen Intel i7-1260P.

**[P6] the timed region is symmetric, stated explicitly per workload:**
- **randtrace band / trace**: BOTH engines emit per-record JSONL to a discarded
  sink (Rust `emit`, byte-identical to the frozen `randtrace`/`trace` output —
  proven against the captured baseline). Both INCLUDE per-record serialization.
- **stress**: BOTH engines run the `DIFF_RUNS` sweep in ONE process and emit a
  SINGLE terminal StressReport (Rust `emit --mode stress`, byte-identical to the
  frozen `stress` — proven). Both EXCLUDE per-record serialization; launch is
  amortized over the whole sweep.

Single-launch floor (no-op invocation, /100): rust ~1.2 ms, haskell(-O2) ~2.9 ms.
The RED rows are NOT a launch artifact — village's randtrace band is 3.5 s of
engine work, ~9% launch, and the ratio is 0.71.

### Table A — randtrace band 0..99, cap 50 (the DESIGN's PRIMARY workload, §2)

| world | haskell -O2 wall (s) | rust --release wall (s) | ratio (haskell/rust) | verdict |
|---|---|---|---|---|
| village | 3.52 | 4.96 | **0.710** | **RED** |
| bar | 1.86 | 1.23 | 1.512 | PASS |
| intrigue | 0.37 | 0.22 | 1.682 | PASS |
| feud | 0.40 | 0.18 | 2.222 | PASS |
| audience | 0.42 | 0.21 | 2.000 | PASS |
| play | 0.38 | 0.21 | 1.810 | PASS |

### Table B — stress (random hot path; DIFF_RUNS walks/process; single terminal emit; median of 5)

| world | haskell -O2 wall (s) | rust --release wall (s) | ratio (haskell/rust) | verdict |
|---|---|---|---|---|
| village | 0.561 | 1.420 | **0.395** | **RED** |
| bar | 0.167 | 0.158 | 1.052 | PASS |
| intrigue | 0.0145 | 0.0100 | 1.450 | PASS |
| feud | 0.0118 | 0.0090 | 1.311 | PASS |
| audience | 0.0172 | 0.0107 | 1.607 | PASS |
| play | 0.0124 | 0.0107 | 1.159 | PASS |

### Table C — trace (planner-heavy, depth-2; per-record emit; median of 5)

| world | turns | haskell -O2 wall (s) | rust --release wall (s) | ratio (haskell/rust) | verdict |
|---|---|---|---|---|---|
| village | 10 | 4.32 | 10.82 | **0.399** | **RED** |
| bar | 500 | 0.180 | 0.220 | **0.818** | **RED** |
| intrigue | 500 | 0.784 | 0.606 | 1.294 | PASS |
| feud | 500 | 0.459 | 0.246 | 1.866 | PASS |
| audience | 500 | 0.506 | 0.310 | 1.633 | PASS |
| play | 500 | 0.501 | 0.322 | 1.555 | PASS |

### Corroboration — randtrace LONG single walk (engine-isolated; launch <1%)

| world | records | cap | haskell -O2 wall (s) | rust --release wall (s) | ratio |
|---|---|---|---|---|---|
| village | 20002 | 20000 | 25.93 | 18.10 | 1.433 |
| bar | 26667 | 20000 | 4.34 | 3.09 | 1.405 |

**Reading of the RED.** The Rust engine is FASTER than Haskell on 5 of 6 worlds
on the design's primary randtrace sweep, and on a long single accumulated-state
walk it is faster on village too (1.43). But village FAILS on the cap-50 sweep
(0.71), on stress (0.40), and on the planner (0.40); bar FAILS on the planner
walk (0.82). The failures correlate with workloads dominated by REPEATED
world-construction / large-state cloning — the randtrace band rebuilds village
per seed, stress clones the initial village state per run, and the depth-2
planner clones states for lookahead — and village has by far the largest fact
set. HYPOTHESIS (observed correlation; NOT yet profiled): Rust `State::clone` /
world-build is O(state size) where the frozen Haskell shares structure, so
clone/build-heavy workloads on the largest world regress. Correctness is
UNAFFECTED — the 500-seed matrix above is byte-clean; Rust computes the identical
records, only slower on village.

**This RED is not dressed as green.** The perf cut-over criterion "for EVERY
shipped world, Rust ≥ Haskell" FAILS. Per §2, a `ratio < 1.0` row is a cut-over
blocker that must be resolved (Rust optimized) or recorded as a deviation the
user weighs at the demo — BEFORE the demo is offered. This is an adjudication
fork, not something the hardening pass may wave through.
</content>
