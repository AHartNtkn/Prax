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
