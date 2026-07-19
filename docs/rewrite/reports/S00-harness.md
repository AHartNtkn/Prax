# S0 — harness                                        2026-07-19

CONTRACT     Freeze tagged (`haskell-freeze`) and mechanically enforced:
             tracked edits AND untracked additions both fail loudly
             (both failure modes observed and restored). prax.cabal diff
             vs the tag = exactly one additive executable stanza.
ORACLE       prax-oracle trace/randtrace/check/fixtures over the frozen
             library, ZERO library edits. Own correctness pin: trace
             reproduces the GoldenDriveSpec labels EXACTLY (village 21
             w/ --idle you, bar 12, intrigue 12). randtrace cross-checked
             against runRandom (3 seeds, turn counts + endings match).
             Deterministic byte-identical across reruns.
FIXTURES     conformance/fixtures/{db,el,query,derive}.json — computed by
             the frozen functions, not transcribed. NOTE: happy-path
             leverage, not spec coverage (recorded in conformance/README).
META-GATE    849 pin labels (712 cases + 137 groups, groups deliberate) in
             conformance/HASKELL_PINS.txt — survives the deletion.
WORKSPACE    rust/ cargo workspace, 7 crates, edition 2024; build + clippy
             -D warnings + test clean; no stub functions, no fake tests.
QUALITY      Review verdict: FIX WAVE → all closed: C1 two oracle describe
             strings aligned byte-identical to the frozen CLI; I1 CI added
             (.github/workflows/verify.yml: freeze-check + both suites);
             I2 freeze-check now catches untracked additions.
SUITES       Haskell 712/712 green with the oracle stanza; cargo test 0
             (skeletons only — honest).
FORKS        none.
