#!/usr/bin/env bash
# perf-table.sh — the S10 §2 / [P6] engine-vs-engine perf table, reproducible.
#
# Rust ≥ Haskell is a cut-over criterion, so it is measured like one: engine
# against engine, both as BUILT binaries, cabal's dependency-resolution latency
# excluded (the Haskell binary is invoked DIRECTLY via `cabal list-bin`, never
# `cabal run`), warm-up discarded, median of K samples, machine named in the
# report.
#
# [P6] THE TIMED REGION IS SYMMETRIC. Two workloads, each internally symmetric:
#
#   stress  — the random hot path (Stress.runRandom == drive_rust::rand_walk),
#             DIFF_RUNS walks in ONE process, a SINGLE terminal emit on BOTH
#             engines (both EXCLUDE per-record serialization). Rust `emit --mode
#             stress` is byte-identical to the frozen `stress` (the S6 stress
#             differential proved it), so the two do identical work. Process
#             launch is amortized over the whole sweep. Covers all six worlds.
#
#   randtrace (long) — one long walk to a large cap, PER-RECORD JSONL emit on
#             BOTH engines (both INCLUDE serialization), launch amortized over
#             thousands of records. Only the two sandbox worlds (village, bar)
#             sustain a long walk; the four narrative worlds reach an ending in
#             ~8 records, so this is the engine-isolated corroboration for the
#             two worlds where launch is <1% of the number.
#
# The single-launch floor (a no-op invocation) is reported too, so a reader can
# see how much of a fast world's number is runtime launch rather than engine.
#
# ratio = haskell_wall / rust_wall; ratio ≥ 1.0 is a PASS, ratio < 1.0 is a RED
# cut-over blocker.
#
# Usage: scripts/perf-table.sh          (K=7 samples, 1 warm-up discarded)
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

RB="rust/target/release/prax-oracle"
HB=$(cabal list-bin prax-oracle)
WORLDS=(village bar intrigue feud audience play)
K=${K:-7}

[ -x "$RB" ] || { echo "build first: cargo build --release -p prax-oracle" >&2; exit 1; }
[ -x "$HB" ] || { echo "build first: cabal build -O2 prax-oracle" >&2; exit 1; }

# The frozen trace idler per world (oracle worldNamed / worlds::idler); intrigue
# is planner-driven (no idler) so its trace is a real planner walk.
idler() { case "$1" in
  bar) echo "--idle you";; village) echo "--idle you";; feud) echo "--idle alice";;
  play) echo "--idle marcus";; audience) echo "--idle envoy";; *) echo "";; esac; }

# median of K timed samples of `REP` back-to-back runs of the given command,
# with WARM warm-up rounds discarded. Prints seconds (float).
median_time() {
  local rep="$1"; shift
  local i j s e
  for j in $(seq 1 "$rep"); do "$@" >/dev/null 2>&1; done   # 1 warm-up round
  local samples=()
  for i in $(seq 1 "$K"); do
    s=$(date +%s.%N)
    for j in $(seq 1 "$rep"); do "$@" >/dev/null 2>&1; done
    e=$(date +%s.%N)
    samples+=("$(awk -v s="$s" -v e="$e" -v r="$rep" 'BEGIN{printf "%.6f",(e-s)/r}')")
  done
  printf '%s\n' "${samples[@]}" | sort -g | awk '{a[NR]=$0} END{print a[int((NR+1)/2)]}'
}

# choose REP so each sample is ~0.5s (lifts a 10ms world above timer noise)
reps_for() {
  local t; t=$(median_time 1 "$@" 2>/dev/null)
  awk -v t="$t" 'BEGIN{r=int(0.5/t); if(r<1)r=1; print r}'
}

ratio() { awk -v h="$1" -v r="$2" 'BEGIN{printf "%.3f", h/r}'; }

echo "# S10 perf table — machine: $(hostname), $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | sed 's/^ *//')"
echo "# K=$K timed samples, 1 warm-up discarded; ratio = haskell/rust; built binaries; cabal list-bin direct."
echo "# rust binary: $RB"
echo "# haskell binary: $HB"
echo

echo "## single-launch floor (no-op invocation, 100 reps)"
rf=$(median_time 100 "$RB"); hf=$(median_time 100 "$HB")
printf '  rust launch: %.6f s   haskell launch: %.6f s\n\n' "$rf" "$hf"

echo "## Table A — stress (random hot path; DIFF_RUNS walks/process; SINGLE terminal emit, both engines; [P6] both-exclude symmetric)"
echo "| world | haskell wall (median s) | rust wall (median s) | ratio (haskell/rust) | verdict |"
echo "|---|---|---|---|---|"
for w in "${WORLDS[@]}"; do
  rep=$(reps_for "$RB" emit "$w" --mode stress)
  h=$(median_time "$rep" "$HB" stress "$w")
  r=$(median_time "$rep" "$RB" emit "$w" --mode stress)
  ra=$(ratio "$h" "$r")
  v=$(awk -v ra="$ra" 'BEGIN{print (ra>=1.0)?"PASS":"RED"}')
  printf '| %s | %s | %s | %s | %s |\n' "$w" "$h" "$r" "$ra" "$v"
done
echo

echo "## Table B — randtrace long walk (engine-isolated; PER-RECORD JSONL emit, both engines; [P6] both-include symmetric; launch <1%)"
echo "| world | records | cap | haskell wall (median s) | rust wall (median s) | ratio (haskell/rust) | verdict |"
echo "|---|---|---|---|---|---|---|"
for pair in "village:2000" "bar:20000"; do
  w=${pair%%:*}; cap=${pair##*:}
  recs=$($RB emit "$w" --mode randtrace --seed 0 --cap "$cap" 2>/dev/null | wc -l)
  h=$(median_time 1 "$HB" randtrace "$w" --seed 0 --cap "$cap" --mode state --candidates)
  r=$(median_time 1 "$RB" emit "$w" --mode randtrace --seed 0 --cap "$cap")
  ra=$(ratio "$h" "$r")
  v=$(awk -v ra="$ra" 'BEGIN{print (ra>=1.0)?"PASS":"RED"}')
  printf '| %s | %s | %s | %s | %s | %s | %s |\n' "$w" "$recs" "$cap" "$h" "$r" "$ra" "$v"
done
echo

echo "## Table C — trace planner walk (planner-heavy; PER-RECORD JSONL emit, both engines; [P6] both-include symmetric)"
echo "| world | records | turns | haskell wall (median s) | rust wall (median s) | ratio (haskell/rust) | verdict |"
echo "|---|---|---|---|---|---|---|"
for w in "${WORLDS[@]}"; do
  il=$(idler "$w"); turns=4000
  recs=$($RB emit "$w" --mode trace --turns "$turns" 2>/dev/null | wc -l)
  rep=$(reps_for "$RB" emit "$w" --mode trace --turns "$turns")
  # shellcheck disable=SC2086
  h=$(median_time "$rep" "$HB" trace "$w" --turns "$turns" $il --depth 2 --mode state --candidates)
  r=$(median_time "$rep" "$RB" emit "$w" --mode trace --turns "$turns")
  ra=$(ratio "$h" "$r")
  v=$(awk -v ra="$ra" 'BEGIN{print (ra>=1.0)?"PASS":"RED"}')
  printf '| %s | %s | %s | %s | %s | %s | %s |\n' "$w" "$recs" "$turns" "$h" "$r" "$ra" "$v"
done
