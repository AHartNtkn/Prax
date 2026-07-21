//! The interpreter: registering practices, discovering an actor's affordances
//! ([`State::possible_actions`]), and applying an action's effects
//! ([`State::perform_action`]) — plus practice spawning, function calls, the
//! drama die, and the retable heir that rebuilds the derived tables in lockstep
//! with any vocabulary change ([`crate::compilepipe::recompile`]).
//!
//! Pure state transformations: the planner (S6) speculatively applies an effect
//! and simply discards the resulting [`State`] (a cheap clone — `Arc` refcount
//! bumps on the interner/defs, structural sharing on the tries). A contradiction
//! surfaces as a queryable `contradiction` fact; an engine-invariant breach
//! (an unseeded `Roll`, a missing clock) panics.
//!
//! Frozen reference: `src/Prax/Engine.hs` (`performCooked`, `possibleActions`,
//! `performAction`, the install setters, `renderText`, `currentTurn`), with the
//! `Prax.Types.emptyState`/`PraxState` shape split into [`State`]/[`Defs`]/
//! [`Runtime`]. `roundBoundary`/expiry-firing/schedule-firing and the
//! planner/analysis tables are S5/S6 and deliberately absent.
//!
//! **The three-tier delta router (ARCHITECTURE, ships in full at S4).** Every
//! insert is classified once and routed:
//!
//! - `!relevant`  → `apply_direct` (db AND view mutated in lockstep — the delta
//!   commutes with closure, so no re-derivation is needed);
//! - monotone     → `apply_grow` (`close_from` continues the closed view with the
//!   one new fact; a `⊥` falls back to a full reclose);
//! - else         → `with_db` → full reclose (`⊥` ⇒ view = db + `contradiction`).
//!
//! Guarded by the stage's flagship net: `view == naive_closure(rules, db)` after
//! every perform. Interner-in-state is settled (S-panel I2): perform/possible_actions
//! take `&mut self`; fork-safety is (1) no cross-fork `Sym` comparison
//! (`Arc::make_mut` clones preserve ids; forks are discarded) and (2) every output
//! and every observable order renders/sorts by name — which is why `expiries` is a
//! `HashMap` (its iteration order is incidental; S5's firing sorts by name).

use std::collections::BTreeMap;
use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;

use crate::compilepipe::{Compiled, Effect, compile_outcome, ground_effect, recompile};
use crate::db::{Bindings, Db, Val, ground, val_to_string};
use crate::derive::{CompiledRule, close, close_from};
use crate::error::WorldError;
use crate::interner::{Interner, Sym};
use crate::path::{CompiledPath, segment_names, segment_names_checked, tokenize};
use crate::planner::{self, Intention, MotiveSignature};
use crate::query::{Cond, Condition, query};
use crate::relevance::{eviction_shadow_names, may_unify_syms};
use crate::rng::{SEED_BOUNDS, roll_step};
use crate::types::{
    Axiom, Character, Desire, Function, Outcome, Practice, ScheduleRule, Want, authored_var_clash,
};

/// A fully grounded, performable action produced by the engine
/// (`Prax.Types.GroundedAction`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundedAction {
    pub practice_id: String,
    pub instance_id: String,
    /// The originating action name (a lookup key).
    pub action_id: String,
    /// Actor + role + query bindings, for grounding the effects.
    pub bindings: Bindings,
    /// Rendered display text.
    pub label: String,
}

/// One top-level effect's execution, as observed by
/// [`State::perform_action_logged`] — the differential oracle's draw log.
/// Reported only for effects whose subtree can draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectStep {
    /// The effect's index in the action's outcome list.
    pub index: usize,
    /// Every `Roll`'s authored odds in this effect's subtree, in traversal order.
    pub odds: Vec<(i64, i64)>,
    /// The stream position before the effect ran.
    pub rng_before: Option<i64>,
    /// The stream position after it ran (a draw ALWAYS advances it).
    pub rng_after: Option<i64>,
    /// Did the effect change the base db? The instrumented hit/miss signal.
    pub changed: bool,
}

/// Every `Roll`'s odds in an effect subtree, in traversal order.
fn roll_odds(e: &Effect) -> Vec<(i64, i64)> {
    let mut out = Vec::new();
    fn go(e: &Effect, out: &mut Vec<(i64, i64)>) {
        match e {
            Effect::Roll(n, d, _, os) => {
                out.push((*n, *d));
                for o in os {
                    go(o, out);
                }
            }
            Effect::ForEach(_, os) => {
                for o in os {
                    go(o, out);
                }
            }
            _ => {}
        }
    }
    go(e, &mut out);
    out
}

/// The authored sources retained (typecheck/persist/diagnostics) plus the
/// compiled forms, rebuilt WHOLESALE by [`recompile`]. Split out of [`State`] so
/// a fork shares it by `Arc` refcount.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Defs {
    practices: BTreeMap<String, Practice>,
    fns: Vec<Function>,
    axioms: Vec<Axiom>,
    characters: Vec<Character>,
    desires: Vec<Desire>,
    schedule: Vec<ScheduleRule>,
    sorts: Vec<(String, Vec<String>)>,
    prediction_scope: Vec<Condition>,
    /// v53 provenance; door-only writes.
    engine_rule_names: Vec<String>,
    compiled: Compiled,
}

/// The mutable runtime: db, the closed view, the clock cursor, and the schedule/
/// expiry/rng bookkeeping. Plain clone; the tries share structurally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runtime {
    db: Db,
    /// The db closed under the axioms. Private; writers are the perform tiers and
    /// [`with_db`](State::with_db).
    view: Db,
    cursor: i32,
    schedule_dues: BTreeMap<String, i64>,
    /// Exact labeled path → due boundary. A `HashMap`, NOT a `BTreeMap`
    /// (S-panel I1): `CompiledPath` gets no `Ord`; iteration order is incidental
    /// and S5's firing sorts by rendered name explicitly.
    expiries: FxHashMap<CompiledPath, i64>,
    rng_seed: Option<i64>,
    /// Each character's standing intention (`Prax.Loop.npcAct`, v35). Runtime state
    /// like `cursor`: starts empty, never derived, never touched by recompile.
    /// `Arc`-wrapped [D-I4] — planner forks never write it, so a fork's clone is a
    /// refcount bump; `npc_act` writes through [`Arc::make_mut`]. Frozen Persist
    /// saves it, so it is persist-shaped now (persistence itself is S9).
    intentions: Arc<BTreeMap<String, Intention>>,
}

/// All state a running simulation needs (`Prax.Types.PraxState`, split three
/// ways). The interner is an owned `Arc` value — the global `unsafePerformIO`
/// pool is gone.
#[derive(Debug, Clone)]
pub struct State {
    interner: Arc<Interner>,
    defs: Arc<Defs>,
    rt: Runtime,
}

impl Default for State {
    fn default() -> State {
        State::new()
    }
}

impl State {
    /// An empty interpreter state (`Prax.Types.emptyState`). The clock is seeded
    /// `turn!0` into BOTH db and view here — construction, not world setup — so
    /// every path has a clock before anything reads it (v44).
    pub fn new() -> State {
        let mut interner = Interner::new();
        let turn0 = tokenize(&mut interner, "turn!0").expect("turn!0 is a valid path");
        let db = Db::empty().insert(&turn0);
        let view = Db::empty().insert(&turn0);
        let mut defs = Defs {
            practices: BTreeMap::new(),
            fns: Vec::new(),
            axioms: Vec::new(),
            characters: Vec::new(),
            desires: Vec::new(),
            schedule: Vec::new(),
            sorts: Vec::new(),
            prediction_scope: Vec::new(),
            engine_rule_names: Vec::new(),
            compiled: Compiled::default(),
        };
        rebuild(&mut interner, &mut defs).expect("empty recompile cannot fail");
        State {
            interner: Arc::new(interner),
            defs: Arc::new(defs),
            rt: Runtime {
                db,
                view,
                cursor: -1,
                schedule_dues: BTreeMap::new(),
                expiries: FxHashMap::default(),
                rng_seed: None,
                intentions: Arc::new(BTreeMap::new()),
            },
        }
    }

    // ---- install API (each ends in recompile, and reclose where db-visible) --

    /// Register a practice and insert its static `data_facts` under
    /// `practiceData.<id>.` (`Prax.Engine.definePractice`). Loud on two actions
    /// sharing a name (a lookup-key collision).
    pub fn define_practice(&mut self, p: Practice) -> Result<(), WorldError> {
        if let Some(dup) = first_duplicate(p.actions.iter().map(|a| a.name.as_str())) {
            return Err(WorldError::DuplicateActionName {
                practice: p.id.clone(),
                action: dup,
            });
        }
        let interner = Arc::make_mut(&mut self.interner);
        let defs = Arc::make_mut(&mut self.defs);
        // dataFacts under practiceData.<id>., through the ordinary insert path.
        let prefix = format!("practiceData.{}.", p.id);
        for f in &p.data_facts {
            let path = tokenize(interner, &format!("{prefix}{f}"))?;
            self.rt.db = self.rt.db.insert(&path);
        }
        defs.practices.insert(p.id.clone(), p);
        rebuild(interner, defs)?;
        reclose(interner, &defs.compiled.rules, &mut self.rt);
        Ok(())
    }

    /// Register several practices in order.
    pub fn define_practices(
        &mut self,
        practices: impl IntoIterator<Item = Practice>,
    ) -> Result<(), WorldError> {
        for p in practices {
            self.define_practice(p)?;
        }
        Ok(())
    }

    /// Register the world's functions (`Prax.Engine.defineFunctions`) — the one
    /// registry. Loud on a duplicate name within this batch OR against the
    /// already-registered set (`Call` resolution is by bare name).
    pub fn define_functions(
        &mut self,
        fns: impl IntoIterator<Item = Function>,
    ) -> Result<(), WorldError> {
        let new: Vec<Function> = fns.into_iter().collect();
        let existing: Vec<&str> = self.defs.fns.iter().map(|f| f.name.as_str()).collect();
        let mut seen: Vec<&str> = Vec::new();
        for f in &new {
            if seen.contains(&f.name.as_str()) || existing.contains(&f.name.as_str()) {
                return Err(WorldError::DuplicateFunctionName {
                    function: f.name.clone(),
                });
            }
            seen.push(&f.name);
        }
        let interner = Arc::make_mut(&mut self.interner);
        let defs = Arc::make_mut(&mut self.defs);
        defs.fns.extend(new);
        rebuild(interner, defs)?;
        Ok(())
    }

    /// The only sanctioned way to change the axioms (`Prax.Engine.setAxioms`):
    /// recompile FIRST (it maintains the cooked rules), THEN reclose (the frozen
    /// ordering — reclose closes under exactly the axiom set just cooked). A
    /// deontic world passes `prax_vocab::deontic::obliged_close(rules)`, so the
    /// □-lifted twins arrive as ordinary members here.
    pub fn set_axioms(&mut self, axioms: Vec<Axiom>) -> Result<(), WorldError> {
        let interner = Arc::make_mut(&mut self.interner);
        let defs = Arc::make_mut(&mut self.defs);
        defs.axioms = axioms;
        rebuild(interner, defs)?;
        reclose(interner, &defs.compiled.rules, &mut self.rt);
        Ok(())
    }

    /// The only sanctioned way to change the desire vocabulary.
    pub fn set_desires(&mut self, desires: Vec<Desire>) -> Result<(), WorldError> {
        let interner = Arc::make_mut(&mut self.interner);
        let defs = Arc::make_mut(&mut self.defs);
        defs.desires = desires;
        rebuild(interner, defs)
    }

    /// The only sanctioned way to change the character roster. A character name
    /// must be a SINGLE path segment (DIV-2): the engine splices it into
    /// `dead.<name>`, `<p>.believes.desires.<name>.…` and every practice
    /// instance path, so a `.`/`!` inside it is a path injection.
    pub fn set_characters(&mut self, characters: Vec<Character>) -> Result<(), WorldError> {
        for c in &characters {
            // UNCHECKED-SPLIT (frozen: none — this guard is DIV-2, additive).
            // `setCharacters` has NO name guard at all and `Prax.Types.character`
            // is a bare record build, so there is no frozen `pathNames` call here
            // to mirror and nothing that can raise. The unguarded split is what
            // this guard wants: `"hall."` splits to two segments and is rejected
            // as the multi-segment name it is, which is the error DIV-2 is about.
            if segment_names(&c.name).len() != 1 {
                return Err(WorldError::MultiSegmentCharacterName {
                    name: c.name.clone(),
                });
            }
        }
        let interner = Arc::make_mut(&mut self.interner);
        let defs = Arc::make_mut(&mut self.defs);
        defs.characters = characters;
        rebuild(interner, defs)
    }

    /// The type-checker's sort declarations (consumed at S9).
    pub fn set_sorts(&mut self, sorts: Vec<(String, Vec<String>)>) -> Result<(), WorldError> {
        let interner = Arc::make_mut(&mut self.interner);
        let defs = Arc::make_mut(&mut self.defs);
        defs.sorts = sorts;
        rebuild(interner, defs)
    }

    /// The conditions the planner predicts over (consumed at S6).
    pub fn set_prediction_scope(&mut self, scope: Vec<Condition>) -> Result<(), WorldError> {
        let interner = Arc::make_mut(&mut self.interner);
        let defs = Arc::make_mut(&mut self.defs);
        defs.prediction_scope = scope;
        rebuild(interner, defs)
    }

    /// The AUTHORING door onto the engine schedule (`Prax.Engine.setSchedule`):
    /// the v40 splice check on every clause (`Prax` reserved; `Actor` reserved
    /// for movers — a schedule rule has no actor), then the shared
    /// [`add_schedule_rules`](State::add_schedule_rules) core.
    pub fn set_schedule(&mut self, rules: Vec<ScheduleRule>) -> Result<(), WorldError> {
        let forbidden = ["Actor".to_owned()];
        for r in &rules {
            for (conds, outs) in &r.body {
                if let Some(v) = authored_var_clash(&forbidden, conds, outs)?.into_iter().next() {
                    return Err(WorldError::ReservedVarClash {
                        context: "Engine.set_schedule".to_owned(),
                        var: v,
                        extra: " (and Actor is reserved for movers; a schedule rule has no actor)"
                            .to_owned(),
                    });
                }
            }
        }
        self.add_schedule_rules(rules)
    }

    /// Install schedule rules, APPENDING to any already registered (both doors
    /// write the one globally-keyed table): single-segment names, positive
    /// periods, no duplicate names ACROSS BOTH DOORS, dues seeded one full period
    /// out (`Prax.Engine.addScheduleRules`). Private; the door calls it too.
    fn add_schedule_rules(&mut self, rules: Vec<ScheduleRule>) -> Result<(), WorldError> {
        for r in &rules {
            // The frozen guard is `filter ((/= 1) . length . pathNames . srName)`
            // (`Engine.hs:291`), and `pathNames` raises on a trailing operator
            // BEFORE the length is taken — so a rule named `"tick."` dies as a
            // malformed sentence, not as a multi-segment name. Both reject; the
            // error identity is the frozen one only through the checked split.
            if segment_names_checked(&r.name)?.len() != 1 {
                return Err(WorldError::MultiSegmentRuleName {
                    name: r.name.clone(),
                });
            }
            if r.period < 1 {
                return Err(WorldError::NonPositivePeriod {
                    name: r.name.clone(),
                });
            }
        }
        let existing: Vec<&str> = self.defs.schedule.iter().map(|r| r.name.as_str()).collect();
        let mut seen: Vec<&str> = Vec::new();
        for r in &rules {
            if seen.contains(&r.name.as_str()) || existing.contains(&r.name.as_str()) {
                return Err(WorldError::DuplicateScheduleRuleName {
                    name: r.name.clone(),
                });
            }
            seen.push(&r.name);
        }
        let now = self.current_turn();
        let interner = Arc::make_mut(&mut self.interner);
        let defs = Arc::make_mut(&mut self.defs);
        for r in &rules {
            self.rt
                .schedule_dues
                .insert(r.name.clone(), now + r.period);
        }
        defs.schedule.extend(rules);
        rebuild(interner, defs)
    }

    /// Seed the drama die (`Prax.Engine.seedDie`): loud on a seed outside the
    /// stream's domain (0 and modulus multiples are fixed points).
    pub fn seed_die(&mut self, seed: i64) -> Result<(), WorldError> {
        let (lo, hi) = SEED_BOUNDS;
        if seed < lo || seed > hi {
            return Err(WorldError::SeedOutOfDomain { seed, lo, hi });
        }
        self.rt.rng_seed = Some(seed);
        Ok(())
    }

    /// Apply a transformation to the fact base and reclose the view — the only
    /// sanctioned way to change the base outside `perform`
    /// (`Prax.Engine.withDb`). The closure builds the new db (interning as needed).
    pub fn with_db(&mut self, f: impl FnOnce(&mut Interner, &Db) -> Db) {
        let interner = Arc::make_mut(&mut self.interner);
        self.rt.db = f(interner, &self.rt.db);
        reclose(interner, &self.defs.compiled.rules, &mut self.rt);
    }

    // ---- perform -------------------------------------------------------------

    /// Apply a single, already-grounded outcome — the public, string-facing
    /// entry, cook-then-run: one engine, two doors (`Prax.Engine.performOutcome`).
    pub fn perform_outcome(&mut self, o: &Outcome) -> Result<(), WorldError> {
        let interner = Arc::make_mut(&mut self.interner);
        let effect = compile_outcome(interner, o)?;
        perform_effect(interner, &self.defs, &mut self.rt, &effect);
        Ok(())
    }

    /// [`State::perform_outcome`] with the outcome GROUNDED to `binds` first
    /// (`Prax.Engine.performOutcome` after `Prax.Engine.groundOutcome`): the
    /// authored template's named variables are substituted, everything else stays
    /// quantified. `VillageSpec`'s percolation pin fires a `witnessed` template
    /// with `Actor` bound and lets the `Witness` quantifier find whoever is there.
    ///
    /// # Errors
    /// The compile-time rejections [`State::perform_outcome`] carries.
    pub fn perform_outcome_grounded(
        &mut self,
        o: &Outcome,
        binds: &[(&str, &str)],
    ) -> Result<(), WorldError> {
        let interner = Arc::make_mut(&mut self.interner);
        let effect = compile_outcome(interner, o)?;
        let mut b = Bindings::new();
        for (k, v) in binds {
            let key = interner.intern(k);
            let val = Val::Sym(interner.intern(v));
            b.insert(key, val);
        }
        let grounded = ground_effect(interner, &effect, &b);
        perform_effect(interner, &self.defs, &mut self.rt, &grounded);
        Ok(())
    }

    /// All actions the named actor can currently perform, across every
    /// instantiated practice and every satisfying binding, evaluated against the
    /// VIEW (`Prax.Engine.possibleActions`). Deterministic: pids in name order,
    /// unify branches in name order.
    pub fn possible_actions(&mut self, actor: &str) -> Vec<GroundedAction> {
        let interner = Arc::make_mut(&mut self.interner);
        possible_actions_impl(interner, &self.defs, &self.rt.view, actor)
    }

    /// Apply every effect of a grounded action, in order (`Prax.Engine.performAction`).
    pub fn perform_action(&mut self, ga: &GroundedAction) {
        self.perform_action_logged(ga, &mut |_| {});
    }

    /// [`State::perform_action`] with the per-effect observation the differential
    /// oracle's DRAW LOG needs ([S7 design S-C5]): `observe` is called once per
    /// top-level effect with what that effect's execution did to the stream and
    /// to the db.
    ///
    /// `Effect::Roll` advances the stream UNCONDITIONALLY, so taken-vs-not leaves
    /// `rng_seed` equal and a draw bug would report as a STATE divergence with no
    /// pointer at the die. The `changed` flag is the instrumented half: a hit
    /// applies its body, a miss does not.
    ///
    /// There is ONE implementation — [`State::perform_action`] delegates here with
    /// a no-op observer — so the instrument can never describe a different
    /// execution than the engine performs.
    pub fn perform_action_logged(&mut self, ga: &GroundedAction, observe: &mut dyn FnMut(EffectStep)) {
        let interner = Arc::make_mut(&mut self.interner);
        let defs = self.defs.as_ref();
        let Some(cp) = defs.compiled.practices.get(&ga.practice_id) else {
            return;
        };
        let Some(ca) = cp.actions.iter().find(|a| a.name == ga.action_id) else {
            return;
        };
        for (index, out) in ca.outs.iter().enumerate() {
            let g = ground_effect(interner, out, &ga.bindings);
            let odds = roll_odds(&g);
            let rng_before = self.rt.rng_seed;
            let facts_before = if odds.is_empty() {
                Vec::new()
            } else {
                self.rt.db.to_labeled_sentences(interner)
            };
            perform_effect(interner, defs, &mut self.rt, &g);
            if !odds.is_empty() {
                observe(EffectStep {
                    index,
                    odds,
                    rng_before,
                    rng_after: self.rt.rng_seed,
                    changed: facts_before != self.rt.db.to_labeled_sentences(interner),
                });
            }
        }
    }

    /// The engine clock's current value — the single `turn` child in the db
    /// (`Prax.Engine.currentTurn`). Loud if absent or not a lone numeric value
    /// (an engine-invariant breach — the clock is seeded and only advanced by
    /// S5's round boundary).
    pub fn current_turn(&mut self) -> i64 {
        let interner = Arc::make_mut(&mut self.interner);
        current_turn(interner, &self.rt.db)
    }

    /// One round boundary (`Prax.Engine.roundBoundary`, v44): advance the clock,
    /// fire due expiries, then due schedule rules in DECLARATION order, re-arming
    /// each a period out from NOW. A pure function of the state; the loop
    /// ([`crate::turn::advance`]) runs it at each rotation wrap.
    ///
    /// **Expiries fire BEFORE rules** for a stated reason (the ghost-observation
    /// law): a fact with lifetime `n` is present rounds `onset..onset+n-1` and GONE
    /// at the boundary — rules-first would let a period-1 rule stamp a belief about
    /// a fact expiring that instant.
    pub fn round_boundary(&mut self) {
        let interner = Arc::make_mut(&mut self.interner);
        round_boundary_impl(interner, &self.defs, &mut self.rt);
    }

    // ---- observation (for the fixture replay and diagnostics) ---------------

    /// The base facts as labeled sentences (`!`/`.` preserved), sorted.
    pub fn labeled_facts(&self) -> Vec<String> {
        self.rt.db.to_labeled_sentences(&self.interner)
    }
    /// The closed view as labeled sentences, sorted.
    pub fn labeled_view(&self) -> Vec<String> {
        self.rt.view.to_labeled_sentences(&self.interner)
    }
    pub fn cursor(&self) -> i32 {
        self.rt.cursor
    }
    /// Set the round-robin cursor. Crate-internal: only [`crate::turn::advance`]
    /// moves the cursor (the loop's one writer); worlds never touch it.
    pub(crate) fn set_cursor(&mut self, cursor: i32) {
        self.rt.cursor = cursor;
    }
    /// The character roster in declaration order (`Prax.Types.characters`).
    pub fn characters(&self) -> &[Character] {
        &self.defs.characters
    }
    /// The names of rules registered through the compiler-level door (v53
    /// provenance) — the reserved-family scan's exemption list (consumed at S9).
    pub fn engine_rule_names(&self) -> &[String] {
        &self.defs.engine_rule_names
    }
    pub fn rng_seed(&self) -> Option<i64> {
        self.rt.rng_seed
    }
    /// The schedule dues (name → next-due boundary).
    pub fn schedule_dues(&self) -> BTreeMap<String, i64> {
        self.rt.schedule_dues.clone()
    }
    /// The one-shot expiry queue, keyed by rendered labeled path (the observable
    /// form; the internal `CompiledPath` key order is incidental).
    pub fn expiries_rendered(&self) -> BTreeMap<String, i64> {
        self.rt
            .expiries
            .iter()
            .map(|(k, v)| (ground(&self.interner, k, &Bindings::new()), *v))
            .collect()
    }
    /// Whether the base db entails a sentence.
    pub fn db_has(&mut self, sentence: &str) -> bool {
        let interner = Arc::make_mut(&mut self.interner);
        let path = tokenize(interner, sentence).expect("valid probe path");
        self.rt.db.exists(interner, &path.segs)
    }
    /// Whether the closed view entails a sentence.
    pub fn view_has(&mut self, sentence: &str) -> bool {
        let interner = Arc::make_mut(&mut self.interner);
        let path = tokenize(interner, sentence).expect("valid probe path");
        self.rt.view.exists(interner, &path.segs)
    }

    /// The naive full-recompute of the view — this state's base closed under its
    /// axioms by the independent [`crate::derive::naive_closure`] oracle (a `⊥`
    /// collapses to `db + contradiction`, exactly as [`reclose`] does). The
    /// flagship ViewInvariant net asserts [`labeled_view`](State::labeled_view)
    /// equals THIS after every perform — the router's soundness guard. Test-only
    /// (`testkit`): shares nothing with the production loop but the substrate.
    #[cfg(feature = "testkit")]
    pub fn naive_view(&self) -> Vec<String> {
        let mut interner = (*self.interner).clone();
        match crate::derive::naive_closure(&mut interner, &self.defs.compiled.rules, &self.rt.db) {
            Ok(v) => v.to_labeled_sentences(&interner),
            Err(_) => {
                let c = tokenize(&mut interner, "contradiction").expect("literal");
                self.rt.db.insert(&c).to_labeled_sentences(&interner)
            }
        }
    }
}

/// The compiler-level door onto engine internals (v46/v53). NOT re-exported by
/// any authoring prelude — cross-crate sealing is by convention plus the S9 `.rs`
/// gate scanner (worlds must not import `door`); Rust visibility cannot express
/// "prax-script only", so it is stated, not hidden.
pub mod door {
    use super::{Arc, ScheduleRule, State, WorldError};

    /// Register compiler-generated schedule rules (the `Prax.Script` story rule),
    /// which authoring code could not — this door omits ONLY the v40 splice guard
    /// (its caller is compiler-level code, squarely inside v45's threat model).
    /// Records the installed names in `engine_rule_names` (v53 provenance) — but
    /// only AFTER `add_schedule_rules`'s guards pass, so a duplicate is never
    /// silently exempted (`Prax.Engine.registerEngineRules`).
    pub fn register_engine_rules(st: &mut State, rules: Vec<ScheduleRule>) -> Result<(), WorldError> {
        let names: Vec<String> = rules.iter().map(|r| r.name.clone()).collect();
        st.add_schedule_rules(rules)?;
        Arc::make_mut(&mut st.defs).engine_rule_names.extend(names);
        Ok(())
    }
}

// ---- the perform core (free functions over the split state) ----------------

// ---- planner seams (S6) -----------------------------------------------------

impl Defs {
    pub(crate) fn compiled(&self) -> &Compiled {
        &self.compiled
    }
    pub(crate) fn characters(&self) -> &[Character] {
        &self.characters
    }
    pub(crate) fn desires(&self) -> &[Desire] {
        &self.desires
    }
}

impl Runtime {
    pub(crate) fn view(&self) -> &Db {
        &self.view
    }
    pub(crate) fn db(&self) -> &Db {
        &self.db
    }
    pub(crate) fn set_intention(&mut self, name: String, intent: Intention) {
        Arc::make_mut(&mut self.intentions).insert(name, intent);
    }
}

impl State {
    /// The actor's best action (`Prax.Planner.pickAction`).
    pub fn pick_action(&mut self, depth: i32, actor: &Character) -> Option<GroundedAction> {
        let State { interner, defs, rt } = self;
        let interner = Arc::make_mut(interner);
        planner::pick_action(interner, defs.as_ref(), rt, depth, actor)
    }
    /// The full scored candidate table in NATIVE result order
    /// (`Prax.Planner.scoreActions`) — the corpus/pin observable [D-C1].
    pub fn score_actions(&mut self, depth: i32, actor: &Character) -> Vec<(GroundedAction, f64)> {
        let State { interner, defs, rt } = self;
        let interner = Arc::make_mut(interner);
        planner::score_actions(interner, defs.as_ref(), rt, depth, actor)
    }
    /// The predictor's guess at the mover's motivated next move
    /// (`Prax.Planner.predictMove`).
    pub fn predict_move(
        &mut self,
        predictor: &Character,
        mover: &Character,
    ) -> Option<GroundedAction> {
        let State { interner, defs, rt } = self;
        let interner = Arc::make_mut(interner);
        planner::predict_move(interner, defs.as_ref(), rt, predictor, mover)
    }
    /// The character's current motive signature (`Prax.Planner.motiveSignature`).
    pub fn motive_signature(&mut self, actor: &Character) -> MotiveSignature {
        let State { interner, defs, rt } = self;
        let interner = Arc::make_mut(interner);
        planner::motive_signature(interner, defs.as_ref(), rt, actor)
    }
    /// The actions a character may actually take (`Prax.Planner.candidateActions`).
    pub fn candidate_actions(&mut self, actor: &Character) -> Vec<GroundedAction> {
        let State { interner, defs, rt } = self;
        let interner = Arc::make_mut(interner);
        planner::candidate_actions(interner, defs.as_ref(), rt, actor)
    }
    /// The pair-grounded read anchors of a prediction (`Prax.Relevance.moverReadAnchors`)
    /// — exposed for the RelevanceSpec/reuse-anchor pins.
    pub fn mover_read_anchors_of(&mut self, actor: &str, mover: &str) -> Vec<SmallVec<[Sym; 6]>> {
        let State { interner, defs, .. } = self;
        let interner = Arc::make_mut(interner);
        let c = defs.compiled();
        crate::relevance::mover_read_anchors(
            interner,
            &c.scope,
            &c.practices,
            &c.fns,
            &c.desires,
            actor,
            mover,
        )
    }
    /// The insert/delete anchor families a grounded action's outcomes can touch
    /// (`Prax.Engine.groundedDeltaAnchors`) — exposed for the EngineSpec pins.
    pub fn grounded_delta_anchors_of(&mut self, ga: &GroundedAction) -> Option<Vec<SmallVec<[Sym; 6]>>> {
        let State { interner, defs, .. } = self;
        let interner = Arc::make_mut(interner);
        grounded_delta_anchors(interner, defs.as_ref(), ga)
    }
    /// Is `mover` inside `actor`'s PREDICTION SCOPE — the world's own scope
    /// template, grounded to the pair and queried against the view
    /// (`Prax.Planner.inScope`)? The one live observable of a world's
    /// `predictionScope`: the planner's round-walk consults exactly this before
    /// it will call [`State::predict_move`] at all, so a world whose scope is
    /// mis-transcribed predicts for the wrong pairs.
    pub fn in_prediction_scope(&mut self, actor: &Character, mover: &Character) -> bool {
        let State { interner, defs, rt } = self;
        let interner = Arc::make_mut(interner);
        planner::in_scope(interner, defs.as_ref(), rt, actor, mover)
    }
    /// The improvable-desire table (`Prax.Relevance.improvableDesires`).
    pub fn improvables(&self) -> &[String] {
        &self.defs.compiled.improvables
    }
    /// The improvable-desire table RECOMPUTED from this state's own compiled
    /// tables, rather than read off the field [`State::improvables`] caches
    /// (`Prax.Relevance.improvableDesires` applied to the state, which the frozen
    /// `RelevanceSpec` compares against the field to prove the field is not
    /// stale). The two must always be equal; a difference is a rebuild bug.
    pub fn improvable_desires_recomputed(&mut self) -> Vec<String> {
        let State { interner, defs, .. } = self;
        let interner = Arc::make_mut(interner);
        let comp = defs.compiled();
        let fn_pool = crate::relevance::cooked_fn_pool(&comp.fns);
        crate::relevance::improvable_desires(
            interner,
            &comp.practices,
            &fn_pool,
            &comp.rules,
            &comp.desires,
            defs.desires(),
        )
    }
    /// Everything this world can EVER contain, as pattern anchors
    /// (`Prax.Relevance.producibleAtoms`) — the dead-condition lint's pool.
    /// `None` = wild (an unresolvable `Call` in a scanned outcome). Computed on a
    /// CLONE of the interner so the check is side-effect-free (the only symbol it
    /// would mint is the idempotent `PraxEvicted` wildcard, and no lint pattern
    /// ever contains it), honouring the `&State` charter of the checker.
    pub(crate) fn producible_atoms(&self) -> Option<Vec<SmallVec<[Sym; 6]>>> {
        let mut interner = (*self.interner).clone();
        let comp = self.defs.compiled();
        let fn_pool = crate::relevance::cooked_fn_pool(&comp.fns);
        crate::relevance::producible_atoms(
            &mut interner,
            &comp.practices,
            &fn_pool,
            &comp.schedule,
            &comp.rules,
            self.rt.db(),
        )
    }
    /// A private clone of the interner for a side-effect-free analysis pass (the
    /// checker's deontic-invokability census interns and resolves without touching
    /// the real pool).
    pub(crate) fn interner_snapshot(&self) -> Interner {
        (*self.interner).clone()
    }
    /// The compiled tables the checker's dead-condition lint reads.
    pub(crate) fn compiled_tables(&self) -> &Compiled {
        self.defs.compiled()
    }
    /// The base fact database (the inspector queries it directly).
    pub(crate) fn base_db(&self) -> &Db {
        &self.rt.db
    }
    /// Would inserting or retracting `sentence` disturb the derived closure
    /// (`Prax.Relevance.relevantDelta`)? False is the FAST PATH the engine takes:
    /// the delta commutes with closure, so no reclose is needed. The eviction
    /// shadow of an `!` insert counts on the delete side.
    ///
    /// # Errors
    /// [`WorldError::TrailingOperator`] / [`WorldError::PathTooLong`] from the
    /// tokenizer.
    pub fn relevant_delta(&mut self, sentence: &str) -> Result<bool, WorldError> {
        let State { interner, defs, .. } = self;
        let interner = Arc::make_mut(interner);
        let path = tokenize(interner, sentence)?;
        let shadows = eviction_shadow_names(interner, &path);
        Ok(relevant_names(
            &path.segs,
            &shadows,
            &defs.compiled.footprint,
        ))
    }
    /// Is this world continuation-safe — do its axioms admit the grow tier at all
    /// (`Prax.Relevance.contMonotone`)?
    pub fn cont_monotone(&self) -> bool {
        self.defs.compiled.cont_monotone
    }
    /// May inserting `sentence` take the continuation (grow) tier rather than a
    /// full reclose (`Prax.Engine.monotoneToks`, the frozen `monotoneInsert`): the
    /// world is continuation-safe, the insert evicts nothing (`!`-free), and it
    /// unifies no negated body pattern (defeats nothing).
    ///
    /// # Errors
    /// [`WorldError::TrailingOperator`] / [`WorldError::PathTooLong`] from the
    /// tokenizer.
    pub fn monotone_insert(&mut self, sentence: &str) -> Result<bool, WorldError> {
        let State { interner, defs, .. } = self;
        let interner = Arc::make_mut(interner);
        let path = tokenize(interner, sentence)?;
        Ok(monotone_toks(
            &path,
            defs.compiled.cont_monotone,
            &defs.compiled.neg_footprint,
        ))
    }
    /// Test-only: tokenize a sentence through this state's interner (the frozen
    /// RelevanceSpec's `map intern (pathNames s)` — so an anchor-membership check
    /// interns the probe in the same id space).
    #[cfg(test)]
    pub(crate) fn intern_segs(&mut self, s: &str) -> SmallVec<[Sym; 6]> {
        let interner = Arc::make_mut(&mut self.interner);
        tokenize(interner, s).expect("probe path").segs
    }
    /// Does an authored conjunction have any solution against the closed view,
    /// with `binds` first SUBSTITUTED into it (`Prax.Query.groundCondition` then
    /// `query`, exactly the mechanism `Prax.Planner.inScope` uses)? The scope
    /// fragments' one live observable, for the SightSpec window pin.
    #[cfg(test)]
    pub(crate) fn conditions_hold(&mut self, conds: &[Condition], binds: &[(&str, &str)]) -> bool {
        let State { interner, rt, .. } = self;
        let interner = Arc::make_mut(interner);
        let mut b = Bindings::new();
        for (k, v) in binds {
            let key = interner.intern(k);
            let val = interner.intern(v);
            b.insert(key, Val::Sym(val));
        }
        let grounded: Vec<Cond> = conds
            .iter()
            .map(|c| {
                let g = crate::query::ground_condition(interner, &b, c).expect("groundable");
                crate::query::compile_condition(interner, &g).expect("compilable")
            })
            .collect();
        !query(interner, rt.view(), &grounded, &Bindings::new()).is_empty()
    }

    /// The compiled want table rendered by name (`Prax.Types.cookedWants`):
    /// character → one entry per `charWants` want, in that list's own order,
    /// each want's conditions rendered. Exposed for the MindsSpec retable pin —
    /// the table itself is compile-pipeline private.
    #[cfg(test)]
    pub(crate) fn compiled_wants_rendered(&self) -> BTreeMap<String, Vec<Vec<String>>> {
        self.defs
            .compiled
            .wants
            .iter()
            .map(|(k, ws)| {
                (
                    k.clone(),
                    ws.iter()
                        .map(|cs| cs.iter().map(|c| self.render_cond(c)).collect())
                        .collect(),
                )
            })
            .collect()
    }

    /// The compiled desire table rendered by name (`Prax.Types.cookedDesires`):
    /// desire name → its `Owner`-templated conditions, cooked once and
    /// independent of which characters hold it.
    #[cfg(test)]
    pub(crate) fn compiled_desires_rendered(&self) -> BTreeMap<String, Vec<String>> {
        self.defs
            .compiled
            .desires
            .iter()
            .map(|(k, cs)| (k.clone(), cs.iter().map(|c| self.render_cond(c)).collect()))
            .collect()
    }

    /// One compiled condition rendered by name. Total over the pattern-bearing
    /// constructors the want/desire pins use; PANICS on anything else rather
    /// than emitting a Debug blob with raw interner ids in it — a renderer that
    /// silently degrades would make a divergence unreadable.
    #[cfg(test)]
    fn render_cond(&self, c: &Cond) -> String {
        match c {
            Cond::Match(p) => format!("Match {}", self.render_segs(p)),
            Cond::Not(p) => format!("Not {}", self.render_segs(p)),
            other => panic!("render_cond: extend me for {other:?}"),
        }
    }

    /// The integer utility of the current view to an ARBITRARY want list —
    /// `Prax.Planner.evaluate` itself, which cooks the wants it is handed rather
    /// than reading a precompiled table. The frozen `evaluate` is partial in
    /// exactly one way (`cookCondition` rejects a nested `Subquery`), so this is
    /// fallible where the frozen raises. `ProjectSpec` scores an endeavor's
    /// pursuit want directly, from `prax-vocab`, which is why this is public.
    ///
    /// # Errors
    /// [`WorldError::NestedSubquery`] from [`crate::query::compile_condition`].
    pub fn evaluate_wants(&mut self, wants: &[Want]) -> Result<i64, WorldError> {
        let State { interner, rt, .. } = self;
        let interner = Arc::make_mut(interner);
        let mut model: Vec<(Vec<Cond>, i32)> = Vec::with_capacity(wants.len());
        for w in wants {
            let mut cs = Vec::with_capacity(w.when.len());
            for c in &w.when {
                cs.push(crate::query::compile_condition(interner, c)?);
            }
            model.push((cs, w.utility));
        }
        Ok(planner::evaluate_compiled(interner, rt.view(), &model))
    }

    /// The integer utility of the current view to a character's own wants —
    /// `Prax.Planner.evaluate` over `Prax.Minds.selfWants`, which in the one
    /// compiled representation is exactly `evaluate_compiled` over
    /// `cooked_self_wants`. Exposed for the PlannerSpec `evaluate` pin and for
    /// `PersonaSpec`/`ConfessionSpec`'s per-mark conscience arithmetic, which
    /// lives in `prax-vocab` and so needs it across the crate boundary.
    pub fn evaluate_self_wants(&mut self, c: &Character) -> i64 {
        let State { interner, defs, rt } = self;
        let interner = Arc::make_mut(interner);
        let comp = defs.compiled();
        let model =
            crate::minds::cooked_self_wants(interner, &comp.wants, &comp.desires, defs.desires(), c);
        planner::evaluate_compiled(interner, rt.view(), &model)
    }
    /// A named desire's dead-now recipe (`Prax.Relevance.livenessOf`).
    #[cfg(test)]
    pub(crate) fn liveness_of(&self, name: &str) -> Option<&crate::relevance::Liveness> {
        self.defs.compiled.liveness.get(name)
    }
    /// The dead-now recipe table rendered by name — the planner corpus's
    /// comparison channel (`Prax.Relevance.livenessOf`). Each entry is the
    /// variant tag plus, for `GateCheck`, each gate's conjunct patterns as
    /// dot-joined segment names. PANICS on a gate shape `liveness_of` does not
    /// build (it emits single-`Match` gates only): a silently skipped conjunct
    /// would hide a real divergence.
    pub fn liveness_rendered(&self) -> BTreeMap<String, (String, Vec<Vec<String>>)> {
        self.render_liveness(&self.defs.compiled.liveness)
    }

    /// The dead-now recipe table RECOMPUTED from this state's own compiled
    /// tables, rendered exactly as [`State::liveness_rendered`] renders the
    /// cached field (`Prax.Relevance.livenessOf` applied to the state, which the
    /// frozen `RelevanceSpec` compares against the field to prove the field is
    /// not stale — the sibling of [`State::improvable_desires_recomputed`]). The
    /// two must always be equal; a difference is a rebuild bug.
    pub fn liveness_recomputed(&mut self) -> BTreeMap<String, (String, Vec<Vec<String>>)> {
        let State { interner, defs, .. } = self;
        let interner = Arc::make_mut(interner);
        let comp = defs.compiled();
        let fn_pool = crate::relevance::cooked_fn_pool(&comp.fns);
        let tbl = crate::relevance::liveness_of(
            interner,
            &comp.practices,
            &fn_pool,
            &comp.rules,
            &comp.desires,
            defs.desires(),
        );
        self.render_liveness(&tbl)
    }

    fn render_liveness(
        &self,
        liveness: &BTreeMap<String, crate::relevance::Liveness>,
    ) -> BTreeMap<String, (String, Vec<Vec<String>>)> {
        let mut out = BTreeMap::new();
        for (name, l) in liveness {
            let entry = match l {
                crate::relevance::Liveness::FloorCheck => ("FloorCheck".to_owned(), Vec::new()),
                crate::relevance::Liveness::AlwaysLive => ("AlwaysLive".to_owned(), Vec::new()),
                crate::relevance::Liveness::GateCheck(gates) => (
                    "GateCheck".to_owned(),
                    gates
                        .iter()
                        .map(|g| g.iter().map(|c| self.render_gate_cond(c)).collect())
                        .collect(),
                ),
            };
            out.insert(name.clone(), entry);
        }
        out
    }

    fn render_gate_cond(&self, c: &Cond) -> String {
        match c {
            Cond::Match(segs) => self.render_segs(segs),
            other => panic!("liveness gate carries an unexpected condition: {other:?}"),
        }
    }

    /// A pattern rendered by name: interned segment names joined by `.` — the
    /// one rendering the corpus compares (compiled conditions carry no `!`/`.`
    /// punctuation, so none is rendered).
    pub fn render_segs(&self, segs: &[Sym]) -> String {
        segs.iter()
            .map(|s| self.interner.resolve(*s))
            .collect::<Vec<_>>()
            .join(".")
    }

    /// The bearing table (`Prax.Relevance.bearingTemplates`) — per character,
    /// the affordance templates whose outcomes could touch their wants.
    pub fn cares_about_table(&self) -> &BTreeMap<String, Vec<String>> {
        &self.defs.compiled.cares_about
    }

    /// [`State::mover_read_anchors_of`], rendered by name.
    pub fn mover_read_anchor_names(&mut self, actor: &str, mover: &str) -> Vec<String> {
        let anchors = self.mover_read_anchors_of(actor, mover);
        anchors.iter().map(|a| self.render_segs(a)).collect()
    }

    // ---- the authored-source read surface (the S7 `worldshape` gate) --------
    //
    // Worlds are authored DATA; a mis-transcribed label, swapped role or weight
    // typo presents at trace time exactly like an engine divergence. The
    // comparator's `worldshape` emits these, canonically encoded, and gates
    // every trace behind a green shape diff (S7 design §2).

    /// The authored practices, in practice-id order.
    pub fn practice_defs(&self) -> &BTreeMap<String, Practice> {
        &self.defs.practices
    }
    /// The authored function registry, in declaration order.
    pub fn functions_src(&self) -> &[Function] {
        &self.defs.fns
    }
    /// The axiom list the world DECLARED (closure included), in declaration order.
    pub fn axioms_src(&self) -> &[Axiom] {
        &self.defs.axioms
    }
    /// The vocabulary of nameable desires, in declaration order.
    pub fn desires_src(&self) -> &[Desire] {
        &self.defs.desires
    }
    /// The authored schedule rules, in DECLARATION order (which is firing order).
    pub fn schedule_src(&self) -> &[ScheduleRule] {
        &self.defs.schedule
    }
    /// The typechecker's sort table.
    pub fn sorts(&self) -> &[(String, Vec<String>)] {
        &self.defs.sorts
    }
    /// The conditions the planner predicts over.
    pub fn prediction_scope(&self) -> &[Condition] {
        &self.defs.prediction_scope
    }
    /// Every axiom head anchor, rendered by name, in table order.
    pub fn axiom_head_names(&self) -> Vec<String> {
        self.defs
            .compiled
            .axiom_heads
            .iter()
            .map(|h| self.render_segs(h))
            .collect()
    }
    /// Every axiom-footprint anchor, rendered by name, in table order (the
    /// `footprint` field the v41 rewrite must reproduce in emission order —
    /// `Prax.AnalysisTableSpec`'s `footprint:` rows). No differential localizer
    /// covers this field (`worldshape` emits `axiom_heads` alone, [P1]); the
    /// native AnalysisTable pin is its sole net.
    pub fn footprint_names(&self) -> Vec<String> {
        self.defs
            .compiled
            .footprint
            .iter()
            .map(|p| self.render_segs(p))
            .collect()
    }
    /// Every negative-footprint anchor (`negFootprint:` rows), rendered by name,
    /// in table order. Same sole-net status as [`State::footprint_names`].
    pub fn neg_footprint_names(&self) -> Vec<String> {
        self.defs
            .compiled
            .neg_footprint
            .iter()
            .map(|p| self.render_segs(p))
            .collect()
    }
    /// The characters with no `dead.<name>` mark in the BASE db
    /// (`Prax.Types.livingCharacters`) — the randtrace walk's extinction test.
    pub fn living_characters(&mut self) -> Vec<Character> {
        let names: Vec<String> = self.defs.characters.iter().map(|c| c.name.clone()).collect();
        let alive: Vec<bool> = names
            .iter()
            .map(|n| !self.db_has(&crate::types::dead_sentence(n)))
            .collect();
        self.defs
            .characters
            .iter()
            .zip(alive)
            .filter(|(_, a)| *a)
            .map(|(c, _)| c.clone())
            .collect()
    }
    /// How many cast members have no `dead.<name>` mark in the BASE db — the
    /// extinction test's count, without materialising and cloning the surviving
    /// [`Character`] structs [`living_characters`](State::living_characters) would
    /// (the random walk's per-turn stop check needs only this number).
    pub fn living_character_count(&mut self) -> usize {
        let names: Vec<String> = self.defs.characters.iter().map(|c| c.name.clone()).collect();
        names
            .iter()
            .filter(|n| !self.db_has(&crate::types::dead_sentence(n)))
            .count()
    }
    /// A binding row rendered by name, in NAME order (never the run-dependent
    /// `Sym` id order) — the oracle's `bindingJSON`.
    pub fn render_bindings(&self, b: &Bindings) -> BTreeMap<String, String> {
        b.iter()
            .map(|(k, v)| {
                (
                    self.interner.resolve(k).to_owned(),
                    crate::db::val_to_string(&self.interner, v),
                )
            })
            .collect()
    }

    /// The child keys of a path in the BASE db, in name order
    /// (`Prax.Db.childKeys`) — the randtrace walk's `ending.E` probe.
    pub fn db_child_keys(&mut self, sentence: &str) -> Vec<String> {
        let interner = Arc::make_mut(&mut self.interner);
        let path = tokenize(interner, sentence).expect("a valid probe path");
        self.rt.db.child_keys(interner, &path.segs)
    }

    /// A character's standing intention, if any (a clone off the runtime map).
    pub fn intention_of(&self, name: &str) -> Option<Intention> {
        self.rt.intentions.get(name).cloned()
    }
    /// Store a character's standing intention (`Prax.Loop.npcAct`).
    pub fn set_intention(&mut self, name: String, intent: Intention) {
        self.rt.set_intention(name, intent);
    }
    /// The standing-intention map (read-only) — for the loop pins that graft it.
    pub fn intentions_map(&self) -> &BTreeMap<String, Intention> {
        &self.rt.intentions
    }
    /// Replace the standing-intention map wholesale — the loop pins' "keep the
    /// stored intention, rewind the world" graft.
    pub fn with_intentions(&mut self, intentions: BTreeMap<String, Intention>) {
        self.rt.intentions = Arc::new(intentions);
    }

    // ---- persist support (`Prax.Persist`, S9) -------------------------------
    //
    // Read/write access the serializer needs. Every `Sym` crosses the file
    // boundary by NAME and is re-interned on load ([S-I1] re-intern hazard):
    // `interner` resolves outbound, `intern_name` mints inbound, so a reloaded
    // `GroundedAction`'s bindings compare content-canonically against freshly
    // computed candidates under the reloaded world's monotonic interner.

    /// The interner, for resolving a saved value's symbols to names on write.
    pub(crate) fn interner(&self) -> &Interner {
        &self.interner
    }
    /// Intern a name into THIS state's interner (re-interning a saved symbol on
    /// load), through `Arc::make_mut` like every other writer.
    pub(crate) fn intern_name(&mut self, name: &str) -> Sym {
        Arc::make_mut(&mut self.interner).intern(name)
    }
    /// The declared schedule rules, for validating a reloaded due's rule name
    /// (`Prax.Persist`'s `srName` re-association check).
    pub(crate) fn schedule_rules(&self) -> &[ScheduleRule] {
        &self.defs.schedule
    }
    /// Set the round-robin cursor on load (persist replaces the whole runtime).
    pub(crate) fn set_cursor_loaded(&mut self, cursor: i32) {
        self.rt.cursor = cursor;
    }
    /// Set the die's stream position raw on load — NOT [`State::seed_die`],
    /// which re-validates `SEED_BOUNDS`; a saved position is already a legal
    /// stream state and must round-trip exactly, including `None` (unseeded).
    pub(crate) fn set_rng_seed_loaded(&mut self, seed: Option<i64>) {
        self.rt.rng_seed = seed;
    }
    /// Replace the schedule dues wholesale on load.
    pub(crate) fn replace_schedule_dues(&mut self, dues: BTreeMap<String, i64>) {
        self.rt.schedule_dues = dues;
    }
    /// Replace the expiry queue on load from labeled sentences (`!`/`.` bearing),
    /// re-tokenizing each into its `CompiledPath` key. Loud on a malformed path.
    pub(crate) fn replace_expiries_from_labeled(
        &mut self,
        entries: &[(String, i64)],
    ) -> Result<(), WorldError> {
        let interner = Arc::make_mut(&mut self.interner);
        let mut map = FxHashMap::default();
        for (sentence, turn) in entries {
            let path = tokenize(interner, sentence)?;
            map.insert(path, *turn);
        }
        self.rt.expiries = map;
        Ok(())
    }
    /// The expiry queue as labeled sentences (`!`/`.` preserved) paired with
    /// their due boundary — the save's `expiry` lines. Ordering is incidental
    /// (`CompiledPath` has no `Ord`); the round trip is set-equal.
    pub(crate) fn expiries_labeled(&self) -> Vec<(String, i64)> {
        self.rt
            .expiries
            .iter()
            .map(|(k, v)| (ground(&self.interner, k, &Bindings::new()), *v))
            .collect()
    }
}

/// Fork-perform: apply a grounded action's effects to a runtime in place
/// (`Prax.Engine.performAction`), the planner's imagined-move primitive. The
/// caller clones the runtime first; the single shared interner mints any new
/// names in one id space, so the delta-vs-read may-unify the reuse gate runs is a
/// same-lineage compare — the frozen "cross-lineage" hazard [S-C1] is absent by
/// construction.
pub(crate) fn perform_action_on(
    interner: &mut Interner,
    defs: &Defs,
    rt: &mut Runtime,
    ga: &GroundedAction,
) {
    let Some(cp) = defs.compiled.practices.get(&ga.practice_id) else {
        return;
    };
    let Some(ca) = cp.actions.iter().find(|a| a.name == ga.action_id) else {
        return;
    };
    for out in &ca.outs {
        let g = ground_effect(interner, out, &ga.bindings);
        perform_effect(interner, defs, rt, &g);
    }
}

/// The insert/delete anchor families one grounded action's outcomes can touch
/// (`Prax.Engine.groundedDeltaAnchors`), bounded statically per call by walking
/// the same cooked outcomes `performAction` executes. `None` when the effects
/// cannot be bounded: an unresolvable `Call`; an insert headed by the literal
/// `practice`; an insert headed by a variable that is not a safe `ForEach` binder
/// (it could ground to `practice` and spawn); or a path with no literal segment
/// at all (evidence-free for [`may_unify_syms`]). The planner treats `None` as
/// opaque: no reuse at or below the node.
pub(crate) fn grounded_delta_anchors(
    interner: &mut Interner,
    defs: &Defs,
    ga: &GroundedAction,
) -> Option<Vec<SmallVec<[Sym; 6]>>> {
    let cp = defs.compiled.practices.get(&ga.practice_id)?;
    let ca = cp.actions.iter().find(|a| a.name == ga.action_id)?;
    let grounded: Vec<Effect> = ca
        .outs
        .iter()
        .map(|o| ground_effect(interner, o, &ga.bindings))
        .collect();
    outcome_delta_anchors(interner, defs, &[], &grounded)
}

fn outcome_delta_anchors(
    interner: &mut Interner,
    defs: &Defs,
    visited: &[&str],
    outs: &[Effect],
) -> Option<Vec<SmallVec<[Sym; 6]>>> {
    delta_go_prime(interner, defs, visited, &FxHashSet::default(), outs)
}

fn delta_go_prime(
    interner: &mut Interner,
    defs: &Defs,
    visited: &[&str],
    safe: &FxHashSet<Sym>,
    outs: &[Effect],
) -> Option<Vec<SmallVec<[Sym; 6]>>> {
    let mut all = Vec::new();
    for o in outs {
        all.extend(delta_go(interner, defs, visited, safe, o)?);
    }
    Some(all)
}

fn delta_go(
    interner: &mut Interner,
    defs: &Defs,
    visited: &[&str],
    safe: &FxHashSet<Sym>,
    o: &Effect,
) -> Option<Vec<SmallVec<[Sym; 6]>>> {
    match o {
        Effect::Insert(p) => {
            let names: SmallVec<[Sym; 6]> = p.segs.clone();
            if might_spawn(interner, safe, &names) || unanchored(&names) {
                None
            } else {
                let mut v = vec![names];
                v.extend(eviction_shadow_names(interner, p));
                Some(v)
            }
        }
        // The deferred retract is environment: same anchors as the bare insert.
        Effect::InsertFor(_, p) => delta_go(interner, defs, visited, safe, &Effect::Insert(p.clone())),
        Effect::Delete(p) => {
            let names: SmallVec<[Sym; 6]> = p.segs.clone();
            if unanchored(&names) {
                None
            } else {
                let mut v = vec![names];
                v.extend(eviction_shadow_names(interner, p));
                Some(v)
            }
        }
        // The body may fire (a roll may hit); its anchors count as a ForEach's, and
        // the roll's guard binds like a ForEach's.
        Effect::ForEach(conds, os) | Effect::Roll(_, _, conds, os) => {
            let mut safe2 = safe.clone();
            safe2.extend(safe_binders(conds));
            delta_go_prime(interner, defs, visited, &safe2, os)
        }
        Effect::Call(fn_, args) => {
            if visited.contains(&fn_.as_str()) {
                Some(Vec::new())
            } else {
                match defs.compiled.fns.get(fn_) {
                    None => None,
                    Some((params, cases)) => {
                        let mut b = Bindings::new();
                        for (&pn, &av) in params.iter().zip(args.iter()) {
                            b.insert(pn, Val::Sym(av));
                        }
                        let mut vv: Vec<&str> = visited.to_vec();
                        vv.push(fn_.as_str());
                        let mut all = Vec::new();
                        for (_cs, os) in cases {
                            let grounded: Vec<Effect> =
                                os.iter().map(|e| ground_effect(interner, e, &b)).collect();
                            all.extend(outcome_delta_anchors(interner, defs, &vv, &grounded)?);
                        }
                        Some(all)
                    }
                }
            }
        }
    }
}

/// Could this insert head reach `spawnedInstanceNames` (`groundedDeltaAnchors`'s
/// `mightSpawn`): a first-position variable that is not a safe binder, or the
/// literal `practice`.
fn might_spawn(interner: &mut Interner, safe: &FxHashSet<Sym>, names: &[Sym]) -> bool {
    match names.first() {
        None => false,
        Some(&n) => {
            if n.is_var() {
                !safe.contains(&n)
            } else {
                n == interner.intern("practice")
            }
        }
    }
}

/// A path with no literal segment carries no anchor evidence at all
/// ([`may_unify_syms`] would clear it against every read pattern), so bounding it
/// would license unsound reuse — opaque instead.
fn unanchored(names: &[Sym]) -> bool {
    names.iter().all(|s| s.is_var())
}

/// The `ForEach` binders that provably cannot take the value `practice`
/// (`Prax.Engine.safeBinders`): variables bound at a NON-FIRST position of a
/// top-level positive `Match` guard and never occurring at the first position of
/// any such guard.
fn safe_binders(conds: &[Cond]) -> FxHashSet<Sym> {
    let mut bound_deep = FxHashSet::default();
    let mut first_pos = FxHashSet::default();
    for c in conds {
        if let Cond::Match(p) = c {
            for &v in p.iter().skip(1) {
                if v.is_var() {
                    bound_deep.insert(v);
                }
            }
            if let Some(&v) = p.first()
                && v.is_var()
            {
                first_pos.insert(v);
            }
        }
    }
    bound_deep.difference(&first_pos).copied().collect()
}

/// Apply a single already-grounded [`Effect`] — case-for-case with the frozen
/// `performCooked`. The state is split (interner mutated, defs read-only during
/// perform, runtime mutated), so this is a free function rather than a method.
fn perform_effect(interner: &mut Interner, defs: &Defs, rt: &mut Runtime, e: &Effect) {
    match e {
        Effect::Delete(path) => {
            let names = &path.segs;
            // A subtree delete takes its descendants' pending timers (v44):
            // purge every expiry entry AT OR UNDER the deleted path (by name
            // prefix), BEFORE the retract.
            rt.expiries.retain(|k, _| !is_prefix(names, &k.segs));
            let shadows = eviction_shadow_names(interner, path);
            if relevant_names(names, &shadows, &defs.compiled.footprint) {
                rt.db = rt.db.retract(names);
                reclose(interner, &defs.compiled.rules, rt);
            } else {
                rt.db = rt.db.retract(names);
                rt.view = rt.view.retract(names);
            }
        }
        Effect::Insert(path) => {
            // A bare insert CANCELS any pending expiry on the exact path (v44's
            // supersession law).
            rt.expiries.remove(path);
            let names = path.segs.clone();
            // The spawn decision reads the PRE-insert BASE (existedBefore), so it
            // is taken before the route mutates the db.
            let spawn = spawned_instance(interner, defs, &rt.db, &names);
            let shadows = eviction_shadow_names(interner, path);
            if !relevant_names(&names, &shadows, &defs.compiled.footprint) {
                rt.db = rt.db.insert(path);
                rt.view = rt.view.insert(path);
            } else if monotone_toks(path, defs.compiled.cont_monotone, &defs.compiled.neg_footprint)
            {
                apply_grow(interner, &defs.compiled.rules, rt, path);
            } else {
                rt.db = rt.db.insert(path);
                reclose(interner, &defs.compiled.rules, rt);
            }
            // A newly spawned instance runs its inits (recursively — spawns can
            // spawn), on the POST-insert state, under the role bindings.
            if let Some(info) = spawn {
                let mut role_b = Bindings::new();
                for (param, value) in info.role_bindings {
                    role_b.insert(param, Val::Sym(value));
                }
                let inits = defs.compiled.practices[&info.pid].inits.clone();
                for init in &inits {
                    let g = ground_effect(interner, init, &role_b);
                    perform_effect(interner, defs, rt, &g);
                }
            }
        }
        Effect::InsertFor(n, path) => {
            // Insert now (through the ordinary Insert path: tiers, spawn, and any
            // stale timer cancelled), then arm a fresh expiry n boundaries out —
            // re-inserting the exact path with a lifetime REFRESHES the due (v44).
            let ins = Effect::Insert(path.clone());
            perform_effect(interner, defs, rt, &ins);
            let due = current_turn(interner, &rt.db) + n;
            rt.expiries.insert(path.clone(), due);
        }
        Effect::Call(fn_, args) => {
            // Registry lookup by name; a missing fn is a silent no-op (frozen; S9
            // makes it static). The FIRST matching case, queried against the BASE
            // db (the frozen quirk), first binding only.
            let Some((params, cases)) = defs.compiled.fns.get(fn_) else {
                return;
            };
            let mut param_b = Bindings::new();
            for (p, a) in params.iter().zip(args) {
                param_b.insert(*p, Val::Sym(*a));
            }
            let mut chosen: Option<(usize, Bindings)> = None;
            for (ci, (conds, _)) in cases.iter().enumerate() {
                if let Some(res) = query(interner, &rt.db, conds, &param_b).into_iter().next() {
                    chosen = Some((ci, res));
                    break;
                }
            }
            if let Some((ci, res)) = chosen {
                let outs = cases[ci].1.clone();
                for out in &outs {
                    let g = ground_effect(interner, out, &res);
                    perform_effect(interner, defs, rt, &g);
                }
            }
        }
        Effect::ForEach(conds, effs) => perform_for_each(interner, defs, rt, conds, effs),
        Effect::Roll(num, den, conds, effs) => {
            // None → panic (an unseeded die is an engine-invariant breach; S9's
            // SeedlessDraw makes it static). Advance UNCONDITIONALLY, store, and
            // on a hit run exactly ForEach on the advanced state.
            let s = rt
                .rng_seed
                .expect("Prax.Engine: Roll executed on an unseeded die (a draw in a world that never called seed_die)");
            let advanced = roll_step(s);
            rt.rng_seed = Some(advanced);
            if advanced % den < *num {
                perform_for_each(interner, defs, rt, conds, effs);
            }
        }
    }
}

/// A `ForEach`/`Roll` body: snapshot ALL bindings against the VIEW (empty seed)
/// up front, then apply the sub-effects per binding — so a mutation from inside
/// the fold cannot extend the quantification.
fn perform_for_each(
    interner: &mut Interner,
    defs: &Defs,
    rt: &mut Runtime,
    conds: &[Cond],
    effs: &[Effect],
) {
    let bindings = query(interner, &rt.view, conds, &Bindings::new());
    for b in bindings {
        for e in effs {
            let g = ground_effect(interner, e, &b);
            perform_effect(interner, defs, rt, &g);
        }
    }
}

/// One round boundary over the split state (`Prax.Engine.roundBoundary`). Advance
/// the clock, fire due expiries, then due schedule rules — see
/// [`State::round_boundary`] for the ghost-observation ordering law.
fn round_boundary_impl(interner: &mut Interner, defs: &Defs, rt: &mut Runtime) {
    let now = current_turn(interner, &rt.db) + 1;
    // Advance the clock on the ordinary insert path: the seeded `!` in `turn!now`
    // excludes `turn!prev`, so relevance/closure tiers apply.
    let clock = tokenize(interner, &format!("turn!{now}")).expect("turn!<n> is a valid path");
    perform_effect(interner, defs, rt, &Effect::Insert(clock));

    // Fire due expiries (due <= now) BEFORE any rule. Drop them from the queue
    // first; fire in RENDERED-NAME order (the determinism contract). That order is
    // UNOBSERVABLE — guarded subtree-retracts COMMUTE — so it is a posture note,
    // not a divergence from the frozen intern-id firing order (DIVERGENCES.md).
    let mut due: Vec<(String, CompiledPath)> = rt
        .expiries
        .iter()
        .filter(|&(_, &d)| d <= now)
        .map(|(k, _)| (ground(interner, k, &Bindings::new()), k.clone()))
        .collect();
    rt.expiries.retain(|_, &mut d| d > now);
    due.sort_by(|a, b| a.0.cmp(&b.0));
    let ordered: Vec<CompiledPath> = due.into_iter().map(|(_, p)| p).collect();
    fire_due_expiries_in(interner, defs, rt, &ordered);

    // Fire due schedule rules in DECLARATION order (the compiled-schedule order),
    // re-arming each `period` boundaries out from NOW. The due membership is read
    // from the dues as they stand (rule names are unique, so a re-arm never moves
    // another rule in or out). Each clause fires exactly as a `ForEach`.
    for rule in &defs.compiled.schedule {
        if rt.schedule_dues.get(&rule.name).copied().unwrap_or(i64::MAX) > now {
            continue;
        }
        let period = defs
            .schedule
            .iter()
            .find(|r| r.name == rule.name)
            .map(|r| r.period)
            .unwrap_or_else(|| {
                panic!(
                    "Prax.Engine.round_boundary: no schedule rule {:?} to resolve its period",
                    rule.name
                )
            });
        for (conds, effs) in &rule.body {
            perform_for_each(interner, defs, rt, conds, effs);
        }
        rt.schedule_dues.insert(rule.name.clone(), now + period);
    }
}

/// Fire a set of due expiries in the given order, each guarded by CURRENT
/// existence (`Prax.Engine.roundBoundary`'s `expireOne`): an entry whose exact
/// fact was evicted since drops silently — no retract, no recompute. The order is
/// the caller's choice because guarded subtree-retracts COMMUTE (a delete of an
/// ancestor subsumes its descendants; a later guard then finds them gone) — the
/// law the commutation proptest pins, and the reason S5 fires in name order.
fn fire_due_expiries_in(
    interner: &mut Interner,
    defs: &Defs,
    rt: &mut Runtime,
    ordered: &[CompiledPath],
) {
    for path in ordered {
        if rt.db.exists(interner, &path.segs) {
            perform_effect(interner, defs, rt, &Effect::Delete(path.clone()));
        }
    }
}

/// The role bindings and practice id of a newly spawned instance, or `None`.
struct SpawnInfo {
    pid: String,
    role_bindings: Vec<(Sym, Sym)>,
}

/// If inserting the sentence named by `names` brings a not-yet-existing practice
/// instance into being, return its role bindings and id so its inits can run
/// once (`Prax.Engine.spawnedInstanceNames`). `existedBefore` reads the BASE db.
fn spawned_instance(
    interner: &mut Interner,
    defs: &Defs,
    base_db: &Db,
    names: &[Sym],
) -> Option<SpawnInfo> {
    let practice_seg = interner.intern("practice");
    let (first, rest) = names.split_first()?;
    if *first != practice_seg {
        return None;
    }
    let (pid_sym, role_and_more) = rest.split_first()?;
    let pid = interner.resolve(*pid_sym).to_owned();
    let def = defs.practices.get(&pid)?;
    defs.compiled.practices.get(&pid)?; // both sides must know the practice
    let num_roles = def.roles.len();
    if role_and_more.len() < num_roles {
        return None;
    }
    let role_vals = &role_and_more[..num_roles];
    let mut instance_names: SmallVec<[Sym; 6]> = SmallVec::with_capacity(2 + num_roles);
    instance_names.push(practice_seg);
    instance_names.push(*pid_sym);
    instance_names.extend_from_slice(role_vals);
    if base_db.exists(interner, &instance_names) {
        return None; // existed before this insert — not a fresh spawn
    }
    let role_params: Vec<Sym> = def.roles.iter().map(|r| interner.intern(r)).collect();
    Some(SpawnInfo {
        pid,
        role_bindings: role_params.into_iter().zip(role_vals.iter().copied()).collect(),
    })
}

/// Every affordance the actor can perform, evaluated against the VIEW
/// (`Prax.Engine.possibleActions`). pids come in name order (`child_keys`),
/// instance unification and the inner condition query both branch in name order,
/// and actions run in declaration order — the determinism contract end to end.
pub(crate) fn possible_actions_impl(
    interner: &mut Interner,
    defs: &Defs,
    view: &Db,
    actor: &str,
) -> Vec<GroundedAction> {
    let practice_seg = interner.intern("practice");
    let actor_var = interner.intern("Actor");
    let actor_sym = interner.intern(actor);
    let pids = view.child_keys(interner, &[practice_seg]);
    let mut out = Vec::new();
    for pid in pids {
        let Some(cp) = defs.compiled.practices.get(&pid) else {
            continue;
        };
        let mut seed = Bindings::new();
        seed.insert(actor_var, Val::Sym(actor_sym));
        for inst in view.unify(interner, &cp.instance_names, seed) {
            let inst_syms = crate::query::ground_names(interner, &inst, &cp.instance_names);
            let instance_id = inst_syms
                .iter()
                .map(|&s| interner.resolve(s).to_owned())
                .collect::<Vec<_>>()
                .join(".");
            for ca in &cp.actions {
                for binding in query(interner, view, &ca.conds, &inst) {
                    let label = render_text(interner, &ca.name, &binding);
                    out.push(GroundedAction {
                        practice_id: pid.clone(),
                        instance_id: instance_id.clone(),
                        action_id: ca.name.clone(),
                        bindings: binding,
                        label,
                    });
                }
            }
        }
    }
    out
}

/// Rebuild the derived tables from the current authored sources
/// (`Prax.Engine.retable`, minus reclose).
fn rebuild(interner: &mut Interner, defs: &mut Defs) -> Result<(), WorldError> {
    defs.compiled = recompile(
        interner,
        &defs.practices,
        &defs.fns,
        &defs.axioms,
        &defs.characters,
        &defs.desires,
        &defs.schedule,
        &defs.prediction_scope,
    )?;
    Ok(())
}

/// Rebuild the cached closed view (`Prax.Engine.reclose`): close the base under
/// the axioms; a `⊥` is surfaced as a queryable `contradiction` fact over the
/// (still-consistent) base rather than crashing.
fn reclose(interner: &mut Interner, rules: &[CompiledRule], rt: &mut Runtime) {
    rt.view = match close(interner, rules, &rt.db) {
        Ok(closed) => closed,
        Err(_) => {
            let c = tokenize(interner, "contradiction").expect("literal path");
            rt.db.insert(&c)
        }
    };
}

/// The continuation tier (`Prax.Engine.applyGrowToks`): grow the base and
/// continue the ALREADY-CLOSED view with the one new fact via [`close_from`]. A
/// `⊥` falls back to the full reclose path (which reaches the same
/// `contradiction` marker from scratch).
fn apply_grow(interner: &mut Interner, rules: &[CompiledRule], rt: &mut Runtime, path: &CompiledPath) {
    match close_from(interner, rules, &rt.view, std::slice::from_ref(path)) {
        Ok(v) => {
            rt.db = rt.db.insert(path);
            rt.view = v;
        }
        Err(_) => {
            rt.db = rt.db.insert(path);
            reclose(interner, rules, rt);
        }
    }
}

/// Can this ground delta change what the axioms derive (`Prax.Engine.relevantNames`)?
/// `false` only when the sentence — and anything its exclusions evict —
/// may-unify NOTHING in the axioms' footprint (the licence to skip reclose).
fn relevant_names(
    names: &[Sym],
    shadows: &[SmallVec<[Sym; 6]>],
    footprint: &[SmallVec<[Sym; 6]>],
) -> bool {
    let primary = std::iter::once(names);
    let shadow_slices = shadows.iter().map(|s| s.as_slice());
    primary
        .chain(shadow_slices)
        .any(|ns| footprint.iter().any(|fp| may_unify_syms(ns, fp)))
}

/// May this insert take the continuation tier (`Prax.Engine.monotoneToks`): the
/// world is continuation-safe, the insert has no `!` (evicts nothing), and it
/// unifies no negated body pattern (defeats nothing).
fn monotone_toks(path: &CompiledPath, cont_monotone: bool, neg_footprint: &[SmallVec<[Sym; 6]>]) -> bool {
    cont_monotone
        && path.excl == 0
        && !neg_footprint
            .iter()
            .any(|nf| may_unify_syms(&path.segs, nf))
}

/// The engine clock's value from the db (`Prax.Engine.currentTurn`); loud on a
/// missing or non-lone-numeric `turn`.
fn current_turn(interner: &mut Interner, db: &Db) -> i64 {
    let turn = interner.intern("turn");
    let ks = db.child_keys(interner, &[turn]);
    match ks.as_slice() {
        [n] => n.parse::<i64>().unwrap_or_else(|_| {
            panic!("Prax.Engine.current_turn: the single \"turn\" value {n:?} is not numeric")
        }),
        other => panic!(
            "Prax.Engine.current_turn: expected exactly one numeric \"turn\" value, found {other:?}"
        ),
    }
}

/// Whether `needle` is a prefix of `hay` (by `Sym` id).
fn is_prefix(needle: &[Sym], hay: &[Sym]) -> bool {
    needle.len() <= hay.len() && needle.iter().zip(hay).all(|(a, b)| a == b)
}

/// The first item that already appeared earlier in the sequence, if any — the
/// duplicate-action-name guard (`Prax.Engine.definePractice`'s `dupActions`).
fn first_duplicate<'a>(items: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let mut seen: Vec<&str> = Vec::new();
    for it in items {
        if seen.contains(&it) {
            return Some(it.to_owned());
        }
        seen.push(it);
    }
    None
}

/// Substitute `[Var]` placeholders in a template using the bindings, leaving
/// unknown placeholders untouched (`Prax.Engine.renderText`).
pub fn render_text(interner: &mut Interner, template: &str, b: &Bindings) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(open) = rest.find('[') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find(']') {
            Some(close) => {
                let name = &after[..close];
                let tail = &after[close + 1..];
                let sym = interner.intern(name);
                match b.get(sym) {
                    Some(v) => out.push_str(&val_to_string(interner, v)),
                    None => {
                        out.push('[');
                        out.push_str(name);
                        out.push(']');
                    }
                }
                rest = tail;
            }
            None => {
                // Unterminated '[': emit it literally and continue scanning.
                out.push('[');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    // H: EngineSpec.hs "Prax.Engine"
    //
    // The frozen Prax.EngineSpec, re-expressed against the Rust State/perform
    // surface. The two groundedDeltaAnchors pins (owed:S6, sole consumer at S6)
    // and the build-order-death label's typeCheck clause (owed:S9) are killed in
    // conformance/KILLED.md; S4 lands an INDEPENDENT compiled-rule-equality
    // regression (`set_axioms_order_independent_cooked_rules`, no Haskell label),
    // and the obligedClose-through-the-engine pin lives in conformance (where
    // prax-vocab's obliged_close is in scope).
    use super::*;
    use crate::query::{CalcOp, Condition};
    use crate::types::{Action, Function, Practice, call, delete, for_each, insert};

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

    fn labels(st: &mut State, actor: &str) -> Vec<String> {
        st.possible_actions(actor).into_iter().map(|g| g.label).collect()
    }

    /// Perform the first action whose label contains `needle`.
    fn step(st: &mut State, actor: &str, needle: &str) {
        let ga = st
            .possible_actions(actor)
            .into_iter()
            .find(|g| g.label.contains(needle))
            .unwrap_or_else(|| {
                panic!("no action matching {needle:?} for {actor}")
            });
        st.perform_action(&ga);
    }

    // Practices ported from EngineSpec.hs's eDSL fixtures.
    fn greet_p() -> Practice {
        Practice::new("greet")
            .name("[Greeter] is greeting [Greeted]")
            .roles(["Greeter", "Greeted"])
            .action(
                Action::new("[Actor]: Greet [Other]")
                    .when([eq("Actor", "Greeter"), eq("Other", "Greeted")])
                    .then([delete("practice.greet.Actor.Other")]),
            )
    }

    fn tend_bar_p() -> Practice {
        Practice::new("tendBar")
            .name("[Bartender] is tending bar")
            .roles(["Bartender"])
            .data_facts([
                "beverageType.beer!alcoholic",
                "beverageType.cider!alcoholic",
                "beverageType.soda!nonalcoholic",
                "beverageType.water!nonalcoholic",
            ])
            .action(
                Action::new("[Actor]: Walk up to bar")
                    .when([
                        neq("Actor", "Bartender"),
                        n("practice.tendBar.Bartender.customer.Actor"),
                    ])
                    .then([insert("practice.tendBar.Bartender.customer.Actor")]),
            )
            .action(
                Action::new("[Actor]: Order [Beverage]")
                    .when([
                        m("practice.tendBar.Bartender.customer.Actor"),
                        n("practice.tendBar.Bartender.customer.Actor!beverage"),
                        m("practiceData.tendBar.beverageType.Beverage"),
                    ])
                    .then([insert("practice.tendBar.Bartender.customer.Actor!order!Beverage")]),
            )
            .action(
                Action::new("[Actor]: Fulfill [Customer]'s order")
                    .when([
                        eq("Actor", "Bartender"),
                        m("practice.tendBar.Bartender.customer.Customer!order!Beverage"),
                    ])
                    .then([
                        delete("practice.tendBar.Bartender.customer.Customer!order"),
                        insert("practice.tendBar.Bartender.customer.Customer!beverage!Beverage"),
                    ]),
            )
    }

    fn duel_p() -> Practice {
        Practice::new("duel")
            .name("[A] duels [B]")
            .roles(["A", "B"])
            .init([insert("practice.duel.A.B.turn!A")])
            .action(
                Action::new("[Actor]: Strike")
                    .when([m("practice.duel.A.B.turn!Actor")])
                    .then([insert("practice.duel.A.B.struck!Actor")]),
            )
    }

    fn math_p() -> Practice {
        Practice::new("math")
            .name("math box [M]")
            .roles(["M"])
            .init([insert("practice.math.M.n!3")])
            .action(
                Action::new("[Actor]: Double")
                    .when([m("practice.math.M.n!N")])
                    .then([call("dbl", vec!["M".to_owned(), "N".to_owned()])]),
            )
    }

    fn dbl_fn() -> Function {
        Function::new("dbl", ["M", "N"]).case(
            [Condition::Calc("R".into(), CalcOp::Mul, "N".into(), "2".into())],
            [insert("practice.math.M.n!R")],
        )
    }

    // The compiled axiom-head families, resolved to names (for the axiomHeads pin).
    fn head_names(st: &State) -> Vec<Vec<String>> {
        st.defs
            .compiled
            .axiom_heads
            .iter()
            .map(|h| h.iter().map(|&s| st.interner.resolve(s).to_owned()).collect())
            .collect()
    }
    fn axiom_head_has(st: &State, s: &str) -> bool {
        // UNCHECKED-SPLIT (frozen: none — a test helper, not a port). It splits
        // this file's own literal head paths (`contradiction`, `feud.A.B`) to
        // compare against the compiled axiom heads; no frozen `pathNames` call
        // stands behind it, and a trailing operator here would be a typo in the
        // test, which the comparison then fails on.
        head_names(st).contains(&segment_names(s))
    }

    // ===== practices / actions / spawning =====

    // H: EngineSpec.hs "cookedDefs mirrors practiceDefs' keys after definePractices"
    #[test]
    fn cooked_defs_mirrors_practice_defs_keys() {
        let mut st = State::new();
        st.define_practices([greet_p(), tend_bar_p(), duel_p(), math_p()]).unwrap();
        st.define_functions([dbl_fn()]).unwrap();
        let authored: Vec<&String> = st.defs.practices.keys().collect();
        let cooked: Vec<&String> = st.defs.compiled.practices.keys().collect();
        assert_eq!(authored, cooked);
    }

    // H: EngineSpec.hs "definePractice inserts static data under practiceData"
    #[test]
    fn define_practice_inserts_static_data() {
        let mut st = State::new();
        st.define_practice(tend_bar_p()).unwrap();
        assert!(st.db_has("practiceData.tendBar.beverageType.cider.alcoholic"));
    }

    // H: EngineSpec.hs "greet: affordance appears, and performing it consumes the instance"
    #[test]
    fn greet_affordance_appears_and_is_consumed() {
        let mut st = State::new();
        st.define_practice(greet_p()).unwrap();
        st.perform_outcome(&insert("practice.greet.max.isaac")).unwrap();
        assert_eq!(labels(&mut st, "max"), ["max: Greet isaac"]);
        step(&mut st, "max", "Greet isaac");
        assert_eq!(labels(&mut st, "max"), Vec::<String>::new());
    }

    // H: EngineSpec.hs "tendBar: walk up -> order -> fulfill delivers the drink"
    #[test]
    fn tend_bar_walk_order_fulfill() {
        let mut st = State::new();
        st.define_practices([tend_bar_p()]).unwrap();
        st.perform_outcome(&insert("practice.tendBar.ada")).unwrap();
        assert_eq!(labels(&mut st, "beth"), ["beth: Walk up to bar"]);
        step(&mut st, "beth", "Walk up to bar");
        assert!(labels(&mut st, "beth").contains(&"beth: Order cider".to_owned()));
        step(&mut st, "beth", "Order cider");
        assert!(labels(&mut st, "ada").iter().any(|l| l.contains("Fulfill")));
        step(&mut st, "ada", "Fulfill");
        assert!(st.db_has("practice.tendBar.ada.customer.beth.beverage.cider"));
        assert!(
            !st.labeled_facts().iter().any(|f| f.contains("customer.beth.order")),
            "pending order cleared"
        );
    }

    // H: EngineSpec.hs "spawning runs init once; only the whose-turn actor can strike"
    #[test]
    fn spawning_runs_init_once_and_only_whose_turn_strikes() {
        let mut st = State::new();
        st.define_practice(duel_p()).unwrap();
        st.perform_outcome(&insert("practice.duel.max.nic")).unwrap();
        assert!(st.db_has("practice.duel.max.nic.turn.max"), "init seeded turn");
        assert_eq!(labels(&mut st, "max"), ["max: Strike"]);
        assert_eq!(labels(&mut st, "nic"), Vec::<String>::new());
    }

    // H: EngineSpec.hs "call into a guarded function applies its calc effect"
    #[test]
    fn call_into_a_guarded_function_applies_its_calc() {
        let mut st = State::new();
        st.define_practice(math_p()).unwrap();
        st.define_functions([dbl_fn()]).unwrap();
        st.perform_outcome(&insert("practice.math.box")).unwrap();
        assert!(st.db_has("practice.math.box.n.3"), "init n=3");
        step(&mut st, "alice", "Double");
        assert!(st.db_has("practice.math.box.n.6"), "n doubled to 6");
    }

    // ===== ForEach =====

    // H: EngineSpec.hs "ForEach applies its outcomes for every binding"
    #[test]
    fn for_each_applies_for_every_binding() {
        let mut st = State::new();
        for s in ["member.a", "member.b", "member.c"] {
            st.perform_outcome(&insert(s)).unwrap();
        }
        st.perform_outcome(&for_each(vec![m("member.X")], vec![insert("greeted.X")]))
            .unwrap();
        for name in ["a", "b", "c"] {
            assert!(st.db_has(&format!("greeted.{name}")));
        }
    }

    // H: EngineSpec.hs "ForEach with zero bindings is a no-op"
    #[test]
    fn for_each_zero_bindings_is_a_noop() {
        let mut st = State::new();
        st.perform_outcome(&insert("unrelated")).unwrap();
        let before = st.labeled_facts();
        st.perform_outcome(&for_each(vec![m("member.X")], vec![insert("greeted.X")]))
            .unwrap();
        assert_eq!(st.labeled_facts(), before);
    }

    // H: EngineSpec.hs "ForEach snapshots its bindings: mutations cannot extend the quantification"
    #[test]
    fn for_each_snapshots_its_bindings() {
        let mut st = State::new();
        st.perform_outcome(&insert("member.a")).unwrap();
        st.perform_outcome(&for_each(
            vec![m("member.X")],
            vec![insert("member.b"), insert("visited.X")],
        ))
        .unwrap();
        assert!(st.db_has("visited.a"), "visited the original member");
        assert!(!st.db_has("visited.b"), "did NOT visit the member inserted mid-fold");
    }

    // H: EngineSpec.hs "ForEach grounds the enclosing action's bindings first"
    #[test]
    fn for_each_grounds_enclosing_action_bindings_first() {
        let mut st = State::new();
        let p = Practice::new("tell").roles(["R"]).action(
            Action::new("[Actor]: tell friends about [Target]")
                .when([m("target.Target")])
                .then([for_each(vec![m("friend.Target.W")], vec![insert("told.W.Target")])]),
        );
        st.define_practices([p]).unwrap();
        for s in [
            "practice.tell.stage",
            "target.bob",
            "friend.bob.carol",
            "friend.bob.dave",
            "friend.eve.mallory",
        ] {
            st.perform_outcome(&insert(s)).unwrap();
        }
        let ga = st.possible_actions("ann").into_iter().next().expect("tell action offered");
        st.perform_action(&ga);
        assert!(st.db_has("told.carol.bob"));
        assert!(st.db_has("told.dave.bob"));
        assert!(!st.db_has("told.mallory.eve"), "a different target's friend must not fire");
    }

    // H: EngineSpec.hs "ForEach nests: outer bindings ground the inner quantifier"
    #[test]
    fn for_each_nests() {
        let mut st = State::new();
        for s in ["row.a", "row.b", "col.x", "col.y"] {
            st.perform_outcome(&insert(s)).unwrap();
        }
        st.perform_outcome(&for_each(
            vec![m("row.R")],
            vec![for_each(vec![m("col.C")], vec![insert("cell.R.C")])],
        ))
        .unwrap();
        for s in ["cell.a.x", "cell.a.y", "cell.b.x", "cell.b.y"] {
            assert!(st.db_has(s), "{s}");
        }
    }

    // H: EngineSpec.hs "ForEach snapshot holds for Delete: removing a member mid-fold still visits all"
    #[test]
    fn for_each_snapshot_holds_for_delete() {
        let mut st = State::new();
        for s in ["member.a", "member.b"] {
            st.perform_outcome(&insert(s)).unwrap();
        }
        st.perform_outcome(&for_each(
            vec![m("member.X")],
            vec![delete("member.X"), insert("visited.X")],
        ))
        .unwrap();
        assert!(st.db_has("visited.a"));
        assert!(st.db_has("visited.b"));
        assert!(!st.db_has("member.a"));
        assert!(!st.db_has("member.b"));
    }

    // H: EngineSpec.hs "ForEach with no conditions applies its outcomes exactly once"
    #[test]
    fn for_each_no_conditions_applies_exactly_once() {
        let mut st = State::new();
        st.perform_outcome(&insert("counter!0")).unwrap();
        st.perform_outcome(&for_each(
            vec![],
            vec![for_each(
                vec![m("counter!N"), Condition::Calc("M".into(), CalcOp::Add, "N".into(), "1".into())],
                vec![insert("counter!M")],
            )],
        ))
        .unwrap();
        assert!(st.db_has("counter!1"), "ran exactly once");
        assert!(!st.db_has("counter!2"), "not twice");
    }

    // ===== axioms / view =====

    // H: EngineSpec.hs "setAxioms re-derives the cached view on a built state"
    #[test]
    fn set_axioms_re_derives_the_cached_view() {
        let mut st = State::new();
        st.perform_outcome(&insert("parent.ada.bea")).unwrap();
        assert!(!st.view_has("elder.ada"), "no axioms: nothing derived");
        st.set_axioms(vec![Axiom::new(vec![m("parent.X.Y")], ["elder.X"])]).unwrap();
        assert!(st.view_has("elder.ada"), "derived after set_axioms");
        st.perform_outcome(&insert("parent.bea.cal")).unwrap();
        assert!(st.view_has("elder.bea"), "new base fact derives too");
    }

    // H: EngineSpec.hs "axiomHeads: fireable heads and the contradiction witness"
    #[test]
    fn axiom_heads_fireable_heads_and_contradiction_witness() {
        let mut st = State::new();
        st.set_axioms(vec![Axiom::new(vec![m("starving.X")], ["hungry.X"])]).unwrap();
        assert!(axiom_head_has(&st, "hungry.X"), "the head");
        assert!(axiom_head_has(&st, "contradiction"), "the ⊥ witness");
        // cookAxioms is deontics-free: a bare rule contributes no □-lifted twin.
        assert!(!axiom_head_has(&st, "obliged.Obligor.hungry.X"), "no lifted head from a bare rule");
    }

    // H: EngineSpec.hs "declared □-closure (spec v51): the lift is content the world declares"
    // H: EngineSpec.hs "a world that does NOT declare its closure carries no lifted twin"
    #[test]
    fn a_world_that_does_not_declare_its_closure_has_no_lifted_twin() {
        // liftAx = a.X -> b.X (the fixture whose □-lifted twin would be
        // obliged.Obligor.b.X). Declared bare, no lift appears in axiomHeads.
        let mut st = State::new();
        st.set_axioms(vec![Axiom::new(vec![m("a.X")], ["b.X"])]).unwrap();
        assert!(axiom_head_has(&st, "b.X"), "base head present");
        assert!(!axiom_head_has(&st, "obliged.Obligor.b.X"), "no lifted twin");
    }

    // The build-order-death label, now with BOTH clauses (owed:S9 discharged):
    // recompile is deontics-free and reads only the axiom list, never the db, so
    // (a) a db-changing insert between two identical set_axioms leaves the cooked
    // rules byte-identical (compared within ONE State, interner ids stable — never
    // a cross-lineage Sym compare), AND (b) the `type_check` verdict is
    // independent of where in the build sequence `set_axioms` is called (compared
    // ACROSS two states — safe, `TypeError` carries only strings, no Sym ids).
    // H: EngineSpec.hs "build-order death: setAxioms-first equals setAxioms-outermost (cookedRules and typeCheck)"
    #[test]
    fn set_axioms_order_independent_cooked_rules() {
        let axs = vec![Axiom::new(vec![m("a.X")], ["b.X"])];
        let mut st = State::new();
        st.set_axioms(axs.clone()).unwrap();
        let rules_before = st.defs.compiled.rules.clone();
        // An obliged-producing db fact lands AFTER the rules are fixed (the order
        // v48 forbade); recompiling the same axioms must reproduce the rules.
        st.perform_outcome(&insert("obliged.w.a.foo")).unwrap();
        st.set_axioms(axs.clone()).unwrap();
        assert_eq!(
            st.defs.compiled.rules, rules_before,
            "recompile must not consult the db (cooked rules depend only on the axiom list)"
        );

        // typeCheck-equality clause: the same world built with `set_axioms` FIRST
        // vs LAST is well-formed either way, and the verdict is identical.
        let mut a = State::new();
        a.set_axioms(axs.clone()).unwrap();
        a.set_desires(vec![Desire::new("d", Want::new(vec![m("a.Owner")], 1))]).unwrap();
        let mut b = State::new();
        b.set_desires(vec![Desire::new("d", Want::new(vec![m("a.Owner")], 1))]).unwrap();
        b.set_axioms(axs).unwrap();
        assert_eq!(
            crate::typecheck::type_check(&a),
            crate::typecheck::type_check(&b),
            "the type_check verdict is independent of set_axioms's build-order position"
        );
    }

    // ===== collision guards =====
    // H: EngineSpec.hs "collision guards (v43, re-expressed against the v47 registry): action names and registered function names must each be unique"

    // H: EngineSpec.hs "two actions with the same name in one practice is a loud construction-time error"
    #[test]
    fn duplicate_action_names_are_rejected() {
        let p = Practice::new("dup")
            .roles(["R"])
            .action(Action::new("dup"))
            .action(Action::new("dup"));
        assert!(matches!(
            State::new().define_practice(p),
            Err(WorldError::DuplicateActionName { .. })
        ));
    }

    // H: EngineSpec.hs "two functions with the same name within ONE defineFunctions batch is a loud error"
    #[test]
    fn duplicate_function_names_within_a_batch_are_rejected() {
        let batch = [Function::new("f", Vec::<String>::new()), Function::new("f", Vec::<String>::new())];
        assert!(matches!(
            State::new().define_functions(batch),
            Err(WorldError::DuplicateFunctionName { .. })
        ));
    }

    // H: EngineSpec.hs "a function name already registered by an EARLIER defineFunctions call is a loud error"
    #[test]
    fn duplicate_function_names_across_calls_are_rejected() {
        let mut st = State::new();
        st.define_functions([Function::new("f", Vec::<String>::new())]).unwrap();
        assert!(matches!(
            st.define_functions([Function::new("f", Vec::<String>::new())]),
            Err(WorldError::DuplicateFunctionName { .. })
        ));
    }

    // ===== the single-segment character-name guard (DIV-2) =====
    // A character name is spliced into engine-built sentences (`dead.<name>`,
    // `<p>.believes.desires.<name>.…`, every practice instance path), so a
    // separator inside it nests the character's fact families under one another
    // and splits its death mark and scope anchors across segment boundaries. The
    // frozen engine has no such guard and behaves INCONSISTENTLY across its own
    // planner sites when one is supplied (DIVERGENCES.md DIV-2); the Rust
    // rejects the name loudly at the one door instead.

    #[test]
    fn a_plain_character_name_is_accepted() {
        let mut st = State::new();
        assert!(
            st.set_characters(vec![Character::new("keeper"), Character::new("inge")])
                .is_ok(),
            "a single-segment name is a name"
        );
        assert_eq!(st.characters().len(), 2);
    }

    #[test]
    fn a_separator_bearing_character_name_is_rejected() {
        for bad in ["hall.keeper", "hall!keeper"] {
            let mut st = State::new();
            assert!(
                matches!(
                    st.set_characters(vec![Character::new(bad)]),
                    Err(WorldError::MultiSegmentCharacterName { ref name }) if name == bad
                ),
                "{bad:?} is a path injection, not a name -- set_characters must reject it"
            );
            assert!(
                st.characters().is_empty(),
                "a rejected roster must not be installed"
            );
        }
    }

    // H: EngineSpec.hs "distinct function names across two calls register cleanly (accumulation)"
    #[test]
    fn distinct_function_names_accumulate() {
        let mut st = State::new();
        st.define_functions([Function::new("f", Vec::<String>::new())]).unwrap();
        st.define_functions([Function::new("g", Vec::<String>::new())]).unwrap();
        let keys: Vec<&String> = st.defs.compiled.fns.keys().collect();
        assert_eq!(keys, ["f", "g"]);
    }

    // ===== RngSpec engine-integration half (the stream rides State) =====

    fn lehmer_next(s: i64) -> i64 {
        (s * 16807) % 2_147_483_647
    }
    fn apply_draw(st: &mut State, num: i64, den: i64, conds: Vec<Condition>, outs: Vec<Outcome>) {
        for o in crate::rng::draw(num, den, conds, outs).unwrap() {
            st.perform_outcome(&o).unwrap();
        }
    }

    // H: RngSpec.hs "the stream (engine state, v50)"
    // H: RngSpec.hs "rollStep is one Park-Miller step, and each draw advances the seed exactly once"
    #[test]
    fn each_draw_advances_the_seed_exactly_once() {
        let s0 = 12345;
        let (s1, s2, s3) = (lehmer_next(s0), lehmer_next(lehmer_next(s0)), lehmer_next(lehmer_next(lehmer_next(s0))));
        assert_eq!(crate::rng::roll_step(s0), s1);
        let mut st = State::new();
        st.seed_die(s0).unwrap();
        apply_draw(&mut st, 1, 2, vec![], vec![]);
        assert_eq!(st.rng_seed(), Some(s1));
        apply_draw(&mut st, 1, 2, vec![], vec![]);
        assert_eq!(st.rng_seed(), Some(s2));
        apply_draw(&mut st, 1, 2, vec![], vec![]);
        assert_eq!(st.rng_seed(), Some(s3));
    }

    // H: RngSpec.hs "the frozen-die law"
    // H: RngSpec.hs "two draws with unsatisfiable guards still advance the seed twice"
    #[test]
    fn unsatisfiable_guards_still_advance_the_seed() {
        let s0 = 5;
        let mut st = State::new();
        st.seed_die(s0).unwrap();
        let impossible = vec![m("ghost.nothing")];
        apply_draw(&mut st, 1, 2, impossible.clone(), vec![insert("should.never.fire")]);
        apply_draw(&mut st, 1, 2, impossible, vec![insert("should.never.fire")]);
        assert_eq!(st.rng_seed(), Some(lehmer_next(lehmer_next(s0))));
        assert!(!st.db_has("should.never.fire"), "the unsatisfiable-guard outs never fired");
    }

    // H: RngSpec.hs "a miss advances: the SAME position, drawn twice, diverges on the next draw"
    #[test]
    fn a_miss_advances_the_position() {
        let s0 = 7;
        let (r1, r2) = (lehmer_next(s0), lehmer_next(lehmer_next(s0)));
        assert_ne!(r1, r2, "fixture: the two roll bases differ");
        let mut st = State::new();
        st.seed_die(s0).unwrap();
        apply_draw(&mut st, 1, 2, vec![], vec![]);
        assert_eq!(st.rng_seed(), Some(r1));
        apply_draw(&mut st, 1, 2, vec![], vec![]);
        assert_eq!(st.rng_seed(), Some(r2));
    }

    // H: RngSpec.hs "hit / miss"
    // H: RngSpec.hs "a hit applies outs (seed 2, odds 1/2 -> rollStep 2 is even)"
    #[test]
    fn a_hit_applies_outs() {
        let s0 = 2;
        assert!(crate::rng::roll_step(s0) % 2 == 0, "fixture: rollStep 2 is a hit");
        let mut st = State::new();
        st.seed_die(s0).unwrap();
        apply_draw(&mut st, 1, 2, vec![], vec![insert("hit.mark")]);
        assert!(st.db_has("hit.mark"));
        assert_eq!(st.rng_seed(), Some(lehmer_next(s0)));
    }

    // H: RngSpec.hs "a miss does not apply outs (seed 1, odds 1/2 -> rollStep 1 is odd)"
    #[test]
    fn a_miss_does_not_apply_outs() {
        let s0 = 1;
        assert!(crate::rng::roll_step(s0) % 2 == 1, "fixture: rollStep 1 is a miss");
        let mut st = State::new();
        st.seed_die(s0).unwrap();
        apply_draw(&mut st, 1, 2, vec![], vec![insert("hit.mark")]);
        assert!(!st.db_has("hit.mark"));
        assert_eq!(st.rng_seed(), Some(lehmer_next(s0)));
    }

    // H: RngSpec.hs "sequential multi-draw (Village.hs's two-arm shape)"
    // H: RngSpec.hs "two draws off one stream roll on successive values and advance twice"
    #[test]
    fn two_draws_off_one_stream_advance_twice() {
        let s0 = 1988;
        let (s1, s2) = (lehmer_next(s0), lehmer_next(lehmer_next(s0)));
        assert!(s1 % 4 < 1, "fixture: base arm hits");
        assert!(s2 % 4 < 2, "fixture: trait arm hits");
        let mut st = State::new();
        st.seed_die(s0).unwrap();
        apply_draw(&mut st, 1, 4, vec![], vec![insert("arm1.fired")]);
        apply_draw(&mut st, 2, 4, vec![], vec![insert("arm2.fired")]);
        assert!(st.db_has("arm1.fired"));
        assert!(st.db_has("arm2.fired"));
        assert_eq!(st.rng_seed(), Some(s2));
    }

    // H: RngSpec.hs "an unseeded die is loud"
    // H: RngSpec.hs "executing a Roll with rngSeed == Nothing is a loud error"
    #[test]
    #[should_panic(expected = "unseeded die")]
    fn executing_a_roll_on_an_unseeded_die_panics() {
        let mut st = State::new();
        st.perform_outcome(&Outcome::Roll(1, 2, vec![], vec![insert("x")])).unwrap();
    }

    // H: RngSpec.hs "seedDie domain guard"
    // H: RngSpec.hs "an in-domain seed is accepted"
    #[test]
    fn seed_die_accepts_in_domain() {
        let mut st = State::new();
        st.seed_die(12345).unwrap();
        assert_eq!(st.rng_seed(), Some(12345));
    }

    // H: RngSpec.hs "seedDie rejects a seed of 0"
    #[test]
    fn seed_die_rejects_zero() {
        assert!(matches!(
            State::new().seed_die(0),
            Err(WorldError::SeedOutOfDomain { .. })
        ));
    }

    // H: RngSpec.hs "seedDie rejects a seed at or above the modulus"
    #[test]
    fn seed_die_rejects_at_or_above_modulus() {
        assert!(matches!(
            State::new().seed_die(2_147_483_647),
            Err(WorldError::SeedOutOfDomain { .. })
        ));
    }

    // H: RngSpec.hs "seedDie rejects a negative seed"
    #[test]
    fn seed_die_rejects_negative() {
        assert!(matches!(
            State::new().seed_die(-5),
            Err(WorldError::SeedOutOfDomain { .. })
        ));
    }
}

#[cfg(test)]
mod boundary_props {
    //! The round boundary's own laws under randomization: expiry-fire commutation
    //! (the S5 adjudication's justification — the frozen fires due expiries in
    //! intern-id order, the Rust in rendered-name order, and the result is the same
    //! because guarded subtree-retracts COMMUTE). Homed in-crate because it drives
    //! [`fire_due_expiries_in`] with two different firing orders on the SAME due
    //! set, which the public API deliberately does not expose.
    use super::*;
    use crate::types::insert_for;
    use proptest::prelude::*;

    /// Arm every path with lifetime 1 (all due at boundary 1), then fire the due
    /// expiries in rendered-name order — ascending if `ascending`, else descending.
    /// The two orders are genuine permutations of one another whenever ≥2 distinct
    /// paths are due, so agreement pins commutation.
    fn fire_all_due(st: &mut State, ascending: bool) {
        let interner = Arc::make_mut(&mut st.interner);
        let defs = st.defs.clone();
        let mut named: Vec<(String, CompiledPath)> = st
            .rt
            .expiries
            .keys()
            .map(|k| (ground(interner, k, &Bindings::new()), k.clone()))
            .collect();
        named.sort_by(|a, b| a.0.cmp(&b.0));
        if !ascending {
            named.reverse();
        }
        let ordered: Vec<CompiledPath> = named.into_iter().map(|(_, p)| p).collect();
        fire_due_expiries_in(interner, defs.as_ref(), &mut st.rt, &ordered);
    }

    proptest! {
        #[test]
        fn expiry_firing_commutes(
            idxs in prop::sample::subsequence(vec![0usize, 1, 2, 3, 4, 5, 6], 1..=7),
            keeps in prop::sample::subsequence(vec![0usize, 1, 2], 0..=3)
        ) {
            // A pool mixing disjoint leaves, an ancestor/descendant pair (the only
            // NON-disjoint case, where firing order could matter), and `!` siblings —
            // PLUS not-yet-due KEEP entries armed UNDER due ancestors (S5 review I1:
            // a fired ancestor's retract purges the keep's queue entry; order
            // independence must hold for the SURVIVING-queue shape too, which the
            // all-lifetime-1 pool alone never exercises).
            const POOL: [&str; 7] = [
                "feels.anger", "feels.anger.toward.bob", "feels.joy",
                "mood!a", "mood!b", "at.home", "at.home.since!noon",
            ];
            const KEEPS: [&str; 3] = [
                "feels.anger.toward.carol", "at.home.guest", "feels.joy.reason",
            ];
            let build = || {
                let mut st = State::new();
                for &i in &idxs {
                    st.perform_outcome(&insert_for(1, POOL[i])).unwrap();
                }
                for &k in &keeps {
                    st.perform_outcome(&insert_for(5, KEEPS[k])).unwrap();
                }
                st
            };
            let mut asc = build();
            fire_all_due(&mut asc, true);
            let mut desc = build();
            fire_all_due(&mut desc, false);
            prop_assert_eq!(asc.labeled_facts(), desc.labeled_facts());
            prop_assert_eq!(asc.labeled_view(), desc.labeled_view());
            // The surviving expiry QUEUE must also be order-independent.
            prop_assert_eq!(asc.expiries_rendered(), desc.expiries_rendered());
        }
    }
}

#[cfg(test)]
mod delta_anchor_spec {
    // The v34 reuse anchor walk, discharged at its sole-consumer stage (S6);
    // the owed:S6 rows are removed from KILLED.md.
    // H: EngineSpec.hs "groundedDeltaAnchors: bounded effects, shadows, spawn opacity, Call resolution"
    // H: EngineSpec.hs "groundedDeltaAnchors: safe ForEach binders bound; unsafe heads stay opaque"
    use super::State;
    use crate::query::Condition;
    use crate::types::{Action, Function, Practice, call, delete, for_each, insert};

    fn m(s: &str) -> Condition {
        Condition::Match(s.into())
    }

    // Build a one-instance practice, ground the named action for `you`, and
    // return the state + grounded action so the walk can be probed.
    fn probe(action: Action, fns: Vec<Function>) -> (State, super::GroundedAction) {
        let p = Practice::new("p").roles(["R"]).action(action);
        let mut st = State::new();
        st.define_practices([p]).unwrap();
        for f in fns {
            st.define_functions([f]).unwrap();
        }
        st.perform_outcome(&insert("practice.p.here")).unwrap();
        let ga = st
            .possible_actions("you")
            .into_iter()
            .next()
            .expect("one grounded action");
        (st, ga)
    }

    #[test]
    fn bounded_insert_and_delete() {
        let (mut st, ga) = probe(Action::new("[Actor]: act").then([insert("mark.Actor.foo")]), vec![]);
        let anchors = st.grounded_delta_anchors_of(&ga).expect("bounded");
        assert!(anchors.contains(&st.intern_segs("mark.you.foo")));

        let (mut st2, ga2) = probe(Action::new("[Actor]: act").then([delete("gone.Actor")]), vec![]);
        let a2 = st2.grounded_delta_anchors_of(&ga2).expect("delete bounded");
        assert!(a2.contains(&st2.intern_segs("gone.you")));
    }

    #[test]
    fn exclusion_insert_carries_shadows() {
        let (mut st, ga) = probe(
            Action::new("[Actor]: act").then([insert("slot!gem.Actor")]),
            vec![],
        );
        let anchors = st.grounded_delta_anchors_of(&ga).expect("bounded");
        // The path itself plus the eviction shadow (slot!PraxEvicted).
        assert!(anchors.contains(&st.intern_segs("slot.gem.you")));
        assert!(anchors.contains(&st.intern_segs("slot.PraxEvicted")));
    }

    #[test]
    fn spawn_headed_inserts_are_opaque() {
        // Literal `practice` head → could spawn → opaque.
        let (mut st, ga) = probe(Action::new("[Actor]: act").then([insert("practice.Actor")]), vec![]);
        assert!(st.grounded_delta_anchors_of(&ga).is_none());
    }

    #[test]
    fn unknown_call_is_opaque_known_call_resolves() {
        // Unknown function → wild → None.
        let (mut st, ga) = probe(
            Action::new("[Actor]: act").then([call("missing", vec!["Actor".into()])]),
            vec![],
        );
        assert!(st.grounded_delta_anchors_of(&ga).is_none());

        // Known function → resolves through its case outcomes.
        let f = Function::new("tidy", ["Who"]).case(vec![], vec![insert("clean.Who")]);
        let (mut st2, ga2) = probe(
            Action::new("[Actor]: act").then([call("tidy", vec!["Actor".into()])]),
            vec![f],
        );
        let anchors = st2.grounded_delta_anchors_of(&ga2).expect("call resolves");
        assert!(anchors.contains(&st2.intern_segs("clean.you")));
    }

    #[test]
    fn safe_foreach_binder_bounds_unsafe_head_opaque() {
        // A ForEach binding X at a NON-first position of a positive Match, then an
        // insert headed by X → safe binder → bounded (X kept as wildcard).
        let safe = Action::new("[Actor]: act").then([for_each(
            vec![m("item.Actor.X")],
            vec![insert("X.done")],
        )]);
        let (mut st, ga) = probe(safe, vec![]);
        assert!(st.grounded_delta_anchors_of(&ga).is_some(), "safe binder bounds the head");

        // A ForEach binding X at the FIRST position → NOT safe → the X-headed
        // insert could ground to `practice` → opaque.
        let unsafe_ = Action::new("[Actor]: act").then([for_each(
            vec![m("X.tag.Actor")],
            vec![insert("X.done")],
        )]);
        let (mut st2, ga2) = probe(unsafe_, vec![]);
        assert!(st2.grounded_delta_anchors_of(&ga2).is_none(), "first-position binder is unsafe");
    }
}
