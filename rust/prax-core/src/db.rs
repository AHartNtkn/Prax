//! The exclusion-logic database: a trie of interned segments whose edges are
//! `.` (multi) or `!` (exclusion). Represented as `Arc` path-copy persistent
//! nodes with sorted `SmallVec` children, so the planner's apply-and-discard
//! clone model is a cheap structural share. Carries the corrected `!` semantics
//! (siblings cleared, the surviving child's subtree preserved) and the v39
//! asserted-flag law (no unasserted childless node survives).
//!
//! Frozen reference: `src/Prax/Db.hs`. The Haskell keys an `IntMap` on raw
//! `symId`s and re-sorts BY NAME at every enumeration point; here children are a
//! `SmallVec<[(Sym, Db); 4]>` kept sorted by `Sym` id (the internal key order),
//! and every observable enumeration (unify's unbound branch, `child_keys`,
//! `to_sentences`, `to_labeled_sentences`) sorts by name. `Sym` ids never leak
//! into output — the determinism contract (PLAN.md).

use std::sync::Arc;

use smallvec::SmallVec;

use crate::error::WorldError;
use crate::interner::{Interner, Sym};
use crate::path::{CompiledPath, tokenize};

/// A value a logic variable can be bound to. [`unify`](Db::unify) only ever
/// produces [`Val::Sym`] (trie keys are symbols); [`Val::Num`] and [`Val::Set`]
/// arise from query operators (`calc`, subqueries) in the query layer.
/// `Integer` becomes `i64` (a recorded deviation, ARCHITECTURE.md); arithmetic
/// that touches it is checked at the query layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Val {
    Sym(Sym),
    Num(i64),
    Set(Vec<Vec<Sym>>),
}

/// Render a value the way `Prax.Db.valToString` does: a symbol resolves to its
/// name, a number to its decimal, a set to the opaque `<Set(n)>` marker (sets
/// are not meant to be grounded into sentences).
pub fn val_to_string(interner: &Interner, v: &Val) -> String {
    match v {
        Val::Sym(s) => interner.resolve(*s).to_owned(),
        Val::Num(n) => n.to_string(),
        Val::Set(xs) => format!("<Set({})>", xs.len()),
    }
}

/// The symbol a value substitutes as when grounding a token position
/// (`Prax.Db.valToSym`): a bound [`Val::Sym`] is returned as-is (no
/// render/re-intern round trip); any other value is rendered via
/// [`val_to_string`] and interned. Consistent with [`val_to_string`] by
/// construction, since interning is a deterministic injective map on strings.
pub fn val_to_sym(interner: &mut Interner, v: &Val) -> Sym {
    match v {
        Val::Sym(s) => *s,
        _ => {
            let rendered = val_to_string(interner, v);
            interner.intern(&rendered)
        }
    }
}

/// Map of logic-variable symbol to bound value — the representation the engine
/// computes over natively (`Prax.Db.Bindings`). Kept sorted by `Sym` id so
/// equality is insertion-order-independent; strings appear only at the
/// authoring/display boundary.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bindings(SmallVec<[(Sym, Val); 8]>);

impl Bindings {
    /// An empty binding set.
    pub fn new() -> Bindings {
        Bindings(SmallVec::new())
    }

    /// The value bound to `key`, if any.
    pub fn get(&self, key: Sym) -> Option<&Val> {
        match self.0.binary_search_by(|(s, _)| s.id().cmp(&key.id())) {
            Ok(idx) => Some(&self.0[idx].1),
            Err(_) => None,
        }
    }

    /// Bind `key` to `value`, replacing any existing binding. Maintains the
    /// sorted-by-id invariant.
    pub fn insert(&mut self, key: Sym, value: Val) {
        match self.0.binary_search_by(|(s, _)| s.id().cmp(&key.id())) {
            Ok(idx) => self.0[idx].1 = value,
            Err(idx) => self.0.insert(idx, (key, value)),
        }
    }

    /// Iterate the bindings in `Sym`-id order.
    pub fn iter(&self) -> impl Iterator<Item = (Sym, &Val)> {
        self.0.iter().map(|(s, v)| (*s, v))
    }

    /// Whether there are no bindings.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A trie node: whether its outgoing edges are exclusive (`!`), whether the path
/// to it was itself asserted as a fact, and its children keyed by segment,
/// sorted by `Sym` id.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Node {
    excl: bool,
    asserted: bool,
    kids: SmallVec<[(Sym, Db); 4]>,
}

/// The world state: an exclusion trie under `Arc` path-copy persistence. Cloning
/// a `Db` bumps a refcount (the planner's apply-and-discard model depends on
/// this); a mutation rebuilds only the nodes on the touched path, sharing the
/// rest structurally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Db(Arc<Node>);

/// Locate the child slot for key `k` among sorted children (`Ok` = index of the
/// existing child, `Err` = insertion point preserving id order).
fn kid_index(kids: &[(Sym, Db)], k: Sym) -> Result<usize, usize> {
    kids.binary_search_by(|(s, _)| s.id().cmp(&k.id()))
}

/// The head-singleton's subtree rooted at `segs[j]` (no model node to meet
/// with): the bare chain `segs[j..]` with each node's `excl = is_excl_after(k)`
/// and only the leaf asserted — exactly what `Db::empty().insert(head)` builds
/// below `segs[j]`. The meet drops it in wholesale where the model has no
/// existing node.
fn head_subchain(head: &CompiledPath, j: usize) -> Db {
    let n = head.segs.len();
    let mut node = Db::from_parts(head.is_excl_after(n - 1), true, Vec::new());
    for k in (j..n - 1).rev() {
        node = Db::from_parts(head.is_excl_after(k), false, vec![(head.segs[k + 1], node)]);
    }
    node
}

impl Db {
    /// The empty database.
    pub fn empty() -> Db {
        Db(Arc::new(Node {
            excl: false,
            asserted: false,
            kids: SmallVec::new(),
        }))
    }

    /// Whether this node's outgoing edges are exclusive (`!`).
    pub fn is_excl(&self) -> bool {
        self.0.excl
    }

    /// Whether the path to this node was itself asserted as a fact.
    pub fn is_asserted(&self) -> bool {
        self.0.asserted
    }

    /// The child under segment `k`, if present.
    pub(crate) fn child(&self, k: Sym) -> Option<&Db> {
        kid_index(&self.0.kids, k)
            .ok()
            .map(|idx| &self.0.kids[idx].1)
    }

    /// The children as an id-sorted slice — for the lattice ([`crate::el`]),
    /// which walks two tries in merge order.
    pub(crate) fn kids(&self) -> &[(Sym, Db)] {
        &self.0.kids
    }

    /// Construct a node from parts, sorting the children by id to preserve the
    /// trie's internal key order — the constructor the lattice uses to rebuild
    /// merged nodes.
    pub(crate) fn from_parts(excl: bool, asserted: bool, mut kids: Vec<(Sym, Db)>) -> Db {
        kids.sort_by(|(a, _), (b, _)| a.id().cmp(&b.id()));
        Db(Arc::new(Node {
            excl,
            asserted,
            kids: kids.into_iter().collect(),
        }))
    }

    /// Whether a node may be eagerly pruned: unasserted AND childless (the v39
    /// invariant's discriminator).
    fn prunable(&self) -> bool {
        !self.0.asserted && self.0.kids.is_empty()
    }

    // ---- insert ----------------------------------------------------------

    /// Insert a tokenized path, with the corrected exclusion rule and v39
    /// endpoint marking (`Prax.Db.insertToks`). `!` after a segment makes that
    /// segment single-valued: its other children are cleared but the surviving
    /// child's subtree is preserved. The endpoint node is marked asserted;
    /// interior marks are carried through untouched.
    pub fn insert(&self, path: &CompiledPath) -> Db {
        self.clone().inserted(path)
    }

    /// The consuming form of [`insert`]. When the receiver's nodes are uniquely
    /// owned (`Arc` refcount 1) — every trie the closure builds up fresh
    /// (`next_delta`, and the `model` it threads round to round) — the descent
    /// mutates in place via `Arc::make_mut`, allocating nothing; a node shared
    /// with a planner fork is cloned exactly where the old path-copy cloned it.
    /// Never worse than [`insert`], strictly cheaper on owned tries — and the
    /// trie path-copy's `node.kids.clone()` was the dominant allocation.
    pub fn inserted(mut self, path: &CompiledPath) -> Db {
        self.insert_in_place(path, 0);
        self
    }

    /// In-place mirror of [`insert_at`]: `Arc::make_mut` gives a uniquely-owned
    /// `&mut Node` (cloning iff shared), then the exact same exclusion/endpoint
    /// logic mutates it. Byte-identical result to `insert_at`.
    fn insert_in_place(&mut self, path: &CompiledPath, i: usize) {
        let node = Arc::make_mut(&mut self.0);
        if i == path.segs.len() {
            node.asserted = true;
            return;
        }
        let n = path.segs[i];
        let child_excl = path.is_excl_after(i);
        let idx = match kid_index(&node.kids, n) {
            Ok(idx) => idx,
            Err(ins) => {
                node.kids.insert(
                    ins,
                    (
                        n,
                        Db(Arc::new(Node {
                            excl: child_excl,
                            asserted: false,
                            kids: SmallVec::new(),
                        })),
                    ),
                );
                ins
            }
        };
        {
            // The child edge's operator becomes this insert's (`.`/`!`), exactly
            // as `insert_at` rebuilt the child with `excl: child_excl`; its
            // asserted mark is preserved. `!` before a further segment sheds the
            // child's siblings but keeps the surviving branch's subtree.
            let child = &mut node.kids[idx].1;
            let cnode = Arc::make_mut(&mut child.0);
            cnode.excl = child_excl;
            if child_excl && i + 1 < path.segs.len() {
                let next = path.segs[i + 1];
                cnode.kids.retain(|(s, _)| s.id() == next.id());
            }
        }
        node.kids[idx].1.insert_in_place(path, i + 1);
    }

    fn insert_at(&self, path: &CompiledPath, i: usize) -> Db {
        let node = &*self.0;
        if i == path.segs.len() {
            // Endpoint: the path to here IS a fact — mark it, keep excl + kids.
            return Db(Arc::new(Node {
                excl: node.excl,
                asserted: true,
                kids: node.kids.clone(),
            }));
        }

        let n = path.segs[i];
        let child_excl = path.is_excl_after(i);

        // The existing child under n (empty scaffold if absent). Its asserted
        // mark is carried through; its children are the base we descend into.
        let (existing_kids, existing_asserted): (SmallVec<[(Sym, Db); 4]>, bool) =
            match kid_index(&node.kids, n) {
                Ok(idx) => {
                    let c = &node.kids[idx].1;
                    (c.0.kids.clone(), c.0.asserted)
                }
                Err(_) => (SmallVec::new(), false),
            };

        // Exclusion: if this insert reaches n via `!` and there is a next
        // segment, n keeps only that next child (with its subtree intact) and
        // sheds its siblings.
        let cleared_kids: SmallVec<[(Sym, Db); 4]> = if child_excl && i + 1 < path.segs.len() {
            let next = path.segs[i + 1];
            existing_kids
                .into_iter()
                .filter(|(s, _)| s.id() == next.id())
                .collect()
        } else {
            existing_kids
        };

        let base = Db(Arc::new(Node {
            excl: child_excl,
            asserted: existing_asserted,
            kids: cleared_kids,
        }));
        let new_child = base.insert_at(path, i + 1);

        let mut kids = node.kids.clone();
        match kid_index(&kids, n) {
            Ok(idx) => kids[idx].1 = new_child,
            Err(idx) => kids.insert(idx, (n, new_child)),
        }
        Db(Arc::new(Node {
            excl: node.excl,
            asserted: node.asserted,
            kids,
        }))
    }

    // ---- meet (EL greatest-lower-bound of a derived head) ----------------

    /// Meet a single grounded head into this model IN PLACE (the closure's
    /// per-round model build). Equivalent to `meet(model, singleton(head))` and
    /// to the former path-copy `meet_head`, but `Arc::make_mut` mutates the head's
    /// spine directly when the model is uniquely owned (which it is inside the
    /// fixpoint loop), sharing every untouched subtree and allocating nothing on
    /// the spine — where the old meet cloned a child vector at every level. `None`
    /// = ⊥ (an exclusive node left with more than one child); on ⊥ the caller
    /// discards the receiver, so a partial mutation is harmless. Pinned
    /// bit-for-bit against the general `meet` by `meet_head_matches_general_meet`.
    pub fn met_one(mut self, head: &CompiledPath) -> Option<Db> {
        self.meet_root(head)?;
        Some(self)
    }

    /// Root level: the root is never exclusive and the head never asserts it, so
    /// just meet the head's first segment's subtree in (mirrors the former
    /// `meet_head` at `i == 0`).
    fn meet_root(&mut self, head: &CompiledPath) -> Option<()> {
        let seg = head.segs[0];
        let node = Arc::make_mut(&mut self.0);
        match kid_index(&node.kids, seg) {
            Ok(idx) => node.kids[idx].1.meet_node(head, 0)?,
            Err(ins) => node.kids.insert(ins, (seg, head_subchain(head, 0))),
        }
        Some(())
    }

    /// Meet the head into the model node for `segs[i]` (mirrors the former
    /// `meet_head_node`): OR-join `excl`/`asserted`, meet the single child
    /// `segs[i+1]`, then report ⊥ if an exclusive node is left multi-child.
    fn meet_node(&mut self, head: &CompiledPath, i: usize) -> Option<()> {
        let node = Arc::make_mut(&mut self.0);
        node.excl = node.excl || head.is_excl_after(i);
        let leaf = i + 1 == head.segs.len();
        if leaf {
            node.asserted = true;
        } else {
            let seg1 = head.segs[i + 1];
            match kid_index(&node.kids, seg1) {
                Ok(idx) => node.kids[idx].1.meet_node(head, i + 1)?,
                Err(ins) => node.kids.insert(ins, (seg1, head_subchain(head, i + 1))),
            }
        }
        if node.excl && node.kids.len() > 1 {
            return None;
        }
        Some(())
    }

    // ---- retract ---------------------------------------------------------
    // (helper `head_subchain` for meet lives just below the impl block.)

    /// Retract the subtree named by `names` (operators are irrelevant to
    /// retract), then eagerly prune every ancestor the deletion leaves
    /// unasserted and childless (`Prax.Db.retractNames`). Establishes the v39
    /// invariant: the trie never contains an unasserted childless node.
    pub fn retract(&self, names: &[Sym]) -> Db {
        self.retract_at(names, 0)
    }

    fn retract_at(&self, names: &[Sym], i: usize) -> Db {
        let node = &*self.0;
        if i == names.len() {
            // retractNames [] db = db
            return self.clone();
        }
        let n = names[i];

        if i == names.len() - 1 {
            // The [n] case: delete n's entry; the caller prunes if this leaves
            // *its* node unasserted and childless.
            let mut kids = node.kids.clone();
            if let Ok(idx) = kid_index(&kids, n) {
                kids.remove(idx);
            }
            return Db(Arc::new(Node {
                excl: node.excl,
                asserted: node.asserted,
                kids,
            }));
        }

        match kid_index(&node.kids, n) {
            Err(_) => self.clone(), // path absent → no-op
            Ok(idx) => {
                let child = &node.kids[idx].1;
                let child2 = child.retract_at(names, i + 1);
                let mut kids = node.kids.clone();
                if child2.prunable() {
                    kids.remove(idx);
                } else {
                    kids[idx].1 = child2;
                }
                Db(Arc::new(Node {
                    excl: node.excl,
                    asserted: node.asserted,
                    kids,
                }))
            }
        }
    }

    // ---- unify / exists --------------------------------------------------

    /// The unification core (`Prax.Db.unifySyms`): descend the trie by the
    /// tokenized pattern, threading `bindings`. A constant or bound-variable
    /// segment descends deterministically; an unbound variable branches over all
    /// children IN NAME ORDER — the determinism contract. Yields every
    /// consistent extension of `bindings`.
    pub fn unify(
        &self,
        interner: &mut Interner,
        segs: &[Sym],
        bindings: Bindings,
    ) -> Vec<Bindings> {
        let mut out = Vec::new();
        self.unify_into(interner, segs, bindings, &mut out);
        out
    }

    /// [`unify`](Db::unify) that APPENDS its bindings to `out` rather than
    /// returning a fresh `Vec` — the query layer's [`Cond::Match`] threads through
    /// here, so a conjunct's results land straight in the accumulator with no
    /// intermediate allocation or copy. Order is identical to [`unify`]'s.
    pub fn unify_into(
        &self,
        interner: &mut Interner,
        segs: &[Sym],
        bindings: Bindings,
        out: &mut Vec<Bindings>,
    ) {
        // Scratch buffers stay on the stack for the overwhelmingly common
        // low-fan-out match (a pattern that descends constants and branches at
        // one or two vars); only a genuinely wide unbound branch spills to the
        // heap. `unify_into` is the closure/planner's leaf, called an enormous
        // number of times, so the removed per-call `Vec` allocation is the win.
        let mut worlds: SmallVec<[(Db, Bindings); 8]> = SmallVec::new();
        worlds.push((self.clone(), bindings));
        for &sym in segs {
            let mut next: SmallVec<[(Db, Bindings); 8]> = SmallVec::with_capacity(worlds.len());
            for (db, b) in worlds {
                if sym.is_var() {
                    match b.get(sym) {
                        Some(v) => {
                            // Bound variable: descend deterministically; the binding
                            // is unchanged, so MOVE it (no clone).
                            let key = val_to_sym(interner, v);
                            if let Some(sub) = db.child(key) {
                                next.push((sub.clone(), b));
                            }
                        }
                        None => {
                            // Unbound variable: branch over children IN NAME ORDER.
                            // Sort child INDICES (not a cloned child vector) to avoid
                            // an owned `(Sym, Db)` copy of every child per node, and
                            // MOVE `b` into the final branch (clone only the rest).
                            let kids = &db.0.kids;
                            let mut order: SmallVec<[usize; 8]> = (0..kids.len()).collect();
                            order.sort_by(|&x, &y| interner.cmp_by_name(kids[x].0, kids[y].0));
                            if let Some((&last_idx, rest)) = order.split_last() {
                                for &idx in rest {
                                    let (k, sub) = &kids[idx];
                                    let mut b2 = b.clone();
                                    b2.insert(sym, Val::Sym(*k));
                                    next.push((sub.clone(), b2));
                                }
                                let (k, sub) = &kids[last_idx];
                                let mut b = b;
                                b.insert(sym, Val::Sym(*k));
                                next.push((sub.clone(), b));
                            }
                        }
                    }
                } else if let Some(sub) = db.child(sym) {
                    // Constant segment: the binding is unchanged, so MOVE it.
                    next.push((sub.clone(), b));
                }
            }
            worlds = next;
        }
        out.reserve(worlds.len());
        for (_, b) in worlds {
            out.push(b);
        }
    }

    /// Whether any node exists at the given (constant) path — `Prax.Db.exists`,
    /// which is `not (null (unify path db empty))`.
    pub fn exists(&self, interner: &mut Interner, segs: &[Sym]) -> bool {
        !self.unify(interner, segs, Bindings::new()).is_empty()
    }

    /// The keys directly beneath the node at a constant path, sorted by name, or
    /// empty if the path is absent (`Prax.Db.childKeys`).
    pub fn child_keys(&self, interner: &Interner, segs: &[Sym]) -> Vec<String> {
        let mut cur = self;
        for &n in segs {
            match cur.child(n) {
                Some(c) => cur = c,
                None => return Vec::new(),
            }
        }
        let mut names: Vec<String> = cur
            .0
            .kids
            .iter()
            .map(|(s, _)| interner.resolve(*s).to_owned())
            .collect();
        names.sort();
        names
    }

    // ---- enumeration -----------------------------------------------------

    /// Enumerate the facts, sorted, joined by `.` (flattening the `.`/`!`
    /// distinction) — `Prax.Db.dbToSentences`, for display, matching, tests.
    /// A childless node is a fact; an asserted node with children emits both its
    /// own path and its descendants'. Under the v39 invariant this reads
    /// presence exactly.
    pub fn to_sentences(&self, interner: &Interner) -> Vec<String> {
        let mut out = Vec::new();
        self.collect_sentences(interner, &mut out);
        out.sort();
        out
    }

    fn collect_sentences(&self, interner: &Interner, out: &mut Vec<String>) {
        for (k, child) in self.0.kids.iter() {
            let name = interner.resolve(*k);
            let cnode = &*child.0;
            if cnode.kids.is_empty() {
                out.push(name.to_owned());
            } else {
                if cnode.asserted {
                    out.push(name.to_owned());
                }
                let mut subs = Vec::new();
                child.collect_sentences(interner, &mut subs);
                for s in subs {
                    out.push(format!("{name}.{s}"));
                }
            }
        }
    }

    /// Like [`to_sentences`](Db::to_sentences) but label-faithful: each edge is
    /// re-emitted with `!` when its child node is exclusive, else `.`; an
    /// asserted interior node emits its own bare labeled path alongside its
    /// descendants'. Inverse of insertion — the basis for exact serialization
    /// (`Prax.Db.dbToLabeledSentences`).
    pub fn to_labeled_sentences(&self, interner: &Interner) -> Vec<String> {
        let mut out = Vec::new();
        self.collect_labeled(interner, &mut out);
        out.sort();
        out
    }

    fn collect_labeled(&self, interner: &Interner, out: &mut Vec<String>) {
        for (k, child) in self.0.kids.iter() {
            let name = interner.resolve(*k);
            let mut subs = Vec::new();
            child.collect_labeled(interner, &mut subs);
            if subs.is_empty() {
                out.push(name.to_owned());
            } else {
                let sep = if child.0.excl { '!' } else { '.' };
                for s in subs {
                    out.push(format!("{name}{sep}{s}"));
                }
                if child.0.asserted {
                    out.push(name.to_owned());
                }
            }
        }
    }

    // ---- string-facing conveniences (authoring boundary) -----------------

    /// Tokenize and insert a sentence — the authoring-boundary convenience.
    pub fn insert_str(&self, interner: &mut Interner, sentence: &str) -> Result<Db, WorldError> {
        let path = tokenize(interner, sentence)?;
        Ok(self.insert(&path))
    }

    /// Tokenize and retract a sentence (operators ignored for retract).
    pub fn retract_str(&self, interner: &mut Interner, sentence: &str) -> Result<Db, WorldError> {
        let path = tokenize(interner, sentence)?;
        Ok(self.retract(&path.segs))
    }

    /// Tokenize a constant path and test presence.
    pub fn exists_str(&self, interner: &mut Interner, sentence: &str) -> Result<bool, WorldError> {
        let path = tokenize(interner, sentence)?;
        Ok(self.exists(interner, &path.segs))
    }
}

/// Substitute bound values into a tokenized path's variable segments, producing
/// a grounded [`CompiledPath`] — no string round trip (`Prax.Db.groundTokens`).
/// The `excl` bitmask (the path's `!`/`.` labels) is carried through unchanged.
/// An unbound head variable grounds to its own name; a bound value substitutes
/// via [`val_to_sym`] (a [`Val::Sym`] as-is, any other value rendered then
/// interned). Bindings are separator-free (every unify-produced value is a single
/// trie key; a `Num` renders as a decimal, a `Set` as an opaque marker), so the
/// segment count is preserved and the bitmask stays aligned (`debug_assert`ed).
pub fn ground_tokens(interner: &mut Interner, path: &CompiledPath, b: &Bindings) -> CompiledPath {
    let mut segs: SmallVec<[Sym; 6]> = SmallVec::with_capacity(path.segs.len());
    for &seg in &path.segs {
        let s = if seg.is_var() {
            match b.get(seg) {
                Some(v) => val_to_sym(interner, v),
                None => seg,
            }
        } else {
            seg
        };
        debug_assert!(
            !interner.resolve(s).contains(['.', '!']),
            "a grounded binding must be separator-free (groundTokens invariant): {:?}",
            interner.resolve(s)
        );
        segs.push(s);
    }
    CompiledPath {
        segs,
        excl: path.excl,
    }
}

/// Substitute bound variables into a tokenized path, preserving `.`/`!`, and
/// re-emit as a sentence (`Prax.Db.ground` / `groundTokens` +
/// `tokensToSentence`). An unbound variable grounds to its own name; a bound
/// value renders via [`val_to_string`].
pub fn ground(interner: &Interner, path: &CompiledPath, bindings: &Bindings) -> String {
    let n = path.segs.len();
    let mut out = String::new();
    for (i, &seg) in path.segs.iter().enumerate() {
        let rendered = if seg.is_var() {
            match bindings.get(seg) {
                Some(v) => val_to_string(interner, v),
                None => interner.resolve(seg).to_owned(),
            }
        } else {
            interner.resolve(seg).to_owned()
        };
        out.push_str(&rendered);
        if i + 1 < n {
            out.push(if path.is_excl_after(i) { '!' } else { '.' });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    // H: DbSpec.hs "Prax.Db"
    //
    // The frozen `Prax.DbSpec`, re-expressed against the Rust engine. The
    // tokenizer group ("tokens: trailing-operator rejection") lives with the
    // tokenizer in `crate::path`; every other DbSpec group is below.
    use super::*;
    use crate::path::{path_names, tokenize};

    /// Build a database from sentences inserted left to right (`DbSpec.build`).
    fn build(interner: &mut Interner, sentences: &[&str]) -> Db {
        let mut db = Db::empty();
        for s in sentences {
            db = db.insert_str(interner, s).unwrap();
        }
        db
    }

    /// The structural-sharing invariant the planner's apply-and-discard
    /// depends on (ARCHITECTURE.md): a clone is a refcount bump, an insert
    /// path-copies only the touched spine, and every untouched subtree stays
    /// SHARED (pointer-equal) between the original and the derived state. A
    /// representation change that silently deep-copies flips this red long
    /// before it flips a profile.
    #[test]
    fn clone_is_refcount_bump_and_untouched_subtrees_stay_shared() {
        let mut i = Interner::new();
        let db = build(
            &mut i,
            &["deep.subtree.with.leaves", "deep.subtree.other", "elsewhere.branch"],
        );

        // Clone: the roots are one allocation.
        let cloned = db.clone();
        assert!(
            Arc::ptr_eq(&db.0, &cloned.0),
            "a Db clone must be a refcount bump, not a copy"
        );

        // Insert along `elsewhere.*`: the untouched `deep` subtree must remain
        // pointer-shared between the original and the derived db.
        let grown = db.insert_str(&mut i, "elsewhere.new.leaf").unwrap();
        let deep = i.intern("deep");
        let old_deep = db.child(deep).expect("deep in original");
        let new_deep = grown.child(deep).expect("deep in derived");
        assert!(
            Arc::ptr_eq(&old_deep.0, &new_deep.0),
            "an insert must path-copy only the touched spine; untouched subtrees stay shared"
        );
        // And the touched spine is NOT shared (the copy really happened).
        let elsewhere = i.intern("elsewhere");
        let old_e = db.child(elsewhere).expect("elsewhere in original");
        let new_e = grown.child(elsewhere).expect("elsewhere in derived");
        assert!(
            !Arc::ptr_eq(&old_e.0, &new_e.0),
            "the touched spine must be a fresh copy"
        );
    }

    /// Conjunctively unify a list of sentences, threading bindings
    /// (`Prax.Db.unifyAll`).
    fn unify_all(db: &Db, interner: &mut Interner, patterns: &[&str]) -> Vec<Bindings> {
        let mut bss = vec![Bindings::new()];
        for pat in patterns {
            let path = tokenize(interner, pat).unwrap();
            let mut next = Vec::new();
            for b in bss {
                next.extend(db.unify(interner, &path.segs, b));
            }
            bss = next;
        }
        bss
    }

    fn sym_name<'a>(interner: &'a Interner, v: Option<&Val>) -> &'a str {
        match v {
            Some(Val::Sym(s)) => interner.resolve(*s),
            other => panic!("expected a bound VSym, got {other:?}"),
        }
    }

    // ===== insert / dbToSentences =====
    // H: DbSpec.hs "insert / dbToSentences"

    // H: DbSpec.hs "basic multi-valued facts"
    #[test]
    fn basic_multi_valued_facts() {
        let mut i = Interner::new();
        let db = build(&mut i, &["foo.bar.baz", "foo.bar.woof", "foo.meow.woof"]);
        assert_eq!(
            db.to_sentences(&i),
            ["foo.bar.baz", "foo.bar.woof", "foo.meow.woof"]
        );
    }

    // H: DbSpec.hs "exclusion replaces the old single value (x.age!32 then x.age!33)"
    #[test]
    fn exclusion_replaces_old_single_value() {
        let mut i = Interner::new();
        let db = build(&mut i, &["x.age!32", "x.age!33"]);
        assert_eq!(db.to_sentences(&i), ["x.age.33"]);
    }

    // H: DbSpec.hs "REGRESSION: re-asserting an ! parent preserves its existing subtree"
    #[test]
    fn regression_reasserting_bang_parent_preserves_subtree() {
        // The Praxish `!` bug: inserting `foo!bar.meow` after `foo!bar.baz` must
        // keep `baz` — the exclusion clears siblings of `bar`, not `bar`'s own
        // subtree.
        let mut i = Interner::new();
        let db = build(&mut i, &["foo!bar.baz", "foo!bar.meow"]);
        assert_eq!(db.to_sentences(&i), ["foo.bar.baz", "foo.bar.meow"]);
    }

    // H: DbSpec.hs "exclusion clears siblings when the ! child changes"
    #[test]
    fn exclusion_clears_siblings_when_bang_child_changes() {
        let mut i = Interner::new();
        let db = build(&mut i, &["p!a.x", "p!b.y"]);
        assert_eq!(db.to_sentences(&i), ["p.b.y"]);
    }

    // H: DbSpec.hs "dot under an ! child accumulates"
    #[test]
    fn dot_under_bang_child_accumulates() {
        let mut i = Interner::new();
        let db = build(&mut i, &["g!closingStar!prebeginning"]);
        assert_eq!(db.to_sentences(&i), ["g.closingStar.prebeginning"]);
    }

    // ===== retract =====
    // H: DbSpec.hs "retract"

    // H: DbSpec.hs "removes a subtree by prefix"
    #[test]
    fn retract_removes_subtree_by_prefix() {
        let mut i = Interner::new();
        let db = build(&mut i, &["foo.bar.baz", "foo.meow.woof"]);
        let db = db.retract_str(&mut i, "foo.bar").unwrap();
        assert_eq!(db.to_sentences(&i), ["foo.meow.woof"]);
    }

    // H: DbSpec.hs "retracting a missing path is a no-op"
    #[test]
    fn retracting_missing_path_is_a_noop() {
        let mut i = Interner::new();
        let db = build(&mut i, &["foo.bar"]);
        let db = db.retract_str(&mut i, "nope.nothere").unwrap();
        assert_eq!(db.to_sentences(&i), ["foo.bar"]);
    }

    // H: DbSpec.hs "INSTANCE PERSISTENCE: an asserted instance survives its transient children draining to nothing"
    #[test]
    fn instance_persistence_asserted_instance_survives_drain() {
        // Bar's `tendBarP` at the Db level: an instance fact asserted at spawn
        // doubles as the parent namespace for fully-drainable transient state.
        // Draining the last transient child must NOT take the instance with it.
        let mut i = Interner::new();
        let instance = "practice.tendBar.bar.ada";
        let db = build(&mut i, &[instance]);
        let db = db
            .insert_str(&mut i, &format!("{instance}.customer.you!order!beer"))
            .unwrap();
        let db = db
            .retract_str(&mut i, &format!("{instance}.customer.you!order"))
            .unwrap();
        let db = db
            .insert_str(&mut i, &format!("{instance}.customer.you!beverage!beer"))
            .unwrap();
        let db = db
            .retract_str(&mut i, &format!("{instance}.customer.you!beverage"))
            .unwrap();

        assert!(
            db.exists_str(&mut i, instance).unwrap(),
            "instance survives"
        );
        assert!(
            !db.exists_str(&mut i, &format!("{instance}.customer.you"))
                .unwrap(),
            "drained transient scaffold is pruned"
        );
        assert_eq!(db.to_sentences(&i), [instance]);
    }

    // H: DbSpec.hs "sibling and shared ancestors survive retracting the other sibling"
    #[test]
    fn siblings_and_shared_ancestors_survive() {
        let mut i = Interner::new();
        let db = build(
            &mut i,
            &[
                "eve.lied.dana.stole.carol.loaf",
                "eve.lied.dana.stole.carol.purse",
            ],
        );
        let db = db
            .retract_str(&mut i, "eve.lied.dana.stole.carol.loaf")
            .unwrap();
        for (path, present) in [
            ("eve.lied.dana.stole.carol.loaf", false),
            ("eve.lied.dana.stole.carol.purse", true),
            ("eve.lied.dana.stole.carol", true),
            ("eve.lied.dana.stole", true),
            ("eve.lied.dana", true),
            ("eve.lied", true),
            ("eve", true),
        ] {
            assert_eq!(db.exists_str(&mut i, path).unwrap(), present, "{path}");
        }
        assert_eq!(db.to_sentences(&i), ["eve.lied.dana.stole.carol.purse"]);
    }

    // H: DbSpec.hs "v38 repro: retracting the last targeted leaf prunes the drained `toward` ancestor"
    #[test]
    fn v38_repro_last_leaf_retract_prunes_toward_ancestor() {
        let mut i = Interner::new();
        let db = build(&mut i, &["carol.feels.angry.toward.bob"]);
        let db = db
            .retract_str(&mut i, "carol.feels.angry.toward.bob")
            .unwrap();
        assert!(!db.exists_str(&mut i, "carol.feels.angry.toward").unwrap());
        assert!(!db.exists_str(&mut i, "carol.feels.angry").unwrap());
        assert!(db.to_sentences(&i).is_empty());
    }

    // H: DbSpec.hs "re-asserted scaffold: an explicitly asserted prefix survives its deep leaf retract"
    #[test]
    fn reasserted_scaffold_survives_deep_leaf_retract() {
        let mut i = Interner::new();
        let db = build(
            &mut i,
            &["carol.feels.angry.toward.bob", "carol.feels.angry"],
        );
        let db = db
            .retract_str(&mut i, "carol.feels.angry.toward.bob")
            .unwrap();
        assert!(db.exists_str(&mut i, "carol.feels.angry").unwrap());
        assert!(!db.exists_str(&mut i, "carol.feels.angry.toward").unwrap());
        assert_eq!(db.to_sentences(&i), ["carol.feels.angry"]);
    }

    // ===== serialization round-trips assertedness =====
    // H: DbSpec.hs "serialization round-trips assertedness"

    // H: DbSpec.hs "labeled: an asserted interior node with children round-trips exactly (marks included)"
    #[test]
    fn labeled_asserted_interior_round_trips_exactly() {
        let mut i = Interner::new();
        let db = build(
            &mut i,
            &[
                "practice.tendBar.bar.ada",
                "practice.tendBar.bar.ada.customer.you",
                "note!seen",
            ],
        );
        let labeled = db.to_labeled_sentences(&i);
        let mut rebuilt = Db::empty();
        for s in &labeled {
            rebuilt = rebuilt.insert_str(&mut i, s).unwrap();
        }
        assert_eq!(rebuilt, db);
    }

    // H: DbSpec.hs "plain: a mark-bearing db rebuilds identically from its flattened sentences"
    #[test]
    fn plain_mark_bearing_db_rebuilds_identically() {
        let mut i = Interner::new();
        let db = build(
            &mut i,
            &[
                "practice.tendBar.bar.ada",
                "practice.tendBar.bar.ada.customer.you",
            ],
        );
        let sentences = db.to_sentences(&i);
        let mut rebuilt = Db::empty();
        for s in &sentences {
            rebuilt = rebuilt.insert_str(&mut i, s).unwrap();
        }
        assert_eq!(rebuilt, db);
    }

    // ===== unify =====
    // H: DbSpec.hs "unify"

    // H: DbSpec.hs "two-sentence join binds shared variable"
    #[test]
    fn two_sentence_join_binds_shared_variable() {
        let mut i = Interner::new();
        let db = build(
            &mut i,
            &[
                "foo.bar.woof",
                "foo.meow.woof",
                "fizz.buzz.foo",
                "some.other.woof",
            ],
        );
        let x = i.intern("X");
        let y = i.intern("Y");
        let results = unify_all(&db, &mut i, &["X.Y.woof", "fizz.buzz.X"]);

        let xs: Vec<&str> = results.iter().map(|b| sym_name(&i, b.get(x))).collect();
        assert_eq!(xs, ["foo", "foo"]);

        let mut ys: Vec<&str> = results
            .iter()
            .filter_map(|b| b.get(y))
            .map(|v| match v {
                Val::Sym(s) => i.resolve(*s),
                other => panic!("expected VSym, got {other:?}"),
            })
            .collect();
        ys.sort_unstable();
        assert_eq!(ys, ["bar", "meow"]);
    }

    // H: DbSpec.hs "bound variable descends deterministically"
    #[test]
    fn bound_variable_descends_deterministically() {
        let mut i = Interner::new();
        let db = build(&mut i, &["char.tim", "char.kevin"]);
        let path = tokenize(&mut i, "char.Who").unwrap();
        assert_eq!(db.unify(&mut i, &path.segs, Bindings::new()).len(), 2);
    }

    // H: DbSpec.hs "constant that is absent yields no bindings"
    #[test]
    fn absent_constant_yields_no_bindings() {
        let mut i = Interner::new();
        let db = build(&mut i, &["char.tim"]);
        let path = tokenize(&mut i, "char.nobody").unwrap();
        assert!(db.unify(&mut i, &path.segs, Bindings::new()).is_empty());
    }

    // ===== ground =====
    // H: DbSpec.hs "ground"

    // H: DbSpec.hs "substitutes bound vars, preserves ! and ."
    #[test]
    fn ground_substitutes_bound_vars_preserving_operators() {
        let mut i = Interner::new();
        let (b_, c_, bev_) = (i.intern("B"), i.intern("C"), i.intern("Bev"));
        let (ada, beth, cider) = (i.intern("ada"), i.intern("beth"), i.intern("cider"));
        let mut b = Bindings::new();
        b.insert(b_, Val::Sym(ada));
        b.insert(c_, Val::Sym(beth));
        b.insert(bev_, Val::Sym(cider));
        let path = tokenize(&mut i, "practice.tendBar.B.customer.C!order!Bev").unwrap();
        assert_eq!(
            ground(&i, &path, &b),
            "practice.tendBar.ada.customer.beth!order!cider"
        );
    }

    // H: DbSpec.hs "unbound var grounds to its own name"
    #[test]
    fn ground_unbound_var_grounds_to_its_own_name() {
        let mut i = Interner::new();
        let path = tokenize(&mut i, "foo.Bar").unwrap();
        assert_eq!(ground(&i, &path, &Bindings::new()), "foo.Bar");
    }

    // H: DbSpec.hs "set-valued binding renders as opaque marker"
    #[test]
    fn ground_set_valued_binding_renders_as_opaque_marker() {
        let mut i = Interner::new();
        let dancers = i.intern("Dancers");
        let (a, b) = (i.intern("a"), i.intern("b"));
        let mut binds = Bindings::new();
        binds.insert(dancers, Val::Set(vec![vec![a], vec![b]]));
        let path = tokenize(&mut i, "all.Dancers").unwrap();
        assert_eq!(ground(&i, &path, &binds), "all.<Set(2)>");
    }

    // ===== unifyNames =====
    // H: DbSpec.hs "unifyNames"

    // H: DbSpec.hs "unifyNames is unify with the parse hoisted out"
    #[test]
    fn unify_is_invariant_under_hoisting_the_parse() {
        // The Rust engine has ONE unify over pre-tokenized segments; the pin's
        // content — hoisting the parse out of the per-binding loop changes
        // nothing — holds because tokenization is a deterministic function.
        let mut i = Interner::new();
        let db = build(&mut i, &["at.bob!square", "at.eve!mill"]);
        let path_a = tokenize(&mut i, "at.Who!Where").unwrap();
        let inside = db.unify(&mut i, &path_a.segs, Bindings::new());
        let path_b = tokenize(&mut i, "at.Who!Where").unwrap();
        let hoisted = db.unify(&mut i, &path_b.segs, Bindings::new());
        assert_eq!(inside, hoisted);
        assert_eq!(inside.len(), 2);
    }

    // ===== groundTokens =====
    // H: DbSpec.hs "groundTokens"

    // H: DbSpec.hs "groundTokens substitutes bindings segment-wise, preserving operators"
    #[test]
    fn ground_over_tokens_preserves_operators_and_plain_paths() {
        let mut i = Interner::new();
        let (who, where_) = (i.intern("Who"), i.intern("Where"));
        let (bob, square) = (i.intern("bob"), i.intern("square"));
        let mut b = Bindings::new();
        b.insert(who, Val::Sym(bob));
        b.insert(where_, Val::Sym(square));
        let path = tokenize(&mut i, "at.Who!Where").unwrap();
        assert_eq!(ground(&i, &path, &b), "at.bob!square");
        let plain = tokenize(&mut i, "plain.path").unwrap();
        assert_eq!(ground(&i, &plain, &Bindings::new()), "plain.path");
    }

    // ===== internTokens / unifySyms =====
    // H: DbSpec.hs "internTokens / unifySyms (the Sym-level cores unify/unifyNames delegate to)"

    // H: DbSpec.hs "internTokens interns tokens' segment names, preserving operators"
    #[test]
    fn tokenize_interns_names_preserving_operators() {
        let mut i = Interner::new();
        let path = tokenize(&mut i, "at.Who!Where").unwrap();
        assert_eq!(path_names(&i, &path), ["at", "Who", "Where"]);
        // Operator after seg 0 is `.`, after seg 1 is `!`.
        assert!(!path.is_excl_after(0));
        assert!(path.is_excl_after(1));
    }

    // H: DbSpec.hs "unifySyms agrees with unifyNames (Bindings is Sym-keyed natively)"
    #[test]
    fn tokenized_query_agrees_with_manual_interning() {
        // Rust has one Sym-level unify; the pin's guarantee (the Sym-level core
        // agrees with the name-level surface) is that tokenizing a pattern and
        // interning its names by hand yield the same segments, hence the same
        // unify results.
        let mut i = Interner::new();
        let db = build(&mut i, &["at.bob!square", "at.eve!mill"]);
        let tokenized = tokenize(&mut i, "at.Who!Where").unwrap();
        let manual: Vec<Sym> = ["at", "Who", "Where"].iter().map(|n| i.intern(n)).collect();
        assert_eq!(tokenized.segs.as_slice(), manual.as_slice());
        assert_eq!(
            db.unify(&mut i, &tokenized.segs, Bindings::new()),
            db.unify(&mut i, &manual, Bindings::new())
        );
    }

    // H: DbSpec.hs "unifySyms branches unbound variables in name order, not id (encounter) order"
    #[test]
    fn unify_branches_unbound_vars_in_name_order() {
        // Insert children out of alphabetical order, so id (encounter) order !=
        // name order; unify must branch in name order.
        let mut i = Interner::new();
        let db = build(&mut i, &["at.zeta", "at.alpha", "at.mu"]);
        let who = i.intern("Who");
        let path = tokenize(&mut i, "at.Who").unwrap();
        let results = db.unify(&mut i, &path.segs, Bindings::new());
        let names: Vec<&str> = results.iter().map(|b| sym_name(&i, b.get(who))).collect();
        assert_eq!(names, ["alpha", "mu", "zeta"]);
    }

    // ===== child_keys (no DbSpec pin; S2 uses it) =====
    #[test]
    fn child_keys_are_name_sorted() {
        let mut i = Interner::new();
        let db = build(&mut i, &["at.zeta", "at.alpha", "at.mu"]);
        let path = tokenize(&mut i, "at").unwrap();
        assert_eq!(db.child_keys(&i, &path.segs), ["alpha", "mu", "zeta"]);
    }
}

#[cfg(test)]
mod proptest_laws {
    //! Randomized trie laws (ARCHITECTURE.md's list): the v39 asserted-flag
    //! invariant under arbitrary op sequences, `!` supersession preserving the
    //! survivor subtree, insert/exists/retract round-trips, and enumeration
    //! inverting insertion.
    use super::*;
    use proptest::prelude::*;

    /// The v39 invariant: no unasserted childless node anywhere in the trie
    /// (the root is exempt — it is never a fact and legitimately empty).
    fn invariant_holds(db: &Db) -> bool {
        fn walk(node: &Node) -> bool {
            node.kids.iter().all(|(_, child)| {
                let c = &*child.0;
                let ok = c.asserted || !c.kids.is_empty();
                ok && walk(c)
            })
        }
        walk(&db.0)
    }

    /// The four-segment alphabet (lowercase ⇒ constant, so these are ground
    /// facts, never variables).
    const ALPHA: [&str; 4] = ["a", "b", "c", "d"];

    /// A random segment from a tiny constant alphabet.
    fn seg() -> impl Strategy<Value = String> {
        prop::sample::select(ALPHA.to_vec()).prop_map(String::from)
    }

    /// A pair of DISTINCT segments, distinct by construction (no rejection
    /// sampling). `d` is a nonzero step mod 4, so `(a + d) % 4 != a` always;
    /// this covers every ordered distinct pair uniformly and, unlike a
    /// `prop_assume!(x != y)` guard over a 25%-collision alphabet, generates
    /// ZERO rejects — so the law it feeds soaks at any `PROPTEST_CASES` budget
    /// instead of aborting on proptest's global-reject ceiling.
    fn distinct_seg_pair() -> impl Strategy<Value = (String, String)> {
        (0usize..4, 1usize..4)
            .prop_map(|(a, d)| (ALPHA[a].to_string(), ALPHA[(a + d) % 4].to_string()))
    }

    /// A random dotted, ground path of 1–4 segments with `.`/`!` separators.
    fn path() -> impl Strategy<Value = String> {
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

    #[derive(Debug, Clone)]
    enum Op {
        Insert(String),
        Retract(String),
    }

    fn op() -> impl Strategy<Value = Op> {
        prop_oneof![path().prop_map(Op::Insert), path().prop_map(Op::Retract),]
    }

    proptest! {
        // The v39 invariant holds after any sequence of inserts and retracts.
        #[test]
        fn invariant_survives_arbitrary_op_sequences(ops in prop::collection::vec(op(), 0..40)) {
            let mut i = Interner::new();
            let mut db = Db::empty();
            for o in ops {
                db = match o {
                    Op::Insert(s) => db.insert_str(&mut i, &s).unwrap(),
                    Op::Retract(s) => db.retract_str(&mut i, &s).unwrap(),
                };
                prop_assert!(invariant_holds(&db), "invariant broke after op; db = {:?}", db.to_labeled_sentences(&i));
            }
        }

        // Enumeration inverts insertion: rebuilding from labeled sentences
        // reproduces the database exactly.
        #[test]
        fn labeled_sentences_round_trip(paths in prop::collection::vec(path(), 0..12)) {
            let mut i = Interner::new();
            let mut db = Db::empty();
            for p in &paths {
                db = db.insert_str(&mut i, p).unwrap();
            }
            let labeled = db.to_labeled_sentences(&i);
            let mut rebuilt = Db::empty();
            for s in &labeled {
                rebuilt = rebuilt.insert_str(&mut i, s).unwrap();
            }
            prop_assert_eq!(rebuilt, db);
        }

        // `!` supersession preserves the surviving child's subtree: after
        // `p!c.x` then `p!c.y`, both x and y survive under c.
        #[test]
        fn bang_supersession_preserves_survivor_subtree(
            p in seg(), c in seg(), (x, y) in distinct_seg_pair()
        ) {
            // x != y by construction (distinct_seg_pair) — no rejection sampling,
            // so the law completes at soak depth rather than exhausting the
            // global-reject budget.
            let mut i = Interner::new();
            let db = Db::empty()
                .insert_str(&mut i, &format!("{p}!{c}.{x}")).unwrap()
                .insert_str(&mut i, &format!("{p}!{c}.{y}")).unwrap();
            let has_x = db.exists_str(&mut i, &format!("{p}.{c}.{x}")).unwrap();
            let has_y = db.exists_str(&mut i, &format!("{p}.{c}.{y}")).unwrap();
            prop_assert!(has_x, "survivor subtree lost x");
            prop_assert!(has_y, "survivor subtree lost y");
        }

        // Insert/exists/retract round-trip for a dotted (non-exclusive) path
        // over an empty db: after insert every prefix exists; after retract the
        // db is empty again.
        #[test]
        fn insert_exists_retract_round_trip(parts in prop::collection::vec(seg(), 1..5)) {
            // Dedupe consecutive equal segments so the path has distinct prefixes
            // (a.a.a is a legitimate but degenerate path; keep it simple).
            let mut i = Interner::new();
            let full = parts.join(".");
            let db = Db::empty().insert_str(&mut i, &full).unwrap();
            // Every prefix of the inserted path exists.
            for k in 1..=parts.len() {
                let prefix = parts[..k].join(".");
                prop_assert!(db.exists_str(&mut i, &prefix).unwrap(), "prefix {} missing", prefix);
            }
            let db = db.retract_str(&mut i, &full).unwrap();
            prop_assert!(db.to_sentences(&i).is_empty(), "db not empty after retract: {:?}", db.to_sentences(&i));
        }
    }
}
