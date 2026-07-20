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
    let mut segs: SmallVec<[Sym; 6]> = SmallVec::new();
    let mut excl: u32 = 0;
    scan_tokens(sentence, |name, op| {
        guard_len(&segs, sentence)?;
        let idx = segs.len();
        let seg = interner.intern(name);
        segs.push(seg);
        if op == Some('!') {
            excl |= 1u32 << idx;
        }
        Ok(())
    })?;
    Ok(CompiledPath { segs, excl })
}

/// Walk a sentence's name/separator pairs, handing each name and the operator
/// that FOLLOWS it to `emit` — the pure shape of `Prax.Db.tokens`, and the ONE
/// place the trailing-operator guard lives.
///
/// Whitespace is trimmed from both ends first (`Prax.Db.trim`); the empty
/// sentence emits nothing. A trailing operator (`.`/`!` with no name after) is a
/// loud [`WorldError::TrailingOperator`], raised BEFORE the preceding name is
/// emitted, exactly where the frozen `tokens` raises it. Both [`tokenize`] and
/// [`segment_names_checked`] go through here so the two cannot drift: a guard
/// implemented twice is a guard that will one day be implemented once.
fn scan_tokens(
    sentence: &str,
    mut emit: impl FnMut(&str, Option<char>) -> Result<(), WorldError>,
) -> Result<(), WorldError> {
    let trimmed = sentence.trim_matches(|c| c == ' ' || c == '\t' || c == '\n' || c == '\r');
    if trimmed.is_empty() {
        return Ok(());
    }
    let mut rest = trimmed;
    loop {
        match rest.find(['.', '!']) {
            // Final segment: no following operator.
            None => return emit(rest, None),
            Some(pos) => {
                let op = rest.as_bytes()[pos] as char;
                let after = &rest[pos + 1..];
                if after.is_empty() {
                    return Err(WorldError::TrailingOperator {
                        sentence: sentence.to_owned(),
                        op,
                    });
                }
                emit(&rest[..pos], Some(op))?;
                rest = after;
            }
        }
    }
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

/// The segment names of a raw sentence string, without interning and WITHOUT the
/// trailing-operator guard — a plain split for the authoring-boundary walkers
/// (`Prax.Query.conditionVars` and friends) that inspect authored strings before
/// compilation exists, must not touch the pool, and are infallible by signature.
/// Outer whitespace is trimmed (as [`tokenize`] does) and the name is split on
/// both separators.
///
/// This is NOT `Prax.Db.pathNames`. `pathNames = map fst . tokens`, and `tokens`
/// RAISES on a trailing operator; this function returns a trailing empty segment
/// instead. Anything mirroring a frozen `pathNames` call in a context that can
/// report an error must use [`segment_names_checked`] — porting a `pathNames`
/// call site to this function silently converts a loud construction-time
/// rejection into a malformed value that renders plausibly.
pub fn segment_names(sentence: &str) -> Vec<String> {
    let trimmed = sentence.trim_matches(|c| c == ' ' || c == '\t' || c == '\n' || c == '\r');
    if trimmed.is_empty() {
        return Vec::new();
    }
    trimmed.split(['.', '!']).map(str::to_owned).collect()
}

/// The segment names of a raw sentence string, without interning — the pure
/// `Prax.Db.pathNames` (`map fst . tokens`), guard and all.
///
/// This is the function a ported `pathNames` call site wants: it raises the same
/// [`WorldError::TrailingOperator`] the frozen `Prax.Db.tokens` raises, at the
/// same input, so a combinator that splices an authored pattern into a built
/// sentence rejects a malformed pattern instead of building a malformed axiom
/// from it.
///
/// # Errors
/// [`WorldError::TrailingOperator`] if the sentence ends in `.` or `!`.
pub fn segment_names_checked(sentence: &str) -> Result<Vec<String>, WorldError> {
    let mut out = Vec::new();
    scan_tokens(sentence, |name, _| {
        out.push(name.to_owned());
        Ok(())
    })?;
    Ok(out)
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
    fn segment_names_checked_is_pathnames_guard_and_all() {
        // `pathNames = map fst . tokens`, so it carries `tokens`' trailing-
        // operator rejection. The unguarded split does NOT — it returns a
        // trailing empty segment — which is why the two are separate functions
        // and why every ported `pathNames` call site takes the checked one.
        assert_eq!(
            segment_names_checked("member.X!F").expect("well formed"),
            ["member", "X", "F"]
        );
        for (s, op) in [("struck.A.V.", '.'), ("struck.A.V!", '!')] {
            assert_eq!(
                segment_names_checked(s),
                Err(WorldError::TrailingOperator {
                    sentence: s.to_owned(),
                    op,
                }),
                "segment_names_checked must raise where Prax.Db.tokens raises"
            );
            assert_eq!(
                segment_names(s).last().map(String::as_str),
                Some(""),
                "the unguarded split is what it is — a trailing empty segment, not an error"
            );
        }
        assert!(segment_names_checked("   ").expect("empty").is_empty());
    }

    #[test]
    fn whitespace_is_trimmed_and_empty_is_the_empty_path() {
        let mut i = Interner::new();
        assert!(tokenize(&mut i, "   ").unwrap().segs.is_empty());
        let p = tokenize(&mut i, "  at.bob  ").unwrap();
        assert_eq!(seg_names(&i, &p), ["at", "bob"]);
    }
}
