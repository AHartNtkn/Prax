# S10 — Hardening + cut-over (pre-demo): the reference is removable     2026-07-21

STATUS       HARDENING COMPLETE. Everything up to the ONE user gate (the demo)
             is done and green. The program now WAITS for the user: read this
             report, play village+bar on Rust `prax play` (with a save/resume),
             and say go — then the single irreversible deletion commit executes
             per the reviewed manifest (S10 design §5 + §8). Nothing irreversible
             has happened: no tag, no deletion, no promotion. All on master.
THE HEADLINE **Correctness at the moment of deletion is proven, and the net that
             SURVIVES the deletion is built and green.** The 500-seed × 6-world
             differential against the live frozen oracle is byte-CLEAN; the
             Rust-side corpus was captured under that certification; the
             `--baseline` comparator (Rust-now vs frozen-certified corpus) is
             built, wired into verify.sh, and 0-divergent over all six worlds —
             so after the frozen tree dies, the regression tripwire remains.
DIFFERENTIAL `matrix --worlds village,bar,intrigue,feud,audience,play
             --seeds 0..499 --cap 50` (state mode) vs the LIVE frozen oracle:
             3006 cells, **0 DIVERGENT, 0 SHAPE-DIVERGENT**, register empty
             (plain-clean). Per-world records compared: village 25525 · bar
             33250 · intrigue 4025 · feud 4025 · play 3857 · audience 4775.
             Distinct walks: bar 500 · village 500 · intrigue 4 · play 3 ·
             audience 2 · feud 1. A deep-walk sweep (village 100k / bar 133k
             records, cap 2000) is also 0-divergent.
PERF FIX     A perf RED surfaced at 500-seed scale (village slower than frozen
             on every workload; bar on the planner walk) and was FIXED, not
             waved through. Profiled first: `Db::clone` is O(1) (Arc bump),
             `State::clone` is O(schedule+expiries) — NOT the fact set; the real
             cost was the derivation closure re-querying village's
             `Subquery+Count` aggregate axioms against the whole model per round.
             Four semantics-preserving fixes (a semi-naive per-rule delta gate +
             allocation-free `entailed`/`meet_head` + `unify` substrate).
             **Isolated review APPROVE**, with the DIV-1 risk DISPROVEN three
             ways: the gate keys on the COMPLETE read footprint (`read_anchors`
             recurses into Subquery/Exists/Absent/Or), so an aggregate's
             count-changing fact is always an anchor and the rule is never
             skipped — a sound restriction of the naive least fixpoint, not the
             frozen bug; DELETIONS are never gated (they route to full naive
             `reclose`; only strictly-monotone inserts hit the gated
             `close_from`; expiry firing goes the full path); and a gate
             always-miss mutation reddens `naive_equals_production`, ViewInvariant,
             and the DIV-1 fragment net. Re-verified by a full independent
             500-seed frozen re-run (0 divergent) + the [P2] baseline re-cert.
PERF TABLE   Rust ≥ Haskell (ratio = haskell/rust; ≥1.0 = Rust wins), [P6]-
             symmetric timed region (both engines built binaries, single-emit).
             **Comfortable Rust wins on five of six worlds and on village's hot
             path**, with ONE honest judgment item for the demo:

             | workload | village | bar | intrigue | feud | audience | play |
             |---|---|---|---|---|---|---|
             | randtrace | **1.81** | 1.83 | 1.68 | 2.22 | 2.00 | 1.81 |
             | stress    | **~1.0 ⚠** | 1.29 | 1.45 | 1.31 | 1.61 | 1.16 |
             | trace     | **~1.0 ⚠** | 1.10 | 1.29 | 1.87 | 1.63 | 1.56 |

             ⚠ **THE ONE JUDGMENT ITEM (yours, at the demo):** village `stress`
             and village `trace` sit at MARGINAL PARITY (~1.0). This is not a
             correctness issue — the differential is byte-clean; Rust computes
             identical records, at ~parity throughput on village's two heaviest
             non-hot-path walks. Two honest qualifiers: (a) all perf numbers were
             taken while the machine was under sustained external load (~6–9,
             other projects), which the plan's methodology warns against — a
             quiescent re-measure would refine these, but two independent careful
             (K=7, interleaved) measurements both landed village-stress at
             ~1.00 (range 0.92–1.12), so parity is the robust finding, not an
             artifact; (b) village-`trace` at the design's 4000-turn workload is
             SUPERLINEARLY infeasible on BOTH engines (≈17s at 20 turns), so its
             ratio is a reduced-turn per-world self-comparison — the
             superlinearity is shared with Haskell, not Rust-specific. The
             residual on village-stress is Haskell's laziness on action labels
             the walk renders but never reads, plus `unify`'s constant factor.
             **The plan makes you the perf arbiter at the demo; this is the call
             it means.**
SURVIVING NET [P2] `compare/matrix --baseline <dir>` added ADDITIVELY (frozen
             path untouched): diffs the live engine against the committed
             `conformance/oracle-baselines/` corpus (no cabal). 0-divergent over
             all six worlds at 500 seeds — which independently RE-CERTIFIES the
             perf fix (corpus captured pre-fix, current Rust byte-identical). A
             surviving frozen-free `#[test]` (mutation-verified: a corrupted
             baseline record → DIVERGENT) and a verify.sh step keep the tripwire
             armed after the frozen dies.
GATES        `cargo test --workspace` **980 passed / 0 failed** · clippy
             `--all-targets -D warnings` clean · resident zero-unsafe sweep (102
             `.rs` files, mutation-verified) · proptest SOAK @ `PROPTEST_CASES=
             100000` green across every ARCHITECTURE law (trie/EL/query/expiry/
             persist + ViewInvariant `view==naive` + reuse==live +
             naive==production) · [P7] byte-exact `stress`/`flow` stdout goldens
             (mutation-reddened) · `type_check == []` resident for all six
             shipped worlds (`every_shipped_world_is_well_formed`) · freeze
             (src app test examples) byte-identical to `haskell-freeze`.
DELETION     PREPARED, NOT EXECUTED. The manifest (S10 design §5 + §8 [P1]–[P7])
             is the reviewed artifact: exactly what dies (src/ app/ test/ oracle/
             prax.cabal cabal.project, freeze-check.sh, drive_frozen.rs, the
             Haskell CI half), what survives (docs/ examples/play.json the
             retargeted scripts, the whole rust/ workspace promoted to root), and
             the per-file promotion path-fixes incl. the [P1] pop-2→1 correction
             for the meta_gate/adjudicated_register helpers. It is ONE commit
             after `git tag haskell-final`, executed only on your "go", and
             re-reviewed at that point. `git diff haskell-final..HEAD` will be
             the whole reviewable change; the frozen tree stays recoverable at
             the tag forever.
YOUR MOVE    1. (optional) skim this + the S10 design manifest. 2. `prax play
             village`, take some turns, `s` to save, quit, `prax play village
             resume`, continue; then the same for `bar`. 3. Say go — or flag the
             village-stress parity as a blocker, in which case it's a perf
             investigation, not a cut-over. Nothing deletes until you do.
FORKS        none blocking. The village-stress/trace parity is a JUDGMENT item
             for the demo, not a fork. DIVERGENCES: DIV-1/2/4 unchanged.
             CARRIED GAPS: NONE.
