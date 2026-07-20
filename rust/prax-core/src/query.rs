//! The condition language over [`Db::unify`]: the authoring [`Condition`]
//! (string operands, what the builder DSL produces) and its interned runtime
//! mirror [`Cond`] (segments and names pre-interned at install). A query is a
//! conjunctive list evaluated left to right, threading a growing set of variable
//! bindings — the list-monad nondeterminism of unification.
//!
//! Frozen reference: `src/Prax/Query.hs`, which carries the whole language TWICE
//! — a `Condition` string evaluator (`query`) and a `CookedCondition` interned
//! evaluator (`queryCooked`), pinned equal by `Prax.CookedSpec`. That duality is
//! a Haskell accident: here there is ONE representation and ONE evaluator. The
//! authoring [`Condition`] is a distinct type family from the runtime [`Cond`];
//! [`compile_condition`] is the only bridge (the install-time choke point), and
//! [`query`] runs over [`Cond`] alone. The dual-path equivalence pins die as
//! `implementation` in `conformance/KILLED.md`; their underlying scenarios are
//! re-expressed as direct result pins on the one evaluator (see the tests).
//!
//! Operand convention (the DSL's, `Prax.Sym`): an operand whose first character
//! is uppercase is a logic variable (resolved against the bindings), otherwise a
//! literal constant. An unresolvable operand (an unbound variable where a value
//! is needed) drops the binding — the defined behaviour of the language (e.g.
//! the tic-tac-toe tie action relies on `Neq` dropping when a winner is unbound).
//! A structurally impossible query (a subquery nested inside a subquery) is
//! rejected loudly at compile, not at run (see [`crate::error::WorldError`]).

use smallvec::SmallVec;

use crate::db::{Bindings, Db, Val, val_to_sym};
use crate::error::WorldError;
use crate::interner::{Interner, Sym};
use crate::path::{segment_names, tokenize};

/// Numeric comparison operators (`lt`, `lte`, `gt`, `gte`) — `Prax.Query.CmpOp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Lt,
    Lte,
    Gt,
    Gte,
}

/// Binary integer operators for [`Condition::Calc`] (`add`, `sub`, `mul`, `mod`)
/// — `Prax.Query.CalcOp`. Division is deliberately absent (it would break the
/// DB's integer-valuedness); `Mod` follows Haskell's `mod`, whose result carries
/// the divisor's sign (non-negative for a positive modulus).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalcOp {
    Add,
    Sub,
    Mul,
    Mod,
}

/// A single authoring condition — the string-surfaced form the builder DSL
/// produces (`Prax.Query.Condition`). Compiled to [`Cond`] at install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// A bare logic sentence, unified against the DB (extends bindings).
    Match(String),
    /// Negation as failure: keep the binding iff the sentence has no match.
    Not(String),
    /// Equality that doubles as assignment: if exactly one operand is an unbound
    /// variable, bind it to the other; if both resolve, keep iff equal.
    Eq(String, String),
    /// Keep the binding iff the two (resolvable) operands differ.
    Neq(String, String),
    /// Numeric comparison of two resolvable operands.
    Cmp(CmpOp, String, String),
    /// `Calc result op lhs rhs`: bind `result` to `lhs op rhs` (integers).
    Calc(String, CalcOp, String, String),
    /// `Count result setVar`: bind `result` to the size of a set-valued var.
    Count(String, String),
    /// Run a nested query and bind `set` to the list of projected rows.
    Subquery {
        /// Variable bound to the resulting set.
        set: String,
        /// Variables projected out of each sub-result row.
        find: Vec<String>,
        /// The sub-query's conditions.
        where_: Vec<Condition>,
    },
    /// Disjunction: the deduplicated union of the bindings from every satisfying
    /// clause (a generator — a clause may bind variables the others don't).
    Or(Vec<Vec<Condition>>),
    /// Negation-as-failure over a whole conjunction: keep the binding iff the
    /// inner conjunction has no solution (`¬∃`).
    Absent(Vec<Condition>),
    /// Dual of [`Condition::Absent`]: keep the binding iff the inner conjunction
    /// has a solution, discarding the witnesses (a boolean `∃`).
    Exists(Vec<Condition>),
}

/// Universal quantification `∀ x. guard(x) → body(x)`, as "there is no
/// `guard`-binding for which `body` fails" (`Prax.Query.forAll`).
pub fn for_all(guard: Vec<Condition>, body: Vec<Condition>) -> Condition {
    let mut inner = guard;
    inner.push(Condition::Absent(body));
    Condition::Absent(inner)
}

/// Material implication `a → b`: either `a` is unsatisfiable from the current
/// binding, or `b` holds (`Prax.Query.implies`).
pub fn implies(a: Vec<Condition>, b: Vec<Condition>) -> Condition {
    Condition::Or(vec![vec![Condition::Absent(a)], b])
}

/// The interned runtime mirror of [`Condition`]: `Match`/`Not` carry the
/// pre-tokenized segments (queries flatten `.`/`!`, so only the segment `Sym`s
/// matter — `Prax.Query.CookedCondition`'s `CMatch [Sym]`); every other operand
/// is a pre-interned [`Sym`]. Produced only by [`compile_condition`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cond {
    Match(SmallVec<[Sym; 6]>),
    Not(SmallVec<[Sym; 6]>),
    Eq(Sym, Sym),
    Neq(Sym, Sym),
    Cmp(CmpOp, Sym, Sym),
    Calc(Sym, CalcOp, Sym, Sym),
    Count(Sym, Sym),
    Subquery {
        set: Sym,
        find: Vec<Sym>,
        where_: Vec<Cond>,
    },
    Or(Vec<Vec<Cond>>),
    Absent(Vec<Cond>),
    Exists(Vec<Cond>),
}

/// Compile an authoring [`Condition`] to its runtime [`Cond`] — the single
/// bridge between the two type families, interning every operand once. `Match`/
/// `Not` sentences are tokenized (rejecting a trailing operator, as [`tokenize`]
/// does); a `Subquery` nested inside another `Subquery` is a loud
/// [`WorldError::NestedSubquery`] (the structural check the frozen evaluator
/// makes at run time, moved to its construction-guard home).
pub fn compile_condition(interner: &mut Interner, cond: &Condition) -> Result<Cond, WorldError> {
    compile_inner(interner, cond, false)
}

fn compile_inner(
    interner: &mut Interner,
    cond: &Condition,
    in_sub: bool,
) -> Result<Cond, WorldError> {
    Ok(match cond {
        Condition::Match(s) => Cond::Match(tokenize(interner, s)?.segs),
        Condition::Not(s) => Cond::Not(tokenize(interner, s)?.segs),
        Condition::Eq(x, y) => Cond::Eq(interner.intern(x), interner.intern(y)),
        Condition::Neq(x, y) => Cond::Neq(interner.intern(x), interner.intern(y)),
        Condition::Cmp(op, x, y) => Cond::Cmp(*op, interner.intern(x), interner.intern(y)),
        Condition::Calc(r, op, x, y) => Cond::Calc(
            interner.intern(r),
            *op,
            interner.intern(x),
            interner.intern(y),
        ),
        Condition::Count(r, s) => Cond::Count(interner.intern(r), interner.intern(s)),
        Condition::Subquery { set, find, where_ } => {
            if in_sub {
                return Err(WorldError::NestedSubquery);
            }
            let set = interner.intern(set);
            let find = find.iter().map(|f| interner.intern(f)).collect();
            let where_ = compile_list(interner, where_, true)?;
            Cond::Subquery { set, find, where_ }
        }
        Condition::Or(clauses) => {
            let mut out = Vec::with_capacity(clauses.len());
            for clause in clauses {
                out.push(compile_list(interner, clause, in_sub)?);
            }
            Cond::Or(out)
        }
        Condition::Absent(cs) => Cond::Absent(compile_list(interner, cs, in_sub)?),
        Condition::Exists(cs) => Cond::Exists(compile_list(interner, cs, in_sub)?),
    })
}

fn compile_list(
    interner: &mut Interner,
    conds: &[Condition],
    in_sub: bool,
) -> Result<Vec<Cond>, WorldError> {
    conds
        .iter()
        .map(|c| compile_inner(interner, c, in_sub))
        .collect()
}

/// Evaluate a conjunctive list of conditions from a starting binding, yielding
/// every consistent binding that satisfies them all (`Prax.Query.query`, the one
/// evaluator — the cooked path is the only path). Left to right, each condition
/// filters and extends the binding list; branching (an unbound variable in a
/// `Match`) is name-ordered by [`Db::unify`], the determinism contract.
///
/// The interner is threaded explicitly (`&mut`) where the frozen code reaches
/// the process-global `unsafePerformIO` pool: `unify` re-keys a bound value,
/// `Eq`/`Neq` compare by rendered symbol, `Cmp`/`Calc`/`Count` read numbers,
/// and a subquery projects rows.
pub fn query(
    interner: &mut Interner,
    db: &Db,
    conds: &[Cond],
    seed: &Bindings,
) -> Vec<Bindings> {
    let mut matches = vec![seed.clone()];
    for cond in conds {
        let mut next = Vec::new();
        for b in matches {
            eval_cond(interner, db, cond, b, &mut next);
        }
        matches = next;
    }
    matches
}

/// Evaluate one condition against one binding, pushing every consistent
/// extension into `out` (`Prax.Query.evalCookedCond`, case for case).
fn eval_cond(interner: &mut Interner, db: &Db, cond: &Cond, b: Bindings, out: &mut Vec<Bindings>) {
    match cond {
        Cond::Match(segs) => out.extend(db.unify(interner, segs, b)),
        Cond::Not(segs) => {
            if db.unify(interner, segs, b.clone()).is_empty() {
                out.push(b);
            }
        }
        Cond::Eq(lhs, rhs) => match (resolve(&b, *lhs), resolve(&b, *rhs)) {
            (Some(l), Some(r)) => {
                if val_to_sym(interner, &l) == val_to_sym(interner, &r) {
                    out.push(b);
                }
            }
            (Some(l), None) => {
                let mut b = b;
                b.insert(*rhs, l);
                out.push(b);
            }
            (None, Some(r)) => {
                let mut b = b;
                b.insert(*lhs, r);
                out.push(b);
            }
            (None, None) => {}
        },
        Cond::Neq(lhs, rhs) => {
            if let (Some(l), Some(r)) = (resolve(&b, *lhs), resolve(&b, *rhs))
                && val_to_sym(interner, &l) != val_to_sym(interner, &r)
            {
                out.push(b);
            }
        }
        Cond::Cmp(op, lhs, rhs) => {
            let l = resolve(&b, *lhs).and_then(|v| num(interner, &v));
            let r = resolve(&b, *rhs).and_then(|v| num(interner, &v));
            if let (Some(l), Some(r)) = (l, r)
                && apply_cmp(*op, l, r)
            {
                out.push(b);
            }
        }
        Cond::Calc(result, op, lhs, rhs) => {
            let l = resolve(&b, *lhs).and_then(|v| num(interner, &v));
            let r = resolve(&b, *rhs).and_then(|v| num(interner, &v));
            if let (Some(l), Some(r)) = (l, r) {
                let mut b = b;
                b.insert(*result, Val::Num(apply_calc(*op, l, r)));
                out.push(b);
            }
        }
        Cond::Count(result, set_var) => {
            if let Some(Val::Set(xs)) = resolve(&b, *set_var) {
                let n = xs.len() as i64;
                let mut b = b;
                b.insert(*result, Val::Num(n));
                out.push(b);
            }
        }
        Cond::Subquery { set, find, where_ } => {
            let results = query(interner, db, where_, &b);
            let rows: Vec<Vec<Sym>> = results
                .iter()
                .map(|r| {
                    find.iter()
                        .map(|&lvar| match r.get(lvar) {
                            Some(v) => val_to_sym(interner, v),
                            None => lvar,
                        })
                        .collect()
                })
                .collect();
            let mut b = b;
            b.insert(*set, Val::Set(rows));
            out.push(b);
        }
        Cond::Or(clauses) => {
            // Disjunction with `nub`: the deduplicated union of every clause's
            // bindings, in clause order then within-clause order. The dedup is
            // local to this Or over this binding (the frozen `nub (concat ...)`),
            // never across sibling bindings.
            let mut local: Vec<Bindings> = Vec::new();
            for clause in clauses {
                for r in query(interner, db, clause, &b) {
                    if !local.contains(&r) {
                        local.push(r);
                    }
                }
            }
            out.extend(local);
        }
        Cond::Absent(cs) => {
            if query(interner, db, cs, &b).is_empty() {
                out.push(b);
            }
        }
        Cond::Exists(cs) => {
            if !query(interner, db, cs, &b).is_empty() {
                out.push(b);
            }
        }
    }
}

/// Whether the conditions are satisfiable from the given binding
/// (`Prax.Query.satisfies`).
pub fn satisfies(interner: &mut Interner, db: &Db, conds: &[Cond], seed: &Bindings) -> bool {
    !query(interner, db, conds, seed).is_empty()
}

/// The number of consistent bindings satisfying the conditions
/// (`Prax.Query.countSatisfying`) — a want scores once per satisfying
/// instantiation.
pub fn count_satisfying(interner: &mut Interner, db: &Db, conds: &[Cond], seed: &Bindings) -> usize {
    query(interner, db, conds, seed).len()
}

/// Resolve an operand symbol: an uppercase-initial (`is_var`) symbol is a
/// variable, looked up in the bindings (`None` if unbound); any other symbol is
/// a literal constant, resolving to itself (`Prax.Query.resolve`).
fn resolve(b: &Bindings, s: Sym) -> Option<Val> {
    if s.is_var() {
        b.get(s).cloned()
    } else {
        Some(Val::Sym(s))
    }
}

/// The integer a value denotes, if any (`Prax.Query.num`): a [`Val::Num`]
/// directly, a [`Val::Sym`] via parsing its name, never a [`Val::Set`].
fn num(interner: &Interner, v: &Val) -> Option<i64> {
    match v {
        Val::Num(n) => Some(*n),
        Val::Sym(s) => read_integer(interner.resolve(*s)),
        Val::Set(_) => None,
    }
}

/// Parse an operand name as an integer, matching the frozen `readMaybe ::
/// Maybe Integer` on the operand shapes the engine can produce: an optional
/// leading `-` then ASCII digits. A non-numeric name (or one out of `i64` range)
/// is `None` — the defined "unresolvable operand drops the binding" path, not a
/// silent swallow. (`i64` replaces `Integer`: a recorded deviation,
/// ARCHITECTURE.md. No reachable operand carries the `+`/whitespace/parenthesis
/// forms the full `Read Integer` instance also accepts — `Val::Num`'s rendering
/// and authored integer literals are always plain `-?[0-9]+`.)
fn read_integer(name: &str) -> Option<i64> {
    let digits = name.strip_prefix('-').unwrap_or(name);
    if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    name.parse::<i64>().ok()
}

fn apply_cmp(op: CmpOp, l: i64, r: i64) -> bool {
    match op {
        CmpOp::Lt => l < r,
        CmpOp::Lte => l <= r,
        CmpOp::Gt => l > r,
        CmpOp::Gte => l >= r,
    }
}

/// `lhs op rhs` in checked `i64`, panicking loudly on overflow (the recorded
/// `i64`-for-`Integer` deviation's contract: correct or crash, never a wrapped
/// wrong answer). `Mod` is Haskell's `mod` — the result carries the divisor's
/// sign — computed via checked remainder plus a sign correction.
fn apply_calc(op: CalcOp, l: i64, r: i64) -> i64 {
    let checked = match op {
        CalcOp::Add => l.checked_add(r),
        CalcOp::Sub => l.checked_sub(r),
        CalcOp::Mul => l.checked_mul(r),
        CalcOp::Mod => haskell_mod(l, r),
    };
    checked.unwrap_or_else(|| panic!("Prax.Query: Calc {op:?} overflow on {l} and {r}"))
}

/// Haskell's `mod`: `a - b * floor(a / b)`, so the result takes the sign of the
/// divisor `b`. `checked_rem` keeps overflow (`i64::MIN mod -1`) and division by
/// zero loud rather than a silent wrap or an undefined result.
fn haskell_mod(a: i64, b: i64) -> Option<i64> {
    let r = a.checked_rem(b)?;
    if r != 0 && (r < 0) != (b < 0) {
        Some(r + b)
    } else {
        Some(r)
    }
}

// ---- authoring-boundary walkers (Prax.Query residents) --------------------

/// Every name a condition *mentions* — a total walk over every constructor,
/// including subquery internals (`Prax.Query.conditionVars`). An
/// over-approximation of what it binds: `Eq`/`Neq`/`Cmp`/`Calc`/`Count` operands
/// are listed whether variable or constant. The authoring-boundary home (strings,
/// pre-compilation) for reserved-variable guards in later stages.
pub fn condition_vars(c: &Condition) -> Vec<String> {
    match c {
        Condition::Match(s) | Condition::Not(s) => segment_names(s),
        Condition::Absent(cs) | Condition::Exists(cs) => {
            cs.iter().flat_map(condition_vars).collect()
        }
        Condition::Or(clauses) => clauses
            .iter()
            .flatten()
            .flat_map(condition_vars)
            .collect(),
        Condition::Subquery { set, find, where_ } => {
            let mut out = Vec::with_capacity(1 + find.len());
            out.push(set.clone());
            out.extend(find.iter().cloned());
            out.extend(where_.iter().flat_map(condition_vars));
            out
        }
        Condition::Eq(a, b) | Condition::Neq(a, b) => vec![a.clone(), b.clone()],
        Condition::Cmp(_, a, b) => vec![a.clone(), b.clone()],
        Condition::Calc(v, _, a, b) => vec![v.clone(), a.clone(), b.clone()],
        Condition::Count(v, s) => vec![v.clone(), s.clone()],
    }
}

/// Every *sentence string* a condition list mentions — the raw authored paths
/// (`Match`/`Not` operands, recursing through `Absent`/`Exists`/`Or`/`Subquery`),
/// as opposed to [`condition_vars`]'s split-and-flattened names
/// (`Prax.Query.condSents`). The read-side home for path-family head-segment
/// checks in later stages.
pub fn cond_sents(conds: &[Condition]) -> Vec<String> {
    conds
        .iter()
        .flat_map(|c| match c {
            Condition::Match(s) | Condition::Not(s) => vec![s.clone()],
            Condition::Absent(cs) | Condition::Exists(cs) => cond_sents(cs),
            Condition::Or(clauses) => clauses.iter().flat_map(|cl| cond_sents(cl)).collect(),
            Condition::Subquery { where_, .. } => cond_sents(where_),
            _ => Vec::new(),
        })
        .collect()
}

/// Substitute bindings into every sentence/operand of a condition, mirroring
/// `Prax.Query.groundCondition`: each operand is grounded as a one-segment path
/// (variables present in the bindings are replaced, preserving `!`/`.`; others
/// are left for the query to quantify). Returns [`WorldError`] only if an
/// operand is not a well-formed path (a trailing operator).
pub fn ground_condition(
    interner: &mut Interner,
    b: &Bindings,
    c: &Condition,
) -> Result<Condition, WorldError> {
    let g = |interner: &mut Interner, s: &str| -> Result<String, WorldError> {
        let path = tokenize(interner, s)?;
        Ok(crate::db::ground(interner, &path, b))
    };
    Ok(match c {
        Condition::Match(s) => Condition::Match(g(interner, s)?),
        Condition::Not(s) => Condition::Not(g(interner, s)?),
        Condition::Eq(x, y) => Condition::Eq(g(interner, x)?, g(interner, y)?),
        Condition::Neq(x, y) => Condition::Neq(g(interner, x)?, g(interner, y)?),
        Condition::Cmp(op, x, y) => Condition::Cmp(*op, g(interner, x)?, g(interner, y)?),
        Condition::Calc(r, op, x, y) => {
            Condition::Calc(g(interner, r)?, *op, g(interner, x)?, g(interner, y)?)
        }
        Condition::Count(r, s) => Condition::Count(g(interner, r)?, g(interner, s)?),
        Condition::Subquery { set, find, where_ } => Condition::Subquery {
            set: g(interner, set)?,
            find: find
                .iter()
                .map(|f| g(interner, f))
                .collect::<Result<_, _>>()?,
            where_: ground_list(interner, b, where_)?,
        },
        Condition::Or(clauses) => {
            let mut out = Vec::with_capacity(clauses.len());
            for clause in clauses {
                out.push(ground_list(interner, b, clause)?);
            }
            Condition::Or(out)
        }
        Condition::Absent(cs) => Condition::Absent(ground_list(interner, b, cs)?),
        Condition::Exists(cs) => Condition::Exists(ground_list(interner, b, cs)?),
    })
}

fn ground_list(
    interner: &mut Interner,
    b: &Bindings,
    conds: &[Condition],
) -> Result<Vec<Condition>, WorldError> {
    conds
        .iter()
        .map(|c| ground_condition(interner, b, c))
        .collect()
}

/// Every DB path a compiled query can consult, at any polarity — including
/// inside `Or`/`Absent`/`Exists`/`Subquery` (`Prax.Query.cookedReadAnchors`).
/// Complete by construction: `Eq`/`Neq`/`Cmp`/`Calc` compare already-bound
/// values and `Count` measures a bound set, so none of them reads a path this
/// walk misses. The read-anchor source the derivation/relevance layer builds on.
pub fn read_anchors(conds: &[Cond]) -> Vec<SmallVec<[Sym; 6]>> {
    let mut out = Vec::new();
    for c in conds {
        collect_anchors(c, &mut out);
    }
    out
}

fn collect_anchors(c: &Cond, out: &mut Vec<SmallVec<[Sym; 6]>>) {
    match c {
        Cond::Match(p) | Cond::Not(p) => out.push(p.clone()),
        Cond::Or(clauses) => {
            for clause in clauses {
                for c in clause {
                    collect_anchors(c, out);
                }
            }
        }
        Cond::Absent(cs) | Cond::Exists(cs) => {
            for c in cs {
                collect_anchors(c, out);
            }
        }
        Cond::Subquery { where_, .. } => {
            for c in where_ {
                collect_anchors(c, out);
            }
        }
        Cond::Eq(..)
        | Cond::Neq(..)
        | Cond::Cmp(..)
        | Cond::Calc(..)
        | Cond::Count(..) => {}
    }
}

#[cfg(test)]
mod tests {
    // H: QuerySpec.hs "Prax.Query"
    //
    // The frozen `Prax.QuerySpec`, re-expressed against the one compiled
    // evaluator, plus the observable content of `Prax.CookedSpec` (its dual-path
    // equivalence pins die in KILLED.md; the queries they ran are re-expressed
    // here as direct result pins).
    use super::*;
    use crate::db::Db;

    fn build(interner: &mut Interner, facts: &[&str]) -> Db {
        let mut db = Db::empty();
        for f in facts {
            db = db.insert_str(interner, f).unwrap();
        }
        db
    }

    /// Compile a condition list, then evaluate it — the authoring-to-runtime
    /// path exactly as a world install would take.
    fn run(
        interner: &mut Interner,
        db: &Db,
        conds: &[Condition],
        seed: &Bindings,
    ) -> Vec<Bindings> {
        let compiled: Vec<Cond> = conds
            .iter()
            .map(|c| compile_condition(interner, c).unwrap())
            .collect();
        query(interner, db, &compiled, seed)
    }

    fn seed(interner: &mut Interner, pairs: &[(&str, &str)]) -> Bindings {
        let mut b = Bindings::new();
        for (k, v) in pairs {
            let key = interner.intern(k);
            b.insert(key, Val::Sym(interner.intern(v)));
        }
        b
    }

    /// A binding rendered to (name, value-string) pairs in `Sym`-id order — the
    /// observable form, comparison-stable across interners.
    fn render(interner: &Interner, b: &Bindings) -> Vec<(String, String)> {
        b.iter()
            .map(|(s, v)| {
                (
                    interner.resolve(s).to_owned(),
                    crate::db::val_to_string(interner, v),
                )
            })
            .collect()
    }

    fn render_all(interner: &Interner, bs: &[Bindings]) -> Vec<Vec<(String, String)>> {
        bs.iter().map(|b| render(interner, b)).collect()
    }

    /// The value bound to `name`, rendered — for single-variable assertions.
    fn look(interner: &Interner, b: &Bindings, name: &str) -> Option<String> {
        // The name is already interned (it was authored into the query); look it
        // up without a fresh &mut by scanning for the resolved name.
        b.iter()
            .find(|(s, _)| interner.resolve(*s) == name)
            .map(|(_, v)| crate::db::val_to_string(interner, v))
    }

    // Convenience Condition builders keeping the pins readable.
    fn m(s: &str) -> Condition {
        Condition::Match(s.to_owned())
    }
    fn n(s: &str) -> Condition {
        Condition::Not(s.to_owned())
    }
    fn eq(a: &str, b: &str) -> Condition {
        Condition::Eq(a.to_owned(), b.to_owned())
    }
    fn neq(a: &str, b: &str) -> Condition {
        Condition::Neq(a.to_owned(), b.to_owned())
    }

    // ===== match / not =====
    // H: QuerySpec.hs "match / not"

    // H: QuerySpec.hs "bare sentence unifies and binds"
    #[test]
    fn bare_sentence_unifies_and_binds() {
        let mut i = Interner::new();
        let db = build(&mut i, &["char.tim", "char.kevin"]);
        assert_eq!(run(&mut i, &db, &[m("char.Who")], &Bindings::new()).len(), 2);
    }

    // H: QuerySpec.hs "negation as failure keeps binding when absent"
    #[test]
    fn negation_keeps_binding_when_absent() {
        let mut i = Interner::new();
        let db = build(&mut i, &["char.tim"]);
        assert_eq!(
            run(&mut i, &db, &[n("isDancing.tim")], &Bindings::new()),
            vec![Bindings::new()]
        );
    }

    // H: QuerySpec.hs "negation as failure drops binding when present"
    #[test]
    fn negation_drops_binding_when_present() {
        let mut i = Interner::new();
        let db = build(&mut i, &["isDancing.tim"]);
        assert!(run(&mut i, &db, &[n("isDancing.tim")], &Bindings::new()).is_empty());
    }

    // ===== eq / neq =====
    // H: QuerySpec.hs "eq / neq"

    // H: QuerySpec.hs "eq binds an unbound variable to a constant"
    #[test]
    fn eq_binds_an_unbound_variable_to_a_constant() {
        let mut i = Interner::new();
        let db = Db::empty();
        let rs = run(&mut i, &db, &[eq("X", "beer")], &Bindings::new());
        assert_eq!(rs.len(), 1);
        assert_eq!(look(&i, &rs[0], "X").as_deref(), Some("beer"));
    }

    // H: QuerySpec.hs "eq of two equal bound values keeps the binding"
    #[test]
    fn eq_of_two_equal_bound_values_keeps() {
        let mut i = Interner::new();
        let s = seed(&mut i, &[("X", "a"), ("Y", "a")]);
        let rs = run(&mut i, &Db::empty(), &[eq("X", "Y")], &s);
        assert_eq!(rs, vec![s]);
    }

    // H: QuerySpec.hs "eq of two differing bound values drops the binding"
    #[test]
    fn eq_of_two_differing_bound_values_drops() {
        let mut i = Interner::new();
        let s = seed(&mut i, &[("X", "a"), ("Y", "b")]);
        assert!(run(&mut i, &Db::empty(), &[eq("X", "Y")], &s).is_empty());
    }

    // H: QuerySpec.hs "neq keeps distinct, drops equal"
    #[test]
    fn neq_keeps_distinct_drops_equal() {
        let mut i = Interner::new();
        let distinct = seed(&mut i, &[("X", "a"), ("Y", "b")]);
        assert_eq!(
            run(&mut i, &Db::empty(), &[neq("X", "Y")], &distinct),
            vec![distinct.clone()]
        );
        let equal = seed(&mut i, &[("X", "a"), ("Y", "a")]);
        assert!(run(&mut i, &Db::empty(), &[neq("X", "Y")], &equal).is_empty());
    }

    // H: QuerySpec.hs "neq with an unbound operand drops the binding (tie-game reliance)"
    #[test]
    fn neq_with_an_unbound_operand_drops() {
        let mut i = Interner::new();
        let s = seed(&mut i, &[("Actor", "tim")]);
        assert!(run(&mut i, &Db::empty(), &[neq("Actor", "Winner")], &s).is_empty());
    }

    // ===== numeric: cmp / calc =====
    // H: QuerySpec.hs "numeric: cmp / calc  (port of tests.js math block)"

    // H: QuerySpec.hs "gt fails then passes across an exclusion update"
    #[test]
    fn gt_fails_then_passes_across_an_exclusion_update() {
        let mut i = Interner::new();
        let db0 = build(&mut i, &["counter.0"]);
        // gt 4 fails at 0.
        assert!(
            run(
                &mut i,
                &db0,
                &[m("counter.Val"), Condition::Cmp(CmpOp::Gt, "Val".into(), "4".into())],
                &Bindings::new()
            )
            .is_empty()
        );
        // calc NewVal = 0 + 5.
        let rs = run(
            &mut i,
            &db0,
            &[
                m("counter.Val"),
                Condition::Calc("NewVal".into(), CalcOp::Add, "Val".into(), "5".into()),
            ],
            &Bindings::new(),
        );
        assert_eq!(rs.len(), 1);
        assert_eq!(look(&i, &rs[0], "NewVal").as_deref(), Some("5"));
        // Exclusion update to 5; now gt 4 holds with Val = 5.
        let db1 = db0.insert_str(&mut i, "counter!5").unwrap();
        let rs = run(
            &mut i,
            &db1,
            &[m("counter.Val"), Condition::Cmp(CmpOp::Gt, "Val".into(), "4".into())],
            &Bindings::new(),
        );
        assert_eq!(
            rs.iter().map(|b| look(&i, b, "Val")).collect::<Vec<_>>(),
            vec![Some("5".to_owned())]
        );
    }

    // H: QuerySpec.hs "chained calc: mul then sub yields -20"
    #[test]
    fn chained_calc_mul_then_sub_yields_minus_20() {
        let mut i = Interner::new();
        let db = build(&mut i, &["counter!5"]);
        let rs = run(
            &mut i,
            &db,
            &[
                m("counter.Val"),
                Condition::Calc("BigVal".into(), CalcOp::Mul, "Val".into(), "Val".into()),
                Condition::Cmp(CmpOp::Lt, "Val".into(), "BigVal".into()),
                Condition::Calc("TinyVal".into(), CalcOp::Sub, "Val".into(), "BigVal".into()),
            ],
            &Bindings::new(),
        );
        assert_eq!(rs.len(), 1);
        assert_eq!(look(&i, &rs[0], "BigVal").as_deref(), Some("25"));
        assert_eq!(look(&i, &rs[0], "TinyVal").as_deref(), Some("-20"));
    }

    // H: QuerySpec.hs "mod binds 17 mod 5 = 2"
    #[test]
    fn mod_binds_17_mod_5_is_2() {
        let mut i = Interner::new();
        let rs = run(
            &mut i,
            &Db::empty(),
            &[Condition::Calc("R".into(), CalcOp::Mod, "17".into(), "5".into())],
            &Bindings::new(),
        );
        assert_eq!(rs.len(), 1);
        assert_eq!(look(&i, &rs[0], "R").as_deref(), Some("2"));
    }

    // H: QuerySpec.hs "mod on a negative left operand follows Haskell semantics: -3 mod 5 = 2"
    #[test]
    fn mod_on_a_negative_left_operand_is_2() {
        let mut i = Interner::new();
        let rs = run(
            &mut i,
            &Db::empty(),
            &[Condition::Calc("R".into(), CalcOp::Mod, "-3".into(), "5".into())],
            &Bindings::new(),
        );
        assert_eq!(rs.len(), 1);
        assert_eq!(look(&i, &rs[0], "R").as_deref(), Some("2"));
    }

    /// A `Mul` whose product overflows `i64` must crash loudly, not wrap — the
    /// recorded `i64`-for-`Integer` deviation's contract (correct or crash).
    #[test]
    #[should_panic(expected = "Calc Mul overflow")]
    fn calc_overflow_is_loud() {
        let mut i = Interner::new();
        let big = (i64::MAX).to_string();
        let s = {
            let mut b = Bindings::new();
            let x = i.intern("X");
            b.insert(x, Val::Num(i64::MAX));
            b
        };
        // X * MAX overflows i64.
        let _ = run(
            &mut i,
            &Db::empty(),
            &[Condition::Calc("R".into(), CalcOp::Mul, "X".into(), big)],
            &s,
        );
    }

    // ===== subquery / count =====
    // H: QuerySpec.hs "subquery / count  (port of tests.js subquery block)"

    fn subquery(set: &str, find: &[&str], where_: Vec<Condition>) -> Condition {
        Condition::Subquery {
            set: set.to_owned(),
            find: find.iter().map(|s| s.to_string()).collect(),
            where_,
        }
    }

    // H: QuerySpec.hs "count dancers other than the actor equals 2"
    #[test]
    fn count_dancers_other_than_the_actor_equals_2() {
        let mut i = Interner::new();
        let db = build(
            &mut i,
            &[
                "char.tim",
                "char.kevin",
                "char.james",
                "char.jer",
                "isDancing.tim",
                "isDancing.kevin",
                "isDancing.jer",
            ],
        );
        let conds = vec![
            m("char.Actor"),
            subquery(
                "Dancers",
                &["Dancer"],
                vec![m("char.Dancer"), m("isDancing.Dancer"), neq("Dancer", "Actor")],
            ),
            Condition::Count("NumDancers".into(), "Dancers".into()),
            eq("NumDancers", "2"),
        ];
        let s = seed(&mut i, &[("Actor", "tim")]);
        let rs = run(&mut i, &db, &conds, &s);
        assert_eq!(rs.len(), 1);
        // The two OTHER dancers (kevin, jer): a set of size 2.
        assert_eq!(look(&i, &rs[0], "Dancers").as_deref(), Some("<Set(2)>"));
        assert_eq!(look(&i, &rs[0], "NumDancers").as_deref(), Some("2"));
    }

    // H: QuerySpec.hs "eq on the count filters out the wrong actor"
    #[test]
    fn eq_on_the_count_filters_out_the_wrong_actor() {
        let mut i = Interner::new();
        let db = build(&mut i, &["char.tim", "char.solo", "isDancing.tim"]);
        let conds = vec![
            m("char.Actor"),
            subquery(
                "Dancers",
                &["Dancer"],
                vec![m("isDancing.Dancer"), neq("Dancer", "Actor")],
            ),
            Condition::Count("NumDancers".into(), "Dancers".into()),
            eq("NumDancers", "2"),
        ];
        // solo sees only tim dancing (count 1), so eq 2 fails.
        let s = seed(&mut i, &[("Actor", "solo")]);
        assert!(run(&mut i, &db, &conds, &s).is_empty());
    }

    // ===== groundCondition =====
    // H: QuerySpec.hs "groundCondition"

    // H: QuerySpec.hs "groundCondition substitutes bindings through every constructor"
    #[test]
    fn ground_condition_substitutes_through_every_constructor() {
        let mut i = Interner::new();
        let b = seed(&mut i, &[("A", "bob")]);
        let cases: &[(Condition, Condition)] = &[
            (m("at.A!P"), m("at.bob!P")),
            (n("seen.A"), n("seen.bob")),
            (eq("A", "X"), eq("bob", "X")),
            (neq("W", "A"), neq("W", "bob")),
            (
                Condition::Cmp(CmpOp::Gt, "A".into(), "N".into()),
                Condition::Cmp(CmpOp::Gt, "bob".into(), "N".into()),
            ),
            (
                Condition::Calc("R".into(), CalcOp::Add, "A".into(), "1".into()),
                Condition::Calc("R".into(), CalcOp::Add, "bob".into(), "1".into()),
            ),
            (
                Condition::Count("R".into(), "A".into()),
                Condition::Count("R".into(), "bob".into()),
            ),
            (
                subquery("S", &["A"], vec![m("p.A")]),
                subquery("S", &["bob"], vec![m("p.bob")]),
            ),
            (
                Condition::Or(vec![vec![m("p.A")], vec![m("q.A")]]),
                Condition::Or(vec![vec![m("p.bob")], vec![m("q.bob")]]),
            ),
            (Condition::Absent(vec![m("p.A")]), Condition::Absent(vec![m("p.bob")])),
            (Condition::Exists(vec![m("p.A")]), Condition::Exists(vec![m("p.bob")])),
        ];
        for (input, want) in cases {
            assert_eq!(&ground_condition(&mut i, &b, input).unwrap(), want);
        }
    }

    // ===== first-order connectives =====
    // H: QuerySpec.hs "first-order connectives (∨, ¬compound, ∃, ∀, →)"

    // H: QuerySpec.hs "Or binds via either clause (disjunction)"
    #[test]
    fn or_binds_via_either_clause() {
        let mut i = Interner::new();
        let db = build(&mut i, &["p.a", "q.b"]);
        let rs = run(
            &mut i,
            &db,
            &[Condition::Or(vec![vec![m("p.X")], vec![m("q.X")]])],
            &Bindings::new(),
        );
        let mut xs: Vec<String> = rs.iter().filter_map(|b| look(&i, b, "X")).collect();
        xs.sort();
        assert_eq!(xs, vec!["a".to_owned(), "b".to_owned()]);
    }

    // H: QuerySpec.hs "Or deduplicates overlapping clauses"
    #[test]
    fn or_deduplicates_overlapping_clauses() {
        let mut i = Interner::new();
        let db = build(&mut i, &["p.a", "q.a"]); // both clauses yield X=a
        assert_eq!(
            run(
                &mut i,
                &db,
                &[Condition::Or(vec![vec![m("p.X")], vec![m("q.X")]])],
                &Bindings::new()
            )
            .len(),
            1
        );
    }

    // H: QuerySpec.hs "Absent is ¬∃ over a compound (no male leader)"
    #[test]
    fn absent_is_not_exists_over_a_compound() {
        let mut i = Interner::new();
        // A male leader exists → Absent fails.
        let db = build(&mut i, &["leader.brown", "brown.sex!male"]);
        assert!(
            run(
                &mut i,
                &db,
                &[Condition::Absent(vec![m("leader.L"), m("L.sex!male")])],
                &Bindings::new()
            )
            .is_empty()
        );
        // Only a female leader → Absent holds.
        let db = build(&mut i, &["leader.lucy", "lucy.sex!female"]);
        assert_eq!(
            run(
                &mut i,
                &db,
                &[Condition::Absent(vec![m("leader.L"), m("L.sex!male")])],
                &Bindings::new()
            ),
            vec![Bindings::new()]
        );
    }

    // H: QuerySpec.hs "Exists is boolean ∃ — satisfiable without leaking witnesses"
    #[test]
    fn exists_is_boolean_without_leaking_witnesses() {
        let mut i = Interner::new();
        let db = build(&mut i, &["char.tim", "char.kev", "here.ok"]);
        // Bare Match multiplies over all chars…
        assert_eq!(
            run(&mut i, &db, &[m("here.OK"), m("char.Who")], &Bindings::new()).len(),
            2
        );
        // …Exists keeps a single binding and does not bind Who.
        let rs = run(
            &mut i,
            &db,
            &[m("here.OK"), Condition::Exists(vec![m("char.Who")])],
            &Bindings::new(),
        );
        assert_eq!(rs.len(), 1);
        assert_eq!(look(&i, &rs[0], "Who"), None);
    }

    // H: QuerySpec.hs "forAll: every patron has a drink (flips when one lacks it)"
    #[test]
    fn for_all_every_patron_has_a_drink() {
        let mut i = Interner::new();
        let cond = for_all(vec![m("patron.P")], vec![m("drink.P")]);
        let has = build(&mut i, &["patron.tim", "patron.kev", "drink.tim", "drink.kev"]);
        assert_eq!(
            run(&mut i, &has, std::slice::from_ref(&cond), &Bindings::new()),
            vec![Bindings::new()]
        );
        let lacks = build(&mut i, &["patron.tim", "patron.kev", "drink.tim"]);
        assert!(run(&mut i, &lacks, std::slice::from_ref(&cond), &Bindings::new()).is_empty());
    }

    // H: QuerySpec.hs "implies: A → B truth table"
    #[test]
    fn implies_truth_table() {
        let mut i = Interner::new();
        let cond = implies(vec![m("raining")], vec![m("wet")]);
        let q = |i: &mut Interner, facts: &[&str]| -> Vec<Bindings> {
            let db = build(i, facts);
            run(i, &db, std::slice::from_ref(&cond), &Bindings::new())
        };
        assert_eq!(q(&mut i, &["raining", "wet"]), vec![Bindings::new()]); // A ∧ B
        assert!(q(&mut i, &["raining"]).is_empty()); // A ∧ ¬B
        assert_eq!(q(&mut i, &["wet"]), vec![Bindings::new()]); // ¬A vacuous
        assert_eq!(q(&mut i, &[]), vec![Bindings::new()]); // ¬A vacuous
    }

    // ===== cookedReadAnchors =====
    // H: QuerySpec.hs "cookedReadAnchors"

    // H: QuerySpec.hs "cookedReadAnchors walks every polarity, including subquery internals"
    #[test]
    fn read_anchors_walks_every_polarity() {
        let mut i = Interner::new();
        let conds = [
            m("a.X"),
            n("b.X"),
            subquery(
                "S",
                &["W"],
                vec![m("c.W.deed"), Condition::Cmp(CmpOp::Gte, "N".into(), "2".into())],
            ),
            Condition::Count("N".into(), "S".into()),
            Condition::Calc("M".into(), CalcOp::Add, "N".into(), "1".into()),
            eq("X", "y"),
            Condition::Or(vec![vec![m("d.X")], vec![Condition::Absent(vec![m("e.X")])]]),
        ];
        let compiled: Vec<Cond> = conds
            .iter()
            .map(|c| compile_condition(&mut i, c).unwrap())
            .collect();
        let anchors = read_anchors(&compiled);
        let want = |i: &mut Interner, s: &str| -> bool {
            let segs = tokenize(i, s).unwrap().segs;
            anchors.contains(&segs)
        };
        assert!(want(&mut i, "a.X"), "a.X read");
        assert!(want(&mut i, "b.X"), "b.X (Not) read");
        assert!(want(&mut i, "c.W.deed"), "subquery inner read");
        assert!(want(&mut i, "d.X"), "Or branch read");
        assert!(want(&mut i, "e.X"), "Absent-in-Or read");
        assert_eq!(anchors.len(), 5);
    }

    // ===== Prax.Cooked: observable content re-expressed on the one evaluator ==
    // H: CookedSpec.hs "Prax.Cooked"
    //
    // The frozen `CookedSpec` pinned `queryCooked == query` on seven fixture
    // cases over an exclusion-bearing db (and cooked-vs-string grounding — killed
    // as duality). With one evaluator there is no pair to compare; the seven
    // scenarios are re-expressed here as DIRECT result pins (expected bindings
    // hand-derived from the semantics, then confirmed against the frozen library
    // in ghci), so their coverage of Match/Not/Absent/Exists/Or/Neq/Eq/Subquery/
    // Count/Cmp over `!`-paths survives the duality's death.
    #[test]
    fn cooked_fixture_scenarios_on_the_one_evaluator() {
        let mut i = Interner::new();
        let db = build(
            &mut i,
            &[
                "at.bob!square",
                "at.eve!mill",
                "at.gale!mill",
                "holding.bob.loaf",
                "regards.dana.carol.thief",
                "regards.gale.carol.thief",
            ],
        );

        // Each case: the condition list and the expected rendered bindings.
        let case0 = run(&mut i, &db, &[m("at.Who!Where")], &Bindings::new());
        assert_eq!(
            render_all(&i, &case0),
            vec![
                vec![("Who".into(), "bob".into()), ("Where".into(), "square".into())],
                vec![("Who".into(), "eve".into()), ("Where".into(), "mill".into())],
                vec![("Who".into(), "gale".into()), ("Where".into(), "mill".into())],
            ]
        );

        let case1 = run(&mut i, &db, &[m("at.Who!mill"), n("holding.Who.loaf")], &Bindings::new());
        assert_eq!(
            render_all(&i, &case1),
            vec![
                vec![("Who".into(), "eve".into())],
                vec![("Who".into(), "gale".into())],
            ]
        );

        let case2 = run(
            &mut i,
            &db,
            &[m("at.Who!Where"), Condition::Absent(vec![m("regards.Who.carol.thief")])],
            &Bindings::new(),
        );
        assert_eq!(
            render_all(&i, &case2),
            vec![
                vec![("Who".into(), "bob".into()), ("Where".into(), "square".into())],
                vec![("Who".into(), "eve".into()), ("Where".into(), "mill".into())],
            ]
        );

        let case3 = run(
            &mut i,
            &db,
            &[Condition::Exists(vec![m("holding.H.loaf")]), m("at.Who!square")],
            &Bindings::new(),
        );
        assert_eq!(render_all(&i, &case3), vec![vec![("Who".into(), "bob".into())]]);

        let case4 = run(
            &mut i,
            &db,
            &[Condition::Or(vec![
                vec![m("at.Who!square")],
                vec![m("regards.Who.carol.thief")],
            ])],
            &Bindings::new(),
        );
        assert_eq!(
            render_all(&i, &case4),
            vec![
                vec![("Who".into(), "bob".into())],
                vec![("Who".into(), "dana".into())],
                vec![("Who".into(), "gale".into())],
            ]
        );

        let case5 = run(
            &mut i,
            &db,
            &[
                subquery("Rs", &["W"], vec![m("regards.W.carol.thief")]),
                Condition::Count("N".into(), "Rs".into()),
                Condition::Cmp(CmpOp::Gte, "N".into(), "2".into()),
            ],
            &Bindings::new(),
        );
        assert_eq!(
            render_all(&i, &case5),
            vec![vec![("Rs".into(), "<Set(2)>".into()), ("N".into(), "2".into())]]
        );

        let case6 = run(
            &mut i,
            &db,
            &[m("at.Who!Where"), neq("Who", "bob"), eq("Place", "Where")],
            &Bindings::new(),
        );
        assert_eq!(
            render_all(&i, &case6),
            vec![
                vec![
                    ("Who".into(), "eve".into()),
                    ("Where".into(), "mill".into()),
                    ("Place".into(), "mill".into())
                ],
                vec![
                    ("Who".into(), "gale".into()),
                    ("Where".into(), "mill".into()),
                    ("Place".into(), "mill".into())
                ],
            ]
        );
    }

    // ===== authoring-boundary walkers (no frozen SpecSpec pin; used by later
    // stages' hygiene guards — tested here for their own correctness) =====

    #[test]
    fn condition_vars_walks_every_constructor() {
        // Every mentioned name, over every constructor incl. subquery internals.
        assert_eq!(condition_vars(&m("at.Who")), vec!["at", "Who"]);
        assert_eq!(condition_vars(&n("seen.A")), vec!["seen", "A"]);
        assert_eq!(condition_vars(&eq("A", "B")), vec!["A", "B"]);
        assert_eq!(
            condition_vars(&Condition::Calc("R".into(), CalcOp::Add, "A".into(), "1".into())),
            vec!["R", "A", "1"]
        );
        assert_eq!(
            condition_vars(&subquery("S", &["W"], vec![m("p.W.deed"), neq("W", "X")])),
            vec!["S", "W", "p", "W", "deed", "W", "X"]
        );
        assert_eq!(
            condition_vars(&Condition::Or(vec![vec![m("p.A")], vec![n("q.B")]])),
            vec!["p", "A", "q", "B"]
        );
    }

    #[test]
    fn cond_sents_collects_raw_paths_recursively() {
        // The raw authored sentence strings, recursing through the compounds.
        let conds = vec![
            m("at.Who"),
            n("seen.Who"),
            eq("A", "B"), // contributes no sentence
            Condition::Absent(vec![m("regards.Who.carol")]),
            Condition::Or(vec![vec![m("p.X")], vec![m("q.Y")]]),
            subquery("S", &["W"], vec![m("inner.W")]),
        ];
        assert_eq!(
            cond_sents(&conds),
            vec!["at.Who", "seen.Who", "regards.Who.carol", "p.X", "q.Y", "inner.W"]
        );
    }

    // ===== compile-time structural guard =====

    #[test]
    fn nested_subquery_is_rejected_at_compile() {
        let mut i = Interner::new();
        let nested = subquery(
            "Outer",
            &["X"],
            vec![subquery("Inner", &["Y"], vec![m("p.Y")])],
        );
        assert_eq!(
            compile_condition(&mut i, &nested),
            Err(WorldError::NestedSubquery)
        );
        // Nesting through Or/Absent inside a subquery is caught too.
        let nested_via_or = subquery(
            "Outer",
            &["X"],
            vec![Condition::Or(vec![vec![subquery("Inner", &["Y"], vec![m("p.Y")])]])],
        );
        assert_eq!(
            compile_condition(&mut i, &nested_via_or),
            Err(WorldError::NestedSubquery)
        );
        // A subquery at top level, and a plain query inside it, compile fine.
        let ok = subquery("Outer", &["X"], vec![m("p.X"), neq("X", "Y")]);
        assert!(compile_condition(&mut i, &ok).is_ok());
    }
}

#[cfg(test)]
mod proptest_laws {
    //! Query law suites (ARCHITECTURE.md's list): compilation is a function of
    //! the authored form (interner-independent results — the heir of
    //! `queryCooked == query`), a seed only narrows a positive query, and `Or`
    //! is deterministic in clause-then-within-clause order with a local `nub`.
    use super::*;
    use crate::db::Db;
    use proptest::prelude::*;
    use std::collections::BTreeMap;

    /// A tiny alphabet: lowercase constants and uppercase variables.
    fn constant() -> impl Strategy<Value = String> {
        prop::sample::select(vec!["a", "b", "c"]).prop_map(String::from)
    }
    fn operand() -> impl Strategy<Value = String> {
        prop::sample::select(vec!["a", "b", "c", "X", "Y", "Z"]).prop_map(String::from)
    }

    /// A ground fact of 1–3 constant segments (dotted).
    fn fact() -> impl Strategy<Value = String> {
        prop::collection::vec(constant(), 1..4).prop_map(|ps| ps.join("."))
    }

    /// A `Match`/`Not` pattern of 1–3 operand segments.
    fn pattern() -> impl Strategy<Value = String> {
        prop::collection::vec(operand(), 1..4).prop_map(|ps| ps.join("."))
    }

    /// A random condition (bounded depth, no Subquery/Count — those need a
    /// bound set to be meaningful; the point is the branching/guard/dedup core).
    fn condition() -> impl Strategy<Value = Condition> {
        let leaf = prop_oneof![
            pattern().prop_map(Condition::Match),
            pattern().prop_map(Condition::Not),
            (operand(), operand()).prop_map(|(a, b)| Condition::Eq(a, b)),
            (operand(), operand()).prop_map(|(a, b)| Condition::Neq(a, b)),
        ];
        leaf.prop_recursive(3, 12, 3, |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 1..3)
                    .prop_map(|cs| Condition::Or(vec![cs])),
                prop::collection::vec(inner.clone(), 1..3).prop_map(Condition::Absent),
                prop::collection::vec(inner, 1..3).prop_map(Condition::Exists),
            ]
        })
    }

    /// Only `Match` conditions — the positive fragment where seeding is
    /// monotone-narrowing (negation and resolve-or-drop arithmetic break that).
    fn match_only() -> impl Strategy<Value = Vec<Condition>> {
        prop::collection::vec(pattern().prop_map(Condition::Match), 0..4)
    }

    /// Build a db and evaluate a condition list from a fresh interner, returning
    /// the rendered bindings (interner-independent, comparison-stable).
    fn eval_fresh(
        facts: &[String],
        conds: &[Condition],
        seed_pairs: &[(String, String)],
    ) -> Vec<Vec<(String, String)>> {
        let mut i = Interner::new();
        let mut db = Db::empty();
        for f in facts {
            db = db.insert_str(&mut i, f).unwrap();
        }
        let mut b = Bindings::new();
        for (k, v) in seed_pairs {
            let key = i.intern(k);
            b.insert(key, Val::Sym(i.intern(v)));
        }
        let compiled: Vec<Cond> = conds
            .iter()
            .map(|c| compile_condition(&mut i, c).unwrap())
            .collect();
        query(&mut i, &db, &compiled, &b)
            .iter()
            .map(|bind| {
                bind.iter()
                    .map(|(s, val)| (i.resolve(s).to_owned(), crate::db::val_to_string(&i, val)))
                    .collect()
            })
            .collect()
    }

    proptest! {
        // Compilation is a function of the AUTHORED form: the rendered results of
        // a query depend only on (facts, conditions, seed), never on the
        // interner's history. Two independent runs (distinct interner states,
        // since the second's pool is seeded differently by the first's dummy
        // interning) agree — the heir of the frozen `queryCooked == query`.
        #[test]
        fn results_are_interner_independent(
            facts in prop::collection::vec(fact(), 0..6),
            cond in condition(),
        ) {
            let conds = vec![cond];
            let first = eval_fresh(&facts, &conds, &[]);
            // Perturb the second interner's pool so ids differ, then evaluate.
            let second = {
                let mut i = Interner::new();
                for junk in ["zzz", "qqq", "Warm", "Up"] {
                    let _ = i.intern(junk);
                }
                let mut db = Db::empty();
                for f in &facts {
                    db = db.insert_str(&mut i, f).unwrap();
                }
                let compiled: Vec<Cond> = conds
                    .iter()
                    .map(|c| compile_condition(&mut i, c).unwrap())
                    .collect();
                query(&mut i, &db, &compiled, &Bindings::new())
                    .iter()
                    .map(|bind| {
                        bind.iter()
                            .map(|(s, v)| (i.resolve(s).to_owned(), crate::db::val_to_string(&i, v)))
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
            };
            prop_assert_eq!(first, second);
        }

        // Seeding a positive (Match-only) query never widens the result set:
        // binding a variable can only prune branches, never create solutions.
        #[test]
        fn seed_only_narrows_positive_queries(
            facts in prop::collection::vec(fact(), 0..6),
            conds in match_only(),
            seed_var in prop::sample::select(vec!["X", "Y", "Z"]),
            seed_val in constant(),
        ) {
            let empty = eval_fresh(&facts, &conds, &[]);
            let seeded = eval_fresh(&facts, &conds, &[(seed_var.to_owned(), seed_val)]);
            prop_assert!(
                seeded.len() <= empty.len(),
                "seed widened results: {} > {}", seeded.len(), empty.len()
            );
        }

        // `Or` yields exactly the clause-order, within-clause-order union with a
        // local `nub`: evaluating `Or [c0, c1, ...]` equals folding the clauses'
        // own results with first-occurrence dedup. A reorder or a global dedup
        // would break this.
        #[test]
        fn or_is_clause_order_with_local_nub(
            facts in prop::collection::vec(fact(), 0..6),
            c0 in prop::collection::vec(pattern().prop_map(Condition::Match), 1..3),
            c1 in prop::collection::vec(pattern().prop_map(Condition::Match), 1..3),
        ) {
            let mut i = Interner::new();
            let mut db = Db::empty();
            for f in &facts {
                db = db.insert_str(&mut i, f).unwrap();
            }
            let or = compile_condition(&mut i, &Condition::Or(vec![c0.clone(), c1.clone()])).unwrap();
            let via_or = query(&mut i, &db, std::slice::from_ref(&or), &Bindings::new());

            // Expected: nub(query(c0) ++ query(c1)) in order.
            let comp0: Vec<Cond> = c0.iter().map(|c| compile_condition(&mut i, c).unwrap()).collect();
            let comp1: Vec<Cond> = c1.iter().map(|c| compile_condition(&mut i, c).unwrap()).collect();
            let mut expected: Vec<Bindings> = Vec::new();
            for cs in [&comp0, &comp1] {
                for r in query(&mut i, &db, cs, &Bindings::new()) {
                    if !expected.contains(&r) {
                        expected.push(r);
                    }
                }
            }

            // Compare rendered (Bindings equality is id-based but both come from
            // the same interner here, so direct equality is valid too).
            let render = |bs: &[Bindings]| -> Vec<BTreeMap<String, String>> {
                bs.iter()
                    .map(|b| b.iter().map(|(s, v)| (i.resolve(s).to_owned(), crate::db::val_to_string(&i, v))).collect())
                    .collect()
            };
            prop_assert_eq!(render(&via_or), render(&expected));
        }
    }
}
