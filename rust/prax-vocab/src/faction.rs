//! Factions: one membership spine (spec `docs/specs/2026-07-12-v31-faction-kin.md` §1).
//!
//! Membership is a base, single-slot fact — `member.<who>!<faction>` — and the
//! `!` IS the semantics: joining, defecting, and marrying-in are all the same
//! exclusion overwrite. Base `allied.*` facts remain legal vocabulary (not every
//! alliance is a membership); [`comrades`] derives additional ones.
//!
//! Frozen reference: `src/Prax/Faction.hs`. [`comrades`] and [`faction_standing`]
//! are AXIOM builders — they change what the planner can READ, not merely what
//! it can do, so a one-character drift here renders plausibly and misbehaves
//! silently (S7 design §3.4). Every guard the frozen module raises with `error`
//! is a [`WorldError`] here, at the same input and in the same order.

use prax_core::error::WorldError;
use prax_core::interner::is_variable_name;
use prax_core::path::segment_names;
use prax_core::query::{matches, neq};
use prax_core::types::{Axiom, Outcome, authored_pat_clash, insert};

/// The frozen `bad`: a name spliced into a built path must be nonempty and
/// carry no separator. Transcribed rather than routed through
/// [`prax_core::path::segment_names`] — that one trims whitespace first, which
/// would reject a name the frozen guard accepts.
fn bad(n: &str) -> bool {
    n.is_empty() || n.contains(['.', '!'])
}

/// `member.<who>!<faction>` (single-slot: the primary allegiance).
pub fn member_path(who: &str, faction: &str) -> Result<String, WorldError> {
    if bad(who) || bad(faction) {
        return Err(WorldError::NotASinglePathSegment {
            context: "Faction.member_path".to_owned(),
            name: if bad(who) { who } else { faction }.to_owned(),
        });
    }
    Ok(format!("member.{who}!{faction}"))
}

/// Join (or defect to, or marry into) a faction: one exclusion overwrite.
pub fn joins(who: &str, faction: &str) -> Result<Outcome, WorldError> {
    Ok(insert(member_path(who, faction)?))
}

/// Shared membership derives alliance — the feud's old base facts, generalized.
/// The derived name stays `allied` so every downstream consumer (mutuality,
/// enemy-of-my-ally, affordances) is unchanged.
pub fn comrades() -> Axiom {
    Axiom::new(
        vec![matches("member.X!F"), matches("member.Y!F"), neq("X", "Y")],
        ["allied.X.Y"],
    )
}

/// Belief-gated faction standing for K-discipline worlds: an offense against my
/// faction-mate, THAT I BELIEVE HAPPENED, makes me regard the offender.
///
/// `faction_standing(pat, label)`: `pat`'s FIRST variable is the offender, the
/// SECOND the victim (loud error otherwise) — e.g. `"struck.A.V"` ⇒
/// `PraxW.believes.struck.A.V ∧ member.V!PraxF ∧ member.PraxW!PraxF ∧ PraxW≠A ⇒
/// regards.PraxW.A.<label>`. The axiom's own believer/shared-faction join
/// variables live in the `Prax` namespace (v40): `pat` is free to name its own
/// variables `W` or `F` (or anything else) without colliding — only the `Prax`
/// namespace itself is off limits. Intra-faction offenders are condemned by
/// their own co-members: nothing exempts an offender who shares the victim's
/// faction (a co-member who believes the deed regards the offender all the same,
/// offender included in the population of possible believers — only the
/// offender's OWN belief of their own act is excluded, by the believer/offender
/// inequality).
pub fn faction_standing(pat: &str, label: &str) -> Result<Axiom, WorldError> {
    let names = segment_names(pat);
    let vars: Vec<&String> = names.iter().filter(|n| is_variable_name(n)).collect();
    let (offender, victim) = match vars.as_slice() {
        [offender, victim, ..] => (offender, victim),
        _ => {
            return Err(WorldError::PatternVariables {
                context: "Faction.faction_standing".to_owned(),
                pattern: pat.to_owned(),
                needs: "name an offender and a victim variable, in that order".to_owned(),
            });
        }
    };
    if let Some(clash) = authored_pat_clash(&[], &names).first() {
        return Err(WorldError::ReservedVarClash {
            context: "Faction.faction_standing".to_owned(),
            var: clash.clone(),
            extra: " -- it is reserved for the axiom's own join variables".to_owned(),
        });
    }
    Ok(Axiom::new(
        vec![
            matches(format!("PraxW.believes.{pat}")),
            matches(format!("member.{victim}!PraxF")),
            matches("member.PraxW!PraxF"),
            neq("PraxW", offender.as_str()),
        ],
        [format!("regards.PraxW.{offender}.{label}")],
    ))
}

#[cfg(test)]
mod tests {
    // H: FactionSpec.hs "Prax.Faction"
    //
    // The frozen `Prax.FactionSpec`, re-expressed against the Rust engine. The
    // guard cases assert the `WorldError` the frozen `error` became, at the same
    // inputs.
    use super::*;
    use prax_core::engine::State;

    /// Two houses: hall (ana, ben) and yard (cass). One shared axiom set
    /// (`comrades`).
    fn houses() -> State {
        let mut st = State::new();
        for o in [
            joins("ana", "hall").unwrap(),
            joins("ben", "hall").unwrap(),
            joins("cass", "yard").unwrap(),
        ] {
            st.perform_outcome(&o).unwrap();
        }
        st.set_axioms(vec![comrades()]).unwrap();
        st
    }

    // H: FactionSpec.hs "memberPath: single-slot exclusion fact"
    #[test]
    fn member_path_is_a_single_slot_exclusion_fact() {
        assert_eq!(
            member_path("ana", "hall").unwrap(),
            "member.ana!hall",
            "the ! separates who from faction"
        );
    }

    // H: FactionSpec.hs "comrades: shared membership derives allied, both directions"
    #[test]
    fn comrades_derives_allied_both_directions() {
        let mut st = houses();
        assert!(st.view_has("allied.ana.ben"), "ana allied ben");
        assert!(st.view_has("allied.ben.ana"), "ben allied ana");
    }

    // H: FactionSpec.hs "comrades: X<>Y guard — no self-alliance"
    #[test]
    fn comrades_neq_guard_forbids_self_alliance() {
        let mut st = houses();
        assert!(
            !st.view_has("allied.ana.ana"),
            "ana is not allied with herself"
        );
    }

    // H: FactionSpec.hs "comrades: cross-faction negative — no shared house, no alliance"
    #[test]
    fn comrades_derives_nothing_across_factions() {
        let mut st = houses();
        assert!(
            !st.view_has("allied.ana.cass"),
            "ana (hall) not allied with cass (yard)"
        );
        assert!(
            !st.view_has("allied.cass.ana"),
            "cass (yard) not allied with ana (hall)"
        );
    }

    // H: FactionSpec.hs "defection un-derives: joining a new faction overwrites the old, retracting stale allied pairs"
    #[test]
    fn defection_un_derives_the_old_alliances() {
        let mut moved = houses();
        moved
            .perform_outcome(&joins("ana", "yard").unwrap())
            .unwrap();
        assert!(
            !moved.view_has("allied.ana.ben"),
            "ana no longer allied with ben (old house)"
        );
        assert!(
            !moved.view_has("allied.ben.ana"),
            "ben no longer allied with ana (old house)"
        );
        assert!(
            moved.view_has("allied.ana.cass"),
            "ana is now allied with cass (new house)"
        );
        assert!(
            moved.view_has("allied.cass.ana"),
            "cass is now allied with ana (new house)"
        );
        assert!(
            !moved.db_has("member.ana!hall"),
            "ana's old membership is gone from the base"
        );
        assert!(
            moved.db_has("member.ana!yard"),
            "ana's new membership is the sole base fact"
        );
    }

    /// A standing world under one `faction_standing` axiom, over the given extra
    /// setup facts.
    fn standing_world(memberships: &[(&str, &str)], facts: &[&str]) -> State {
        let mut st = State::new();
        for (who, faction) in memberships {
            st.perform_outcome(&joins(who, faction).unwrap()).unwrap();
        }
        for f in facts {
            st.perform_outcome(&insert(*f)).unwrap();
        }
        st.set_axioms(vec![faction_standing("struck.A.V", "brutal").unwrap()])
            .unwrap();
        st
    }

    // H: FactionSpec.hs "factionStanding: an unbelieved offense moves no one"
    #[test]
    fn faction_standing_needs_a_belief() {
        let mut st = standing_world(
            &[
                ("ana", "hall"),
                ("ben", "hall"),
                ("dave", "hall"),
                ("cass", "yard"),
            ],
            &[],
        );
        for s in [
            "regards.ben.ana.brutal",
            "regards.dave.ana.brutal",
            "regards.cass.ana.brutal",
        ] {
            assert!(
                !st.view_has(s),
                "no one regards ana as brutal (no belief asserted): {s}"
            );
        }
    }

    // H: FactionSpec.hs "factionStanding: a believed offense moves co-members only"
    #[test]
    fn faction_standing_moves_co_members_only() {
        let mut st = standing_world(
            &[
                ("ana", "hall"),
                ("ben", "hall"),
                ("dave", "hall"),
                ("cass", "yard"),
            ],
            &[
                "dave.believes.struck.ana.ben",
                "cass.believes.struck.ana.ben",
                "ana.believes.struck.ana.ben",
            ],
        );
        assert!(
            st.view_has("regards.dave.ana.brutal"),
            "dave (co-member of victim ben's faction) regards ana as brutal"
        );
        assert!(
            !st.view_has("regards.cass.ana.brutal"),
            "cass (a different faction) believes it too but derives nothing"
        );
        assert!(
            !st.view_has("regards.ana.ana.brutal"),
            "ana (the offender) derives no self-regard, even believing her own act"
        );
    }

    // H: FactionSpec.hs "factionStanding: the victim's own belief of the offense against them derives their regard too"
    #[test]
    fn faction_standing_covers_the_victims_own_belief() {
        let mut st = standing_world(
            &[("ana", "hall"), ("ben", "hall")],
            &["ben.believes.struck.ana.ben"],
        );
        assert!(
            st.view_has("regards.ben.ana.brutal"),
            "ben (the victim, a co-member of himself trivially) regards ana as brutal"
        );
    }

    // H: FactionSpec.hs "factionStanding: defection dissolves the regard (retraction's sharpest case)"
    #[test]
    fn faction_standing_dissolves_on_defection() {
        // the regard is DERIVED from co-membership; membership is a base fact.
        // dave believed the offense while a hall member and regarded ana; the
        // moment he defects, the derivation loses its join and the regard is
        // gone — while his belief (a base fact) persists untouched.
        let mut held = standing_world(
            &[("ana", "hall"), ("ben", "hall"), ("dave", "hall")],
            &["dave.believes.struck.ana.ben"],
        );
        assert!(
            held.view_has("regards.dave.ana.brutal"),
            "co-membered: dave regards ana"
        );
        let mut defected = held;
        defected
            .perform_outcome(&joins("dave", "yard").unwrap())
            .unwrap();
        assert!(
            !defected.view_has("regards.dave.ana.brutal"),
            "defected: the regard un-derives"
        );
        assert!(
            defected.db_has("dave.believes.struck.ana.ben"),
            "his belief persists — only the solidarity is gone"
        );
    }

    // H: FactionSpec.hs "memberPath: an empty or separator-bearing name errors loudly"
    #[test]
    fn member_path_rejects_empty_and_separator_bearing_names() {
        for (who, faction) in [("", "hall"), ("a.b", "hall"), ("ana", "ha!ll")] {
            assert!(
                matches!(
                    member_path(who, faction),
                    Err(WorldError::NotASinglePathSegment { .. })
                ),
                "memberPath {who:?} {faction:?} must be a loud rejection"
            );
        }
    }

    // H: FactionSpec.hs "factionStanding: a pattern naming fewer than two variables errors loudly"
    #[test]
    fn faction_standing_needs_two_variables() {
        assert!(
            matches!(
                faction_standing("struck.A.constant", "brutal"),
                Err(WorldError::PatternVariables { .. })
            ),
            "an offender-only pattern is an error, not a silent single-variable guard"
        );
    }

    // H: FactionSpec.hs "factionStanding: the usability win -- W and F are ordinary variables now (v40 moved the axiom's own join variables to the Prax namespace)"
    #[test]
    fn faction_standing_leaves_w_and_f_to_the_author() {
        assert!(
            faction_standing("struck.W.V", "brutal").is_ok(),
            "an offender named W no longer collides with anything"
        );
        assert!(
            faction_standing("struck.A.F", "brutal").is_ok(),
            "a victim named F no longer collides with anything"
        );
    }

    // H: FactionSpec.hs "factionStanding: a pattern authoring the Prax namespace errors loudly"
    #[test]
    fn faction_standing_rejects_the_prax_namespace() {
        assert!(
            matches!(
                faction_standing("struck.PraxW.V", "brutal"),
                Err(WorldError::ReservedVarClash { .. })
            ),
            "an offender named PraxW collides with the axiom's own (now-namespaced) believer variable"
        );
    }
}
