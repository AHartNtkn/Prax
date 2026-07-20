# S3 design — Derive + the closed view (panel input; agent-side)

Frozen reference: src/Prax/Derive.hs (+ docs/specs/2026-07-12-v27*, v33 LEDGER
notes, v51's lifting ruling). Builds on S1 (Db/EL) and S2 (Cond/query). Scope:
rust/prax-core/derive.rs + testkit's naive oracle + the view-invariant
property. NOT in scope: the three-tier delta ROUTING and reclose/state (S4).

REWRITTEN after the two-lens panel (.superpowers/sdd/rs-s3-panel-*.md): the
soundness lens PROVED (live probes against the frozen library) that the frozen
deltaJoin is INCOMPLETE on a reachable fragment, forcing the program's first
divergence adjudication [S-C1]; the design lens killed the premature head-meet
walk [D-I3], surfaced the missing ground-to-path primitive [D-I2], and gated
the property laws [D-C1, D-C2]. All findings folded.

## The adjudication [S-C1 — the program's first DIVERGENCES.md entry]

The frozen semi-naive deltaJoin seeds only top-level Match positions from the
delta (Derive.hs:128). A rule whose OTHER conditions (Exists/Subquery/Count/
Or) read a DERIVED predicate disjoint from its Match therefore never re-fires
once the Match fact leaves the delta — proven: `[Match p.X, Exists[Match q.Y]]
→ r.X` with `[Match trigger] → q.thing` drops r.a in the frozen engine while
the naive closure derives it. This CONTRADICTS the design of record: the view
"IS the closure of the base under the axioms" (ViewInvariantSpec's own
statement), and the frozen code documents semi-naive as an OPTIMIZATION
("nothing already known is re-derived") — an optimization that changes results
is a bug. Shipped worlds are unaffected (their aggregate conditions read the
same predicate as their Match — shape luck, now stated). RULING (per the
program's divergence protocol; spec authority, no user fork needed —
fiction-invisible in every shipped world): **the NAIVE CLOSURE IS THE
SEMANTICS; Rust production must equal it everywhere.** The Haskell is not
patched; DIVERGENCES.md gains the entry; no comparator suppression is needed
(shipped traces agree).

De facto semantics for non-monotone bodies, now stated rather than lucked:
facts only ACCUMULATE during closure (meet never retracts), so a Not/Absent
condition can only turn off over rounds and a fired head stays; the result is
deterministic GIVEN rule declaration order and round structure — rounds fire
rules in declaration order until no fresh facts. Naive and production both
implement exactly this; naive==production is achievable deterministically and
is the flagship law.

## Design

1. ONE implementation, CORRECT delta optimization: rules split STATICALLY at
   compile into
   - FAST-PATH rules: bodies of Match + pure binding filters only (Eq/Neq/
     Cmp/Calc — they read bindings, not the db). Delta-seeded per the frozen
     per-Match-position join (position i from delta, others from model, union
     over i, dedup; a fast-path rule always has ≥1 Match — a filter-only body
     is degenerate and evaluates on the model [S-I2's branch, now explicit:
     rules with NO Match position evaluate their whole body against the model
     EVERY round]).
   - FULL-EVAL rules: any body containing Exists/Absent/Not/Or/Subquery/Count
     — re-evaluated against the FULL model every round. This is what makes
     the optimization an optimization (identical results to naive, proven by
     the flagship law) instead of the frozen bug.
   `close(rules, base, interner) -> Result<Db, Contradiction>`.
2. Head grounding needs a primitive S3 OWNS [D-I2]: `ground_head(head:
   &CompiledPath, b: &Bindings) -> CompiledPath` — substitute bound values
   into variable segments producing a COMPILED path (no string round-trip;
   val_to_sym for non-Sym values). Heads bind only separator-free values
   (debug_assert + law test).
3. `close_from(model, delta, rules, interner)` — the v33 continuation for
   S4's monotone tier. HONESTY [S-I4]: a non-monotone delta does NOT fail
   loudly here — a stale derived fact survives silently; the guard is S4's
   router (contMonotone × insert kind) plus the S4/S7 full-recompute
   invariant checks, and THIS function's doc says exactly that. S3's own net:
   the continuation law property, gated to monotone shapes [D-C1].
4. Head-meet: plain `meet(model, singleton(head))` via S1's public lattice —
   MEET semantics (⊥ on a second distinct exclusive child; NO sibling
   clearing — insert semantics would be wrong here [S-I3]). The direct-walk
   optimization is BANKED (measure first [D-I3]). The law test's generator
   must hit the `x!b`-into-`{x.a}` conflict corner and the asserted-endpoint
   corner explicitly [S-I3].
5. NO lifting in derive (v51).
6. Determinism: fresh facts are deduped structurally but SORTED BY RENDERED
   NAME before meeting [D-I4] — the ⊥ witness (which head renders into the
   Contradiction) is thereby deterministic and name-ordered, matching the
   naive oracle; Contradiction renders label-faithfully (`!`/`.` preserved,
   tokensToSentence semantics [S-M3]).
7. testkit oracle: `naive_closure(rules: &[CompiledRule], base, interner)` —
   full-query every rule every round, no delta machinery, no static split;
   shares trie/meet/query with production (STATED honestly [S-M2]: the net
   catches closure-strategy bugs, not substrate bugs — the substrate has its
   own S1/S2 law suites). Same input type as production; no S4 restatement
   needed [D-3 resolved].
8. Properties: **naive == production** on generated rule/base sets — the
   generator MUST cover [D-C2, S-C1]: multi-Match bodies, aggregate bodies
   (Match+Subquery+Count+Cmp — the shipped 4-cond shape), Exists/Not bodies
   incl. reading DERIVED predicates (the frozen bug's fragment — this is
   where Rust must beat the frozen engine), no-Match bodies, multi-head
   rules, recursive rules (Kin shape). Continuation law gated to monotone
   generators [D-C1]. Idempotence. ⊥ from either join order (up-to-Err
   comparison [D-I4]). Singleton-meet corner laws [S-I3].
9. Fixture replay: derive.json (actual committed counts: feud 7→16, village
   22→34 [D-M1 — the note's earlier counts were written from memory, the
   recorded failure class]) byte-for-byte; PLUS a new recursive-closure
   fixture (Kin's axioms) dumped via the oracle exe before implementation
   [D-2 — oracle/ is the permitted surface; extend `fixtures` with a `kin`
   corpus]. The frozen-bug fragment gets a NEGATIVE fixture: the probe's
   scenario recorded with BOTH the frozen output and the correct output, so
   the divergence is a committed artifact, not prose.
10. Pin re-points [D-I1]: DeriveSpec's obligedClose/footprint/negPatterns/
    monotone pins land at S4 (allowlist deferral, made loud in KILLED.md the
    S2-M3 way); ViewInvariantSpec's world-turn shard lands S4/S7; S3 takes
    DeriveSpec's closure-semantics pins + the property laws.
