# Conformance — the semantic contract's bookkeeping

- `HASKELL_PINS.txt` — the meta-gate's ground truth: every `testCase` AND
  `testGroup` label from the frozen `test/Prax/*.hs` (849 total; groups are
  included DELIBERATELY — a group label names a contract area and must be
  accounted for like a case: re-expressed under some `// H:` comment or
  killed by name in `KILLED.md`). Extracted by `scripts/extract-pins.py`;
  survives the Haskell deletion as the committed manifest.
- `fixtures/*.json` — unit corpora computed by the frozen implementation
  (`prax-oracle fixtures <name>`). They are DIFFERENTIAL LEVERAGE for the
  pre-world stages, NOT spec-coverage: the db corpus in particular is a
  happy-path sample — fixture-green never substitutes for re-expressing the
  spec file's pins (the meta-gate enforces that separately).
- `KILLED.md` — pins not re-expressed, each with category
  (decimal | implementation | haskell-only) and a one-line reason.
