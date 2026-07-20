//! Minds as objects of belief — the planner's core trio (`Prax.Minds`, the
//! subset the planner imports [D-I3]): the believed-model lookup and the two
//! cooked-want plumbers `evaluateCooked` scores. The string-surfaced diagnostics
//! (`wantFor`/`selfWants`) and the common-knowledge axiom builders
//! (`professed`/`conventional`) are the frozen library surface and live in
//! `prax_vocab::minds`, pinned by `MindsSpec` there.
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
