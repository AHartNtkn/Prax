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
| S8 | Script + Play + Audience ([A1]) | DONE | [S08](reports/S08-script-play-audience.md) |
| S9 | TypeCheck + AnalysisTable + Stress + Persist + Inspect + CLI | IN PROGRESS (panel done: SOUND + COMPLETE-WITH-GAPS, 0 critical; implementer running) | — |
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

**Open gaps: NONE.** CG-1 was the only one, and S8 closed it.

### CG-1 — the v44 SUPERSESSION law had no AUTHORED-DATA net at world scale — **CLOSED at S8**

**The law.** A BARE insert onto a path already carrying a pending expiry CANCELS
that expiry, so the fact stands permanently
(`prax-core/src/engine.rs`, `Effect::Insert`). A re-`InsertFor` on the same path
routes through the same branch and then re-arms: a REFRESH, not a supersession.

**What the gap was.** The law had an ENGINE-scale net from S5
(`schedule_spec::law_3_bare_re_insert_cancels_the_timer` and
`engine_replay::engine_scenarios_replay_byte_for_byte`, both over a bare
`State::new()`), and a WORLD-scale net from the S7 fix wave
(`supersession_world`, the same law over the real compiled `village_world()`,
with a control and the refresh trap beside it). But in both, the bare insert is
performed BY THE TEST. No shipped world reached the case through its own
AUTHORED data, so a supersession bug that only manifested through authoring had
no net — measured slice by slice: bar/dm/village write `*.feels.*` through
exactly two routes, `feel_toward_for` (an `InsertFor`) and `unfeel_toward` (a
`Delete`), and a 42-turn village trace hit `expiries.remove` 100 times, all
refreshes, zero bare supersessions.

**What closed it.** S8's `conformance::script_supersession`, driving
`conformance/fixtures/cg1_supersession.json` — a real authored play-script whose
scene setup arms its OWN timer (`{"insertFor":{"rounds":3,"sentence":
"lantern.lit"}}`) and whose one beat bare-inserts that same path. Left alone the
lantern goes out at boundary 3 and the junction reading its absence ends the
story; shielded, the pending expiry is cancelled, the fact stands eight
boundaries later — more than twice the authored lifetime — and the ending never
comes. Supersession, from authored data, at world scale, through the JSON door,
with the CONTROL and the REFRESH trap pinned beside it.

**The claim that had to be broken first.** Both this file and the S8 design
asserted the gap could not close at S8: *"`Prax.Script.compile` REFUSES at
construction any authored condition or outcome headed `scenePatience` — so a
bare insert onto a live script timer is not merely absent from S8's worlds, it
is inexpressible in a script."* **That is FALSE.** The refusal covers only the
compiler-owned `scenePatience` family. `InsertFor` is in the AUTHORED `Outcome`
surface: `sceneSetup` accepts it (`Script.hs:105`), `beatEffects` accepts it
(`Script.hs:120`), and `Prax.Script.Json` spells it directly
(`Json.hs:139-140`). A script can arm a timer on any path it likes and then
cancel it, and `compile` never looks. The S8 soundness panel found this, and it
was re-measured independently on the FROZEN engine over the same committed file
before any Rust was written:

```
compile ACCEPTED the authored InsertFor + a bare Insert of the same path
control    (i, lantern.lit, ending) = [(0,True,Nothing),(1,True,Nothing),(2,True,Nothing),
                                       (3,False,Just "darkness"),(4,False,Just "darkness"), …]
supersede  expiries after the beat  = []
supersede  (i, lantern.lit, ending) = [(0,True,Nothing) … (8,True,Nothing)]
```

**The mutation evidence.** With `rt.expiries.remove(path)` deleted from
`prax-core`'s `Effect::Insert` arm, `cargo test --workspace --no-fail-fast`
gives exactly FOUR REDs and nothing else:

```
schedule_spec::tests::law_3_bare_re_insert_cancels_the_timer                        (S5, engine scale)
engine_replay::replay::engine_scenarios_replay_byte_for_byte                        (S5, engine scale)
supersession_world::tests::a_bare_insert_onto_a_live_village_timer_supersedes_it    (S7, world scale, driven externally)
script_supersession::tests::a_beats_bare_insert_supersedes_the_scripts_own_authored_timer   (S8, AUTHORED)
```

The fourth is the new one, and it is the one the gap was about. The control
(`the_authored_timer_fires_on_schedule_when_nothing_touches_it`) stays GREEN
under that deletion, as it must — deleting the cancellation does not stop
expiries firing — which is what makes the pair a measurement rather than a
coincidence.

**Differential.** The same committed file is the `cg1` world on BOTH engines —
the frozen oracle reads it at `oracle/TraceMain.hs`'s `cg1ScriptPath`, the Rust
registry embeds it at `prax_oracle::worlds::CG1_SCRIPT_JSON` — so the two cannot
be driven by different scripts. It is a FIXTURE, not shipped content, so it is
absent from `allWorldNames` exactly as `probe` and `bigfeud` are.

**The REFRESH half** is covered twice over at S8: by the authored re-arm
(`re_arming_the_authored_timer_refreshes_rather_than_supersedes`) and by scene
RE-ENTRY re-arming a patience marker through the compiler's scene-entry fold,
reached by an authored `goto`
(`conformance::script_spec::re_entry_resets_a_timed_junction`).

**Disposition.** CLOSED. Nothing carries to S10.
