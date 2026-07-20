# S6 — Planner + Minds + Relevance + Sight (the summit)   2026-07-20

CONTRACT     PlannerSpec 22/22, MindsSpec, RelevanceSpec (13 synthetic + 5
             villageWorld rows ADDED owed:S7), SightSpec (1 owed:S9) — the
             meta-gate now polices all four. The 6 owed:S6 rows DISCHARGED.
             PlannerSpec's decimal pins re-expressed as ordering/choice with
             the bits delegated to the corpus — zero decimal KILLED rows.
THE CENTRAL  Bit-exactness against the frozen planner is OBSERVED, not
CLAIM        transcribed: planner.json + npc.json regenerate BYTE-IDENTICAL
             from the frozen engine; the comparison channel is raw u64
             (castDoubleToWord64 → to_bits) with no decimal round-trip
             anywhere; and the fold re-association `(base+acc)+0.9v` REDDENS
             THE CORPUS REPLAY in the canary world, 1 ULP apart. The canary
             recipe was CORRECTED by the implementer's own arithmetic search
             (the panel's 2^53 formulation was infeasible; sensitivity enters
             through the nested 0.9 — base 12 / acc 3.5 / v 0.9).
DIVERGENCE   DIV-2: the frozen engine accepts a separator-bearing character
             name and then reads it THREE ways across its own planner (death
             anchor splits; inScope re-splits through a string; scopeReads
             does not). The Rust rejects such names loudly — single path
             segments, the house precedent for every other engine-facing
             name — making the class unreachable rather than silently
             divergent. Found by PROBE, not inference; invisible to every net
             the stage had built.
QUALITY      Review APPROVE + fix wave: I-1 (death anchor tokenized — port
             self-consistency), I-2 (guarded by DIV-2), I-3 (the [S-C1]
             collision fixture was inert at the decision level; strengthened
             and re-verified by the controller: deleting the scope-read
             component now reddens the replay). Reuse gate netted by the
             cone-mediated AND eviction-shadow fixtures (one was caught inert
             mid-stage and fixed). reuse==live flagship with REUSE_HITS
             reach-proof. no-FMA OBSERVED on both toolchains.
PROCESS      Three implementers: the first handed off clean at context budget
             with the hard part (canary payoffs) settled and its own
             destructive-revert mistake reported; the second went idle
             unreported (review substituted); the fix wave reported.
SUITES       Rust 302 · Haskell 712 · freeze intact · verify.sh green.
FORKS        none. DIVERGENCES: DIV-2 (+ the recorded i32-utility bound).
