# S1 — Sym + Db + EL                                   2026-07-20

CONTRACT     63 Haskell pins accounted 1:1 (SymSpec 7 = 6 re-expressed + 1
             killed haskell-only [the intern strictness race — the hazard
             cannot exist without the global pool]; DbSpec 38/38; ELSpec
             18/18 re-expressed). Meta-gate LIVE and mutation-proven (a
             removed // H: fails loudly). Corrected `!` insert, v39
             asserted-flag law, name-order determinism, ⊥ symmetry — each
             mutation-proven a real detector (6 semantic mutations flipped
             their pins).
DIFFERENTIAL fixture replay: db.json + el.json, every case asserted, byte-
             identical to frozen-oracle regeneration; no silent skips.
QUALITY      Review APPROVE (implementer died unreported — third occurrence;
             review substituted, all checks re-run independently). I1 closed:
             the structural-sharing net now guards clone-is-refcount-bump +
             untouched-subtree sharing (S6's planner depends on it). M1 doc
             aligned; M2 the 32-segment cap recorded as a stated bound; M3
             the fixture Set case now exercises real Val::Set rendering.
             clippy -D warnings clean · 0 unsafe · 67 Rust tests green.
SUITES       Haskell 712/712 · freeze byte-identical · verify.sh green.
FORKS        none. DIVERGENCES: none (the segment cap is a stated bound,
             loud and unreachable, not an observable divergence).
