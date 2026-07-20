# S3 design — Derive + the closed view (panel input; agent-side)

Frozen reference: src/Prax/Derive.hs (+ docs/specs/2026-07-12-v27*, the v33
continuation notes in LEDGER, v51's lifting ruling). Builds on S1 (Db/EL) and
S2 (Cond/query). Scope: rust/prax-core/derive.rs + testkit's naive oracle +
the view-invariant property. NOT in scope: the three-tier delta ROUTING and
reclose/state (S4 — engine); this stage delivers the closure functions S4
routes to.

## Design

1. ONE implementation: `close(rules: &[CompiledRule], base: &Db, interner:
   &mut Interner) -> Result<Db, Contradiction>` — semi-naive forward chaining:
   delta starts as the whole base; each round fires every rule through
   delta-join (for each positive Match position i, evaluate the body with
   position i against DELTA and every other condition against MODEL; union
   over i; dedup); ground heads; keep the not-entailed fresh ones; meet each
   into the model (⊥ propagates as Err(Contradiction(head sentence)));
   delta := the fresh facts; stop when none. The frozen run/runCooked twins
   collapse; closure/closureFrom's callers become this.
2. `CompiledRule { body: Vec<Cond>, heads: Vec<CompiledPath> }` — heads may
   carry variables; grounding a head = substitute bindings (S2's ground path)
   into a token list; heads bind only separator-free values (the frozen
   groundTokens invariant — stated as a debug_assert + a law test, since the
   engine's binding sources guarantee it).
3. `close_from(model: &Db, delta: &Db, rules, interner) -> Result<Db,
   Contradiction>` — the v33 continuation: continue an ALREADY-CLOSED model
   with a monotone delta; the monotone-only obligation is the CALLER's (S4's
   router proves it via contMonotone + the insert kind) and is stated on the
   function + guarded by the view-invariant property in tests.
4. `entailed`/head-meet WITHOUT the intermediate singleton-db allocation the
   frozen code pays (`insertToks h emptyDb` per head): walk the head path
   directly against the model (leq of a path against a trie is a straight
   descent; meet of a path into a model is insert-with-⊥-detection — an
   exclusive node receiving a SECOND distinct child is the ⊥ case; the
   asserted flag joins as OR). Must be semantics-identical to
   meet(model, singleton(h)) — pinned by a law test comparing both forms on
   generated cases (the singleton form via S1's public meet).
5. NO lifting anywhere in derive (v51: the axiom list arrives closure-included
   from the world's own declaration).
6. Determinism: the RESULT model is join-order independent (meet is
   commutative/associative — S1's law tests); iteration order affects only
   work done. Fresh-fact dedup via a name-independent structural set
   (CompiledPath Eq/Hash on Syms is fine INTERNALLY — ids never escape).
7. testkit: `naive_closure(axioms_src: &[Axiom-authoring], base) -> ...` —
   compiles each authored axiom freshly through the public door per call,
   full-queries (no delta), loops to fixpoint. Shares ONLY meet/leq/trie with
   production. (The Axiom AUTHORING type is S4's; for S3 the oracle takes
   pre-compiled rules built through a separate fresh compile call — the
   sharing-nothing property is the point, not the input type; restate at S4.)
8. Properties: naive == production on generated rule/base sets (bounded:
   ≤4 rules, ≤3 body conds, ≤8 base facts — the state space that matters);
   close_from(close(base), monotone-delta) == close(base+delta) (the
   continuation soundness law); idempotence close(close(m)) == close(m);
   ⊥ cases (a rule head forcing an exclusive second child) surface from
   either join order. Plus the derive.json fixture replay (feud 9→9,
   village 9→12 — real shipped closures) byte-for-byte.

## Panel charge

Soundness: is the delta-join exact vs Derive.hs:120-128 (the per-position
seeding, the nub timing, entailed-filter placement — an error here silently
under- or over-derives on worlds with recursive rules like Kin's)? Is the
direct head-meet provably equivalent to the singleton meet (the asserted-OR
and excl-flag corners)? Is close_from's stated obligation sufficient (what
EXACTLY breaks if a non-monotone delta sneaks in — is the failure loud)?
Design: is collapsing closure/closureFrom's callers right; is the oracle's
sharing-nothing claim honest given it uses the same trie; anything simpler?
Completeness: which DeriveSpec/ViewInvariantSpec pins map where; the
Contradiction sentence rendering (label-faithful?); Kin/Repute/Faction
(S7 consumers) recursive-closure patterns that S3's generators must cover.
