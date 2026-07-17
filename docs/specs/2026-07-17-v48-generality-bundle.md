# v48 — The generality bundle: five hardcodings unpicked; leverage graduates to its own round

Fourth of the audit-queued rounds (v45 → v46 → v47 → **v48** → v49 leverage). Rewritten
after the three-lens isolated panel (`.superpowers/sdd/v48-spec-review-*.md`) — whose
sharpest result was catching the same factual error through three independent routes,
and whose design lens concluded the Blackmail item cannot ship inside a bundle at all.

## What the panel changed, headline first

1. **The first draft's deontic world-census was wrong at both ends** [all three
   lenses, independently]: Bar never lifted (it has no axioms — its tipping obligation
   is *producibility*, not lifting); the lifting worlds are **Village and Feud**; and
   Village is a SECOND deontic consumer (the shakedown's `comply → owe → oblige`
   statically produces `obliged.eve.favor`). The reviewer's note stands as the round's
   argument-in-miniature: the spec author, writing carefully with the records open,
   still miscounted the deontic worlds — which is precisely why the gate must be
   DETECTION, not a hand-set flag.
2. **Blackmail (old item 2) leaves the bundle** — three lenses converged: as amended it
   was an incoherent chimera (general punishment and motivation bolted to a mandatory
   evidence trigger, forcing the flagship non-exposure test to author a FAKE evidence
   pattern — the audit's defect class reintroduced at the trigger); its "fidelity
   restoration" framing over-claimed (v30 approved the epistemic model — the
   motive-belief deposit; three-axis parameterization is genuinely new design); and it
   punted its one load-bearing decision (the mechanism/content boundary) to the plan.
   It restructures a validated shipped mechanic and is **queued as v49**, its own round
   with a pre-gate panel — see the closing section for the design fork and the five
   binding mechanism constraints the panel established. NOT banked: queued next, per
   the standing directive that this defect class is never left around.

## 1. □-lifting gates on deontic producibility

`liftObliged` adds an `obliged.Obligor.*` twin for every all-Match axiom in every
axiom-bearing world. Corrected census: **Village and Feud** carry lifted rules today;
Village can produce `obliged.*` (comply → owe) and KEEPS them; **Feud cannot** (no
Deontic/Debt/Blackmail import; heads are kinship/feud families) and its lifted twins —
genuinely unfireable — VANISH. The gate: lift iff the world can produce an `obliged.*`
fact.

- **Detection, not a flag**: an author using obligations must get DEON property 1
  automatically; the panel's own census error is the demonstration that human
  enumeration fails silently.
- **The decision pool** is the UNLIFTED producers: practice insert atoms, schedule-rule
  insert atoms, db facts AS OF RETABLE TIME, and unlifted axiom heads. Two panel-forced
  precisions: (a) do NOT reuse `producibleAtoms` — it reads `cookedRules` (a laziness
  cycle against the field being computed) and includes lifted heads (self-fulfilling:
  the gate would always lift); the pool is a new query over the unlifted inputs. (b)
  The db leg sees only facts present at retable time — the stated build-order
  invariant: obliged-producing setup facts must precede the final retable (both
  axiom worlds already build `setAxioms`-outermost; the invariant is now a stated
  contract, and a violation is the author's, documented at `setAxioms`).
- **Mechanics**: `cookedRules` re-homes from `setAxioms` into `retable`; `setAxioms`
  becomes retable-then-reclose. This IS engine-internals surface [panel F5, accepted]:
  the ordering guarantee `setAxioms` currently owns (cookedRules set before its
  reclose) must be re-established and the SETTER-COHERENCE INVARIANT stated and
  pinned — every producer-changing setter (definePractices, defineFunctions,
  setSchedule, setDesires, setCharacters, setAxioms) leaves the lift decision current;
  the verification generalizes the after-the-fact pin to every setter, not just the
  oblige-practice-added-after-setAxioms seed case.
- **The string reference path stays ungated BY DESIGN** [panel, completeness I1]:
  `Derive.run`/`closure` are pure `[Axiom] → Db` with no producer pool and always
  lift. Equivalence with the gated `runCooked` holds exactly when the gated-out rules
  are unfireable — so **ViewInvariant doubles as the gate's soundness net**: if
  detection ever wrongly skips a producible world, the view diverges and the net
  fires. This is stated in both modules' docs.
- **Consumers named**: `axiomDerivable` walks lifted heads into
  `improvables`/`liveness`/`caresAbout` — verified safe for Feud (its wants never
  unify `obliged.Obligor.*`) and moot for Village; the plan carries the check.
- **Exactness**: exactly ONE pin changes — `feudPin` loses its 8 lifted rows
  (footprint ×6, axiomHead ×2), itemized. `villagePin` and all others BYTE-IDENTICAL.
  All goldens byte-identical everywhere (vanished rules were unfireable). Any other
  movement = BLOCK.

## 2. Confession's discharge verb parameterizes

`confess` hardcodes `confessed` into the discharge path. The verb becomes an argument
(recant, boast, admit — same machinery); shipped sites pass `"confessed"`; behavior
identical, goldens byte-identical.

## 3. Stress's coverage family parameterizes

`sceneReached` hardcodes Script's `currentScene`. Coverage becomes a declarable family
parameter, defaulted by the CLI's script entry points to `currentScene`. The plan
carries the full consumer list (stressTest/runRandom/StressReport signatures,
StressSpec AND the CLI stress entry, ScriptSpec's coverage uses) [panel I7].

## 4. `disapprovalP` moves to its consumer

Shipped content in the `Reactions` mechanism module moves to `Prax.Worlds.Bar` (its
only consumer); `ReactionsSpec` builds its own minimal reaction so the mechanism keeps
unit coverage independent of Bar's content.

## 5. `feelingSomeone` collapses into `feelingToward`

A literal alias since v39. One name survives; the per-target-pricing guidance moves to
the survivor's haddock; Village's `smoulders` and the specs re-point.

## Exactness (bundle-wide)

Items 2-5: goldens and pins byte-identical. Item 1: `feudPin` −8 rows, itemized;
everything else byte-identical. Persist untouched; no format bump.

## v49 — leverage, queued (the panel's verdict and the design fork)

The Blackmail generalization is REAL work the audit correctly demanded, and it gets a
real round, immediately after this one. The panel's two coherent designs:

- **(a) Information-leverage charter**: evidence stays mandatory AND punishment stays
  parameterized *exposure* — the module remains "blackmail," and the burned-barn
  application is out of charter. REJECTED under the standing directive: the audit's
  second-application member ("dig my field or I burn your barn") would remain
  inexpressible — the defect class survives.
- **(b) General coercion primitive** (the v49 design): the
  threaten/comply/defy/punish skeleton + motive-belief deposit + prediction
  credibility becomes an evidence-OPTIONAL primitive; blackmail is a thin instance
  adding the evidence gate and exposure punishment; a protection racket is another
  instance. This kills the class member fully.

Five mechanism constraints the panel established, binding on the v49 design:
1. The credibility deposit's desire-name derives from the authored punitive want's
   `desireName`, and the want must be registered (`setDesires`) and held
   (`charDesires`) — else the threat is silently non-credible [soundness I3].
2. A demand-independent compliance marker replaces the debt-shaped re-buy guard —
   repeat extraction stays impossible for every demand kind, and the v49 verification
   drives a RE-threat after compliance [soundness I4].
3. The standing-threat `Or [threat, defiance]` disjunction survives in BOTH the punish
   action's availability and the authored want; punishment is availability-gated with
   only its EFFECT authored; the verification drives punish against a STANDING threat
   [soundness I5].
4. The new authored surfaces (demand, punishment, want) carry the v40 splice guards
   [completeness I6].
5. The mechanism/content boundary (which fragments of the old reveal are exported
   mechanism vs Village-authored content) is decided IN the v49 spec, not its plan
   [design F4]; BlackmailSpec's exact v30 arithmetic (−63.84 …) is preserved under
   whatever call-site reshaping lands [completeness I4].

## Out of scope

v49 (queued, above). Sort scoping that models Call bindings (v47's recorded
successor, unforced).
