//! The [S-I6] `as_role` equality, run against the SHIPPED WORLD'S OWN template.
//!
//! DIV-4 replaces the frozen `asRole`'s `groundCondition` with S4's
//! [`prax_core::types::rename_vars`], and `WitnessSpec`'s pin proves the two
//! value-identical over the templates the shipped call sites use. But
//! `prax-vocab` may not depend on `prax-worlds`, so that pin necessarily
//! reproduces Bar/Village's `together` AS DATA — and a copy is exactly as
//! correct as the day it was written. Its chain to the real world closes by
//! argument across two files: `worldshape bar` holds the world's copy identical
//! to the frozen one, and the WitnessSpec pin holds the vocab crate's copy
//! identical to the literal.
//!
//! `conformance` depends on both crates, so here it closes by CONSTRUCTION: the
//! equality runs over [`prax_worlds::bar::together`] and
//! [`prax_worlds::village::together`] themselves. If a world's co-presence changed
//! shape, the WitnessSpec pin would keep testing the old one while still passing;
//! these would not. The village's matters most: it is the world with the MOST
//! `as_role` call sites behind it (Rumor's gossip, Deceit's lie, Confession's
//! confess, and Blackmail's trigger AND punish).

#[cfg(test)]
mod tests {
    use prax_core::db::{Bindings, Val};
    use prax_core::interner::Interner;
    use prax_core::query::{Condition, ground_condition};
    use prax_vocab::witness::{CoPresence, as_role};
    use prax_worlds::bar::together as bar_together;
    use prax_worlds::village::together as village_together;

    /// The frozen implementation, transcribed from `src/Prax/Witness.hs`:
    /// `map (groundCondition (Map.singleton (intern "Witness") (VSym (intern v))))`.
    /// The ORACLE for the pin below, never a shipped path.
    fn ground_reference(v: &str, copresence: &CoPresence) -> Vec<Condition> {
        let mut i = Interner::new();
        let mut b = Bindings::new();
        let key = i.intern("Witness");
        let val = Val::Sym(i.intern(v));
        b.insert(key, val);
        copresence
            .iter()
            .map(|c| ground_condition(&mut i, &b, c).expect("a shipped template is well-formed"))
            .collect()
    }

    #[test]
    fn as_role_agrees_with_ground_condition_on_every_shipped_worlds_own_co_presence() {
        for (world, template) in [("bar", bar_together()), ("village", village_together())] {
            assert!(
                !template.is_empty(),
                "an empty co-presence would make this pin vacuous ({world})"
            );
            // Every role the shipped call sites retarget to: Rumor/Deceit/
            // Confession use `Hearer`, Blackmail's trigger uses the victim's own
            // bound variable, and `Witness` is the identity case.
            for role in ["Hearer", "V", "Witness"] {
                assert_eq!(
                    as_role(role, &template),
                    ground_reference(role, &template),
                    "as_role({role:?}) diverges from groundCondition on {world}'s own `together`"
                );
            }
        }
    }
}
