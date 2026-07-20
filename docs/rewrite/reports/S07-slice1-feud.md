# S7 slice 1 — feud (the first world crosses)          2026-07-20

CONTRACT     FeudSpec/KinSpec/FactionSpec: 42 labels, 42 `// H:`, 0 KILLED.
             prax-vocab opens with faction + kin (signature-for-signature;
             each module's own path helper preserved). Both worlds ported —
             feud AND bigfeud ([D-I8]).
DIFFERENTIAL worldshape GREEN both worlds; `compare feud --mode trace
             --turns 24 --emit state` clean; the matrix clean on both worlds
             (raised past the 100-seed floor to the MEASURED 3,000-effective-
             record floor per [I2] — matrix now reports effective records so
             the floor is checked, not assumed).
THE HARNESS  The slice's most important verification, and it FAILED FIRST:
BITES        the reviewer injected a real closure bug (fixpoint capped at one
             round, dropping resents.carol.alice / resents.dave.alice). The
             harness caught it but classified TURN and pointed at the turn
             loop. Two independent defects: the view-reclassification rule
             lived INSIDE the STATE rung (7th) while the hazard it corrects
             lands on rungs 2-4, and its lookback was measured in ordinals
             while idle passes desynchronise ordinals from turns. Both fixed;
             CONTROLLER-RE-VERIFIED with an independent injection: now
             STATE(view), traced to the record where the views first differed,
             naming both missing facts. This matters beyond feud — slices 2-4
             carry the same bug SHAPE (defeater-name string surgery).
QUALITY      Review FIX WAVE (2 Critical, 4 Important), all closed: [C1] above;
             [C2] faction_standing silently accepted a trailing-operator
             pattern the frozen combinator rejects loudly (the guard lives one
             call deeper than the slice-0 grep covered — not reachable today,
             reachable at slice 4); [I1] the worldshape bodies diff now
             localizes to the differing node; [I2] the seed floor corrected to
             measured reality; [I3] compare reports what it COMPARED, not what
             was requested; [I4] the eagerness deviation stated and pinned.
SUITES       Rust 418 · Haskell 712 · freeze intact · verify.sh green.
FORKS        none. DIVERGENCES: none new.
