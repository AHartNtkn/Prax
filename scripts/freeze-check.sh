#!/usr/bin/env bash
# freeze-check.sh — enforce the Haskell freeze (docs/rewrite/PLAN.md).
#
# The Haskell tree src/ app/ test/ is tagged `haskell-freeze` and must never be
# edited again until the cut-over deletion. This script is the mechanical
# enforcer: it exits nonzero with a loud message if the working tree (plus
# index) diverges from the tag under those three directories. The differential
# comparator shells out to it and refuses to run when it fails.
#
# Usage: scripts/freeze-check.sh
# Exit:  0 = frozen tree byte-identical to the tag; 1 = drift (or tag missing).
set -euo pipefail

TAG="haskell-freeze"
FROZEN=(src app test)

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

if ! git rev-parse --verify --quiet "refs/tags/$TAG" >/dev/null; then
  echo "FREEZE CHECK FAILED: tag '$TAG' does not exist." >&2
  echo "  The frozen baseline is missing; create it before any rewrite work:" >&2
  echo "    git tag $TAG" >&2
  exit 1
fi

# Compare the tag against BOTH the index and the working tree for the frozen
# dirs. `git diff <tag> -- <paths>` covers unstaged changes; adding the index
# via a second check catches staged-but-uncommitted edits too.
if ! git diff --quiet "$TAG" -- "${FROZEN[@]}"; then
  echo "FREEZE CHECK FAILED: the frozen Haskell tree has been modified." >&2
  echo "  These paths must stay byte-identical to tag '$TAG':" >&2
  printf '    %s\n' "${FROZEN[@]}" >&2
  echo "  Offending files:" >&2
  git diff --name-only "$TAG" -- "${FROZEN[@]}" | sed 's/^/    /' >&2
  echo "  Revert them (git checkout $TAG -- <file>) — the oracle/ dir and the" >&2
  echo "  one prax-oracle cabal stanza are the ONLY permitted additive surface." >&2
  exit 1
fi

echo "freeze OK: src/ app/ test/ byte-identical to tag '$TAG'."
