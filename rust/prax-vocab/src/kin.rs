//! Kinship: base facts + derived closure (spec §2). Marriage moves membership —
//! the fold's payoff in one line.
//!
//! Frozen reference: `src/Prax/Kin.hs`. Like [`crate::faction`] this is an AXIOM
//! builder: [`kin_axioms`] changes what every reader SEES, so its rules are
//! transcribed one for one and pinned against the frozen closure.

use prax_core::error::WorldError;
use prax_core::query::{matches, neq};
use prax_core::types::{Action, Axiom, Outcome, insert};

use crate::faction::joins;

/// The frozen `bad` (the same guard [`crate::faction`] transcribes).
fn bad(n: &str) -> bool {
    n.is_empty() || n.contains(['.', '!'])
}

/// Marriage symmetry, siblings, grandparents, in-laws — all derived, all
/// retraction-safe (dissolve the base fact and the closure forgets with it).
///
/// The two in-law rules are deliberately one-directional: the ACQUIRED relative
/// comes first, the ego (the spouse doing the acquiring) second — `inLaw.P.B`
/// reads "P is B's in-law", where P is A's parent or sibling and B is A's
/// spouse, matching the spec's own "spouse's parent / sibling's spouse" naming.
/// No symmetric `inLaw.B.P` is derived; a caller wanting the reverse lookup adds
/// it explicitly, the same way `married` states its own symmetry as a rule
/// rather than assuming it.
pub fn kin_axioms() -> Vec<Axiom> {
    vec![
        Axiom::new(vec![matches("married.A.B")], ["married.B.A"]),
        Axiom::new(
            vec![matches("parent.P.X"), matches("parent.P.Y"), neq("X", "Y")],
            ["sibling.X.Y"],
        ),
        Axiom::new(
            vec![matches("parent.G.P"), matches("parent.P.C")],
            ["grandparent.G.C"],
        ),
        Axiom::new(
            vec![matches("married.A.B"), matches("parent.P.A")],
            ["inLaw.P.B"],
        ),
        Axiom::new(
            vec![matches("married.A.B"), matches("sibling.A.S")],
            ["inLaw.S.B"],
        ),
    ]
}

/// `wed(joiner, faction, spouse)`: the marriage fact plus the joiner's
/// membership overwrite. WHO moves households (and to which faction) is the
/// author's choice per wedding — world content, not module policy.
///
/// The frozen guard checks `joiner` and `spouse` only; `faction` is checked by
/// the [`joins`] call this builds, which here runs eagerly rather than when the
/// outcome is forced.
pub fn wed(joiner: &str, faction: &str, spouse: &str) -> Result<Vec<Outcome>, WorldError> {
    if bad(joiner) || bad(spouse) {
        return Err(WorldError::NotASinglePathSegment {
            context: "Kin.wed".to_owned(),
            name: if bad(joiner) { joiner } else { spouse }.to_owned(),
        });
    }
    Ok(vec![
        insert(format!("married.{joiner}.{spouse}")),
        joins(joiner, faction)?,
    ])
}

/// Succession as exclusion: any child of the dead holder may claim; the
/// single-slot office takes one — first motivated claimant wins. No invented
/// primogeniture (age does not exist in the vocabulary).
pub fn succession(office: &str) -> Result<Action, WorldError> {
    if bad(office) {
        return Err(WorldError::NotASinglePathSegment {
            context: "Kin.succession".to_owned(),
            name: office.to_owned(),
        });
    }
    Ok(
        Action::new(format!("[Actor]: claim the office of {office}"))
            .when([
                matches(format!("office.{office}!H")),
                matches("dead.H"),
                matches("parent.H.Actor"),
                neq("Actor", "H"),
            ])
            .then([insert(format!("office.{office}!Actor"))]),
    )
}

#[cfg(test)]
mod tests {
    // H: KinSpec.hs "Prax.Kin"
    //
    // The frozen `Prax.KinSpec`, re-expressed against the Rust engine.
    use super::*;
    use crate::faction::comrades;
    use prax_core::engine::{GroundedAction, State};
    use prax_core::types::{Practice, delete};

    /// Two generations: gran > pat > {ana, ben}; a separate unmarried family
    /// mia > {cass, dan}; ana marries chris (a stranger to both families).
    fn kin_world() -> State {
        let mut st = State::new();
        for s in [
            "parent.gran.pat",
            "parent.pat.ana",
            "parent.pat.ben",
            "parent.mia.cass",
            "parent.mia.dan",
            "married.ana.chris",
        ] {
            st.perform_outcome(&insert(s)).unwrap();
        }
        st.set_axioms(kin_axioms()).unwrap();
        st
    }

    fn kin_view() -> Vec<String> {
        kin_world().labeled_view()
    }

    /// Two houses (hall: ana, ben; yard: cass), plus ana's parent pat, so a
    /// wedding into yard has an in-law to un-derive on dissolution.
    fn house_world() -> State {
        let mut st = State::new();
        for o in [
            joins("ana", "hall").unwrap(),
            joins("ben", "hall").unwrap(),
            joins("cass", "yard").unwrap(),
            insert("parent.pat.ana"),
        ] {
            st.perform_outcome(&o).unwrap();
        }
        let mut axioms = kin_axioms();
        axioms.push(comrades());
        st.set_axioms(axioms).unwrap();
        st
    }

    fn wedded_world() -> State {
        let mut st = house_world();
        for o in wed("ana", "yard", "cass").unwrap() {
            st.perform_outcome(&o).unwrap();
        }
        st
    }

    /// Succession fixture: a role-free practice hosting the claim action for
    /// office "throne", rex the holder, ana and ben his children, cass
    /// unrelated. A zero-role practice is spawned by inserting the bare
    /// `practice.succession` fact with no trailing role value: `possible_actions`
    /// only requires the instance fact to exist and unify — there are no role
    /// values to bind.
    fn succession_world() -> State {
        let mut st = State::new();
        st.define_practice(
            Practice::new("succession")
                .roles(Vec::<String>::new())
                .action(succession("throne").unwrap()),
        )
        .unwrap();
        for s in [
            "practice.succession",
            "office.throne!rex",
            "parent.rex.ana",
            "parent.rex.ben",
        ] {
            st.perform_outcome(&insert(s)).unwrap();
        }
        st
    }

    fn opts(st: &mut State, actor: &str) -> Vec<String> {
        st.possible_actions(actor)
            .into_iter()
            .map(|ga| ga.label)
            .collect()
    }

    fn can_claim(st: &mut State, actor: &str) -> bool {
        opts(st, actor)
            .iter()
            .any(|l| l.contains("claim the office of throne"))
    }

    fn find(st: &mut State, actor: &str, needle: &str) -> GroundedAction {
        st.possible_actions(actor)
            .into_iter()
            .find(|ga| ga.label.contains(needle))
            .unwrap_or_else(|| panic!("no {needle:?} for {actor}"))
    }

    // H: KinSpec.hs "marriage symmetry: married.A.B derives married.B.A"
    #[test]
    fn marriage_is_symmetric() {
        assert!(
            kin_view().contains(&"married.chris.ana".to_owned()),
            "chris married ana too"
        );
    }

    // H: KinSpec.hs "marriage symmetry negative: an unmarried pair derives no symmetric fact"
    #[test]
    fn marriage_symmetry_invents_no_marriage() {
        let view = kin_view();
        for s in ["married.dan.cass", "married.cass.dan"] {
            assert!(
                !view.contains(&s.to_owned()),
                "dan and cass are never married: {s}"
            );
        }
    }

    // H: KinSpec.hs "sibling: shared parent, X<>Y, derives sibling both ways"
    #[test]
    fn a_shared_parent_derives_siblinghood_both_ways() {
        let view = kin_view();
        for s in ["sibling.ana.ben", "sibling.ben.ana"] {
            assert!(view.contains(&s.to_owned()), "ana and ben are siblings: {s}");
        }
    }

    // H: KinSpec.hs "sibling negative: no shared parent, no sibling fact"
    #[test]
    fn no_shared_parent_derives_no_siblinghood() {
        let view = kin_view();
        for s in ["sibling.ana.cass", "sibling.cass.ana"] {
            assert!(
                !view.contains(&s.to_owned()),
                "ana and cass share no parent, not siblings: {s}"
            );
        }
    }

    // H: KinSpec.hs "grandparent: parent-of-parent derives grandparent"
    #[test]
    fn parent_of_parent_derives_grandparent() {
        assert!(
            kin_view().contains(&"grandparent.gran.ana".to_owned()),
            "gran is ana's grandparent"
        );
    }

    // H: KinSpec.hs "grandparent negative: no parent chain, no grandparent fact"
    #[test]
    fn no_parent_chain_derives_no_grandparent() {
        assert!(
            !kin_view().contains(&"grandparent.gran.chris".to_owned()),
            "gran is not chris's grandparent"
        );
    }

    // H: KinSpec.hs "inLaw (spouse's parent): married.A.B + parent.P.A derives inLaw.P.B"
    #[test]
    fn a_spouses_parent_is_an_in_law() {
        assert!(
            kin_view().contains(&"inLaw.pat.chris".to_owned()),
            "pat (ana's parent) is chris's in-law"
        );
    }

    // H: KinSpec.hs "inLaw (spouse's parent) negative: no marriage, no in-law via that parent"
    #[test]
    fn an_unmarried_childs_parent_is_no_ones_in_law() {
        assert!(
            !kin_view().iter().any(|s| s.starts_with("inLaw.mia.")),
            "mia's unmarried children yield no inLaw.mia.* fact at all"
        );
    }

    // H: KinSpec.hs "inLaw (sibling's spouse): married.A.B + sibling.A.S derives inLaw.S.B"
    #[test]
    fn a_siblings_spouse_is_an_in_law() {
        assert!(
            kin_view().contains(&"inLaw.ben.chris".to_owned()),
            "ben (ana's sibling) is chris's in-law"
        );
    }

    // H: KinSpec.hs "inLaw (sibling's spouse) negative: siblings without a marriage derive no in-law"
    #[test]
    fn unmarried_siblings_derive_no_in_law() {
        assert!(
            !kin_view()
                .iter()
                .any(|s| s.starts_with("inLaw.cass.") || s.starts_with("inLaw.dan.")),
            "cass and dan are siblings but unmarried — neither is anyone's in-law"
        );
    }

    // H: KinSpec.hs "wed: guards forced — joiner and spouse must be single path segments"
    #[test]
    fn wed_rejects_multi_segment_names() {
        for (joiner, spouse) in [("", "cass"), ("ana", "b.ad"), ("ana", "b!ad")] {
            assert!(
                matches!(
                    wed(joiner, "hall", spouse),
                    Err(WorldError::NotASinglePathSegment { .. })
                ),
                "wed {joiner:?} {spouse:?} must be a loud rejection"
            );
        }
    }

    // H: KinSpec.hs "wed: inserts the marriage fact and overwrites the joiner's membership"
    #[test]
    fn wed_marries_and_moves_the_membership() {
        let mut st = wedded_world();
        assert!(
            st.db_has("married.ana.cass"),
            "married.ana.cass is a base fact"
        );
        assert!(
            st.db_has("member.ana!yard"),
            "ana's membership moved to yard"
        );
        assert!(
            !st.db_has("member.ana!hall"),
            "ana's old hall membership is gone"
        );
    }

    // H: KinSpec.hs "wed: the membership overwrite un-derives the joiner's old alliances (Faction composition)"
    #[test]
    fn wed_un_derives_the_joiners_old_alliances() {
        let mut st = wedded_world();
        assert!(
            !st.view_has("allied.ana.ben"),
            "ana no longer allied with ben (old house)"
        );
        assert!(
            !st.view_has("allied.ben.ana"),
            "ben no longer allied with ana (old house)"
        );
        assert!(
            st.view_has("allied.ana.cass"),
            "ana is now allied with cass (new house)"
        );
        assert!(
            st.view_has("allied.cass.ana"),
            "cass is now allied with ana (new house)"
        );
    }

    // H: KinSpec.hs "dissolution: retracting married un-derives in-laws but leaves membership UNCHANGED"
    #[test]
    fn dissolution_un_derives_in_laws_and_leaves_membership() {
        let mut dissolved = wedded_world();
        assert!(
            dissolved.view_has("inLaw.pat.cass"),
            "pre-dissolution: pat (ana's parent) is cass's in-law (via ana's marriage)"
        );
        dissolved
            .perform_outcome(&delete("married.ana.cass"))
            .unwrap();
        assert!(
            !dissolved.view_has("inLaw.pat.cass"),
            "post-dissolution: the in-law fact is gone"
        );
        assert!(
            !dissolved.view_has("married.cass.ana"),
            "post-dissolution: the symmetric marriage fact is gone"
        );
        assert!(
            dissolved.db_has("member.ana!yard"),
            "post-dissolution: ana's membership is UNCHANGED in the base"
        );
        assert!(
            dissolved.view_has("member.ana!yard"),
            "post-dissolution: ana's membership is UNCHANGED in the view"
        );
        assert!(
            dissolved.view_has("allied.ana.cass"),
            "the designed asymmetry: membership-derived alliance survives (it was never a kin derivation)"
        );
    }

    // H: KinSpec.hs "succession: not offered while the holder lives"
    #[test]
    fn succession_is_not_offered_while_the_holder_lives() {
        let mut st = succession_world();
        assert!(
            !can_claim(&mut st, "ana"),
            "ana cannot claim the throne while rex lives"
        );
    }

    // H: KinSpec.hs "succession: only children may claim, once the holder is dead"
    #[test]
    fn only_children_may_claim_a_dead_holders_office() {
        let mut st = succession_world();
        st.perform_outcome(&insert("dead.rex")).unwrap();
        assert!(can_claim(&mut st, "ana"), "ana (a child) may claim");
        assert!(can_claim(&mut st, "ben"), "ben (a child) may claim");
        assert!(
            !can_claim(&mut st, "cass"),
            "cass (not a child of rex) may not claim"
        );
    }

    // H: KinSpec.hs "succession: a performed claim overwrites the slot and closes the affordance for the other child"
    #[test]
    fn a_performed_claim_closes_the_race() {
        let mut st = succession_world();
        st.perform_outcome(&insert("dead.rex")).unwrap();
        let ga = find(&mut st, "ana", "claim the office of throne");
        st.perform_action(&ga);
        assert!(
            st.db_has("office.throne!ana"),
            "the office now belongs to ana"
        );
        assert!(
            !st.db_has("office.throne!rex"),
            "rex's old holding is gone (single-slot overwrite)"
        );
        assert!(
            !can_claim(&mut st, "ben"),
            "ben can no longer claim — the race is closed"
        );
        assert!(
            !can_claim(&mut st, "ana"),
            "ana cannot re-claim her own office (she is not dead)"
        );
    }

    // H: KinSpec.hs "succession: guards forced — office names must be single path segments"
    #[test]
    fn succession_rejects_multi_segment_office_names() {
        for office in ["", "a.b", "a!b"] {
            assert!(
                matches!(
                    succession(office),
                    Err(WorldError::NotASinglePathSegment { .. })
                ),
                "succession {office:?} must be a loud rejection"
            );
        }
    }
}
