#!/usr/bin/env python3
"""Extract one golden decision sequence from a FROZEN Haskell spec file.

The goldens are never re-captured (S7 design §6): they are EXTRACTED from the
frozen tree, which cannot be edited, so "adjust the golden to match the new
behaviour" is not a reachable move.  This script is the extractor half; the
byte-identity assertion and the second, independent derivation from
`prax-oracle trace` live in scripts/golden-check.sh.

Usage:  extract-golden.py <spec-file> <binding-name>
Prints the list's string elements, one per line.

FAILS LOUD on a zero-element extraction [D-C3(c)]: a silently empty extraction
would make the byte-identity check compare nothing against nothing and report
green, which is the exact failure the check exists to prevent.
"""

import re
import sys

STRING = re.compile(r'"((?:[^"\\]|\\.)*)"')


def extract(path, name):
    """The string elements of `name = [ "…", "…" ]` in `path`, in order."""
    with open(path, encoding="utf-8") as fh:
        lines = fh.read().splitlines()

    start = None
    for i, line in enumerate(lines):
        # The binding's equation, not its type signature: `name =` or `name =`
        # followed by the opening bracket on the next line.
        if re.match(rf"^{re.escape(name)}\s*=\s*(\[)?\s*$", line):
            start = i
            break
    if start is None:
        sys.exit(
            f"extract-golden: no binding `{name} =` in {path}. The golden's source "
            f"moved or was renamed — fix the extractor, never the golden."
        )

    out = []
    depth = 0
    seen_open = False
    for line in lines[start:]:
        depth += line.count("[") - line.count("]")
        seen_open = seen_open or "[" in line
        out.extend(STRING.findall(line))
        if seen_open and depth == 0:
            break
    else:
        sys.exit(f"extract-golden: the list `{name}` in {path} is never closed.")

    if not out:
        sys.exit(
            f"extract-golden: extracted ZERO lines for `{name}` from {path}. "
            f"An empty extraction would make the byte-identity check pass "
            f"vacuously — refusing."
        )
    return out


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    for line in extract(sys.argv[1], sys.argv[2]):
        # Haskell string escapes that appear in these sequences: only `\"`.
        print(line.replace('\\"', '"'))


if __name__ == "__main__":
    main()
