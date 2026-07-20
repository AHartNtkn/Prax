//! The ONE compile choke point (the `Prax.Engine.retable` + `Prax.Cooked` heir):
//! the runtime [`Effect`] family and every authored→runtime conversion. Strings
//! are the authoring surface; here they stop being the computation surface. The
//! authoring AST ([`crate::types`]) crosses to the interned runtime forms exactly
//! once — [`recompile`] rebuilds the whole [`Compiled`] table wholesale, invoked
//! by every install-time setter ([`crate::engine`]), so the hot loop never
//! re-cooks a practice, a function, an axiom body, or a want.
//!
//! Frozen reference: `src/Prax/Cooked.hs` (`cookOutcome`/`groundCookedOutcome`/
//! `cookPractice`/`cookFunctions`/`cookScheduleRule`) and `Prax.Engine.retable`.
//! The `Cooked*` mirror duality dies with the port: there is ONE representation,
//! so grounding runs once over it (the `groundCooked* == ground*` equivalences
//! die as `implementation` in `KILLED.md`; their content is re-expressed here).

use std::collections::BTreeMap;

use smallvec::SmallVec;

use crate::db::{Bindings, ground_tokens};
use crate::derive::{
    CompiledRule, axiom_footprint, axiom_head_patterns, axiom_neg_patterns, monotone_axioms,
};
use crate::error::WorldError;
use crate::interner::{Interner, Sym};
use crate::path::{CompiledPath, tokenize};
use crate::query::{Cond, Condition, compile_condition, ground_cond, ground_names};
use crate::types::{Axiom, Character, Desire, Function, Outcome, Practice, ScheduleRule};

/// The runtime effect family — the interned mirror of [`Outcome`]. `Insert`/
/// `Delete`/`InsertFor` carry a pre-tokenized [`CompiledPath`] (segments + the
/// `!`/`.` bitmask); `Call`'s fn stays a `String` (the registry key, never
/// unified) while its args are interned. `ForEach`/`Roll` recurse. Built only by
/// [`compile_outcome`]; crate-internal (the runtime type family never surfaces to
/// authors).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Effect {
    Insert(CompiledPath),
    Delete(CompiledPath),
    InsertFor(i64, CompiledPath),
    Call(String, Vec<Sym>),
    ForEach(Vec<Cond>, Vec<Effect>),
    Roll(i64, i64, Vec<Cond>, Vec<Effect>),
}

/// The cooked mirror of [`crate::types::Action`]: conditions and effects
/// precompiled; the name (a display template, never re-parsed) carried unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledAction {
    pub(crate) name: String,
    pub(crate) conds: Vec<Cond>,
    pub(crate) outs: Vec<Effect>,
}

/// The cooked mirror of [`Practice`]: the instance-unification pattern
/// (`practice.<pid>.<Role1>…`) pre-split once, every action precooked, and the
/// spawn-time inits precooked (`Prax.Types.CookedPractice`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledPractice {
    pub(crate) instance_names: Vec<Sym>,
    pub(crate) actions: Vec<CompiledAction>,
    pub(crate) inits: Vec<Effect>,
}

/// The cooked mirror of [`ScheduleRule`]: body clauses precooked, the name
/// carried unchanged (a lookup/persist key). Firing is S5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledScheduleRule {
    pub(crate) name: String,
    pub(crate) body: Vec<(Vec<Cond>, Vec<Effect>)>,
}

/// A compiled function's interned params and cooked guarded cases — the value in
/// the `Call`-resolution registry.
pub(crate) type CompiledFn = (Vec<Sym>, Vec<(Vec<Cond>, Vec<Effect>)>);

/// The whole derived table, rebuilt wholesale by [`recompile`]. Crate-private, so
/// adding S6's tables (improvables/liveness/cares_about) later is a cheap field
/// add — a present-but-empty table would invite an accidental consumer, so S6's
/// tables simply do not exist yet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Compiled {
    pub(crate) practices: BTreeMap<String, CompiledPractice>,
    /// Keyed by `fnName`: (interned params, cooked cases). Uniqueness is the
    /// setter's loud guard, so the map never silently collapses a duplicate.
    pub(crate) fns: BTreeMap<String, CompiledFn>,
    pub(crate) schedule: Vec<CompiledScheduleRule>,
    /// Per character name, each want's conditions precooked (same order as
    /// `charWants`, paired with utilities by construction).
    pub(crate) wants: BTreeMap<String, Vec<Vec<Cond>>>,
    /// Per vocabulary desire name, the (Owner-templated) conditions precooked.
    pub(crate) desires: BTreeMap<String, Vec<Cond>>,
    pub(crate) scope: Vec<Cond>,
    pub(crate) rules: Vec<CompiledRule>,
    pub(crate) footprint: Vec<SmallVec<[Sym; 6]>>,
    /// Every axiom head plus the `contradiction` witness.
    pub(crate) axiom_heads: Vec<SmallVec<[Sym; 6]>>,
    pub(crate) neg_footprint: Vec<SmallVec<[Sym; 6]>>,
    pub(crate) cont_monotone: bool,
}

/// Compile one [`Outcome`] to its runtime [`Effect`] (`Prax.Cooked.cookOutcome`),
/// interning every segment once and rejecting a trailing operator ([`tokenize`]).
pub(crate) fn compile_outcome(interner: &mut Interner, o: &Outcome) -> Result<Effect, WorldError> {
    Ok(match o {
        Outcome::Insert(s) => Effect::Insert(tokenize(interner, s)?),
        Outcome::Delete(s) => Effect::Delete(tokenize(interner, s)?),
        Outcome::InsertFor(n, s) => Effect::InsertFor(*n, tokenize(interner, s)?),
        Outcome::Call(fn_, args) => {
            Effect::Call(fn_.clone(), args.iter().map(|a| interner.intern(a)).collect())
        }
        Outcome::ForEach(conds, outs) => Effect::ForEach(
            compile_conds(interner, conds)?,
            compile_effects(interner, outs)?,
        ),
        Outcome::Roll(num, den, conds, outs) => Effect::Roll(
            *num,
            *den,
            compile_conds(interner, conds)?,
            compile_effects(interner, outs)?,
        ),
    })
}

/// Substitute bindings into a runtime [`Effect`] (`Prax.Cooked.groundCookedOutcome`):
/// `Insert`/`Delete`/`InsertFor` reuse [`ground_tokens`] (no string rebuild),
/// `Call` args substitute like [`ground_names`], `ForEach`/`Roll` recurse through
/// both guard and body.
pub(crate) fn ground_effect(interner: &mut Interner, e: &Effect, b: &Bindings) -> Effect {
    match e {
        Effect::Insert(p) => Effect::Insert(ground_tokens(interner, p, b)),
        Effect::Delete(p) => Effect::Delete(ground_tokens(interner, p, b)),
        Effect::InsertFor(n, p) => Effect::InsertFor(*n, ground_tokens(interner, p, b)),
        Effect::Call(fn_, args) => Effect::Call(fn_.clone(), ground_names(interner, b, args)),
        Effect::ForEach(conds, effs) => Effect::ForEach(
            conds.iter().map(|c| ground_cond(interner, b, c)).collect(),
            effs.iter().map(|e| ground_effect(interner, e, b)).collect(),
        ),
        Effect::Roll(num, den, conds, effs) => Effect::Roll(
            *num,
            *den,
            conds.iter().map(|c| ground_cond(interner, b, c)).collect(),
            effs.iter().map(|e| ground_effect(interner, e, b)).collect(),
        ),
    }
}

fn compile_conds(interner: &mut Interner, conds: &[Condition]) -> Result<Vec<Cond>, WorldError> {
    conds.iter().map(|c| compile_condition(interner, c)).collect()
}

fn compile_effects(interner: &mut Interner, outs: &[Outcome]) -> Result<Vec<Effect>, WorldError> {
    outs.iter().map(|o| compile_outcome(interner, o)).collect()
}

/// Compile a [`Practice`] to its cooked form (`Prax.Cooked.cookPractice`): the
/// instance pattern `practice.<pid>.<Role1>…` built as a segment list directly
/// (never a dotted string reparsed — a zero-role practice would leave a trailing
/// separator), every action precooked, and the inits precooked.
fn cook_practice(interner: &mut Interner, p: &Practice) -> Result<CompiledPractice, WorldError> {
    let mut instance_names = Vec::with_capacity(2 + p.roles.len());
    instance_names.push(interner.intern("practice"));
    instance_names.push(interner.intern(&p.id));
    for r in &p.roles {
        instance_names.push(interner.intern(r));
    }
    let actions = p
        .actions
        .iter()
        .map(|a| {
            Ok(CompiledAction {
                name: a.name.clone(),
                conds: compile_conds(interner, &a.when)?,
                outs: compile_effects(interner, &a.then)?,
            })
        })
        .collect::<Result<_, WorldError>>()?;
    let inits = compile_effects(interner, &p.init_outcomes)?;
    Ok(CompiledPractice {
        instance_names,
        actions,
        inits,
    })
}

/// Rebuild the whole derived table from the authored sources (`Prax.Engine.retable`).
/// `rules` is S3's compile over the axiom list AS GIVEN — deontics-free: whatever
/// □-lifted rules a deontic world declared are already in `axioms`, so recompile
/// decides no lift. The axiom-derived tables then read that `rules`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn recompile(
    interner: &mut Interner,
    practices: &BTreeMap<String, Practice>,
    fns: &[Function],
    axioms: &[Axiom],
    characters: &[Character],
    desires: &[Desire],
    schedule: &[ScheduleRule],
    prediction_scope: &[Condition],
) -> Result<Compiled, WorldError> {
    // The axioms compiled (deontics-free) — everything else reads these.
    let rules: Vec<CompiledRule> = axioms
        .iter()
        .map(|ax| {
            let heads: Vec<&str> = ax.then.iter().map(String::as_str).collect();
            CompiledRule::compile(interner, &ax.when, &heads)
        })
        .collect::<Result<_, WorldError>>()?;

    let mut practices_c = BTreeMap::new();
    for (id, p) in practices {
        practices_c.insert(id.clone(), cook_practice(interner, p)?);
    }

    let mut fns_c = BTreeMap::new();
    for f in fns {
        let params: Vec<Sym> = f.params.iter().map(|p| interner.intern(p)).collect();
        let cases = f
            .cases
            .iter()
            .map(|c| {
                Ok((
                    compile_conds(interner, &c.conditions)?,
                    compile_effects(interner, &c.outcomes)?,
                ))
            })
            .collect::<Result<Vec<_>, WorldError>>()?;
        fns_c.insert(f.name.clone(), (params, cases));
    }

    let mut schedule_c = Vec::with_capacity(schedule.len());
    for r in schedule {
        let body = r
            .body
            .iter()
            .map(|(conds, outs)| {
                Ok((
                    compile_conds(interner, conds)?,
                    compile_effects(interner, outs)?,
                ))
            })
            .collect::<Result<Vec<_>, WorldError>>()?;
        schedule_c.push(CompiledScheduleRule {
            name: r.name.clone(),
            body,
        });
    }

    let mut wants = BTreeMap::new();
    for c in characters {
        let ws = c
            .wants
            .iter()
            .map(|w| compile_conds(interner, &w.when))
            .collect::<Result<Vec<_>, WorldError>>()?;
        wants.insert(c.name.clone(), ws);
    }

    let mut desires_c = BTreeMap::new();
    for d in desires {
        desires_c.insert(d.name.clone(), compile_conds(interner, &d.want.when)?);
    }

    let scope = compile_conds(interner, prediction_scope)?;

    let footprint = axiom_footprint(&rules);
    let mut axiom_heads = axiom_head_patterns(&rules);
    axiom_heads.push(smallvec::smallvec![interner.intern("contradiction")]);
    let neg_footprint = axiom_neg_patterns(&rules);
    let cont_monotone = monotone_axioms(interner, &rules);

    Ok(Compiled {
        practices: practices_c,
        fns: fns_c,
        schedule: schedule_c,
        wants,
        desires: desires_c,
        scope,
        rules,
        footprint,
        axiom_heads,
        neg_footprint,
        cont_monotone,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Val;
    use crate::query::CalcOp;

    // The CookedSpec grounding pin (owed:S4 discharge): with the raw/cooked
    // duality dead, `groundCookedOutcome == groundOutcome` has no second
    // representation to agree with — grounding runs once over the one Effect
    // family. This pins that grounding over EVERY Effect constructor: grounding a
    // compiled outcome equals compiling the already-grounded outcome.

    // H: CookedSpec.hs "groundCookedOutcome matches groundOutcome for every remaining construct"
    #[test]
    fn ground_effect_grounds_every_construct() {
        let mut i = Interner::new();
        let mut b = Bindings::new();
        b.insert(i.intern("W"), Val::Sym(i.intern("bex")));
        let cases: &[(Outcome, Outcome)] = &[
            (
                Outcome::Insert("mark.W!seen".into()),
                Outcome::Insert("mark.bex!seen".into()),
            ),
            (Outcome::Delete("gone.W".into()), Outcome::Delete("gone.bex".into())),
            (
                Outcome::InsertFor(3, "temp.W".into()),
                Outcome::InsertFor(3, "temp.bex".into()),
            ),
            (
                Outcome::Call("fn".into(), vec!["W".into(), "k".into()]),
                Outcome::Call("fn".into(), vec!["bex".into(), "k".into()]),
            ),
            (
                Outcome::ForEach(
                    vec![Condition::Match("g.W".into())],
                    vec![Outcome::Insert("h.W".into())],
                ),
                Outcome::ForEach(
                    vec![Condition::Match("g.bex".into())],
                    vec![Outcome::Insert("h.bex".into())],
                ),
            ),
            (
                Outcome::Roll(
                    1,
                    2,
                    vec![Condition::Match("g.W".into())],
                    vec![Outcome::Insert("h.W".into())],
                ),
                Outcome::Roll(
                    1,
                    2,
                    vec![Condition::Match("g.bex".into())],
                    vec![Outcome::Insert("h.bex".into())],
                ),
            ),
        ];
        for (input, want) in cases {
            let compiled = compile_outcome(&mut i, input).unwrap();
            let got = ground_effect(&mut i, &compiled, &b);
            let expected = compile_outcome(&mut i, want).unwrap();
            assert_eq!(got, expected, "grounding {input:?}");
        }
    }

    #[test]
    fn compile_outcome_calc_and_paths() {
        // A sanity check that compile_outcome interns/tokenizes each constructor.
        let mut i = Interner::new();
        let o = Outcome::ForEach(
            vec![Condition::Calc("R".into(), CalcOp::Mul, "N".into(), "2".into())],
            vec![Outcome::Insert("n!R".into())],
        );
        let e = compile_outcome(&mut i, &o).unwrap();
        assert!(matches!(e, Effect::ForEach(_, _)));
    }
}
