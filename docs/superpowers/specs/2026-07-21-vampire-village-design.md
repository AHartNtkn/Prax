# Vampire Village — design

A hidden-role social-deduction simulation built on the Prax engine: a village
where, after the first day, one inhabitant becomes a vampire. Vampires feed
(spreading the curse) while hiding what they are; the rest must discover the
threat and end it — by killing, or by the priest's holy cure. The game ends when
no living humans remain (the vampires win) or no vampires remain (the village
wins). The purpose is a **full-fidelity emergent simulation**, run many times to
collect statistics and surface rare, unscripted storylines, and — if the runs
are interesting — playable as one of the villagers.

## The thesis: nothing by fiat

Every dramatic beat emerges from wants weighed by the planner, never from a
scripted guarantee. The vampire's caution, a villager's hesitation to snatch a
scarf on thin suspicion, a mob forming, a family shielding a turned relative, a
feud curdling into a false accusation — all of it is the utility calculus of
full-fidelity NPCs reasoning (within their sight-scope) about consequences. This
is why depth-2 omniscient-within-scope deliberation earns its cost *here*: the
game IS the scheming. (Recorded principle: deals/deterrence/betrayal are
emergent utility comparisons, never mechanism guarantees.)

## Cornerstones (locked with the designer)

1. **Full-fidelity NPCs.** The existing planner (depth-2, sight-scope-gated
   prediction) for every villager. Performance is whatever it measures to;
   fidelity is not traded for throughput.
2. **Detection = witnessed acts + the mark/scarf/disguise web.** A vampire is
   detectable, and that detectability is the vampire's central dilemma:
   - A **bite** leaves a **neck mark** on the victim. Every vampire bears a mark
     (patient zero included — the mark is how vampirism manifests; its *origin*
     is off-screen, not an on-screen bite).
   - A **scarf** covers the neck mark. Wearing one is ordinary — *except* it is a
     social faux pas in specific contexts (e.g. in church), so a scarf worn there
     is itself mild evidence.
   - The mark is exposed when the scarf comes off: voluntarily, or **snatched**.
     Snatching is not ordinary behaviour and carries a **standing penalty if the
     accusation proves false** (no mark under the scarf) — so it is only worth
     doing on real suspicion.
   - **Disguise:** a vampire can disguise before feeding so a witness/victim
     learns "*someone* bit me," not *who*. The disguise is fragile and its
     identity-masking can leak.
   Detection is by being **witnessed** (within a villager's sight scope), then
   carried by **gossip** — the same machinery the village already runs for
   witnessed theft.
3. **Isolated infection.** A vampire cannot tell another vampire from a human.
   No coordination; a victim may not know who turned them. Maximises paranoia and
   tragedy: a vampire may unknowingly accuse another; a hunter may be infected and
   not know the neighbour beside them is too.
4. **Priest-and-mob elimination.** Only the **priest** can perform the holy
   **cure** — the vampire must be restrained / on holy ground. **Anyone can
   kill**, but a kill is socially fraught, permanent, and (if the target was
   human) murder. Cure saves the soul but is hard; killing is easy but costly.

## The core loop

```
   hunger builds ──► vampire seeks an unwitnessed moment ──► FEED (bite)
        ▲                                                      │
        │                                       leaves a neck MARK on victim,
        │                                       arms a 24h TURN timer, sates
        │                                       hunger, arms a 24h feed cooldown
        │                                                      │
   24h cooldown ◄─────────────────────────────────────────────┤
                                                               │
        (if witnessed) ──► belief "someone/ X bit Y" ──► GOSSIP spreads
                                                               │
   victim wears SCARF to hide the mark ◄── mark is evidence ──►│
        │                                                      ▼
        │                                              SUSPICION accrues
   snatch SCARF (risky) ──► mark EXPOSED ──► confirmed ──►  ACCUSE
        │                        │                              │
   (no mark → false-           │                    ┌──────────┴──────────┐
    accusation penalty)        ▼                    ▼                     ▼
                          24h later: TURN         KILL (fraught,      restrain +
                          (victim → vampire,      permanent, may       PRIEST CURE
                          gains hunger+survival)  be murder)          (holy ground)
```

Endings: **vampire win** = no living humans remain (all turned or dead);
**village win** = no vampires remain (all killed or cured).

## The emergent drives (utility layer)

**Vampire wants** (its behaviour is the planner reconciling these):
- **Blood-hunger** — a recurring pressure (the village `hunger` mechanic
  re-skinned) sated by a bite. A bite *both* feeds and infects; spread is the
  *byproduct* of feeding, not a strategic goal (consistent with isolated
  infection). Hunger supplies urgency to feed even when it is risky.
- **Survival** — the vampire wants not to die. Exposure → suspicion → mob or
  priest → death/cure. From this, *emergently*: feed unseen, keep the mark
  covered, avoid the church-scarf faux pas, deflect suspicion. The depth-2
  lookahead is what lets it foresee "if I feed in view, they will witness,
  gossip, and come for me."

**Villager wants:**
- Ordinary life-wants (work, family, standing) — the social substrate, largely
  inherited from the village world.
- **Fear / self-preservation** — sharpens as evidence of vampires mounts,
  driving investigation, gossip, accusation, scarf-snatching, and killing/curing
  — each weighed against its cost (the false-accusation penalty, the moral weight
  of a kill). Acting on thin suspicion is *punished*, so the community moves only
  when evidence is strong — emergent, never scripted.

## Prax realisation (design level)

Reuses the village substrate directly: movement/locations (`world` practice),
the **sight/perception clock and scope** (already the gate that makes NPCs
non-psychic), **gossip/whisper**, **relationships & feuds**, **beliefs**,
**endings**, and **schedule** (market/Sabbath). `prax-vocab` modules likely
reused: `witness` (sighting→belief), `repute` (standing/suspicion).

New facts (illustrative, not final): `vampire.X`; `mark.X.neck`;
`wearing.X.scarf`; `turning.X` (+ 24h expiry → `vampire.X`); `bloodHunger.X`;
`fed.X` (+ 24h feed-cooldown expiry); `Y.believes.vampire.X`,
`Y.believes.bit.X.Z`, `Y.believes.mark.X`; `accused.Y.X`, `slander.Y.X` (a
disproven accusation → standing penalty); `restrained.X`; `cured.X`; `dead.X`;
`ending.vampires`, `ending.village`.

New practices/actions (illustrative): **vampire** — `feed` (co-located target,
hungry, cooldown clear → mark + turn-timer + sate + cooldown; witnessed → belief;
disguise → identity-masked belief), `disguise`, `wear/remove scarf`, `deflect`;
**villager** — `snatch scarf` (suspect Y; mark present → exposed; absent →
`slander` penalty; gated by suspicion), `accuse`, `gossip` (reuse whisper),
`kill`, `restrain`; **priest** — `cure` (restrained target on holy ground);
**world** — movement/wait.

Time: **day/night** phases on the turn clock (night = sparse sightings = the
feeding window; day = market, Sabbath-in-church, gossip). Cooldowns/timers are
expiries. Runs ~20–30 days to an ending.

## Phase 1 — the vertical slice (this spec's build target)

Prove the loop **generates the intended drama** and measure real perf, at a
tractable size, before scaling.

**Scope in:**
- **Cast ~8**, reusing the village's relationship/feud substrate, with the
  structure the loop needs: the **priest** (cure); **patient zero** (turns on
  night 1); at least one **standing feud** (false-accusation fuel) and one
  **family bond** (protection instinct); a natural gossip-carrier. (Concrete
  roster is Phase-1 authoring, e.g. Father Aldric the priest, Mara the herbalist
  as patient zero, a baker/blacksmith/widow/watchman, a feuding pair.)
- **Locations:** square, church (holy ground + scarf faux pas), a couple of homes
  / the mill — enough that movement produces co-presence and *gaps* in sight.
- **Day/night** cycle; the two **cooldown/turn timers**; both **endings**.
- The full loop: feed → mark → scarf → witness → gossip → suspicion → expose →
  kill/cure → turn.

**Scope out (later phases):** the full ~30 cast, professions/schedules at scale,
mass-run tooling, play-tuning.

**Success criteria & validation** (this is *new* content — no frozen oracle, so
validation is behavioural, per the project's test-as-you-build rule):
1. `type_check` clean for the world; it builds and runs without engine errors.
2. **Per-mechanic tests** (each written with its feature): a bite leaves a mark
   and arms the turn timer; a scarf hides the mark from witnesses; a snatch
   exposes a present mark and penalises a false one; a witnessed feed creates the
   right belief (identity-masked under disguise); the turn fires at 24h; the feed
   cooldown blocks a second bite; the priest's cure removes a vampire; each
   ending fires on its condition.
3. **The loop closes both ways** in play: a seeded run reaches the vampire
   ending, and another reaches the village ending.
4. **Emergence check:** across a stress sweep, *both* endings occur with a spread
   of run-lengths, and at least one run exhibits an unscripted beat (e.g. a
   feud-driven false accusation, or a family shielding a turned member). This is
   the real signal that the drives interact rather than just execute.
5. **Perf measured** (not gated): record village-vs-this per-turn cost at cast 8,
   so Phase 2's scale-up starts from a real number.

**Open risks:**
- Whether depth-2 lookahead is deep enough for a vampire to *foresee* the
  witness→gossip→mob chain and act cautiously, or whether caution needs a more
  direct want (e.g. a standing/suspicion aversion) to surface at depth 2. Resolve
  empirically in Phase 1; it is the crux of whether the emergence works.
- Balance of the false-accusation penalty vs. fear: too high and no one ever
  snatches (vampires never caught); too low and mobs murder innocents constantly.
  Tune against the stress distribution.

## Roadmap (later phases, each its own spec → plan → build)

- **Phase 2 — scale:** ~30 cast with professions, families, feuds, richer
  locations and day-schedules; tune the sight-scope density.
- **Phase 3 — mass-run & mining:** extend the existing stress harness (which
  already runs-many and aggregates endings/coverage) to log full run transcripts
  and surface statistically-rare or dramatically-interesting runs.
- **Phase 4 — play:** tuning and presentation for playing a villager via the CLI.
