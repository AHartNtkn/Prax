# S8 design — Script + Play + Audience: the scene layer, the engine door, the JSON format (panel input; agent-side)

Frozen reference: `src/Prax/Script.hs`, `src/Prax/Script/Json.hs`, `src/Prax/Worlds/{Play,Audience}.hs`, `test/Prax/{ScriptSpec,Script/JsonSpec}.hs`, `examples/play.json`, `app/Main.hs`'s `play`/`flow`/`dump-play` arms, `oracle/TraceMain.hs`'s `play`/`audience` entries (already present). Builds on S1–S7. Scope: `rust/prax-script` (script.rs AST + smart constructors, compile.rs, json.rs, flow chart), `rust/prax-worlds/src/{play,audience}.rs`, two entries in `prax-oracle/src/worlds.rs`, `rust/conformance/src/{script_spec,script_json_spec}.rs` (46 labels, 2 allowlist files). NOT in scope: `typeCheck`'s reserved-family verdict, `Persist`, `Inspect`, the CLI (all S9).

S7 was 410 labels of grind over a comparator that was the design. S8 is the inverse: 46 labels, no new vocab, and **three genuinely undecided things** — the door's semantics, the JSON's compatibility grade, and what a golden cannot see in a lowering. Audience arrives here by S7 [A1]; Play arrives with it, and both are compiled from data rather than composed from combinators, which is the whole reason the stage exists.

## 1. The scene layer

### 1.1 The AST — and the one thing the brief names that does not exist

`script.rs` mirrors `Script.hs` field-for-field: `Script{cast, scenes, start}`, `CastMember{name, playable, desires, traits}`, `Scene{id, opening, setup, beats, junctions}`, `Beat{label, speaker: Option<String>, when, effects}`, `Junction{name, to: Option<String>, when, after: Option<i64>}`. All `String`/`Vec`, no interning — this is the authoring family (ARCHITECTURE's stance); it converts only inside `compile`.

**[R1] There is no `Memory` type, and S8 must not invent one.** The memory feature was deleted end-to-end at spec v46. Its ONLY surviving trace is a deserializer guard: a scene object carrying a `"memories"` key fails LOUDLY with a message naming the feature (`Json.hs`'s `FromJSON Scene`), because aeson's `withObject` would otherwise drop it silently — the same "same bytes, different meaning" stance `Persist`'s v3 bump took. Port the guard, not the feature.

Smart constructors, signature-for-signature (S7 §3's rule): `member`, `player`, `wanting`, `concerned_with`, `with_traits`, `scene`, `beat`, `quip`, `goto`, `ending`, `after`, `timeout`. All INFALLIBLE (S4's builders-build-values rule) — and this is not a convenience: the frozen `goto` documents explicitly that its guard lives at `compile`, not at construction, *because a `Junction` can be built by raw constructor or by JSON decode*. Every validation is at the consumption point, uniformly over all three construction routes. Keep that seam exactly.

### 1.2 `compile` — what is mechanical

`pub fn compile(scr: &Script) -> Result<State, WorldError>`. Five guards, then a build, then a fold:

1. **Guards** (frozen order, preserved): duplicate junction names within one scene → `DuplicateJunctionName`; `after: Some(n)` with `n < 1` → `ZeroDelayJunction`; an authored sentence headed `scenePatience` in ANY of the five enumerated lists (scene setup, junction `when`, beat conditions, beat effects, cast-desire conditions) → `ReservedFamilyAuthored{site, sentence}`; a Prax-namespaced variable in a scene setup or a junction `when` (the v40 hygiene sites — these two splice into the story rule) → S4's `ReservedVarClash`. The five error strings are not pinned (the frozen suite only asserts `isLeft`), but the guard ORDER is observable through *which* error a doubly-offending script gets, and nothing frozen pins it → native pin (§7).
2. **Build**: `State::new()` → `define_practices([beats_p])` → `define_functions(core_fns())` → `set_characters(cast_chars)` → `door::register_reserved_families` → `door::register_engine_rules([story_rule])?` → fold `setup` through `perform_outcome`. The frozen nests these; the Rust API is `&mut` setters, so the nesting inverts to a statement sequence — the semantic order above is the frozen's, read outside-in.
3. **The beats practice**: `id "beats"`, `name "scene dialogue"`, `roles ["Stage"]`, actions = scenes in declaration order × that scene's beats in declaration order. Each compiled action: label = `bake_actor(speaker, label)`; `when` = `[Match "currentScene!<sid>", Match "character.Actor"] ++ [Eq "Actor" <spk>]? ++ beat.when`; `then` = beat.effects. Instance `practice.beats.stage` is inserted by the setup fold — one instance, `Stage` bound to the literal `stage`.
4. **The story rule**: `ScheduleRule "story"`, period 1, one clause per (scene, junction) in declaration × declaration order. Clause guard = `[Match "currentScene!<sid>", Absent [Match "ending.E"]] ++ junction.when ++ [Not "scenePatience.<sid>.<jname>"]?`; clause body = transition → `Insert "currentScene!<next>" : setup_of(next)`, ending → `[Insert "ending!<name>"]`.
5. **`setup_of(sid)`** = `[InsertFor n "scenePatience.<sid>.<j>"` for each timed junction in declaration order`]` **followed by** the scene's authored setup. Runs on all three entry paths (compile-time start, transition, re-entry) because every path threads through it.
6. **The compile-time setup fold**: `practice.beats.stage`, then `character.<c>` per cast member in order, then `trait.<c>.<t>`, then `currentScene!<start>`, then `setup_of(start)`.

The engine machinery this rides is already landed and already correct: S5's boundary fires expiries BEFORE rules (so a marker armed with lifetime `n` at entry is gone at boundary `n`, and the `Not` clause first becomes true exactly there), and fires a rule's clauses SEQUENTIALLY against the threading state, each as a `ForEach`. That threading is what makes the four frozen ordering pins pass; verify it at slice start rather than assuming it, because if clauses were re-queried against the boundary's start state, all four go red at once and the cause would look like a lowering bug.

### 1.3 What needs judgment

- **`bake_actor`.** Frozen: a hand-rolled scan replacing every `[Actor]` occurrence, never rescanning the inserted text. `str::replace` has the same semantics. Equivalent for every reachable input including a speaker name containing `[Actor]`. State it, pin it natively, do not "simplify" it into a `format!`.
- **`script_player`.** Frozen `error`s when no cast member is playable. Ruling: `Result<&str, WorldError::NoPlayableCastMember>` (S4's loud-error rule), consumed by the worlds' `PLAYER_NAME` at build, which `.expect()`.
- **`current_scene_of`.** `unify "currentScene.S"` and take the first solution. `currentScene` is written with `!`, so the slot is exclusive and there is at most one — the "first" is not an ordering commitment. Say so; do not port a `sort`.
- **Error surface vs `error`.** Every frozen `error` in Script.hs becomes a `WorldError` variant; nothing panics. `compile` is the outermost fallible thing in the crate.

### 1.4 The load-bearing orderings (three of four are golden-invisible)

- **O1 — compiled action-label order** (scene decl × beat decl, speakers baked). Feeds `possible_actions`' native order, hence the planner's full-tie fallback and randtrace's `pick` index. **No frozen pin sees it.** `worldshape.shape.action_labels` sees it — and that net evaporates at cut-over.
- **O2 — story-clause order** (scene decl × junction decl) and the eager forward-only fold. **Four frozen ScriptSpec pins see it directly.** The one ordering the frozen suite genuinely guards.
- **O3 — `setup_of`'s emission order**: patience markers BEFORE authored setup. Observable only if an authored setup outcome evicts or supersedes a marker path — which `compile` makes inexpressible (§5). Native pin anyway: it is a lowering contract, and the argument for its unobservability is the same argument that could be wrong tomorrow.
- **O4 — the compiled beat's `when` conjunct order**. Determines binding order at query time. `worldshape.bodies`-only.

O1/O3/O4 are exactly the S7 [C2] shape: *the only net is the shape gate, and the shape gate asserts equal-to-frozen and dies at deletion.* §7 is the response.

### 1.5 `flow_chart`

Pure string rendering; Mermaid `graph TD`. Mechanical. **But**: the frozen pin only asserts seven substrings appear, and `flow` output is a stated cut-over equality criterion — so pin the WHOLE rendered string for `play_script()` natively, not seven `contains` calls.

## 2. The engine door — S8 is the first caller

**[R2] The calls.** `door::register_reserved_families(&mut st, [SCENE_PATIENCE_FAMILY, CURRENT_SCENE_PATH])` then `door::register_engine_rules(&mut st, [story_rule])?`. Constants live in prax-script (S4 §4: one home per constant; prax-core cannot depend on prax-script). Reserved families FIRST, then the rule, then the setup fold. The relative order is unobservable at S8 — fixed by convention and stated, not discovered.

**[R3] The two doors get two disciplines, and the S4 panel's idempotence question closes HERE.** `register_reserved_families` is `extend` today — no dedup. `register_engine_rules` records names only after the duplicate guard passes, and is loud. That asymmetry is right and should be explicit:
- **Rule names are a keyed table** → duplicates are a COLLISION → loud `DuplicateScheduleRuleName`, both directions, both doors.
- **Reserved families are a SET** → registration is a monotone set-add → make the door IDEMPOTENT. An error would be the wrong loudness for adding a member to a set; the consumer (S9's checker) asks a membership question duplicates cannot change — but a duplicated list renders into diagnostics, and "the list is a set" is cheaper to guarantee at the door than to remember at every call site.
Today `compile` always starts from `State::new()`, so no shipped path calls the family door twice. The idempotence is for S9's persist/reload and any future re-compile; the pin is a direct double-call, not a world.

**[R4] The door has no frozen counterpart, and the differential must not pretend otherwise.** The frozen `TypeCheck` imports the two constants directly; there is no `registerReservedFamilies` in the frozen tree. The Rust door exists because the crate graph forbids prax-core → prax-script (the same cycle argument that split `obligedClose` at S4). Consequence: the reserved list is a Rust-only field. It goes in a NATIVE pin, NOT into `worldshape.shape` — a Rust-only key in a document whose whole job is cross-engine diffing is a lie waiting to be believed. **S8 owes prax-core a `State::reserved_families()` accessor**; S9's checker reads the state's own list through it.

**[R5] The two-door collision — the owed:S8 row.** Re-expresses as: build `play_world()`, then `set_schedule([ScheduleRule::new("story", 2)])` → `Err(DuplicateScheduleRuleName)`. Only that direction is reachable for a compiled world; the converse and the bare-state case are already pinned at S5. The pin's home stays `schedule_rule_spec.rs` (a `// H:` label is a claim about provenance). Provenance is also differential-visible if the frozen `worldshape` emits `engine_rule_names` — **verify at slice start**; if the frozen side omits it, extend `oracle/` rather than dropping the check.

## 3. The JSON format

**[R6] Byte-compatibility is REQUIRED, not optional.** `dump-play` prints `encodeScript Play.playScript`, and PLAN.md's cut-over criteria require `check/stress/flow/dump-play outputs equal`. Equal stdout is byte equality. Mechanism: aeson 2 with `ordered-keymap` emits objects with SORTED keys; `#[derive(Serialize)]` emits DECLARATION order and would not be byte-equal. **Ruling**: build `serde_json::Value` (whose `Map` is a `BTreeMap` without `preserve_order`), one function per frozen `object [...]`, including the conditional key omissions. Sorted keys then come free, and compact `to_string` is byte-comparable. This mirrors the frozen module's own hand-written-schema decision.

**[R7] What "loads unchanged" requires, precisely — six claims:**
1. `examples/play.json` is not edited. It is NOT under `src/ app/ test/`, so `freeze-check.sh` does not cover it, and "adjust the file until the decoder passes" is currently a reachable move — the exact failure [D-C3] closed for goldens. **Commit its SHA256 alongside `conformance/goldens/SHA256SUMS` and check it in CI.** The file survives the cut-over, so this is the only form of "unchanged" that still means something after deletion.
2. `decode_script(bytes)` succeeds. 3. The decoded AST `==` the Rust-built `play_script()`. 4. `worldshape(compile(decoded))` == `worldshape(compile(play_script()))` == the FROZEN `worldshape play`. 5. "Plays identically" = the differential clean. 6. `encode_script(&play_script())` is byte-identical to the file.

**Claim 6 must be MEASURED before it is pinned**: `cabal run prax -- dump-play | diff - examples/play.json`. If they differ, the shipped file is loadable-but-not-a-re-emission, and the ruling degrades to: pin the Rust encoder against the FROZEN `dump-play` bytes (via an additive `oracle/` subcommand), pin decode against the FILE, and record why the file is not its own re-emission. **Do not edit `examples/play.json` to make claim 6 true.**

**[R8] The decoder's alternative order is semantics.** aeson's `<|>` chain tries `match, not, eq, neq, cmp, calc, count, subquery, or, absent, exists` in that order, and a present-but-malformed payload FAILS THAT ALTERNATIVE AND FALLS THROUGH. Rust mirrors this as an ordered key probe with fall-through, not a match on "which key is present". Frozen has no pin → native pin.

**[R9] Unknown keys stay IGNORED except `memories`.** `deny_unknown_fields` would be a stricter contract smuggled in as hygiene: it would reject forward-compatible files the frozen accepts. Mirror exactly. If we want strictness it is an S10 fork question, not an S8 convenience.

## 4. The two worlds

`play_world() = compile(&play_script()).expect(...)`, likewise audience. **No new vocab** — `core_model` and `emotion` landed in S7 slices 2–3. That is [A1]'s dividend. Registry idlers must match the frozen pairing (`play` → `marcus`, `audience` → `envoy`), or the trace compares a different walk.

**The S7 panel's open question, answered by reading the encoder: `worldshape` assumes NO authored provenance.** Four specifics: `shape.practices` iterates `practice_defs()` (a script world has one, `beats`, whose labels are GENERATED — the encoder does not care, but a shape diff here means "the lowering is wrong" rather than "the transcription is wrong"); `bodies.schedule` encodes compiler-generated conditions through the same total encoder; **`check_setup_rolls_zero` is structurally blind here** (it scans practice INIT outcomes; a script's setup is a compile-time fold the state does not retain) — so assert the guarantee we actually have: **a script world's `rng_seed` is `None`, so any setup Roll would have PANICKED at compile**; and **`shape.state` carries a NON-EMPTY expiries map at t=0 for `audience`** — the first shipped world that does, so [D-I5]'s initial-expiries addition earns its keep here.

## 5. The differential

Order: both `worldshape --check` first ([S-I2]), then traces, then the matrix. `compare play --mode trace --turns 24` (idler `marcus`); `compare audience --mode trace --turns 12` (idler `envoy`). Both streams TERMINATE on `ending.E` — short by construction, and the TERMINATION rung is the one most likely to fire first on a lowering bug, since a wrong clause order changes *which* ending and a wrong marker arms the wrong boundary. Then `matrix --worlds play,audience` with the distinct-walk criterion and provenance tags.

**Predictions, stated so they can be wrong**: audience — **low single digits to low teens** distinct walks, budget stop = SATURATION; both endings (`granted`, `dismissed`) must appear. play — **≥3 and probably under 15**; saturation again. **What the numbers MEAN**: a low count here is a property of a two-scene script whose junctions END the story, not a defect of the sweep — the report must say so on its face. **The falsifiable one**: the walk identity includes the terminal stop, so play's three endings imply **at least three distinct walks**; fewer is a finding about the walk driver or the world, not a seed-range knob.

**CG-1: S8 must NOT claim to close it.** The only timer in the script machinery is the compiler-armed patience marker, and `compile` REFUSES any authored `scenePatience` sentence in all five lists — so a bare insert onto a live script timer is INEXPRESSIBLE in a script. But S8 contributes one honest line to CG-1's "what covers it": **the REFRESH half of the v44 law gets its first AUTHORED-data, world-scale exercise here** (scene re-entry re-arms through `setup_of`, reached by an authored `goto`). The SUPERSESSION half stays open — record the strengthening; do not let it read as closure.

## 6. The pins

- **Allowlist +2 files, 46 labels**: `ScriptSpec.hs` (34) and `Script/JsonSpec.hs` (12), group labels included.
- **Owed:S8 discharge — exactly one row**: ScheduleRuleSpec's story-collision, re-expressed per [R5] and REMOVED.
- **KILLED rows S8 must WRITE (owed: S9), one**: ScriptSpec's mid-scene save/resume persistence-symmetry pin — `Persist` has no Rust twin until S9. The timing behavior itself is pinned at S8 without persistence. Category `deferral`, owed `S9`, in the house's loud style.
- **Native pins owed** (each verified to REDDEN under its own named mutation): O1, O3, O4, the compile guard ORDER, `bake_actor` semantics, the JSON key-probe order, `flow_chart`'s exact string, `reserved_families` content + door idempotence, the `rng_seed == None` script arm.
- **Every native pin says in the file why it carries no frozen label** (the `coerce.rs` precedent S7 §14 named as the pattern `endeavor` failed to follow).

## 7. The cut-over-relevant audit — S8 starts it, for its own surface

S7's standing lesson, twice earned: **a net that only asserts equal-to-frozen EVAPORATES at deletion.** S8 owes an audit table in its evidence report, one row per surface:

| surface | nets today | survives? | action |
|---|---|---|---|
| O1 compiled action-label order | `worldshape.shape` only | **no** | native pin |
| O3 `setup_of` emission order | `worldshape.bodies` only | **no** | native pin |
| O4 beat `when` conjunct order | `worldshape.bodies` only | **no** | native pin |
| story rule name/period | shape + 4 ScriptSpec pins | yes | none |
| initial expiries (audience) | shape + the boundary-5 pin | yes | none |
| `flow_chart` output | 7 substrings + CLI equality (S9) | partially | full-string native pin |
| JSON encoder bytes | `dump-play` equality (S9/S10) | **no** — the frozen encoder dies | pin against the FILE + committed SHA256 |
| reserved-families list | nothing (Rust-only field) | n/a | native pin [R4] |

**The JSON pins must be FILE-driven, not frozen-encoder-driven**, because `examples/play.json` survives the cut-over and the frozen encoder does not — the same retargeting [D-C3] designed for goldens, applied to the one data file the PLAN promises to keep.

## 8. Panel charge

1. **Door semantics.** Is [R3]'s asymmetry right — rules loud, families idempotent — or should the family door also be loud? Is idempotence-at-the-door the right home versus a `BTreeSet` field, given persist/reload at S9? Does [R4]'s "Rust-only field stays out of worldshape" hold? Construct the state where door order becomes observable — if none exists at S8, does one exist at S9?
2. **The JSON compatibility ruling.** Is [R6] right that `dump-play` equality FORCES byte-compatibility, or is it satisfiable at S9 by a normalizing dump? If `dump-play` ≠ the shipped file (unmeasured), which of [R7]'s six claims is the pin? Attack [R8]: is fall-through-on-malformed faithfully reproducible without aeson's `MonadFail`-in-`Parser`, and is there an input where the two differ? Is [R9] the right fidelity call, or is the frozen behavior a bug the program's ruling says we should fix?
3. **Compiled practices × worldshape.** Does the encoder really carry no provenance assumption? Does `shape.setup_db` as a SET still suffice for a world whose setup is a fold with an `InsertFor` in it ([S-C6] killed the set claim once; the carve-out was a setup that consumes the DIE, and this consumes a TIMER)? Does `rng_seed == None` guarantee what the die-scan guaranteed?
4. **The ordering claims.** Are O1–O4 complete? Is the compiled-beat `when` order observable in enumeration order for any shipped script? Is the guard-order claim correct against the frozen guard chain?
5. **Faithfulness where no golden can see it** (S7 [C2] applied prospectively). For each of O1/O3/O4 and the guard order: what mutation would a frozen pin catch, and what would ONLY `worldshape` catch? Is §7's row set complete, and does each proposed native pin actually redden under a nameable mutation?
6. **The distinct-walk predictions.** Plausible? Is "at least three for play" the right falsifiable claim? Should the walk identity for script worlds include the SCENE PATH as well as the action sequence and terminal stop?
7. **CG-1 hygiene.** Is the REFRESH strengthening real or a restatement? Is there ANY authored route in a script — including through JSON, whose `Outcome` surface is wider than the smart constructors' — to a bare insert onto a live timer that `compile`'s five-list sweep misses? (An enumeration is exactly the kind of thing that goes stale.)
8. **Scope honesty.** Is anything in the frozen `play` loop that S8's library surface must provide being quietly deferred to S9 without a KILLED row?
