# Exclusion Logic as a Deontic Logic — distilled notes (for LEDGER #34)

Source (read in full, local): `references/papers/EVAIEL.pdf` —
Richard Evans, "Introducing Exclusion Logic as a Deontic Logic," DEON 2010,
Springer LNCS 6181, pp. 179–195. DOI 10.1007/978-3-642-14183-6_14.
Page numbers below are the paper's internal pages (1–18).

This is the primary source that was blocking #34 (deontic `should`/obligation +
norm-conflict). These notes capture everything needed so we needn't re-read the
PDF. **The single most important fact for implementation is the notational flip
between the paper and our engine — see §0.**

## 0. Notational flip vs. our engine (READ FIRST)

The paper uses two binary tree operators; **their meanings are swapped relative
to `Prax.Db`**:

| concept | paper | our engine (`Prax.Db`) |
|---|---|---|
| multi-valued / compatible ("*one* of the ways A is B"; `A:B` & `A:C` coexist) | `:` (a.k.a. edge label `*`) | `.` |
| exclusion / single-valued ("B is the *only* way A is"; `A.B` excludes `A.C`) | `.` (a.k.a. edge label `!`) | `!` |

So the paper's obligation sentence **`Ob:P` (multi-valued) = our `Ob.P`**, and the
paper's exclusion `A.B` = our `A!B`. Our trie DB *is* an Exclusion-Logic model (a
single labeled rooted tree); `unify`/`insert` already implement EL's tree +
exclusion semantics (the corrected `!` is exactly EL's "only child" edge).

## 1. Exclusion Logic (EL) core

- A **pre-propositional** modal logic: **no negation, no disjunction**, only a
  restricted implication. Designed to capture Hierarchical Finite-State Machines
  declaratively — "trees of data" (p.1). Material incompatibility is taken as
  conceptually prior to negation (Brandom/Sellars), so incompatibility is
  expressed by the exclusion operator instead of `¬` + non-logical axioms (p.1–2).
- **Syntax (Def 1, p.2):** `X ::= S | S:X | S.X`; `E ::= X | E ∧ E`; `G ::= E | E → E`.
  Carefully **stratified**: terms → conjunctions → implications. `→` is **not
  recursively embeddable** (`P→Q` is well-formed; `P→Q→R` is not).
- **Two operators (Remark 1, p.2):** `A:B` = "B is *a* way A is" (compatible;
  `A:B` and `A:C` coexist). `A.B` = "B is the *only* way A is" (excludes `A.C`).
- **Negation is limited (p.3):** `¬P` = the *least contentful* claim incompatible
  with P. `⊥ ≡ A.A ∧ A.B`; `¬P ≡ P → ⊥`. Can negate a *simple* term (`P` or `P.X`)
  but **cannot** negate a complex term (`P∧Q`, `P→Q`) — `→` isn't embeddable.
- **Disjunction is limited (p.3):** `P∨Q` = least upper bound; a role `A` can play
  for `A.B ∨ A.C` but isn't identical to it. No general `∨`.
- **Semantics (p.4–5):** interpret expressions in a lattice of **Labeled Rooted
  Trees (LRTs)**. Edges labeled `*` (multi; our `.`) or `!` (the only child; our
  `!`). Valid LRT = acyclic, connected, irredundant, *respectful of exclusion*
  (Def 3). `signature s_X(v)` = the path of symbols root→v (Def 4). Partial order
  `≤` by subgraph isomorphism + edge specificity (`! ≤ *`) (Def 6): `A ≤ B` iff A
  carries at least as much info as B. `⊥ ≤ X ≤ ⊤`; lattice with glb `⊓` (=
  conjunction; `X⊓Y=⊥` if incompatible, Def 8) and lub `⊔` (Def 9). (Prop 5, p.10)
- **Satisfaction (Def 10, p.10):** `⊨_X E iff Sat(X, R_X, *, E)`. `A∧B` = both.
  **`A→B` at v = for every reachable v' (transitive closure E*_X): ¬Sat(A) or
  Sat(B)`** — implication quantifies over *all* vertices of the *one* model, so it
  is **stronger than material implication** but weaker than Lewis strict (p.10–11).
  Key consequence: `A.B ⊭ A.C → Z` (p.16) — used to defuse Chisholm (§2 below).
- **Decision procedure (Def 13–18, Prop 6–7, p.11–13):** don't enumerate models —
  build a **canonical minimal model `m(X)`** and test entailment directly:
  `X ⊨ Y iff ⊔[X] ≤ ⊔[Y] iff m(X) ≤ m(Y)`. `m` maps conjunctions to LRTs and
  repeatedly applies implications `G` to close the model (`m(G,A)`).
  **Complexity: polynomial — `nm²k²`** (n = #judgments, m = max #conjuncts, k =
  max conjunct complexity). "Testing entailment is polynomial-time and practically
  efficient" (p.13).

## 2. DEL — Deontic Exclusion Logic (§2, p.13–17)

The payoff. **DEL is EL with one syntactic addition and *zero* semantic changes.**

- **Syntax (p.14):** add `□X` to terms: `X ::= S | S:X | S.X | □X`; `E`, `G` as
  before. `□P` reads "**P should be the case**."
- **`□P` is pure syntactic sugar for `Ob:P`** (p.14) — `Ob` is an ordinary EL
  **constant**, suggestively named. In our notation (flip): **`□P` ≡ the fact
  `Ob.P`**. No new semantics: obligations are just normal sentences in the tree.
  Because `Ob:` is the multi-valued operator, **many obligations coexist**
  (`Ob.P`, `Ob.Q`, …), while incompatible ones collapse (see property 2).
- **Stratification (Fig 13, p.14):** well-formed: `□A ∧ □B`, `A → □B`, `□□A`.
  **Ill-formed: `□(A∧B)`, `□(A→B)`** — `□` applies only to *simple terms*, never
  to conjunctions/implications. `□` **may iterate** (`□□A`) but may **not** scope
  over the logical connectives.
- **Why it is a (minimal) deontic logic (§2.3, p.15):** three intuitions a deontic
  `Ob` must satisfy, all already respected by EL's semantics:
  1. **Closure under strict implication:** `P→Q, □P ⊨ □Q` (via EL's `→` sat cond).
  2. **No incompatible deontic judgments:** if P, Q incompatible, *no* model
     satisfies `□P ∧ □Q` (via EL's `∧`: the conjunction is `⊥`). → **norm conflict
     surfaces as unsatisfiability**, not silent coexistence.
  3. **Ought ≠ is:** `□P ⊭ P` — obligations can stay unsatisfied.
  These *permit* (don't force) reading `□` deontically ⇒ DEL is a *minimal* deontic
  logic.
- **Avoids the SDL paradoxes (§2.4, p.15–17):**
  - **Tautologies-obligatory** (SDL `⊨ □P` for tautology P): DEL's only tautologies
    are implications (`A.B→A`), and `□` can't apply to implications ⇒ none inside `□`.
  - **Ross's paradox** (SDL `□P ⊨ □(P∨Q)`): DEL disallows `∨` inside `□` (stratified).
  - **Chisholm's puzzle / contrary-to-duty (p.16–17):** the four claims are
    represented (using `P.True`/`P.False` for `P`/`¬P`):
    1. `⊤ → □ Go.True`   2. `Go.True → □ Tell.True`
    3. `Go.False → □ Tell.False`   4. `Go.False`
    Because DEL's `→` is stronger than material (`A.B ⊭ A.C→Z`), the bad inference
    (4)→(2) is blocked; you get `□□ Tell.True` **and** `□ Tell.False`, compatibly.
    **`□□ Tell.True`** = "*ideally*, had events unfolded as they should, Jones
    should have told his neighbours" — **iterated `□` captures the ideal / CTD
    nesting** (p.17).
- **Vs von Wright / Castañeda (Fig 14, p.17):** `□□P`: SDL yes, DEL **yes**, vW no.
  `□(P→Q)`: SDL yes, DEL **no**, vW no. DEL = principled middle ground.
- **Computational note (§3, p.17):** "DEL has been used to power a multi-agent
  simulation" — social practices modelled in DEL: **turn-taking, social status,
  queuing up, and taboo activity**; "expressive and intuitive for representing
  social practices declaratively." (This is Versu/Praxis — DEL is its logic.)

## 3. Implementation implications for #34 (grounding for the plan)

Our engine already realises the EL substrate. To add a *faithful* deontic layer:

1. **Representation — obligation as a first-class fact.** Mirror the existing
   `violated.<who>.<norm>` convention (`Prax.Reactions.violationPath`). An
   obligation "who should φ" = **`Ob.<who>.<φ>`** (our `.` = paper's `Ob:`, so
   multi-valued: an agent may hold several obligations at once). This is exactly
   `□φ` for that agent.
2. **A stratified `should` operator in `Prax.Query`/effects.** Add a smart
   constructor / `Condition` + an `Outcome`:
   - assert an obligation (`oblige who φ` ⇒ `Insert "Ob.who.φ"`),
   - query it (`isObliged who φ` ⇒ `Match "Ob.who.φ"`), fulfilment (φ holds),
     violation (φ became impossible / an exclusive counter-value holds).
   Keep it **stratified**: the operator takes a *simple term* (a sentence), never a
   compound `Condition` — this is what buys the paradox-avoidance, so it must be a
   discipline of the API, not decoration.
3. **Norm conflict = exclusion (property 2).** Two obligations on *exclusive* values
   of one slot (`Ob.A.go!true` vs `Ob.A.go!false`) are the paper's incompatible
   `□P∧□Q ⇒ ⊥`. Design choice to make: **detect** the conflict (surface it — a DM
   or agent must resolve/prioritise) rather than let our `!`-insert silently
   overwrite (last-write-wins). Norm-conflict *resolution* (the second half of #34)
   is where priorities/defeasibility live; the paper gives the *detection* (⊥) but
   not a resolution policy — cf. Alchourrón–Makinson "Hierarchies of Regulations"
   [ref 1] and Boella–van der Torre hierarchical normative systems [ref 2] for
   ordering, if we want principled priorities.
4. **Behavioural coupling is already ours (Versu did the same).** No planner change:
   a `Want [ isObliged self φ, Not φ ] (−k)` (or a positive want on fulfilment) makes
   the utility planner pursue/avoid obligations — the same mechanism that already
   drives `violationOf` in `Prax.Worlds.Bar`. DEL supplies the *representation*; the
   simulation supplies the *motivation*.
5. **Contrary-to-duty via iterated `□`.** `□□φ` = `Ob.Ob.φ` — the "ideal" obligation
   that would have held. Useful for reparative/second-best norms (Chisholm).

### Gaps at v14, all closed by v15 (`Prax.EL` + `Prax.Derive`)
These were the honest limitations when the deontic layer (v14) first shipped. The
derivation layer (v15) closed them by implementing the paper's machinery for real:
- **Closure under implication (property 1) — DONE, by authored declaration.**
  `Prax.Derive` forward-chains domain rules to a fixpoint (`m(X)`); the `□`-lift
  itself is an authored declaration in `Prax.Deontic` (`obligedClose` lifts each
  all-`Match` rule under `□`), so a deontic world writes `setAxioms (obligedClose
  axs)` and `Ob.P` + `P→Q` yields `Ob.Q`. (v15 shipped the lift as an automatic
  engine step; v51 moved it out of the general engine into the world's own
  declaration — the checker flags a world that can invoke an obligation yet omits
  the closure, `Prax.TypeCheck.DeonticUnclosed`.)
- **Incompatibility detection — DONE, faithfully.** `Prax.Db` retains the `!`/`.`
  labels (the world state is a real Exclusion-Logic model), so `Prax.EL.meet`
  detects `□P∧□Q ⇒ ⊥` from either side of the clash — no silent overwrite, and no
  schema needed.
- **The `m(X)` decision procedure — BUILT.** `Prax.EL` is the LRT lattice
  (`meet`/`leq`); it is also the substrate #8 (the static type checker) will reuse,
  which would add `⊔`/general entailment on top.

## 4. References cited by the paper (p.18), for onward reading if needed
Alchourrón & Makinson, *Hierarchies of Regulations and their Logic* (1981) [1];
Boella & van der Torre, *Permissions and Obligations in Hierarchical Normative
Systems*, ICAIL (2003) [2]; Makinson & van der Torre, *Input/Output Logics* (2000)
[7] and *Permission from an I/O Perspective* (2003) [8]; Brandom, *Making It
Explicit* (1994) [3]; Castañeda, *The Paradoxes of Deontic Logic* (1981) [5]; von
Wright, *Deontic Logic* (1951) [9]; Evans, *The Logical Form of Status-Function
Declarations* (2009) [6]. (Not obtained; [1]/[2] are the leads for norm-conflict
*resolution* ordering if we take that further.)
