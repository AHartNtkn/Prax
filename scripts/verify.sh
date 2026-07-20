#!/usr/bin/env bash
# verify.sh — the R-series standing verification entry point (docs/rewrite/PLAN.md).
#
# Runs, in order and failing loudly on the first problem:
#   1. freeze-check          — the frozen Haskell tree is byte-identical to the tag
#   2. cabal build prax-oracle — the one additive surface still compiles
#   3. golden-check          — the four decision-sequence goldens are still DERIVED
#                              from the frozen literals, hashed, and independently
#                              re-derived from the engine's own walk
#   4. cabal test            — the frozen Haskell suite stays green (the pins' source)
#   5. cargo build/clippy/test (workspace) — the Rust side compiles clean, no warnings
#
# Step 5 includes the comparator's end-to-end harness tests, which DRIVE THE
# FROZEN ORACLE as a subprocess — so this script (and CI) needs both toolchains
# in one place. That is why there is one verification entry point and not two.
#
# Every step is mandatory; there is no skip path. Run from anywhere in the repo.
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

step() { printf '\n=== %s ===\n' "$1"; }

step "1/5 freeze-check"
./scripts/freeze-check.sh

step "2/5 cabal build prax-oracle (additive oracle surface)"
cabal build prax-oracle

step "3/5 golden-check (the goldens are derived, never re-captured)"
./scripts/golden-check.sh

step "4/5 cabal test (frozen Haskell suite must stay green)"
cabal test

step "5/5 cargo build + clippy + test (Rust workspace)"
cargo build --manifest-path rust/Cargo.toml --workspace
cargo clippy --manifest-path rust/Cargo.toml --workspace -- -D warnings
cargo test --manifest-path rust/Cargo.toml --workspace

printf '\nverify: all checks passed.\n'
