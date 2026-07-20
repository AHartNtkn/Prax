# S5 — Loop + Schedule                                 2026-07-20

CONTRACT     LoopSpec 5 re-expressed + 8 owed (4→S6 planner, 4→S7 worlds —
             the implementer CORRECTED my brief's owed-stage with the
             later-of rule); ScheduleSpec 14/14; ScheduleRuleSpec 24 + 3
             owed (2→S9 verdicts, 1→S8). The v44 boundary end-to-end:
             clock → expiries → rules in declaration order, as-mutated
             (not snapshot), late-fire due<=now, re-arm-from-now; the
             ghost-observation law a deterministic pin; advance wrap
             i<=cursor incl. equality, dead-skip, reaper case.
ADJUDICATION Expiry FIRING order: rendered-name (frozen: intern-id) —
             posture note, NOT a DIV: due entries leave the queue BEFORE
             firing, so same-boundary interaction is structurally
             impossible; the reviewer PROVED commutation incl. the
             overlapping-ancestor case, and the mixed-lifetime proptest now
             pins facts, view, AND the surviving queue (review I1 closed).
QUALITY      Review APPROVE, no Critical. 4 mutation REDs (ghost ordering,
             wrap equality, re-arm, declaration order) + 3 reviewer
             mutations. I1/M1/M2 controller-closed at 237 green. Deviations
             adjudicated: turn.rs homing (the S0 stub's declared purpose),
             natural due seeding (no white-box backdoor), the later-of
             owed rule (now the standing convention).
SUITES       Rust 237 · Haskell 712 · freeze intact · verify.sh green.
FORKS        none. DIVERGENCES: the firing-order posture note (+ the
             keep-entry sentence).
