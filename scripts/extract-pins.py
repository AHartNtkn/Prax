#!/usr/bin/env python3
"""extract-pins.py — the meta-gate's ground truth.

Extract every `testCase "..."` and `testGroup "..."` label from the frozen
Haskell test tree (test/**/*.hs) into a sorted `<file>\t<label>` manifest. The
meta-gate (a later Rust conformance test) asserts each label appears in exactly
one `// H:` comment or one KILLED.md row; this manifest is what survives the
eventual deletion of the Haskell tree, so it must be reproducible from the tag.

The label is the FIRST Haskell string literal following the `testCase`/
`testGroup` token, parsed with proper handling of `\\"` and `\\\\` escapes so a
quote inside a label never truncates it.

Usage:  scripts/extract-pins.py [TEST_DIR] [OUT_FILE]
        (defaults: test  conformance/HASKELL_PINS.txt)
Prints the total label count to stderr.
"""
import os
import sys

TOKENS = ("testCase", "testGroup")


def read_string_literal(s, i):
    """Parse a Haskell string literal starting at s[i] == '"'.

    Returns (value, next_index) or (None, i) if s[i] is not a quote.
    Handles \\" and \\\\ escapes; the returned value is the literal's raw
    inner text (escapes left as written, which is how the labels are matched).
    """
    if i >= len(s) or s[i] != '"':
        return None, i
    j = i + 1
    out = []
    while j < len(s):
        c = s[j]
        if c == "\\":
            nxt = s[j + 1] if j + 1 < len(s) else ""
            if nxt == "" or nxt in " \t\r\n":
                # A Haskell string gap: backslash, whitespace (possibly spanning
                # lines), closing backslash — denotes nothing. Collapse it so the
                # label is a single line, exactly as the compiler reads it.
                k = j + 1
                while k < len(s) and s[k] in " \t\r\n":
                    k += 1
                if k < len(s) and s[k] == "\\":
                    k += 1  # consume the closing backslash of the gap
                j = k
                continue
            # An ordinary escape: copy the pair verbatim (\" stays \").
            out.append(s[j : j + 2])
            j += 2
            continue
        if c == '"':
            return "".join(out), j + 1
        out.append(c)
        j += 1
    # Unterminated literal (should never happen in valid source).
    return "".join(out), j


def labels_in(text):
    """Yield every (token, label) whose string literal follows a TOKENS word."""
    n = len(text)
    for tok in TOKENS:
        start = 0
        while True:
            k = text.find(tok, start)
            if k == -1:
                break
            start = k + len(tok)
            # Require a word boundary before the token (not part of a longer id).
            if k > 0 and (text[k - 1].isalnum() or text[k - 1] in "_'"):
                continue
            # Skip whitespace to the next non-space char.
            j = start
            while j < n and text[j] in " \t\r\n":
                j += 1
            if j < n and text[j] == '"':
                val, _ = read_string_literal(text, j)
                if val is not None:
                    yield tok, val


def main():
    test_dir = sys.argv[1] if len(sys.argv) > 1 else "test"
    out_file = sys.argv[2] if len(sys.argv) > 2 else "conformance/HASKELL_PINS.txt"

    rows = set()
    for root, _dirs, files in os.walk(test_dir):
        for fn in sorted(files):
            if not fn.endswith(".hs"):
                continue
            path = os.path.join(root, fn)
            rel = os.path.relpath(path)
            with open(path, encoding="utf-8") as fh:
                text = fh.read()
            for _tok, label in labels_in(text):
                rows.add((rel, label))

    ordered = sorted(rows)
    os.makedirs(os.path.dirname(out_file) or ".", exist_ok=True)
    with open(out_file, "w", encoding="utf-8") as out:
        for rel, label in ordered:
            out.write(rel + "\t" + label + "\n")

    print("wrote %d pin labels to %s" % (len(ordered), out_file), file=sys.stderr)


if __name__ == "__main__":
    main()
