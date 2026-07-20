//! `CompiledPath` and the ONE tokenizer — the single authoring boundary where a
//! sentence string becomes interned segments. A path is a run of segment names
//! separated by `.` (ordinary, multi-valued descent) or `!` (exclusion: the
//! parent has exactly one child). Queries treat `.` and `!` identically; the
//! distinction is retained here (the exclusion bitmask) so the trie stays a
//! faithful Exclusion-Logic model.
//!
//! Frozen reference: `Prax.Db.tokens` / `internTokens`. `CompiledPath` fuses
//! the Haskell's `[(Sym, Maybe Char)]` into a `SmallVec` of segments plus a
//! `u32` where bit `i` records that the separator AFTER segment `i` is `!`
//! (so segment `i`'s trie node is exclusive). A sentence ending in an operator
//! is rejected loudly — a trailing operator would set a leaf's exclusion flag
//! that nothing ever reads.

use smallvec::SmallVec;

use crate::error::WorldError;
use crate::interner::{Interner, Sym};

/// The exclusion bitmask is a `u32`; a path may carry at most this many
/// segments (real paths are a handful deep). Beyond it the bitmask cannot label
/// a separator, so the tokenizer rejects rather than corrupt an exclusion flag.
const MAX_SEGMENTS: usize = 32;

/// A tokenized path: interned segments plus an exclusion bitmask. Bit `i` is set
/// iff the separator following segment `i` is `!` (segment `i`'s node is
/// exclusive). The final segment has no following separator, so its bit is
/// always clear.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompiledPath {
    pub segs: SmallVec<[Sym; 6]>,
    pub excl: u32,
}

impl CompiledPath {
    /// Whether segment `i`'s node is exclusive (the separator after it was `!`).
    #[inline]
    pub fn is_excl_after(&self, i: usize) -> bool {
        self.excl & (1u32 << i) != 0
    }
}

/// Tokenize a sentence into a [`CompiledPath`], interning each segment.
///
/// Whitespace is trimmed from both ends first (`Prax.Db.trim`). The empty
/// sentence is the empty path. A trailing operator (`.`/`!` with no name after)
/// is a loud [`WorldError::TrailingOperator`], matching the frozen
/// `Prax.Db.tokens`.
pub fn tokenize(interner: &mut Interner, sentence: &str) -> Result<CompiledPath, WorldError> {
    let trimmed = sentence.trim_matches(|c| c == ' ' || c == '\t' || c == '\n' || c == '\r');

    let mut segs: SmallVec<[Sym; 6]> = SmallVec::new();
    let mut excl: u32 = 0;

    if trimmed.is_empty() {
        return Ok(CompiledPath { segs, excl });
    }

    // Walk name/separator pairs. Each name gets the operator that FOLLOWS it;
    // the sentence must end in a name (a trailing operator is rejected).
    let mut rest = trimmed;
    loop {
        let sep_pos = rest.find(['.', '!']);
        match sep_pos {
            None => {
                // Final segment: no following operator.
                guard_len(&segs, sentence)?;
                let seg = interner.intern(rest);
                segs.push(seg);
                break;
            }
            Some(pos) => {
                let name = &rest[..pos];
                let op = rest.as_bytes()[pos] as char;
                let after = &rest[pos + 1..];
                if after.is_empty() {
                    return Err(WorldError::TrailingOperator {
                        sentence: sentence.to_owned(),
                        op,
                    });
                }
                guard_len(&segs, sentence)?;
                let idx = segs.len();
                let seg = interner.intern(name);
                segs.push(seg);
                if op == '!' {
                    excl |= 1u32 << idx;
                }
                rest = after;
            }
        }
    }

    Ok(CompiledPath { segs, excl })
}

/// Guard the segment count against the exclusion-bitmask width before pushing
/// the next segment. (`segs.len()` is the index the next `push` will occupy.)
fn guard_len(segs: &SmallVec<[Sym; 6]>, sentence: &str) -> Result<(), WorldError> {
    if segs.len() >= MAX_SEGMENTS {
        return Err(WorldError::PathTooLong {
            sentence: sentence.to_owned(),
            segments: segs.len() + 1,
        });
    }
    Ok(())
}

/// The segment names of a path, in path order — the flattened form both `.` and
/// `!` collapse to, for matching and retract (`Prax.Db.pathNames`).
pub fn path_names(interner: &Interner, path: &CompiledPath) -> Vec<String> {
    path.segs
        .iter()
        .map(|&s| interner.resolve(s).to_owned())
        .collect()
}

/// The segment names of a raw sentence string, without interning — the pure
/// `Prax.Db.pathNames` (`map fst . tokens`) for authoring-boundary walkers
/// (`Prax.Query.conditionVars` and friends) that inspect authored strings
/// before compilation exists and so must not touch the pool. Outer whitespace is
/// trimmed (as [`tokenize`] does) and the name is split on both separators.
pub fn segment_names(sentence: &str) -> Vec<String> {
    let trimmed = sentence.trim_matches(|c| c == ' ' || c == '\t' || c == '\n' || c == '\r');
    if trimmed.is_empty() {
        return Vec::new();
    }
    trimmed.split(['.', '!']).map(str::to_owned).collect()
}

#[cfg(test)]
mod tests {
    // H: DbSpec.hs "tokens: trailing-operator rejection (v43 -- a trailing op would set a leaf's exclusion flag nothing ever reads)"
    //
    // The tokenizer is the heir of `Prax.Db.tokens`; these are its DbSpec pins.
    use super::*;
    use crate::error::WorldError;

    fn seg_names(interner: &Interner, path: &CompiledPath) -> Vec<String> {
        path_names(interner, path)
    }

    // H: DbSpec.hs "a sentence with no trailing operator is unaffected"
    #[test]
    fn no_trailing_operator_is_unaffected() {
        // tokens "at.bob" == [("at", Just '.'), ("bob", Nothing)]: two segments,
        // the separator after "at" is `.` (exclusion bit clear).
        let mut i = Interner::new();
        let p = tokenize(&mut i, "at.bob").unwrap();
        assert_eq!(seg_names(&i, &p), ["at", "bob"]);
        assert!(!p.is_excl_after(0));
    }

    // H: DbSpec.hs "an interior ! (not trailing) is unaffected"
    #[test]
    fn interior_bang_is_unaffected() {
        // tokens "turn!0" == [("turn", Just '!'), ("0", Nothing)]: the separator
        // after "turn" is `!` (segment 0's node is exclusive).
        let mut i = Interner::new();
        let p = tokenize(&mut i, "turn!0").unwrap();
        assert_eq!(seg_names(&i, &p), ["turn", "0"]);
        assert!(p.is_excl_after(0));
    }

    // H: DbSpec.hs "a sentence ending in ! is a loud construction-time error"
    #[test]
    fn trailing_bang_is_rejected() {
        let mut i = Interner::new();
        assert_eq!(
            tokenize(&mut i, "at.bob!"),
            Err(WorldError::TrailingOperator {
                sentence: "at.bob!".to_owned(),
                op: '!',
            })
        );
    }

    // H: DbSpec.hs "a sentence ending in . is a loud construction-time error"
    #[test]
    fn trailing_dot_is_rejected() {
        let mut i = Interner::new();
        assert_eq!(
            tokenize(&mut i, "at.bob."),
            Err(WorldError::TrailingOperator {
                sentence: "at.bob.".to_owned(),
                op: '.',
            })
        );
    }

    #[test]
    fn whitespace_is_trimmed_and_empty_is_the_empty_path() {
        let mut i = Interner::new();
        assert!(tokenize(&mut i, "   ").unwrap().segs.is_empty());
        let p = tokenize(&mut i, "  at.bob  ").unwrap();
        assert_eq!(seg_names(&i, &p), ["at", "bob"]);
    }
}
