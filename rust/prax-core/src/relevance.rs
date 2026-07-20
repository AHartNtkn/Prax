//! The two relevance primitives the S4 engine router needs: [`may_unify_syms`]
//! (the hot delta-vs-footprint classification) and [`eviction_shadow_names`] (the
//! sibling shadows of an exclusion insert). The rest of `Prax.Relevance`
//! (`improvableDesires`/`livenessOf`/`bearingTemplates`/the atom pools) is the
//! planner's, and lands at S6 with its consumers — a present-but-empty table
//! would invite an accidental consumer.
//!
//! Frozen reference: `src/Prax/Relevance.hs` (`mayUnifySyms`, `evictionShadowNames`).
//!
//! One stated invariant carries [`may_unify_syms`]'s conservativity (an
//! assumption about authored worlds, not a construction guarantee): entity names
//! never collide with predicate-name literals — no character, place, or value is
//! named `lied`, `believes`, `regards`, and so on. The `anchored` clause spends
//! it: a pattern overlap covered entirely by variables carries no evidence the
//! two patterns denote the same predicate, so it is discarded.

use smallvec::SmallVec;

use crate::interner::{Interner, Sym};
use crate::path::CompiledPath;

/// Could a grounded instance of one path pattern be an instance (or a
/// prefix/extension) of the other, on pre-split, pre-interned paths
/// (`Prax.Relevance.mayUnifySyms`) — the planner-hot classification the router
/// runs for every primitive delta against every footprint pattern. Segments unify
/// when either is a variable or they are equal; a length mismatch is
/// prefix-compatible (a `Match` sees subtrees), so the walk zips to the shorter
/// path. A pair unifies only if some overlapping segment is a shared LITERAL
/// (both sides constant and equal) — an overlap covered entirely by variables
/// carries no evidence the two patterns denote the same predicate. Variable-ness
/// is the parity bit and a shared literal is `Sym`-id equality — the hottest
/// classification in the engine, an `Int` equality rather than a `String` one.
pub fn may_unify_syms(a: &[Sym], b: &[Sym]) -> bool {
    let anchored = a
        .iter()
        .zip(b)
        .any(|(x, y)| !x.is_var() && !y.is_var() && x == y);
    anchored
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.is_var() || y.is_var() || x == y)
}

/// The eviction shadows of an exclusion insert, computed on the labeled path
/// (`Prax.Relevance.evictionShadowNames`). One shadow per `!` operator: the
/// segment names up to and including that point, followed by a fresh
/// `PraxEvicted` segment (interns as a variable — uppercase initial,
/// Prax-namespaced machinery — so [`may_unify_syms`] treats it as the wildcard it
/// denotes and no authored name can collide with it). Each exclusion clears the
/// displaced sibling's entire subtree (arbitrary depth), and [`may_unify_syms`]
/// compares only up to the shorter path, so the truncated shadow covers every
/// want under it.
pub fn eviction_shadow_names(
    interner: &mut Interner,
    path: &CompiledPath,
) -> Vec<SmallVec<[Sym; 6]>> {
    let evicted = interner.intern("PraxEvicted");
    let mut out = Vec::new();
    for j in 0..path.segs.len() {
        if path.is_excl_after(j) {
            let mut shadow: SmallVec<[Sym; 6]> = path.segs[..=j].iter().copied().collect();
            shadow.push(evicted);
            out.push(shadow);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    // RelevanceSpec proper lands at S6 with the planner; these are the two S4
    // primitives' own correctness tests (no frozen pin consumed here).
    use super::*;
    use crate::path::tokenize;

    fn segs(i: &mut Interner, s: &str) -> Vec<Sym> {
        tokenize(i, s).unwrap().segs.to_vec()
    }

    #[test]
    fn may_unify_needs_a_shared_literal_anchor() {
        let mut i = Interner::new();
        // Same predicate literal, variable tail: unifies.
        let a = segs(&mut i, "lied.X");
        let b = segs(&mut i, "lied.bob");
        assert!(may_unify_syms(&a, &b));
        // Different predicate literals: no unify.
        let c = segs(&mut i, "regards.X");
        assert!(!may_unify_syms(&a, &c));
        // All-variable overlap: no literal anchor, evidence-free -> no unify.
        let v1 = segs(&mut i, "X.Y");
        let v2 = segs(&mut i, "P.bob");
        assert!(!may_unify_syms(&v1, &v2));
        // A conflicting literal at an aligned position blocks the unify.
        let d = segs(&mut i, "at.bar");
        let e = segs(&mut i, "at.mill");
        assert!(!may_unify_syms(&d, &e), "aligned differing literals cannot unify");
    }

    #[test]
    fn may_unify_is_prefix_compatible() {
        let mut i = Interner::new();
        // A shorter Match pattern sees a longer fact's subtree.
        let short = segs(&mut i, "practice.tendBar");
        let long = segs(&mut i, "practice.tendBar.ada.customer.beth");
        assert!(may_unify_syms(&short, &long));
    }

    #[test]
    fn eviction_shadows_one_per_bang_truncated_with_wildcard() {
        let mut i = Interner::new();
        let evicted = i.intern("PraxEvicted");
        // `a.b!c!d`: `!` after b (index 1) and after c (index 2).
        let path = tokenize(&mut i, "a.b!c!d").unwrap();
        let shadows = eviction_shadow_names(&mut i, &path);
        let a = i.intern("a");
        let b = i.intern("b");
        let c = i.intern("c");
        let want1: SmallVec<[Sym; 6]> = smallvec::smallvec![a, b, evicted];
        let want2: SmallVec<[Sym; 6]> = smallvec::smallvec![a, b, c, evicted];
        assert_eq!(shadows, vec![want1, want2]);
        // A `!`-free path has no shadows.
        let plain = tokenize(&mut i, "a.b.c").unwrap();
        assert!(eviction_shadow_names(&mut i, &plain).is_empty());
    }
}
