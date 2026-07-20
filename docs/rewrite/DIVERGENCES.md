# Adjudicated divergences — where the Rust is right and the Haskell is not

Per the program's ruling (docs/rewrite/PLAN.md): divergences are adjudicated
against the SPECS; Haskell bugs are never reproduced and never patched;
entries here carry the spec authority, the fiction consequence, and the
comparator posture.

## DIV-1 (S3): the frozen semi-naive closure is incomplete on cross-predicate aggregate bodies

**What**: `Prax.Derive.deltaJoin` (Derive.hs:120-128, both `run` and
`runCooked`) seeds only top-level `Match` positions from the delta. A rule
whose Exists/Subquery/Count/Or condition reads a DERIVED predicate disjoint
from its Match seed never re-fires after the Match fact leaves the delta —
the closure silently under-derives. Proven by live probe (recorded as the
negative fixture in conformance/fixtures/): `[Match p.X, Exists[Match q.Y]]
→ r.X` with `[Match trigger] → q.thing`: frozen omits `r.a`; the closure
contains it.

**Spec authority**: the view "IS the closure of the base under the axioms"
(ViewInvariantSpec's stated invariant); the frozen code documents semi-naive
as an optimization ("nothing already known is re-derived") — an optimization
that changes results contradicts its own contract. The Rust implements the
naive-equivalent closure (static fast-path/full-eval rule split; the
naive==production law is the flagship property).

**Fiction consequence**: none in any shipped world — their aggregate
conditions read the same predicate as their Match (shape luck, now recorded).
A future world with a cross-predicate aggregate axiom derives correctly on
Rust and incorrectly on the frozen Haskell.

**Comparator posture**: no suppression needed — shipped traces agree. The
negative fixture pins both outputs so the divergence is a committed artifact.

## Recorded posture (not a DIV): the ⊥-witness is selected by name order

When a closure round forces two or more distinct values into one exclusive slot,
BOTH engines report a single ⊥ witness — but they select it differently. The Rust
sorts the round's fresh heads by rendered name and folds the meet in that order,
so the reported witness is the name-least conflicting head (`derive.rs` `run`,
design I4). The frozen engine folds `foldM meetOne` in `nub` (generation) order,
so it reports the first conflicting head in generation order. The DeriveSpec pin
is stated up-to-set ("names AN offending head", DeriveSpec:75) and the flagship
`naive == production` law is internally consistent (both closures share the same
sort+fold), so this selection is verified against the naive oracle, not against
frozen's `nub` order.

This is NOT a divergence, because no shipped world produces ⊥ during closure:
`derive.json` has zero contradiction cases, and kin/div1 never force a conflicting
exclusive slot, so `check_closure_case`'s exact-witness comparison is never
exercised against frozen and no trace can differ. It is recorded here PRE-EMPTIVELY
so that a future ⊥-bearing fixture whose Rust witness differs from frozen's is read
as this known name-order-vs-nub-order selection difference (still up-to-set correct),
not mistaken for a fresh correctness divergence. Should such a world ever ship, this
posture graduates to a numbered DIV with a comparator suppression on the witness
field.
