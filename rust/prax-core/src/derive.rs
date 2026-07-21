//! Forward-chaining derivation: domain axioms `body → head` closed to a
//! fixpoint (the paper's canonical model `m(G,A)`). Heads are `meet`-ed into the
//! model so a forced exclusive slot yields `⊥` — a detected contradiction, never
//! a silent overwrite. Closure is a view (the base stays the source of truth),
//! so a conclusion whose premise is retracted simply disappears.
//!
//! Frozen reference: `src/Prax/Derive.hs`. The port makes ONE deliberate,
//! adjudicated divergence (`docs/rewrite/DIVERGENCES.md` DIV-1): the frozen
//! semi-naive [`Prax.Derive.deltaJoin`] seeds only top-level `Match` positions
//! from the delta, so a rule whose `Exists`/`Subquery`/`Count`/`Or` condition
//! reads a *derived* predicate disjoint from its `Match` under-derives once the
//! `Match` fact leaves the delta. The view's own contract is that it "IS the
//! closure of the base under the axioms", so the least fixpoint (the naive
//! closure) is the semantics and the frozen semi-naive is an incomplete
//! optimization. This module implements the *correct* closure by splitting rules
//! STATICALLY at compile time:
//!
//! - **fast-path** rules — a body of `Match` + pure binding filters
//!   (`Eq`/`Neq`/`Cmp`/`Calc`, which read bindings, not the db) — are
//!   delta-seeded exactly as the frozen per-`Match`-position join
//!   ([`Prax.Derive.deltaJoin`], `Derive.hs:127-133`). Semi-naive is provably
//!   complete on this fragment (every positive contributor to a binding is a
//!   top-level `Match`), so this optimization changes nothing.
//! - **full-eval** rules — any body containing `Exists`/`Absent`/`Not`/`Or`/
//!   `Subquery`/`Count` — are re-evaluated against the FULL model every round.
//!   This is what keeps the optimization an optimization instead of the frozen
//!   bug; it is where the Rust beats the frozen engine.
//!
//! A degenerate body with no `Match` position (only binding filters — the frozen
//! `[] -> query model body` branch, `Derive.hs:129`) is evaluated against the
//! full model every round like a full-eval rule.
//!
//! The flagship property (`conformance`'s `derive_props`) is `naive == production`
//! over a generator that covers the frozen bug's fragment, pinning the divergence
//! as a red/green artifact.

use smallvec::SmallVec;

use crate::db::{Bindings, Db, ground};
use crate::error::WorldError;
use crate::interner::{Interner, Sym};
use crate::path::{CompiledPath, tokenize};
use crate::query::{CmpOp, Cond, Condition, compile_condition, query, read_anchors};
use crate::relevance::may_unify_syms;

/// A detected contradiction (`⊥`): the rendered head sentence whose assertion
/// was incompatible with the model (`Prax.Derive.Contradiction`). The string is
/// label-faithful — `!`/`.` preserved, exactly as the offending grounded head
/// renders (the ⊥ witness the closure reports).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contradiction(pub String);

/// How a rule's body is evaluated each round — decided ONCE at compile
/// ([`CompiledRule::compile`]), never re-inspected in the hot loop.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RuleKind {
    /// Body of `Match` + pure binding filters, with ≥1 `Match`. Delta-seeded per
    /// the frozen per-position join; the payload is the `Match` positions in the
    /// body (`Derive.hs:128`'s `pos`).
    FastPath(SmallVec<[usize; 4]>),
    /// Any body with a db-reading non-`Match` condition (`Exists`/`Absent`/`Not`/
    /// `Or`/`Subquery`/`Count`), OR a degenerate `Match`-free filter body.
    /// Re-evaluated against the full model every round.
    FullEval,
}

/// An axiom compiled once per world: the body pattern-split into runtime
/// [`Cond`]s, the head templates pre-tokenized into [`CompiledPath`]s (keeping
/// their `!`/`.` labels so exclusion is honoured when they are met into the
/// model), and the static evaluation strategy. The heir of
/// `Prax.Derive.CookedRule` — the compile choke point does this once, so the
/// closure loop never re-cooks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledRule {
    body: Vec<Cond>,
    heads: Vec<CompiledPath>,
    kind: RuleKind,
    /// Every db path the body reads, at any polarity ([`read_anchors`]) — computed
    /// once here so the semi-naive loop can skip a rule in a round whose delta
    /// touches none of them. Within a single closure the model only grows, so a
    /// rule newly-derives a fact only when a fact added last round may-unifies one
    /// of these anchors; a negation only tightens and a monotone threshold only
    /// crosses when its counted set (a read anchor) gains a member. Empty means the
    /// body reads no path (a degenerate constant/filter body): never skipped.
    read_anchors: Vec<SmallVec<[Sym; 6]>>,
}

impl CompiledRule {
    /// Compile an authored rule `body → heads`: each [`Condition`] to its runtime
    /// [`Cond`] ([`compile_condition`]) and each head sentence to a
    /// [`CompiledPath`] ([`tokenize`], rejecting a trailing operator). The static
    /// rule split is computed here, once.
    pub fn compile(
        interner: &mut Interner,
        body: &[Condition],
        heads: &[&str],
    ) -> Result<CompiledRule, WorldError> {
        let body: Vec<Cond> = body
            .iter()
            .map(|c| compile_condition(interner, c))
            .collect::<Result<_, _>>()?;
        let heads: Vec<CompiledPath> = heads
            .iter()
            .map(|h| tokenize(interner, h))
            .collect::<Result<_, _>>()?;
        let kind = classify(&body);
        let read_anchors = read_anchors(&body);
        Ok(CompiledRule {
            body,
            heads,
            kind,
            read_anchors,
        })
    }
}

/// The static rule split (`DIVERGENCES.md` DIV-1): a body with any db-reading
/// non-`Match` condition, or with no `Match` at all, is [`RuleKind::FullEval`];
/// otherwise it is [`RuleKind::FastPath`] over its `Match` positions.
fn classify(body: &[Cond]) -> RuleKind {
    let has_db_reading_non_match = body.iter().any(|c| {
        matches!(
            c,
            Cond::Not(_)
                | Cond::Absent(_)
                | Cond::Exists(_)
                | Cond::Or(_)
                | Cond::Subquery { .. }
                | Cond::Count(..)
        )
    });
    if has_db_reading_non_match {
        return RuleKind::FullEval;
    }
    let positions: SmallVec<[usize; 4]> = body
        .iter()
        .enumerate()
        .filter_map(|(i, c)| matches!(c, Cond::Match(_)).then_some(i))
        .collect();
    if positions.is_empty() {
        RuleKind::FullEval
    } else {
        RuleKind::FastPath(positions)
    }
}

/// Substitute bound values into a head template's variable segments, producing a
/// grounded [`CompiledPath`] — no string round-trip (`Prax.Derive.groundTokens`,
/// `Derive.hs:115`). The `excl` bitmask (the head's `!`/`.` labels) is carried
/// through unchanged, which is what makes the meet honour exclusion and the ⊥
/// witness label-faithful. An unbound head variable grounds to its own name.
///
/// A bound value substitutes via [`val_to_sym`] — a [`Val::Sym`] as-is, any other
/// value rendered then interned. Heads bind only separator-free values (every
/// unify-produced binding is a single trie key; a `Num` renders as a decimal and
/// a `Set` as an opaque marker — none carry `.`/`!`), so the segment count is
/// preserved and the bitmask stays aligned (`debug_assert`ed, and pinned by the
/// `ground_head_carries_the_head_excl_bitmask` law).
fn ground_head(interner: &mut Interner, head: &CompiledPath, b: &Bindings) -> CompiledPath {
    crate::db::ground_tokens(interner, head, b)
}

/// The label-faithful rendering of a grounded head — `!`/`.` preserved
/// (`Prax.Db.tokensToSentence`). Reuses [`ground`] with an empty binding, which
/// on a ground path just re-emits each segment's name joined by its separator.
fn render_head(interner: &Interner, head: &CompiledPath) -> String {
    ground(interner, head, &Bindings::new())
}

/// Does the model already entail the head token-for-token
/// (`Prax.Derive.entailed`: `leq m (insertToks h emptyDb)`)? The round's
/// entailed heads are dropped before the meet-fold, against the ROUND-START model.
fn entailed(model: &Db, head: &CompiledPath) -> bool {
    // `leq(model, Db::empty().insert(head))` specialised to one grounded path and
    // walked WITHOUT building the singleton Db (the closure's hottest allocation:
    // one throwaway chain per candidate head per round). The singleton is a linear
    // chain whose node for segment `i` carries `excl = head.is_excl_after(i)` and
    // whose LEAF alone is asserted; `leq` then demands, at each level, a model node
    // at least as exclusive (Excl ≤ Multi) and — at the leaf — asserted. Interior
    // singleton nodes are unasserted, so their assertedness clause is vacuous. The
    // `entailed_matches_leq_singleton` proptest pins this against the general `leq`.
    let mut node = model;
    let n = head.segs.len();
    for (i, &seg) in head.segs.iter().enumerate() {
        let Some(child) = node.child(seg) else {
            return false;
        };
        if head.is_excl_after(i) && !child.is_excl() {
            return false;
        }
        if i + 1 == n && !child.is_asserted() {
            return false;
        }
        node = child;
    }
    true
}

/// Meet a single grounded head into the model — MEET semantics (`⊥` on a second
/// distinct child under an exclusive node, assertedness OR-joined), NOT insert
/// semantics (no sibling clearing). `None` is the paper's `⊥` (`Prax.Derive.meetOne`,
/// via `Prax.EL.meet`).
///
/// The banked direct-walk optimization (ARCHITECTURE.md): `meet(model,
/// singleton(head))` touches ONLY the head's path, yet the general [`meet`] both
/// materialises the singleton Db AND clones the model's whole child vector at
/// every level. [`meet_head`] path-copies just the head's spine — sharing every
/// untouched subtree — so a derived head meet is O(head depth × local width)
/// rather than O(model width). Pinned bit-for-bit against the general `meet` by
/// `meet_head_matches_general_meet`.
fn meet_one(model: &Db, head: &CompiledPath) -> Option<Db> {
    meet_head(model, head, 0)
}

/// Meet the head-singleton's `segs[i]` child into `node` (a model subtree),
/// path-copying `node` and recursing down the spine. Only called at the root
/// (`i == 0`), where `node` is never exclusive, so the root's own `⊥` check is
/// vacuous; interior/leaf `⊥` is handled in [`meet_head_node`].
fn meet_head(node: &Db, head: &CompiledPath, i: usize) -> Option<Db> {
    let seg = head.segs[i];
    let met_child = match node.child(seg) {
        Some(existing) => meet_head_node(existing, head, i)?,
        None => head_subchain(head, i),
    };
    let mut kids: Vec<(Sym, Db)> = node.kids().iter().filter(|(s, _)| *s != seg).cloned().collect();
    kids.push((seg, met_child));
    Some(Db::from_parts(node.is_excl(), node.is_asserted(), kids))
}

/// Meet the head-singleton node at depth `i` into the model node `existing` for
/// `segs[i]`: OR-join `excl`/`asserted`, merge the singleton's single child
/// (`segs[i+1]`, unless this is the leaf), and report `⊥` when an exclusive node
/// is left with more than one child — exactly [`meet`]'s per-node rule.
fn meet_head_node(existing: &Db, head: &CompiledPath, i: usize) -> Option<Db> {
    let excl = existing.is_excl() || head.is_excl_after(i);
    let asserted = existing.is_asserted() || (i + 1 == head.segs.len());
    let mut kids: Vec<(Sym, Db)> = existing.kids().to_vec();
    if i + 1 < head.segs.len() {
        let seg1 = head.segs[i + 1];
        let met = match existing.child(seg1) {
            Some(c) => meet_head_node(c, head, i + 1)?,
            None => head_subchain(head, i + 1),
        };
        kids.retain(|(s, _)| *s != seg1);
        kids.push((seg1, met));
    }
    if excl && kids.len() > 1 {
        return None;
    }
    Some(Db::from_parts(excl, asserted, kids))
}

/// The head-singleton's subtree rooted at `segs[j]` (no model node to meet with):
/// the bare chain `segs[j..]` with each node's `excl = head.is_excl_after(k)` and
/// only the leaf asserted — exactly what `Db::empty().insert(head)` builds below
/// `segs[j]`.
fn head_subchain(head: &CompiledPath, j: usize) -> Db {
    let n = head.segs.len();
    let mut node = Db::from_parts(head.is_excl_after(n - 1), true, Vec::new());
    for k in (j..n - 1).rev() {
        node = Db::from_parts(head.is_excl_after(k), false, vec![(head.segs[k + 1], node)]);
    }
    node
}

/// Close a base [`Db`] under a set of [`CompiledRule`]s: apply the rules to a
/// fixpoint and return the closed model. `Err` reports the first contradiction
/// (the name-least ⊥ witness). With no rules the base is returned unchanged (the
/// identity that keeps un-axiomatised worlds free — `Prax.Derive.closure`'s
/// `closure [] db0 = Right db0`).
pub fn close(
    interner: &mut Interner,
    rules: &[CompiledRule],
    base: &Db,
) -> Result<Db, Contradiction> {
    if rules.is_empty() {
        return Ok(base.clone());
    }
    // From scratch: round 1 must evaluate every rule (the whole base is the delta),
    // so pass `None` — the round-delta gate engages only from round 2 on.
    run(interner, rules, base.clone(), base.clone(), None)
}

/// Continue an ALREADY-CLOSED model with new base `facts`: the frozen
/// `Prax.Derive.closureFrom` — exactly [`close`]'s loop, entered at
/// `(closed ∪ facts, delta = facts)`. The facts are inserted (base-fact
/// insert semantics, sibling-clearing under `!`) to form the starting model and
/// seeded as the first delta.
///
/// HONESTY (the S4 continuation tier's precondition, `DIVERGENCES` soundness
/// I4): this function is sound ONLY when the facts are monotone for these axioms
/// — `!`-free and unifying no negated body pattern. On a non-monotone delta it
/// does NOT fail loudly: a `!`-conflict is caught as `⊥`, but a fact that
/// *un-fires* a rule (matches a `Not`/`Absent` body pattern) leaves the
/// previously-derived fact silently in place — the loop only ever ADDS. It
/// CANNOT self-detect that silent stale survivor by construction. The guard is
/// the CALLER's (S4's engine router: `contMonotone × insert kind`) plus the
/// per-mutation testkit oracle (`view == naive_closure`) in the S4/S7 nets, not
/// anything here.
pub fn close_from(
    interner: &mut Interner,
    rules: &[CompiledRule],
    closed: &Db,
    facts: &[CompiledPath],
) -> Result<Db, Contradiction> {
    let mut model = closed.clone();
    let mut delta = Db::empty();
    for f in facts {
        model = model.insert(f);
        delta = delta.insert(f);
    }
    if rules.is_empty() {
        return Ok(model);
    }
    // Continuation: the inserted facts ARE the round-1 delta, so only the rules
    // they can reach need evaluating — the crux of the continuation tier's speed.
    run(interner, rules, model, delta, Some(facts.to_vec()))
}

/// The semi-naive fixpoint loop, shared by [`close`] and [`close_from`]. Each
/// round grounds every rule's heads over its bindings (fast-path rules
/// delta-seeded, full-eval rules against the full model), drops the heads the
/// round-start model already entails, dedups them, sorts by rendered name (so
/// the ⊥ witness is deterministic and name-least, `DIVERGENCES` design I4), then
/// meets them in — `⊥` short-circuits. The fresh heads become the next delta.
/// Terminates when a round produces no fresh fact.
fn run(
    interner: &mut Interner,
    rules: &[CompiledRule],
    mut model: Db,
    mut delta: Db,
    delta_facts: Option<Vec<CompiledPath>>,
) -> Result<Db, Contradiction> {
    // The ground facts added last round, for the per-rule delta gate. `None` in
    // round 1 of a from-scratch closure means "the delta is everything" — no rule
    // is gated. From round 2 on this is the previous round's fresh facts.
    let mut delta_facts = delta_facts;
    loop {
        let mut fresh: Vec<(String, CompiledPath)> = Vec::new();
        for rule in rules {
            // Semi-naive gate: a rule can only newly-fire this round if a fact
            // added last round may-unifies one of its read anchors. Skipping the
            // rest is exact — their bindings are unchanged from last round, where
            // their heads were already entailed (a rule with no read anchors reads
            // no path, so it is never gated). The gate reuses the engine router's
            // own [`may_unify_syms`] relevance predicate.
            if let Some(df) = delta_facts.as_deref()
                && !rule.read_anchors.is_empty()
                && !df.iter().any(|f| {
                    rule.read_anchors
                        .iter()
                        .any(|a| may_unify_syms(&f.segs, a))
                })
            {
                continue;
            }
            for b in rule_bindings(interner, rule, &model, &delta) {
                for h in &rule.heads {
                    let g = ground_head(interner, h, &b);
                    if !entailed(&model, &g) {
                        let name = render_head(interner, &g);
                        fresh.push((name, g));
                    }
                }
            }
        }
        if fresh.is_empty() {
            return Ok(model);
        }
        // Structural dedup + name-order (rendering is injective on CompiledPath,
        // so dedup-by-name == dedup-structural — DIVERGENCES design I4).
        fresh.sort_by(|a, b| a.0.cmp(&b.0));
        fresh.dedup_by(|a, b| a.0 == b.0);

        // The next delta is a fresh Db of exactly this round's new facts
        // (Derive.hs:122 `foldl insertToks emptyDb fresh`).
        let mut next_delta = Db::empty();
        for (_, g) in &fresh {
            next_delta = next_delta.insert(g);
        }
        let next_facts: Vec<CompiledPath> = fresh.iter().map(|(_, g)| g.clone()).collect();

        // Meet each fresh head into the model; the first ⊥ is the name-least one.
        let mut next_model = model;
        for (name, g) in &fresh {
            next_model = match meet_one(&next_model, g) {
                Some(m) => m,
                None => return Err(Contradiction(name.clone())),
            };
        }
        model = next_model;
        delta = next_delta;
        delta_facts = Some(next_facts);
    }
}

/// The bindings a rule contributes this round: full-eval rules query their whole
/// body against the model; fast-path rules do the frozen per-`Match`-position
/// delta join (`Derive.hs:130-133`), union over positions with a structural dedup.
fn rule_bindings(
    interner: &mut Interner,
    rule: &CompiledRule,
    model: &Db,
    delta: &Db,
) -> Vec<Bindings> {
    match &rule.kind {
        RuleKind::FullEval => query(interner, model, &rule.body, &Bindings::new()),
        RuleKind::FastPath(positions) => {
            let mut out: Vec<Bindings> = Vec::new();
            for &i in positions {
                for b in join_at(interner, &rule.body, i, model, delta) {
                    if !out.contains(&b) {
                        out.push(b);
                    }
                }
            }
            out
        }
    }
}

/// One `Match`-position's delta join (`Derive.hs:132-133`'s `joinAt i`): fold the
/// body left to right threading bindings, evaluating position `i` against
/// `delta` and every other position against `model`.
fn join_at(
    interner: &mut Interner,
    body: &[Cond],
    i: usize,
    model: &Db,
    delta: &Db,
) -> Vec<Bindings> {
    let mut bs = vec![Bindings::new()];
    for (j, c) in body.iter().enumerate() {
        let db = if j == i { delta } else { model };
        let mut next = Vec::new();
        for b in bs {
            next.extend(query(interner, db, std::slice::from_ref(c), &b));
        }
        bs = next;
    }
    bs
}

/// The test-only naive closure oracle (`testkit` feature): full-query EVERY rule
/// against the full model EVERY round — no delta machinery, no static split. The
/// heir of the frozen `runCooked`-vs-`closure` cross-check: it shares the
/// substrate (`query`/`meet`/`ground_head`) with production but the LOOP is
/// independent (naive full-query-to-fixpoint vs semi-naive delta-join), which is
/// exactly the closure-strategy surface DIV-1 turns on. The `naive == production`
/// property (`conformance`) is the flagship law; substrate bugs are caught by the
/// S1/S2 law suites, not this net (stated honestly — `S03-design.md` §7).
///
/// Same input type as production ([`CompiledRule`]), so no restatement is needed.
#[cfg(feature = "testkit")]
pub fn naive_closure(
    interner: &mut Interner,
    rules: &[CompiledRule],
    base: &Db,
) -> Result<Db, Contradiction> {
    if rules.is_empty() {
        return Ok(base.clone());
    }
    let mut model = base.clone();
    loop {
        let mut fresh: Vec<(String, CompiledPath)> = Vec::new();
        for rule in rules {
            for b in query(interner, &model, &rule.body, &Bindings::new()) {
                for h in &rule.heads {
                    let g = ground_head(interner, h, &b);
                    if !entailed(&model, &g) {
                        let name = render_head(interner, &g);
                        fresh.push((name, g));
                    }
                }
            }
        }
        if fresh.is_empty() {
            return Ok(model);
        }
        fresh.sort_by(|a, b| a.0.cmp(&b.0));
        fresh.dedup_by(|a, b| a.0 == b.0);
        let mut next_model = model;
        for (name, g) in &fresh {
            next_model = match meet_one(&next_model, g) {
                Some(m) => m,
                None => return Err(Contradiction(name.clone())),
            };
        }
        model = next_model;
    }
}

// ---- the axiom-derived analysis tables (S4 OWNS these; consumed by the engine
// router and the planner's reuse cone) --------------------------------------

/// Every path pattern the axioms can read or write: body atoms at any polarity
/// (including inside `Absent`/`Exists`/`Or`/`Subquery` — the [`read_anchors`]
/// walk) and head templates (`Prax.Derive.axiomFootprint`). Any □-lifted rules a
/// deontic world declared are ordinary rules in the list, so they need no second
/// enumeration. A ground delta that may-unify none of these commutes with
/// closure (the engine's delta-irrelevance fast path).
pub fn axiom_footprint(rules: &[CompiledRule]) -> Vec<SmallVec<[Sym; 6]>> {
    let mut out = Vec::new();
    for r in rules {
        out.extend(read_anchors(&r.body));
        for h in &r.heads {
            out.push(h.segs.clone());
        }
    }
    out
}

/// Every pattern under a negation in any body (`Prax.Derive.axiomNegPatterns`):
/// inserting a fact these match can UN-fire a rule (retraction), so such facts
/// never take the continuation tier. `Not` contents and everything inside an
/// `Absent` (a `¬∃`); positive atoms are excluded.
pub fn axiom_neg_patterns(rules: &[CompiledRule]) -> Vec<SmallVec<[Sym; 6]>> {
    let mut out = Vec::new();
    for r in rules {
        for c in &r.body {
            neg_of(c, &mut out);
        }
    }
    out
}

fn neg_of(c: &Cond, out: &mut Vec<SmallVec<[Sym; 6]>>) {
    match c {
        Cond::Not(p) => out.push(p.clone()),
        Cond::Absent(cs) => out.extend(read_anchors(cs)), // everything inside a ¬∃
        Cond::Exists(cs) => {
            for c in cs {
                neg_of(c, out);
            }
        }
        Cond::Or(clauses) => {
            for cl in clauses {
                for c in cl {
                    neg_of(c, out);
                }
            }
        }
        Cond::Subquery { where_, .. } => {
            for c in where_ {
                neg_of(c, out);
            }
        }
        Cond::Match(_)
        | Cond::Eq(..)
        | Cond::Neq(..)
        | Cond::Cmp(..)
        | Cond::Calc(..)
        | Cond::Count(..) => {}
    }
}

/// Every head template the axioms can write (`Prax.Derive.axiomHeadPatterns`) —
/// any □-lifted rules a deontic world declared included, since they are ordinary
/// rules. A delta that feeds some axiom can change derived facts only in these
/// families. (The engine appends the `contradiction` witness to this.)
pub fn axiom_head_patterns(rules: &[CompiledRule]) -> Vec<SmallVec<[Sym; 6]>> {
    rules
        .iter()
        .flat_map(|r| r.heads.iter().map(|h| h.segs.clone()))
        .collect()
}

/// Is the axiom set continuation-safe: does adding base facts only ever ADD
/// derived facts (given the caller also avoids negated patterns)?
/// (`Prax.Derive.monotoneAxioms`, the frozen accept/reject decision table
/// verbatim.) Conditions must be monotone-up: `Match`/`Not`/`Absent` (negations
/// handled via [`axiom_neg_patterns`]), recursion through `Exists`/`Or`/
/// `Subquery`, `Count` freely, `Cmp` only in the grows-only direction (the count
/// side growing past a numeric literal — `Gt`/`Gte` with the literal right,
/// `Lt`/`Lte` with it left), and `Eq`/`Neq` only over pattern-bound variables. An
/// `Eq`/`Neq` over an aggregate-bound variable (a `Count` result or a `Subquery`
/// set) expresses exactly-k/not-k, anti-monotone as the aggregate grows past k.
/// `Calc` (and any other `Cmp` shape) disables the tier for the world.
pub fn monotone_axioms(interner: &Interner, rules: &[CompiledRule]) -> bool {
    rules.iter().all(|r| {
        let aggs = agg_vars(&r.body);
        r.body.iter().all(|c| cond_ok(interner, &aggs, c))
    })
}

/// Every variable bound by an aggregate anywhere in the body (a body shares one
/// binding environment, so a `Count`/`Subquery` result nested under
/// `Exists`/`Or`/`Subquery` is still visible to an `Eq`/`Neq` elsewhere).
fn agg_vars(body: &[Cond]) -> Vec<Sym> {
    let mut out = Vec::new();
    for c in body {
        collect_agg(c, &mut out);
    }
    out
}

fn collect_agg(c: &Cond, out: &mut Vec<Sym>) {
    match c {
        Cond::Count(r, _) => out.push(*r),
        Cond::Subquery { set, where_, .. } => {
            out.push(*set);
            for c in where_ {
                collect_agg(c, out);
            }
        }
        Cond::Exists(cs) => {
            for c in cs {
                collect_agg(c, out);
            }
        }
        Cond::Or(clauses) => {
            for cl in clauses {
                for c in cl {
                    collect_agg(c, out);
                }
            }
        }
        _ => {}
    }
}

fn cond_ok(interner: &Interner, aggs: &[Sym], c: &Cond) -> bool {
    match c {
        Cond::Match(_) | Cond::Not(_) | Cond::Absent(_) | Cond::Count(..) => true,
        Cond::Eq(l, r) | Cond::Neq(l, r) => !aggs.contains(l) && !aggs.contains(r),
        Cond::Exists(cs) => cs.iter().all(|c| cond_ok(interner, aggs, c)),
        Cond::Or(clauses) => clauses
            .iter()
            .all(|cl| cl.iter().all(|c| cond_ok(interner, aggs, c))),
        Cond::Subquery { where_, .. } => where_.iter().all(|c| cond_ok(interner, aggs, c)),
        Cond::Cmp(op, l, r) => match op {
            CmpOp::Gt | CmpOp::Gte => numeric_literal(interner, *r),
            CmpOp::Lt | CmpOp::Lte => numeric_literal(interner, *l),
        },
        Cond::Calc(..) => false,
    }
}

/// Whether a symbol's name is a non-empty run of ASCII digits (the frozen
/// `numeric` test — a numeric literal on the safe side of a threshold).
fn numeric_literal(interner: &Interner, x: Sym) -> bool {
    let s = interner.resolve(x);
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    // H: DeriveSpec.hs "Prax.Derive (m(X) closure)"
    //
    // The frozen `Prax.DeriveSpec`'s closure-semantics pins, re-expressed against
    // the Rust closure. The obligedClose/axiomFootprint/axiomNegPatterns/
    // monotoneAxioms pins test S4 analysis-table builders (`compilepipe`), not the
    // closure surface, and are deferred loudly in `conformance/KILLED.md`.
    use super::*;
    use crate::db::Db;
    use crate::interner::Interner;
    use crate::query::Condition;

    fn build(interner: &mut Interner, facts: &[&str]) -> Db {
        let mut db = Db::empty();
        for f in facts {
            db = db.insert_str(interner, f).unwrap();
        }
        db
    }

    fn rule(interner: &mut Interner, body: &[Condition], heads: &[&str]) -> CompiledRule {
        CompiledRule::compile(interner, body, heads).unwrap()
    }

    /// The closed model's sentences, or `[]` on contradiction
    /// (`DeriveSpec.closedFacts`).
    fn closed_facts(interner: &mut Interner, rules: &[CompiledRule], base: &Db) -> Vec<String> {
        match close(interner, rules, base) {
            Ok(d) => d.to_sentences(interner),
            Err(_) => Vec::new(),
        }
    }

    /// The facts the axioms ADD (closure minus base) — `Prax.Derive.derived`.
    fn derived(interner: &mut Interner, rules: &[CompiledRule], base: &Db) -> Vec<String> {
        let base_sents = base.to_sentences(interner);
        closed_facts(interner, rules, base)
            .into_iter()
            .filter(|s| !base_sents.contains(s))
            .collect()
    }

    fn m(s: &str) -> Condition {
        Condition::Match(s.to_owned())
    }

    // H: DeriveSpec.hs "no axioms ⇒ the base is returned unchanged"
    #[test]
    fn no_axioms_returns_the_base_unchanged() {
        let mut i = Interner::new();
        let base = build(&mut i, &["a.b"]);
        assert_eq!(close(&mut i, &[], &base), Ok(build(&mut i, &["a.b"])));
    }

    // H: DeriveSpec.hs "a single domain rule derives a consequence"
    #[test]
    fn a_single_domain_rule_derives_a_consequence() {
        let mut i = Interner::new();
        let axs = [rule(&mut i, &[m("at.W.bar")], &["in.W.building"])];
        let base = build(&mut i, &["at.bex.bar"]);
        assert!(closed_facts(&mut i, &axs, &base).contains(&"in.bex.building".to_owned()));
    }

    // H: DeriveSpec.hs "closure reaches a multi-step (transitive) fixpoint"
    #[test]
    fn closure_reaches_a_multi_step_transitive_fixpoint() {
        let mut i = Interner::new();
        let axs = [rule(
            &mut i,
            &[m("reaches.X.Y"), m("reaches.Y.Z")],
            &["reaches.X.Z"],
        )];
        let base = build(&mut i, &["reaches.a.b", "reaches.b.c", "reaches.c.d"]);
        let d = derived(&mut i, &axs, &base);
        assert!(d.contains(&"reaches.a.c".to_owned()), "a→c");
        assert!(d.contains(&"reaches.a.d".to_owned()), "a→d (two derived hops)");
        assert!(d.contains(&"reaches.b.d".to_owned()), "b→d");
    }

    // H: DeriveSpec.hs "relational join with variable binding (grandparent)"
    #[test]
    fn relational_join_with_variable_binding_grandparent() {
        let mut i = Interner::new();
        let axs = [rule(
            &mut i,
            &[m("parent.X.Y"), m("parent.Y.Z")],
            &["grandparent.X.Z"],
        )];
        let base = build(&mut i, &["parent.tom.bob", "parent.bob.ann"]);
        assert_eq!(derived(&mut i, &axs, &base), ["grandparent.tom.ann"]);
    }

    // CASE 5 (S3 review I1): a two-`Match` rule whose NON-FIRST position binds a
    // fact that is DERIVED a round after the first position's base fact enters the
    // delta. This is the exact shape per-position delta seeding exists for: round 1
    // seeds `p.a.b` from the delta but `r.b.c` is not in the model yet; round 2
    // seeds the just-derived `r.b.c` from the delta at POSITION 1, and only then
    // does `chain.a.c` fire. A single-position seed (`positions.iter().take(1)`,
    // the review's mutation) seeds only position 0 and silently drops `chain.a.c`,
    // while the flagship `naive_equals_production` net stayed green — this pin
    // closes that blind spot deterministically.
    #[test]
    fn two_match_second_position_binds_a_later_derived_fact() {
        let mut i = Interner::new();
        let axs = [
            rule(&mut i, &[m("p.X.Y"), m("r.Y.Z")], &["chain.X.Z"]),
            rule(&mut i, &[m("seed.Y.Z")], &["r.Y.Z"]),
        ];
        let base = build(&mut i, &["p.a.b", "seed.b.c"]);
        let d = derived(&mut i, &axs, &base);
        assert!(d.contains(&"r.b.c".to_owned()), "r.b.c is derived from seed.b.c");
        assert!(
            d.contains(&"chain.a.c".to_owned()),
            "chain.a.c: the SECOND Match binds r.b.c, derived a round after p.a.b \
             entered the delta — dropped by single-position seeding. derived={d:?}"
        );
    }

    // H: DeriveSpec.hs "closure is a VIEW: base untouched, so derivation is defeasible"
    #[test]
    fn closure_is_a_view_so_derivation_is_defeasible() {
        let mut i = Interner::new();
        let axs = [rule(&mut i, &[m("at.W.bar")], &["in.W.building"])];
        let base = build(&mut i, &["at.bex.bar"]);
        assert!(closed_facts(&mut i, &axs, &base).contains(&"in.bex.building".to_owned()));
        assert!(!base.to_sentences(&i).contains(&"in.bex.building".to_owned()));
        // Retract the premise from the BASE and re-close: the conclusion is gone
        // with no manual undo (the base is the source of truth).
        let base2 = base.retract_str(&mut i, "at.bex.bar").unwrap();
        assert!(!closed_facts(&mut i, &axs, &base2).contains(&"in.bex.building".to_owned()));
    }

    // H: DeriveSpec.hs "⊥ DETECTED: rules forcing one exclusive slot to two values contradict"
    #[test]
    fn bottom_detected_two_exclusive_values_contradict_either_order() {
        let mut i = Interner::new();
        let a1 = rule(&mut i, &[m("trigger")], &["light!red"]);
        let a2 = rule(&mut i, &[m("trigger")], &["light!green"]);
        let base = build(&mut i, &["trigger"]);
        assert!(close(&mut i, &[a1.clone(), a2.clone()], &base).is_err());
        assert!(close(&mut i, &[a2.clone(), a1.clone()], &base).is_err());
        // The ⊥ witness names an offending head (up-to-set, DeriveSpec:75).
        let witness = close(&mut i, &[a1, a2], &base).unwrap_err();
        assert!(
            witness == Contradiction("light!red".to_owned())
                || witness == Contradiction("light!green".to_owned()),
            "witness was {witness:?}"
        );
    }

    // H: DeriveSpec.hs "consistent exclusive derivation is fine (no false ⊥)"
    #[test]
    fn consistent_exclusive_derivation_is_fine_no_false_bottom() {
        let mut i = Interner::new();
        let axs = [rule(&mut i, &[m("wedding.W")], &["status.W!married"])];
        let base = build(&mut i, &["wedding.bex"]);
        assert!(close(&mut i, &axs, &base).is_ok());
        assert!(closed_facts(&mut i, &axs, &base).contains(&"status.bex.married".to_owned()));
        // The head's `!` label survives grounding (ground_head carries the excl
        // bitmask): the closed model marks the slot exclusive.
        let closed = close(&mut i, &axs, &base).unwrap();
        assert!(closed.to_labeled_sentences(&i).contains(&"status.bex!married".to_owned()));
    }

    // H: DeriveSpec.hs "⊥ from EITHER side: a derived multi value clashes with a base EXCLUSIVE fact"
    #[test]
    fn bottom_from_either_side_derived_multi_clashes_with_base_exclusive() {
        let mut i = Interner::new();
        let axs = [rule(&mut i, &[m("summoned.W")], &["place.W.hall"])];
        // bex is exclusively at the bar; the rule derives a DIFFERENT place with `.`.
        let base = build(&mut i, &["place.bex!bar", "summoned.bex"]);
        assert_eq!(
            close(&mut i, &axs, &base),
            Err(Contradiction("place.bex.hall".to_owned()))
        );
    }

    // H: DeriveSpec.hs "closureFrom continues a closed model exactly as a from-scratch closure"
    #[test]
    fn close_from_continues_a_closed_model_exactly_as_from_scratch() {
        let mut i = Interner::new();
        let axs = [
            rule(&mut i, &[m("parent.X.Y")], &["elder.X"]),
            rule(&mut i, &[m("elder.X"), m("wise.X")], &["sage.X"]),
        ];
        let base = build(&mut i, &["parent.ada.bea", "wise.ada"]);
        let closed = close(&mut i, &axs, &base).unwrap();
        // A monotone new fact cascading through BOTH rules.
        let new_fact = tokenize(&mut i, "parent.cal.dun").unwrap();
        let cont = close_from(&mut i, &axs, &closed, std::slice::from_ref(&new_fact)).unwrap();
        let scratch = close(&mut i, &axs, &base.insert(&new_fact)).unwrap();
        assert_eq!(
            cont.to_labeled_sentences(&i),
            scratch.to_labeled_sentences(&i)
        );
    }

    // ---- head-meet is MEET, not INSERT (the singleton-meet corner laws, S-I3) ----

    // The corner where meet and insert DISAGREE: a `!` head endpoint conflicting
    // with an existing multi sibling is ⊥ under meet, but would silently clear the
    // sibling under insert. This discriminates a correct (meet) head-meet from an
    // insert-semantics one.
    #[test]
    fn head_meet_bang_endpoint_into_existing_multi_sibling_is_bottom_not_clear() {
        let mut i = Interner::new();
        let axs = [rule(&mut i, &[m("trigger")], &["x!b"])];
        let base = build(&mut i, &["x.a", "trigger"]);
        // meet({x.a}, x!b): x exclusive with children {a,b} ⇒ ⊥. Insert would give {x.b}.
        assert_eq!(
            close(&mut i, &axs, &base),
            Err(Contradiction("x!b".to_owned()))
        );
    }

    // The asserted-endpoint corner: a head asserting an existing UNASSERTED
    // interior OR-joins the asserted flag (meet's `asserted = a1 || a2`), keeping
    // both the interior's own path and its descendants'.
    #[test]
    fn head_meet_asserted_endpoint_or_joins_the_flag() {
        let mut i = Interner::new();
        let axs = [rule(&mut i, &[m("trigger")], &["a"])];
        let base = build(&mut i, &["a.b", "trigger"]);
        let closed = close(&mut i, &axs, &base).unwrap();
        let sents = closed.to_sentences(&i);
        assert!(sents.contains(&"a".to_owned()), "asserted interior emits itself");
        assert!(sents.contains(&"a.b".to_owned()), "and keeps its descendant");
    }

    // ground_head carries the head template's excl bitmask through substitution
    // (no string round-trip); an exclusive head grounds to an exclusive path.
    #[test]
    fn ground_head_carries_the_head_excl_bitmask() {
        let mut i = Interner::new();
        let head = tokenize(&mut i, "status.W!married").unwrap();
        let mut b = Bindings::new();
        b.insert(i.intern("W"), crate::db::Val::Sym(i.intern("bex")));
        let g = ground_head(&mut i, &head, &b);
        assert_eq!(render_head(&i, &g), "status.bex!married");
        assert_eq!(g.excl, head.excl);
    }

    // ---- the S4 analysis-table builders (owed:S4 discharges) ----------------
    //
    // axiomFootprint (incl. the obligedClose-lifted-forms clause) lives in
    // prax-vocab::deontic, where obligedClose is in scope; axiomNegPatterns and
    // monotoneAxioms are pure over CompiledRule and are pinned here.

    use crate::query::{CalcOp, CmpOp};

    /// Whether a pattern list contains the tokenized segments of `s`.
    fn has_path(i: &mut Interner, ps: &[SmallVec<[Sym; 6]>], s: &str) -> bool {
        let segs = tokenize(i, s).unwrap().segs;
        ps.contains(&segs)
    }

    // H: DeriveSpec.hs "axiomNegPatterns collects exactly the negated interiors"
    #[test]
    fn axiom_neg_patterns_collects_exactly_the_negated_interiors() {
        let mut i = Interner::new();
        let axs = [rule(
            &mut i,
            &[m("a.X"), Condition::Absent(vec![m("b.X"), Condition::Not("c.X".into())])],
            &["d.X"],
        )];
        let np = axiom_neg_patterns(&axs);
        assert!(has_path(&mut i, &np, "b.X"), "Absent interior");
        assert!(has_path(&mut i, &np, "c.X"), "Not inside Absent");
        assert!(!has_path(&mut i, &np, "a.X"), "positive atom is NOT negated");
    }

    // H: DeriveSpec.hs "monotoneAxioms accepts the count-threshold shape and rejects anti-monotone"
    #[test]
    fn monotone_axioms_accepts_count_threshold_and_rejects_anti_monotone() {
        let mut i = Interner::new();
        let mono = |i: &mut Interner, body: &[Condition], heads: &[&str]| {
            let r = [rule(i, body, heads)];
            monotone_axioms(i, &r)
        };
        let sub = |set: &str, find: &[&str], where_: Vec<Condition>| Condition::Subquery {
            set: set.into(),
            find: find.iter().map(|s| s.to_string()).collect(),
            where_,
        };

        assert!(mono(&mut i, &[m("a.X")], &["b.X"]), "Match-only is safe");
        assert!(
            mono(
                &mut i,
                &[sub("Rs", &["W"], vec![m("r.W.T")]),
                  Condition::Count("N".into(), "Rs".into()),
                  Condition::Cmp(CmpOp::Gte, "N".into(), "3".into())],
                &["n.T"]
            ),
            "the notoriety shape (Subquery+Count+Cmp Gte literal) is safe"
        );
        assert!(
            !mono(
                &mut i,
                &[Condition::Count("N".into(), "Rs".into()),
                  Condition::Cmp(CmpOp::Lt, "N".into(), "3".into())],
                &["q.T"]
            ),
            "Cmp Lt with the literal on the right is anti-monotone"
        );
        assert!(
            !mono(
                &mut i,
                &[Condition::Calc("M".into(), CalcOp::Add, "N".into(), "1".into())],
                &["q.M"]
            ),
            "Calc disables the tier"
        );
        assert!(
            !mono(
                &mut i,
                &[sub("Rs", &["W"], vec![m("r.W.T")]),
                  Condition::Count("N".into(), "Rs".into()),
                  Condition::Eq("N".into(), "3".into())],
                &["n.T"]
            ),
            "Eq over a count-bound variable is anti-monotone (exactly-k)"
        );
        assert!(
            !mono(
                &mut i,
                &[Condition::Count("N".into(), "Rs".into()),
                  Condition::Neq("N".into(), "3".into())],
                &["q.T"]
            ),
            "Neq over a count-bound variable is anti-monotone too"
        );
        assert!(
            mono(
                &mut i,
                &[m("a.X"), m("b.Y"), Condition::Eq("X".into(), "Y".into())],
                &["c.X"]
            ),
            "Eq over Match-bound names stays monotone"
        );
    }
}

#[cfg(test)]
mod optimization_laws {
    //! The two perf-motivated transforms in this module, pinned so a semantic
    //! regression reddens locally rather than only in a downstream differential:
    //! (1) the allocation-free [`entailed`] equals the general
    //! `leq(model, Db::empty().insert(head))` it replaced; (2) the per-round
    //! delta gate in [`run`] leaves the closure identical to an ungated run.
    use super::*;
    use crate::el::{leq, meet};
    use crate::interner::Interner;
    use proptest::prelude::*;

    fn seg() -> impl Strategy<Value = String> {
        prop::sample::select(vec!["a", "b", "c", "d"]).prop_map(String::from)
    }

    /// A random ground path of 1–4 segments with `.`/`!` separators.
    fn path_str() -> impl Strategy<Value = String> {
        prop::collection::vec((seg(), prop::bool::ANY), 1..5).prop_map(|parts| {
            let mut s = String::new();
            for (idx, (name, bang)) in parts.iter().enumerate() {
                if idx > 0 {
                    s.push(if *bang { '!' } else { '.' });
                }
                s.push_str(name);
            }
            s
        })
    }

    fn build(interner: &mut Interner, facts: &[String]) -> Db {
        let mut db = Db::empty();
        for f in facts {
            db = db.insert_str(interner, f).unwrap();
        }
        db
    }

    proptest! {
        /// [`entailed`] is exactly `leq` against the head's singleton model, over
        /// arbitrary models and heads (`!`/`.` and assertedness varied).
        #[test]
        fn entailed_matches_leq_singleton(
            facts in prop::collection::vec(path_str(), 0..8),
            head_s in path_str(),
        ) {
            let mut i = Interner::new();
            let model = build(&mut i, &facts);
            let head = tokenize(&mut i, &head_s).unwrap();
            let singleton = Db::empty().insert(&head);
            prop_assert_eq!(entailed(&model, &head), leq(&model, &singleton));
        }

        /// [`meet_one`]'s path-copy `meet_head` equals the general
        /// `meet(model, Db::empty().insert(head))` it replaced — both the ⊥ status
        /// and, when defined, the resulting model — over arbitrary models and heads.
        #[test]
        fn meet_head_matches_general_meet(
            facts in prop::collection::vec(path_str(), 0..8),
            head_s in path_str(),
        ) {
            let mut i = Interner::new();
            let model = build(&mut i, &facts);
            let head = tokenize(&mut i, &head_s).unwrap();
            let singleton = Db::empty().insert(&head);
            match (meet_one(&model, &head), meet(&model, &singleton)) {
                (Some(a), Some(b)) => {
                    prop_assert_eq!(a.to_labeled_sentences(&i), b.to_labeled_sentences(&i));
                }
                (None, None) => {}
                (a, b) => prop_assert!(
                    false, "meet_head ⊥-status differs: {:?} vs {:?}",
                    a.map(|d| d.to_labeled_sentences(&i)),
                    b.map(|d| d.to_labeled_sentences(&i)),
                ),
            }
        }
    }

    /// The naive full-query oracle with the SAME loop as [`run`] but the delta
    /// gate forced OFF (`delta_facts = None` every round) — the reference the
    /// gated `run` must match fact-for-fact.
    fn run_ungated(
        interner: &mut Interner,
        rules: &[CompiledRule],
        mut model: Db,
        mut delta: Db,
    ) -> Result<Db, Contradiction> {
        loop {
            let mut fresh: Vec<(String, CompiledPath)> = Vec::new();
            for rule in rules {
                for b in rule_bindings(interner, rule, &model, &delta) {
                    for h in &rule.heads {
                        let g = ground_head(interner, h, &b);
                        if !entailed(&model, &g) {
                            fresh.push((render_head(interner, &g), g));
                        }
                    }
                }
            }
            if fresh.is_empty() {
                return Ok(model);
            }
            fresh.sort_by(|a, b| a.0.cmp(&b.0));
            fresh.dedup_by(|a, b| a.0 == b.0);
            let mut next_delta = Db::empty();
            for (_, g) in &fresh {
                next_delta = next_delta.insert(g);
            }
            let mut next_model = model;
            for (name, g) in &fresh {
                next_model = match meet_one(&next_model, g) {
                    Some(m) => m,
                    None => return Err(Contradiction(name.clone())),
                };
            }
            model = next_model;
            delta = next_delta;
        }
    }

    fn cond_m(s: &str) -> Condition {
        Condition::Match(s.to_owned())
    }

    proptest! {
        /// The delta-gated [`close_from`] equals an ungated continuation over a
        /// transitive rule set (`a→b→c→d`) and arbitrary base + continuation
        /// facts — the gate must never drop a derivation.
        #[test]
        fn gated_close_from_equals_ungated(
            base_facts in prop::collection::vec(path_str(), 0..6),
            cont_facts in prop::collection::vec(path_str(), 0..4),
        ) {
            let mut i = Interner::new();
            // A transitive chain plus a join, so later rounds genuinely fire.
            let rules = vec![
                CompiledRule::compile(&mut i, &[cond_m("a.X")], &["b.X"]).unwrap(),
                CompiledRule::compile(&mut i, &[cond_m("b.X")], &["c.X"]).unwrap(),
                CompiledRule::compile(&mut i, &[cond_m("c.X"), cond_m("d.X")], &["e.X"]).unwrap(),
            ];
            let base = build(&mut i, &base_facts);
            // A self-contradictory base is off-topic here (the continuation tier's
            // precondition is a consistent closed model); skip it.
            let Ok(closed) = close(&mut i, &rules, &base) else {
                return Ok(());
            };
            let cont: Vec<CompiledPath> =
                cont_facts.iter().map(|f| tokenize(&mut i, f).unwrap()).collect();

            // Gated continuation (production path).
            let gated = close_from(&mut i, &rules, &closed, &cont);
            // Ungated reference: the same starting model/delta, gate off.
            let mut model = closed.clone();
            let mut d = Db::empty();
            for f in &cont {
                model = model.insert(f);
                d = d.insert(f);
            }
            let ungated = run_ungated(&mut i, &rules, model, d);

            match (gated, ungated) {
                (Ok(g), Ok(u)) => {
                    prop_assert_eq!(g.to_labeled_sentences(&i), u.to_labeled_sentences(&i));
                }
                (Err(g), Err(u)) => prop_assert_eq!(g, u),
                (g, u) => prop_assert!(false, "gate changed ⊥ status: {:?} vs {:?}", g, u),
            }
        }
    }
}
