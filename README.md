# prax

A Haskell reimplementation of **Versu** (Richard Evans & Emily Short's simulationist
storytelling system), building up from **Praxish** (Max Kreminski's JavaScript reconstruction
of Versu's *Praxis* language).

The engine models a social world as a set of sentences in **exclusion logic**, with **social
practices** that offer role-based affordances and **autonomous agents** that choose actions by
utility. See the design writeups:

- `docs/research/versu-notes.md` — Versu architecture, from the 2014 IEEE paper.
- `docs/research/praxis-praxish-notes.md` — exclusion logic + the Praxish reference impl.
- `docs/LEDGER.md` — the full feature ledger (what's built, planned, and research-blocked).

## Status

v1 (faithful engine core) in progress. Done so far:

- `Prax.Db` — the exclusion-logic trie: `insert` (with the **corrected `!` semantics**, fixing
  a data-loss bug in Praxish's `db.js`), `retract`, `unify`, `ground`.

## Build & test

Requires GHC 9.x + Cabal.

```sh
cabal build       # compile the library
cabal test        # run the test suite (tasty)
```

## References

Primary source material (the Versu paper, the Praxish checkout, etc.) is downloaded into
`references/`, which is git-ignored — kept locally, never distributed.
