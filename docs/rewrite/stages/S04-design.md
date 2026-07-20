# S4 design — Types + Engine + the builder API (panel input; agent-side)

Frozen reference: src/Prax/Types.hs, Engine.hs, Cooked.hs, Rng.hs (draw/rollStep/seedBounds), Deontic.hs (obligedClose only), Derive.hs (the analysis-table builders), Relevance.hs (mayUnifySyms/evictionShadowNames only). Builds on S1 (Db/EL/Interner), S2 (Condition/Cond, query, read_anchors), S3 (CompiledRule, close/close_from, naive_closure, ground_head). Scope: rust/prax-core types.rs + engine.rs + a new compilepipe.rs + rng.rs + relevance.rs (two primitives) + a minimal prax-vocab::deontic. NOT in scope: roundBoundary/expiry-firing/schedule firing (S5), planner/intentions/desire analyses (S6), typeCheck (S9).

This is the make-or-break authoring surface: everything S7's 20 vocab modules and 6 worlds consume is fixed here.

## 1. The authoring family and its builders

types.rs gains the string-surfaced family, mirroring Types.hs field-for-field where load-bearing:

```rust
pub enum Outcome {
    Insert(String), Delete(String), InsertFor(i64, String),
    Call(String, Vec<String>),
    ForEach(Vec<Condition>, Vec<Outcome>),
    Roll(i64, i64, Vec<Condition>, Vec<Outcome>),   // public variant; `draw` is the guarded surface
}
pub struct Practice { pub id: String, pub name: String, pub roles: Vec<String>,
    pub actions: Vec<Action>, pub data_facts: Vec<String>, pub init_outcomes: Vec<Outcome> }
pub struct Action { pub name: String, pub when: Vec<Condition>, pub then: Vec<Outcome> }
pub struct Function { pub name: String, pub params: Vec<String>, pub cases: Vec<FnCase> }
pub struct ScheduleRule { pub name: String, pub period: i64, pub body: Vec<(Vec<Condition>, Vec<Outcome>)> }
pub struct Axiom { pub when: Vec<Condition>, pub then: Vec<String> }   // authoring twin of S3's CompiledRule
pub struct Want { pub when: Vec<Condition>, pub utility: i32 }
pub struct Desire { pub name: String, pub want: Want }
pub struct Character { pub name: String, pub wants: Vec<Want>, pub desires: Vec<String>, pub bound_to: Option<String> }
```

Builder idiom per ARCHITECTURE: `Action::new(label).when([...]).then([...])`, `Practice::new(id).roles([...]).action(...).data_facts([...]).init([...])`, `Function::new(name, params).case(when, then)`, `Character::new(name).want(...).holds(...)`, `ScheduleRule::new(name, period).clause(when, then)`. Builders take `impl Into<String>`; collection seams are `Vec<Condition>`/`Vec<Outcome>` on free constructors, positional params, and stored fields (the empty case `vec![]` infers; literal parity with frozen source; S1-S3 are already Vec-uniform) — single-level `impl IntoIterator<Item = Condition>` ONLY on omittable fluent setters (`.when(...)`), where the empty case is solved by omitting the call [D-panel I1; the double-generic Item = impl Into<_> form is BANNED — `[]` fails inference; compile-check the single-level form before committing the signature]. Builders are INFALLIBLE — they build values. Free outcome constructors join S2's condition surface: `insert(s)`, `delete(s)`, `insert_for(n, s)`, `call(f, args)`, `for_each(when, then)`; `Roll` is built only by `rng::draw` on the guarded path (the variant stays public so the unseeded-Roll pin can construct one).

**The rename primitive [D-panel I5]**: Coerce's namespaceKernel (op-preserving alpha-rename of authored variables into the Prax namespace, first-appearance order) is a general authoring-surface operation S7 cannot build from outside — `rename_vars(map_or_rule, conds) -> Vec<Condition>` (+ the outcome twin) is born in types.rs beside the walkers, pinned by CoerceSpec's rename-law scenarios when S7 arrives (an S4 unit pin covers the mechanics).

**Hygiene (v40).** `outcome_vars(&Outcome) -> Vec<String>` (total walk, ForEach/Roll recursing through guards AND bodies) and `outcome_sents(&[Outcome])` are born in types.rs; `authored_var_clash(forbidden, conds, outs)` and `authored_pat_clash` beside them. Blocklist semantics verbatim: Prax-namespaced (`is_prax_var`) OR in `forbidden`. Fire sites, exactly the frozen ones — combinator/install boundaries, never the builders: `draw` (forbidden = `[]`), `set_schedule` (forbidden = `["Actor"]`, per clause), and S7's vocab combinators.

**Loud-error inventory** — frozen `error` calls become `Result<_, WorldError>` at construction/install; new variants: `DuplicateActionName` (within one practice); `DuplicateFunctionName` (batch AND registry); `DuplicateScheduleRuleName` (batch AND cross-door; text names both doors); `MultiSegmentRuleName`; `NonPositivePeriod`; `ReservedVarClash { context, var }`; `SeedOutOfDomain` (frozen bounds); `DrawOdds` (0 < num < den).

Two stay PANICS (engine-invariant breaches): executing a `Roll` with `rng_seed == None` (S9's SeedlessDraw makes it static), and `current_turn` finding no lone numeric `turn` child.

## 2. State / Defs / Runtime, the install API, compilepipe::recompile

```rust
pub struct State { interner: Arc<Interner>, defs: Arc<Defs>, rt: Runtime }
struct Defs {   // authored sources retained (typecheck/persist/diagnostics)
    practices: BTreeMap<String, Practice>, fns: Vec<Function>, axioms: Vec<Axiom>,
    characters: Vec<Character>, desires: Vec<Desire>, schedule: Vec<ScheduleRule>,
    sorts: Vec<(String, Vec<String>)>, prediction_scope: Vec<Condition>,
    engine_rule_names: Vec<String>,      // v53 provenance; door-only writes
    reserved_families: Vec<String>,      // seeded ["turn", "contradiction"]; door-grown (§4)
    compiled: Compiled,                  // rebuilt WHOLESALE by compilepipe::recompile
}
struct Compiled {
    practices: BTreeMap<String, CompiledPractice>,   // instance_names, actions, inits
    fns: BTreeMap<String, (Vec<Sym>, Vec<(Vec<Cond>, Vec<Effect>)>)>,
    schedule: Vec<CompiledScheduleRule>,
    wants: BTreeMap<String, Vec<Vec<Cond>>>, desires: BTreeMap<String, Vec<Cond>>,
    scope: Vec<Cond>,
    rules: Vec<CompiledRule>,
    footprint: Vec<SmallVec<[Sym; 6]>>, axiom_heads: Vec<SmallVec<[Sym; 6]>>,
    neg_footprint: Vec<SmallVec<[Sym; 6]>>, cont_monotone: bool,
}
struct Runtime {  // plain clone; tries share structurally
    db: Db, view: Db,                    // private; writers: the perform tiers + with_db
    cursor: i32,                          // -1
    schedule_dues: BTreeMap<String, i64>,
    expiries: FxHashMap<CompiledPath, i64>,  // exact labeled path → due boundary
                                             // [S-panel I1: HashMap, NOT BTreeMap —
                                             // CompiledPath gets no Ord; iteration
                                             // order is INCIDENTAL and S5's firing
                                             // sorts by rendered name explicitly]
    rng_seed: Option<i64>,
}
```

`State::new()` seeds `turn!0` into BOTH db and view (emptyState verbatim). Install API on State, each ending in `recompile` (and `reclose` where db-visible): `define_practices` (data_facts via with_db), `define_functions`, `set_axioms` (recompile FIRST, then reclose — the frozen ordering), `set_characters`, `set_desires`, `set_schedule`, `seed_die`, `set_sorts`, `set_prediction_scope`, `with_db`. All return `Result<(), WorldError>`; worlds `.expect()` at build. The compiler door is a `pub mod door` NOT re-exported by any authoring prelude: `register_engine_rules` (shares `add_schedule_rules`; records names post-guard) and `register_reserved_families` (§4). Cross-crate sealing is by convention + the S9 `.rs` gate scanner (worlds must not import `door`); Rust visibility cannot express "prax-script only" — stated, not hidden.

**compilepipe.rs is the ONE choke point** (the retable heir). It owns the runtime `Effect` family and every authored→runtime conversion:

```rust
pub(crate) enum Effect {
    Insert(CompiledPath), Delete(CompiledPath), InsertFor(i64, CompiledPath),
    Call(String, Vec<Sym>),               // fn key stays String — registry key, never unified
    ForEach(Vec<Cond>, Vec<Effect>), Roll(i64, i64, Vec<Cond>, Vec<Effect>),
}
pub(crate) fn compile_outcome(...) -> Result<Effect, WorldError>;
pub(crate) fn ground_effect(...) -> Effect;   // groundCookedOutcome heir
pub(crate) fn recompile(...) -> Result<(), WorldError>;
```

`recompile` rebuilds `Compiled` wholesale: practices/fns/schedule/wants/desires/scope, `rules` (S3's compile over `defs.axioms` — deontics-free), then the axiom-derived tables S4 OWNS (the owed builders, homed in derive.rs): `axiom_footprint` (any-polarity body anchors + heads), `axiom_neg_patterns` (exactly the negated interiors), `axiom_head_patterns` (+ the `contradiction` witness), `monotone_axioms` (the frozen accept/reject decision table verbatim).

**S6's tables do NOT exist at S4** (improvables/liveness/cares_about land AT S6 — a present-but-empty table invites an accidental consumer; `Compiled` is crate-private so adding fields later is cheap). relevance.rs at S4: exactly `may_unify_syms` and `eviction_shadow_names` (the router's needs).

**obligedClose adjudication (owed row): discharge at S4, split per the panel's C1** — the first draft's "constants in vocab, S9 needs them anyway" rationale CYCLES in the crate graph (prax-core's checker → prax-vocab → prax-core; Cargo forbids it). Resolution (the panel's option (b), sharpened by v51's own precedent): the S9 lint never needed the OPERATOR — twin-presence and already-lifted are SYNTACTIC, computable from the prefix alone (v51's alreadyLifted was prefix-based) — so the CONSTANTS (`OBLIGED_HEAD`, `OBLIGED_LIFT_PREFIX`, `PUNITIVE_PREFIX`, `obligation_path`) live in prax-core (`vocab_consts` module, one home, checker-visible, no cycle) and the `obliged_lift`/`obliged_close` OPERATORS live in `prax_vocab::deontic`, building on the core constants. prax-vocab is still born at S4 (one module); the pin (closing over `obliged_close(axs)` derives the sub-obligation; bare closure does NOT lift) lands in conformance.

## 3. engine.rs — perform semantics and the router

`perform_outcome` compiles then delegates (one engine, two doors, no dual impl). `perform_effect` case-for-case with performCooked:

- **Insert(path)**: (1) cancel any pending expiry keyed by the EXACT labeled path (v44 supersession); (2) route through the tiers; (3) spawn check: `[practice, pid, r1..rn]` with pid registered, role count exact, instance NOT in the PRE-insert BASE db → run compiled `inits` under role bindings recursively — spawns can spawn.
- **Delete(path)**: purge expiries AT OR UNDER the path (name-prefix, labels ignored) BEFORE the retract; route irrelevant→apply_direct else with_db. Deletes never take the continuation tier.
- **InsertFor(n, path)**: perform as Insert (tiers, spawn, stale-cancel), then arm `expiries[path] = current_turn + n` (refresh on re-insert). Firing is S5's. (S5 flag: frozen fires dues in intern-id Ord order; the determinism contract says name order — adjudicate there.)
- **Call(fn, args)**: registry lookup by String name (v47); missing fn = silent no-op (frozen; S9 makes it static; pinned as such); FIRST matching case, queried against the BASE db (frozen quirk — pinned); first binding only.
- **ForEach(conds, effs)**: snapshot ALL bindings against the VIEW (empty seed), then apply.
- **Roll(num, den, conds, effs)**: None → panic; advance UNCONDITIONALLY (i64 Lehmer), store, on hit run exactly ForEach on the advanced state. rng.rs pulled INTO S4 (recorded stage-plan amendment; S5's Rng scope reduces to loop-level stream fixtures).

**The three-tier router ships at S4, in full** (its inputs — footprint, may_unify, shadows, neg_footprint, cont_monotone — are all S4 obligations; reclose-always would re-open engine internals in S5–S7 and forfeit the 621s→19s fix):

```
insert: !relevant(names, shadows)  → apply_direct (db AND view in lockstep)
        monotone(cont_monotone && no '!' && no neg_footprint unify)
                                   → apply_grow: close_from(...); ⊥ → full reclose
        else                       → with_db → reclose (⊥ ⇒ view = db + "contradiction")
```

Guarded by this stage's flagship net: `view == naive_closure(rules, db)` after EVERY perform in randomized + golden sequences.

`possible_actions(&mut self, actor)` / `perform_action` verbatim (view-queried, compiled, pids in NAME order); `ground_action_label` via renderText. **grounded_delta_anchors + safe_binders DEFERRED to S6** (sole consumer is v34 reuse) — two new KILLED rows, owed: S6.

Interner (settled; restated per S-panel I2): perform/possible_actions take `&mut self`. The fork-safety guarantee is two-part: (1) NO CROSS-FORK Sym COMPARISON — make_mut clones preserve existing ids, the planner discards forks, and nothing compares Syms across fork lineages; (2) every OUTPUT and every OBSERVABLE ORDER renders/sorts by name (ids are internal keys whose iteration orders are incidental — which is WHY expiries is a HashMap and S5 firing sorts by name). Persist round-trips by name.

## 4. Reserved families, provenance, the clock

`reserved_families` seeded `["turn", "contradiction"]` (constants in prax-core). scenePatience/currentScene are prax-script content (S8): constants live THERE; prax-script registers them via `door::register_reserved_families` — one home per constant; the S9 checker reads the state's own list. Enforcement stays STATIC (S9); S4 does not block reserved writes at perform, and says so. `engine_rule_names` door-only, post-guard. `current_turn` loud; dues seeded start-sated.

## 5. Pin surface at S4

- The 5 owed discharges: CookedSpec grounding → per-constructor `ground_effect` pins; DeriveSpec obligedClose → the prax-vocab pin; footprint/negPatterns/monotone → direct decision-table pins.
- EngineSpec: everything EXCEPT the two groundedDeltaAnchors pins (owed: S6). The build-order-death label is OWED WHOLE to S9 (its typeCheck-equality clause is only expressible then; consuming the label at S4 would silently drop that clause — the meta-gate accounts labels exactly once [D-panel I3]); S4 adds an INDEPENDENT compiled-rule-equality regression test that consumes no Haskell label.
- RngSpec: ALL pins land S4.
- GateSpec: the shared-guard half (authoredVarClash through draw); scanner half owed S9.
- TypeCheckSpec: nothing (S9 verdicts); S4 provides the substrate.
- ViewInvariant reborn on the real engine: generated vocabularies + random perform sequences; after EVERY perform, view == naive_closure (⊥ up-to-witness). World-turn shard at S7.
- **Engine fixture corpus, PRE-implementation [D-panel I4]**: extend `prax-oracle fixtures` with `engine` — unit perform-sequences → full state dumps from the FROZEN engine, one case per semantics corner the naive-closure net cannot see (spawn incl. the BASE-vs-view opacity case and re-spawn-after-delete, ForEach snapshot, Call's BASE-db quirk + first-case-first-binding, expiry arm/cancel/purge, Roll advance-on-miss, the ⊥ collapse). The Rust replay asserts byte-for-byte — perform semantics pinned by OBSERVATION of the frozen engine, not by transcription.

## 6. Panel charge

Attack, in order of expected yield:
1. **The builder surface, via a real port** (Core.adjustScore + scoreAtLeast as the acid test): is format!-driven path building ergonomic for ~200 combinators, or does it demand a helper NOW? The when/then seam types; .expect() vs ?-friendly world harness.
2. **Spawn corners**: BASE-db existedBefore vs view-visible instance (construct the divergence); recursive spawn termination; spawn × monotone tier; role-count-short inserts under `!`; inits with non-role variables (frozen grounds to a literal — keep or flag).
3. **The router-at-S4 decision**: does the naive-oracle net adequately guard close_from's contract; the ⊥-fallback equivalence; does the generator reach the monotone tier with cont_monotone both true and false?
4. **Interner-in-state**: is append-only fork-safety airtight vs the planner's clone-and-discard AND the persist path, or should S4 adopt an interior-mutable interner (new dep = panel decision)?
5. **Reserved growth + doors**: register_reserved_families idempotence; the S9 scanner as seal; provenance/reservation drift across two door calls.
6. **The deferrals' honesty**: grounded_delta_anchors→S6 (visibility later?); prax-vocab early birth; the S5 Rng re-scope.
