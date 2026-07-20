//! Debt: an obligation with a beneficiary.
//!
//! [`crate::deontic`]'s obligation already carries a duty-bearer
//! (`obliged.<who>.<content>`), but names no one to whom the duty is owed. A
//! debt is exactly that obligation PLUS a beneficiary: the new fact
//! `debt.<creditor>.<debtor>.<content>` sits beside the (unmodified) obligation,
//! so the rest of the deontic machinery ([`crate::deontic::fulfilled`],
//! [`crate::deontic::in_breach`], [`crate::deontic::breach`], …) keeps working on
//! `content` exactly as authored — a debt is thin vocabulary over Deontic, not a
//! parallel mechanism.
//!
//! **Stratification carries over unchanged** (Deontic's rule, restated here):
//! `content` is a SIMPLE TERM, one sentence, never a compound ∧/∨/→ — it may
//! itself be a dotted path (e.g. `"repaid.dell.cora.coin"`), just never a
//! conjunction of separate obligations. What IS guarded here, loudly, in the
//! established segment idiom ([`crate::project`]'s `endeavor`,
//! [`crate::persona`]): a `creditor`/`debtor` name must be a single path segment
//! (no `.` or `!`) — unlike `content`, these name a party, not a duty, and a
//! dotted party name would silently misparse the fact's four-part shape.
//!
//! Frozen reference: `src/Prax/Debt.hs`. The frozen `debtPath` raises `error` on
//! a punctuated party name; here that is a [`WorldError::NotASinglePathSegment`],
//! which [`owe`]/[`settle`]/[`owes`] propagate.

use prax_core::error::WorldError;
use prax_core::query::{Condition, matches};
use prax_core::types::{Outcome, delete, insert};

use crate::deontic::{discharge, oblige};

/// The frozen ``any (`elem` ".!")`` party-name guard. A party name is spliced
/// between two `.` separators, so a separator inside it would nest one family
/// under another and misparse the fact's four-part shape.
fn punctuated(n: &str) -> bool {
    n.contains(['.', '!'])
}

/// The DB path of a debt: `debt.<creditor>.<debtor>.<content>`.
///
/// # Errors
/// [`WorldError::NotASinglePathSegment`] if the creditor or debtor name carries
/// a `.`/`!` — the creditor first, matching the frozen guard order.
pub fn debt_path(creditor: &str, debtor: &str, content: &str) -> Result<String, WorldError> {
    if punctuated(creditor) || punctuated(debtor) {
        return Err(WorldError::NotASinglePathSegment {
            context: "Debt.debt_path".to_owned(),
            name: if punctuated(creditor) {
                creditor
            } else {
                debtor
            }
            .to_owned(),
        });
    }
    Ok(format!("debt.{creditor}.{debtor}.{content}"))
}

/// `owe(creditor, debtor, content)` — debtor now owes creditor: the debt fact
/// AND the underlying [`crate::deontic::oblige`], in one call (a debt IS an
/// obligation with a beneficiary; both facts assert together, or not at all).
///
/// # Errors
/// [`debt_path`]'s party-name rejection.
pub fn owe(creditor: &str, debtor: &str, content: &str) -> Result<Vec<Outcome>, WorldError> {
    Ok(vec![
        insert(debt_path(creditor, debtor, content)?),
        oblige(debtor, content),
    ])
}

/// `settle(creditor, debtor, content)` — the debt is cleared: deletes the debt
/// fact and [`crate::deontic::discharge`]s the obligation. The world supplies
/// whatever transfer action earns this (a repay/gift/forgive act); [`settle`] is
/// only the bookkeeping that follows.
///
/// # Errors
/// [`debt_path`]'s party-name rejection.
pub fn settle(creditor: &str, debtor: &str, content: &str) -> Result<Vec<Outcome>, WorldError> {
    Ok(vec![
        delete(debt_path(creditor, debtor, content)?),
        discharge(debtor, content),
    ])
}

/// Condition: `debtor` currently owes `creditor` `content`.
///
/// # Errors
/// [`debt_path`]'s party-name rejection.
pub fn owes(creditor: &str, debtor: &str, content: &str) -> Result<Condition, WorldError> {
    Ok(matches(debt_path(creditor, debtor, content)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_core::engine::State;
    use prax_core::query::{eq, not_};
    use prax_core::types::{Action, Character, Practice};
    use prax_core::vocab_consts::obligation_path;

    use crate::deontic::breach;
    use crate::repute::standing_unless;
    use crate::witness::{CoPresence, observable};

    // H: DebtSpec.hs "Prax.Debt"
    //
    // The frozen `Prax.DebtSpec`, re-expressed against the Rust engine, over the
    // spec's own three-character fixture.
    //
    // The tale: cora lends dell a favor. When dell stiffs her, cora calls it in
    // (co-present) — a witnessed default sours dell's standing with whoever was
    // there to see it; an unwitnessed one leaves no such trace. Repaying settles
    // the debt AND the standing, though the witness's own memory never clears.

    fn together() -> CoPresence {
        vec![matches("at.Actor!P"), matches("at.Witness!P")]
    }

    fn demand() -> Action {
        observable(
            &together(),
            "violated.dell.defaulted",
            Action::new("[Actor]: demand dell repay the favor")
                .when([
                    owes("Actor", "dell", "favor").unwrap(),
                    matches("at.Actor!P"),
                    matches("at.dell!P"),
                    not_("demanded.Actor.dell.favor"),
                ])
                .then([
                    insert("demanded.Actor.dell.favor"),
                    breach("dell", "defaulted"),
                ]),
        )
    }

    fn repay() -> Action {
        let mut then = settle("cora", "dell", "favor").unwrap();
        then.push(insert("atoned.dell"));
        then.push(delete("violated.dell.defaulted"));
        Action::new("[Actor]: dell repays cora the favor")
            .when([eq("Actor", "dell"), owes("cora", "Actor", "favor").unwrap()])
            .then(then)
    }

    fn debt_practice() -> Practice {
        Practice::new("debt")
            .roles(["R"])
            .action(demand())
            .action(repay())
    }

    /// `witness_present` toggles whether ren shares cora and dell's square
    /// (versus being off at the mill) when the default happens.
    fn mk_world(witness_present: bool) -> State {
        let mut st = State::new();
        st.define_practices([debt_practice()]).unwrap();
        st.set_characters(
            ["cora", "dell", "ren"]
                .into_iter()
                .map(Character::new)
                .collect(),
        )
        .unwrap();
        let mut setup = vec![insert("practice.debt.here")];
        setup.extend(owe("cora", "dell", "favor").unwrap());
        setup.push(insert("at.cora!square"));
        setup.push(insert("at.dell!square"));
        setup.push(insert(format!(
            "at.ren!{}",
            if witness_present { "square" } else { "mill" }
        )));
        for o in &setup {
            st.perform_outcome(o).expect("debt setup");
        }
        st.set_axioms(vec![
            standing_unless("violated.Debtor.defaulted", "atoned.Debtor", "deadbeat").unwrap(),
        ])
        .unwrap();
        st
    }

    /// Perform the named actor's action whose label mentions `needle`.
    fn do_act(who: &str, needle: &str, st: &mut State) {
        let found = st
            .possible_actions(who)
            .into_iter()
            .find(|ga| ga.label.contains(needle));
        match found {
            Some(ga) => st.perform_action(&ga),
            None => panic!(
                "no action for {who} matching {needle:?}; had: {:?}",
                st.possible_actions(who)
                    .into_iter()
                    .map(|ga| ga.label)
                    .collect::<Vec<_>>()
            ),
        }
    }

    // H: DebtSpec.hs "fact conventions"
    // H: DebtSpec.hs "debtPath is debt.<creditor>.<debtor>.<content>"
    #[test]
    fn debt_path_is_debt_creditor_debtor_content() {
        assert_eq!(
            debt_path("cora", "dell", "favor").unwrap(),
            "debt.cora.dell.favor"
        );
    }

    // H: DebtSpec.hs "owes matches the debt fact"
    #[test]
    fn owes_matches_the_debt_fact() {
        assert_eq!(
            owes("cora", "dell", "favor").unwrap(),
            Condition::Match("debt.cora.dell.favor".to_owned())
        );
    }

    // H: DebtSpec.hs "owe asserts the debt fact and Deontic's oblige, in one call"
    #[test]
    fn owe_asserts_the_debt_fact_and_deontics_oblige() {
        assert_eq!(
            owe("cora", "dell", "favor").unwrap(),
            vec![insert("debt.cora.dell.favor"), oblige("dell", "favor")]
        );
    }

    // H: DebtSpec.hs "settle retracts the debt fact and Deontic's discharge"
    #[test]
    fn settle_retracts_the_debt_fact_and_deontics_discharge() {
        assert_eq!(
            settle("cora", "dell", "favor").unwrap(),
            vec![delete("debt.cora.dell.favor"), discharge("dell", "favor")]
        );
    }

    // H: DebtSpec.hs "lifecycle: owe creates BOTH facts; settle removes BOTH"
    // H: DebtSpec.hs "owe inserts the debt fact and the obligation; settle removes both"
    #[test]
    fn owe_inserts_both_facts_and_settle_removes_both() {
        let content = "repaid.dell.cora.coin";
        let mut st = State::new();
        for o in owe("cora", "dell", content).unwrap() {
            st.perform_outcome(&o).unwrap();
        }
        assert!(
            st.db_has("debt.cora.dell.repaid.dell.cora.coin"),
            "debt fact"
        );
        assert!(
            st.db_has(&obligation_path("dell", content)),
            "obligation fact"
        );
        for o in settle("cora", "dell", content).unwrap() {
            st.perform_outcome(&o).unwrap();
        }
        assert!(
            !st.db_has("debt.cora.dell.repaid.dell.cora.coin"),
            "debt fact gone"
        );
        assert!(
            !st.db_has(&obligation_path("dell", content)),
            "obligation gone"
        );
    }

    // H: DebtSpec.hs "guards"
    // H: DebtSpec.hs "a dotted creditor name errors loudly"
    #[test]
    fn a_dotted_creditor_name_errors_loudly() {
        assert_eq!(
            debt_path("cor.a", "dell", "favor"),
            Err(WorldError::NotASinglePathSegment {
                context: "Debt.debt_path".to_owned(),
                name: "cor.a".to_owned(),
            })
        );
    }

    // H: DebtSpec.hs "a punctuated debtor name errors loudly"
    #[test]
    fn a_punctuated_debtor_name_errors_loudly() {
        assert_eq!(
            debt_path("cora", "de!l", "favor"),
            Err(WorldError::NotASinglePathSegment {
                context: "Debt.debt_path".to_owned(),
                name: "de!l".to_owned(),
            })
        );
    }

    // H: DebtSpec.hs "demand -> deadbeat: belief-gated standing, defeated by settling"
    // H: DebtSpec.hs "an unwitnessed default derives no third-party regard"
    #[test]
    fn an_unwitnessed_default_derives_no_third_party_regard() {
        let mut st = mk_world(false);
        do_act("cora", "demand", &mut st);
        assert!(
            st.db_has("violated.dell.defaulted"),
            "the world records the breach"
        );
        assert!(
            !st.db_has("ren.believes.violated.dell.defaulted.seen"),
            "ren, elsewhere, holds no belief of it"
        );
        assert!(
            !st.view_has("regards.ren.dell.deadbeat"),
            "and so derives no THIRD-PARTY deadbeat regard"
        );
        assert!(
            st.view_has("regards.dell.dell.deadbeat"),
            "dell is unavoidably co-present at his own default, so he still \
             witnesses (and regards) himself — belief-gating is per observer, \
             not a blanket suppression"
        );
    }

    // H: DebtSpec.hs "a witnessed default derives deadbeat standing for the witness"
    #[test]
    fn a_witnessed_default_derives_deadbeat_standing_for_the_witness() {
        let mut st = mk_world(true);
        do_act("cora", "demand", &mut st);
        assert!(
            st.db_has("ren.believes.violated.dell.defaulted.seen"),
            "ren saw it"
        );
        assert!(
            st.view_has("regards.ren.dell.deadbeat"),
            "ren regards dell a deadbeat"
        );
    }

    // H: DebtSpec.hs "settling (atoned-shaped) defeats the standing; the witness's memory persists"
    #[test]
    fn settling_defeats_the_standing_but_the_witnesss_memory_persists() {
        let mut st = mk_world(true);
        do_act("cora", "demand", &mut st);
        do_act("dell", "repays", &mut st);
        assert!(
            !st.db_has("debt.cora.dell.favor"),
            "settled: the debt is gone"
        );
        assert!(
            !st.view_has("regards.ren.dell.deadbeat"),
            "no regard survives settling"
        );
        assert!(
            st.db_has("ren.believes.violated.dell.defaulted.seen"),
            "ren still remembers the default"
        );
    }
}
