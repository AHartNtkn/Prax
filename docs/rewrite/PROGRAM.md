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
| S2 | Query (one compiled path) | DONE | [S02](reports/S02-query.md) |
| S3 | Derive + view (design-heavy) | DONE | [S03](reports/S03-derive.md) |
| S4 | Types + Engine + builder API (design-heavy) | DONE | [S04](reports/S04-engine.md) |
| S5 | Loop + Schedule + Rng (stream landed S4) | DONE | [S05](reports/S05-loop-schedule.md) |
| S6 | Planner + Minds + Relevance + Sight (design-heavy; fidelity summit) | DONE | [S06](reports/S06-planner.md) |
| S7 | Vertical world slices: Feud → Intrigue → Bar → Village (Audience → S8, [A1]) | SLICES 1-4 LANDED, slice-4 fix wave applied; slice-4 evidence report outstanding | [S07-1](reports/S07-slice1-feud.md) [S07-2](reports/S07-slice2-intrigue.md) [S07-3](reports/S07-slice3-bar.md) |
| S8 | Script + Play + Audience ([A1]) | — | — |
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

## CARRIED GAPS

Laws the program has NOT netted at the scale it intended to, carried forward
here so they cannot evaporate between stages. A gap is not a divergence — the
Rust is right — so `DIVERGENCES.md` is the wrong home. Each entry names the law,
what covers it, what does NOT, and where it is due. **A gap still open at S10
must appear in the cut-over criteria as a stated limit of coverage.**

### CG-1 — the v44 SUPERSESSION law has no AUTHORED-DATA net at world scale

**The law.** A BARE insert onto a path already carrying a pending expiry CANCELS
that expiry, so the fact stands permanently
(`prax-core/src/engine.rs`, `Effect::Insert`). A re-`InsertFor` on the same path
routes through the same branch and then re-arms: a REFRESH, not a supersession.

**What covers it.**

- **Engine scale, S5**: `conformance::schedule_spec::law_3_bare_re_insert_cancels_the_timer`
  and `conformance::engine_replay::engine_scenarios_replay_byte_for_byte`.
  Re-verified in the slice-4 fix wave: with `rt.expiries.remove(path)` deleted,
  `cargo test --workspace` gives **exactly those two REDs** (138 passed, 2
  failed) and nothing else. Real detectors — but they drive a bare
  `State::new()`: no practices, no rules, no derived view, no eviction shadows.
- **World scale, S7 fix wave**: `conformance::supersession_world` — the same law
  over the real compiled `village_world()`, through the village's own emotion
  vocabulary, with a control (a timer left alone still fires) and the refresh
  case pinned beside it so the trap cannot be mistaken for the law. Reddens
  under the same deletion.

**What does NOT cover it.** No shipped world reaches the case through its own
AUTHORED data, so a supersession bug that only manifests through authoring has
no net. Measured, slice by slice:

- **bar / dm (slice 3)** and **village (slice 4)** write `*.feels.*` through
  exactly two routes — `feel_toward_for` (an `InsertFor`) and `unfeel_toward` (a
  `Delete`). There is no bare-insert route to the family in either. Instrumented
  over village: a 42-turn trace hits `expiries.remove` 100 times, **all
  refreshes, zero bare supersessions**; 60 randtrace seeds, zero. With the line
  deleted, `worldshape village --check`, the trace, 60 randtrace seeds and the
  300-seed matrix all stay CLEAN.
- **S07-design §13 asserted village writes the family "through more than one
  route" and made a slice-4 mutation RED binding on that premise. The premise is
  FALSE**; §13 is struck and corrected, and §14 records the measurement.
- **S8 (Audience, Play, Script) does NOT reach it either, and cannot.** Checked
  in the fix wave: neither `src/Prax/Worlds/Audience.hs` nor
  `src/Prax/Worlds/Play.hs` calls any timer-arming combinator; the only timer in
  the script machinery is `InsertFor n scenePatience.<sid>.<j>`
  (`src/Prax/Script.hs:398`), armed by the compiler, and Audience does arm one
  (`timeout "dismissed" 5`). But `Prax.Script.compile` REFUSES at construction
  any authored condition or outcome headed `scenePatience`
  (`scenePatienceOffenders`, `Script.hs:282`) — so a bare insert onto a live
  script timer is not merely absent from S8's worlds, it is inexpressible in a
  script. S8 will not close this gap.

**Disposition.** Carried to **S10**. Nothing before it can close it: the
remaining stages add no world whose authored data reaches the case. S10 either
authors a world that does, or states in the cut-over criteria that the law's
world-scale coverage is engine-driven (`supersession_world`) and not
authored-data-driven, and that a supersession bug reachable only through
authoring would ship unnetted.
