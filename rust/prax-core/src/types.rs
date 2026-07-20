//! The two type families and the boundary between them: the string-surfaced
//! authoring AST (`Condition`/`Outcome`/`Practice`/…, what the builder DSL
//! produces) and the interned runtime types (`Cond`/`Effect`/`CompiledPath`).
//! The conversion is the single compile choke point ([`crate::compilepipe`]);
//! there is no runtime mirror duality to keep in sync.
//!
//! Frozen reference: `src/Prax/Types.hs` (the authoring family, the v40 hygiene
//! walkers, `isPraxVar`) and `src/Prax/Coerce.hs`'s `namespaceKernel` (the
//! rename primitive [D-panel I5]). The `Cooked*` mirror family, `PraxState`, and
//! the derived analysis tables that Types.hs also declares live at their real
//! Rust homes ([`crate::compilepipe`], [`crate::engine`]) — this module is the
//! authoring surface only.
//!
//! **Builder idiom (ARCHITECTURE, D-panel I1).** Scalar string arguments take
//! `impl Into<String>`. The `Condition`/`Outcome` collection seams are
//! `Vec<Condition>`/`Vec<Outcome>` on free constructors, positional params, and
//! stored fields (`vec![]` infers the empty case; literal parity with the frozen
//! source). Single-level `impl IntoIterator<Item = Condition>` is used ONLY on
//! the omittable fluent setters ([`Action::when`]/[`Action::then`],
//! [`FnCase`]/[`ScheduleRule`] clauses), where the empty case is solved by
//! omitting the call — the double-generic `Item = impl Into<_>` form is BANNED
//! (an empty `[]` fails inference; the single-level form is compile-checked by
//! this module's own tests). Builders are INFALLIBLE — they build values; the
//! loud construction guards live at install ([`crate::engine`]).

use std::collections::BTreeMap;

use crate::error::WorldError;
use crate::path::segment_names_checked;
use crate::query::{Condition, cond_sents, flat_condition_vars};

// ---- the outcome language -------------------------------------------------

/// An effect on the world (`Prax.Types.Outcome`). Postconditions never need an
/// explicit remove for single-valued slots — the `!` exclusion in [`Outcome::Insert`]
/// handles that.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Assert a sentence (may spawn a practice — see the engine).
    Insert(String),
    /// Retract a subtree.
    Delete(String),
    /// Assert a sentence that EXPIRES `n` round boundaries after this insert
    /// (v44). Re-inserting the exact path with a lifetime refreshes the due;
    /// re-inserting it bare cancels it (the supersession law).
    InsertFor(i64, String),
    /// Invoke a registered [`Function`] by name with args.
    Call(String, Vec<String>),
    /// Quantified effect: for EVERY binding of the guard (evaluated against the
    /// closed view, snapshot at entry), apply the sub-outcomes.
    ForEach(Vec<Condition>, Vec<Outcome>),
    /// The drama die (v50): advance the engine RNG stream UNCONDITIONALLY, then
    /// roll on the advanced value — on a hit apply the body exactly as a
    /// [`Outcome::ForEach`]. Built only by [`crate::rng::draw`] on the guarded
    /// path; the variant stays public so the unseeded-`Roll` pin can construct one.
    Roll(i64, i64, Vec<Condition>, Vec<Outcome>),
}

/// Free outcome constructors joining S2's condition surface — the authoring
/// verbs a world writes (`insert(s)`, `delete(s)`, …). `Roll` has no free
/// constructor: it is built only by [`crate::rng::draw`] on the guarded path.
pub fn insert(s: impl Into<String>) -> Outcome {
    Outcome::Insert(s.into())
}
pub fn delete(s: impl Into<String>) -> Outcome {
    Outcome::Delete(s.into())
}
pub fn insert_for(n: i64, s: impl Into<String>) -> Outcome {
    Outcome::InsertFor(n, s.into())
}
pub fn call(f: impl Into<String>, args: Vec<String>) -> Outcome {
    Outcome::Call(f.into(), args)
}
pub fn for_each(when: Vec<Condition>, then: Vec<Outcome>) -> Outcome {
    Outcome::ForEach(when, then)
}

// ---- practices / actions --------------------------------------------------

/// A social practice: a role-parameterized bundle of affordances
/// (`Prax.Types.Practice`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Practice {
    /// Unique id; the DB key under `practice.`.
    pub id: String,
    /// Display template (may contain `[Role]`s).
    pub name: String,
    /// Role variables; the instance key.
    pub roles: Vec<String>,
    /// Affordances offered to participants.
    pub actions: Vec<Action>,
    /// Static facts, inserted under `practiceData.<id>.`.
    pub data_facts: Vec<String>,
    /// Run once when an instance first spawns.
    pub init_outcomes: Vec<Outcome>,
}

impl Practice {
    /// A practice with everything empty but its id.
    pub fn new(id: impl Into<String>) -> Practice {
        Practice {
            id: id.into(),
            name: String::new(),
            roles: Vec::new(),
            actions: Vec::new(),
            data_facts: Vec::new(),
            init_outcomes: Vec::new(),
        }
    }
    /// Set the display template.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Practice {
        self.name = name.into();
        self
    }
    /// Set the role variables (the instance key).
    #[must_use]
    pub fn roles<S: Into<String>>(mut self, roles: impl IntoIterator<Item = S>) -> Practice {
        self.roles = roles.into_iter().map(Into::into).collect();
        self
    }
    /// Append one affordance.
    #[must_use]
    pub fn action(mut self, action: Action) -> Practice {
        self.actions.push(action);
        self
    }
    /// Set the static data facts.
    #[must_use]
    pub fn data_facts<S: Into<String>>(mut self, facts: impl IntoIterator<Item = S>) -> Practice {
        self.data_facts = facts.into_iter().map(Into::into).collect();
        self
    }
    /// Set the spawn-time init outcomes.
    #[must_use]
    pub fn init(mut self, outcomes: impl IntoIterator<Item = Outcome>) -> Practice {
        self.init_outcomes = outcomes.into_iter().collect();
        self
    }
}

/// An affordance: a named, conditioned bundle of outcomes (`Prax.Types.Action`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    /// Display template; also the action's id (a lookup key).
    pub name: String,
    /// Preconditions (a conjunctive query).
    pub when: Vec<Condition>,
    /// Effects applied when performed.
    pub then: Vec<Outcome>,
}

impl Action {
    /// A conditionless, effectless action with this label.
    pub fn new(label: impl Into<String>) -> Action {
        Action {
            name: label.into(),
            when: Vec::new(),
            then: Vec::new(),
        }
    }
    /// Set the preconditions. Omit for an unconditional action (the empty case,
    /// D-panel I1).
    #[must_use]
    pub fn when(mut self, conds: impl IntoIterator<Item = Condition>) -> Action {
        self.when = conds.into_iter().collect();
        self
    }
    /// Set the effects. Omit for an effectless action.
    #[must_use]
    pub fn then(mut self, outs: impl IntoIterator<Item = Outcome>) -> Action {
        self.then = outs.into_iter().collect();
        self
    }
}

// ---- functions ------------------------------------------------------------

/// A named function: guarded conditional effects (`Prax.Types.Function`). The
/// first case whose conditions hold runs; the rest are skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub cases: Vec<FnCase>,
}

/// One guarded case of a [`Function`] (`Prax.Types.FnCase`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnCase {
    pub conditions: Vec<Condition>,
    pub outcomes: Vec<Outcome>,
}

impl Function {
    /// A function with a name and parameter list, no cases yet.
    pub fn new<S: Into<String>>(name: impl Into<String>, params: impl IntoIterator<Item = S>) -> Function {
        Function {
            name: name.into(),
            params: params.into_iter().map(Into::into).collect(),
            cases: Vec::new(),
        }
    }
    /// Append one guarded case.
    #[must_use]
    pub fn case(
        mut self,
        when: impl IntoIterator<Item = Condition>,
        then: impl IntoIterator<Item = Outcome>,
    ) -> Function {
        self.cases.push(FnCase {
            conditions: when.into_iter().collect(),
            outcomes: then.into_iter().collect(),
        });
        self
    }
}

// ---- schedule rules -------------------------------------------------------

/// One recurring engine-schedule rule (v44): every `period` round boundaries,
/// ground each clause's conditions against the world and apply its outcomes per
/// binding (`Prax.Types.ScheduleRule`). Firing itself is S5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleRule {
    /// Single segment; the persist re-association key.
    pub name: String,
    /// Round boundaries between firings.
    pub period: i64,
    pub body: Vec<(Vec<Condition>, Vec<Outcome>)>,
}

impl ScheduleRule {
    /// A schedule rule with a name and period, no clauses yet.
    pub fn new(name: impl Into<String>, period: i64) -> ScheduleRule {
        ScheduleRule {
            name: name.into(),
            period,
            body: Vec::new(),
        }
    }
    /// Append one clause `(when, then)`.
    #[must_use]
    pub fn clause(
        mut self,
        when: impl IntoIterator<Item = Condition>,
        then: impl IntoIterator<Item = Outcome>,
    ) -> ScheduleRule {
        self.body
            .push((when.into_iter().collect(), then.into_iter().collect()));
        self
    }
}

// ---- axioms / wants / desires / characters --------------------------------

/// An implication rule `when → then` — the authoring twin of S3's
/// [`crate::derive::CompiledRule`] (`Prax.Derive.Axiom`). Heads are sentence
/// templates that keep their `!`/`.` labels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Axiom {
    pub when: Vec<Condition>,
    pub then: Vec<String>,
}

impl Axiom {
    /// `axiom body heads`.
    pub fn new<S: Into<String>>(when: Vec<Condition>, then: impl IntoIterator<Item = S>) -> Axiom {
        Axiom {
            when,
            then: then.into_iter().map(Into::into).collect(),
        }
    }
}

/// A desire query whose every satisfying instantiation adds `utility` to a
/// candidate future world (`Prax.Types.Want`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Want {
    pub when: Vec<Condition>,
    pub utility: i32,
}

impl Want {
    pub fn new(when: Vec<Condition>, utility: i32) -> Want {
        Want { when, utility }
    }
}

/// A nameable desire: a [`Want`] whose conditions may use the reserved variable
/// `Owner`, instantiated per character (`Prax.Types.Desire`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Desire {
    pub name: String,
    pub want: Want,
}

impl Desire {
    pub fn new(name: impl Into<String>, want: Want) -> Desire {
        Desire {
            name: name.into(),
            want,
        }
    }
}

/// Death (and eviction) are represented by the fact `dead.<name>`
/// (`Prax.Types.deadSentence`). A dead character stays in the cast list but is
/// skipped in turn-taking and lookahead. Ported as its own helper rather than
/// inlined at each site (§3: port each module's own path helper, never replace
/// it) — a call site that spells the path out no longer moves when the helper
/// does.
pub fn dead_sentence(name: &str) -> String {
    format!("dead.{name}")
}

/// A character/agent (`Prax.Types.Character`). Wants drive autonomous choice; a
/// practice-bound character only acts within its bound practice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Character {
    pub name: String,
    pub wants: Vec<Want>,
    /// Names of vocabulary [`Desire`]s this character holds.
    pub desires: Vec<String>,
    /// Restrict actions to this practice id.
    pub bound_to: Option<String>,
}

impl Character {
    /// A character with no wants, no desires, and no binding.
    pub fn new(name: impl Into<String>) -> Character {
        Character {
            name: name.into(),
            wants: Vec::new(),
            desires: Vec::new(),
            bound_to: None,
        }
    }
    /// Append one want.
    #[must_use]
    pub fn want(mut self, want: Want) -> Character {
        self.wants.push(want);
        self
    }
    /// Hold a named vocabulary desire.
    #[must_use]
    pub fn holds(mut self, desire: impl Into<String>) -> Character {
        self.desires.push(desire.into());
        self
    }
    /// Bind the character to a practice id.
    #[must_use]
    pub fn bound_to(mut self, pid: impl Into<String>) -> Character {
        self.bound_to = Some(pid.into());
        self
    }
}

// ---- the rename primitive [D-panel I5] ------------------------------------

/// Op-preservingly alpha-rename authored variables through every condition
/// constructor, applying a name→name substitution `subst` (a name absent from
/// the map maps to itself) — the heir of `Prax.Coerce.namespaceKernel`'s
/// `renameCond`. `Match`/`Not` sentences are split on `.`/`!`, each segment name
/// substituted, and rejoined, so a segment's following operator is preserved
/// (never a naive string replace that could corrupt punctuation); a
/// [`Condition::Subquery`]'s set/find binders and their interior uses move
/// together. A general authoring-surface operation S7's Coerce builds on;
/// pinned in full by CoerceSpec's rename-law scenarios when S7 lands.
pub fn rename_vars(subst: &BTreeMap<String, String>, conds: &[Condition]) -> Vec<Condition> {
    conds.iter().map(|c| rename_cond(subst, c)).collect()
}

/// The outcome twin of [`rename_vars`]: rename through every [`Outcome`]
/// constructor. `Insert`/`Delete`/`InsertFor` sentences and `Call` args are
/// substituted; `ForEach`/`Roll` recurse through both guard and body.
pub fn rename_outcomes(subst: &BTreeMap<String, String>, outs: &[Outcome]) -> Vec<Outcome> {
    outs.iter().map(|o| rename_outcome(subst, o)).collect()
}

fn sub<'a>(subst: &'a BTreeMap<String, String>, name: &'a str) -> String {
    subst.get(name).cloned().unwrap_or_else(|| name.to_owned())
}

/// Rename each `.`/`!`-separated segment name of a sentence, preserving the
/// operators (the string-side heir of `renameSentence`'s tokens round-trip; no
/// interner needed at the authoring boundary).
fn rename_sentence(subst: &BTreeMap<String, String>, s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    let mut seg = String::new();
    for ch in trimmed.chars() {
        if ch == '.' || ch == '!' {
            out.push_str(&sub(subst, &seg));
            out.push(ch);
            seg.clear();
        } else {
            seg.push(ch);
        }
    }
    out.push_str(&sub(subst, &seg));
    out
}

fn rename_cond(subst: &BTreeMap<String, String>, c: &Condition) -> Condition {
    match c {
        Condition::Match(s) => Condition::Match(rename_sentence(subst, s)),
        Condition::Not(s) => Condition::Not(rename_sentence(subst, s)),
        Condition::Eq(x, y) => Condition::Eq(sub(subst, x), sub(subst, y)),
        Condition::Neq(x, y) => Condition::Neq(sub(subst, x), sub(subst, y)),
        Condition::Cmp(op, x, y) => Condition::Cmp(*op, sub(subst, x), sub(subst, y)),
        Condition::Calc(r, op, x, y) => {
            Condition::Calc(sub(subst, r), *op, sub(subst, x), sub(subst, y))
        }
        Condition::Count(r, s) => Condition::Count(sub(subst, r), sub(subst, s)),
        Condition::Subquery { set, find, where_ } => Condition::Subquery {
            set: sub(subst, set),
            find: find.iter().map(|f| sub(subst, f)).collect(),
            where_: rename_vars(subst, where_),
        },
        Condition::Or(clauses) => {
            Condition::Or(clauses.iter().map(|cl| rename_vars(subst, cl)).collect())
        }
        Condition::Absent(cs) => Condition::Absent(rename_vars(subst, cs)),
        Condition::Exists(cs) => Condition::Exists(rename_vars(subst, cs)),
    }
}

fn rename_outcome(subst: &BTreeMap<String, String>, o: &Outcome) -> Outcome {
    match o {
        Outcome::Insert(s) => Outcome::Insert(rename_sentence(subst, s)),
        Outcome::Delete(s) => Outcome::Delete(rename_sentence(subst, s)),
        Outcome::InsertFor(n, s) => Outcome::InsertFor(*n, rename_sentence(subst, s)),
        Outcome::Call(f, args) => {
            Outcome::Call(f.clone(), args.iter().map(|a| sub(subst, a)).collect())
        }
        Outcome::ForEach(cs, os) => {
            Outcome::ForEach(rename_vars(subst, cs), rename_outcomes(subst, os))
        }
        Outcome::Roll(n, d, cs, os) => {
            Outcome::Roll(*n, *d, rename_vars(subst, cs), rename_outcomes(subst, os))
        }
    }
}

// ---- v40 hygiene walkers --------------------------------------------------

/// Every name an outcome MENTIONS — a total walk over every constructor,
/// `ForEach`/`Roll` recursing through both guard conditions ([`crate::query::condition_vars`])
/// and nested outcomes (`Prax.Types.outcomeVars`). The shared home for
/// reserved-variable guards ([`crate::engine`]'s `set_schedule`,
/// [`crate::rng::draw`]).
///
/// Fallible for the same reason [`crate::query::condition_vars`] is: `Insert`/`Delete`/
/// `InsertFor` go through the frozen `Prax.Db.pathNames`, which RAISES on a
/// trailing operator.
///
/// # Errors
/// [`WorldError::TrailingOperator`] if a written sentence ends in `.`/`!`.
pub fn outcome_vars(o: &Outcome) -> Result<Vec<String>, WorldError> {
    match o {
        Outcome::Insert(s) | Outcome::Delete(s) | Outcome::InsertFor(_, s) => {
            segment_names_checked(s)
        }
        Outcome::Call(_, args) => Ok(args.clone()),
        Outcome::ForEach(cs, os) | Outcome::Roll(_, _, cs, os) => {
            let mut out = flat_condition_vars(cs)?;
            for o in os {
                out.extend(outcome_vars(o)?);
            }
            Ok(out)
        }
    }
}

/// Every SENTENCE STRING an outcome list mentions — the raw authored paths it
/// inserts, deletes, or reads (a `ForEach`/`Roll` guard's own conditions via
/// [`cond_sents`], its body recursively) (`Prax.Types.outcomeSents`).
pub fn outcome_sents(outs: &[Outcome]) -> Vec<String> {
    outs.iter()
        .flat_map(|o| match o {
            Outcome::Insert(s) | Outcome::Delete(s) | Outcome::InsertFor(_, s) => vec![s.clone()],
            Outcome::Call(_, _) => Vec::new(),
            Outcome::ForEach(cs, os) | Outcome::Roll(_, _, cs, os) => {
                let mut v = cond_sents(cs);
                v.extend(outcome_sents(os));
                v
            }
        })
        .collect()
}

/// Is a name in the reserved `Prax` namespace (`Prax.Types.isPraxVar`): `Prax`
/// followed by at least one more character. All machinery variables live there,
/// so authors can never collide with them by accident.
pub fn is_prax_var(s: &str) -> bool {
    s.len() >= 5 && s.starts_with("Prax")
}

/// Names an author-supplied fragment MUST NOT use when a combinator splices it
/// into generated conditions/outcomes (the v40 hygiene boundary,
/// `Prax.Types.authoredVarClash`). Two sources of capture, one check: the `Prax`
/// namespace ([`is_prax_var`]) OR `forbidden`, names the combinator itself also
/// binds in the SAME splice. `forbidden` is a BLOCKLIST, not an allowlist: a
/// name neither Prax-prefixed nor listed is unrestricted (`[]` is "nothing extra
/// is forbidden"). Returns the offenders; each caller raises its own contextual
/// error.
///
/// # Errors
/// The trailing-operator rejection [`crate::query::condition_vars`]/[`outcome_vars`] carry
/// from the frozen `pathNames`: a malformed authored sentence dies here, before
/// any hygiene verdict is reached, exactly as the frozen `authoredVarClash`
/// does.
pub fn authored_var_clash(
    forbidden: &[String],
    conds: &[Condition],
    outs: &[Outcome],
) -> Result<Vec<String>, WorldError> {
    let mut names = flat_condition_vars(conds)?;
    for o in outs {
        names.extend(outcome_vars(o)?);
    }
    Ok(names
        .into_iter()
        .filter(|v| crate::interner::is_variable_name(v))
        .filter(|v| is_prax_var(v) || forbidden.iter().any(|f| f == v))
        .collect())
}

/// [`authored_var_clash`] for string-pattern arguments that are not
/// [`Condition`]s: the same two-source, `forbidden`-is-a-blocklist check over
/// already-split names (`Prax.Types.authoredPatClash`). Callers extract the
/// names themselves (and drop any they mean to exempt) before calling.
pub fn authored_pat_clash(forbidden: &[String], names: &[String]) -> Vec<String> {
    names
        .iter()
        .filter(|v| crate::interner::is_variable_name(v))
        .filter(|v| is_prax_var(v) || forbidden.iter().any(|f| f == *v))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::CalcOp;

    fn m(s: &str) -> Condition {
        Condition::Match(s.to_owned())
    }
    fn n(s: &str) -> Condition {
        Condition::Not(s.to_owned())
    }

    // ===== builder compile-check (D-panel I1) =====
    //
    // The single-level `impl IntoIterator<Item = Condition/Outcome>` fluent
    // setter form: a non-empty array literal infers, and the empty case is
    // reached by OMITTING the setter (never by passing `[]`, which would fail
    // inference under the banned double-generic form). Recorded green here.
    #[test]
    fn builder_fluent_setters_take_array_literals_and_omit_for_empty() {
        // Non-empty array literals into .when/.then: Item = Condition/Outcome
        // is inferred without annotation.
        let a = Action::new("[Actor]: Order [Beverage]")
            .when([m("practice.tendBar.B.customer.Actor"), n("x!y")])
            .then([insert("practice.tendBar.B.customer.Actor!order")]);
        assert_eq!(a.when.len(), 2);
        assert_eq!(a.then.len(), 1);

        // The empty conditions case: omit `.when` — no `[]` inference problem.
        let unconditional = Action::new("[Actor]: idle").then([insert("noted")]);
        assert!(unconditional.when.is_empty());

        // A fully empty action: omit both.
        let bare = Action::new("bare");
        assert!(bare.when.is_empty() && bare.then.is_empty());

        // Practice/Function/ScheduleRule/Character builders chain likewise.
        let p = Practice::new("tendBar")
            .name("[Bartender] is tending bar")
            .roles(["Bartender"])
            .data_facts(["beverageType.beer!alcoholic"])
            .action(a)
            .init([insert("practice.tendBar.B.spawned")]);
        assert_eq!(p.id, "tendBar");
        assert_eq!(p.roles, ["Bartender"]);
        assert_eq!(p.actions.len(), 1);

        let f = Function::new("dbl", ["M", "N"])
            .case([Condition::Calc("R".into(), CalcOp::Mul, "N".into(), "2".into())],
                  [insert("practice.math.M.n!R")]);
        assert_eq!(f.cases.len(), 1);

        let sr = ScheduleRule::new("market", 3).clause([m("turn!Now")], [insert("marketDay")]);
        assert_eq!(sr.body.len(), 1);

        let c = Character::new("gil")
            .want(Want::new(vec![m("did.duty")], 20))
            .holds("dutiful")
            .bound_to("conduct");
        assert_eq!(c.wants.len(), 1);
        assert_eq!(c.desires, ["dutiful"]);
        assert_eq!(c.bound_to.as_deref(), Some("conduct"));
    }

    // ===== rename primitive mechanics (S4 unit pin for D-panel I5) =====

    fn subst(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    #[test]
    fn rename_vars_preserves_operators_through_every_constructor() {
        // Victim W -> PraxD, other free var V -> PraxW, mirroring namespaceKernel.
        let s = subst(&[("W", "PraxD"), ("V", "PraxW")]);
        // Match/Not sentences: segment names renamed, `!`/`.` preserved.
        assert_eq!(
            rename_vars(&s, &[m("at.W!place.V")]),
            [m("at.PraxD!place.PraxW")]
        );
        assert_eq!(rename_vars(&s, &[n("dead.W")]), [n("dead.PraxD")]);
        // Eq/Neq/Cmp/Calc/Count operands.
        assert_eq!(rename_vars(&s, &[Condition::Eq("W".into(), "V".into())]),
                   [Condition::Eq("PraxD".into(), "PraxW".into())]);
        assert_eq!(
            rename_vars(&s, &[Condition::Calc("W".into(), CalcOp::Add, "V".into(), "1".into())]),
            [Condition::Calc("PraxD".into(), CalcOp::Add, "PraxW".into(), "1".into())]
        );
        // Subquery: set/find binders and interior uses move together.
        assert_eq!(
            rename_vars(&s, &[Condition::Subquery {
                set: "W".into(),
                find: vec!["V".into()],
                where_: vec![m("p.V.W")],
            }]),
            [Condition::Subquery {
                set: "PraxD".into(),
                find: vec!["PraxW".into()],
                where_: vec![m("p.PraxW.PraxD")],
            }]
        );
        // Or / Absent / Exists recurse.
        assert_eq!(
            rename_vars(&s, &[Condition::Or(vec![vec![m("a.W")], vec![n("b.V")]])]),
            [Condition::Or(vec![vec![m("a.PraxD")], vec![n("b.PraxW")]])]
        );
        // A name absent from the map is left untouched (constants, Owner, …).
        assert_eq!(rename_vars(&s, &[m("at.Owner.bar")]), [m("at.Owner.bar")]);
    }

    #[test]
    fn rename_outcomes_mirror_covers_every_constructor() {
        let s = subst(&[("W", "PraxD"), ("V", "PraxW")]);
        assert_eq!(rename_outcomes(&s, &[insert("mark.W!seen.V")]),
                   [insert("mark.PraxD!seen.PraxW")]);
        assert_eq!(rename_outcomes(&s, &[delete("gone.W")]), [delete("gone.PraxD")]);
        assert_eq!(rename_outcomes(&s, &[insert_for(3, "temp.W")]),
                   [insert_for(3, "temp.PraxD")]);
        assert_eq!(rename_outcomes(&s, &[call("fn", vec!["W".into(), "k".into()])]),
                   [call("fn", vec!["PraxD".into(), "k".into()])]);
        assert_eq!(
            rename_outcomes(&s, &[for_each(vec![m("g.W")], vec![insert("h.V")])]),
            [for_each(vec![m("g.PraxD")], vec![insert("h.PraxW")])]
        );
    }

    // ===== v40 walkers and isPraxVar =====

    #[test]
    fn outcome_vars_walks_every_constructor() {
        let vars = |o: &Outcome| outcome_vars(o).expect("well-formed sentences");
        assert_eq!(vars(&insert("at.Who!place")), ["at", "Who", "place"]);
        assert_eq!(vars(&call("fn", vec!["A".into(), "b".into()])), ["A", "b"]);
        assert_eq!(
            vars(&for_each(vec![m("g.W")], vec![insert("h.W.X")])),
            ["g", "W", "h", "W", "X"]
        );
    }

    #[test]
    fn outcome_vars_rejects_a_trailing_operator_where_the_frozen_pathnames_raises() {
        // Frozen: `outcomeVars (Insert "a.b.")` is `pathNames "a.b."`, which
        // raises `Prax.Db.tokens: trailing operator '.' in "a.b."` (verified on
        // the frozen engine). The unchecked split returns `["a", "b", ""]`.
        for o in [insert("a.b."), delete("a.b."), insert_for(2, "a.b.")] {
            assert_eq!(
                outcome_vars(&o),
                Err(WorldError::TrailingOperator {
                    sentence: "a.b.".to_owned(),
                    op: '.',
                }),
                "{o:?} must be rejected where the frozen pathNames raises"
            );
        }
        assert!(
            outcome_vars(&for_each(vec![m("g.W")], vec![insert("h!")])).is_err(),
            "a ForEach body is walked too"
        );
        assert!(
            outcome_vars(&for_each(vec![m("g.")], vec![insert("h")])).is_err(),
            "a ForEach guard is walked through conditionVars"
        );
        // `Call` carries argument NAMES, not sentences — the frozen `Call _ as -> as`
        // does not tokenize them, so nothing is rejected here.
        assert_eq!(
            outcome_vars(&call("fn", vec!["a.b.".into()])).expect("Call args are not paths"),
            ["a.b."]
        );
    }

    #[test]
    fn outcome_sents_collects_raw_paths_recursively() {
        let outs = vec![
            insert("at.Who"),
            call("fn", vec!["A".into()]), // contributes no sentence
            for_each(vec![m("g.W")], vec![delete("h.W")]),
        ];
        assert_eq!(outcome_sents(&outs), ["at.Who", "g.W", "h.W"]);
    }

    #[test]
    fn is_prax_var_is_prax_plus_at_least_one_char() {
        assert!(is_prax_var("PraxD"));
        assert!(is_prax_var("PraxW2"));
        assert!(!is_prax_var("Prax"));
        assert!(!is_prax_var("Actor"));
        assert!(!is_prax_var("S"));
    }

    #[test]
    fn authored_var_clash_flags_prax_and_forbidden_only() {
        let forbidden = vec!["Actor".to_owned()];
        let clash = |f: &[String], c: &[Condition], o: &[Outcome]| {
            authored_var_clash(f, c, o).expect("well-formed sentences")
        };
        // Prax-namespaced var in conditions.
        assert_eq!(clash(&[], &[m("flag.PraxD")], &[]), ["PraxD"]);
        // Prax-namespaced var in outcomes.
        assert_eq!(clash(&[], &[], &[insert("marked.PraxW")]), ["PraxW"]);
        // Nested ForEach guard.
        assert_eq!(
            clash(&[], &[], &[for_each(vec![m("y.PraxD")], vec![insert("done")])]),
            ["PraxD"]
        );
        // Subquery free-var list.
        assert_eq!(
            clash(
                &[],
                &[Condition::Subquery { set: "S".into(), find: vec!["PraxD".into()], where_: vec![m("seen.ok")] }],
                &[]
            ),
            ["PraxD"]
        );
        // A forbidden splice name (Actor) is caught; an ordinary var is not.
        assert_eq!(clash(&forbidden, &[m("who.Actor")], &[]), ["Actor"]);
        assert!(clash(&forbidden, &[m("who.Other")], &[]).is_empty());
        // Constants are never flagged.
        assert!(clash(&[], &[m("plain.path")], &[]).is_empty());
        // A malformed authored sentence dies BEFORE any hygiene verdict, as the
        // frozen walk does — the guard never gets to call it clean.
        assert_eq!(
            authored_var_clash(&[], &[m("who.Other.")], &[]),
            Err(WorldError::TrailingOperator {
                sentence: "who.Other.".to_owned(),
                op: '.',
            })
        );
    }

    #[test]
    fn authored_pat_clash_over_split_names() {
        assert_eq!(authored_pat_clash(&[], &["mark".into(), "PraxD".into()]), ["PraxD"]);
        assert!(authored_pat_clash(&[], &["mark".into(), "X".into()]).is_empty());
    }
}
