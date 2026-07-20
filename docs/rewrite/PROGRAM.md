# The Rust rewrite (R-series) — program of record

The approved program plan (verbatim from the gate) governs; this file is the
living status surface. The DESIGN is the contract — the five semantic
invariants, the specs, the LEDGER — not the Haskell. The Rust must be RIGHT:
divergences are adjudicated against the specs; Haskell bugs are never
reproduced (the user's ruling); genuinely ambiguous forks go to the user.

The Haskell tree (`src/ app/ test/` at tag `haskell-freeze`) is never edited
again. Permitted diff surface: `oracle/` + one additive stanza in
`prax.cabal`. Enforced mechanically by `scripts/freeze-check.sh` and the
comparator.

## Status

| Stage | Scope | State | Report |
|---|---|---|---|
| S0 | freeze tag, oracle exe, fixtures, workspace scaffold, verify script | DONE | [S00](reports/S00-harness.md) |
| S1 | Sym + Db + EL | DONE | [S01](reports/S01-sym-db-el.md) |
| S2 | Query (one compiled path) | IN PROGRESS | — |
| S3 | Derive + view (design-heavy) | — | — |
| S4 | Types + Engine + builder API (design-heavy) | — | — |
| S5 | Loop + Schedule + Rng | — | — |
| S6 | Planner + Minds + Relevance + Sight (design-heavy; fidelity summit) | — | — |
| S7 | Vertical world slices: Feud → Audience → Intrigue → Bar → Village | — | — |
| S8 | Script + Play | — | — |
| S9 | TypeCheck + AnalysisTable + Stress + Persist + Inspect + CLI | — | — |
| S10 | Hardening + cut-over | — | — |

## Registers

- `docs/rewrite/DIVERGENCES.md` — adjudicated Haskell bugs the Rust fixes
  (what, why the spec rules for Rust, fiction consequence, comparator
  suppression). Empty until one exists.
- `docs/rewrite/FORKS.md` — semantic-fork questions raised to the user.
  Empty until one exists.
- `conformance/KILLED.md` — Haskell pins not re-expressed, each with category
  (decimal | implementation | haskell-only) and reason; audited by the
  meta-gate test.
- Evidence reports: `docs/rewrite/reports/S<NN>-<name>.md`, one screen each.
