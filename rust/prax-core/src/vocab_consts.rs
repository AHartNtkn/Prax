//! The vocabulary constants the S9 type checker needs to see without a crate
//! cycle (design C1). The □-obligation operator's OPERATORS live in
//! [`prax_vocab::deontic`], building on these; but the S9 lint's already-lifted
//! and twin-presence tests are SYNTACTIC — computable from the lifted prefix
//! alone (v51's precedent) — so the CONSTANTS live here in `prax-core`, one home,
//! checker-visible, so the checker (`prax-core`) → `prax-vocab` → `prax-core`
//! cycle the naive placement would create cannot form.
//!
//! Frozen reference: `Prax.Deontic.{obligedHead, obligedLiftPrefix, obligationPath}`
//! and `Prax.Coerce.punitivePrefix`.

/// The obligation operator's head literal (`Prax.Deontic.obligedHead`) — the ONE
/// home for the vocabulary.
pub const OBLIGED_HEAD: &str = "obliged";

/// The prefix every □-lifted sentence carries (`Prax.Deontic.obligedLiftPrefix`),
/// `obliged.Obligor.` — the full lifted-shape convention in one place, so the S9
/// checker's already-lifted detection and the lift itself cannot desync.
pub const OBLIGED_LIFT_PREFIX: &str = "obliged.Obligor.";

/// The generated punitive desire's name prefix (`Prax.Coerce.punitivePrefix`),
/// `punishes-` — the one home the coercion combinator writes and the S9 checker's
/// unmotivated-coercion lint reads.
pub const PUNITIVE_PREFIX: &str = "punishes-";

/// The DB path of an obligation: `obliged.<who>.<content>`
/// (`Prax.Deontic.obligationPath`). `content` is a simple term (a sentence) kept
/// verbatim, so an exclusion `!` inside it survives into the trie.
pub fn obligation_path(who: &str, content: &str) -> String {
    format!("{OBLIGED_HEAD}.{who}.{content}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_the_frozen_vocabulary() {
        assert_eq!(OBLIGED_HEAD, "obliged");
        assert_eq!(OBLIGED_LIFT_PREFIX, "obliged.Obligor.");
        assert_eq!(PUNITIVE_PREFIX, "punishes-");
        assert_eq!(obligation_path("bex", "settle.up"), "obliged.bex.settle.up");
    }
}
