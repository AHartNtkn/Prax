#!/usr/bin/env bash
# freeze-check.sh — enforce the frozen reference surface (docs/rewrite/PLAN.md).
#
# The Haskell tree src/ app/ test/ is tagged `haskell-freeze` and must never be
# edited again until the cut-over deletion. This script is the mechanical
# enforcer: it exits nonzero with a loud message if the working tree (plus
# index) diverges from the tag under those directories. The differential
# comparator shells out to it and refuses to run when it fails.
#
# `examples` is here too, added at S8. examples/play.json is simultaneously the
# DECODE fixture and the ENCODE golden for the script codec — the frozen
# `dump-play` output is byte-identical to it, and so is the Rust one — so
# "adjust the file until the decoder passes" would corrupt both nets at once,
# and until S8 nothing stopped it. The file is byte-identical to the tag and it
# SURVIVES the cut-over, so after this script dies with the Haskell tree its
# successor net is the conformance pin that asserts the file equals the Rust
# encoder's own output for `play_script()`
# (`conformance::script_json_spec::the_shipped_file_is_the_encoders_own_output_byte_for_byte`).
# That successor is semantic rather than a hash: a hash manifest would say the
# bytes changed, this says the bytes stopped being the world's re-emission.
#
# Usage: scripts/freeze-check.sh
# Exit:  0 = frozen surface byte-identical to the tag; 1 = drift (or tag missing).
set -euo pipefail

TAG="haskell-freeze"
FROZEN=(src app test examples)

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

if ! git rev-parse --verify --quiet "refs/tags/$TAG" >/dev/null; then
  echo "FREEZE CHECK FAILED: tag '$TAG' does not exist." >&2
  echo "  The frozen baseline is missing; create it before any rewrite work:" >&2
  echo "    git tag $TAG" >&2
  exit 1
fi

# Two checks, both required:
# 1. Tracked content: `git diff <tag> -- <paths>` catches any edit to a file
#    the tag knows, staged or not.
# 2. Untracked additions: a NEW file under a frozen dir is invisible to the
#    diff above but just as much a freeze violation (the oracle could import
#    it and silently alter the reference), so it is checked separately.
if ! git diff --quiet "$TAG" -- "${FROZEN[@]}"; then
  echo "FREEZE CHECK FAILED: the frozen reference surface has been modified." >&2
  echo "  These paths must stay byte-identical to tag '$TAG':" >&2
  printf '    %s\n' "${FROZEN[@]}" >&2
  echo "  Offending files:" >&2
  git diff --name-only "$TAG" -- "${FROZEN[@]}" | sed 's/^/    /' >&2
  echo "  Revert them (git checkout $TAG -- <file>) — the oracle/ dir and the" >&2
  echo "  one prax-oracle cabal stanza are the ONLY permitted additive surface." >&2
  exit 1
fi

untracked=$(git ls-files --others --exclude-standard -- "${FROZEN[@]}")
if [ -n "$untracked" ]; then
  echo "FREEZE CHECK FAILED: new files added under the frozen reference surface." >&2
  echo "  The frozen dirs admit NO additions (a new module could be imported" >&2
  echo "  by the oracle and silently alter the reference):" >&2
  printf '    %s\n' $untracked >&2
  echo "  Remove them — new code belongs under oracle/, rust/, or scripts/." >&2
  exit 1
fi

echo "freeze OK: ${FROZEN[*]} byte-identical to tag '$TAG' (no additions)."
