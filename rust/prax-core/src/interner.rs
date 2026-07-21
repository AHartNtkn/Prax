//! Interned path segments. `Arc<Interner>` lives in the engine state,
//! replacing the frozen tree's process-global `unsafePerformIO` pool. Variable-ness
//! is packed into id parity (the kept var-bit trick), so the hottest predicate
//! stays a bit test. Ids never leak into output — all observable text renders
//! through the name lookup (the determinism contract, PLAN.md).
//!
//! The frozen reference is `src/Prax/Sym.hs` (spec
//! `docs/specs/2026-07-12-v29-interning.md`). The Haskell keeps a global
//! `IORef` pool guarded by `unsafePerformIO`/`NOINLINE` and a bang-pattern
//! discipline against a lazy-thunk ordering race; none of that machinery
//! survives here — the interner is an owned, append-only value, and Rust's
//! strict evaluation makes the whole strictness-race class of bug impossible by
//! construction (see `conformance/KILLED.md`).

use rustc_hash::FxHashMap;
use std::cmp::Ordering;

/// An interned segment name. The low bit is the variable flag (uppercase-first
/// name ⇒ variable, the engine's existing convention); the remaining bits are a
/// dense first-encounter index into the interner's name table. The numeric
/// value is *run-dependent* and must never be serialized, rendered, or compared
/// across interners — the only observable form of a `Sym` is its name, via
/// [`Interner::resolve`], and every observable order sorts BY NAME.
///
/// `Ord` is by raw value (first-encounter id order): this is the trie's
/// internal child key order, never an output order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sym(u32);

impl Sym {
    /// Is this segment a logic variable? A single bit test — the whole point of
    /// packing variable-ness into the id (`Prax.Sym.symIsVar`).
    #[inline]
    pub fn is_var(self) -> bool {
        self.0 & 1 == 1
    }

    /// The raw interned id — the trie's `SmallVec`-keying value (children are
    /// sorted by it), the heir of `Prax.Sym.symId`'s `IntMap`-keying escape
    /// hatch. Crate-internal: never compare, serialize, or render this outside
    /// the engine; it is first-encounter ordered and run-dependent.
    #[inline]
    pub(crate) fn id(self) -> u32 {
        self.0
    }

    /// Reconstruct a `Sym` from a raw id (the read side of the keying escape
    /// hatch, heir of `Prax.Sym.symOfId`). Production never reconstructs a `Sym`
    /// from a bare id — the trie keys directly on `Sym` — so this exists only to
    /// exercise the round-trip identity the Haskell `symId`/`symOfId` pair
    /// carried; hence test-only.
    #[cfg(test)]
    #[inline]
    pub(crate) fn from_raw(id: u32) -> Sym {
        Sym(id)
    }
}

/// The owned, append-only intern pool. Names map to first-encounter `Sym`s;
/// `resolve` recovers the name. No global state, no `unsafePerformIO`: the pool
/// is a plain value threaded (as `Arc<Interner>`) through the engine state.
#[derive(Debug, Clone, Default)]
pub struct Interner {
    fwd: FxHashMap<String, Sym>,
    /// Names in first-encounter order; `Sym(v)` resolves to `names[v >> 1]`.
    names: Vec<String>,
    /// `name_rank[i]` is the position of `names[i]` in byte-lexicographic
    /// name order. Maintained on every new intern so [`cmp_by_name`] — the
    /// determinism contract's primitive, called O(k·log k) per unbound `unify`
    /// branch across the whole closure/planner — is an integer compare rather
    /// than a repeated string compare. It is EXACTLY the order `resolve(a).cmp(
    /// resolve(b))` gives (a total order on distinct names), so no observable
    /// order changes; only the cost does.
    name_rank: Vec<u32>,
}

impl Interner {
    /// A fresh, empty interner.
    pub fn new() -> Interner {
        Interner::default()
    }

    /// Intern a segment: total, idempotent, a function within one interner.
    /// A new name takes the next first-encounter index; variable-ness (packed
    /// into the low bit) is decided once here by the uppercase-first rule.
    pub fn intern(&mut self, name: &str) -> Sym {
        if let Some(&s) = self.fwd.get(name) {
            return s;
        }
        let index = self.names.len() as u32;
        let var_bit = u32::from(is_variable_name(name));
        let sym = Sym((index << 1) | var_bit);
        self.names.push(name.to_owned());
        self.fwd.insert(name.to_owned(), sym);
        self.insert_rank(index as usize, name);
        sym
    }

    /// Splice the new name's index into the byte-lexicographic rank order.
    /// The new name goes at rank `p` (its sorted position among all names);
    /// every name previously ranked ≥ `p` shifts up by one. Amortized cheap:
    /// after a world's vocabulary is established, `intern` is a hash hit and
    /// never reaches here; new segments (turn numbers, fresh belief atoms)
    /// arrive rarely on the hot path.
    fn insert_rank(&mut self, index: usize, name: &str) {
        // Position among existing names by byte order (binary search over the
        // rank permutation, whose k-th entry is the name-index at rank k).
        // `order[k]` is recovered as the index whose rank is k; we search the
        // sorted names directly via the current permutation.
        let p = self.name_rank_position(name);
        for r in self.name_rank.iter_mut() {
            if (*r as usize) >= p {
                *r += 1;
            }
        }
        self.name_rank.push(p as u32);
    }

    /// The byte-lexicographic rank the given name occupies among the already-
    /// ranked names (i.e. how many existing names sort strictly before it).
    fn name_rank_position(&self, name: &str) -> usize {
        // `self.names[i]` has rank `self.name_rank[i]`; count names sorting
        // before `name`. A linear pass is O(n) — same order as the shift below,
        // and simpler than threading a separate sorted index that must also be
        // shifted. New-intern events taper to near-zero on the hot path.
        self.names[..self.name_rank.len()]
            .iter()
            .filter(|existing| existing.as_str() < name)
            .count()
    }

    /// The segment a symbol names — the *only* observable form of a `Sym`.
    /// Panics on a foreign id (impossible for any `Sym` this interner produced),
    /// exactly as `Prax.Sym.symName` errors loudly.
    pub fn resolve(&self, sym: Sym) -> &str {
        let index = (sym.0 >> 1) as usize;
        match self.names.get(index) {
            Some(name) => name,
            None => panic!("Interner::resolve: unknown symbol id {}", sym.0),
        }
    }

    /// Compare two symbols BY NAME — the determinism contract's primitive at
    /// every enumeration point (unify's unbound branch, `child_keys`, sentence
    /// enumeration). Never compare by raw id for an observable order.
    pub fn cmp_by_name(&self, a: Sym, b: Sym) -> Ordering {
        let ra = self.name_rank[(a.0 >> 1) as usize];
        let rb = self.name_rank[(b.0 >> 1) as usize];
        ra.cmp(&rb)
    }
}

/// A path segment is a variable iff its first character is uppercase
/// (`Prax.Db.isVariable` / `Prax.Sym`'s convention). The empty segment is a
/// constant. Uses Unicode uppercase to mirror Haskell's `Data.Char.isUpper`.
pub fn is_variable_name(name: &str) -> bool {
    match name.chars().next() {
        Some(c) => c.is_uppercase(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    // H: SymSpec.hs "Prax.Sym"
    //
    // The semantic pins of the frozen `Prax.SymSpec`. The strictness-race pin
    // ("symName forces its argument before touching the pool") is Haskell-only
    // and killed in conformance/KILLED.md — Rust is strict and owns the pool,
    // so no lazy-thunk/pool-write ordering hazard exists to guard against.
    use super::*;

    // H: SymSpec.hs "intern/symName round-trips and is idempotent"
    #[test]
    fn intern_resolve_round_trips_and_is_idempotent() {
        let mut i = Interner::new();
        let a = i.intern("square");
        assert_eq!(i.resolve(a), "square");
        assert_eq!(i.intern("square"), a);
    }

    // H: SymSpec.hs "variable-ness is packed into parity"
    #[test]
    fn variable_ness_is_packed_into_parity() {
        let mut i = Interner::new();
        assert!(!i.intern("square").is_var(), "constants are not variables");
        assert!(i.intern("Actor").is_var(), "uppercase-initial segments are");
        assert!(!i.intern("").is_var(), "empty segment is a constant");
    }

    // H: SymSpec.hs "distinct names get distinct symbols"
    #[test]
    fn distinct_names_get_distinct_symbols() {
        let mut i = Interner::new();
        assert_ne!(i.intern("mill"), i.intern("square"));
    }

    // H: SymSpec.hs "symId/symOfId round-trip (Prax.Db's IntMap-keying escape hatch)"
    #[test]
    fn id_from_raw_round_trip() {
        // The heir of the symId/symOfId escape hatch: a Sym's raw id round-trips
        // to the same Sym and still resolves to its name. (In Rust the trie keys
        // directly on Sym, so this identity is the whole content the Haskell
        // IntMap-keying indirection carried.)
        let mut i = Interner::new();
        let a = i.intern("gazebo");
        assert_eq!(Sym::from_raw(a.id()), a);
        assert_eq!(i.resolve(Sym::from_raw(a.id())), "gazebo");
    }

    // H: SymSpec.hs "distinct symbols have distinct ids"
    #[test]
    fn distinct_symbols_have_distinct_ids() {
        let mut i = Interner::new();
        assert_ne!(i.intern("alpha").id(), i.intern("beta").id());
    }
}
