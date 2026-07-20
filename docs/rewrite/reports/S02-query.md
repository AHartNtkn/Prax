# S2 — Query (one compiled path)                       2026-07-20

CONTRACT     QuerySpec 30/30 re-expressed on the ONE evaluator; CookedSpec's
             4 duality pins killed (implementation — no second evaluator to
             agree with) with every underlying scenario re-expressed as a
             direct pin, ghci-confirmed against the frozen library (3 re-run
             independently by the reviewer, incl. mod sign-of-divisor on
             negative operands). Meta-gate green over the grown allowlist.
DIFFERENTIAL fixture replay query.json 28/28 byte-for-byte; regenerated
             fixture byte-identical to committed.
QUALITY      Review APPROVE, no Critical/Important. 4 mutation REDs
             (Or order, Calc overflow loudness, Not guard, Eq val semantics).
             3 proptest laws incl. the interner-independence heir of
             queryCooked==query. Deviations, both adjudicated strictly
             conservative: nested-subquery guard promoted to construction
             time; &mut Interner threading (no observable leak). Minors
             closed: read_integer's grammar boundary stated exactly; the
             groundCookedOutcome S4-deferral made LOUD in KILLED.md.
SUITES       prax-core 91 + conformance 7; Haskell 712/712; freeze intact;
             verify.sh exit 0.
FORKS        none. DIVERGENCES: none observable.
