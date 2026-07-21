# S10 design — HARDENING + CUT-OVER: the last stage, the one user gate, and the single irreversible commit (panel input; agent-side)

Frozen reference: none new — S10 writes almost no engine code. Its subjects are the whole assembled tree: the six shipped worlds (`prax-worlds/src/{village,bar,intrigue,feud,audience,play}.rs`), the comparator (`prax-oracle/src/{main,matrix,drive_frozen,drive_rust,compare,classify,stress,worldshape,worlds}.rs`), the conformance suite and its data dir (`rust/conformance/src/*.rs` + the repo-root `conformance/` data), the four scripts (`verify.sh`, `golden-check.sh`, `die-seed-sweep.sh`, `freeze-check.sh`), the CI workflow, and the Haskell tree that dies (`src/ app/ test/ oracle/ prax.cabal cabal.project`). Builds on S0–S9, which hand off an owed-ledger of ZERO (PROGRAM.md S9 row) and a finished program-wide cut-over audit (S09 report §8) that S10 EXECUTES. NOT in scope: any new semantics, any world change, any pin re-expression — those are all done. S10 is measurement, then one deletion.

**House convention.** This is agent-side input to the two-lens panel, S08/S09 form. A binding **§8 PANEL RULINGS** addendum is appended after the panel runs and GOVERNS on every point it touches; the `[R*]` rulings below stand until then. The §11-rulings convention (soundness + design/completeness, corrections folded as `[P*]`) applies verbatim.

**The stakes, stated once.** Every prior stage was reversible: a wrong pin reddens, a wrong port is caught by the differential, nothing is lost because the frozen tree still answers. S10 ends that. The deletion commit removes the ONLY independent oracle the program has — after it, "is the Rust right?" can never again be asked of the Haskell. So the entire burden of S10 is to make the state at the deletion commit one where the question has already been answered exhaustively AND where nothing that will still need answering has been left pointing at a corpse. The manifest (§5) is the artifact the panel adversarially reviews BEFORE a byte is deleted.

## 1. The hardening matrix — the full differential, run while the frozen oracle still lives

**[R1] The matrix is the last and largest exercise of the frozen-vs-Rust differential, and it is captured on the way past.** The tool already exists and is correct (`matrix.rs`, S7); S10 runs it at the program's stated scale and captures its Rust side as the [P4] baseline.

**The run.** Six shipped worlds, 500 seeds each, cap 50, state mode, one process:

```
cargo run --release --manifest-path rust/Cargo.toml -p prax-oracle -- \
  matrix --worlds village,bar,intrigue,feud,audience,play \
         --seeds 0..499 --cap 50 --jobs $(nproc) --format report
```

`matrix.rs` hardwires `Mode::State` and `Emit::matrix()` per cell, so "state mode" is structural, not a flag. Each world contributes one `trace` cell (its idler-driven decision walk) plus 500 `randtrace` cells; 3006 cells total, each a full record-by-record state comparison behind a green `worldshape` gate that is checked ONCE per world up front (`matrix.rs:260`, the [I4] hoist). The `--format report` block is embedded VERBATIM in the S10 evidence report — its `provenance_violations` guard (`matrix.rs:639`) forbids a hand-typed number, and its `distinct walks` / `budget stop` columns forbid reading a replayed record count as coverage.

**What "clean" means, precisely.** Every cell is `Clean` or `CleanModAdjudicated`; zero `Divergent`; zero `ShapeDivergent`. The adjudicated register (`conformance/ADJUDICATED.json`) is currently EMPTY (PROGRAM.md registers; DIVERGENCES holds DIV-1/2/4 which are S6-adjudicated and already suppressed or already Rust-correct), so on a correct tree every cell is plain `Clean` and `run()` returns `true` (exit 0). "Clean" is that exit code plus a report block whose `DIVERGENT` and `SHAPE-DIVERGENT` columns are `0` for all six worlds.

**How a divergence is classified and adjudicated.** On any cell divergence the comparator localizes (the `Emit::all()` rerun truncated to the divergent ordinal, `main.rs:492`) and the view-mode reclassification (`main.rs:465`) fires, yielding one of `TURN | ENUMERATION | DECISION | STATE | SCHEDULE | RNG | STATE(view)` with a fact-level path diff. Each such signal is adjudicated against the SPECS per the PLAN's ruling — (a) Rust bug → fix Rust and re-run; (b) Haskell bug → Rust keeps correct behavior, DIVERGENCES.md row + register suppression; (c) genuinely ambiguous → FORK to the user. A divergence at S10 is the one thing that can send the program backward, and the report must show the matrix clean before anything downstream proceeds.

**The [P4] baseline capture — the load-bearing new artifact.** The clean matrix run's Rust-side JSONL is the corpus `compare --baseline` will diff against forever after. It is captured HERE, while the frozen oracle still certifies it, so the baseline inherits the frozen's authority at the instant of capture: every stream committed to `conformance/oracle-baselines/` is one the Haskell just agreed with record-for-record. Layout: `conformance/oracle-baselines/<world>/{trace.jsonl, randtrace-<seed>.jsonl}` (or a single concatenated `<world>.jsonl` with the header records delimiting cells — panel to weigh corpus size vs net strength; see §7). This is a NEW captured artifact, not a script output — the matrix gains a `--capture-baseline <dir>` sink that writes the same `rust_stream` (`main.rs:581`) it is already computing, so the committed corpus is byte-identically the streams the frozen just cleared. **Do not thin the corpus below what makes `--baseline` a real regression net**: a baseline of only the trace cell would let a randtrace-only regression pass silently post-deletion.

**The wall-clock reality.** This is heavy and cabal-backed. The frozen side of every cell is a `cabal run -v0 prax-oracle` subprocess (`drive_frozen.rs:96`); the Rust side is in-process. The freeze-rev-keyed cache (`drive_frozen.rs:137`) means each distinct `(world, seed, mode, emit)` argv is a subprocess exactly ONCE, memoized under `rust/target/oracle-cache/<rev>/`; a re-run is disk reads. `--jobs $(nproc)` parallelizes over seeds within a world (`matrix.rs:443`), and the subprocess is the bottleneck, so wall-clock scales roughly with `3006 / nproc` cold cabal invocations — hours on a first cold run, minutes warm. It is driven as a background job (not interactively), its report block captured, its baseline committed. The die-seed sweep (`die-seed-sweep.sh`, §4/§5) rides the same tool and cache and is run in the same session against `village` as the [P3] RNG-crossing companion.

*Panel-charge §1.* Is 500 seeds × cap 50 the scale the PLAN's cut-over criterion actually names (yes — PLAN.md "Cut-over"), and is the matrix's `--min-records`/saturation machinery irrelevant here because the seed count is stated flat (500), not floor-derived? Is the baseline corpus captured at a granularity that makes `compare --baseline` catch a randtrace regression, not just a trace one? Is the register genuinely empty so "clean" means plain-clean, and is any DIV-* suppression that fires at 500 seeds accounted (it should not — DIV-1/2/4 are not matrix suppressions)?

## 2. The perf table — Rust ≥ Haskell, honestly

**[R2] Perf is a cut-over criterion, so it is measured like one: engine against engine, warm, release-vs-`-O2`, harness overhead excluded, reproducible from a committed invocation.** The dishonest version measures `cabal run` (which pays GHC/cabal startup on every call) against a warm Rust binary and declares victory; that is a harness comparison, not an engine comparison, and the note forbids it.

**What is measured.** The one workload both engines execute identically and deterministically: the `randtrace` walk (`drive_rust::rand_walk` ⇔ `Stress.runRandom`), which touches the full hot path — advance, `possible_actions`, `pick`, `perform_action`, boundary firing, expiries — without the planner's depth-2 search dominating the number (the planner is exercised by the `trace` walk; both are reported). Per world, a fixed seed band (e.g. `0..99`) at cap 50, summed to a total wall-clock and a per-record figure (records-compared is the honest denominator — a world that dead-ends early runs fewer records, `matrix.rs`).

**How each engine is invoked.** Both as BUILT BINARIES, no build/startup in the timed region:
- Rust: `cargo build --release -p prax-oracle`, then the built binary drives the walk in-process (`prax-oracle` already runs the Rust engine in-process, `drive_rust.rs` — "never a subprocess").
- Haskell: `cabal build -O2 prax-oracle`, then invoke the binary that `cabal list-bin prax-oracle` reports DIRECTLY (bypassing `cabal run`'s per-call resolution), so GHC startup is paid once as process launch, not per record, and cabal's dependency-resolution latency is excluded.

The frozen suite is already `-O2` (it is the pins' source and CI builds it release-ish); the Rust dev profile is `opt-level=2` (`Cargo.toml`), but the perf claim is made on `--release` to be the strongest honest statement.

**The acceptance bar.** For every shipped world, Rust total wall-clock over the sweep ≤ Haskell's — i.e. Rust throughput ≥ Haskell throughput, the PLAN's "Rust ≥ Haskell." Reported as a table: `world | records | haskell wall (median of k) | rust wall (median of k) | ratio`, `ratio = haskell/rust ≥ 1.0` for every row or the criterion FAILS. Methodology hardening: warm-up run discarded, median of `k≥5` runs (or `hyperfine --warmup 3` if available), single-socket pinning stated, machine named in the report so the number is reproducible.

**How a regression is surfaced.** A row with `ratio < 1.0` is a RED cut-over blocker printed as such — the perf table is a gate, not decoration. Because the deletion is irreversible, a perf regression is investigated and resolved (Rust optimized, or the number honestly reported as a recorded deviation the user weighs at the demo) BEFORE the demo is offered; it never silently passes.

*Panel-charge §2.* Is measuring the built Haskell binary via `cabal list-bin` (not `cabal run`) the honest exclusion of harness overhead, or does it hide a real deployment cost the user cares about? Is `randtrace` the right primary workload, or does Rust ≥ Haskell need to hold on the planner-heavy `trace` walk too (both should be tabled)? Is median-of-k with a discarded warm-up enough, or is variance across worlds wide enough to need per-world run counts? Is `--release` the fair Rust profile given the suite ships at `opt-level=2`?

## 3. proptest soak + the full green matrix

**[R3] The whole mechanical gate, run at soak depth, is a precondition to offering the demo — and it is exactly `verify.sh` plus a raised proptest budget.** Nothing here is new machinery; S10 turns the dials up and requires green.

**The commands (all must pass, in order, loud on first failure):**
- Full suite: `cargo test --manifest-path rust/Cargo.toml --workspace --no-fail-fast` — conformance green (the ~290 conformance lib tests + prax-core 247 + prax-worlds 44 + prax-oracle 59), including the meta-gate (`meta_gate.rs`, the exactly-once accounting over 849 HASKELL_PINS labels with the [P2] non-empty ≥849 floor) and the golden loaders.
- Lint: `cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings` — `--all-targets` so test code (most of the workspace) is covered.
- Zero unsafe: a scan asserting no `unsafe` token in the workspace source (the S9 diff was unsafe-free; S10 asserts it tree-wide — cleanest as a resident conformance sweep reusing `source_sweep::every_rust_source`, or a `#![forbid(unsafe_code)]` audit per crate).
- proptest SOAK: the banked law suites — trie/EL/query/expiry/persist (ARCHITECTURE) plus the flagship incremental==naive view invariant (`view_invariant.rs`) and reuse==live (planner) — re-run at raised depth: `PROPTEST_CASES=100000 cargo test --manifest-path rust/Cargo.toml --workspace <law filters>`. The persist round-trip law (S9 [R6]) and the expiry/supersession laws (CG-1's engine-scale nets) are in this set. Soak depth and any new regression seeds are committed under `rust/conformance/proptest-regressions/`.

`verify.sh` is the standing entry point and runs the suite/clippy already; S10's green matrix is `verify.sh` passing AT the deletion commit's tree (§4 shows verify.sh itself changes at that commit) PLUS the soak run recorded in the evidence report.

*Panel-charge §3.* Is `PROPTEST_CASES` at soak the right knob, and is the soak run's determinism (committed regression seeds) preserved so a soak failure is reproducible? Is the zero-unsafe assertion a resident test (survives) rather than a one-shot grep (evaporates)? Does the soak set actually include every ARCHITECTURE-named law (trie/EL/query/expiry/persist), and is each one non-vacuous at soak depth?

## 4. The [P4]/[P3] retarget BUILD — built AT the deletion commit, because its counterparty must be gone first

**[R4] This code cannot be written before the deletion and cannot be omitted from it. It is part of the deletion commit's diff, specified here so the manifest reviewer sees the whole change.** The honest form (S09 [P4]): not "specified and unbuilt at S9" (dead prose) but "deferred to the commit that makes it possible, with a binding contract." That commit is S10's deletion commit.

**`compare --baseline` (and `matrix --baseline`).** A new mode that replaces the frozen side with the committed corpus: instead of `drive_frozen::run_jsonl(spec.frozen_args(...))`, load `conformance/oracle-baselines/<world>/<cell>.jsonl` and feed it as the "reference" stream into the SAME `compare::compare_streams` (`main.rs:425`). The Rust side is still `rust_stream` (`main.rs:581`), computed live. So the net becomes Rust-now vs Rust-at-cut-over — a regression tripwire: any future engine change that perturbs a walk reddens against the frozen-certified baseline. The classifier, diff, register, and report block are unchanged; only the reference source swaps.

**What `drive_frozen.rs` deletion touches (exhaustive — from the grep).** The module is DELETED (not stubbed). Every caller must lose its frozen call in the same commit or fail to compile:
- `main.rs:27` `mod drive_frozen;` — removed.
- `main.rs` `cmd_check`/`check_compare` (139,142), `cmd_stress`/`stress_compare` (183,186), `cmd_worldshape`/`shape_compare` (214,217): these three differentials lose their frozen side. Their SUCCESSORS already exist as native conformance pins (S9: `typecheck_spec` + the describe golden; the StressReport native pins + CLI stress golden; `analysis_table_spec` + worldshape's slice-time role ending). Ruling: the `check`/`stress`/`worldshape` oracle SUBCOMMANDS are removed at the deletion commit (their job is done natively); `compare`/`matrix` survive on `--baseline`.
- `main.rs` `run_one_behind` (413), `localization_streams` (500), `view_divergence_before` (544): the compare/matrix hot path — retargeted to the baseline loader.
- `main.rs:387,142,186,217` `freeze_rev()` calls and `matrix.rs:261` `freeze_rev()`: the freeze rev was the cache key and the report tag; with no frozen subprocess there is no cache and no rev. Replace the report tag with a baseline-corpus identity (e.g. the corpus dir's committed content hash) so a stale baseline still cannot lie.
- `harness_tests.rs`: every test that drives the frozen oracle (worldshape/check/stress agreement, trace/randtrace/view/die-seed agreement, freeze-check gating, the cache-hoist and jobs tests — lines 80,218,241,266,344,409–414,679) loses its counterparty. These retarget to baseline-vs-Rust self-consistency tests OR are removed as frozen-only plumbing; the classifier self-test (`classifier_selftest.rs`) and the record/diff/walk-identity unit tests are frozen-free and SURVIVE.

**`freeze_check`/`freeze_rev` removed without orphaning callers.** `freeze_check()` (`drive_frozen.rs:40`) has exactly one production caller (`run_raw`, line 135, internal to the deleted module) and one test (`harness_tests.rs:413`); both die with the module. `freeze_rev()` callers are the six enumerated above, all retargeted in the same commit. `scripts/freeze-check.sh` (the shell subject) is deleted (§5). After the commit, `grep -rn 'freeze_check\|freeze_rev\|drive_frozen\|freeze-check' rust/ scripts/` must return NOTHING except historical prose in `docs/`.

**Script retargets.**
- `die-seed-sweep.sh` [P3]: drives `compare <world> --mode randtrace --die-seed` (line 33) — retarget the oracle invocation to `--baseline` so the RNG/die-seed sweep compares the live die-reseeded walk against the committed baseline. The default `village 20 20 50` grid is unchanged; only the reference source swaps.
- `verify.sh`: drop step 1 (`freeze-check.sh` — subject gone), step 2 (`cabal build prax-oracle` — no cabal), step 4 (`cabal test` — no frozen suite). Step 3 (`golden-check.sh`) retargets (below). Step 5 (`cargo build/clippy/test`) survives and becomes the spine, with its `--manifest-path rust/Cargo.toml` updated to the promoted root (§5).
- `golden-check.sh`: drop step 0 (freeze) and step 1 (extraction from `test/`, which is gone) and step 3 (cross-derivation via `cabal run` — no cabal); KEEP step 2 (the `conformance/goldens/SHA256SUMS` hash check), which the script's own header names as its "designed successor … committed WHILE THE FREEZE LIVES." `scripts/extract-golden.py` (frozen extractor) is deleted with the frozen tree.

*Panel-charge §4.* Is deleting the `check`/`stress`/`worldshape` oracle subcommands (rather than baseline-feeding them) right, given their native successors already exist — or does the differential harness want them baseline-fed for symmetry with `compare`? Is replacing the freeze-rev cache key with a corpus content-hash sufficient to keep a stale baseline from lying? Does `grep -rn 'cabal'` over the post-commit `scripts/` + `rust/` return only `docs`/comment prose? Are the `harness_tests.rs` retargets (baseline self-consistency) actual nets or vacuous after the frozen side is gone?

## 5. THE DELETION MANIFEST — the high-stakes, irreversible core

**[R5] One commit, after the tag `haskell-final`, deleting exactly the enumerated paths and promoting the workspace, such that `cargo test --workspace` + `verify.sh` are green immediately after. Nothing below is approximate: the reviewer checks the tree against these lists.**

### 5a. Pre-deletion tag

`git tag haskell-final` on the commit IMMEDIATELY BEFORE the deletion — the last state in which the frozen tree, the frozen oracle, the clean 500-seed matrix report, the captured baseline, and the perf table all coexist. This is the recovery/review point: the entire deletion is diffable as `git diff haskell-final..HEAD`, and the frozen tree is forever recoverable at the tag. The existing `haskell-freeze` tag is retained as history (its enforcement — `freeze-check.sh` — dies, but the tag pointer costs nothing and dates the freeze). The deletion is ONE commit; there is no partial state.

### 5b. Paths that DIE (exhaustive)

Frozen Haskell library and its accidents:
- `src/` (the entire frozen Haskell implementation)
- `app/` (`Main.hs` — the frozen CLI)
- `test/` (the frozen spec suite — the pins' source; HASKELL_PINS.txt is already a committed snapshot, §5d)
- `oracle/` (`TraceMain.hs` — the additive differential Haskell; dies WITH its counterparty, PLAN.md)
- `prax.cabal`, `cabal.project` (the cabal project)
- `dist-newstyle/` (cabal build output — already `.gitignore`d, so nothing tracked dies; the local dir is removed as cleanup)

Frozen-facing tooling:
- `scripts/freeze-check.sh` (its subject `git diff haskell-freeze -- src app test examples` no longer exists)
- `scripts/extract-golden.py` (extracts goldens from `test/` — gone; `SHA256SUMS` is the successor)
- `scripts/extract-pins.py` (extracts the meta-gate manifest from `test/` — per S09 [P2] it is NOT wired into `verify.sh` and `HASKELL_PINS.txt` is read-as-committed, so this is dead tooling → delete)

Frozen-driving Rust:
- `rust/prax-oracle/src/drive_frozen.rs` (deleted, not stubbed — §4)
- the frozen-driving arms of `main.rs`/`matrix.rs`/`harness_tests.rs` (retargeted or removed, §4)

CI:
- the Haskell half of `.github/workflows/verify.yml`: the `haskell-actions/setup@v2` step, the `~/.cabal/store`+`dist-newstyle` cache, the `cabal update` step. The one `verify` job survives, retargeted to Rust-only + the new workspace root (§5c).

### 5c. Paths that SURVIVE

- `docs/` in full (PLAN, PROGRAM, ARCHITECTURE, DIVERGENCES, reports, stages — the DESIGN is the contract).
- `references/` (`papers/ praxish/ repraxis/` — `.gitignore`d, local, kept).
- `examples/play.json` (the one data file the PLAN promises to keep; its net is the file-driven SHA256 + the `prax-cli`/`conformance` encoder-self-emission pins, S8 [R7]).
- `conformance/` DATA dir at repo root (`fixtures/ goldens/ HASKELL_PINS.txt ADJUDICATED.json KILLED.md README.md`) — survives; MERGES with the promoted crate (§5d).
- `README.md` (survives; edited to drop Haskell build instructions — an edit, not a deletion).
- The retargeted scripts: `verify.sh`, `golden-check.sh` (hashes-only), `die-seed-sweep.sh` (`--baseline`).
- NEW at this commit: `conformance/oracle-baselines/` (the [P4] captured corpus, §1).
- The entire `rust/` workspace — PROMOTED to root (§5d).

### 5d. THE WORKSPACE PROMOTION — enumerate every path it touches

`rust/` moves to the repo root. This is the manifest's most error-prone half, because the crate tree and the repo-root data dir it reaches into are BOTH moving relative to each other, and one `..` miscount silently orphans a fixture.

**The directory move + the `conformance/` collision.**
- `rust/Cargo.toml` → `/Cargo.toml`; `rust/Cargo.lock` → `/Cargo.lock`. Members are relative (`prax-core`, …) and unchanged; the header comment "lives at rust/ until cut-over" is corrected.
- `rust/prax-{core,vocab,script,worlds,cli,oracle}` → `/prax-*` (straight move).
- `rust/conformance/` (the CRATE: `Cargo.toml`, `src/`, `proptest-regressions/`) COLLIDES with the existing `/conformance/` (the DATA dir). Resolution: **MERGE** — the crate's `Cargo.toml` + `src/` + `proptest-regressions/` land INSIDE `/conformance/` alongside `fixtures/ goldens/ HASKELL_PINS.txt ADJUDICATED.json KILLED.md`. After the merge there is one `/conformance/` that is both a cargo crate and the data root. This is the single collision in the promotion and the reviewer must confirm it is handled, not clobbered.

**Every `CARGO_MANIFEST_DIR`- and `include_str!`-relative climb that must be RECOMPUTED (grep-verified list).** These reach from `rust/<crate>` UP into the root `conformance/`/`examples/`. Because the crate rises one level (rust/conformance → conformance) AND its data target is now inside the merged dir, the climb arithmetic changes per file — a blanket shift is WRONG; each is recomputed and proven by a green `cargo test`:

| file | current | after promotion (crate at `/conformance`, `/prax-*`) |
|---|---|---|
| `conformance/src/fixtures.rs:25` | `CARGO_MANIFEST_DIR + ../../conformance/fixtures` | `CARGO_MANIFEST_DIR + fixtures` |
| `conformance/src/goldens.rs:26` | `CARGO_MANIFEST_DIR/../../conformance/goldens` | `CARGO_MANIFEST_DIR/goldens` |
| `conformance/src/engine_replay.rs:26` | `../../conformance/fixtures/engine.json` | `fixtures/engine.json` |
| `conformance/src/npc_replay.rs:31` | `../../conformance/fixtures/npc.json` | `fixtures/npc.json` |
| `conformance/src/planner_replay.rs:41` | `../../conformance/fixtures/planner.json` | `fixtures/planner.json` |
| `conformance/src/meta_gate.rs:82` | `CARGO_MANIFEST_DIR` pop pop → root, `+conformance/HASKELL_PINS.txt` | crate IS `/conformance`; read `CARGO_MANIFEST_DIR/HASKELL_PINS.txt` (pop-pop→`/` then `conformance/…` still resolves, but must be re-verified) |
| `conformance/src/adjudicated_register.rs:26` | `CARGO_MANIFEST_DIR.parent().parent()` → root, `+conformance/ADJUDICATED.json` | re-verified against merged layout |
| `conformance/src/script_json_spec.rs:34` | `include_str!("../../../examples/play.json")` | `include_str!("../../examples/play.json")` (src→conformance→root, `examples/…`) |
| `conformance/src/script_supersession.rs:72` | `include_str!("../../../conformance/fixtures/cg1_supersession.json")` | `include_str!("../fixtures/cg1_supersession.json")` (src→conformance, `fixtures/…`) |
| `prax-oracle/src/worlds.rs:34` | `include_str!("../../../conformance/fixtures/cg1_supersession.json")` | `include_str!("../../conformance/fixtures/cg1_supersession.json")` (src→prax-oracle→root, `conformance/fixtures/…`) |
| `prax-cli/src/main.rs:306` | `include_str!("../../../examples/play.json")` | `include_str!("../../examples/play.json")` |

**The sweep roots that walk "every `.rs` under `rust/`"** must retarget to the promoted workspace root:
- `conformance/src/source_sweep.rs:11` `rust_root()` = `CARGO_MANIFEST_DIR/..` (feeds the inline-golden gate `goldens.rs` and the `unchecked_split_gate`) — after merge, `CARGO_MANIFEST_DIR/..` = `/`, the workspace root, which is correct; VERIFY the canonicalize target exists.
- `conformance/src/meta_gate.rs` `rust_root()` + `collect_rs_files(rust_root())` (the `// H:` pin sweep, line ~180) — same retarget; the pin corpus must still be swept over ALL crates.
- `conformance/src/gate_scanner.rs` — scans `prax-worlds/src/*.rs`; path relative, re-verified.

**CI + gitignore:**
- `.github/workflows/verify.yml`: `Swatinem/rust-cache` `workspaces: rust` → `workspaces: .`; the `run: ./scripts/verify.sh` step's script now targets the root workspace; the Haskell steps deleted (§5b).
- `.gitignore`: `rust/target/` → `target/` (keep the plain `target/`); the "workspace lives at rust/" comment corrected; the Haskell/cabal artifact block may be pruned.
- `scripts/verify.sh` + `scripts/die-seed-sweep.sh`: every `--manifest-path rust/Cargo.toml` → `--manifest-path Cargo.toml` (or dropped, run from root).

**The orphan sweep — the manifest's completeness proof.** Before the commit is offered for review, and after it is staged, these greps must be clean (only `docs/` prose may match):
```
grep -rn 'rust/'            scripts/ .github/ Cargo.toml prax-*/ conformance/   # no stale workspace prefix
grep -rn '\.\./\.\./\.\.'   prax-*/ conformance/                                 # no over-deep climb
grep -rn 'cabal\|haskell-freeze\|drive_frozen\|freeze_check\|freeze_rev\|freeze-check' \
                            scripts/ .github/ prax-*/ conformance/               # no reference to a deleted thing
```
And the definitional proof: `cargo test --manifest-path Cargo.toml --workspace` and `./scripts/verify.sh` GREEN on the post-deletion tree. A green `cargo test` is exactly the statement "no path climb orphaned a fixture," because every fixture-reading test panics loudly on a missing file (`fixtures.rs`, `goldens.rs`, `meta_gate.rs` all `panic!` on read failure — there is no silent-pass path).

### 5e. Every net that would BREAK the moment the frozen tree is gone — and its successor, confirmed IN PLACE BEFORE deletion

This is the S9 audit's whole purpose; S10 confirms each successor exists at the tag `haskell-final` so the post-deletion suite is green:

| frozen-comparing net | dies because | successor (built WHEN) | in place pre-deletion? |
|---|---|---|---|
| `prax-oracle compare`/`matrix` | frozen subprocess gone | `--baseline` vs `conformance/oracle-baselines/` (built §4, corpus captured §1) | corpus YES (captured this stage); mode built AT commit |
| `die-seed-sweep.sh` | drives frozen `compare` | folds into `--baseline` [P3] | AT commit |
| `worldshape`/`check`/`stress` differentials | frozen `run_json` gone | native `analysis_table_spec` / `typecheck_spec`+describe golden / StressReport pins+CLI golden (S9) | YES (S9-DONE) |
| `golden-check.sh` extraction+cross-derivation | `test/` + `cabal` gone | `conformance/goldens/SHA256SUMS` (S7 [D-C3], committed) | YES |
| meta-gate manifest (849 labels) | `test/` tree gone | `HASKELL_PINS.txt` committed snapshot + [P2] ≥849 non-empty floor (S9) | YES |
| `examples/play.json` byte-identity | frozen encoder gone | file survives + SHA256 + encoder-self-emission native pins (S8 [R7]) | YES |
| CLI stdout equality | frozen `prax` binary gone | committed exact-byte stdout goldens (S9 [P5]) | YES |
| `freeze-check.sh` | subject deleted | none needed — removed in the commit | n/a |

The two rows the program repeatedly flagged most-at-risk — the meta-gate manifest and the prax-oracle retarget — are the ones with the clearest confirmation: the manifest floor is built and tested (S9 [P2]), and the retarget's corpus is captured THIS stage under frozen certification, with only the mode-swap deferred to the commit that makes it possible. Post-deletion, `cargo test --workspace` runs the native successors and `verify.sh` runs the hash + baseline nets; both green is the cut-over's definition of "the reference was removable."

*Panel-charge §5.* Is the dies/survives partition EXHAUSTIVE — is there any tracked path under `src/ app/ test/ oracle/` or any frozen-facing script/CI fragment not listed? Is the `conformance/` crate-vs-data collision the ONLY promotion collision, and is the merge (not clobber) resolution correct? Is the per-file climb-recomputation table right to the `..`-count, and is `cargo test --workspace` green a sufficient completeness proof (are there any fixture reads that DON'T panic-on-missing and could pass silently)? Do the three orphan greps return only `docs/` prose? Is `haskell-final` the right recovery point and the deletion genuinely one commit?

## 6. The demo — the one user gate in the whole program

**[R6] The controller prepares everything up to the demo autonomously, then STOPS. The demo is the only place the user is consulted, "go" is the only approval, and NOTHING irreversible happens before it.**

**What the user is asked to do.** Play `village` and then `bar` on the Rust CLI, `prax play`, with a save/resume in the middle: start village, make several turns, save (`s`), quit, resume from the save (`prax play <world> resume`), continue to a natural stopping point; then the same for bar. This exercises the interactive loop, `renderScene` (the demo-verified-not-pinned surface, S9 [R11]), `playerActions` filtering, and the Persist round-trip (S9 [R6]) end-to-end through the human's hands — the surfaces no conformance pin covers because no frozen label exists for the interactive loop.

**What "go" authorizes.** Exactly one thing: the irreversible deletion commit of §5 (after `git tag haskell-final`). It authorizes nothing else and is the whole program's single approval (PLAN.md: "the user plays village+bar on Rust and says go").

**What the controller has done BEFORE offering the demo, and where it stops.** Autonomously and in order: the clean 500-seed matrix (§1) with its baseline captured, the perf table Rust ≥ Haskell (§2), the full green + soak matrix (§3), the `type_check == []` criterion confirmed for all six shipped worlds (the standing S9 net, carried into the matrix), and the deletion manifest (§5) STAGED and self-reviewed against the three orphan greps — but NOT committed. The `--baseline` retarget code (§4) is prepared as the deletion commit's content, reviewed by the panel, and held. The controller then STOPS and presents: the one-screen evidence report (matrix block, perf table, green/soak status, open forks — there should be none) and the deletion manifest for adversarial review. It does not tag, does not delete, does not promote. Only on the user's "go" does it execute the single commit. If the user does not say go, the frozen tree stands and nothing is lost — the reversibility the whole program preserved holds right up to that word.

*Panel-charge §6.* Is the save/resume demo the right exercise of the surfaces conformance cannot reach (`renderScene`, the stdin loop, Persist end-to-end), and is anything the user would touch NOT covered by either a pin or the demo? Is it airtight that no irreversible action (tag, delete, promote, force-push) can occur before "go" — including no premature baseline commit that would bless an unverified corpus? Does the evidence report give the user enough to say go responsibly without drowning them (PLAN's one-screen rule)?

## 7. § PANEL CHARGE — the adversarial review, before a byte is deleted

The two lenses attack this note as the last line of defense before an irreversible act. Soundness: is the state at the deletion commit one where the Rust's correctness is already proven and nothing that still needs the frozen tree is left pointing at it? Design/completeness: is the manifest exhaustive and the promotion total?

1. **Is the deletion manifest EXHAUSTIVE — nothing orphaned, no surviving reference to a deleted path?** Sweep `src/ app/ test/ oracle/` and every script/CI fragment independently: is any tracked frozen-facing path missing from §5b? After the staged deletion, do `grep -rn 'cabal\|haskell-freeze\|drive_frozen\|freeze_check\|freeze_rev\|freeze-check'` and `grep -rn 'rust/'` and `grep -rn '\.\./\.\./\.\.'` over the non-`docs` tree return NOTHING? Is `cargo test --workspace` green a genuine completeness proof, or is there a fixture read that fails silently rather than panicking?

2. **Does every frozen-comparing net have its successor built BEFORE deletion, so the post-deletion suite is green?** Walk §5e row by row: for each, is the successor actually resident at `haskell-final` (not "to be built later")? Is the [P4] baseline corpus captured at a granularity that makes `compare --baseline` a real regression net, and is it captured while the frozen certifies it? Are the `check`/`stress`/`worldshape` native successors (S9) truly sufficient, or does removing those oracle subcommands drop coverage the differential was silently providing?

3. **Is the workspace promotion COMPLETE?** Is the `conformance/` crate-vs-data collision correctly merged? Is the per-file climb-recomputation table (§5d) right to the exact `..` count — trace each `include_str!` and `CARGO_MANIFEST_DIR` path by hand? Do the "every `.rs` under `rust/`" sweep roots (`source_sweep`, `meta_gate`, `gate_scanner`) retarget correctly so the pin corpus and source gates still cover all crates? Are CI `workspaces:` and `.gitignore` `target/` updated?

4. **Is the perf methodology HONEST?** Does measuring the built Haskell binary (via `cabal list-bin`, not `cabal run`) fairly exclude harness overhead without hiding a real cost? Is the workload (randtrace + trace) and the denominator (records compared) right? Is median-of-k with a discarded warm-up and a named machine reproducible? Is `--release`-vs-`-O2` the fair engine-to-engine comparison?

5. **The irreversibility discipline.** Is it airtight that no tag/delete/promote/baseline-commit happens before "go"? Is `haskell-final` the correct recovery point, is the deletion exactly one commit, and is `git diff haskell-final..HEAD` the whole reviewable change? Could a divergence discovered at the 500-seed scale (that smaller sweeps missed) still send the program backward — and does the note handle that (adjudicate, never proceed on red)?

6. **Scope honesty.** Is anything in S10 quietly deferred without a home — a net assumed green that no command in §3 actually runs, a surface the demo is assumed to cover that it does not? Is `type_check == []` for all six shipped worlds actually asserted in the matrix/suite, not just claimed? Does the note write ZERO new engine semantics, as a hardening stage must?

## 8. PANEL RULINGS (governs — overrides §§1–7 on every point it touches)

Two-lens panel against the real tree before any deletion is drafted. Soundness: **SOUND-WITH-CORRECTIONS** (1 critical, correctable). Design/completeness: **COMPLETE-WITH-GAPS** (1 critical, correctable). Both confirmed the load-bearing invariant: **correctness AT the moment of deletion is genuinely proven** — the 500-seed frozen-vs-Rust matrix runs against the live oracle THIS stage before the tag, the deletion refuses to proceed on any red, the §5b dies-list is exhaustive, the `conformance/` merge is the only collision and is clean, all 11 `include_str!` climb rows are correct to the `..`-count, the [P2] manifest floor is present and bites, the S9 native successors (typecheck/analysis-table/stress) are resident and STRONGER than the frozen differential ever was, and `compare_streams`' max-len + `Value::Null`-fill means a mis-framed baseline reddens loudly — it cannot pass vacuously. Rulings in `.superpowers/sdd/rs-s10-panel-{soundness,design}.md`. The corrections below are binding.

### [P1] (from [D-C1]) — §5d pop-based helper arithmetic is WRONG; CORRECTED. The fix is a two-helper change, not two include-line edits.
`conformance/src/meta_gate.rs:81` (`fn repo_root()` = `CARGO_MANIFEST_DIR.pop().pop()`) and `conformance/src/adjudicated_register.rs:25` (`CARGO_MANIFEST_DIR.parent().parent()`) reach the root data files via POP, not `../`. Post-promotion `CARGO_MANIFEST_DIR = …/Prax/conformance`, so pop-pop lands at `…/Documents` — one level ABOVE the repo root — and every read `panic!`s on a missing file. The §5d table's claim "pop-pop→`/` … still resolves" is arithmetically FALSE and is STRUCK. **Binding fix:** in BOTH helpers change the pop/`parent` count from **2 to 1** (lands at `…/Prax`, keep `join("conformance/…")`/`join("docs/…")`). These are SHARED helpers — `meta_gate::repo_root()` feeds HASKELL_PINS.txt, KILLED.md (×2), PROGRAM.md; `adjudicated_register::repo_root()` feeds ADJUDICATED.json, DIVERGENCES.md — so the deletion commit edits the HELPER once, not the individual call sites, or the KILLED.md/PROGRAM.md/DIVERGENCES.md reads break. The `\.\./\.\./\.\.` orphan grep does NOT catch a pop-count; the mandated green `cargo test --workspace` (panics loudly on the missing file) is the sole backstop and must be run on the staged tree before the commit is offered.

### [P2] (from [S-C1]) — the surviving `--baseline` net must be FORCED to run in a standing gate, and its first green recorded pre-"go". BINDING.
`compare --baseline` is the SOLE comparator net that survives deletion, it does not exist yet, it is built AT the deletion commit, and §4's "retarget OR remove" escape permits an implementer to remove every frozen `harness_tests.rs` case and retarget none — leaving `cargo test --workspace` running ZERO tests over the `compare_streams`/matrix/baseline-loader pipeline, so the loader (new code written after the frozen is dead) would first execute in a manual operator run with no oracle to adjudicate. **Binding fixes, all three:** (a) DELETE the "OR removed" escape — ≥1 baseline-vs-Rust self-consistency test MUST survive as a `#[test]` in `prax-oracle` so `cargo test --workspace` exercises the loader path over the committed corpus; (b) add a first `compare --baseline`/`matrix --baseline` run over ALL SIX worlds (must be green) to `verify.sh` step 5, and wire `die-seed-sweep.sh --baseline` in (it covers `village` only today and nothing calls it), so the tripwire is proven ARMED at the deletion commit; (c) fold "first `compare --baseline` green (six worlds)" into §6's pre-"go" evidence, so the loader is certified while the frozen still lives to cross-check the captured corpus's bytes. This does not weaken core soundness (proven by the pre-deletion clean matrix); it closes the one place a SURVIVING net is left unrun at the instant its counterparty is destroyed.

### [P3] (from [D-N1]/[D-N2]) — the orphan-grep completeness proof is corrected. BINDING.
(a) Orphan grep #1 MUST also sweep `.gitignore` and `README.md` (`.gitignore:20` holds a stale `rust/target/` the advertised grep never sees; handled by the CI+gitignore bullet, but the "completeness proof" must actually cover it). (b) `conformance/KILLED.md:5,7` carry stale `rust/conformance/src/…` and `` `rust/` `` paths; KILLED.md SURVIVES, lives under `conformance/`, and is NOT `docs/` prose — so the note's "only `docs/` prose may match" pass-condition is FALSE as written. **Binding:** enumerate `conformance/KILLED.md` among the files needing a path edit at the deletion commit (`rust/conformance/src/…`→`conformance/src/…`, `` `rust/` ``→the workspace root), so the orphan greps pass legitimately rather than by an operator hand-waving two known hits.

### [P4] (from [D-N3]) — the source-sweep scope broadening is made explicit and SCOPED. BINDING.
Post-promotion `source_sweep::rust_root()` and `meta_gate::rust_root()` (`CARGO_MANIFEST_DIR/..` / pop-once) resolve from `…/Prax/rust` to the repo ROOT `…/Prax`, broadening the `.rs` sweep from rust-only to whole-repo. Coverage is a superset and safe TODAY (no `.rs` exists outside `rust/`; `references/` holds none), but a future stray `.rs` anywhere — e.g. an out-of-manifest `// H:` comment — would trip the meta-gate's exactly-once accounting as a spurious unaccounted label. **Binding:** at the deletion commit, scope both sweep roots to an EXPLICIT workspace-crate enumeration (the six `prax-*` + `conformance` dirs) rather than "the workspace root," OR document the whole-repo broadening loudly in-code with the exactly-once-accounting hazard named. Explicit enumeration preferred (it keeps the gate's domain equal to what it means to cover).

### [P5] (from [D-N4]) — CI frozen residue removed. BINDING.
`.github/workflows/verify.yml`'s surviving job step `name:` ("verify (freeze-check, oracle, golden-check, cabal test, cargo)") names deleted subjects, and `fetch-depth: 0` + `fetch-tags: true` existed to give `freeze-check.sh` the history to diff `haskell-freeze`. At the deletion commit: update the step name to the Rust-only reality and drop the now-purposeless deep fetch (nothing left diffs a tag).

### [P6] (from [S-N6]) — the perf timed region must be proven SYMMETRIC. BINDING on §2 methodology.
The frozen `randtrace` runs as a per-cell subprocess emitting JSONL per record (stdout serialization + process launch in the Haskell timed region); Rust's `rand_walk` runs in-process. If the Rust timed region excludes equivalent record serialization, the ratio is inflated in Rust's favour and a marginally-slower Rust engine could pass "Rust ≥ Haskell." **Binding:** pin the timed region symmetric — both engines either include or both exclude record emission (e.g. time the built Haskell binary doing the full walk with a single terminal emit, matching Rust's in-process walk), and STATE the chosen symmetry in the perf table's methodology note. The bias if uncorrected favours Rust, so this protects the honesty of a PASS.

### [P7] (from [S-N8]) — `stress`/`flow` CLI goldens must be BYTE-EXACT to match the S9 [P5] claim. BINDING.
S9 [P5] promised "committed exact-byte stdout goldens" for check/stress/flow/dump-play, but `prax-cli`'s `flow_renders_the_default_play_script` asserts only `contains(...)` and `stress_renders_a_deterministic_report` asserts `starts_with(...)`+`contains(...)`, naming "the oracle differential" (which DIES at deletion) as the byte-exact net. **Binding:** commit exact-byte stdout goldens for `stress` and `flow` (matching `check`/`dump-play`'s exactness) and assert against them, so a mid-output format drift is caught post-deletion by a resident net rather than only by a dying differential. Captured while the frozen still certifies the bytes.

### Panel handoff
No blocking-for-fatality ruling: the deletion's core soundness holds. [P1]–[P7] are binding corrections to the manifest/methodology, all landing AT the deletion commit or in the pre-demo hardening — EXCEPT [P6]/[P7], which are pre-demo (perf methodology; the stress/flow goldens captured while the frozen lives). Execution order: pre-demo HARDENING first (§1 matrix + `--capture-baseline` sink, §2 perf with [P6] symmetry, §3 soak, [P7] goldens) → STOP at the demo → on "go", the deletion commit carrying §4/§5 with [P1]/[P2a]/[P3]/[P4]/[P5] folded and [P2b]/[P2c] proven green. Nothing irreversible before "go".
