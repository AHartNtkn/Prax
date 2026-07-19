# Rust engine architecture — agent-side decision record

Not a user-facing document (the program plan's ruling); this is the shared
contract for implementers, reviewers, and panels. Decisions here were produced
by the program's design phase and are revised only through stage panels, with
revisions recorded in place.

## Stance

One representation, one implementation, no global state. The authoring AST
(string-surfaced) and the runtime types (interned, pre-split) are DIFFERENT
TYPE FAMILIES; only the install-time compiler converts. The v28 string war
cannot recur because the un-compiled path does not type-check into the engine.
Strings live at exactly three boundaries: authoring, rendering, persistence.

## Crates (workspace at rust/)

prax-core (engine) · prax-vocab (20 content combinator modules — pure value
builders) · prax-script (Prompter layer + serde format) · prax-worlds ·
prax-cli · prax-oracle (comparator) · conformance (cross-module tests, one
file per Haskell spec file). The script layer's engine-rule door
(registerEngineRules, v53 provenance) is sealed: reachable by prax-script,
not by the public authoring surface.

## Core types (prax-core)

- `Sym(u32)`, bit 0 = is-variable (v29 parity trick kept). `Interner` owned,
  `Arc<Interner>` inside state; append-only; ids never observable — all
  output through resolve, all observable orders sort BY NAME.
- `CompiledPath { segs: SmallVec<[Sym;6]>, excl: u32 }` — excl bit i = the
  separator after seg i is `!`. One tokenizer at the authoring boundary
  (trailing-operator rejection kept).
- `Db(Arc<Node>)`, `Node { excl: bool, asserted: bool, kids: SmallVec<[(Sym,
  Db);4]> sorted by id }` — persistence by Arc::make_mut path-copy; clone =
  refcount bump (the planner's apply-and-discard depends on this). Corrected
  `!` insert (clear siblings, keep survivor's subtree) and the v39
  asserted-flag law (no unasserted childless node; eager prune) verbatim.
  Deterministic-enumeration contract: name-order at unify's unbound branch,
  child_keys, sentence enumeration, expiry firing.
- `el`: meet/leq as sorted-merge walks; ⊥ = exclusive node forced to two
  children; assertedness ORs.
- Authoring family: `Condition/Outcome/Action/Practice/Axiom/ScheduleRule/
  Function` with String paths + builder functions (`matches/not_/eq/insert/
  ...`; `Action::new(label).when([...]).then([...])`). Runtime family:
  `Cond/Effect` over `CompiledPath`/`Sym`. ONE evaluator (`query`), ONE
  performer, ONE closure implementation.
- `Bindings`: sorted `SmallVec<[(Sym, Val);8]>`; `Val = Sym | Num(i64) |
  Set(...)`. i64 replaces Integer with checked arithmetic (loud overflow —
  recorded deviation).
- `State { defs: Arc<Defs>, rt: Runtime }`. Defs: authored sources retained
  (typecheck/persist/diagnostics) + compiled forms + derived analysis tables
  (footprint, axiom_heads, neg_footprint, cont_monotone, improvables,
  liveness, cares_about) — rebuilt wholesale by ONE compile choke point
  (compilepipe::recompile, the retable heir) invoked by every setter.
  Runtime: db, view (closure cache), cursor, intentions, schedule_dues,
  expiries, rng_seed — plain clone, tries share structurally.
  db/view private; writers are engine::perform (three-tier delta routing:
  irrelevant → apply_direct; monotone insert → closure continuation; else
  full reclose) and with_db. The view invariant is structural.
- Errors: construction guards `Result<_, WorldError>` (thiserror; loud,
  #[must_use], full diagnostics — worlds .expect() at build); engine
  invariant breaches panic; contradiction = queryable fact, never an error.
- Scores: i32 utilities × f64 0.9/0.5 discounts, accumulation order PINNED
  (mirrors the Haskell fold order → bit-exact decision parity; ordering is
  the contract, decimals never pinned).
- RNG: MINSTD bit-exact in i64. Persist: serde JSON, versioned, facts as
  labeled sentences; loud version rejection.

## Verification machinery (in-crate)

- testkit (feature): naive_closure oracle (~60 lines, shares only meet/leq +
  trie with production) asserted view == naive_closure(axioms_src, db) after
  every mutation in randomized + golden sequences — the ViewInvariant heir.
- proptest law suites: trie laws (asserted-flag walker, `!` supersession,
  insert/retract round-trips), EL lattice laws, query compiled==fresh-parse,
  v44 expiry laws, persist round-trip.
- Every re-expressed test carries `// H: <SpecFile> "<label>"`; the meta-gate
  test audits all 849 manifest labels against these + KILLED.md.

## Deps

smallvec, rustc-hash, thiserror, serde(+json); dev: proptest. Nothing else
without a stage-panel decision.

## Banked levers (measure first)

Arena trie + undo log (if Arc path-copy dominates profiles); fixed-point
scores (if f64 platform issues appear); im-style maps for intentions/expiries
(if a world holds thousands).
