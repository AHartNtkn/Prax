//! Minds as objects of belief — the planner's core trio (`Prax.Minds`, the
//! subset the planner imports [D-I3]): the believed-model lookup and the two
//! cooked-want plumbers `evaluateCooked` scores. The string-surfaced diagnostics
//! (`wantFor`/`selfWants`) and the common-knowledge axiom builders
//! (`professed`/`conventional`) are the frozen library surface and live in
//! `prax_vocab::minds`, where most `MindsSpec` pins land; the two that read this
//! crate's internals (`believedWants`' provenance law and the compiled
//! want/desire tables) are pinned here.
//!
//! Frozen reference: `src/Prax/Minds.hs`
//! (`believedDesires`/`cookedDesiresFor`/`cookedSelfWants`).

use std::collections::BTreeMap;

use crate::db::{Bindings, Db, Val};
use crate::interner::Interner;
use crate::path::tokenize;
use crate::query::{Cond, ground_cond};
use crate::types::{Character, Desire};

/// The vocabulary desires the predictor `p` believes (any provenance) the mover
/// `m` to have, in VOCABULARY ORDER (`Prax.Minds.believedDesires`). The model can
/// be wrong — it is the predictor's, not the mover's. Prefix-existence of
/// `<p>.believes.desires.<m>.<name>` in the view: any provenance child (`.seen`/
/// `.heard.<src>`/`.presumed`) satisfies, since the belief fact makes that path
/// an interior node.
pub(crate) fn believed_desires(
    interner: &mut Interner,
    view: &Db,
    desires: &[Desire],
    p: &str,
    m: &str,
) -> Vec<Desire> {
    desires
        .iter()
        .filter(|d| {
            let sentence = format!("{p}.believes.desires.{m}.{}", d.name);
            let path = tokenize(interner, &sentence).expect("belief path is well-formed");
            view.exists(interner, &path.segs)
        })
        .cloned()
        .collect()
}

/// Ground a list of desires' precooked templates (`cookedDesires`) for an owner,
/// pairing each with its utility (`Prax.Minds.cookedDesiresFor`) — the shared core
/// behind `cooked_self_wants` and the planner's believed-model lookup. Owner is
/// ground by SUBSTITUTION (`ground_cond`), matching each site's mechanism.
pub(crate) fn cooked_desires_for(
    interner: &mut Interner,
    desires_cooked: &BTreeMap<String, Vec<Cond>>,
    owner: &str,
    ds: &[Desire],
) -> Vec<(Vec<Cond>, i32)> {
    let mut b = Bindings::new();
    b.insert(interner.intern("Owner"), Val::Sym(interner.intern(owner)));
    ds.iter()
        .map(|d| {
            let conds = desires_cooked.get(&d.name).cloned().unwrap_or_default();
            let grounded: Vec<Cond> = conds.iter().map(|c| ground_cond(interner, &b, c)).collect();
            (grounded, d.want.utility)
        })
        .collect()
}

/// `selfWants`' cooked mirror: cooked conditions paired with utility, fed to
/// `evaluate_compiled` (`Prax.Minds.cookedSelfWants`). Traverses `charWants` (in
/// order, paired with the precooked `wants_table` by construction) then the
/// character's held vocabulary desires (in vocabulary order). Depends only on the
/// compiled tables and the character — never on state — so it is invariant across
/// a pick's forks (the §1 permitted hoist).
pub(crate) fn cooked_self_wants(
    interner: &mut Interner,
    wants_table: &BTreeMap<String, Vec<Vec<Cond>>>,
    desires_cooked: &BTreeMap<String, Vec<Cond>>,
    desires: &[Desire],
    c: &Character,
) -> Vec<(Vec<Cond>, i32)> {
    let mut out: Vec<(Vec<Cond>, i32)> = Vec::new();
    let empty = Vec::new();
    let cooked_wants = wants_table.get(&c.name).unwrap_or(&empty);
    for (cs, w) in cooked_wants.iter().zip(c.wants.iter()) {
        out.push((cs.clone(), w.utility));
    }
    let held: Vec<Desire> = desires
        .iter()
        .filter(|d| c.desires.contains(&d.name))
        .cloned()
        .collect();
    out.extend(cooked_desires_for(interner, desires_cooked, &c.name, &held));
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::db::Db;
    use crate::engine::State;
    use crate::interner::Interner;
    use crate::query::matches;
    use crate::types::{Want, insert, rename_vars};

    /// The frozen vocabulary of two desires: ida's sweet tooth is
    /// `Owner`-templated; rex's grudge names him outright.
    fn vocab() -> Vec<Desire> {
        vec![
            Desire::new(
                "sweet-tooth",
                Want::new(vec![matches("holding.Owner.cake")], 5),
            ),
            Desire::new("grudge-rex", Want::new(vec![matches("shamed.rex")], 7)),
        ]
    }

    /// `Prax.Minds.believedWants`: the predictor's believed model of the mover —
    /// every vocabulary desire the predictor believes (ANY provenance) the mover
    /// to have, instantiated for the mover. In the one compiled representation
    /// this is [`believed_desires`] composed with the `Owner` grounding
    /// `prax_vocab::minds::want_for` performs.
    fn believed_wants(
        interner: &mut Interner,
        view: &Db,
        desires: &[Desire],
        p: &str,
        m: &str,
    ) -> Vec<Want> {
        let subst = BTreeMap::from([("Owner".to_owned(), m.to_owned())]);
        believed_desires(interner, view, desires, p, m)
            .iter()
            .map(|d| Want::new(rename_vars(&subst, &d.want.when), d.want.utility))
            .collect()
    }

    // H: MindsSpec.hs "believedWants reads any provenance, and only believed desires"
    #[test]
    fn believed_wants_reads_any_provenance_and_only_believed_desires() {
        let mut i = Interner::new();
        let vocab = vocab();
        // The world as the profession leaves it: rex PRESUMES ida's sweet tooth
        // (a derived, `.presumed` motive-belief); nobody believes anything of rex.
        let base = Db::empty()
            .insert_str(&mut i, "rex.believes.desires.ida.sweet-tooth.presumed")
            .unwrap();

        assert_eq!(
            believed_wants(&mut i, &base, &vocab, "ida", "rex"),
            Vec::<Want>::new(),
            "ida believes no desire of rex's, so she models nothing"
        );

        // A `.heard.<src>` provenance is read exactly like any other.
        let told = base
            .insert_str(&mut i, "ida.believes.desires.rex.grudge-rex.heard.sam")
            .unwrap();
        assert_eq!(
            believed_wants(&mut i, &told, &vocab, "ida", "rex"),
            vec![Want::new(vec![matches("shamed.rex")], 7)],
            "hearsay counts — and ONLY the believed desire, not the whole vocabulary"
        );

        // And presumption counts too, grounded to the mover.
        assert_eq!(
            believed_wants(&mut i, &base, &vocab, "rex", "ida"),
            vec![Want::new(vec![matches("holding.ida.cake")], 5)]
        );
    }

    // H: MindsSpec.hs "setCharacters retables cookedWants; retable tracks cookedDesires"
    #[test]
    fn set_characters_retables_cooked_wants_and_retable_tracks_cooked_desires() {
        // `cookedWants` is keyed by character name, each want's conditions
        // precooked in `charWants`' own order and paired with that want's utility;
        // `cookedDesires` is keyed by desire name, the vocabulary's
        // `Owner`-template cooked ONCE, independent of who holds it. Both tables
        // are private to the compile pipeline, so they are read where the engine
        // reads them: through `cooked_self_wants`, which the scorer runs.
        let mut st = State::new();
        st.set_desires(vocab()).unwrap();
        let rex = Character::new("rex")
            .want(Want::new(vec![matches("x")], 1))
            .want(Want::new(vec![matches("y.Z")], 2))
            .holds("grudge-rex");
        st.set_characters(vec![Character::new("ida"), rex.clone()])
            .unwrap();

        for o in [
            insert("x"),
            insert("y.a"),
            insert("y.b"),
            insert("shamed.rex"),
            insert("holding.ida.cake"),
        ] {
            st.perform_outcome(&o).unwrap();
        }

        // The tables themselves, by name: `cookedWants` is keyed by character —
        // ida is KEYED with an empty list, not absent — and rex's two wants are
        // cooked in `charWants`' own order, the variable `Z` surviving cooking.
        let wants = st.compiled_wants_rendered();
        assert_eq!(wants.keys().collect::<Vec<_>>(), ["ida", "rex"]);
        assert_eq!(wants["ida"], Vec::<Vec<String>>::new());
        assert_eq!(
            wants["rex"],
            vec![vec!["Match x".to_owned()], vec!["Match y.Z".to_owned()]]
        );
        // `cookedDesires` is keyed by DESIRE name, the vocabulary's
        // Owner-template cooked once — independent of who holds it.
        let desires = st.compiled_desires_rendered();
        assert_eq!(
            desires.keys().collect::<Vec<_>>(),
            ["grudge-rex", "sweet-tooth"]
        );
        assert_eq!(desires["grudge-rex"], ["Match shamed.rex".to_owned()]);
        assert_eq!(desires["sweet-tooth"], ["Match holding.Owner.cake".to_owned()]);

        // And the tables are what the scorer actually reads: ida scores nothing.
        assert_eq!(st.evaluate_self_wants(&Character::new("ida")), 0);
        // rex: `x`×1 (one binding) + `y.Z`×2 (TWO bindings — the variable survived
        // cooking) + the grudge desire's 7, i.e. charWants in order, each paired
        // with its own utility, plus the held vocabulary desire.
        assert_eq!(st.evaluate_self_wants(&rex), 1 + 2 * 2 + 7);

        // `cookedDesires` is keyed by DESIRE name and `Owner`-templated: nobody in
        // the cast holds `sweet-tooth`, yet the table grounds it for any owner.
        assert_eq!(
            st.evaluate_self_wants(&Character::new("ida").holds("sweet-tooth")),
            5
        );

        // setCharacters RETABLES: rex's want list is replaced wholesale, and the
        // cooked conditions follow. A stale table (old cooked conditions zipped
        // against the new utilities) would score `x`×1 + 7 = 8.
        let rex2 = Character::new("rex")
            .want(Want::new(vec![matches("y.Z")], 1))
            .holds("grudge-rex");
        st.set_characters(vec![Character::new("ida"), rex2.clone()])
            .unwrap();
        assert_eq!(
            st.evaluate_self_wants(&rex2),
            2 + 7,
            "the cooked want table was rebuilt, not reused (a stale table would \
             pair the old `x` conditions with the new utility and score 8)"
        );
    }

    /// A silence the tests above would not otherwise state: `believed_desires`
    /// returns the vocabulary's order, not the belief facts' order.
    #[test]
    fn believed_desires_is_in_vocabulary_order() {
        let mut i = Interner::new();
        let mut db = Db::empty();
        for f in [
            "ida.believes.desires.rex.grudge-rex.seen",
            "ida.believes.desires.rex.sweet-tooth.seen",
        ] {
            db = db.insert_str(&mut i, f).unwrap();
        }
        let names: Vec<String> = believed_desires(&mut i, &db, &vocab(), "ida", "rex")
            .into_iter()
            .map(|d| d.name)
            .collect();
        assert_eq!(
            names,
            vec!["sweet-tooth".to_owned(), "grudge-rex".to_owned()]
        );
    }
}
