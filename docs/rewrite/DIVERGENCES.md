# Adjudicated divergences — where the Rust is right and the Haskell is not

Per the program's ruling (docs/rewrite/PLAN.md): divergences are adjudicated
against the SPECS; Haskell bugs are never reproduced and never patched;
entries here carry the spec authority, the fiction consequence, and the
comparator posture.

## DIV-1 (S3): the frozen semi-naive closure is incomplete on cross-predicate aggregate bodies

**What**: `Prax.Derive.deltaJoin` (Derive.hs:120-128, both `run` and
`runCooked`) seeds only top-level `Match` positions from the delta. A rule
whose Exists/Subquery/Count/Or condition reads a DERIVED predicate disjoint
from its Match seed never re-fires after the Match fact leaves the delta —
the closure silently under-derives. Proven by live probe (recorded as the
negative fixture in conformance/fixtures/): `[Match p.X, Exists[Match q.Y]]
→ r.X` with `[Match trigger] → q.thing`: frozen omits `r.a`; the closure
contains it.

**Spec authority**: the view "IS the closure of the base under the axioms"
(ViewInvariantSpec's stated invariant); the frozen code documents semi-naive
as an optimization ("nothing already known is re-derived") — an optimization
that changes results contradicts its own contract. The Rust implements the
naive-equivalent closure (static fast-path/full-eval rule split; the
naive==production law is the flagship property).

**Fiction consequence**: none in any shipped world — their aggregate
conditions read the same predicate as their Match (shape luck, now recorded).
A future world with a cross-predicate aggregate axiom derives correctly on
Rust and incorrectly on the frozen Haskell.

**Comparator posture**: no suppression needed — shipped traces agree. The
negative fixture pins both outputs so the divergence is a committed artifact.

## DIV-2 (S6): a separator-bearing character name is accepted by the frozen engine and read three different ways by its own planner

**What**: nothing in the frozen engine forbids a character name containing a
path separator (`.` or `!`) — `Prax.Types.character` is a bare record build and
`setCharacters` has no guard. Supply one and the frozen planner disagrees with
ITSELF about what the name means, because the same name reaches three sites
through three different surfaces:

- `Prax.Relevance.moverReadAnchors`' death anchor (Relevance.hs:284) is
  `map intern (pathNames (deadSentence (charName m)))` — `pathNames` SPLITS, so
  a mover named `hall.keeper` yields the 3-segment anchor `[dead, hall, keeper]`.
- The same function's `scopeReads` (Relevance.hs:281) grounds at the SYM level
  (`groundCookedCondition`), substituting one Sym for one segment — the name
  stays a single segment.
- `Prax.Planner.inScope` (Planner.hs:74) grounds through
  `groundCondition` → `Prax.Db.ground` = `tokensToSentence (groundTokens …)`,
  which rebuilds a STRING that `query`'s `Match s -> pathNames s`
  (Query.hs:133) then RE-SPLITS — so here the name splits again.

The consequences are behavioral, not cosmetic. `mayUnifySyms` truncates to the
shorter path and compares segment-wise, so a 2-segment delta anchor
`[dead, "hall.keeper"]` matches a 2-segment read anchor but fails against the
3-segment one — the reuse gate fires on one reading and misses on the other,
giving opposite reuse decisions on the same world. Likewise a prediction scope
`[Match "near.Actor"]` becomes `[near, hall, keeper]` under `inScope` and
`[near, "hall.keeper"]` under the sym-level path: the mover is included in or
dropped from the imagined round, moving the score and possibly the pick. The
RENDERED name is identical in every case, so no dump can see the difference.

**What the Rust does**: `State::set_characters` REJECTS any name containing `.`
or `!` with `WorldError::MultiSegmentCharacterName`, naming the character and
the reason. The class becomes unreachable rather than silently divergent. The
port is also made self-consistent regardless of the guard: `mover_read_anchors`
builds its death anchor with `tokenize(interner, "dead.<m>")`, the same
tokenizer `candidate_actions` and `living_characters` already use, so all three
Rust sites agree on how a name segments (pinned by
`the_death_read_anchor_is_tokenized_like_every_other_death_sentence`, which
calls `mover_read_anchors` directly with `hall.keeper` because the guard makes
the difference unobservable through `State`).

**Spec authority**: single-segment naming is the established convention for
every other engine-facing name — schedule-rule names carry an explicit
construction guard in the frozen engine itself
(`Prax.Engine.addScheduleRules`: "schedule rule name must be a single
segment"), and `ScheduleRule`'s `srName` is documented "single segment; the
persist re-association key" (Types.hs:213). A character name is spliced into
engine-built sentences exactly as a rule name is spliced into its due key:
`dead.<name>`, `<p>.believes.desires.<name>.…`, `practice.<pid>.<role>.<name>`.
A separator inside it nests the character's fact families under one another and
splits its death mark and scope anchors across segment boundaries. That is a
path injection, not a name — so the guard is the rule the frozen engine already
states, applied at the door that was missing it.

The alternative — routing the Rust `in_scope` back through the string surface
to match `inScope` — was rejected: it would reproduce an implementation
accident (frozen's own `moverReadAnchors` does NOT do this), and the program's
ruling is that Haskell bugs are never reproduced.

**Fiction consequence**: none. No shipped world uses a separator-bearing
character name. A world that tried one gets a loud construction-time error
instead of two engines quietly telling different stories about the same cast.

**Comparator posture**: no suppression. Frozen traces never contain such a
name, so no fixture, corpus row or replay is affected — the planner and npc
corpora regenerate byte-identically across this change. The divergence is
purely about what the two engines ACCEPT: everything the frozen engine accepts
and the Rust also accepts, the two agree on bit for bit.

## DIV-3 (S7): the vocab combinators' name guards are EAGER, where the frozen ones fire only when the built sentence is forced

**What**: `Prax.Faction.memberPath` guards its two names with `error`, and
`Prax.Faction.joins who faction = Insert (memberPath who faction)` puts that
guard inside a lazily-forced `String`. So the frozen guard fires whenever some
consumer forces the path — and NEVER for a consumer that forces only the list
spine. `Prax.Kin.wed` inherits this exactly: it checks `joiner` and `spouse`
itself and leaves `faction` to the `joins` it builds. Both observations verified
on the frozen engine at the same input:

```
HASKELL length (wed "ana" "b.ad" "cass") -> 2
HASKELL show   (wed "ana" "b.ad" "cass") -> ERROR: Faction: names must be nonempty
        single path segments (no '.' or '!'): ("ana","b.ad")
```

The frozen `Prax.KinSpec` (KinSpec.hs:122-127) contains exactly such a
spine-only observer: it asserts on `length (wed …)`.

**What the Rust does**: `member_path`, `joins` and `wed` return
`Result<_, WorldError>`, so the rejection happens at CONSTRUCTION —
`wed("ana", "b.ad", "cass")` is `Err(NotASinglePathSegment)` with no forcing
involved.

**Spec authority**: the guard exists to stop a malformed name being spliced into
a built path (`member.<who>!<faction>`), which is a construction-time property of
authored data; the program's established posture is that such guards fire at the
door (DIV-2 makes the same argument for character names, and the frozen engine
itself guards schedule-rule names at construction). A guard whose firing depends
on which observer happens to force the value is not a guard on the data — it is a
guard on the consumer. Rust also has no lazy `String` to hang an `error` inside:
the alternative would be a panicking accessor, which the rewrite forbids
outright.

**Fiction consequence**: none. Every real consumer forces the path (that is what
performing an outcome does), and for every such consumer the two engines are
identical: same guard, same input, same rejection. The difference is observable
only by an observer that never looks at the value it is guarding.

**Comparator posture**: no suppression. The deviation cannot appear in a trace —
a world reaching a turn has already forced every setup outcome, so a world whose
`wed` carries a bad faction never reaches the comparator on either side; it fails
to build on one and fails at its first force on the other. Pinned natively by
`kin::tests::wed_rejects_a_bad_faction_at_construction`, which asserts the Rust
behaviour at the exact input the frozen spec observes spine-only.

## Recorded posture (not a DIV): the ⊥-witness is selected by name order

When a closure round forces two or more distinct values into one exclusive slot,
BOTH engines report a single ⊥ witness — but they select it differently. The Rust
sorts the round's fresh heads by rendered name and folds the meet in that order,
so the reported witness is the name-least conflicting head (`derive.rs` `run`,
design I4). The frozen engine folds `foldM meetOne` in `nub` (generation) order,
so it reports the first conflicting head in generation order. The DeriveSpec pin
is stated up-to-set ("names AN offending head", DeriveSpec:75) and the flagship
`naive == production` law is internally consistent (both closures share the same
sort+fold), so this selection is verified against the naive oracle, not against
frozen's `nub` order.

This is NOT a divergence, because no shipped world produces ⊥ during closure:
`derive.json` has zero contradiction cases, and kin/div1 never force a conflicting
exclusive slot, so `check_closure_case`'s exact-witness comparison is never
exercised against frozen and no trace can differ. It is recorded here PRE-EMPTIVELY
so that a future ⊥-bearing fixture whose Rust witness differs from frozen's is read
as this known name-order-vs-nub-order selection difference (still up-to-set correct),
not mistaken for a fresh correctness divergence. Should such a world ever ship, this
posture graduates to a numbered DIV with a comparator suppression on the witness
field.

## DIV-4 (S7): `asRole` retargets a co-presence template by RENAMING, not by grounding

**What**: `Prax.Witness.asRole` is
`map (groundCondition (Map.singleton (intern "Witness") (VSym (intern v))))` —
it substitutes through `Prax.Query.groundCondition`, whose token round-trip goes
through `Prax.Db.tokens` and so RAISES on a malformed template. The Rust
`prax_vocab::witness::as_role` substitutes through S4's
`prax_core::types::rename_vars` instead: pure, infallible, operator-preserving.

**Why**: `ground_condition` needs a `&mut Interner` and returns a `Result`. Every
caller of `as_role` is a pure value-builder in `prax-vocab`
(`Rumor.gossip`, `Deceit.lie`, `Confession.confess`, and `Blackmail`'s trigger and
punish conditions), so threading an interner and a rejection through it would
infect five combinator signatures — and their callers — for a fallibility no
shipped template can reach. S7 design [S-I6] rules for `rename_vars`, with this
entry, a WitnessSpec equality pin, and a stated contract as the price.

**Where they agree, verified rather than argued**: on the frozen engine at the
shipped template, and on the Subquery binder shape [S-I6] names as the thing to
check —

```
HASKELL asRole "Hearer" [Match "…at.Actor!P", Match "…at.Witness!P"]
     -> [Match "…at.Actor!P", Match "…at.Hearer!P"]
HASKELL asRole "Hearer" [Subquery {subSet="Witness", subFind=["C"], subWhere=[Match "at.C!P"]}]
     -> [Subquery {subSet="Hearer", subFind=["C"], subWhere=[Match "at.C!P"]}]
```

Both are reproduced value-for-value by
`prax_vocab::witness::tests::as_role_agrees_with_ground_condition_on_every_shipped_template`,
which builds the frozen implementation (`ground_condition` under a
`Witness → VSym v` binding) as the ORACLE and compares against `as_role` over
every shipped `CoPresence` and every role the shipped call sites retarget to.

**Where they differ, and the contract that closes it**:

1. *Fallibility.* `groundCondition` rejects a template whose sentence ends in an
   operator; `rename_vars` renders the malformed sentence and carries on.
   Observed on the frozen engine:
   `asRole "Hearer" [Match "at.Witness!"] -> ERROR -> Prax.Db.tokens: trailing
   operator '!' in "at.Witness!"`. Pinned in both directions by
   `the_two_implementations_differ_off_the_shipped_shape`. No shipped template is
   malformed, and `worldshape`'s bodies comparison holds each world's template
   identical to the frozen one, so a malformed template cannot arrive unnoticed.
2. *Substitution rule.* `groundTokens` substitutes only segments the tokenizer
   classifies as VARIABLES; `rename_sentence` substitutes any segment whose name
   matches. These coincide because the substituted key is the fixed, capitalized
   `Witness`, which is always a variable — which is why the key is not a
   parameter of `as_role`.

**Stated contract**: the replacement `v` MUST be a variable name. The frozen
function substitutes a `VSym`, so a non-variable replacement yields a template
that no longer quantifies — an authoring error under either implementation, but
only this one would carry it silently. Asserted, not merely written down, by
`as_roles_contract_the_replacement_is_a_variable`.

**Fiction consequence**: none. All five shipped call sites substitute a variable
(`Hearer`, or a victim's own bound variable), no shipped template is malformed,
and no shipped template carries a Subquery binder at all.

**Comparator posture**: no suppression. `as_role` is not reached by any slice-3
world (Bar authors `CoPresence` but never retargets it); slice 4's Village
worlds exercise all five call sites, and their traces are compared with no
adjudication.

## Recorded posture (not a DIV): due expiries fire in rendered-name order

At a round boundary (`roundBoundary`, spec v44) the engine fires every due
expiry BEFORE any due rule. The frozen engine iterates the due set in
`Map [(Sym, Maybe Char)]` `Ord` order — that is, INTERN-ID order,
interleaved by whenever each labeled path was first interned — and folds a
guarded `CDelete` over it. The Rust fires the same due set in RENDERED-NAME
order (`engine.rs` `round_boundary_impl` sorts the due paths by their labeled
sentence before `fire_due_expiries_in`), because the runtime keys `expiries`
by `CompiledPath` in an `FxHashMap` whose iteration order is incidental by
design (S4 [S-panel I1]); the determinism contract then names name-order as the
one observable enumeration order.

The two firing orders reach the SAME state, so this is a posture note, not a
numbered DIV. Expiry firing COMMUTES: each `CDelete` fires only if its exact
fact still `exists` (the existence guard), and a subtree retract subsumes its
descendants — so for any two due paths, whether disjoint or in an
ancestor/descendant relation, either order removes exactly the same nodes and
leaves the same queue. The v50-era soundness note records that all due expiries
fire before any rule reads the state, so no rule can observe a partial firing
order either. The commutation is pinned as a law (`engine.rs`
`boundary_props::expiry_firing_commutes`: any two firing orders → same
`labeled_facts`/`labeled_view`), and the boundary's insensitivity to the
HashMap's incidental layout is pinned end-to-end
(`conformance/schedule_spec.rs` `round_boundary_is_insertion_order_insensitive`).

This differs from the same-boundary RULE firing order, which IS observable and
IS the contract: due rules fire in DECLARATION order (frozen `cookedSchedule`
order; Rust `compiled.schedule` order), pinned by ScheduleSpec law 8b
(`law_8b_due_rules_fire_in_declaration_order`). Rules do not commute — a
period-1 opener whose effect a later same-boundary rule reads must fire first —
so name-order would be WRONG there; only the expiry retracts, which commute,
are reordered.

The keep-entry case is covered explicitly: an ancestor firing purges a
not-yet-due descendant's QUEUE entry; because due entries leave the queue
before any firing, the purge set is the same whichever order the due set
fires in — the surviving queue, not just the surviving facts, is
order-independent (pinned by the mixed-lifetime commutation proptest).

## DIV-5 (S8): the `"memories"` guard — a guard whose whole purpose is loudness — is silent on an explicit `null`

**What**: `Prax.Script.Json`'s `FromJSON Scene` rejects a scene object carrying
the pre-v46 `"memories"` key, LOUDLY, because the decoder otherwise ignores
unknown keys and a script authored against the deleted memory feature would
decode with its content quietly dropped — the same "same bytes, different
meaning" stance `Prax.Persist`'s v3 format bump takes for saves. The guard is
`hasMemories <- isJust <$> (o .:? "memories" :: Parser (Maybe Value))`
(`Json.hs:207`).

aeson's `.:?` maps an explicit JSON `null` to `Nothing` exactly as it maps a
MISSING key. So the guard does not fire on `"memories": null`. Measured on the
frozen decoder:

```
{"id":"a","memories":[]}    -> Left "… Scene's \"memories\" field is no longer supported …"
{"id":"a","memories":null}  -> Right (Script {…, sceneId = "a", sceneOpening = "", …})
```

The hole sits at precisely the JSON spelling an author is most likely to leave
behind when half-deleting a field.

**What the Rust does**: `prax_script::json`'s `check_no_memories` fires on the
KEY BEING PRESENT, `null` included. The Rust rejects a strict superset of what
the frozen rejects, in the direction the guard was written for.

**Spec authority**: the guard's own stated purpose. Its documentation says it
exists so a pre-v46 script cannot decode "with that content quietly dropped" —
a guard against silent key-dropping that is itself silent on one spelling does
not do the job it declares. This is an implementation bug in a loudness
mechanism, not a contract: the program's ruling that Haskell bugs are never
reproduced applies to it cleanly.

It applies FAR more cleanly here than to the neighbouring
ignore-unknown-keys behaviour, and the two must not be fused. Tolerating unknown
keys IS the frozen contract — a `deny_unknown_fields` port would reject
forward-compatible files the frozen accepts, which is a strictness FORK question
(S10's, if anyone's), not hygiene. The Rust reproduces ignore-unknown-keys
exactly (`prax_script::json::tests::unknown_keys_are_ignored_except_memories`)
and strengthens only the one guard whose declared job is to be loud.

**Fiction consequence**: none. No shipped file carries the key in any spelling
(`grep -rn "emor" src/ app/ test/ examples/` finds only the guard and its own
test), and a file that did was already broken content under either engine — the
difference is whether the author is told.

**Comparator posture**: no suppression. The divergence is unreachable from any
trace: it is a decode-time rejection, and no shipped world is loaded from a file
carrying the key. Pinned in both directions by
`conformance::script_json_spec::a_scene_json_carrying_a_removed_memories_field_is_rejected_loudly`,
which asserts the array spelling (where the two engines agree) and the `null`
spelling (where they do not) side by side.
