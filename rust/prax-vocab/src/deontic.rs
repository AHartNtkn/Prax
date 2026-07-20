//! Obligations and their `□`-closure (DEON property 1): the `obligedLift`/
//! `obligedClose` OPERATORS that lift a world's rules into their normative twins
//! so an invoked obligation actually binds. The vocabulary CONSTANTS
//! (`obliged.*` head, the lifted prefix) live in [`prax_core::vocab_consts`] —
//! checker-visible without a crate cycle (design C1); this module builds the
//! operators on them.
//!
//! Frozen reference: `src/Prax/Deontic.hs` (`obligedLift`/`obligedClose`). The
//! paper's result (Evans, DEON 2010) is that a deontic logic needs no new
//! semantics over Exclusion Logic: `□P` is the ordinary fact `Ob:P`. This module
//! adds no engine machinery — a world that closes under `□` declares it with
//! `set_axioms(obliged_close(&rules))`, and the general derivation engine
//! ([`prax_core::derive`]) closes over exactly the list it is handed, lift
//! included. Stratification is the whole point (it avoids the SDL paradoxes): the
//! lift applies only to a purely-conjunctive rule, never one whose body uses a
//! non-`Match` condition.

use prax_core::query::Condition;
use prax_core::types::Axiom;
use prax_core::vocab_consts::OBLIGED_LIFT_PREFIX;

/// Prefix every `□`-lifted sentence with `obliged.Obligor.`.
fn lift_sent(s: &str) -> String {
    format!("{OBLIGED_LIFT_PREFIX}{s}")
}

/// Lift a purely-conjunctive domain rule under the obligation operator
/// (`Prax.Deontic.obligedLift`): prefix `obliged.Obligor.` to every body `Match`
/// and every head, so `□A ⊢ □B` whenever `A ⊢ B`. A rule whose body uses any
/// non-`Match` condition is not lifted (nothing sensible to place under `□`) —
/// `None`.
pub fn obliged_lift(ax: &Axiom) -> Option<Axiom> {
    if ax.when.iter().all(|c| matches!(c, Condition::Match(_))) {
        let when = ax
            .when
            .iter()
            .map(|c| match c {
                Condition::Match(s) => Condition::Match(lift_sent(s)),
                other => other.clone(),
            })
            .collect();
        let then = ax.then.iter().map(|s| lift_sent(s)).collect();
        Some(Axiom { when, then })
    } else {
        None
    }
}

/// Close a world's axioms under the obligation operator (DEON property 1): the
/// authored rules plus the `□`-lifted twin of every all-`Match` rule
/// (`Prax.Deontic.obligedClose`). A deontic world declares its closure with
/// `set_axioms(obliged_close(&rules))`; the general engine closes over exactly
/// this list, lift included.
pub fn obliged_close(axs: &[Axiom]) -> Vec<Axiom> {
    let mut out: Vec<Axiom> = axs.to_vec();
    out.extend(axs.iter().filter_map(obliged_lift));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_core::db::Db;
    use prax_core::derive::{CompiledRule, axiom_footprint, close};
    use prax_core::interner::{Interner, Sym};
    use prax_core::path::tokenize;
    use smallvec::SmallVec;

    fn m(s: &str) -> Condition {
        Condition::Match(s.to_owned())
    }

    fn compile_rules(i: &mut Interner, axs: &[Axiom]) -> Vec<CompiledRule> {
        axs.iter()
            .map(|ax| {
                let heads: Vec<&str> = ax.then.iter().map(String::as_str).collect();
                CompiledRule::compile(i, &ax.when, &heads).unwrap()
            })
            .collect()
    }

    fn has_path(i: &mut Interner, ps: &[SmallVec<[Sym; 6]>], s: &str) -> bool {
        let segs = tokenize(i, s).unwrap().segs;
        ps.contains(&segs)
    }

    fn build(i: &mut Interner, facts: &[&str]) -> Db {
        let mut db = Db::empty();
        for f in facts {
            db = db.insert_str(i, f).unwrap();
        }
        db
    }

    // A unit sanity check mirroring DeonticSpec's obligedLift/obligedClose
    // structural pins; DeonticSpec.hs proper is not in the S4 allowlist (only the
    // operator is in scope), so no // H is consumed here.
    #[test]
    fn obliged_lift_and_close_shapes() {
        let liftable = Axiom::new(vec![m("a.X")], ["b.X"]);
        let unliftable = Axiom::new(vec![m("c.Y"), Condition::Not("d.Y".into())], ["e.Y"]);
        assert_eq!(
            obliged_lift(&liftable),
            Some(Axiom::new(
                vec![m("obliged.Obligor.a.X")],
                ["obliged.Obligor.b.X"]
            ))
        );
        assert_eq!(obliged_lift(&unliftable), None);
        assert_eq!(
            obliged_close(&[liftable.clone(), unliftable.clone()]),
            vec![
                liftable,
                unliftable,
                Axiom::new(vec![m("obliged.Obligor.a.X")], ["obliged.Obligor.b.X"]),
            ]
        );
    }

    // The two owed:S4 DeriveSpec discharges that need obliged_close in scope
    // (prax-vocab depends on prax-core, so both axiom_footprint and obliged_close
    // are available here).

    // H: DeriveSpec.hs "axiomFootprint collects bodies (any polarity) and heads; obligedClose adds the lifted forms"
    #[test]
    fn axiom_footprint_collects_bodies_heads_and_obliged_close_lifts() {
        let mut i = Interner::new();
        let ax = Axiom::new(
            vec![m("parent.X.Y"), Condition::Absent(vec![m("dead.X")])],
            ["elder.X"],
        );
        let fp = axiom_footprint(&compile_rules(&mut i, &[ax]));
        assert!(has_path(&mut i, &fp, "parent.X.Y"), "body atom");
        assert!(has_path(&mut i, &fp, "dead.X"), "negated body atom");
        assert!(has_path(&mut i, &fp, "elder.X"), "head");

        // cookAxioms is deontics-free: a bare all-Match rule contributes no
        // lifted twin. Declaring the closure adds the lifted rule, whose body and
        // head then appear in the footprint.
        let base_ax = [Axiom::new(vec![m("a.X")], ["b.X"])];
        let bare = axiom_footprint(&compile_rules(&mut i, &base_ax));
        let closed = axiom_footprint(&compile_rules(&mut i, &obliged_close(&base_ax)));
        assert!(has_path(&mut i, &bare, "a.X"), "bare base body kept");
        assert!(has_path(&mut i, &bare, "b.X"), "bare base head kept");
        assert!(!has_path(&mut i, &bare, "obliged.Obligor.a.X"), "bare has no lifted body");
        assert!(!has_path(&mut i, &bare, "obliged.Obligor.b.X"), "bare has no lifted head");
        assert!(has_path(&mut i, &closed, "obliged.Obligor.a.X"), "obligedClose lifts body");
        assert!(has_path(&mut i, &closed, "obliged.Obligor.b.X"), "obligedClose lifts head");
    }

    // H: DeriveSpec.hs "obligedClose: a domain rule (written once) also closes under obligation"
    #[test]
    fn obliged_close_closes_an_obliged_context() {
        let mut i = Interner::new();
        let axs = [Axiom::new(vec![m("at.W.bar")], ["in.W.building"])];
        // bex ought to be at the bar.
        let base = build(&mut i, &["obliged.bex.at.bex.bar"]);

        // Under the declared closure, an obliged context derives the sub-obligation.
        let closed_rules = compile_rules(&mut i, &obliged_close(&axs));
        let closed = close(&mut i, &closed_rules, &base).unwrap();
        assert!(
            closed
                .to_sentences(&i)
                .contains(&"obliged.bex.in.bex.building".to_owned()),
            "sub-obligation derived under declared closure (□a ⊢ □b)"
        );

        // Bare closure does NOT lift: no auto-obligation.
        let bare_rules = compile_rules(&mut i, &axs);
        let bare = close(&mut i, &bare_rules, &base).unwrap();
        assert!(
            !bare
                .to_sentences(&i)
                .contains(&"obliged.bex.in.bex.building".to_owned()),
            "bare closure does NOT lift"
        );
    }
}
