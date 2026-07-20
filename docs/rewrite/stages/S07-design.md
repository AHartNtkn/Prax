# S7 design — Vertical world slices: the differential harness turns on (panel input; agent-side)

Frozen reference: the 19 content modules `src/Prax/{Core,Reactions,Emotion,Debt,Coerce,Blackmail,Confession,Beliefs,Witness,Rumor,Repute,Faction,Kin,Deceit,Persona,Project,Conversation,Arc,Deontic}.hs`, the worlds `src/Prax/Worlds/{Feud,Intrigue,Bar,Village}.hs`, `oracle/TraceMain.hs` (the permitted additive surface), and the world/vocab spec files. Builds on S1–S6. Scope: `rust/prax-vocab` (18 modules), `rust/prax-worlds` (4 worlds + the `dm` variant), `rust/prax-oracle` (THE COMPARATOR — the stage's real design work), `rust/conformance` (~25 new spec files, ~410 labels), and additive `oracle/` subcommands. NOT in scope: Script/Play/Audience (S8 — see [A1]), typeCheck/persist/stress/CLI (S9).

S1–S6 were verified against *fixtures*. S7 runs the two engines side by side on real content. The engineering is voluminous but mechanical; **the design work is the comparator**, whose job is to answer one question fast: *which went wrong — the port of the world, or the engine underneath it?*

## [A1] Stage-plan amendment: Audience moves to S8

`Prax.Worlds.Audience` imports `Prax.Script` — a Prompter-compiled world, not a combinator-composed one; its only non-Script vocab is `Prax.Core`, and it has **no spec file of its own** (its pins live in ScriptSpec, S8). Keeping it here drags the whole script layer (scene lowering, the `story` engine rule, the scenePatience/currentScene reserved families, the engine door) into slice 2. **Ruling**: S7 = **Feud → Intrigue → Bar → Village**; Audience joins Play at S8, which flips the (generic) comparator on for both script worlds at near-zero cost. PROGRAM.md's S7 row is edited to match; the S10 cut-over matrix is unchanged (6 worlds).

## 1. The comparator (`prax-oracle`, Rust bin)

### 1.1 Driving both engines
**Frozen**: `Command` on `cabal run -v0 prax-oracle -- <subcmd>` (overridable by `$PRAX_ORACLE_CMD`), streaming JSONL. `scripts/freeze-check.sh` runs FIRST on every invocation; a dirty frozen tree aborts before a record is produced. **Rust**: in-process; a walk driver reproduces `traceWalk`/`randWalk` step for step and emits the SAME record via one `record(&State, …) -> Value` builder (oracle canon: every list name-sorted; facts via `labeled_facts`). Comparison is `Value` vs `Value`, so field order cannot matter; the Rust side never round-trips its own JSON.
**The randtrace walk is comparator-owned**: the MMIX LCG, `pick`, the `ending.E` stop, the `passes > living` dead-end rule, and cap-decrements-only-on-action are transcribed into `prax-oracle/src/walk.rs` (S9's Stress port reuses it).
**Caching** (mandatory at 100+ seeds × 4 worlds): frozen JSONL memoized under `target/oracle-cache/<freeze-rev>/…`, keyed by the git rev of `haskell-freeze` so a stale cache cannot lie.

### 1.2 Comparison
Records compared in order; the run stops at the FIRST divergent record (the divergence of record) — the localization anchor. A shorter stream on either side is itself a divergence.

### 1.3 The classifier — five classes, ordered, each presupposing agreement above it

| Class | Fires when | Load-bearing evidence | Points at |
|---|---|---|---|
| **ENUMERATION** | `candidates` differ | candidate diff at t with pre-state facts equal | possible_actions ordering/filters — **or a world-port error** (§2) |
| **DECISION** | candidates equal, `action` differs | equal candidates + equal facts/rng/dues at t−1 | the planner: fold association, discounts, tiebreak, reuse gate, intention hold |
| **RNG** | action equal, `rng` differs | the engine Lehmer field; in randtrace, `walkSeed` with EQUAL candidate-list length | CRoll execution: taken/not, advance-on-miss, draw order in ForEach |
| **SCHEDULE** | action+rng equal, `boundary`/`dues`/`expiries` differ | the boundary flag + the two maps | boundary firing, re-arming, expiry arm/cancel/purge, v44 supersession |
| **STATE** | all above equal, `facts` differ | the fact-level path diff (§1.5) | perform semantics, spawn, ForEach snapshot, Call's base-db quirk, closure |

Two mis-classification hazards, mitigated: **(a) a view-only divergence is invisible in `state` mode** and surfaces a turn later as ENUMERATION/DECISION — on ANY divergence the localizer auto-reruns both in `--mode view` and, if views differ at t−1 while base dbs agree, reclassifies as `STATE(view)`. This is the DIV-1 shape and the single most valuable rule in the classifier. **(b) `walkSeed` divergence in randtrace is usually a symptom** (the pick index depends on `len(acts)`): differing `walkSeed` with differing candidate-list LENGTH is ENUMERATION, never RNG.
**The class is triage, not a verdict** — the artifact of record is the record PAIR plus the full field diff; the output says so.

### 1.4 Localization
On divergence at T, re-invoke both with richer emission truncated to T: `--candidates` (exists for randtrace; **S7 adds it to `trace`**) and **`--scores` (new)**: the acting character's `scoreActions` table at depths 0..D as `(label, castDoubleToWord64)` in native result order [D-C1]. Output: records side by side, differing fields, candidate diff at T and T−1, and for DECISION the score-table diff with the first bit-differing row.

### 1.5 The fact-level path diff
Both `facts` arrays are labeled sentences. Three buckets: `only_frozen` / `only_rust` (set differences, grouped by longest common SEGMENT PREFIX, rendered as a capped tree so one closure bug cannot emit 4,000 lines) and **`relabeled`** — same path, different operator (`x!y` vs `x.y`), a distinct bug class (exclusion semantics / ground round-trip) that must never be buried in the set differences. Family summary first, tree second.

### 1.6 Matrix mode
One line per (world, seed); worlds shape-checked (§2) before any seed runs. Trailing per-world counts of `clean | clean-mod-adjudicated | DIVERGENT | SHAPE-DIVERGENT`; nonzero exit on any DIVERGENT/SHAPE-DIVERGENT; `--jobs N` parallelizes over seeds (frozen invocations are the bottleneck; the cache is keyed, so it is safe).

### 1.7 The adjudicated-divergence register
`conformance/ADJUDICATED.json` (data, parsed by the comparator AND a conformance test). Entry: `{id, world, mode, class, fields, paths, seeds, from_turn, note}`. Three load-bearing laws:
1. **Per-field-difference suppression, never per-record.** A record is `clean-mod-adjudicated` only if EVERY differing field is covered, and for `facts` every differing PATH matches a pattern. One fresh path alongside an adjudicated one ⇒ DIVERGENT. Fresh signal is never drowned.
2. **No propagation.** An adjudicated difference TRUNCATES the comparison at that record; the comparator does not pretend to have compared the tail. A divergence whose consequences propagate is a fork question, not a suppression.
3. **Anti-drift gate.** A conformance test asserts a bijection between ADJUDICATED ids and DIVERGENCES.md `## DIV-n` headings declaring a suppression. Neither register grows without the other.
**The register ships EMPTY** — DIV-1 and DIV-2 need no suppression, and no-op entries would lie about the mechanism. Instead, GateSpec-tradition mutation evidence: a test-only fixture register + synthetic divergent record pairs proving (a) a covered field is suppressed, (b) a co-occurring uncovered path defeats suppression, (c) a class mismatch defeats suppression. A suppression mechanism nobody has seen discriminate is not a mechanism.

### 1.8 CLI + report consumption
`compare <world> --mode trace|randtrace …`, `explain <world> --at T`, `worldshape <world> [--check]`, `matrix [--worlds …] --seeds 0..99 --cap 50 [--jobs N] [--format report]`. Layout: `prax-oracle/src/{main,record,drive_frozen,drive_rust,walk,classify,diff,register,matrix,worldshape}.rs`. Reports embed `matrix --format report`'s per-world block VERBATIM — no hand-typed matrix numbers, ever.

## 2. World fidelity: the `worldshape` corpus — adjudicated YES, build it first

Worlds are authored DATA; a mis-transcribed label, swapped role, weight typo or dropped setup fact presents exactly like an engine divergence. **Ruling**: an additive `oracle/` subcommand `worldshape <world>` emitting canonical JSON per world, gating every trace. Two top-level keys so a shape mismatch reports differently from a body mismatch:
- `shape` — practice ids (name order) with name, roles in order, **action labels in declaration order**, data_facts, init-outcome sentences; character names in order with bound_to, want utilities in order, held desires; desire names; schedule rule names+periods; sorts; the die seed; **the setup db as labeled sentences** (setup ORDER is observable in the RESULT via `!` exclusions, so a set comparison suffices); axiom heads in order.
- `bodies` — every Condition/Outcome under a small **canonical encoder implemented on both sides** (`["Match","at.Actor!bar"]`, `["ForEach",[…],[…]]`). Haskell `show` vs Rust `Debug` must NEVER be the channel.
~80 LOC each side; every port error becomes a one-line structural diff before a turn runs; reused at S8 and S9. **Action LABELS are the fidelity crux** — the goldens are label sequences and the tiebreak is by label.

## 3. The vocab port pattern

18 modules, ~313 spec labels, pure value-builders over the S4 surface. **Signature-for-signature**, not idiomatic-Rust: one Rust fn per exported Haskell fn, snake_case, same parameter ORDER/arity/return; `&str` params (NOT `impl Into<String>` — vocab combinators take 4–6 string params and nest; the Into idiom stays on prax-core builders). **Port each module's own path helper; never replace it** (`belief_about`, `obligation_path`, `arc_sentence`, `talk_path`, `subject_of`, `punitive_prefix`) — inventing a nicer one de-syncs a call site and surfaces three slices later as a fact diff. Guards become `Result<_, WorldError>` exactly where the frozen calls `error`/`authoredVarClash`/`authoredPatClash`; slice step 0 is a grep enumerating them (known: Rumor, Deceit, Blackmail, Confession, Repute, Faction; Project's four `endeavor` arms; Confession's `segOk`). Pins: the module's spec file, ALL labels, one `// H:` each.

**The five that are NOT mechanical:**
1. **Coerce — `namespace_kernel`**: victim → PraxD; every other free var **in first-appearance order, excluding Owner** → PraxW, PraxW2, … (no PraxW1). `nub` is order-preserving dedup — a BTreeSet silently sorts and renames differently; verify `condition_vars`' per-constructor order against `conditionVars` first; the returned Desire's NAME feeds S9's CoercionUnmotivated lint.
2. **Witness — CoPresence/as_role**: `pub type CoPresence = Vec<Condition>` (alias, not newtype). `asRole` uses `groundCondition`, but Rust's needs `&mut Interner` and returns Result — infecting every downstream signature. **Ruling**: implement `as_role` with S4's `rename_vars` (pure, infallible, operator-preserving), value-identical for the single-segment substitution that is the only case arising; pin the equivalence via WitnessSpec + worldshape bodies.
3. **Project — `endeavor`**: generates two practices, a pursuit desire, per-part ledger keys, once-guards, dependency guards; generated **label order and guard order are golden-visible**. Transcribe literally, verify structurally via worldshape.
4. **Confession/Repute (and Faction/Kin/Persona) — axiom builders**: they change the VIEW, not just affordances. `incorrigible` builds a Count/Cmp threshold; `standingUnless`/`checkedDefeater` derive defeater head NAMES by string surgery over `subjectOf` — a one-character drift silently changes what the planner can read while rendering plausibly. **These are where DIV-1's shape lives** — §1.3's view-mode reclassification exists for them.
5. **Deontic's computing half**: `conflicts`/`incompatiblePairs`/`obligationsOf` run a scratch Db. `conflicts` mints a **private throwaway Interner** (no id escapes; insert_all + two exists), preserving the frozen signature; `obligations_of(interner, db, who)` takes the CALLER's interner and is the one place prax-vocab touches the engine at runtime — flagged in the crate doc so nobody "cleans it up".
Also: `Prax.Core.coreFns` ends in a `FnCase []` fallback and matching is FIRST-match — case order is observable.

## 4. The slices

| # | World | NEW vocab | First exercises (risk) | Differential milestone |
|---|---|---|---|---|
| 1 | **feud** | faction, kin | derive/Kin/Faction transitive rules at world scale; **defeasible retraction** ⇒ neg-footprint tier + full reclose on real data | worldshape; `trace --turns 24 --mode state`; randtrace **100 × 50** |
| 2 | **intrigue** | core, emotion, beliefs | defineFunctions + Call routing (BASE-db quirk, first-case/first-binding), desire-driven planning, **endings** | worldshape; **GoldenDrive intrigue 12**; trace 24; randtrace 100 × 50 |
| 3 | **bar** (+dm) | reactions, deontic (non-S4 exports), conversation, arc, witness | **widest integration**: schedule rules + sightRule, expiries + supersession, reaction SPAWN, Subquery/Count/Cmp/Calc, obligations/violations, the **director practice** (bound_to), TWO worlds from one module | worldshape ×2; **GoldenDrive bar 12**; **the LoopSpec 25-turn narration**; trace 40; randtrace **100 × 50 each** |
| 4 | **village** | project, witness (full), rumor, repute, deceit, persona, debt, coerce, blackmail, confession | **mechanism-dense**: seedDie + draw ⇒ first world exposure of the **CRoll stream**, coercion's rename kernel, confession/absolution axioms, obligedClose, the v37 gathering wake, endeavor part-sets, the sight window | worldshape; **GoldenDrive village 21 (idle=you)**; the v37-wake pin; trace 42; randtrace **300 × 50** + a **die-seed sweep** |

**Sequencing**: risk rises monotonically and each slice's new machinery is netted before the next can hide behind it. Feud has no schedule/functions/rng/desires — a divergence there is closure or enumeration, nothing else. Bar before Village because Bar's schedule/expiry/spawn load is a strict subset of Village's ambient load.
**Budget**: ≥100 × cap 50 is a FLOOR, scaled by mechanism density, normalized on **effective turns** (endings truncate walks — top up until ≥3,000 effective turns per world). **Gap the PLAN misses**: `randtrace --seed` is the WALK seed; the engine die seed is fixed inside villageWorld, so the Roll space is barely sampled. **Add `randtrace --die-seed S`** (additive) and sweep village over ≥20 die seeds × 20 walk seeds — without it the RNG class is nearly untested until S10, and RNG bugs are invisible in a golden.

## 5. Owed discharges, pins, allowlist

All 9 `owed: S7` rows discharge and are REMOVED: LoopSpec ×3 → slice 3 (bar: the 25-turn golden narration, the emergent+director outcomes, dead-character skip), LoopSpec v37-wake → slice 4, RelevanceSpec ×5 (all vs villageWorld) → slice 4. The four LoopSpec rows are native Rust re-expressions over `run_npc_ticks` AND cross-checked by `compare bar --mode trace` — note the shape difference (runNpcTicks omits idle lines; driveLabels emits `"<name>: -"`), so they are two nets, not one.
Allowlist growth per slice: (1) FeudSpec, KinSpec, FactionSpec; (2) IntrigueSpec, CoreSpec, EmotionSpec, BeliefsSpec; (3) BarSpec, DirectorSpec, ReactionsSpec, DeonticSpec, ConversationSpec, ArcSpec, WitnessSpec; (4) VillageSpec, ProjectSpec, RumorSpec, ReputeSpec, DeceitSpec, PersonaSpec, DebtSpec, CoerceSpec, BlackmailSpec, ConfessionSpec, **GoldenDriveSpec** (per-file; its three world labels span slices 2–4 but no stage boundary intervenes). ≈**410 labels at S7** — half the manifest. The comparator is the design; the pins are the grind.

## 6. The stop rule

On divergence the slice STOPS; no further vocab lands until adjudicated against the SPECS: (1) **worldshape first** — shape/bodies differ ⇒ port error, fix the world, no adjudication; (2) Rust bug ⇒ fix Rust; (3) frozen bug ⇒ Rust keeps the correct behavior, Haskell NEVER patched, numbered DIV entry, and *iff a shipped trace actually differs* an ADJUDICATED entry (the bijection gate); (4) ambiguous ⇒ FORK question, work proceeds on the specs-best-supported default.
**Mechanized discipline**: goldens are never re-captured. They are EXTRACTED from the frozen tree into `conformance/goldens/*.txt`, and `scripts/golden-check.sh` (CI, beside freeze-check) asserts each is byte-identical to the literal in the frozen spec files. Goldens become DERIVED from a tree that cannot be edited — "adjust the golden" is not a reachable move.

## 7. Panel charge

1. **Classifier precision**: are the boundaries and precedence right? Beyond the two named hazards — can SCHEDULE present as STATE (a due fires a boundary late; facts differ, dues agree at the sampled instant)? Construct the adversarial record pair for each ORDERED PAIR of classes; name the load-bearing evidence field; if a pair is indistinguishable from the record alone, the rerun must add the distinguishing field.
2. **The worldshape corpus**: attack cost/benefit and the canonical encoder; is the shape/bodies seam right? Does setup-db-as-set really cover setup ORDER, or is there a `!`-sequence whose intermediate state is observable later? Should worldshape also dump the S6 tables (discharging the five village RelevanceSpec rows structurally)?
3. **Slice ordering**: right risk ramp, or should `draw` be exercised earlier in a synthetic world so RNG is netted before the dense slice? Does anything in Bar depend on machinery only Village exercises?
4. **The budget**: is ≥100 × cap 50 the right floor and "effective turns" the right normalization? Is the die-seed sweep necessary now or an S10 concern? What actually determines coverage — seeds, cap, or branching factor?
5. **The register's laws**: does no-propagation make some legitimate frozen-bug adjudication un-encodable (local consequences that nonetheless propagate through one derived fact)? Empty-register-with-mutation-fixture vs no-op entries for discoverability?
6. **The non-mechanical five**: is `as_role` → `rename_vars` value-identical to `groundCondition` for every shipped CoPresence incl. Subquery binders? Is `conflicts`' throwaway interner sound (no id escape, no cross-lineage compare)?
7. **[A1] and the S8 boundary**: is moving Audience right, or should Prompter's compile half be pulled forward as S4 pulled Rng? Either way S8 inherits: the story rule + scenePatience/currentScene reserved families unregistered through all of S7 (no S7 world exercises `door::register_reserved_families` or the two-door collision), ScheduleRuleSpec's playWorld row still owed:S8, and worldshape's encoder meets script-COMPILED practices for the first time at S8 — does it assume authored provenance?
