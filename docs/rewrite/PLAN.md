# Plan: The Rust rewrite (R-series)

## Context

Replace the Haskell Prax implementation entirely with Rust. The DESIGN is the
contract (the five semantic invariants, the specs, the LEDGER), not the Haskell:
only load-bearing semantics survive; Haskell accidents die. Per the user's
process ruling: no architecture documents for human review — semantic fidelity
is verified MECHANICALLY (differential testing against the frozen Haskell + a
1:1-audited re-expressed pin corpus), engineering quality is agent-owned under
adversarial review, and the user sees only one-screen evidence reports
(informational), specific semantic-fork questions, and the final playable demo
(the one approval in the program).

## Settled decisions (user)

- Haskell frozen in-tree until parity, then deleted in one commit.
- Rust builder DSL is the authoring surface.
- Full per-stage rigor (panels/reviews), but gates replaced by mechanical
  verification per the ruling above.

## What the user will see (the whole surface)

1. `docs/rewrite/PROGRAM.md` — living status table (stage | state | report).
2. One-screen evidence reports per stage (`docs/rewrite/reports/S*.md`):
   pins green / killed counts, differential matrix lines, clippy/proptest,
   perf, open forks. No approval semantics.
3. FORK questions (rare): one paragraph per genuinely ambiguous case — the
   specs silent or self-conflicting on an observable behavior. Options stated
   with fiction-level consequences; the default is whatever the specs' intent
   best supports (Haskell's behavior is evidence, never the default by
   virtue of incumbency); work proceeds on the default until answered.
4. The cut-over demo: playing village and bar on Rust `prax play` (with
   save/resume) and saying go.

## Architecture (agent-owned; condensed decision record)

Cargo workspace, edition 2024: `prax-core` (engine), `prax-vocab` (the 20
content combinator modules — pure value-builders, the durable social
vocabulary), `prax-script` (Prompter layer + serde format), `prax-worlds`
(the 6 worlds), `prax-cli`, `prax-oracle` (differential comparator),
`conformance` (cross-module + world tests, one file per Haskell spec file).

Core decisions: ONE compiled representation — the authoring AST
(string-surfaced `Condition`/`Outcome`/... builders) is a separate type family
from the runtime types (`Cond`/`Effect`/`CompiledPath` with interned `Sym`s),
converted only at install by one compile choke point (the retable heir); the
entire raw/cooked mirror duality, the twin implementations, and the global
`unsafePerformIO` intern pool die (interner becomes `Arc<Interner>` in state;
var-bit parity trick kept). Db: exclusion trie as `Arc` path-copy persistent
nodes (sorted `SmallVec` children), preserving the planner's apply-and-discard
clone model; corrected `!` semantics + v39 asserted-flag law verbatim;
determinism contract written down (name-order at every enumeration point).
Errors: construction guards become `Result<_, WorldError>` (thiserror; loud),
engine-invariant breaches panic, contradiction stays a queryable fact.
Scores: i32 utilities × f64 0.9/0.5 discounts with PINNED accumulation order
(bit-exact cross-language; ordering is the contract, decimal pins die).
`i64` replaces `Integer` with checked arithmetic (recorded deviation).
MINSTD RNG bit-exact. Deps: smallvec, rustc-hash, thiserror, serde(+json),
proptest (dev). The ViewInvariant dual-path net's heir: a test-only naive
closure oracle (~60 lines, shares nothing with the production loop) asserted
after every mutation in randomized+golden sequences, plus proptest law suites
(trie/EL/query/expiry/persist laws).

## Verification (replaces user gates)

- **The freeze, precisely**: `src/ app/ test/` tagged `haskell-freeze`, never
  edited again — mechanically enforced (the comparator refuses to run if
  `git diff --quiet haskell-freeze -- src app test` fails). ONE additive
  surface: a new `oracle/` cabal executable (~250 LOC) importing the frozen
  library — trace/randtrace/check/fixtures subcommands emitting canonical
  JSONL (per-turn: actor, action, boundary, cursor, rng, dues, expiries,
  sorted fact list via dbToLabeledSentences). Zero library edits needed.
- **Divergence adjudication (the user's ruling — the Rust must be RIGHT, not
  Haskell-equal)**: a divergence is a signal, not automatically a failure.
  Each one is adjudicated against the SPECS (the authority; the Haskell is
  evidence, never precedent): (a) Rust bug → fix Rust; (b) Haskell bug —
  behavior contradicting its own spec/LEDGER record — → the Rust keeps the
  CORRECT behavior, the Haskell is never patched, and the fix is recorded in
  `docs/rewrite/DIVERGENCES.md` (what, why the spec says Rust is right,
  fiction consequence) and registered with the comparator as an ADJUDICATED
  divergence (a precise suppression: world/class/path pattern) so the matrix
  reads "clean modulo adjudicated fixes" and fresh signal is never drowned;
  (c) genuinely ambiguous (specs silent or self-conflicting) → fork question
  to the user. Implementation bugs are NEVER reproduced.
- **The comparator** (`prax-oracle` Rust bin): record-by-record state
  comparison; on divergence, auto-rerun with candidate lists and classify
  (ENUMERATION | DECISION | STATE | SCHEDULE | RNG) + fact-level path diff;
  consults the adjudicated-divergences register. Matrix mode: one line per
  (world, seed), `clean | clean-mod-adjudicated | DIVERGENT`.
- **The meta-gate**: every re-expressed Rust test carries `// H: <SpecFile>
  "<label>"`; a conformance test parses the Haskell spec labels (from the
  tree now, a committed manifest after deletion) and asserts each of the
  ~760 labels appears in exactly one `// H:` comment OR one KILLED.md row
  (category: decimal | implementation | haskell-only + one-line reason).
  Pin accounting is a red/green test, not a claim.
- Golden decision-traces: committed as Rust baselines AND differentially
  cross-checked; never edited to match new behavior.

## Stages (each = numbered R-series rounds, house cadence, LEDGER row each)

S0 harness (freeze tag, oracle/ exe, fixture dump, workspace scaffold, CI both suites) — 5%
S1 Sym+Db+EL (DbSpec/ELSpec/SymSpec-semantics; fixture replay) — 10%
S2 Query, one compiled path (QuerySpec + CookedSpec's observable half) — 8%
S3 Derive + view, incrementality kept (v27/v33 perf history), ViewInvariant
   reborn as incremental==naive proptest — DESIGN-HEAVY — 8%
S4 Types+Engine+builder API (the make-or-break authoring surface; EngineSpec,
   RngSpec guards, GateSpec hygiene half) — DESIGN-HEAVY — 15%
S5 Loop+Schedule+Rng (LoopSpec advance, Schedule*, MINSTD stream) — 5%
S6 Planner+Minds+Relevance+Sight (discount order pinned, tiebreak, v34 reuse
   with reuse==live proptest, v35 intentions; decimal→ordering pins) —
   DESIGN-HEAVY, the fidelity summit — 15%
S7 Vertical world slices Feud→Audience→Intrigue→Bar→Village, each slice =
   needed vocab modules + world + DIFFERENTIAL ON (trace + randtrace ≥100
   seeds × cap 50, state mode) — 20%
S8 Script+Play (JSON round-trip; examples/play.json loads unchanged) — 5%
S9 TypeCheck+AnalysisTable+Stress+Persist(new serde format, no cross-engine
   save compat)+Inspect+CLI; GateSpec scanner retargeted at .rs — 6%
S10 Hardening + cut-over (matrix 6 worlds × 500 seeds, perf table, demo) — 3%

Design-heavy stages (S3/S4/S6) get adversarial panels BEFORE dependents start.

## Cut-over (all mechanical except the demo)

Conformance green + meta-gate green; differential matrix clean (500 seeds ×
cap 50 per world, state mode; check/stress/flow/dump-play outputs equal);
clippy -D warnings, zero unsafe, proptest soak; perf table Rust ≥ Haskell;
the user plays village+bar on Rust and says go. Deletion commit (after tag
`haskell-final`): src/, test/, app/, oracle/, prax.cabal, cabal.project,
Haskell CI die; cargo workspace promotes to repo root; docs/ + references/ +
examples/play.json survive in full; trace/replay tooling survives as
Rust-vs-own-goldens QA.

## First executable step

S0: tag `haskell-freeze`; write `oracle/TraceMain.hs` + the one cabal stanza;
dump + commit the unit fixture corpora (db/EL/query/derive tables); scaffold
the cargo workspace; wire CI to run both suites + the freeze check.

---
