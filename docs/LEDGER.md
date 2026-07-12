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
  one stated invariant — entity names never collide with predicate-name literals); pattern
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
  not needed for the village's arc.
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
- **Sighting recency-salience** *(banked — v23)*: `Prax.Sight` sightings are single-slot (a new
  sighting overwrites the old) and `sightedWithin` gates prediction scope with a hard
  ticks-since-sighted threshold — a sighting one tick old and one at the horizon's edge are
  credited identically. A recency-weighted salience model (confidence decaying smoothly with
  elapsed ticks rather than a boolean cutoff) is banked, not built; needs a principled decay
  function before it's more than a heuristic.
- **Decay & drift**: scores cool toward baseline via a bodiless ticker (the v18 `_clock` pattern);
  rates must be authored world parameters with stated semantics, not tuned constants.
- **Calendar & gatherings** *(partially seeded — v23: `Prax.Sight`'s ticker already advances a
  global `turn!N` every round, the first brick of the clock; what's missing is authored
  clock-gated scene spawns keyed off it, not the clock itself)*: recurring clock-gated scene
  spawns (market day, festival) — the mixing dynamic that makes gossip percolate.

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
