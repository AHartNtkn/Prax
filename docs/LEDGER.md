# Feature Ledger

Every capability we intend `prax` to support, derived from the Versu paper and Praxish. Status:
- **v1** — in the first milestone (faithful engine core).
- **v2** — the core-model round (emotions & relationships), built as `Prax.Core`.
- **v3** — reactions-as-practices & a first norm, built as `Prax.Reactions`.
- **v4** — per-agent beliefs (incl. false beliefs), built as `Prax.Beliefs`.
- **v5** — conversation (speakers, topics, quips), built as `Prax.Conversation`.
- **v6** — a story manager (DM) as a metalevel agent (the bar `director`).
- **v7** — character arcs (internal high-level state), built as `Prax.Arc`.
- **v8** — first-order connectives in the query language (∀/∃/∨/→).
- **v9** — cast removal + a dramatic vertical slice (`Prax.Worlds.Intrigue`).
- **v10** — QA tooling: the inspector (`Prax.Inspect`) and stress-test (`Prax.Stress`).
- **v11** — persistence: save/load a session (`Prax.Persist`), CLI save + resume.
- **v12** — a Prompter-lite scene-authoring layer (`Prax.Script`) that compiles a
  CAST + scene-graph play-script to practices, with an auto flow-chart + scene coverage;
  demonstrated by `Prax.Worlds.Play`. Play-scripts round-trip through readable JSON
  (`Prax.Script.Json`) — the editable authoring format, chosen over a bespoke parser.
- **v13** — player-as-DM: the human occupies the drama-manager slot, steering an
  autonomous cast with metalevel nudges (`Prax.Worlds.Bar` `barDirectorWorld`, `prax dm`).
- **v14** — a first-class deontic `should`/obligation layer (`Prax.Deontic`): `□φ` as the
  fact `obliged.<who>.<φ>`, conflict detection via the `!` exclusion, and contrary-to-duty
  (`□□`); the bar's settle-up is now a real obligation.
- **v15** — a forward-chaining **derivation layer** (`Prax.EL` + `Prax.Derive`): domain rules
  closed to a fixpoint via the paper's `m(X)`, exact `⊥` detection, auto-`□`-lift (obligation
  closure), a defeasible closed *view* on the read path; `Prax.Worlds.Feud` is the emergent demo.
- **v16** — a static **well-formedness checker** (`Prax.TypeCheck`): unbound-variable,
  exclusion-cardinality, and dangling-reference checks over a world's authored sentences (`prax check`).
- **v17** — **ML-style sort inference** completing #8: sorts declared by membership, every
  position/variable sort inferred by unification, conflicts reported (`Prax.TypeCheck`).
- **v18** — the remaining **Prompter compilation features** in `Prax.Script`: memories (one-shot
  exposition), timed junctions (a scene clock), and character sketches (concerns→wants, traits→facts).
- **v19** — **quantified outcomes** (`ForEach`, the dual of v8's condition quantifiers) and
  **authored witnessing** (`Prax.Witness`): co-present characters come to believe an action
  happened, with `.seen` provenance (multi-valued from v20 — see below); observability is a
  semantic property the author states, not an automatic event log. `Prax.Worlds.Village` seeds the
  sandbox arc (`prax village`).
- **v20** — **sourced rumor propagation** (`Prax.Rumor` `gossip`/`heard`): a character tells a
  co-present hearer what they have evidence for, planting the same event-belief with hearsay
  provenance. Provenance becomes **multi-valued** (`.seen`/`.heard.<source>`, replacing v19's
  exclusive `!seen`), so witnessing and hearsay for the same event coexist instead of one
  overwriting the other, and corroboration (multiple named `.heard.<source>` edges) is countable.
  `Prax.Worlds.Village` grows: carol spreads the theft on her own; hearsay licenses
  `eye … with suspicion`, never `confront` (eyewitness-only); a world-authored relationship gate
  lets distrust close the gossip channel.
- **v21** — **derived reputation** (`Prax.Repute` `standing`/`standingUnless`/`regardedAs`/
  `notoriety`): `regards.<observer>.<subject>.<label>` is never stored, only *derived* from an
  observer's evidence (seen or heard alike) — so it inherits information asymmetry and
  defeasibility for free. Standing is defeated by **atonement, not amnesia**: `standingUnless`
  guards the derivation with a *base-fact* defeater, dissolving every regard on one insertion while
  every belief (the memory of the deed) persists untouched — and because the belief never went
  away, **re-offense revokes the defeater**, so standing and notoriety snap back instantly on a
  repeat. `notoriety` turns corroboration into a threshold-gated global fact (an authored world
  parameter). `Prax.Worlds.Village` completes its arc: theft → witnessing → rumor → three regards
  → notoriety tips bob into atonement → the village relents — and, because the planner can see the
  snap-back, an atoned thief facing a restocked stall is *deterred*, never touching it again.
- **v22** — **secrets & deception** (`Prax.Deceit` `conceal`/`lie`): a concealment want — that
  nobody believe some deed (`Absent [Anyone.believes.<event>]`) — makes an agent avoid witnesses
  *by planning*: the lookahead already simulates v19's witness deposits, so waiting for privacy
  falls out of ordinary utility, no stealth system needed. A lie is an assertion without evidence:
  `lie` mirrors v20's `gossip` and plants the identical `.heard.<liar>` hearsay, so a fabrication
  is indistinguishable from truth to everyone but the liar, and the whole rumor/reputation stack
  (v20/v21) cascades on it unmodified — hearing your own lie back replaces the lie action with
  plain gossip, seamlessly. `Prax.Worlds.Village` gains a villain: bob conceals his theft, waiting
  out a genuinely watched square (walking away alone isn't enough — carol keeps her own post; the
  bread is safe exactly as long as *someone* is watching); eve, out of authored malice, frames
  carol, and the frame-up settles into regard, shunning, and notoriety exactly as truth would —
  with an honest injustice at the end: framed carol has no recourse (amends needs a loaf she never
  took; exculpation needs ground-truth event records — an idea banked here, then **rejected in
  v25**: the vocabulary's refusal to fake ground truth is a stated commitment, not a gap; see v25's
  banked-item rewrite below).
- **v23** — **realistic lookahead: round-walk over believed minds** (`Prax.Minds`, `Prax.Sight`,
  a rewritten `Prax.Planner`; spec `docs/specs/2026-07-10-v23-planner-realism-design.md`). The old
  lookahead's `worldValue` (now **deleted**) maxed over every living character's every action,
  scored by the *planning actor's own wants* — three demonstrated failures: speculative (credited
  others with actions they would never choose, e.g. carol's top move became an unevidenced
  accusation), omniscient (used movers' *true* wants, so a secret plot was foreseeable by anyone),
  and combinatorially explosive (v22's village suite: 8.7s → 621s). Replaced by a round-walk: each
  other character within the actor's **epistemic scope** (`predictionScope`, the v19 co-presence
  template — default everyone) gets one myopic, *motivated-only* move predicted from the actor's
  **believed** model of them (`predictMove`), in cast order, before the actor recurses on its own
  next choice (`scoreActions`, `pickAction`). A mind the actor holds no belief about, or a mover
  out of scope, is modeled as still, never as helpful — and the model can be *wrong* (prediction
  uses the actor's beliefs, not the mover's true wants). Desires become nameable and believable
  (`Prax.Minds`: owner-parameterized `Desire` templates, `charDesires`; motive-beliefs reuse the
  v20 provenance shape unchanged, so gossip/lie/confide/forget all work on
  `desires.<owner>.<name>` for free); "public"/"secret" is recovered as derived, defeasible common
  knowledge (`professed`/`conventional` → `.presumed`) rather than a flag. `Prax.Sight` adds
  sightings as ordinary location-beliefs (`believes.at`/`atSince`), maintained by a compiled
  per-round ticker (`turn!N`, the v18 `_clock` idiom) — "who's around" is itself information with
  an authored horizon, not global. Intrigue's plot is now a believed mind (a confidant's
  `predictMove` of cassia foresees the poisoning; the victim's does not; a leak flips it). Two
  world edits were needed outside Intrigue and are recorded honestly in the spec's §6: the
  village's `dana` gets a sanctioned mill-anchoring want (the same idiom `bob`'s stall-anchor
  already used), and the bar's `LoopSpec` golden-trace turn budget is re-derived from 20 to 25
  (5 rounds × a cast grown by one for the sight ticker) — the narration itself is unchanged.
  Master suite: 5.5s → 0.8s (the rewrite's own speedup). The true referee — v22's village suite,
  once landed on top — bears this out directly: ~19s (`cabal test --test-options='--pattern
  "Prax.Worlds.Village"'`), down from the 621s blowup, a >30× recovery, at the *original* pre-blowup
  order of magnitude (8.7s).
- **v24** — **endeavors: staged practices with dormant pursuits** (`Prax.Project` `endeavor`/
  `Stage`; spec `docs/specs/2026-07-11-v24-project-design.md`). A project *type* is authored
  vocabulary, like a practice or a desire; `endeavor pid weight undertakeLabel gate stages`
  compiles it to three things a world wires in: the undertake `Action`, a staged `Practice`
  (one instance per owner — undertaking twice is never offered again), and a named pursuit
  `Desire` that counts completed stages (`practice.<pid>.Owner.done.S`) at `+weight` each.
  Progress itself is the reward, so horizon length stops mattering — every next stage is
  locally visible to the ordinary planner, no lookahead change needed; this is the concrete
  case that finally **closes #21's runtime-want-injection question by worked pattern**, not
  just by claim (see #21's updated note, below): the pursuit desire is *dormant* — zero
  bindings, zero utility — for any disposed character with no instance yet, and undertaking
  (an ordinary planner choice) is exactly "injecting a want by inserting a fact." Because the
  pursuit is a named, nameable desire, an endeavor is automatically theory-of-mind content
  (`Prax.Minds`): whoever comes to believe a character pursues one gets it fed straight into
  `predictMove`. `Prax.Witness`'s `witnessed` outcome-builder is extracted as a first-class,
  exported combinator (previously folded only inside `observable`) so a generated stage can
  carry public observability in its own effects — `Prax.Project` needed this, not a new
  primitive. `Prax.Worlds.Village` closes its own moral arc on this: bob — deterred since v21,
  concealing since v22 — takes up `earnBread` from a clean t=0 free-play start: undertakes at
  the stall, sweeps the square in public, walks to the mill, fetches flour, returns, and bakes
  the loaf he could no longer safely steal, done by turn 32 of the same running world every
  earlier round used. Watching him sweep is enough for the village to learn his purpose (a
  one-line inference axiom presumes the pursuit for anyone who believed the sweep), and once he
  stands at the mill, `predictMove` anticipates the flour trip specifically for whoever holds
  that belief — myopically (no prediction at the square, where no stage is yet available) and
  belief-relative (co-present, unbelieving dana predicts nothing, proving prediction reads the
  predictor's beliefs, not the mover's true state). The opportunism stays honest: mid-project,
  with the square genuinely empty, stealing (71.18) still beats pressing on to the next stage
  (50.46) — industry is chosen because it's watched and safe, not because temptation stopped
  scoring. One sanctioned test amendment (spec §3): "an atoned thief is deterred" swaps its
  stale "bob holds no loaf" proxy for a direct non-re-offense assertion
  (`practice.earnBread.bob.done.s3`), since the endeavor now gives bob a lawful loaf the
  original proxy couldn't anticipate.
- **v25** — **persona: traits as conduct-valuations** (`Prax.Persona`; spec
  `docs/specs/2026-07-11-v25-persona-design.md`). A first draft (goal-bundle traits — a trait
  installs a desire) was **rejected**: pressed, it added nothing real, since a bearer behaves
  identically to a character assigned the desires directly. The shipped model instead values
  **conduct**: a `Trait` is a named bundle of desires over the bearer's *own* conduct-marks
  (`Trait "honest" [Desire "clean-conscience" (Want [Match "Owner.lied.…"] (-6))]`) —
  `personaVocabulary`/`bearing`/`cast` wire a roster's traits into desires and facts, and
  `transparent` derives that everyone presumes a bearer's valuations (defeasibly, from t=0). A
  trait costs contrary conduct rather than forbidding it — the soft planner's usual idiom, no
  new enforcement. Conduct needed something to value, so `Prax.Deceit.lie` gained one outcome:
  `Insert "Actor.lied.Hearer.<pat>"`, the liar's own memory of the deed, rooted under their name
  like all mental state — a **mark on the liar**, never a world-rooted ground-truth record (the
  banked exculpation idea, rejected below). `Prax.Worlds.Village` gains **gale**, eve's
  temperament contrast: both carry the identical named `spites-carol` desire, but gale bears
  `honest`, so her conscience (−6/lie) outprices what any single whisper buys (+4/head) — eve
  whispers, gale never does, and a predictor told of both spites predicts the difference
  (`predictMove`). **The round's signature finding, surfaced in implementation, not predicted by
  the spec's first draft**: eve's whisper deceives gale too, and an honest believer turns out to
  be the perfect vector — gale spreads the falsehood she now genuinely holds by ordinary
  `gossip`, no lie, no mark, no conscience cost, and even carries it back to eve, handing the
  liar "evidence" for her own fabrication. *The honest villager launders the lie.* The spec was
  amended in place (§4 "The laundering") once this was observed live, and `VillageSpec` pins the
  corrected claim. Suite: 292 tests (`PersonaSpec`, `DeceitSpec` additions, `VillageSpec`
  additions).
- **v26** — **planner work elimination** (spec `docs/specs/2026-07-11-v26-planner-work.md`).
  A performance round with an exactness contract: bit-for-bit identical planner decisions,
  pinned by golden decision-sequence tests (`Prax.GoldenDriveSpec`, captured live pre-change,
  held byte-identical throughout). Four changes: the closed view became a cached per-state
  field behind `Prax.Engine.withDb`/`setAxioms`/`setDesires` (one closure per state instead of
  ~15k re-computations per village round); `Prax.Relevance` skips predictions no authored
  action could motivate (conservative outcome↔want pattern analysis with polarity, resting on
  one stated invariant — entity names never collide with predicate-name literals; a
  vocabulary-only "could it EVER matter" check — v33 later adds the missing state dimension,
  "could it matter NOW," alongside it — see the v33 legend row); pattern
  parsing hoisted out of the binding loops with a token-level closure loop (tokenization was
  ~48% of runtime); and the village tests share their two deterministic drive trajectories
  instead of re-simulating overlapping prefixes. Measured, uncontended: the full suite ~726s
  (292 tests) → **~114s (301 tests)**, the `Prax.VillageSpec` group ~580–660s → **~116s**, one
  profiled free-play round 7.07s → 2.83s. The residual planner cost is one axiom-closure per
  *distinct* state the search visits (71.8% of the post-round profile) — v27's target (below).
- **v27** — **incremental view maintenance** (#17; spec
  `docs/specs/2026-07-11-v27-incremental-view.md`). The exactness contract carried over, with a
  stronger net the user designated the round's core: `Prax.ViewInvariantSpec` asserts, after
  EVERY turn of real drives, that the cached `readView` equals a from-scratch closure
  (label-faithfully), with a doctored-view test proving the checker can fail. Three tiers now
  build the view: deltas the axioms cannot see (`relevantDelta` vs the pre-tokenized
  `axiomFootprint`, eviction shadows included) apply to base and view in lockstep with no
  derivation at all; `!`-free inserts that defeat nothing (`monotoneInsert`: no negated-body
  unification, `monotoneAxioms`-safe world) grow the ALREADY-CLOSED view via `closureFrom` —
  `closure`'s own semi-naive loop re-entered at the old fixpoint; everything else takes the
  full reclose. Review closed a real classifier hole before it could ever fire: `Eq`/`Neq`
  over an aggregate-bound variable is anti-monotone (exactly-k un-fires as the count grows) —
  proven by probe, rejected by `aggVars` tracking. Measured: from-scratch closures 11,840 →
  ~330 per profiled round; the round 2.83s → **1.32s** (7.07s at v26's start); full suite
  ~50–60s (machine-variance range across the round's recorded runs). DRed-style truth maintenance recorded as **not warranted**: from-scratch closure is
  ~5% of the round, and the continuation IS the delta derivation DRed would compute.
- **v28** — **the world compiles once** (spec `docs/specs/2026-07-12-v28-cooked-world.md`).
  Authored conditions and outcomes cook to token form once per world (`CookedCondition` and
  `queryCooked` in `Prax.Query` — a case-for-case transcription of the string evaluator,
  pinned by an equivalence property over every constructor; `CookedOutcome`/`CookedPractice`
  conversions in `Prax.Cooked`; the containers in `Prax.Types`, maintained like every derived
  field by the Engine helpers, with `setCharacters` joining the helper family and the
  grep-gate). The hot paths run on names end to end: `possibleActions`/`performCooked`
  (the string `performOutcome` delegates — one engine, two doors), the planner's scoring
  (`evaluateCooked` over cooked wants), and the closure loop (`runCooked` over pre-cooked
  rules) — while the STRING `closure` path is deliberately retained as the independent
  reference `Prax.ViewInvariantSpec` recomputes against every turn. Consolidations en route:
  one home for eviction shadows, the applyGrow string bridge killed, `cpFns` first-wins
  fixing a latent duplicate-function-name lookup asymmetry. Measured: profiled round 1.32s →
  **0.69s** (allocation 2.2GB → 0.75GB); full suite ~55s → **~22–30s** (320 tests). The
  post-round profile's top centres are now segment *comparisons* (`mayUnifyNames` ~25%,
  `unifyNames` ~25%) — the interning criterion is met, designating v29.
- **v29** — **segment interning** (spec `docs/specs/2026-07-12-v29-interning.md`), and an
  honest wash. `Prax.Sym` (FastString-style global pool; variable-ness packed into id parity;
  three pool doors each carrying a load-bearing bang after two lazy-argument races were found
  by implementation and the third by review); the `Db` trie re-keyed on `IntMap` with every
  String signature unchanged and tie-break name-ordering explicitly restored where the old
  `Map String` gave it for free; `Val`/`Bindings`/the cooked pipeline symbolic end to end.
  Exactness held (goldens byte-identical, ViewInvariant green throughout; DbSpec/ELSpec/
  QuerySpec passed untouched). **Measured result: ~10% at best, within machine noise**
  (unprofiled A/B round 0.19–0.33s vs v28's 0.22–0.36s; suite flat at ~24–26s, 329 tests).
  The v28 profile's "segment comparisons ~50%" attribution was misleading: the cost was list
  traversal and allocation around the comparisons — short segments fail at the first
  character, nearly as cheap as an Int test. Recorded as the round's lesson: cost-centre
  shares attribute time to a function, not to the instruction the optimization targets.
  Kept (correct, reviewed, marginally positive, and the consistent endpoint of v28's
  strings-stop-being-computational design); the next real levers are architectural — delta
  scoring/undo-log search, or the eventual embedding port — not representation.
- **v30** — **leverage: blackmail & debt, priced** (`Prax.Debt`, `Prax.Blackmail`; spec
  `docs/specs/2026-07-12-v30-blackmail-debt.md`, three in-round amendments). The backlog's oldest
  named commitment (parked since v22 for its own design round), folded with debt per user direction,
  probe-verified live before the spec was written (session probe, depth 2). **The leverage model**:
  a threat is a motive-belief deposit — `threaten` inserts `victim.believes.desires.<extorter>
  .<punitive-desire>.heard.<extorter>`, the same channel confiding/lying already ride (v20/v22), so
  the victim's own round-walk predicts and prices the exposure with no new epistemics.
  **Credibility is self-motivation** (probe finding): the extortionist's punitive desire
  (`punishes-<id>`) is what motivates *threatening* in the first place — exposing pays off from it
  one lookahead ply away — so a myopically-unmotivated planner move correctly won't foresee
  compliance; character coherence, not accident, makes the threat believable (a pure bluffer is
  expressible but not self-motivating — banked with the script layer). **A standing threat is
  exposable too** (probe finding: gating exposure on defiance alone makes stalling free forever —
  the classic hole); with exposure available against silence, waiting ties defiance and never
  dominates. **The compliance arithmetic, pinned both sides**: `BlackmailSpec` ports the session
  probe directly — two onlookers, comply scores −63.84 against wait −71.84 and defy −75.80 (buy
  wins); one onlooker, defy and wait tie exactly at −54.2 (buy still −63.84, now dominated) —
  audience size alone flips the decision, authored not tuned. `Prax.Debt` gives blackmail something
  to extract: a debt *is* an obligation with a beneficiary (`owe`/`settle`, thin over
  `Prax.Deontic`); default is belief-gated deadbeat standing (`standingUnless` on a *witnessed*
  breach, `Prax.Witness.observable` wrapping `Deontic.breach`) — an unwitnessed default derives no
  third-party regard, but review found the debtor himself is unavoidably co-present at his own
  default, so he always regards himself a deadbeat even when no one else does, a
  self-regard/third-party-spread distinction the shipped test now asserts explicitly rather than
  leaving implied. **Banked, found in implementation**: porting the probe surfaced a real bug — an
  unguarded `comply` let a renewed threat re-extract silence indefinitely, the planner's own
  recursive lookahead discovering repeat extraction before any guard existed and inflating the
  two-onlooker buy score to −51.24 against the guarded, canonical −63.84; the fix (a re-buy guard,
  mirroring the probe exactly) closed it in `shakedown`, and the discovery itself banks **repeat /
  serial extortion** as a real future mechanic (escalating price, multiple blackmailers), not
  attempted this round.

  **The village demo blocked twice, then resolved.** Both drafted arcs (carol/eve with per-head
  fear, and a dana/bob theft-evidence fallback) failed on measured traces, not taste: per-head fear
  can't simultaneously permit witnessed whispering (needs ≤1/head) and compel compliance (needs
  ~10/head) — one weight, two irreconcilable jobs; and theft-evidence shakedowns catch the framed
  exactly as readily as the guilty (v22's indistinguishability is the point, not a bug), displacing
  dana's already-shipped bread arc. dana/bob is retired as an arc outright: in this village, bob's
  crimes are either fully witnessed or perfectly secret, a Catch-22 recorded as a faithful result of
  the world as authored, not a gap to fill. The resolution is **threshold fear**, bob's own idiom
  generalized: nonlinear fear serves both masters because its marginal price is zero below the
  brink and catastrophic at it. eve gains `Want [Match "notorious.eve.slanderer"] (−15)` (mirroring
  bob's `notorious.bob.thief` exactly) wired by `standingUnless … "slanderer"` +
  `notoriety "slanderer" 3`; the whispering ACT itself becomes observable (`witnessed together
  "whispered.Actor.Hearer"` — content stays secret, only the act is caught). Blackmail victims now
  live **one witness from the brink**: a single whisper, witnessed by two co-present villagers at
  once (the addressee plus any bystander), lands two of the three regards notoriety needs in one
  action — carol, who happens to hold direct `.seen` evidence of that same whisper, shakes eve down
  (`shakedown` evidence `"whispered.V.H"`, price `favor`), and eve — one exposure from notoriety —
  pays rather than risk it. Two real bugs the blocked attempt surfaced shipped alongside the
  resolution: `villageP`'s role `V` (colliding with the shakedown's own evidence-variable
  convention) renamed to `Scene` at its source, and `shakedown`'s reserved-variable guard extended
  to `Hearer`/`Actor`.

  **The sanctioned retelling of v25's laundering.** Threshold fear has a structural second
  consequence beyond the demo itself: once eve holds two regarders (round 1's own whisper — two
  co-present villagers witness one act), any further whisper to a third party is an instant
  notoriety trip with no atonement path authored, so she becomes a **one-shot liar** — the pre-v30
  world had her whisper three times over the same 49-turn free-play trace (dana, "you", gale);
  post-v30, exactly once, ever, confirmed directly (`["eve.lied.dana.stole.carol.loaf"]`, the
  crispest fact — no `notorious.eve.slanderer` derives, and carol's own frame-up never gets past its
  first believer either). This structurally breaks v25's own unmodified "the honest villager
  launders the lie" test, whose free-play assertion needed eve to eventually reach gale directly —
  she now structurally never does. Rather than weaken the test, it was **retold**, per the v22
  retelling precedent (a documented amendment when new vocabulary genuinely changes what free play
  can show, never a silent edit): the free-play assertions that still hold (the frame-up, eve's
  mark, gale never lying) are kept and sharpened with "exactly one whisper, ever"; the laundering
  mechanism itself — v25's real finding, that an honest believer is the perfect vector for a lie —
  moves to a forced continuation (the affordance is still legally available, one-shot-per-hearer
  permits it; force it, then drive, and watch gale relay it exactly as she always did) so the
  mechanism stays pinned rather than silently going untested. `docs/WALKTHROUGH.md` §25/§27/§28
  retell this honestly, including the golden re-capture's one drift line (round 3's whisper to
  "you" becomes "Go to mill" — the threshold, not a per-head cost, is what changed it).

  **v25's parked "getting-caught-lying" item (spec §6), partially landed.** Not lie-detection or
  content-exposure — the whisper ACT alone becomes observable (co-present villagers come to believe
  *that* a whisper happened, `witnessed together "whispered.Actor.Hearer"`), while *what was said*
  stays exactly as secret as before. This gives blackmail its leverage (carol needs only the act,
  never the content) but is not the parked item in full — no one can yet catch what a lie actually
  claimed, only that a whispering happened. Noted precisely so it isn't mistaken for the fuller
  mechanic.

  Suite: 339 (`Prax.Debt`) → 354 (`Prax.Blackmail`, incl. a reserved-variable guard fix found by
  review) → 360 (`Prax.Worlds.Village`'s shakedown arc + the one sanctioned retelling), all green
  throughout; zero warnings; hlint clean; `prax check` on all 7 worlds; the village golden
  re-captured once, in its own commit, with exactly one drift line itemized (a world change, never
  an engine change).
- **v31** — **one spine, two generators: factions & kinship** (`Prax.Faction`, `Prax.Kin`; spec
  `docs/specs/2026-07-12-v31-faction-kin.md`). Two backlog rows folded per user direction because
  they share one primitive: **membership**. It's a base, single-slot fact,
  `member.<who>!<faction>` — joining, defecting, and marrying-in are all the same `!` exclusion
  overwrite, not three mechanisms. `comrades` derives `allied.X.Y` from shared membership and
  **keeps the name `allied`**, so everything downstream (the mutuality axiom, "enemy of my ally",
  `societyP`'s shun affordance) consumes it unmodified — the base `allied.*` vocabulary itself stays
  legal (`bigFeud`'s pairwise chain is unchanged, a benchmark's own design, not every alliance is a
  membership). **The generalization's proof**: `Prax.Worlds.Feud`'s two hand-authored
  `allied.bob.carol`/`allied.carol.dave` setup facts are deleted, replaced by three house-`joins`
  facts, and `FeudSpec` — every one of its 5 original tests — passes byte-unmodified; no assertion
  in it even mentions `allied.*`, so the refactor is invisible to the contract it must preserve.
  `factionStanding` extends v21's `standingUnless` shape with a membership join (belief-gated,
  spec-tested) but ships unwired into any world this round — review caught the v30-class bug again
  (a `W`/`F` reserved-variable collision that silently no-ops the axiom), fixed with the same
  `reservedClash`-style loud guard `Prax.Blackmail.shakedown` already established, plus two
  deliberate pins: fratricide (an offender's own faction-mates still condemn them) and victim
  self-belief, both asserted directly rather than left implicit. `Prax.Kin`'s `kinAxioms` are pure
  derivation (marriage symmetry, sibling, grandparent, two in-law rules) — the in-law rules are
  **stated one-directional** (acquired-relative-first, ego-second; no symmetric back-edge), and
  dissolution is retraction-safe with a designed asymmetry: retracting `married` un-derives every
  in-law it supported, but membership does **not** un-derive, because `wed`'s transfer is a base `!`
  move, not a derivation — whoever moved households stays moved through a divorce, like a real
  defection. `wed joiner faction spouse` compiles a wedding to exactly two things (the marriage
  fact, one membership overwrite), with both `joiner` and `spouse` name-guarded (review found the
  first draft guarded only `joiner`, leaving `spouse` spliceable into the fact unguarded).
  Succession reuses the same exclusion idiom for offices: any child of a dead holder may claim
  `office.<name>!<holder>`, the single slot resolves the race to one, honestly — no invented
  age/primogeniture (age isn't modeled; "eldest" would be an unprincipled fact). **The wedding
  beat, live**: esme starts inert in her own single-member house (`wren` — `comrades` needs two
  members to derive anything, so this is structural, not scripted); `wed "esme" "kestrel" "dave"`
  (the bride moves — an authored per-wedding choice, not module policy) flips her derived world in
  one pass — she becomes a comrade of the whole `kestrel` house and inherits `resents.esme.alice`,
  the grudge she had no part in creating — and the planner picks it up unprompted, on the first
  try: esme shuns alice within her first few ticks alongside bob/carol/dave, no BLOCK, no tuning.
  The village is untouched this round (goldens byte-identical; nothing in `Prax.Worlds.Village`
  imports either new module) — `factionStanding`'s village wiring is a stated, deferred decision,
  `FactionSpec`-pinned rather than built speculatively. Banked: multi-affiliation, holdings
  inheritance beyond bare offices, births (`parent.*` must be asserted, never generated), divorce
  as a driven action (dissolution is tested via raw retraction only), and the village faction
  wiring just noted. Suite: 371 (`Prax.Faction`, incl. the reserved-variable fix) → 389
  (`Prax.Kin`, incl. the `wed`-guard fix) → 392 (`Prax.Worlds.Feud` refactor + the wedding beat),
  all green throughout; zero warnings; hlint clean; `prax check` on all 7 worlds; grep-gates
  empty.
- **v32** — **confession & absolution: the road back is real, and it narrows**
  (`Prax.Confession`; spec `docs/specs/2026-07-12-v32-confession.md`, two in-round amendments).
  A deliberately small round closing two dangling hooks at once: v25's parked "getting-caught"
  item (banked in v25 as "truth recovery, if it is ever built, is committed to flow through
  mark-bearers — confession, testimony — never consultation") and v30's inert `recanted.<who>`
  defeater name (`standingUnless "whispered.V.H" "recanted.V" "slanderer"` had no action that
  ever inserted it — "no atonement path authored" — until now). **Marks convert, never delete**:
  `confess` turns `<who>.lied.<hearer>.<event>` into `<who>.confessed.<hearer>.<event>` — the
  memory persists, only its valence changes, so a trait can price the confessed form at 0, a mild
  residue, or the full price. **Confession is self-incriminating by design**: it deposits the
  deed into the hearer's beliefs through the ordinary sourced-hearsay channel (v20's `.heard`),
  so the whole rumor/reputation stack cascades on a confession exactly as on gossip. **Absolution
  is a separate, refusable, second-party act**: confessing clears your conscience; only an
  absolver's grant (inserting the world's defeater) clears your standing — you can confess and be
  refused. **Fed-up-ness is knowledge, not bookkeeping**: `incorrigible` points `Prax.Repute.
  notoriety`'s own Count idiom inward — an absolver's patience is spent once she *believes* ≥k
  distinct past instances of the deed, by any provenance, so permanence (beliefs never die),
  per-absolver independence, and confession-shopping-until-word-spreads all fall out rather than
  being bolted on. **Re-offense deletes the defeater** (v21's re-steal idiom again): a fresh lie
  snaps standing back from memory nobody lost, before a now-less-patient audience.

  **The deposit-pattern amendment, forced by an empirical block.** `confess`'s first cut took one
  pattern, matched against the mark *and* deposited to the hearer. Task 1 shipped it clean (28
  tests), but Task 2's village wiring hit exactly the shape mismatch the spec had flagged as a
  risk: eve's conscience-mark is content-shaped (`stole.C.loaf`, naming the person she framed)
  while her slanderer standing is act-shaped (`whispered.V.H`, naming her). One pattern cannot
  serve both what the mark *is* and what confessing it *reveals*; built and probed both naive
  wirings live against the engine, watched `absolve` never get offered to anyone under either,
  and reported the BLOCK rather than improvising. The spec was amended (`confess` gains a second,
  `depositPat` argument, groundable only from `Actor`/`H`/the mark's own variables, checked and
  loudly erroring otherwise — worlds whose deed is self-shaped pass the same pattern twice,
  explicitly, no default) and the module fixed to match, both before Task 2 resumed.

  **Task 1's own incorrigible bug, found before a single test was written.** The plan's own
  worked code for `incorrigible` reused the deed pattern's non-offender variables verbatim in
  both the outer existence `Match` and the counting `Subquery` — but `Prax.Repute.notoriety`'s
  own shape (the pattern this was meant to mirror) uses *different* names for the counted role in
  each. A literal transcription binds every deed variable in the outer `Match` before the
  `Subquery` ever runs, so the count is always 1 and a `k > 1` threshold can never fire, in any
  multi-variable deed pattern — confirmed directly against the engine (a believer of two distinct
  instances, `k=2`, evaluated `False`) before any assertion was pinned. Fixed by generalizing
  `notoriety`'s own `W`/`W0` convention to every deed variable (dummy `<name>0` witnesses in the
  outer `Match`, the true names free for the `Subquery` alone to count) — `k=2`/`k=3` then
  evaluated correctly against the same fixture.

  **The probed arithmetic, measured live before being pinned** (`scoreActions`/`pickAction`
  against the real module, v30's own discipline). Spontaneous confession: a mild secret (stake 4)
  scores confess = 0.0 against holding your tongue at −2.0 — confesses; an expensive one (stake
  20) scores hold-your-tongue = 37.94 against confess = 0.0 — doesn't. Confession as blackmail
  defense (a confessed secret is spent leverage: the extorter's `expose` deposits nothing new
  once the hearer already has it self-sourced): at a steep price against a mild secret (price 30,
  fear 3), confessing to the sole co-present hearer beats complying and defying on the merits, not
  a tie — and `expose` is then fully dead (no other hearer left to expose to); at a cheap price
  against a severe secret (price 1, fear 30), complying still wins. Both sides matched the spec's
  stated expectation on first measurement, no BLOCK — but the first pin (steep-price case) rested
  on a three-way label tie between confess/defy/wait because the fixture's victim carried no
  conscience cost on the underlying mark; review caught it, a matching conscience desire was added
  to that fixture alone, and the case was re-measured and re-pinned on an honest, strict margin.

  **The village demo: two arcs probed, one structurally capped, one shipped.** Primary arc — carol
  (the wronged party, given an arbitrarily generous professed `merciful` desire) absolving eve
  after confession — was built and measured across `mercifulValue ∈ {0,5,…,50}` and never beats
  eve's ordinary baseline at *any* value: confessing to a genuinely new regarder (carol wasn't yet
  one) eats the full, immediate notoriety-threshold hit, while the planner's own `othersScore` term
  applies only a fixed 0.5 discount to a *predicted* absolution's value — a hard ceiling no
  authored desire magnitude can clear. Documented, not shipped — the "threshold drama" the spec
  asked to measure, measured and found insufficient. Fallback arc — eve confesses to gale, who
  already regards her a slanderer from directly witnessing the whisper — costs nothing ("free
  below the brink," v30's own idiom: confessing scores exactly tied with eve's routine baseline,
  no notoriety spike) and cleanly unlocks gale's `absolve`. Shipped, forced per the wedding/theft
  precedent.

  **A real regression, root-caused, not patched around: gale's cheap-grace loophole.** Making
  `confess` generally available (not just to eve) gave gale's `honest` trait a hole its own design
  forbids: her only desire priced the *lied* form of her mark at −6 and said nothing about the
  *confessed* form (defaulting to 0 relief once it converts), so her depth-2 lookahead saw
  "lie, then immediately confess" as a way to buy the +4/head spite payoff for the price of a
  self-erasable −6 — defeating the v25 "her conscience outprices the spite" invariant outright
  (traced turn-by-turn: her first free-play whisper decision flipped from losing, 5.42 vs. a 10.84
  baseline, to winning, 15.68 vs. 12.28, with the wiring present). The spec was amended to allow —
  and, for a bearer whose entire narrative purpose is unconditional honesty, require — pricing the
  confessed form at the *full* price too: a second `honest` desire prices `confessed` identically
  to `lied`. Re-verified byte-for-byte against the pre-confession-wiring trace.

  **A mechanical, honestly-recorded side effect: the v26 pre-filter pays.** `confessWhisper`'s own
  outcome list `Delete`s a `lied`-mark shape — the first authored village action ever to do so —
  which mechanically flips `Prax.Relevance.improvableDesires`'s analysis of `clean-conscience`
  from un-improvable to improvable (its premise, "no action deletes a lied-mark," is no longer
  true). `Prax.RelevanceSpec`'s table assertion flips accordingly, with the mechanism stated in
  its own comment. Consequence: `predictMove`'s v26 skip — free for any conscience-only believed
  model of a `transparent`-presumed bearer since t=0 — no longer applies to gale, so every in-scope
  predictor now scores her candidates on every relevant turn instead of skipping the round
  entirely. **Measured, isolated, three ways** (`cabal run -v0 prax-test -- -p "Village"`,
  best-of-3, zero concurrent builds verified before each side, a dedicated `git worktree` for the
  pre-arc side so it never shares a build with HEAD): the full 36-test Village group at HEAD
  (post-round, pre-filter spent) runs **219.38s** (227.17s/246.78s/219.38s); the SAME 31
  pre-existing tests alone (excluding the 5 new v32 additions, via an explicit `-p` exclusion
  filter) run **171.64s** at HEAD (190.64s/171.64s/180.98s); the identical 31-test group at
  `fd436de` (the commit immediately before the village wiring landed, pre-filter still intact)
  runs **31.11s** (31.11s/32.23s/31.35s). The gap between the pre-arc and HEAD-filtered numbers —
  **≈140.5s, a ≈5.5× slowdown on 31 tests whose own code never changed**, cleanly isolated from
  the 5 new tests' own ≈47.7s cost (219.38s − 171.64s) — was recorded here as "the pre-filter loss
  alone." **Amended in place by v33, on remeasurement, not confession: that attribution was
  wrong.** v33 built exactly the state-aware pre-filter this entry called for and re-ran the
  identical 31-test A/B: reclaiming the skip recovers ≈39s (171.64s → **132.75s**), not ≈140.5s —
  the pre-filter's real cost is the smaller number, not the whole gap. A controller profile at
  HEAD (3.0s vs. 0.69s at v28-on-the-pre-v30 world) attributes the residual ≈100s to
  **world-richness** in the village's grown axiom set, not to the pre-filter: three Count-bearing
  aggregate axioms now run in every closure continuation (`notoriety "thief" 3`,
  `notoriety "slanderer" 3`, `incorrigible "whispered.V.H" 2 …` — `deltaJoinCooked` ~17%, `num`
  ~3.7% of the profile) alongside larger per-primitive classification footprints (`mayUnifySyms`
  ~11%) — cost the 31.11s-era world never carried and that no relevance filter removes without
  shrinking the world itself. (The v33 implementer's own first guess, that this residual was a
  confound from `Prax.Repute` merging in just before this round, was checked against `git log` and
  rejected: `Prax.Repute` dates to v21, well before this round's own `fd436de` baseline, so it was
  already present on both sides of the gap and explains none of it.) The 31.11s epoch belongs to a
  poorer world and was never reachable by fixing the filter alone. Stated plainly, per the round's
  own instruction: the 5.5× multiple was real and worth investigating — it correctly pointed at
  the v33 decision below — but this entry's original explanation of *why* the gap was that large
  was itself wrong; see the v33 legend row for the corrected account and the measured recovery.

  **Task 2b's ghost investigation: a premise disproved, a fix reverted, the real fix banked.**
  Probing `Prax.Db.retract`'s known ghost-ancestor imprecision (a drained ancestor path reads as a
  phantom fact) under the controller's premise that interior nodes aren't independently
  representable facts, pruning childless ancestors on retraction was implemented, RED/GREEN-pinned,
  and then broke a real, unmodified test: `Prax.Worlds.Bar`'s `tendBarP` practice asserts a
  bartender's instance fact at a path (`practice.tendBar.<Place>.<Bartender>`) that *also* anchors
  transient per-customer state nested beneath it — draining that transient state to zero (a normal
  order→fulfill→drink cycle) pruned the instance fact itself, permanently destroying the
  bartender's affordance for the rest of the run. The trie cannot distinguish "an asserted fact
  that happens to be childless" from "an ordinary ancestor, now childless because its only occupant
  was retracted" — both are represented identically. Pruning is correct for the first half of that
  ambiguity and actively wrong for the second; the premise that motivated the attempt was
  disproved by evidence, not just complicated by it. **Reverted** (`retractNames` restored
  byte-identical to its pre-task form); the DbSpec ghost-pruning test was **replaced**, not
  deleted, with an INSTANCE PERSISTENCE test pinning the opposite (a drained instance path must
  still `exist`) as the regression net against ever reintroducing the pruning by accident. The
  principled fix — marking a node as an asserted endpoint independently of child-emptiness — is a
  distinct `Db`-type change, banked as **asserted-endpoint marking** (see "Future ideas to
  investigate," below); `retract`'s and `dbToSentences`'s haddocks now name it directly so the
  ambiguity is found already-planned, not rediscovered.

  **Banked, per the user: recidivism into character.** Becoming a liar *by* lying (fed-up-ness
  shaping the offender's own future disposition, not just an observer's regard) needs bearer-side
  desires to be fact-driven — but `charDesires :: [String]` (`Prax.Types`) is a static field fixed
  at character construction, not something an in-world action can insert or delete the way a base
  fact can. Closing this needs a `Prax.Minds`-level engine change (desires gated on believed-own
  facts, not just assigned at cast time), belonging with a future Arc-vocabulary round, not this
  one. Stated with the obstacle, not silently dropped.

  Suite: 421 (`Prax.Confession`, incl. the `incorrigible` fix and the RED-checked blackmail-margin
  fix) → 424 (the deposit-pattern amendment's own 3 new tests) → 429 (`Prax.Worlds.Village`'s
  redemption arc, incl. the `honest`-trait fix and the sanctioned `RelevanceSpec` flip) → 431
  (`Prax.Db`'s reverted-and-repinned ghost investigation), all green throughout; zero warnings;
  hlint clean; `prax check` on all 7 worlds; grep-gates empty; `Prax.GoldenDriveSpec`/
  `Prax.ViewInvariantSpec` byte-identical throughout — no golden re-capture needed anywhere this
  round (the new affordances are forced-trajectory only; eve's free play is unperturbed).
- **v33** — **state-conditioned relevance** (spec
  `docs/specs/2026-07-13-v33-live-relevance.md`). v26's relevance pre-filter reasons from
  vocabulary alone — "could ANY action EVER improve this want-kind?" — a question v32's
  `confess` made permanently true for every conscience desire, spending the skip world-wide
  even though it stays sound in almost every actual state (the v32 entry above is amended in
  place with the corrected accounting). This round adds the missing dimension: **could it
  matter NOW?** Two liveness recipes, classified once per world by
  `Prax.Relevance.livenessOf` and consulted by `predictMove`'s pair-skip
  (`Prax.Planner.deadNow`) alongside the existing static check: **FloorCheck** (negative
  want-kinds) — a rule improves only by LOWERING its satisfaction count, so a count of zero
  right now is unconditionally the floor, sound regardless of conjunct structure, decided by
  one `countSatisfying` against the desire's own Owner-grounded conditions; **GateCheck**
  (positive want-kinds) — a top-level conjunct that (a) no authored action outcome
  may-unify-inserts, AND (b) no axiom head derives (the existing `derivable` conservatism),
  AND (c) currently has zero bindings, makes the WHOLE conjunction's rise impossible this
  turn. Everything else (`Subquery`/`Count`/`Calc`-tainted wants, an unresolvable `wild`
  pool, weight 0) stays **AlwaysLive**, conservative by construction. Both checks are
  **pair-level only**: if any believed desire is live, the FULL model scores, dead
  deterrents included — a mixed live+dead believed model must still have its dead deterrent
  deter, pinned by a dedicated test and RED-verified against a model-content-filtering
  mutation (a wrong implementation that dropped dead-now desires from the SCORED model,
  rather than only from the skip decision, flips a real tie between two otherwise-equal
  actions). Task 1 caught its own plan's error before a test was built against it: the plan
  named the village's third positive desire `fears-scandal`; grepping `src/` found no such
  desire anywhere in `Prax.Worlds.Village` — the actual one is `punishes-whisper` — and the
  plan doc was corrected rather than a test written against a name that isn't there.

  **The measured recovery, and the correction it forces.** The same 31 pre-existing,
  uncontended `Prax.Worlds.Village` tests v32 A/B'd: **132.75s** (best of 3:
  140.02s/132.75s/135.00s), down from v32's **171.64s** — a real ≈39s reclaim (≈23% of the regressed 171.64s runtime; ≈28% of the
  ≈140.5s v32 regression, not a full return to the pre-v32 **31.11s**. This ≈39s IS the
  pre-filter's actual cost, which is why the v32 entry's "the gap... is the pre-filter loss
  alone" is amended above rather than left standing beside a truer number: a controller
  profile at HEAD (3.0s vs. 0.69s at v28-on-the-pre-v30 world) assigns the residual ≈100s to
  **world-richness** — three Count-bearing aggregate axioms now running in every closure
  continuation (`deltaJoinCooked` ~17%, `num` ~3.7% of the profile) plus larger per-primitive
  classification footprints (`mayUnifySyms` ~11%) — cost the 31.11s-era world never carried
  and that no relevance filter, however state-aware, removes without shrinking the world
  itself. The 31.11s epoch belongs to a poorer world and was never reachable by fixing the
  filter alone. Suite: 437 → **441** (`RelevanceSpec`/`PlannerSpec` additions, incl. the
  mixed-model pin), ~236s → **~188s**, all green throughout; zero warnings; hlint clean;
  `prax check` on all 7 worlds; grep-gates empty; goldens byte-identical; ViewInvariant green
  throughout. **Banked, not built** (targeting the world-richness residual this round's
  profiling isolated, not the pre-filter question this round closed): footprint
  discrimination indexing and axiom-family partitioning for the continuation loop (see
  "Future ideas to investigate," below). **v34 built exactly the mechanism this residual
  pointed at and measured it directly**: reuse reclaims ≈9.5% of the post-v33 runtime
  (120.10s vs. 132.75s) — not the rest of the residual. The remainder of the recursion cost
  turns out to be semantically necessary under the exactness contract: the village's own
  reputation-cascade writes (the whisper's `Delete recanted.Actor`, the `believes`→`regards`
  cone) genuinely reach every mover's reads, so those predictions must recompute, not merely
  be re-derived waste — see the v34 legend row below for the measured account.
- **v34** — **prediction reuse — and the honest limit of it** (spec
  `docs/specs/2026-07-13-v34-prediction-reuse.md`). User-directed from measured branch
  statistics: over 70 village free-play turns (60 NPC picks at depth 2), the same picks cost
  **89ms at depth 0, 2.3s at depth 1, 44.5s at depth 2** — each lookahead level multiplying
  by ~20–26×, pure recursive branching, not state machinery. One template is most of the
  tree: the gossip whisper grounds 458 of 674 top-level candidates (68%), each taken at most
  once per pick. And within a pick, sibling post-states' predictions equalled the parent
  state's prediction in **4,014 of 4,014** observed (candidate, mover) comparisons —
  `scoreActions` was re-running `predictMove` at every tree node even where a node's state
  differed from the pick's root by only a few outcome tokens that provably couldn't change
  most movers' predictions. This round makes that proof and reuses the root's prediction
  wherever it holds: three static per-world enumerations (what a `predictMove` pair reads;
  what an action's grounded outcomes touch; which axiom heads a fact family can fire), a
  root-memo prediction per pick filled lazily per mover, and a per-node path-delta anchor set
  expanded through a derived-fact cone (an axiom's head joins the cone whenever any body atom
  may-unifies something already in it) — a node reuses the root's prediction exactly when the
  cone misses the mover's read set, and goes opaque (no reuse anywhere below it) on anything
  unboundable. **Task 2b tightened the opacity rule itself**: a broadcast `ForEach` insert
  (the whisper's own shape, previously forced opaque unconditionally) is bounded instead when
  its variable head is a *safe binder* — never occurring at the first position of any guard
  `Match`, so it can never unify the registry literal `practice` — while an evidence-free
  (all-variable) path stays opaque by construction regardless of binder safety, closing an
  in-principle soundness hole the safe-binder rule would otherwise have opened. Exactness held
  throughout: goldens byte-identical, ViewInvariant green, decisions bit-for-bit; both reuse
  guards are mutation-verified in both directions (dropping the reuse guard entirely fails
  both payoff fixtures verbatim; dropping only the cone fails the derived-fact fixture alone,
  leaving the base-fact fixture green exactly as the discrimination predicts).

  **The measured recovery, and why the projection came up short.** The same 31-test village
  A/B (uncontended, best-of-3) against the recorded epochs — 31.11s pre-v32, 171.64s
  post-v32, 132.75s post-v33 — landed at **120.10s** post-Task-2, a real ≈9.5% reclaim, then
  **123.57s** post-2b, overlapping Task 2's own 120.10–130.85s run range: 2b is a perf
  **wash**, kept for the correctness it buys (closing the haddock's stated-vs-actual
  divergence) rather than for speed. Full suite: 449 tests @ ~165–187s (machine-noise range).
  An attribution pass over 68,286 `predictAt` calls explains the shortfall against the spec's
  projected win (whisper subtrees collapsing to the depth-0 floor): before 2b, 98% of calls
  sat on opaque paths (every broadcast `ForEach` insert tripped the spawn guard); 2b brought
  opacity down to 25%, but the freed 73% moved into cone∩read-set **INTERSECTION** (74%), not
  reuse (still 1%) — the whisper's own writes (`Delete recanted.Actor`, forfeiting amends; the
  `believes`→`regards` cone) genuinely reach every mover's reputation read. The branch probe's
  4,014/4,014 equality was contingent on that one traced state, not provable: under the exact
  contract, those pairs must recompute live. The residual 25% opacity is exactly two
  literal-`practice`-rooted templates (`Go to [Place]`, `take up honest work`) — the sound
  floor of the current rule, not a missed case.

  **Banked**: below-existing-instance practice-path inserts could bound exactly rather than
  stay opaque (`spawnedInstanceNames`'s existed-before semantics would let `Go to`/`honest
  work` un-opaque); per-reachable-head cone precision (`extendDelta` currently joins every
  axiom head on any feed — a per-head reachable-from-the-delta cone would free some
  whisper-adjacent pairs, though the raw `recanted.V` dependency would still defeat the
  culprit-facing ones — see "Future ideas to investigate," below). Suite: 445 (Task 1) → 448
  (Task 2) → 449 (Task 2b + its own evidence-free-path fix), all green throughout; zero
  warnings; hlint clean; goldens byte-identical; ViewInvariant green throughout.
- **v35** — **intentions: reconsideration semantics replace always-deliberate** (`Prax.Types`
  `Intention` + an `intentions` runtime field on `PraxState`; `Prax.Planner.motiveSignature`;
  `Prax.Loop.npcAct`; spec `docs/specs/2026-07-13-v35-intentions.md`). **The round's first
  accepted semantics change since the exactness era began at v26** — user-directed: "agents
  would not plan potentially whispering secrets every few seconds — always considering every
  possibility every step is not realistic and very wasteful." Three probes grounded the
  redesign before a line of implementation: a **91.5% pick-stability ceiling** over a 70-turn
  village drive (379/414 turns unchanged; carol, one of the most expensive deliberators, never
  changed her pick once); **the anchor family is structurally exhausted** — a chained cache
  upgraded with the banked per-head-cone lever served **zero** picks, because the village's
  axiom graph deliberately chains co-presence into reputation (movement → togetherness →
  witnessing → belief → notoriety → regard), so every non-wait action's cone reaches all six
  characters and no refinement of the v26–v34 proof family can see that gale walking to the
  mill is irrelevant to carol in the square; and **motivational triggers caught all 35 real
  pick changes while licensing 290/414 (70%) skips**, at near-zero cost.

  **A mid-round reversal, stated plainly.** The first-cut signature grain (full
  grounded-candidate equality) measured **INERT** at the real own-turn interval — 0 serves,
  120/120 deliberated, 1.0× — because the per-turn probes above had measured the wrong
  interval: villagers move every round, and movement churns co-presence groundings. The probe
  ladder that found the working grain: unfiltered templates, 26% served/0 divergences;
  templates dropped entirely (the "bold agent" variant, **rejected**), 50 served but 19
  divergences — dana served "Wait" for the whole drive while a fresh pick wanted "shun carol"
  every round, proving opportunity *appearance itself* carries dramatic signal; **want-bearing
  templates**, 45 served (38%) with **zero divergences**, every one of the remaining
  deliberations (the probe's 75/45 split) defensible — arrivals expiring a movement pick (55),
  satisfaction changes (8), motive updates (2), first turns (6). The spec was amended in place
  to this grain.

  **The shipped semantics**: a character holds a standing intention plus the motive signature
  it was chosen against, and re-deliberates only when the signature changes — commitment is
  the default, reconsideration the exception, per Bratman. Four named components, none a tuned
  number: (1) what I can do that I care about — my standing action is still offered (full
  grounded equality; a stale grounding can never be acted) AND the want-bearing template set
  changed (`Relevance.bearingTemplates`, a `caresAbout` table); (2) how I'm doing — the
  per-want satisfaction count vector, kept as counts, not summed, so two profiles can't mask
  each other; (3) what's driving me — the live-desire set (v33's floor/gate machinery, pointed
  at oneself); (4) what motives I know of — the believed-motive facts I hold on others.
  **Accepted gaps, pinned as INTENDED**: a one-beat lag when only another's *predicted* reply
  to a fresh affordance would change my pick (invisible until a signature-visible consequence
  lands), and the same one-beat lag on second-order opportunities generally — both dedicated
  tests (the quiet pin, the non-bearing pin), not silently absent.

  **Measured, honestly, with the variance called out.** Goldens are **byte-identical** — the
  re-capture protocol the spec expected to need went unused, zero drift to itemize. Full
  suite: 454 tests @ **153.22s**, down from the ~175–186s baseline (deliberation itself now
  mostly skipped). The **paired drive bench is the primary performance evidence** (same
  140-turn trajectory, both loops, one process, eliminating cross-run noise): **52.3s vs.
  97.1s = 1.9× on sustained play**. The 31-test village A/B is reported but **not
  attributed**: best-of-3 98.17s against v34's 120.10s epoch looks like a win, but the *inert*
  v1 tree (0 serves, the mechanism doing nothing) measured 113.97s on pure run variance alone —
  **below** the v34 epoch with zero mechanism engaged — so the suite A/B's noise band swamps
  an effect this size; the paired bench is what the round actually stands on. Two mutations
  confirm the pins bite: dropping the bearing filter fails the non-bearing pin; dropping the
  `stillOffered` guard lets a vanished action get performed, and the standing-gone pin catches
  it. Zero warnings; hlint clean; no mutation markers remain.
- **v36** — **decay & drift: episodic state on the clock** (`Prax.Drift`; spec
  `docs/specs/2026-07-14-v36-drift.md`). User-directed, scope sharpened in review before a
  line of code, on two rejections. The original "scores cool toward baseline" framing
  (**grudge-cooling**) is REJECTED: dispositions (grudges, conduct marks, trust, standing)
  never decay — they change only through ACTS (confession, absolution, amends, the v20–v21
  machinery already built) — because a timer erasing them would undermine
  atonement-not-amnesia (discharge must cost something) and history-persists-through-marks
  (truth going unrecoverable is drama, not bookkeeping). The v35-era **recency-gradient**
  stays declined, permanently: `Prax.Sight`'s hard `sightedWithin` window is the intended
  model, not a smoothed approximation of one. What's left is genuinely episodic — appetite,
  intoxication, arousal — the scale a game actually represents (hours to weeks).

  **The mechanism** (`Prax.Drift`, the v18/`Prax.Sight` idiom pointed at state evolution: a
  compiler of authored rules into ordinary practice content, zero engine/planner/query
  surface). A bodiless per-round drifter (`_drift`, blank-label, riding after `_sight` in
  the cast) carries a `due.<name>!D` fact per rule; its one action gates each body
  `ForEach` on `turn!Now >= D` and re-arms `due.<name>!D2` at `D2 = Now + period` — **from
  NOW, not from D**, so a stalled world doesn't rapid-fire its backlog on resume. Three
  construction-time guards, the v30/v31 class: a rule name must be a single segment; a body
  may not mention the reserved `D`/`D2`/`Now` variables (collision with the due gate); a
  period must be positive. `Prax.TypeCheck` gains `ClocklessDrift` (Check 5): a world
  registering the `drift` practice without a `turn` fact is flagged loudly rather than
  silently never firing.

  **Two cargo cycles, one shape.** Village hunger (build-up): every `mealPeriod` (3)
  rounds, `appetite.<who>` bearers gain `hungry.<who>`; `suffers-hunger` (`Want
  [Match "hungry.Owner"] -22`) prices eating at exactly what it costs — a held loaf
  forfeits 10, a completed 3-stage endeavor forfeits 9 more (the `eat` action tears the
  instance down), 19 total against 22 relief, a **+3** margin the planner actually picks
  (mutation-verified: dropping relief to 12 flips the choice back to hoarding, net −7). Bar
  metabolism (wear-off): every `soberPeriod` (2) rounds, each patron's `drinks!N`
  decrements toward a `Gte 1` floor (never negative), and `checkSober` mirrors
  `checkTipsy`'s own threshold (`Cmp Lte M 1` clears `tipsy`) — the same number read both
  directions, one home for the fact.

  **The emergent fiction, unplanned.** Village free play produces a hungry bob eating the
  loaf he *stole* outright — no credit ever forfeited on it — and only later earning and
  eating a second, honest loaf before the theft's atonement beat completes. `postTheftAt`
  moved 70→96: 10 rounds under the old 7-member round is 70, 12 under the new 8-member
  round (`_drift` joins the cast) is 96, and the arc genuinely needs those two extra
  rounds now — eat-the-stolen-loaf, then earn-and-eat-a-second, not eat-then-forgive. In
  the bar, the same roster growth costs `LoopSpec`'s fixed 25-turn golden its last two
  lines: bex's arc-completing "settle in, feeling you belong here" actually lands at turn
  27 (confirmed directly, not assumed), two turns past the window's edge — bex is
  `hopeful`, not yet `belonging`, when the replay ends; the warmth held, it just needed two
  turns the extra silent tick spent elsewhere.

  **The golden protocol held.** Both worlds' goldens were re-captured live from the driven
  output, never hand-authored, each cargo task's re-capture in its own commit separate from
  the code that moved it (`cf82427`/`70afce0` village, `0d02b02`/`0957ff7` bar), itemized
  line by line in both task reports. The village's 21-line window shows only two ordinary
  `"_drift: "` no-op lines (the due hasn't come due inside that short a capture) — the
  hunger cycle itself is pinned directly, at the real turn counts, in `VillageSpec` (absent
  at `freePlayAt 23`, present at `freePlayAt 24`, re-armed three rounds later), not
  inferred from the golden slice. Intrigue and feud goldens: untouched. ViewInvariant green
  throughout; suite 465 (Task 1) → 468 (Task 2) → 472 (Task 3), all green; zero warnings;
  hlint clean throughout.

  **The paired drive bench, re-run against the drifting village** (same 140-turn
  trajectory, both loops, one process — the v35 protocol exactly), with a same-machine,
  same-session pre-drift control (commit `81380ed`, the 7-member roster) run alongside it
  to separate drift's own effect from ordinary run-to-run noise: pre-drift **50.1s / 82.1s
  / 1.6×** (20 deliberations of 140 turns: bob 7/20, dana 6/20, carol 2/20, eve 2/20, gale
  2/20, `_sight` 1/20) vs. post-drift **51.8s / 70.9s / 1.4×** (33 deliberations: bob
  **18/18** — every single one of his turns — dana 6/18, carol 2/18, eve 3/17, gale 2/17,
  `_sight` 1/17, `_drift` 1/17). The attribution the spec's acceptance demands: bob's count
  alone absorbs 11 of the 13 added deliberations — the mealtime hunger cycle is a second
  gate/satisfaction-changing rhythm layered on his existing endeavor-stage churn, pushing
  him from re-deliberating on roughly a third of his turns to literally all of them; every
  other named character is flat against the pre-drift control within one deliberation (eve
  +1, plausibly a downstream motivational ripple, not a sub-threshold read); `_drift`
  contributes only its own one-time first-turn cost, the same idiom as `_sight`. **The
  pulse wakes bob and essentially nobody else — the acceptance holds; not BLOCKED.** The
  intentions loop's wall time is nearly flat (50.1s→51.8s) because bob's added real
  deliberation work roughly cancels the discount every turn gets from an 8th, mostly-free
  cast member; the always-deliberate loop, whose cost tracks raw turns rather than
  deliberations, takes that whole discount (82.1s→70.9s) — the 1.6×→1.4× ratio drop is that
  discount, not a regression in the reconsideration mechanism. Against the
  originally-recorded v35 numbers (52.3s/97.1s/1.9×, a different session): intentions is
  within noise (52.3→51.8s); the rest of the gap is *not* attributable to v36 alone — the
  same-session pre-drift control already measured 82.1s/1.6×, so roughly half the drop from
  97.1s is ordinary cross-session variance (the v35 row's own documented pattern), the
  other half the roster-dilution effect above. Full suite: 472 @ 146.83s.

  **Correction, made and cited here per the fix-don't-confess discipline (see the v37 row
  below).** This round's own mechanism silently degraded gate precision for exactly the
  fact family it exists to move: `_drift`'s pulses joined `Prax.Relevance.worldAtomPools`'s
  scan without an exclusion, so the pool's wild/action-insertable fallback swallowed every
  clock-moved fact — `hungry.*` here, and (found by v37's attendance probe) any future
  clock-gated positive desire — reclassifying it from `GateCheck` to `AlwaysLive`.
  Conservative direction, so nothing was ever unsound and no golden moved: this is a
  precision loss (v33's liveness-skip optimization silently lost its intended target on
  this fact family), not a correctness bug. v37 repaired it (`worldAtomPools` now excludes
  the drift practice by id) and re-verified both ends: `suffers-hunger` (negative,
  `FloorCheck`) was never touched by either the bug or the fix; any positive hunger-shaped
  desire regains its gate under the repair. The measurements recorded above (bench numbers,
  deliberation counts, golden protocol) are unaffected by this — they were never about gate
  classification — and stand exactly as recorded.
- **v37** — **calendar & gatherings: the clock convenes, the town shows up** (`Prax.Drift`
  `gathering`; `Prax.Relevance` reclassification; spec
  `docs/specs/2026-07-14-v37-gatherings.md`). User-directed (the banked item: recurring
  clock-gated scene spawns — the mixing dynamic that makes gossip percolate), probed live
  before speccing, with one clean success and one real discovery. **The calendar worked
  first try**: a pair of v36-shaped pulse rules (open spawns a practice instance + event
  fact, close tears both down) ran a 26-round probe, opening and recurring exactly on
  schedule — spawning and closing a practice instance from a pulse body works today. **But
  nobody attended**, and the diagnosis is a v36 regression, not a v37 gap: fresh
  deliberation at the open market picked "Go to square" correctly, yet the characters'
  motive signatures were byte-equal before and after the market opened — v36 had made the
  ticker an ordinary practice action, polluting `worldAtomPools`, so every clock-moved fact
  looked "action-insertable" and the v33 gate classifier refused to gate on any of them.

  **The fix, and the semantics it encodes.** The user's position, adopted directly ("I
  expected tickers to be able to change motives anyway"): tickers change motives — an NPC
  must be able to make a different decision the instant something clock-moved becomes true.
  Mechanically, `worldAtomPools` now excludes the drift practice's own outcomes by id
  (`Prax.Drift` exports `driftPracticeId`; `Prax.Relevance` imports it — no cycle, `Drift`
  only imports `Types`/`Query`/`Db`). Review hunted both exactness holes and found them
  EMPTY: axiom-derivable wants are unconditionally improvable independent of the pools (the
  exclusion can't create a false negative there), and the drift practice's own function
  bodies (dynamic `Calls`) degrade to `poolWild`/`AlwaysLive` on their own account, not
  through the exclusion. The v33 environment-gate concept regains its defining example: a
  clock-moved fact family, no authored outcome inserts it and no axiom derives it — that is
  what an environment does. Consequence, pinned both ways: `drawn-to-market` (event ∧
  presence, positive) now classifies `GateCheck [event]` — dead between gatherings (zero
  planning cost town-wide), LIVE the instant the market opens, so the live-desire component
  of every attendee's v35 motive signature flips and they re-deliberate TO the gathering;
  the close flips it back and they re-deliberate AWAY. v36's hunger-shaped positive desires
  regain their gate too — the silent precision regression repaired, stated in the amended
  row above; `suffers-hunger` itself, negative, was never affected (`FloorCheck` doesn't
  consult the pools).

  **The `gathering` combinator** (`Prax.Drift`, beside `driftSetup`): two period-cadenced
  `DriftRule`s, `<name>Open`/`<name>Close`; seeds place the open due at `period`, the close
  due at `period + duration` (v36's start-sated convention — the first gathering convenes
  one full period in). Loud construction-time guards: `0 < duration < period` (no overlap,
  no null event) and the inherited single-segment name guard. Review traced two hazards to
  ground: phase-drift ruled out BY TRACE (one drifter, one action, one `turn!Now` match per
  round, a +1/round monotone clock keeps the open→close interval exactly `duration` every
  cycle, verified two full recurrences deep); the double-seeding hazard is real and
  documented in the haddock itself — feeding the gathering's own rules to `driftSetup`
  would seed both dues at `period`, opening and closing on the same pulse, a market that
  never convenes.

  **Cargo: village market day, and the cadence correction Task 4's own measurement forced.**
  `marketCalendar = gathering "market" 6 1` opens a bare `market` practice instance and a
  `marketDay.square` event fact in the square, for one round every sixth round.
  `drawn-to-market` prices attendance at +3: strictly above the +1 loitering anchors (a
  market beats an idle preference) and strictly below the +4 conduct stakes / +5 event
  wants (drama still outranks festivity), wired onto all five villagers (bob, carol, dana,
  eve, gale) — `you` excluded. The cargo first shipped at period 2/duration 1 — chosen so
  the golden's 21-turn capture window would witness a full open→close cycle — and the
  implementer's own golden re-capture and drama re-indexing (bob stays at the open fair,
  round 2: 51.49 vs. 49.02) were correct FOR THAT CADENCE, reviewer-confirmed. **Task 4's
  paired drive bench then measured its true cost at production scale**: at period 2 the
  market toggles open/closed every single round, so `drawn-to-market`'s `GateCheck` flips
  every round for every villager who holds it — there are no quiet rounds left, town-wide.
  A 140-turn paired drive tripled (68.3s → 193.2s, 33 → 90 deliberations; every one of
  bob/carol/dana/eve/gale deliberating on essentially every turn), and the reconsideration
  discount collapsed (1.2×→1.1×) for a market-attending village. The gate itself was exact
  throughout — every re-deliberation traced to a real motive-signature change, never a
  stale or sub-threshold read — so this was never a correctness bug; the cadence was the
  defect, chosen for golden-window visibility (a constraint v36's own hunger pulse had
  already shown unnecessary — its cycle is pinned at real turn counts, not required to be
  golden-visible). **Shipped, corrected: period 6, duration 1.** VillageSpec/LoopSpec pins
  re-derived by observation against the live trace, not assumed: convergence and
  percolation move to the market's actual first opening (round 6, turns 48–55); dispersal
  keeps gale's confirmed departure and the cycle's recurrence, dropping the period-2
  cadence's "a stronger stake stays" clause — dana's suspicion arc resolves at turn 28, long
  before this cadence's first market close reaches her, so it no longer functions as a
  competing stake at the relevant moment (traced, not assumed; not reproduced rather than
  weakened, per the round's own discipline). "Same spite, different temperaments" and
  "deterrence plus opportunity yields industry" both revert to their exact pre-v37 form —
  the market's later first opening no longer reaches either moment. The golden re-capture
  at the corrected cadence moves exactly one line back to its pre-v37 value (bob's round-3
  turn, "Wait a moment" → "Go to mill"): the 21-turn window no longer witnesses a market at
  all, own commit, itemized. Percolation — the mixing dynamic the item was banked for — is
  still measured, not asserted, just at the corrected turns: `quietWitnesses` 1 (dana alone)
  vs. `marketWitnesses` 4 (you/bob/carol/dana), pinned with exact counts in `VillageSpec`.

  **The paired drive bench, corrected cadence.** Same 140-turn trajectory, both loops, one
  process (this session's machine ran heavily loaded by other concurrent agents throughout;
  the deliberation counts are scheduling-independent and hold regardless, the times are the
  cleanest this session could produce). Pre-cargo control at `4b041d3` (post-v36-fix,
  pre-Task-3): **68.3s / 78.9s = 1.2×** (33 deliberations: bob 18/18, dana 6/18, carol
  2/18, eve 3/17, gale 2/17). Post-cargo at the corrected cadence: **93.0s / 113.3s = 1.2×**
  (51 deliberations: bob 18/18 unchanged — his hunger cycle, not the market, drives that —
  carol 6/18, dana 12/18, eve 7/17, gale 6/17, each a bounded 2–3× bump matching the two to
  three open/close cycles a 140-turn drive now crosses at period 6). The reconsideration
  discount is back to its full pre-cargo value (1.2×→1.2×, against 1.2×→1.1× at period 2):
  re-deliberation is bounded and synchronized to the market's own open/close boundaries
  exactly as the spec's acceptance describes, not a town permanently woken. Not BLOCKED.

  Full suite: 485 tests, green throughout the round (zero warnings; hlint clean; goldens
  byte-identical save the one adjudicated-then-reverted village line; bar/intrigue/feud
  untouched; ViewInvariant green).
- **v38** — **chance & feelings: a die for the drama, and the moods that use it** (`Prax.Rng`;
  `Prax.Emotion`; `CalcOp`'s `Mod`; spec `docs/specs/2026-07-15-v38-chance-feelings.md`).
  User-directed, reframed by the user at design review: emotions mostly reuse existing machinery
  (episodic facts + desires for pricing, v36 pulses for wear-off, Reactions for event context), so
  **this is an infrastructure round** — the missing primitives built as general facilities — **with
  emotions as the example application**. The user's two design calls: feelings COEXIST (not the
  Versu single-slot mood the engine shipped with since v2), and stochastic onset ships now rather
  than staying banked. THE INVARIANT, restated and pinned at both scales: **emotions change
  decision-making, never what decisions can be made** — `candidateActions` is identical in every
  mood, asserted both at fixture scale (`Prax.EmotionSpec`'s full grounded-candidate equality with
  and without every vocabulary feeling) and at world scale (carol's `candidateActions` unchanged
  angry or calm, `Prax.Worlds.VillageSpec`).

  **`Mod`, one operator, on rationale-consistency.** `CalcOp` gains `Mod` (Haskell semantics: the
  result carries the divisor's sign) — closing a gap in the project's own stated reason for
  omitting division ("keep the DB integer-valued"): modulo IS integral, so refusing it was
  inconsistent with the rationale that justified refusing division. Pinned both directions
  (`17 \`mod\` 5 = 2`, `(-3) \`mod\` 5 = 2`) and round-tripped through the JSON play-script format —
  a gap the brief's own transcription missed (`Json.hs`'s `calcTag`/`parseCalc` were non-exhaustive
  over the new constructor until a Task 1 fix closed it): an untested tag was the reviewer's one
  Medium finding, closed same-round, mutation-verified.

  **`Prax.Rng` — a die with provenance.** A deterministic random stream lives as an ordinary
  `seed!N` fact, so reproducibility, goldens, replay, and persistence all survive it for free. The
  generator is Park–Miller MINSTD (`seed' = seed × 16807 mod 2147483647`), checked against its own
  canonical stream (1→16807→282475249) with its domain guard confirmed to exclude both fixed points
  — mechanism with published provenance, fixed in the module, never tuned; the AUTHORED numbers are
  the odds, stated in the haddock as a drama die, not a statistics library. The `draw` combinator
  compiles "with probability num/den, further conds, apply outs" to an unconditional seed advance
  followed by a guarded `ForEach` — **the frozen-die law**: every draw spends exactly one stream
  step whether or not it hits, pinned directly (two consecutive draws with an unsatisfiable extra
  guard still advance the seed to exactly lehmer²(s₀), the guarded outs never fire), because a
  provocation that failed once must not fail identically forever. `SeedlessDraw` flags a
  `draw`-using world with no `rngSetup` (the `ClocklessDrift` precedent); the initial seed is an
  authored world parameter, not a mechanism constant — it selects the playthrough's fate, and
  goldens pin it (the village ships `villageSeed = 1988`, a nod to the generator's own publication
  year). The brief's transcribed module placement for the shared AST walkers
  (`conditionVars`/`outcomeVars`) did not compile as written — `outcomeVars` needs `Outcome`
  (`Prax.Types`), and `Prax.Types` already depends on `Prax.Query`, so putting it in `Prax.Query`
  would be a module cycle GHC can't take without `.hs-boot` files this project doesn't use;
  `outcomeVars` relocated to `Prax.Types` instead (`conditionVars` stayed in `Prax.Query` as
  specified) — a placement fix forced by the module graph, not a design disagreement, flagged and
  reviewer-confirmed legitimate.

  **The mood system dies — coexistence makes `setMood`'s remembered-prior machinery meaningless.**
  `feels.<who>.<emotion>[.toward.<target>]` replaces the Versu-inherited single-slot
  `mood!<feeling>.toward!<target>` wholesale: multi-valued, so angry at two people while afraid of
  a third coexist, each fact independent; a want reading the untargeted path sees targeted
  instances too (`Match` sees subtrees). `Prax.Emotion` is the new home for the Ekman vocabulary,
  wear-off (`feelingsFade`, one drift rule per world at an authored period, shipped
  test-compressed per the now-standard label), and the authoring guidance (prefer negative pricing
  — a feeling as discomfort driving its own discharge — both for the psychology and because v33's
  FloorCheck keeps unfelt negative desires planning-free; positive emotion-desires are
  action-insertable and thus AlwaysLive, allowed with the cost named).

  **The migration's own sequencing bug, caught honestly, not routed around.** The round plan
  ordered `Prax.Core`'s mood-section deletion one task before its last real consumer (`Bar.hs`)
  died — Task 2 hit exactly the condition its own brief named as a legitimate BLOCK trigger (a
  consumer reads moods in a way that changes decisions) and reported rather than improvised past
  it; the plan was amended in place (`4d7d579`) to move the deletion into Task 3, landing in the
  same commit that migrates the Bar. Task 2 shipped `Prax.Emotion` plus the four consumers whose
  migration didn't depend on the ordering (Reactions, Play, Intrigue, DirectorSpec's fixture);
  `Prax.Core` and `Prax.Emotion` coexisted for one intra-round commit, never pushed standalone —
  the plan amendment is the edict-compliance record for that.

  **The Bar: every mood reference classified, not assumed.** A full grep audit of `Bar.hs`'s 24
  mood references sorted into three piles: 2 content preconditions kept as `feelingToward` (the
  act — warning, gossiping — literally expresses the feeling); 4 pure availability gates
  (greeting, greeting-back, starting a conversation, buying a round — the brief's prose names only
  two of these as illustrative examples, the audit found two more structurally identical) REMOVED
  per the invariant and replaced with authored pricing, weights verified against the live planner
  before being pinned (grudging courtesy −3 against the existing +2 greeted-want; grudging round −8
  against the +6 bought-want, confirmed at depth-2 lookahead to actually flip the buy decision); 18
  `setMood` write sites converted to `feelToward`, dropping the now-meaningless `cause` argument.
  The bar golden (12 turns) was unaffected; the longer `LoopSpec` golden (25 turns) moved exactly
  one line — ada's earlier greeting now prices an ongoing discomfort against her own later choice
  to take offense over it, tipping a previously-narrow preference from taking offense to waiting —
  an intentional consequence of the standing-discomfort pricing model, not a bug, flagged for
  reviewer judgment and confirmed as such. A CLI narration reader (`app/Main.hs`) that queried the
  deleted `mood!` family directly — not in any task's assigned file list — was caught by the same
  grep audit and fixed to read `feels`, live-verified in play; left unfixed it would have gone
  silently, permanently blank.

  **Carol's temper: the odds sentences, and a golden unmoved of necessity.** The village cargo
  wires a `shortTempered.carol` disposition (never fading — a trait, not an episodic fact) into a
  double-armed `draw` on being shunned: a 1-in-4 base arm for anyone, a 2-in-4 second arm gated on
  the trait — both odds sentences authored, not tuned. `smoulders` prices standing anger at −8,
  discharged by carol's existing confrontation affordance. At the shipped seed (1988), the golden's
  own dramatic beat genuinely does make carol angry at dana mid-trace (computed and verified
  against the live state, not assumed) — yet the 21-turn golden window is byte-identical, BY
  NECESSITY: carol has no confront outlet inside that window, so the −8 sits as a uniform offset
  across every option and never changes her argmax. The uniform-offset claim was re-verified under
  both the broken and the fixed price shape below and holds identically either way — reviewer-
  confirmed.

  **The round's hard lesson, stated plainly: the discharge was initially INERT.** `unfeelToward`'s
  leaf-only delete left a drained-but-present `toward` ancestor standing, and `smoulders`'s bare
  subtree price (`Match "Owner.feels.angry"`) kept reading it after the "discharge" — the v32
  drained-ancestor ambiguity's first shipped-mechanic bite (`Prax.Db.retract`'s documented
  imprecision, banked at v32 as **asserted-endpoint marking**, amended below with this round as its
  first real casualty). The shipped pin was a cheater: it asserted the leaf's own absence, which is
  true, while the price it was meant to demonstrate never actually lifted. Caught by review (2
  HIGH — the inert discharge and its cheater pin; 1 MODERATE — a false Lehmer arithmetic comment),
  fixed by binding a real target leaf instead of testing bare existence — a new `feelingSomeone who
  emotion targetVar` helper — and re-pinned on `evaluateCooked`'s exact values (−7 angry → 6
  discharged, a +13 swing: the smoulder's own +8 relief plus the confront act's own +5 want firing
  simultaneously, computed not assumed), MUTATION-VERIFIED: reverting to the broken subtree shape
  reproduces the exact failure (`expected: 6 but got: −2`) before the fix restores green.
  `feelingSomeone`'s safety was by convention, not enforced, until the banked engine fix landed
  (v39: retract now prunes drained scaffolding by construction); the per-target pricing shape it
  enables (−8 per grudge, not per feeling) is the deliberate design going forward for any future
  multi-target emotion pricing, kept for that reason now, not for safety. The Bar was audited
  empirically under the same risk (probed with a planted-then-drained feeling on both shapes) and
  found already residue-safe by construction — no fix needed there.

  Suite: 514 tests, 227.74s, zero failures throughout the round; zero warnings; hlint clean; `prax
  check` well-formed on all 7 worlds; ViewInvariant green; goldens byte-identical in the bar
  (12-turn) and every world untouched by the migration (village, intrigue, feud), with the one
  adjudicated `LoopSpec` line itemized above and the village golden confirmed unmoved of necessity,
  not by omission.
- **v39** — **asserted endpoints: the trie learns which nodes are facts** (`Prax.Db`; `Prax.EL`;
  `Prax.Persist`; spec `docs/specs/2026-07-15-v39-asserted-endpoints.md`). User-directed,
  prioritized as a BUG CLASS over new content: `Db`'s trie could not distinguish an interior node
  ASSERTED as a fact from one standing only as scaffolding beneath deeper facts, and the two-sided
  evidence already on the books forced the shape of the fix rather than leaving it a judgment call.
  v32's reverted pruning had shown some interior nodes ARE facts (the Bar's practice instances must
  survive their sub-facts draining); v38's inert discharge had shown some interior nodes are NOT
  facts, and reading them as such is a lie the query layer tells (`unfeelToward`'s leaf delete left
  `feels.angry.toward` standing, and the subtree-matching price never lifted). Both requirements
  were correct; the representation was deficient. The fix: one bit, one invariant. `Db` gains a
  strict `asserted` flag beside its exclusion flag (`Db !Bool !Bool (IntMap Db)`), established
  entirely in the two mutators — `insertToks`' terminal case marks its endpoint (mid-path traversal
  and exclusion-eviction both preserve existing marks), `retractNames`' recursion prunes an
  unasserted childless child at the level it returns through instead of reinserting it — which
  establishes THE INVARIANT: the trie never contains an unasserted childless node. Queries are
  UNTOUCHED BY DESIGN: under the invariant, "node exists" is exactly "asserted, or has living
  descendants," which is what `unifySyms`/`exists`/`childKeys`/`Match` already read, so the diff to
  every query path is the mechanical 2→3-field pattern widening, no logic change. `Prax.EL`'s
  lattice extends pointwise, with the choice forced by its own laws, not picked: `meet`'s assertion
  is `a1 || a2` (disjunction) — the lower-bound law `meet a b \`leq\` a` fails under conjunction
  (pinned, ELSpec:69, the observable `["a","a.b"]` collapsing to `["a.b"]`); `leq` gains the
  conjunct `(aa || not ab)` (asserting is strictly more information than scaffolding, mirroring
  `Excl ≤ Multi`, pinned both ways, ELSpec:76). Serialization becomes MORE principled, not just
  patched: `dbToSentences`/`dbToLabeledSentences` now emit an asserted interior node as its own
  sentence alongside its descendants', and `insertAll` re-asserts each on reload — assertedness
  round-trips through plain sentences with no format change, pinned in both `DbSpec` and
  `PersistSpec` (including the asserted-interior-with-children case).

  **The RED story: two pins that had been honestly documenting their own wrong expectations,
  flipped to expect the fix.** `DbSpec`'s INSTANCE PERSISTENCE pin and its `dbToSentences`
  companion had been asserting the GHOST behavior all along — a drained ancestor reading as a live
  fact — because that was what the un-marked trie actually did; flipped to expect the post-fix
  truth, both failed against the old engine (recorded verbatim: `expected: False but got: True`),
  then passed against the new one. Six RED assertions total across `DbSpec`/`ELSpec`/`PersistSpec`,
  each failing exactly because the old trie conflated the two node kinds.

  **The adjudication, traced not assumed: zero golden movement.** The bar bell's flagged
  phantom-customer risk (`Subquery` over `customer.C`, the one site that could count a
  drained-but-present customer toward its ≥2 threshold) traced EMPTY — no shipped trajectory
  evaluates the bell against a drained customer, so the prune changes nothing observable,
  confirming v38's own sweep rather than re-litigating it. Every world transcript (bar, village,
  intrigue, feud) and every prior unit pin passed byte-identical; no threshold weakened, nothing
  re-captured.

  **The comment-truth wave.** The engine commit (`cf8ee8e`) left three shipped comments false —
  `Village.hs`'s `smoulders` haddock still describing the drained residue as live, and two test
  comments in `VillageSpec`/`EmotionSpec` still citing the retired ambiguity — caught by review as
  an internal contradiction the same commit had created (its own `Emotion.hs` haddock rewrite said
  the residue trap was gone while `Village.hs` said the opposite); fixed same-round, pure-comment
  (`d7d84b6`), re-verified clean. `feelingSomeone` is KEPT — not for safety, which is now the
  engine's job, but for the per-target pricing shape (−8 per grudge, not per feeling) v38's
  reviewer judged the better semantics.

  Suite: 521 tests (514 + 7 new pins: 4 `DbSpec`, 2 `ELSpec`, 1 `PersistSpec`; INSTANCE PERSISTENCE
  flipped in place, not added), zero failures; zero warnings; hlint clean; `prax check`
  well-formed on all 7 worlds; ViewInvariant green; goldens byte-identical throughout.
- **v40** — **hygienic machinery variables: one namespace, one guard** (`Prax.Types`;
  `Prax.Drift`; `Prax.Rng`; `Prax.Sight`; `Prax.Faction`; `Prax.Confession`; `Prax.Blackmail`;
  `Prax.Relevance`; `Prax.Repute`; spec `docs/specs/2026-07-15-v40-hygienic-variables.md`). First
  of four user-directed **foundations passes** — strict improvements (elegance/usability up,
  complexity flat or down) over new content, queued in order: v40 hygiene vars → v41 analysis
  unification on the cooked form → v42 dead-condition lint → v43 the hygiene bundle (fn/action-
  name collision guards, clock extraction from Sight, Persist version header, excl-bit trivia).
  This entry closes the queue's first item; **v41 is next**.

  **The two-tier finding.** Interface variables — `Actor`/`Owner`/`Witness`/`Hearer`/
  `Seer`/`Seen`/`Spot`/`Anyone` — are the authoring contract, not the defect: worlds write them
  deliberately to mean what the engine grounds them to, and they keep their names unchanged.
  The actual defect is machinery variables: combinator-internal names spliced into the SAME
  condition/outcome list as an author-supplied fragment (THE RULE). Wholly-generated bodies with
  no author splice — `Emotion.feelingsFade`'s own `W`/`E`, `Kin.succession`'s `H`, every
  `Function` body's call-scoped params — are out of scope by the same rule, not overlooked: the
  brief's own inventory had listed `feelingsFade` for rename, which turned out to self-contradict
  (renaming it into the Prax namespace made `driftP`'s blanket guard reject the library's own
  shipped rule at construction), caught by observed RED and correctly reverted rather than shipped.

  **The consolidation.** Five bespoke reserved-name lists and their private walkers — `Drift`
  (D/D2/Now), `Rng` (S/S2/S3/R), `Faction` (`reservedClash`), `Confession` (`reservedIn`),
  `Blackmail` — collapse into one shared pair, `Prax.Types.authoredVarClash`/`authoredPatClash`
  (~20 lines total), every call site now a single guard clause. The forbid-polarity earned its
  own parameter rename mid-round: a first `Sight.hs` draft read the list as an ALLOW set and
  passed it the sighting's own contract variables, rejecting `sightP`'s own required inputs —
  caught immediately (103 failures, every world exercising `sightP`), fixed, and the parameter
  renamed `interface`→`forbiddenSplices` with the polarity stated first in its own haddock so the
  same misreading can't recur silently. The lists themselves shrank to genuine interface splices:
  `W`/`F` in `Faction.factionStanding`, `D`/`Ds`/`N` in `Confession.incorrigible`, the whole `Rng`
  family, are now unremarkable author variable names — a usability win, positively pinned in each
  combinator's spec (not just "no longer forbidden," each has a passing test that says so).

  **Two latent gaps, found and closed, neither in the original inventory.** `Sight.sightP`
  spliced the authored sighting template into the same `ForEach` as the ticker's own machinery
  with zero guard before this round — closed by the same shared guard. `Prax.Repute.standing`'s
  own haddock claimed "`Regarder` is reserved" without enforcing it; verified genuinely spliced
  (`standingWith` builds `Match ("Regarder.believes." ++ pat)`, a direct concatenation with the
  author's own deed pattern) before acting, then enforced through the single `standingWith` entry
  point both `standing` and `standingUnless` share — 3 RED-observed pins, each confirmed
  non-vacuous (the axiom builds a valid-but-corrupt result absent the guard, not an unrelated
  failure).

  **Alpha-invariance held.** Pure renaming of generated query variables never reaches a fact, a
  label, or serialized state — goldens were byte-identical on the FIRST full-suite run, before
  any test file was touched, confirming the claim by direct observation rather than by absence of
  a diff. The world-source gate is now durable rather than a remembered discipline: `GateSpec`
  scans every `src/Prax/Worlds/*.hs` literal for a `Prax`-namespaced token, replacing what had
  been manual grep-checking, its own scanner mutation-evidenced (four discrimination cases) before
  being trusted as the gate.

  Suite: 540/540 (531 original + GateSpec's 10 − 4 old fixtures folded into usability-win
  companions = 537, + 3 Repute follow-up pins), zero warnings, hlint clean, `prax check`
  well-formed on all 7 worlds, ViewInvariant green. Lows, non-blocking: `isPraxVar` reserves the
  whole `Prax` prefix rather than tracking the spec's exact `Prax<Uppercase>` shape (sound — never
  lets real machinery through, just conservative); `Rumor.gossip`/`Deceit.lie` splice author
  fragments beside interface variables with no fresh machinery variable, so correctly outside this
  round's scope — banked here as a future-hygiene note for the v43 bundle, since no v43 entry
  exists yet to carry it.
- **v41** — **one analysis surface: the world-model analyses read the cooked form**
  (`Prax.Derive`; `Prax.Relevance`; `Prax.Engine`; `Prax.Query`; spec
  `docs/specs/2026-07-15-v41-one-analysis-surface.md`). Second of four user-directed
  foundations passes. The defect: the static analyses were split across parallel walker
  families that had to be kept mentally in sync — a string-side `wantPatterns`/
  `outcomeAtoms`/`condPatterns`/`mayUnify` beside the cooked `mayUnifySyms`/
  `cookedReadAnchors` (v34) — and the sides had already drifted once in a way only carried
  in someone's head (`wantPatterns` doesn't extract subquery internals; `cookedReadAnchors`
  does; both correct for their own consumers, invisible from either alone). The fix: cook
  first, analyze only the cooked form. `Derive`'s four axiom analyses
  (`axiomFootprint`/`axiomNegPatterns`/`axiomHeadPatterns`/`monotoneAxioms`) re-typed onto
  `[CookedRule]`; `Relevance`'s three (`improvableDesires`/`livenessOf`/`bearingTemplates`)
  re-typed onto `PraxState`, reading `cookedDefs`/`cookedRules`/`cookedDesires`/
  `cookedWants`; `retable` became two-stage (cook, then analyze the cooked tables);
  `cookedReadAnchors` moved to `Prax.Query` as the one home for the walk it shares with
  Derive's own condition-anchor logic. The string walkers are DELETED, not wrapped —
  `mayUnify`, `wantPatterns`, `outcomeAtoms`, `condPatterns`, and the string-side
  `evictionShadows` are gone from the tree (grep-proof). `relevantDelta` rewritten over
  `internTokens`/`evictionShadowNames` in place of the old `pathNames`/`evictionShadows`
  round trip. A free win fell out of the switch: cooked rules already carry the □-lifted
  forms `cookAxioms` produces, so three duplicate lift enumerations died with the string
  walkers — `axiomFootprint`'s own, `axiomHeadPatterns`'s (via the shared rules list), and
  `axiomDerivable`'s manual `obliged.W.` prefixing.

  **The one-surface rule, now stated once in both sides' module docs:** the authoring
  boundary is string (the v38/v40 splice guards run before cooking exists, and stay
  string-side by design), the world model is cooked. Everything downstream of `retable`
  reads one representation, not two kept in sync by convention.

  **Two benign notes, not zero-diff claims.** (1) The analyses' Call-resolution pool bias
  flipped last-wins→first-wins over duplicate `fnName`s, now matching the engine's own
  `lookupCookedFn` — unobservable in every shipped world (only the Bar's `tendBar`
  practice declares functions, three distinct names; a real collision is impossible until
  v43 lands its guard). (2) `axiomDerivable`'s lifted-head set genuinely shrank, and the
  plan first mischaracterized why: old code manufactured a spurious `obliged.W.<head>` for
  EVERY axiom head unconditionally, but cooked rules carry □-lifted forms only for
  liftable (all-`Match`-body) axioms via `liftObliged` — a rule with no □-form cannot
  derive one, so the new set is strictly more correct, not a spelling swap. Caught by the
  Task 2 review as a mischaracterized-evidence finding (Important, no code fix), amended
  into the plan in place (`55a8078`); unobservable in-suite because no want or gate
  candidate anywhere anchors on the literal `obliged` (grep-confirmed), not because the
  two forms are equivalent.

  **The equivalence net was laid BEFORE the switch, not after.** `test/Prax/
  AnalysisTableSpec.hs` pins every derived analysis table — `contMonotone`/`improvables`/
  `liveness`/`caresAbout`/`footprint`/`negFootprint`/`axiomHeads` — for all 7 shipped
  worlds (village, bar, bar-director, intrigue, feud, audience, play), captured
  observationally from the pre-switch code, order included (`ebd00f2`, suite 540→547).
  The switch (`2b5919e`) then had to pass all 7 pins unchanged to land at all; it did.
  Goldens byte-identical; ViewInvariant green.

  Suite: 547/547, zero warnings, hlint clean, `prax check` well-formed on all 7 worlds.
  Queue item closed: v41 is the second of the four foundations passes (v40 hygiene vars →
  **v41 analysis unification** → v42 dead-condition lint → v43 the hygiene bundle);
  **v42 is next**, and it is the first NEW analysis written against the unified surface —
  the reason this round came before it rather than after.
- **v42** — **the dead-condition lint: flag what the world can never satisfy**
  (`Prax.Relevance`; `Prax.TypeCheck`; spec
  `docs/specs/2026-07-15-v42-dead-condition-lint.md`). Third of four user-directed
  foundations passes (v40 hygiene vars → v41 analysis unification → **v42 dead-condition
  lint** → v43 the hygiene bundle), and the first NEW analysis written against v41's
  unified surface rather than a migration onto it. Check 7 in `Prax.TypeCheck`: a
  positive `Match` conjunct — top level or inside `Exists` — that may-unifies nothing
  the world can ever contain is flagged `DeadCondition` with an author-legible site
  label. Scanned sites are affordances and motives only: action conditions,
  function-case conditions, `ForEach` guards at all three outcome homes (action
  outcomes, init outcomes, function-case outcomes), desires, character wants.

  **The probe-decided scope.** A live pre-spec probe (scratch `V42DeadProbe.hs`) over
  all 7 shipped worlds found every practice condition and want clean; the only hits were
  feud's axiom bodies — machine-generated □-lifted rule bodies (`liftObliged`'s DEON
  auto-lifting, speculative by design) and `Prax.Kin.kinAxioms` wired into feud
  wholesale, with the fixture's own haddock documenting the inert remainder as
  deliberate ("inclusion is free … harmless … no `parent.*` base fact exists until a
  wedding inserts one"). Both hits draw the same line: axiom bodies are OUT of the
  lint's scope — an unfireable rule is harmless, a dead affordance or motive is the
  unambiguous-bug class.

  **One surface, one pass.** `producibleAtoms`, new in `Prax.Relevance`, is the lint's
  entire producer pool: `cookedOutcomeAtoms`'s insert half over every practice (the
  drifter included — clock-moved facts exist, and village's drawn-to-market desire
  depends on it), the initial db's own facts, every axiom head (`crHeads`, □-lifted
  forms included) whether or not its rule can fire, and the engine's own
  `contradiction` witness (`reclose` inserts it at ⊥). Written once against the cooked
  tables — sharing `cookedFnPool`/`cookedOutcomeAtoms` for the pool and `mayUnifySyms`
  as the matcher — no string-side walker returned; this is the v41 dividend the round
  exists to spend.

  **The conservativity ledger.** A wild world (an unresolvable `Call`) silences the
  lint entirely (`producibleAtoms` returns `Nothing`); unanchored patterns (every
  segment a variable) are exempt — they match everything, and `mayUnifySyms`'s
  anchored-literal rule would otherwise flag exactly the undead; negations, `Or`
  clauses, and `Subquery` interiors are unflagged, each a plausibly-intentional shape
  (a vacuously-true negation, a half-dead disjunction, an always-empty subquery as a
  `Count`-≤-0 comparison's intended meaning).

  **Verification.** RED observed with the check implemented but unwired: exactly the 4
  flag-asserting cases fail (typo'd action conjunct, dead positive inside `Exists`,
  dead `ForEach` guard, dead desire + dead want), every negative case stays green. Two
  mutations after GREEN, each killing exactly its named pin: dropping the `Exists`
  recursion in `positives` fails only the Exists case; dropping the initial-db-facts
  line from `producibleAtoms` fails only the all-worlds pin (shipped worlds lean on
  setup facts as producers). No shipped world flags — the all-worlds pin (which gained
  the missing `audienceWorld`) holds clean. Three minimal test fixtures (`DriftSpec`'s
  drifty well-formedness pin, `TypeCheckSpec`'s "correct little practice" and "ForEach
  binds" cases) gained honest producers rather than weakened pins — the lint biting a
  synthetic fixture is the fix-the-prerequisite case, not evidence against the check.

  **Round mechanics, one clause.** Task 1's implementer wedged mid-task (tree written,
  nothing verified or committed); the controller completed verification and commit, and
  the task review re-derived the load-bearing claims independently — RED reconstructed
  from scratch, the full suite re-run, the soundness ledger checked line-by-line against
  the diff — rather than trusting the report, and approved (the two mutation
  observations remain the controller's, recorded in the task report).

  Suite: 558/558 (547 + 11 new `TypeCheck` cases), zero warnings, hlint clean, `prax
  check` well-formed on all 7 worlds, goldens byte-identical. Queue item closed: v42 is
  the third of the four foundations passes (v40 hygiene vars → v41 analysis unification
  → **v42 dead-condition lint** → v43 the hygiene bundle); **v43 is next** — fn/action-
  name collision guards, clock extraction from `Sight`, `Persist` version header,
  excl-bit trivia, plus the v40 Lows: Actor-capture in `driftP`/`sightP` author bodies,
  `Rumor`/`Deceit` splice-point guards.
- **planned** — committed for later; well-understood from sources.
- **research-needed** — blocked on an external dependency (an embedding model, #42) or an unsettled
  design question (#8). The DEON 2010 exclusion-logic paper that formerly blocked #34/#8 is now
  obtained and distilled (`docs/research/deon-notes.md`).

Paper = Evans & Short 2014 (see `docs/research/versu-notes.md`). "P§" = its section/page.

## Core logic engine

| # | Feature | Status | Source | Notes |
|---|---------|--------|--------|-------|
| 1 | Trie world-state DB (all state = sentences) | v1 | P§VI | `newtype Db = Db (Map String Db)` |
| 2 | `.` multi-valued descent | v1 | P§VII | |
| 3 | `!` exclusion (single-valued, sibling-clearing) | v1 | P§VII | **fixes Praxish bug** — clears siblings, preserves child subtree |
| 4 | Prefix = object; delete subtree by prefix | v1 | P§VII | `retract` |
| 5 | Unification / pattern match (vars = Capitalized) | v1 | Praxish `db.js` | list-monad over bindings |
| 6 | Query ops: not / eq(assign) / neq / lt·lte·gt·gte / calc / subquery | v1 | Praxish `praxish.js` | typed `Condition` ADT |
| 7 | Full FOL queries: ∀, ∃, ∨, → | v8 | P§VII | `Prax.Query` `Or`/`Absent`/`Exists` + `forAll`/`implies`; nests freely |
| 8 | Static type inference / checker (ML-style) | v16–17 | P§VII p.120 | `Prax.TypeCheck` `typeCheck`. **v16** (declaration-free, sound): unbound variables, exclusion-cardinality consistency, dangling `Call`/spawn refs. **v17** (ML-style *sort* inference): sorts declared by membership (`sorts` on `PraxState`), every position/variable sort inferred by union-find and conflicts reported (agent-vs-gender). Every shipped world checks clean; the bar declares `beverage`/`place`/…; `prax check`. Sort-checking is a conservative type system (may reject genuinely-polymorphic positions; declare only monomorphic ones) |

## Practices & actions

| # | Feature | Status | Source | Notes |
|---|---------|--------|--------|-------|
| 9 | Practices as first-class instantiable objects | v1 | P§VIII | `process.<id>.<roles>` |
| 10 | Role-agnostic practices (any cast fills roles) | v1 | P§VIII | key to replayability |
| 11 | Actions: conditions + outcomes (insert/delete/call) | v1 | Praxish | |
| 12 | Practice `data`, `init`-on-spawn, `functions`/`cases` | v1 | Praxish | |
| 13 | Concurrent practices; options = union of affordances | v1 | P§V | falls out of the loop |
| 14 | Constitutive affordances (only available in-practice) | v1 | P§VIII | |
| 15 | Norms: violation-marking postconditions + norm desires | v3 | P§VIII-D | `Prax.Reactions` `markViolation`/`violationOf`; strong-negative want ⇒ planner avoids |
| 16 | Reactions as practices (spawned by an action's outcomes) | v3 | P§X | `Prax.Reactions` `spawnReaction`/`endReaction`; `disapprovalP`; response chains |
| 17 | Conditional effects / domain axioms in the action language | v15 | P§VIII | `Prax.Derive`: domain rules `body → head` forward-chained to a fixpoint (the paper's `m(X)`) over `Prax.EL`, by **semi-naive** evaluation (fire only on newly-derived facts — ~8× faster than naive at scale); reads see the closed **view** (`readView`), which is defeasible (derivations recompute from the base) and opt-in (`axioms=[]` ⇒ unchanged). Auto-`□`-lift gives obligation-closure (DEON property 1). Exact `⊥` detection. Demo: `Prax.Worlds.Feud` (`bigFeud n` scales it) |

## Agents & action selection

| # | Feature | Status | Source | Notes |
|---|---------|--------|--------|-------|
| 18 | Utility-based reactive selection (apply-evaluate-undo) | v1 | P§IX | immutability ⇒ no explicit undo |
| 19 | Per-character wants; utility = Σ modifier × #bindings | v1 | P§IX-A | Versu-faithful; supersedes Praxish global goals |
| 20 | Round-walk lookahead over believed minds, w/ discounts (0.9 self / 0.5 other) | v1, v23 | Praxish `planner.js` (v1); redesigned (v23, spec `docs/specs/2026-07-10-v23-planner-realism-design.md` §4) | `Prax.Planner`. **v1**'s `worldValue` — max over every living character's every available action, scored by the *planning actor's* own wants — is **deleted**: it was speculative (credited others with actions they'd never choose), omniscient (used movers' *true* wants), and combinatorially explosive. **v23**: `scoreActions` predicts each other character within the actor's epistemic `predictionScope` (default everyone) exactly once, myopically, from the actor's **believed** model of them (`predictMove`, `Prax.Minds`) — only if the move strictly improves that belief over doing nothing (unmotivated moves are not predicted) — in cast round-robin order after the actor, before the actor recurses on its own next choice. `depth` still counts only the actor's own future plies; the CLI/loop keep depth 2 |
| 21 | Wants as arbitrary logic sentences (∃/∀ desires) | v8\* | P§IX-A | unblocked by #7 — a want is now any FOL formula; runtime want injection needs no separate mechanism (a want gated on a fact is injectable by inserting the fact). **Closed by worked pattern in v24**: `Prax.Project`'s pursuit `Desire` is dormant (zero bindings, zero utility) for any disposed character with no project instance, and undertaking — an ordinary planner choice — inserts the very fact that switches it on; bob's `charDesires = ["pursues-earnBread"]` carries the disposition permanently, live only once he acts on it |
| 22 | Character arcs / interiority (high-level internal choices) | v7 | P§X | `Prax.Arc`; bex's hopeful→belonging/lonely arc gates its wants; against-desires transformation is player-only |
| 23 | Swaygent-style volition/influence selection | research-needed | Praxish `swaygent.js` | Ensemble-inspired alt selector |

## Core model (emotion / relationship / belief)

| # | Feature | Status | Source | Notes |
|---|---------|--------|--------|-------|
| 24 | Emotions (Ekman, single-slot, remembers target+cause+prev) | v2 | P§X | `Prax.Core` `setMood`; mood `!`-override + `priorMood` |
| 25 | Role-evaluation relationships (multiple, asymmetric, w/ reason) | v2 | P§X | `Prax.Core` `adjustScore`; `A.relationship.B.role.score!N`/`reason!Why` |
| 26 | Public symmetric relationship state | v2 | P§X | `Prax.Core` `setBond` writes both orderings |
| 27 | Beliefs: shared world + per-issue divergence | v4 | P§X | `Prax.Beliefs` `believe`/`believesThat`/`forget`; `X.believes.<issue>!V` |
| 28 | Quantified / nested beliefs | research-needed | P§XI | Versu itself couldn't do this |
| 29 | Conversation: speakers, topics, quips (template + effects) | v5 | P§X / ES blog | `Prax.Conversation` `quip`/`changeSubject`; speaker turn-taking; quips shift core model & beliefs |

## Story management & authoring

| # | Feature | Status | Source | Notes |
|---|---------|--------|--------|-------|
| 30 | DM / story manager as a special practice | v6 | P§VI, XI | bar `director`: a bound metalevel agent with story-level wants; injects a rivalry |
| 31 | Player as DM | v13 | P§XI | `Prax.Worlds.Bar` `barDirectorWorld`: the human is bound to the metalevel `direct` practice (stir a rivalry / kindle warmth / cast a pall) and steers an autonomous cast; the CLI offers a bound player only its practice's affordances (via `candidateActions`). `prax dm` |
| 32 | Readable serialization for play-scripts (JSON) | v12 | P§VII-VIII | `Prax.Script.Json`: round-trips a `Script` to/from JSON — an editable authoring/exchange format with no bespoke grammar to maintain; `prax play <file.json>`, `prax dump-play`, `examples/play.json`. (Chosen over a custom `.prompter` parser.) |
| 33 | Prompter-style play-script front end (scene/beat/junction → practices) | v12,18 | P§XII | `Prax.Script`: CAST + scene-graph eDSL, `compile`, auto `flowChart`; a bodiless narrator fires junctions. **v18** adds the deferred compilation features: **memories** (`memory` — one-shot exposition on first-trigger), **timed junctions** (`after`/`timeout` — a passive scene clock), and **character sketches** (`concernedWith` → wants, `withTraits` → facts). Scene *parameters/bounds* are subsumed (affordances are already scene-local). The readable text surface is deliberately omitted — JSON (#32) stands in. |
| 34 | Deontic `should` / obligation operator; norm-conflict resolution | v14 | DEON 2010 | `Prax.Deontic`: `□φ` = fact `obliged.<who>.<φ>` (the paper's `Ob:φ` sugar, no semantic change); conflict *detection* via `!`-exclusion collapse (property 2); breach reuses `violated.…`; contrary-to-duty (`□□`) via nested obligations; behavioural coupling by Wants, planner unchanged. Resolution is *emergent* (utility) — explicit priority is a documented extension. Gaps: no entailment-closure (property 1), no `m(X)`/LRT (that's #8). Grounding: `docs/research/deon-notes.md` |

## Runtime, tooling, UX

| # | Feature | Status | Source | Notes |
|---|---------|--------|--------|-------|
| 35 | CLI menu loop (act / more), narration | v1 | P§V UI | |
| 36 | Round-robin turn loop | v1 | Praxish `app.js` | |
| 37 | Deterministic playback / replay | v10 | P§VI | pure loop ⇒ reproducible traces (golden replay); mid-session save/resume-to-file via `Prax.Persist` (v11) |
| 38 | Runtime inspector ("why is X true / why did preconds fail") | v10 | P§VI | `Prax.Inspect` `explain`/`firstFailing` (revives `killsPerStep`) |
| 39 | Stress-test harness (many auto-played runs) | v10 | P§VI | `Prax.Stress` — seeded random all-AI runs; endings + action coverage + dead-ends; **scene coverage** (which scenes random play reaches — Prompter's report) added in v12; CLI `prax stress` |
| 40 | Serializable world state (save/load) | v11 | P§VI | `Prax.Persist` (facts + cursor); exact round-trip; CLI in-game `s` save + `resume` |
| 41 | Rich branching multi-character episode (content) | v9 | P§XII | `Prax.Worlds.Intrigue`: murder, character death (cast-removal), betrayal/loyalty/complicity endings, romance. `Prax.Worlds.Play` (v12) recasts it as a 2-scene play-script |
| 43 | Cast removal (death / eviction) | v9 | P§VIII-D | `dead.<name>` fact; `Prax.Types.livingCharacters`; loop/planner skip the dead |
| 42 | PWIM embedding-based free-text player input | research-needed | arXiv 2406.00942 | external model dependency |

## Open research to close
- Only **#42** (PWIM free-text input) remains research-needed, and it is an external-model dependency,
  not a paper to obtain.
- The **DEON 2010 paper** (`references/papers/EVAIEL.pdf`, distilled in `docs/research/deon-notes.md`)
  is obtained and fully applied: it grounded the deontic layer #34 (v14) and the `m(X)` derivation
  closure #17 (v15). #8's type checker (v16–17) turned out to need only sentence-structure analysis,
  not the full LRT decision procedure.

## Future ideas to investigate
- **Incremental view maintenance for the derivation layer (#17)** *(done — v27: the
  irrelevant-delta fast path + the monotone-insert continuation; see the legend row)*. What
  remains deliberately unbuilt: DRed-style truth maintenance for the non-monotone residue —
  measured not warranted (from-scratch closure is ~5% of a profiled round after v27; the
  continuation already computes exactly the delta derivation support-tracking would). Revisit
  only if a future world's axiom mix (heavy defeaters, anti-monotone counts) pushes the reclose
  share back up — the ViewInvariant net makes any such attempt safe to try. Smaller residual
  notes, unmeasured beyond the v27 profile: tokenization inside the continuation's delta-joins
  (~40% of the now-small round) and the per-primitive classification cost
  (`mayUnifyNames` ~8%) — diminishing returns at current scale.
- **Planner runtime under cast growth (v25) — substantially addressed by v26.** The v25 regression
  (full suite ~38.66s → ~726s when gale joined; `Prax.VillageSpec` alone ~580–660s; no isolated
  pre-growth group timing was ever taken) was profiled in v26 and turned out to be dominated by
  decision-irrelevant work, not by the realism itself: recomputed axiom closures (~15k per round
  for the same states), predictions provably no action could motivate (1,373 `predictMove`
  calls/round contributing zero decisions at sampled states), string re-tokenization (~48% of
  runtime), and tests re-simulating overlapping trajectory prefixes. After v26's exact
  eliminations: suite ~114s, Village group ~116s, one free-play round 2.83s. What remains is the
  faithful cost of realism (`transparent` making each trait-bearer a mind others predict; the
  round-walk itself) multiplied by one closure per distinct lookahead state — the incremental-view
  item (#17, designated v27) is the remaining lever. Fact growth was measured NOT to be a factor
  (51 → 97 sentences over 7 rounds); memory GC is not a lever here.
- **Hard priority tiers for action selection (from Praxish's `swaygent.js`).** Ensemble/CiF-style
  selection tags actions with a symbolic tier — `forbidden` / `required` / `normal` — that sorts
  *above* numeric utility, giving categorical "you must / may not" rules. Our planner and norms are
  all *soft* (a strong-negative want steers away, but nothing is inviolable). Borrowing tiers would
  give the deontic layer (#34, v14) **hard** norm enforcement: an obligation ⇒ `required`, a
  prohibition ⇒ `forbidden`. It is a selection-paradigm change, not a Versu feature — Swaygent is
  Praxish's alt selector, whereas we (faithfully) use Versu's utility planner — and combining hard
  tiers with N-ply lookahead (prune forbidden branches, propagate required) is the non-trivial part.
  A "beyond Versu" enhancement, not a parity gap.
- **Asserted-endpoint marking for `Prax.Db`** *(DONE — v39, banked v32, evidenced v38; see the v39
  legend row above; spec `docs/specs/2026-07-15-v39-asserted-endpoints.md`)*. Banked at v32 when
  naive ancestor-pruning on retraction was tried, RED/GREEN-pinned, and reverted: `Prax.Worlds.Bar`'s
  `tendBarP` instance fact anchors transient per-customer state beneath it, and pruning drained
  that instance away with its last customer — the trie could not tell "an asserted fact that
  happens to be childless" from "an ordinary ancestor, now childless because its only occupant was
  retracted," both represented identically. Evidenced at v38, which gave the ambiguity its first
  shipped-mechanic bite: `Prax.Emotion.unfeelToward` left a drained `.toward` ancestor standing,
  and `smoulders`'s subtree price kept charging for a discharged feeling, safe only by the
  `feelingSomeone` convention, not by construction. Landed at v39: `Db` gained a strict `asserted`
  bit beside its exclusion flag, `insertToks` marks it, `retractNames` prunes an unasserted
  childless node eagerly at the level it returns through, establishing the invariant this entry
  was written to close — the trie never holds an unasserted childless node — with queries
  untouched by design and `feelingSomeone`'s safety now enforced by construction, not convention.
- **Footprint discrimination indexing** *(banked — v33, found while profiling the residual gap
  the round's A/B left after the state-aware relevance filter shipped)*. The controller's
  profile at HEAD attributes ≈11% of the profiled round to `mayUnifySyms` inside
  per-primitive classification: the atom-pool footprints every axiom/desire is tested
  against have grown with the village's authored vocabulary since v28, so each may-unify
  scan now walks a larger pool than it did at the world this cost was last measured against.
  An index keyed on a cheap discriminant (head symbol, arity) could narrow a footprint scan
  to only the atoms that could possibly may-unify before falling back to the general test,
  rather than scanning the whole pool per candidate. Located by profiling, not designed or
  attempted this round.
- **Axiom-family partitioning for the continuation loop** *(banked — v33, the same profiling
  pass)*. `deltaJoinCooked` (~17%) and `num` (~3.7%) together are the closure continuation's
  own cost for the village's Count-bearing aggregate axioms (`notoriety` ×2, `incorrigible`)
  — three of them now run in every continuation, where the 31.11s-era world ran none.
  Partitioning axioms by family (Count-bearing vs. plain Horn) so a continuation re-evaluates
  only the family a given delta could possibly affect, rather than every axiom unconditionally,
  is the natural lever this profile points at — unbuilt, and not designed against a concrete
  world this round beyond the profiling that found it.
- **Below-existing-instance practice-path bounding** *(banked — v34, found while attributing
  Task 2b's opacity residual)*. The two templates still opaque after the safe-binder rule —
  `Go to [Place]` (`Insert "practice.world.World.at.Actor!Place"`) and `take up honest work`
  (practice-namespaced progress inserts) — are opaque because a literal-`practice`-headed
  insert can in general bring a new practice instance into being, which `groundedDeltaAnchors`
  cannot bound. But both templates only ever insert *beneath an instance path that already
  exists* at prediction time (the world/place registry and the endeavor's own staged practice
  are both spawned once, at world construction or undertake, never per-move) — `Prax.Engine`'s
  `spawnedInstanceNames` already tracks exactly this existed-before fact. A refinement that
  checks the insert's instance prefix against `spawnedInstanceNames` before falling back to the
  unconditional practice-opacity rule could bound these two templates exactly, un-opaquing the
  two paths that currently poison every route through a `Go to` or `honest work` step — not
  attempted this round; the attribution pass located the lever, it didn't design the check.
- **Per-reachable-head cone precision** *(banked — v34, the same attribution pass)*.
  `extendDelta` joins every `axiomHeads`-reachable family into the cone the moment a delta
  feeds any of them, rather than only the heads actually reachable *from that specific delta*.
  For the village, one whisper's delta feeds the reputation axioms and so drags every mover's
  `regards` read into cone∩read-set intersection, even for movers whose own read is on a
  disjoint head reachable from a *different* fed family. A per-head reachability refinement
  (propagate only the heads the delta's own fed families can actually reach, rather than the
  transitive closure over all axioms touched) could free some of the 74% currently sitting in
  INTERSECTION back into REUSE — though the raw `recanted.<actor>` anchor dependency (not
  axiom-derived at all) would still defeat the pairs that read it directly, so this bounds the
  achievable gain, it doesn't eliminate the live-recompute floor. Unbuilt; located by profiling
  attribution, not designed against a concrete implementation. **Both levers, probe-tested
  at the outer loop by v35's investigation and found insufficient there**: a chained cache
  upgraded with the per-head-cone lever served **zero** picks, because the village's own
  axiom graph chains co-presence into reputation regardless of cone precision — the outer
  loop's cost was closed by v35's reconsideration semantics, not by sharpening this family
  further; both levers stay banked, now scoped explicitly to within-pick precision only
  (narrowing which nodes of a single already-triggered deliberation reuse a prediction —
  never to skipping deliberation itself).

## Sandbox extension backlog (brainstormed 2026-07-10)

Target frame: a **large-cast, long-time sandbox** where the player is one agent among many;
**symbolic only** (no external models — PWIM #42 stays parked). All of these are *beyond Versu*.
Unless marked foundational, each compiles onto the existing layers and can be taken or left in
isolation. **K is the keystone**: most of Tier 1–3 stacks on it (marked ⤷K).

**K. Witnessing / event deposits** *(done — v19: `ForEach` quantified outcomes in the engine;
`Prax.Witness` authored observability; `Prax.Worlds.Village` seed, CLI `prax village`)*. When an
action is performed, co-located characters acquire a persistent *belief that it happened*;
characters elsewhere don't. Generalizes v3's per-action authored reactions into "react to any
action" (the old event-bus idea). Resolved as a small engine primitive (`ForEach`, quantifying an
outcome over every binding) plus a compiled-per-action combinator (`observable`) built on it — not
a separate hook. Unlocks information asymmetry — the root of reputation, rumor, secrets, alibis.

Tier 1 — compiled social structures:
- **⤷K Gossip / rumor propagation** (`Prax.Rumor`) *(done — v20: `gossip`/`heard`, authored per
  event-pattern like `observable`; evidence is a prefix match on `believes.<event>`; spreading is
  want-driven)*: share a held belief with a co-located, relationship-gated hearer, planting the
  same belief. Reuses `Prax.Beliefs`; false rumors already work. Reputation travels. *Resolved in
  v20:* provenance is multi-valued (`.seen`/`.heard.<source>`), so a `.heard.<source>` edge for
  someone who witnessed **and** later hears the same event sits *beside* their `.seen` edge instead
  of overwriting it — evidence accumulates and corroboration (multiple named sources) is countable.
- **⤷K Reputation** (`Prax.Repute`) *(done — v21: `standing`/`standingUnless`/`regardedAs`/
  `notoriety` — per-observer standing derived from believed deeds
  (`believes.X.(stole.Y._) ⇒ regards.X.Y.thief`); defeated by a *base-fact* defeater, not by
  deleting the belief — atonement, not amnesia, so re-offense (which revokes the defeater fact)
  makes standing snap back from memory that was never lost; `notoriety` counts derived regards at
  an authored threshold)*. Score effects from standing (a reaction, not an axiom) remain unbuilt —
  not needed for the village's arc. `notoriety`'s counting shape (an outer existence check and an
  inner counting `Subquery` deliberately naming the counted role differently) gets a **second use
  in v32**: `Prax.Confession.incorrigible` points the identical Count idiom inward — regard
  yourself, not a third party — to derive an absolver's patience from what she *believes*, not
  from any separately bookkept count.
- **Factions & membership** (`Prax.Faction`) *(done — v31, folded with Kinship below onto one
  shared spine: **membership**. `member.<who>!<faction>` is a base, single-slot fact; the `!` is
  the whole semantics — joining, defecting, and marrying-in are the same exclusion overwrite.
  `comrades` generalizes the feud's old pairwise `allied.*` setup facts ("my faction's enemy is my
  enemy") into a derivation from shared membership, keeping the `allied` name so every existing
  consumer (the mutuality axiom, the feud's shun affordance) needs no change — proved by
  `Prax.Worlds.Feud`'s refactor, where `FeudSpec`'s 5 original tests pass byte-unmodified.
  `factionStanding` (belief-gated regard through a faction-mate, `standingUnless`'s shape) ships
  spec-tested but unwired into any world. Join/leave/exile practices and place-scoped deontic
  norm-sets are not this round's scope — `joins`/`comrades`/`factionStanding` are the vocabulary;
  authoring practices on top of them is free)*. Banked: multi-affiliation (one character, several
  factions at once), faction offices/leadership beyond bare succession, place-scoped deontic
  norm-sets, village wiring for `factionStanding`.
- **Debt & favors** (`Prax.Debt`) *(done — v30: `owe`/`settle`, thin over `Prax.Deontic` — a debt
  *is* an obligation with a beneficiary, `debt.<creditor>.<debtor>.<content>` inserted alongside
  `oblige`, both facts one call, one call to reverse both. Default becomes belief-gated **deadbeat
  standing**: a witnessed breach (`Prax.Witness.observable` wrapping `Deontic.breach`) derives
  `regards.<W>.<debtor>.deadbeat` via `standingUnless`, defeated by repayment inserting
  `atoned.<debtor>` — the same positive-fact defeater idiom `Prax.Repute` (v21) already uses, not a
  new mechanism. An unwitnessed default derives no *third-party* regard — but the debtor is
  unavoidably co-present at his own default, so he always regards himself a deadbeat regardless of
  any outside witness, a self-regard/third-party-spread distinction review found underspecified and
  the shipped test now asserts explicitly)*.
- **Kinship & households** (`Prax.Kin`) *(done — v31, folded with Factions above: kinship is what
  *generates* memberships. Base vocabulary is `parent.<parent>.<child>` and `married.<a>.<b>`;
  `kinAxioms` is pure derivation (marriage symmetry, sibling, grandparent, two in-law rules —
  **stated one-directional**, acquired-relative-first) — retraction-safe for free, with a designed
  asymmetry: dissolving a marriage un-derives every in-law, but membership does **not** un-derive,
  since `wed`'s transfer is a base `!` move, not a derivation. `wed joiner faction spouse` compiles
  a wedding to the marriage fact plus one membership overwrite — inheritance-as-bond, generalized
  past the original "marriage as bond+obligations" framing into the same exclusion idiom
  membership already uses. Offices generalize identically: `office.<name>!<holder>` + `succession`,
  a claim gated on the holder's death and the claimant being a child — the single slot resolves
  competing claims to one, honestly, with no invented age/primogeniture)*. Banked: inheritance of
  holdings beyond bare offices, births (a `parent.*` fact must be asserted, never generated by
  play), divorce as a driven action (dissolution is tested via raw retraction only).

Tier 2 — agent interiority for long time-spans:
- **Projects / endeavors** (`Prax.Project`) *(done — v24: `endeavor`/`Stage` — authored project
  types compile to an undertake action, a staged one-instance-per-owner practice, and a named,
  dormant pursuit desire that rewards completed stages directly, so horizon length never enters
  the planner's lookahead; a witnessed stage is theory-of-mind content the moment `Prax.Minds`
  believes it. `Prax.Worlds.Village`'s `earnBread` closes the village's moral arc: deterred,
  concealing bob is given honest work, takes it up unprompted, and the village learns his
  purpose by watching)*. Banked residuals, not attempted this round: **abandonment** (walking
  away from an in-progress instance mid-stage — the current model has no "give up" outcome, only
  completion); **cooperative projects** (multiple owners on one instance — `roles = ["Owner"]`
  is deliberately single-slot); **type synthesis** (authoring a *family* of endeavors from a
  higher-order description rather than one `endeavor` call per project type).
- **Personality → volition** (`Prax.Persona`) *(done — v25: traits as **conduct-valuations**, not
  goal-bundles. A first draft bundling goals directly (`vengeful` ≡ installs [my grudges avenged]
  +k) was rejected — pressed, a bearer behaved identically to a character handed the desires
  directly, so the layer added nothing real; a goal is a plain desire needing no trait. The shipped
  model instead values the bearer's own *conduct*: a `Trait` bundles desires over the bearer's own
  conduct-marks (`honest` costs a lie-mark, not forbids the lie), `personaVocabulary`/`bearing`/
  `cast` wire a roster's traits into desires and setup facts, and `transparent` derives that a
  bearer's valuations are presumed, defeasibly, from t=0)*. `Prax.Worlds.Village`'s gale/eve
  contrast demonstrates it: identical spite, different temperament, different conduct.
- **⤷K Secrets & deception** (`Prax.Deceit`) *(done — v22: `conceal`/`lie` — a concealment want
  (`Absent [Anyone believes <deed>]`) makes the planner avoid witnesses automatically, lookahead
  already simulating the v19 witness deposits; `lie` plants the same `.heard.<liar>` hearsay as
  `gossip`, so a fabrication is indistinguishable from truth once heard, and hearing your own lie
  back turns it right back into gossip — the lie/gossip duality that makes the whole v20/v21 stack
  run on a falsehood unmodified)*. `Prax.Worlds.Village` gains a villain on this: bob conceals his
  theft; eve frames carol out of authored malice, and the frame-up cascades through the unmodified
  v20/v21 machinery to real shunning and notoriety, with an honest injustice — the framed have no
  recourse (amends needs a loaf never taken).
- **Ground-truth event records & exculpation** *(rejected, v25 — spec §2, overturning the v22 §5
  banked idea)*: an event record (deed tokens / a calendar) actions could be checked against was
  banked as "the honest way to eventually let the framed clear their name." Design review
  overturned it: **history persists only through the marks it makes** — beliefs, memories,
  consequences — and the vocabulary must be able to reach states where the truth is genuinely
  unrecoverable, which a world-rooted, narrator-consultable event ledger would foreclose by
  construction (it would be an oracle nothing in-world holds). v25's `lie` gains a residue instead:
  a mark on the liar alone (`<liar>.lied.<hearer>.<event>`, their own memory — owned, forgettable,
  perishable), never a record anyone can consult as ground truth. Truth recovery, if it is ever
  built, is committed to flow through mark-bearers — confession, testimony — never consultation.
  **The confession half arrives in v32** (below): a lied-mark converts to a confessed one and
  deposits sourced testimony through the ordinary hearsay channel, exactly the mark-bearer path
  this commitment named. It closes the liar's *own* road back — her mark, her standing — not a
  third party's frame-up: carol, framed by eve in v22, still has no recourse, since nothing in
  v32 lets anyone but the liar herself confess to a lie she told.
- **Confession & absolution** (`Prax.Confession`) *(done — v32: `confess`/`absolve`/
  `incorrigible` — a lied-mark converts to a confessed one (never deleted, so a trait can still
  price the residue); confessing self-incriminates through the ordinary sourced-hearsay channel
  (v20), so the whole reputation stack cascades on it exactly as on gossip; absolution is a
  separate, refusable second-party act that inserts the world's own standing-defeater; an
  absolver's patience (`incorrigible`, `Prax.Repute.notoriety`'s Count idiom pointed inward) is
  what she *believes*, permanent by memory and per-absolver. `Prax.Worlds.Village` wires eve's
  road back onto it: confessing to gale, who already regards her a slanderer, costs nothing and
  unlocks absolution; confessing to the *actually* wronged party (carol) was probed and found
  structurally incapable of beating eve's baseline at any authored generosity — documented, not
  shipped)*. Banked: recidivism into character (an offender's own future disposition shifting from
  repeated lies needs `charDesires` to be fact-driven rather than a static field — a `Prax.Minds`
  engine change); un-deceiving the original hearer (retracting a planted content-belief needs
  belief-retraction semantics); confessor-side penance obligations; public (one-to-many)
  confession; priest-like roles.
- **Blackmail** (`Prax.Blackmail`) *(done — v30, split out from v22)*: `shakedown` compiles the
  four-action protocol (threaten/comply/defy/expose) the session probe validated live before the
  spec was written. A threat is a motive-belief deposit (the same channel confiding/lying already
  ride); credibility is self-motivation, not omniscience — the extortionist's own punitive desire
  is what motivates threatening in the first place, so a myopically-unmotivated planner move
  correctly can't foresee compliance, yet the threat is credible anyway (character coherence). A
  standing threat is exposable too (stalling ties defiance rather than dominating it — the classic
  hole closed). The compliance arithmetic is pinned both sides in `BlackmailSpec`, ported straight
  from the probe: two onlookers, comply beats wait/defy (−63.84 vs −71.84/−75.80); one onlooker,
  defy and wait tie exactly (−54.2), comply no longer worth it. `Prax.Worlds.Village`'s carol/eve
  arc instantiates it for real: threshold fear (its own legend entry, above) makes a single
  witnessed whisper land two of the three regards notoriety needs, and carol's shakedown extracts
  real silence from that one witness's worth of leverage. Bluffing, threat expiry, and
  counter-blackmail are out of scope, banked below.
- **Repeat / serial extortion** *(banked — v30, found by the planner's own lookahead)*: porting the
  session probe into `shakedown`'s `comply` surfaced a real gap before it was guarded — an unguarded
  repeat threat let the planner's recursive lookahead discover it could be paid off again, inflating
  the two-onlooker buy score to −51.24 against the guarded, canonical −63.84 (`Prax.BlackmailSpec`).
  The gap is closed for this round (`comply`'s guard against an already-standing debt, mirroring the
  probe exactly) — but escalating, serial extortion (a debt that grows, or a threat that renews on
  its own clock) is a real, planner-discovered future mechanic, not merely a hypothetical extension,
  banked here rather than built.
- **Counterfactual placement (per-agent world-views)** *(banked — v23 spec §4a "honest residual")*:
  a predicted in-scope mover is still simulated at their *true* position, not the predictor's
  *believed* one — imagining them where the predictor thinks they are requires giving every
  predictor its own simulable view of the world, the per-agent-world-view machinery Versu itself
  declined to build. Base facts leaking into predictions and template-fixed believed weights (no
  per-observer intensities) are the same residual: full per-agent world-views, deferred wholesale.
- **Sighting recency-salience** *(DECLINED by design — user decision, v35-era backlog review)*:
  `Prax.Sight` sightings are single-slot and `sightedWithin` gates prediction scope with a hard
  ticks-since-sighted window. A smooth recency-weighted confidence model was banked at v23 and is
  now explicitly rejected, permanently: the hard window IS the intended model. The gradient would
  add authoring surface and per-pair evaluation arithmetic inside the scope check — the hottest
  gate in prediction — for no gameplay-visible behavior difference; complexity up, cost up,
  utility nil-to-negative. Do not re-propose.
- **Decay & drift** *(DONE — v36, see the legend row above; spec
  `docs/specs/2026-07-14-v36-drift.md`)*: episodic state on the clock; the original "scores cool
  toward baseline" framing was REJECTED in review (dispositions never decay — they change through
  acts; games represent hours-to-weeks).
- **Emotions** *(DONE — v38, see the legend row above; spec
  `docs/specs/2026-07-15-v38-chance-feelings.md`)*: episodic, coexisting feeling-states, priced by
  ordinary author-chosen desires, with stochastic onset and a drift-pulse wear-off — shipped
  exactly along the lines this entry anticipated (v33's liveness skip, v35 signatures, and v36
  pulses served as the existing stack the entry predicted they would; the stochastic-onset piece
  it flagged as round-sized became `Prax.Rng`). Residue, banked: per-feeling fade stamps
  (`feelingsFade` sweeps every standing feeling on one shared pulse per world regardless of onset
  time — coarse by design, until a world needs per-instance timing, per its own haddock);
  per-emotion periods (one period per world today, not per emotion); emotion visibility to other
  minds (believed feelings, deterrence-by-anger — v38 ships own-planning pricing only); intensity
  levels (a feeling is present or absent, no magnitude). The **asserted-endpoint marking** item
  (above, banked at v32) is elevated, not closed by this round — see its own entry for the v38
  casualty that raised its priority.
- **Calendar & gatherings** *(DONE — v37, see the legend row above; spec
  `docs/specs/2026-07-14-v37-gatherings.md`)*: recurring clock-gated scene spawns (market
  day) ship, formalized as the `gathering` combinator over `Prax.Drift`'s pulse rules — the
  mixing dynamic banked for is now measured, not asserted (percolation pinned at 4
  market-witnesses vs. 1 quiet-witness in `VillageSpec`). Festival content beyond the
  market instance and multiple simultaneous gatherings stay out of scope, per the spec.

Tier 3 — host-game boundary:
- **⤷K Chronicler / salience queries** (`Prax.Chronicle`): derived summaries over the event stream
  ("a feud started", "the mayorship changed hands") — quest-hook generation, and the answer to
  emergence nobody can see.
- **Embedding API**: a `step / inject / query` surface for a host engine; design once a host exists.

Foundational watchlist (high bar; none currently urgent): hard priority tiers (above) — wait for a
demonstrated soft-norm failure; incremental view maintenance (above) — Tier 1 multiplies axioms ×
cast, so measure then decide; locality-scoped action discovery / level-of-detail — premature before
a large world exists to profile. Notably *not* foundational: runtime want injection (#21) — a want
gated on a fact is injectable by inserting the fact.
