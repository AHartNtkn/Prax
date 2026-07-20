#!/usr/bin/env bash
# golden-check.sh — the goldens are DERIVED from a tree that cannot be edited.
#
# The four decision-sequence goldens (S7 design §6, [D-C3]) are the planner's
# contract: any change that perturbs a single decision must fail. The standing
# hazard is not that they break — it is that someone "adjusts the golden to
# match the new behaviour". So the goldens in conformance/goldens/ are not a
# second copy to be maintained; they are EXTRACTED from the frozen Haskell spec
# files, and this check asserts byte-identity. The frozen tree is enforced
# immutable by scripts/freeze-check.sh, so "edit the golden" would mean editing
# the freeze, which fails loudly one step earlier.
#
# Three assertions, in order:
#
#   1. EXTRACTION — each golden file is byte-identical to the literal in its
#      frozen spec file. A zero-line extraction fails loud (an empty extraction
#      would make this comparison vacuous — [D-C3(c)], the meta-gate's
#      stage_states_or_die idiom).
#   2. HASHES — conformance/goldens/SHA256SUMS matches. This is the check's
#      DESIGNED SUCCESSOR: the cut-over DELETES the frozen tree, and a guarantee
#      whose expiry is not designed is not a guarantee. The hashes are committed
#      WHILE THE FREEZE LIVES, so after deletion the check retargets to them
#      with the extraction step dropped and nothing else changed.
#   3. CROSS-DERIVATION — the same sequences are re-derived from
#      `prax-oracle trace`, which walks the engine rather than reading the spec
#      file. Two independent derivations of one file: the spec's literal and the
#      engine's own behaviour. (Note the shape difference the design names: the
#      GoldenDrive sequences are `"<actor>: <label>"` with `"-"` for an idle
#      turn, while the LoopSpec narration is the bare label with idle turns
#      OMITTED — two nets, not one.)
#
# The Rust golden TESTS that LOAD these files arrive with their slices; what
# ships now is the file, the hash, the extraction, and the cross-derivation.
#
# Usage: scripts/golden-check.sh [--update]
#   --update  re-extract the goldens and rewrite SHA256SUMS (the ONLY way they
#             ever change: by the frozen tree changing, which cannot happen).
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

GOLDENS_DIR=conformance/goldens
ORACLE=${PRAX_ORACLE_CMD:-"cabal run -v0 prax-oracle --"}
UPDATE=0
if [ "${1:-}" = "--update" ]; then UPDATE=1; fi

# name | spec file | Haskell binding | oracle trace args | shape
# shape=drive : "<actor>: <action>" for every turn (GoldenDriveSpec.driveLabels)
# shape=narr  : the bare label, idle turns omitted (Loop.runNpcTicks)
GOLDENS=(
  "village-21|test/Prax/GoldenDriveSpec.hs|villageGolden|trace village --turns 21 --idle you --mode decisions|drive"
  "bar-12|test/Prax/GoldenDriveSpec.hs|barGolden|trace bar --turns 12 --mode decisions|drive"
  "intrigue-12|test/Prax/GoldenDriveSpec.hs|intrigueGolden|trace intrigue --turns 12 --mode decisions|drive"
  "loop-bar-25|test/Prax/LoopSpec.hs|expectedTrace|trace bar --turns 25 --mode decisions|narr"
)

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fail() { echo "GOLDEN CHECK FAILED: $*" >&2; exit 1; }

mkdir -p "$GOLDENS_DIR"

# ---- 1. extraction ---------------------------------------------------------
for row in "${GOLDENS[@]}"; do
  IFS='|' read -r name spec binding _args _shape <<< "$row"
  python3 scripts/extract-golden.py "$spec" "$binding" > "$tmp/$name.txt"
  n=$(wc -l < "$tmp/$name.txt")
  if [ "$n" -eq 0 ]; then
    fail "extracted ZERO lines for $binding from $spec (the extractor should have died first)"
  fi
  if [ "$UPDATE" -eq 1 ]; then
    cp "$tmp/$name.txt" "$GOLDENS_DIR/$name.txt"
    continue
  fi
  if [ ! -f "$GOLDENS_DIR/$name.txt" ]; then
    fail "$GOLDENS_DIR/$name.txt is missing. Regenerate with scripts/golden-check.sh --update."
  fi
  if ! diff -u "$GOLDENS_DIR/$name.txt" "$tmp/$name.txt" > "$tmp/$name.diff"; then
    echo "--- $name: the committed golden and the frozen literal disagree ---" >&2
    cat "$tmp/$name.diff" >&2
    fail "$name drifted from $spec:$binding. The goldens are DERIVED — never edit \
$GOLDENS_DIR/$name.txt to match new behaviour; a difference here means either the frozen \
tree was edited (freeze-check would have caught it) or the golden was hand-adjusted."
  fi
  echo "  extraction OK: $name ($n lines, from $spec:$binding)"
done

# ---- 2. hashes -------------------------------------------------------------
if [ "$UPDATE" -eq 1 ]; then
  ( cd "$GOLDENS_DIR" && sha256sum ./*.txt > SHA256SUMS )
  echo "golden-check: goldens and SHA256SUMS rewritten from the frozen tree."
  exit 0
fi
if [ ! -f "$GOLDENS_DIR/SHA256SUMS" ]; then
  fail "$GOLDENS_DIR/SHA256SUMS is missing — it is the check's designed successor for after \
the frozen tree is deleted, and must be committed WHILE THE FREEZE LIVES."
fi
( cd "$GOLDENS_DIR" && sha256sum --quiet --check SHA256SUMS ) \
  || fail "the committed hashes do not match the golden files."
echo "  hashes OK: $GOLDENS_DIR/SHA256SUMS"

# ---- 3. cross-derivation from the engine -----------------------------------
for row in "${GOLDENS[@]}"; do
  IFS='|' read -r name _spec _binding args shape <<< "$row"
  # shellcheck disable=SC2086
  $ORACLE $args > "$tmp/$name.jsonl"
  python3 - "$tmp/$name.jsonl" "$shape" > "$tmp/$name.derived" <<'PY'
import json, sys
path, shape = sys.argv[1], sys.argv[2]
with open(path, encoding="utf-8") as fh:
    for line in fh:
        r = json.loads(line)
        if "end" in r or "actor" not in r:
            continue
        if shape == "drive":
            print(f"{r['actor']}: {r['action']}")
        elif not r["idle"]:
            print(r["action"])
PY
  if ! diff -u "$GOLDENS_DIR/$name.txt" "$tmp/$name.derived" > "$tmp/$name.xdiff"; then
    echo "--- $name: the engine's own walk disagrees with the golden ---" >&2
    cat "$tmp/$name.xdiff" >&2
    fail "$name failed its SECOND, independent derivation. The spec literal and the engine \
walk are supposed to be two derivations of one file; if they disagree, one of them is not \
what it claims to be."
  fi
  echo "  cross-derivation OK: $name (via prax-oracle $args)"
done

echo "golden-check: 4 goldens, extracted + hashed + cross-derived."
