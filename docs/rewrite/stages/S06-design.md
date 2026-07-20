# S6 design — Planner + Minds + Relevance + Sight: the fidelity summit (panel input; agent-side)

Frozen reference: src/Prax/Planner.hs (THE fidelity-critical file), Minds.hs, Relevance.hs (the S6 half), Sight.hs, Loop.hs (npcAct/runNpcTicks), Engine.hs (groundedDeltaAnchors/safeBinders only), Types.hs (MotiveSignature/Intention/Liveness). Builds on S1–S5. Scope: rust/prax-core planner.rs + a NEW minds.rs + relevance.rs (the S6 tables) + sight.rs + turn.rs (npc_act/run_npc_ticks) + engine.rs (grounded_delta_anchors, the Runtime intentions field) + compilepipe.rs (three Compiled fields). NOT in scope: producibleAtoms (S9), any shipped world (S7), persist of intentions (S9 — but the field must be persist-shaped now).

This is the summit: every differential golden and every S7 world trace is downstream of `pickAction`. A one-ulp or one-tiebreak divergence here is not a score bug, it is a DIFFERENT STORY. The contract is bit-reproducibility against the frozen engine, verified by observation (the planner corpus, §7), not by transcription.

## 1. The scoring arithmetic, bit-exactly

**The integer core.** `evaluate`/`evaluateCooked` are INTEGER: `Σ utility × #satisfying` (count = query bindings, duplicates included). Rust: `evaluate_compiled(&State, &[(Vec<Cond>, i32)]) -> i64` with i64 accumulation. `predictMove`'s internal sort is over these INTEGERS — no FP in prediction. FP exists in exactly one place: `valueAfter`.

**The one f64 lift.** `base = fromIntegral (evaluateCooked …)` — one i64→f64 conversion, exact for |x| < 2⁵³. Every frozen f64 op maps to EXACTLY one Rust f64 op with identical operand order. The frozen expression tree:

```
valueAfter = base + (othersScore + selfNext)
othersScore = foldl over othersAfter st1:  acc' = acc + 0.5 * lift(eval s')   (LEFT fold, acc₀ = 0.0)
selfNext    = 0.9 * v   where v = head of go (d-1)   (or 0.0 on empty)
rest        = 0.0 when d <= 0
```

Rust shapes, verbatim:

```rust
let base = evaluate_compiled(&st1, &model) as f64;
let rest = if d <= 0 { 0.0 } else {
    let mut acc = 0.0_f64;
    for m in others_after(&st1, actor) {              // st1's LIVING cast, rotation order
        if let Some(ga) = predict_at(...) {
            acc = acc + 0.5 * (evaluate_compiled(&s2, &model) as f64);
        }                                              // skip = no term
    }
    let self_next = match go(d - 1, after_delta, after_round).first() {
        Some(&(_, v)) => 0.9 * v,
        None          => 0.0,
    };
    acc + self_next                                    // ONE add
};
base + rest                                            // outermost add LAST
```

Load-bearing: `0.5 * x` on a small-integer lift is EXACT; the rounding lives in the fold's ADDs and `0.9 * v` — association order IS the contract: `base + (acc + 0.9v)`, never `(base + acc) + 0.9v`. Rust is strict IEEE f64 (no fast-math, no auto-FMA); GHC x86_64 is SSE2 — same ops. Do NOT hoist, refactor, or "simplify"; the shape is the spec.

**Sort and tiebreak.** Frozen: `sortOn (Down s, gaLabel)`, STABLE. Rust: `scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.label.cmp(&b.0.label)));` — stable, so full ties fall to CANDIDATE ENUMERATION ORDER, which is observable and is S4's possible_actions order (pids by name, instance bindings by name, actions by declaration, condition bindings by name), already pinned. `candidateActions` = that list, `[]` if `dead.<name>` in BASE db, filtered to `bound_to`. `total_cmp` vs GHC `Ord Double` differ only at NaN/−0.0 — both unreachable (finite sums; all zeros arise +0.0); the corpus would catch it.

**Depth.** `go d`: `d <= 0` ⇒ rest = 0; `selfNext` recurses `go (d-1)`; `pickAction depth = first of scoreActions depth`. `lookaheadDepth = 2` is the LOOP's constant — prax-cli (S9) + replay tests, NOT prax-core (core takes depth as a parameter, as frozen).

**One permitted hoist**: `cookedSelfWants st1 actor` depends only on defs + actor; computed once per pick (frozen recomputes per node, ~5,400×/round). Value-identical (integer conds, same tables) — a shape deviation with an equality argument.

## 2. The v34 prediction-reuse memo

`type PathDelta = Option<Vec<SmallVec<[Sym; 6]>>>` (`None` = opaque; nothing at/below reuses).

**extendDelta** (always st0's tables): `Just(old ++ new ++ [h | feeds, h <- axiom_heads, h ∉ old])`, `feeds = any new may_unify footprint`. Heads dedup against OLD only — verbatim. axiom_heads includes the contradiction witness.

**The reuse gate** (`predictAt`): reuse the ROOT's step iff transparent path AND mover in root cohort AND no delta anchor may_unify the mover's root read anchors. Everything else computes LIVE at the CURRENT state `s` (not st0); a miss is NEVER cached as a root value.

**The strict-Rust memo** (replacing frozen lazy Map values):

```rust
struct PickMemo {
    cohort: Vec<String>,                                   // others_after(st0), EAGER (names)
    steps:  FxHashMap<String, Option<GroundedAction>>,     // filled on first REUSE per mover
    reads:  FxHashMap<String, Vec<SmallVec<[Sym; 6]>>>,    // filled on first GATE CHECK per mover
}
```

A plain `&mut` memo threaded through `go`, filling on demand against a retained root fork (st0.clone(), cheap). OnceCell REJECTED (filling needs `&mut Interner`; interior mutability fights the walk's borrows). Eager two-pass REJECTED (precomputing every root prediction is the work v34 avoids). Lazy-fill order differs from frozen's force order — unobservable (values are pure functions of (st0, actor, mover); only interner-id allocation shifts; ids unobservable).

**Soundness, re-expressed**: the reuse==live flagship proptest — generated vocabularies + perform-prefixes, scoreActions run twice via an internal `reuse: bool` test switch, FULL scored tables equal by (label, f64::to_bits). The three frozen reuse pins re-express behaviorally (fixtures where a wrong reuse changes a decision).

**grounded_delta_anchors + safe_binders land in engine.rs** (frozen home; walks Effects, resolves Calls through Compiled.fns — pub(crate) internals). Verbatim opacity rules: unresolvable Call; insert head literally `practice`; insert head a variable ∉ safe_binders; all-variable paths. safe_binders = vars bound at a non-first position of a top-level positive CMatch and never at a first position; Call resets. The two owed EngineSpec pins discharge here.

## 3. Minds — a NEW minds.rs

- `believed_desires(st, p, m)`: filter defs.desires IN VOCABULARY ORDER by view prefix-existence of `<p>.believes.desires.<m>.<name>` (any provenance child satisfies). The `believes.desires` constants join vocab_consts (one home; planner's believesRead anchor + msKnownMotives read the same family).
- Compiled-wants plumbing as frozen consumes it: `cooked_self_wants` = zip(Compiled.wants[name], charWants utilities) ++ `cooked_desires_for` (own desires, vocab order); desires ground Owner by SUBSTITUTION, dead_now grounds Owner by SEED BINDING — mirror each site's mechanism exactly (equivalent, but cheap fidelity is copying, not proving).
- `want_for`/`self_wants` (string-surfaced diagnostics) exist for MindsSpec pins; the planner never touches strings.
- `professed()`/`conventional()` are AXIOM builders in prax-core::minds, NOT vocab (frozen library surface, pinned by MindsSpec, read by planner core; no cycle).

## 4. Relevance — the S6 tables land in Compiled

Compiled gains exactly three fields: `improvables: Vec<String>`, `liveness: BTreeMap<String, Liveness>`, `cares_about: BTreeMap<String, Vec<String>>`, rebuilt in recompile AFTER rules/practices/fns/wants/desires (retable's order). `Liveness { FloorCheck, GateCheck(Vec<Vec<Cond>>), AlwaysLive }` in relevance.rs, pub(crate).

relevance.rs S6 additions (all over COMPILED forms): `cooked_fn_pool`, `cooked_outcome_atoms` (Option=wild; eviction shadows on deletes; InsertFor=Insert; cycle-guarded Call), `cooked_want_patterns` (pos/neg/uncertain; Absent swaps polarity; Calc/Count/Subquery taint), `world_atom_pools` (MOVER practices + inits ONLY — never the schedule; that exclusion makes schedule-moved facts environment gates, the v35 wake), `axiom_derivable`, `improvable_desires` + `liveness_of` (decision recipes VERBATIM), `bearing_templates` (→ cares_about; read_anchors is deliberately a DIFFERENT walk from want_patterns), `mover_read_anchors` (per-pair at pick time, NOT a table: scope template Actor:=actor Witness:=m; the believes family with PraxD wildcard; the dead mark; every practice instance pattern + action conds + outcome-embedded conds with Actor:=m; fn cases fully wild; desire conds Owner:=m).
**S6 vs S9**: producible_atoms stays OUT (typeCheck's, ranges over schedule + live db); cooked_outcome_atoms/cooked_fn_pool born S6, shared forward.

## 5. Intentions (v35) — a Runtime change, flagged loud

S4's Runtime has NO intentions field; S6 ADDS `intentions: BTreeMap<String, Intention>` (name-keyed, plain clone; frozen Persist saves it, so persist-shaped now). `MotiveSignature { bearing, satisfaction (COUNTS per want, cooked_self_wants order), live_desires, known_motives }` and `Intention { act: Option<GroundedAction>, basis }` in planner.rs (Eq). motive_signature verbatim (four walks, no scoring; bearing = sorted dedup of candidate ids ∩ cares_about; known_motives = the two-level child_keys walk). Signature comparison is list EQUALITY (both languages enumerate deterministically from the same fact set).

`npc_act(depth, actor, st)` in turn.rs: stored intention + equal signature + still_offered (FULL GroundedAction equality vs current candidates; None always offered) → act WITHOUT deliberating; else pick_action, STORE (a None pick is stored too — doing nothing is a commitment), act. `run_npc_ticks`: advance → npc_act, labels of performed actions only.

## 6. Sight

sightRule landed S5. S6 owns Sight.hs's remainder: `sighted_within(h)` (the atSince window fragment, verbatim — sight.rs stays four conditions long). Everything else sight-shaped is S7 content. SightSpec re-expresses at S6 except its typeCheck label (owed S9).

## 7. The pin surface at S6

- The 6 owed discharges (rows REMOVED): EngineSpec ×2 → engine.rs; LoopSpec ×4 (intention family) → conformance.
- **PlannerSpec (21 + group): ALL re-express at S6; expected ZERO decimal KILLED rows.** Exact-equality kept where exact by construction (depth-0 integer lifts: 10.0, 0.0); rounding-bearing values (9.0 = 0.9*10.0) RESTATED as ordering/choice assertions with the exact value delegated to the corpus. The decimal→ordering conversion happens inside re-expressions, not via KILLED rows.
- MindsSpec (9): all at S6. RelevanceSpec: ~12 re-express on synthetic fixtures; FIVE drive villageWorld → KILLED deferral owed:S7 (the later-of convention). SightSpec: 9 at S6, 1 owed:S9. LoopSpec: 4 stay owed:S7.
- **THE PLANNER CORPUS — build FIRST (adjudicated: the program's most valuable artifact).** Extend prax-oracle fixtures with:
  - `planner.json`: synthetic worlds (the specs' inline fixtures: tendBar, ∀-host, believed/false/gossiped models, scope gates, deadNow recipes, reuse-triggering deltas) × dumps of scoreActions tables at depths 0/1/2 (label + score as round-trip show decimals), pickAction, predictMove per pair, motiveSignature, AND the relevance tables rendered. Rust replay compares scores by f64::to_bits after parse. Include a FOLD-ORDER CANARY: ≥3 movers, depth 2, utilities engineered so a wrong association changes the bits.
  - `npc.json`: runNpcTicks narrations + final dumps over synthetic casts (20–30 turns, boundaries, deaths, intention holds/wakes) — npcAct end-to-end before any world exists.

## 8. Panel charge

1. **The fold-order VERIFICATION**: is corpus+to_bits airtight, or can compensating double-errors hide? Design the canary that discriminates `base + (acc + 0.9v)` from every plausible wrong association; confirm no FMA/contraction in release; confirm Aeson emits shortest-round-trip doubles for every corpus score.
2. **The memo redesign**: fill-on-demand equivalence total (miss-at-s never cached; cohort eagerness right)? Does the reuse==live generator actually REACH reuse (instrument to require both arms)?
3. **The gate's fidelity**: a wide gate changes decisions only under specific delta shapes — what nets it: the three frozen pins + which corpus scenarios? Construct the adversarial case (delta feeding an axiom whose head the mover reads only via a shadow).
4. **The intentions Runtime change**: clone cost per fork; equality derives; does still_offered's full-equality survive Bindings representation differences?
5. **Candidate-order dependence**: can any S6 path reorder candidates (memo, forks, interner growth)? Prove not, or pin it.
6. **The S7 boundary**: the five village-owed RelevanceSpec rows; cares_about with no over-bearing world; depth-2 outside core; what the npc corpus leaves for barWorld to hit cold.

## 9. PANEL RULINGS (binding addendum — folded from both lenses; where this section conflicts with §§1-8, THIS governs)

- **[S-C1] The memo's cross-lineage invariant, STATED and enforced**: predictAt's may_unify compares walk-lineage delta Syms against st0-clone-lineage read-anchor Syms — sound iff EVERY CONSTANT read anchor is st0-resident (recompile-interned before any clone; the sole post-clone read name, the PraxD wildcard, is a variable and never id-compared). This is now an invariant, not an accident: mover_read_anchors sources the scope template from Compiled.scope (never re-cooks raw prediction_scope), and the planner corpus gains the collision fixture (a runtime-minted constant — Calc/Count result — colliding by name with a scope-read literal; a wrong impl misses the match and reuses unsoundly). st0.clone() remains the fill basis (frozen's rootReads are st0-based). **The fixture's guard is load-bearing** (S6 review I-3): as first built, the minting action was guarded on `Not gate.2`, which made the gate family a read anchor of every mover in its own right — the scope's contribution to `mover_read_anchors` was then a duplicate, and deleting the whole scope-read component changed no score, pick or prediction anywhere in the corpus. The guard is now `Not computed.Actor`, so the scope read is the ONLY anchor intersecting the delta: suppressing it reuses bob's root `Nothing` stale and alice's depth-1 and depth-2 pick flips from `compute the gate` to `Wait about`. The claim this ruling makes — a wrong impl misses the match and reuses unsoundly — is now netted at the DECISION level, not merely by the `readAnchors` dump.
- **[S-I1] still_offered's soundness surface**: the id-sorted-SmallVec + derived-Eq Bindings representation is PINNED as load-bearing (content-canonical equality under a monotonic interner); the S9 persist-reload reintern hazard is flagged forward (a loud note in the S9 stage inputs: intentions' GroundedActions must re-associate by rendered content, not raw ids).
- **[S-I2 — CORRECTED BY THE IMPLEMENTER'S ARITHMETIC; this recipe governs]**
  The panel's "2^53-adjacent utilities" is INFEASIBLE and unnecessary: authored
  utilities are small ints (i32 here, Int in frozen — see the recorded bound in
  ARCHITECTURE), so reaching a 2^53 base would need ~2.1M satisfying bindings.
  A search of the world-realizable payoff space found NO discrimination at
  small base with acc a multiple of 0.5 and v an integer, and 4132
  discriminating sets once the NESTED 0.9 carries a full mantissa (at depth 2
  the inner score is itself `base1 + (acc1 + 0.9*base0)`). The canary pins the
  smallest: **base = 12, acc = 3.5 (>=3 movers summing to 7), v = 0.9 (depth-1
  = 0 + (0 + 0.9*1))** — the two associations land exactly 1 ULP apart
  (4625284074552279695 vs ...696). Base must be COMPARABLE TO `acc + 0.9v`,
  not large. Any world-driven corpus canary uses these payoffs. Also required:
  ≥3 movers, a Nothing-skip vs 0.0-term discriminator, and a 0.9-placement
  discriminator; no-FMA/contraction flags verified on BOTH the corpus generator build and the replay build; score channel pinned "as aeson/show emits" only as prose — the COMPARISON channel is raw bits [D-I1]: the oracle dumps castDoubleToWord64 (u64) per score; the replay compares u64 == to_bits. No decimal round-trip in the trusted base.
- **[S-I3] The gate's corpus fixtures, explicit**: the cone-mediated case (mover reads derived head H only; candidate inserts base B; extendDelta's fold adds H) AND the eviction-shadow (PraxEvicted) variant are named planner.json fixtures.
- **[S-I4 + D-I2] NO production reuse flag**: the `reuse: bool` driver switch is dead. The net is at the reuse SITE — debug_assert (test builds) that a reused step equals the live stepPredict when the gate fires — which both removes the prod hook and PROVES the generator reaches reuse (the assert's hit-count is asserted nonzero in the proptest).
- **[D-C1] planner.json emits scoreActions rows in NATIVE result order** — a stated exception to the oracle's sort-everything convention (the ordering IS the observable under test); the exception is scoped to the scored tables alone.
- **[D-I3] The minds split corrected**: prax-core::minds = believed_desires + cooked_self_wants + cooked_desires_for ONLY (what the planner imports); professed/conventional/want_for/self_wants → prax_vocab::minds (frozen library surface, MindsSpec pins land there).
- **[D-I4] intentions is Arc-wrapped**: `Arc<BTreeMap<String, Intention>>` with make_mut on write — planner forks never write intentions, so clone stays a refcount bump (no measure needed: forks-never-write makes Arc strictly dominant, not a tuning choice).
- **[D-completeness] Mandatory at S6**: the five villageWorld RelevanceSpec rows → KILLED deferral owed:S7 (else the gate fails at S6-DONE); npc.json gains a schedule-gated wake scenario; planner.json gains the cares_about wild-Call branch; the 4 owed:S6 LoopSpec intention pins are native-Rust re-expressions (not corpus items).
