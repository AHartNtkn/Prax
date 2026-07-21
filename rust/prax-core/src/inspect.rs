//! Authoring inspector: "why is this action unavailable?" (Versu's runtime
//! inspector — "find out why an action's preconditions have failed").
//!
//! Frozen reference: `src/Prax/Inspect.hs`. An action's conditions are a
//! conjunctive `[Cond]` evaluated left to right. [`first_failing`] finds the
//! first condition whose prefix empties the binding set — the one that ruled the
//! action out (Praxish's `killsPerStep`). [`explain`] surfaces that for every
//! practice instance an action could apply to. Reuses the query evaluator; no
//! engine changes.
//!
//! [`explain`] runs on a CLONE of the interner (like the S9 checker), so
//! inspection is side-effect-free; it reads the compiled tables directly, and the
//! instance query it builds from `CompiledPractice::instance_names` carries NO
//! trailing separator for a zero-role practice (`practice.<pid>`, the v43
//! trailing-operator class), because the cooked instance pattern is a segment
//! list, never a reparsed dotted string.

use crate::db::{Bindings, Db, Val};
use crate::engine::{State, render_text};
use crate::interner::Interner;
use crate::query::{Cond, query};

/// The first condition (in evaluation order) after which the conjunction has no
/// solution from `b0` — the condition that blocks the query — or `None` if the
/// whole conjunction is satisfiable (`Prax.Inspect.firstFailing`).
pub fn first_failing(
    interner: &mut Interner,
    db: &Db,
    conds: &[Cond],
    b0: &Bindings,
) -> Option<Cond> {
    for k in 1..=conds.len() {
        if query(interner, db, &conds[..k], b0).is_empty() {
            return Some(conds[k - 1].clone());
        }
    }
    None
}

/// Render a blocking condition for the `blocked by:` verdict. A `Match`/`Not`
/// resolves its path by NAME (so the reason names the family it gates on — the
/// belief-precondition pin asserts `believes` appears); anything else falls back
/// to its debug shape. The frozen pin asserts by substring, so the exact format
/// is free (design §5) — only the family name must survive.
fn render_cond(interner: &Interner, c: &Cond) -> String {
    let join = |p: &[crate::interner::Sym]| {
        p.iter()
            .map(|s| interner.resolve(*s))
            .collect::<Vec<_>>()
            .join(".")
    };
    match c {
        Cond::Match(p) => join(p),
        Cond::Not(p) => format!("not {}", join(p)),
        other => format!("{other:?}"),
    }
}

/// For `actor`, explain every action whose rendered name contains `needle`: for
/// each practice instance it could apply to, either it is ` — AVAILABLE` or it is
/// ` — blocked by: <cond>` (`Prax.Inspect.explain`).
pub fn explain(st: &State, actor: &str, needle: &str) -> Vec<String> {
    let mut interner = st.interner_snapshot();
    let db = st.base_db();
    let comp = st.compiled_tables();
    let practice_seg = interner.intern("practice");
    let actor_key = interner.intern("Actor");
    let actor_sym = interner.intern(actor);

    let mut out = Vec::new();
    for pid in db.child_keys(&interner, &[practice_seg]) {
        let Some(cp) = comp.practices.get(&pid) else {
            continue;
        };
        let mut seed = Bindings::new();
        seed.insert(actor_key, Val::Sym(actor_sym));
        // The instance pattern is the cooked segment list — no trailing separator
        // for a zero-role practice (`practice.<pid>`).
        let instances = db.unify(&mut interner, &cp.instance_names, seed);
        for inst in instances {
            for a in &cp.actions {
                let label = render_text(&mut interner, &a.name, &inst);
                if label.contains(needle) {
                    let verdict = match first_failing(&mut interner, db, &a.conds, &inst) {
                        None => " — AVAILABLE".to_owned(),
                        Some(c) => format!(" — blocked by: {}", render_cond(&interner, &c)),
                    };
                    out.push(format!("{label}{verdict}"));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::matches;
    use crate::types::{Action, Practice, insert};

    // A self-contained sanity check that `explain` renders AVAILABLE / blocked
    // and that a zero-role practice's instance query carries NO dangling
    // separator. The IntrigueSpec-labelled re-expressions (owed rows 11/12) live
    // in `prax-worlds/src/intrigue.rs` over the shipped intrigue world.
    #[test]
    fn zero_role_practice_instance_has_no_dangling_separator() {
        // A zero-role practice: its instance fact is exactly `practice.shrine`.
        let shrine = Practice::new("shrine")
            .action(Action::new("[Actor]: kneel").then([insert("knelt.Actor")]));
        let mut st = State::new();
        st.define_practices([shrine]).unwrap();
        // Spawn the instance directly (`practice.shrine`, no roles).
        st.perform_outcome(&insert("practice.shrine")).unwrap();
        let out = super::explain(&st, "marcus", "kneel").join("");
        assert!(
            out.contains("AVAILABLE"),
            "a spawned zero-role practice's action must be explainable — the \
             instance query must be `practice.shrine`, not `practice.shrine.`: {out:?}"
        );
    }

    #[test]
    fn explain_reports_a_blocking_condition() {
        let gate = Practice::new("gate").roles(["R"]).action(
            Action::new("[Actor]: pass")
                .when([matches("hasKey.Actor")])
                .then([insert("passed.Actor")]),
        );
        let mut st = State::new();
        st.define_practices([gate]).unwrap();
        st.perform_outcome(&insert("practice.gate.here")).unwrap();
        let out = super::explain(&st, "bob", "pass").join("");
        assert!(
            out.contains("blocked by") && out.contains("hasKey"),
            "the missing key must be named as the blocker: {out:?}"
        );
    }
}
