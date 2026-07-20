#!/usr/bin/env bash
# die-seed-sweep.sh — the S7 §4 / [D-I6] integration sweep, made repeatable.
#
# §4 mandates: "sweep village over >=20 die seeds x 20 walk seeds". That sweep
# was an operator action with no record in the repo, which is exactly what
# §13(2) legislates against — a reported quantity whose invocation is invisible
# cannot be reproduced. This script IS the invocation.
#
# The walk seed picks the randtrace walk; --die-seed reseeds the engine's own
# CRoll stream under that fixed walk, so the grid crosses draw OUTCOMES with
# walk shapes. Every cell must be `clean`; the first that is not stops the sweep
# loudly with its full invocation.
#
# Die seeds run 1..N: `Prax.Engine.seedDie`'s domain is [1, 2147483646] (0 and
# multiples of the modulus are Lehmer fixed points -- a die that always rolls the
# same face), and the frozen oracle refuses 0 loudly.
#
# Usage: scripts/die-seed-sweep.sh [WORLD] [DIE_SEEDS] [WALK_SEEDS] [CAP]
#   defaults: village 20 20 50   (400 cells)
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

world=${1:-village}
n_die=${2:-20}
n_walk=${3:-20}
cap=${4:-50}

oracle=(cargo run -q --manifest-path rust/Cargo.toml -p prax-oracle --)

printf 'die-seed sweep: world=%s die seeds=1..%d walk seeds=0..%d cap=%d (%d cells)\n' \
  "$world" "$n_die" "$((n_walk - 1))" "$cap" "$((n_die * n_walk))"

clean=0
for die in $(seq 1 "$n_die"); do
  for seed in $(seq 0 $((n_walk - 1))); do
    inv=(compare "$world" --mode randtrace --seed "$seed" --cap "$cap" --die-seed "$die")
    if out=$("${oracle[@]}" "${inv[@]}"); then
      case "$out" in
        *clean*) clean=$((clean + 1)) ;;
        *)
          printf 'DIE-SEED SWEEP FAILED (not clean)\n  invocation: prax-oracle %s\n  output: %s\n' \
            "${inv[*]}" "$out" >&2
          exit 1
          ;;
      esac
    else
      printf 'DIE-SEED SWEEP FAILED (nonzero exit)\n  invocation: prax-oracle %s\n' \
        "${inv[*]}" >&2
      exit 1
    fi
  done
done

printf 'die-seed sweep: %d/%d clean.\n' "$clean" "$((n_die * n_walk))"
