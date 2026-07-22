# Vampire Village — detection (Phase 2 slice)

The second sub-project on top of the [infection skeleton](2026-07-21-vampire-village-design.md).
The skeleton proved the loop *closes* but the vampires always win — there is no way to discover the
threat. This slice builds the **information layer**: how a witnessed act, a lingering mark, and
gossip turn into *suspicion* — and, crucially, the vampire's **emergent counterplay** of hiding.
It stops short of elimination (accuse / kill / priest-cure), which is the next slice; here suspicion
forms and spreads and the vampire visibly changes its behaviour to avoid it, but suspicion is not
yet lethal.

## The thesis holds: nothing by fiat

The vampire does not hide because a rule says "vampires hide." It hides because being *identified*
is intrinsically costly to it (a `conceal` want, the village's own idiom), and the depth-2 planner,
foreseeing that an undisguised bite tells the victim exactly who bit them, chooses to disguise
first. Every tell has an honest use, so no observation is proof — suspicion is always a weighed
inference, never a certainty. The paranoia is the point.

## Cornerstones (settled with the designer)

1. **Two evidence channels.**
   - *The act:* the `feed` action becomes **witnessed** — every co-present character, **including the
     victim** (who is always co-present), comes to believe a bite occurred. An axiom bridges that to
     suspicion: believing `X bit Y` ⟹ believing `X` is a vampire (biting is how vampirism manifests).
   - *The evidence:* an exposed neck **mark** is visible — believing `X` is marked ⟹ suspecting `X`
     (a mark means bitten-or-turned; either way, suspect).
2. **Disguise masks the apparent identity.** The witnessed bite records the biter's *apparent*
   identity, not their true name. A single slot `appears.X!<name>` is normally `appears.X!X`; a
   `disguise` action flips it to `appears.X!someone`. An undisguised bite deposits `bit.X.<victim>`
   (names X → the victim believes `vampire.X`); a disguised bite deposits `bit.someone.<victim>` (no
   attribution). The vampire holds `conceal("vampire.<self>")`, so it disguises before feeding —
   and **isolated infection emerges**: a disguised biter means even the victim cannot name who
   turned them.
3. **The mark is hidden by a scarf — but a scarf is not a tell.** Every vampire and fresh victim
   bears `mark.X.neck`; `wearing.X.scarf` suppresses the mark-evidence channel. Two things keep the
   scarf honest:
   - **Winter + a stay-warm want.** The village is set in winter and everyone holds a mild want that
     a scarf satisfies (warmth), so wearing one is ordinary — a scarf is *not* evidence in the cold.
   - **The church faux-pas.** Wearing a scarf on holy ground carries a social cost. For an ordinary
     villager warmth < that cost indoors, so they bare their neck in church; a vampire keeps the
     scarf on to hide the mark despite the cost — so **a scarf worn in church is the anomaly**, the
     one place the tell survives.
4. **Every tell is ambiguous.** Symmetric with #3's scarf:
   - **Disguise** has an honest use — at least one villager holds a **disreputable-indulgence** want
     (slip to a den / an illicit tryst) plus a want *not to be seen* doing it, so they disguise too.
     A masked figure might be a vampire mid-bite or a philanderer; snatching the mask may reveal a
     mark, or merely a scandal.
   - A wrong guess is punished: **snatching** a disguise/scarf and finding *no* mark inflicts a
     **false-accusation penalty** (a `slander` standing hit on the snatcher — the village's own
     `standing_unless` defeasible-standing idiom), so the community only acts on real suspicion.
5. **Suspicion spreads by gossip, not omniscience.** Beliefs are witnessed within sight scope and
   carried by the existing `rumor` machinery (`.heard.<source>` edges beside `.seen`), so evidence
   accumulates across the village without anyone being psychic.

## The information flow

```
   FEED (now witnessed) ──► every co-present incl. the VICTIM believes `bit.<apparent>.<victim>`
        │                                    │
   if disguised: <apparent> = "someone"      │  if undisguised: <apparent> = the biter's name
   → no attribution, concealment kept        ▼
        │                          W.believes.vampire.<biter>   ◄─ axiom: witnessed bite ⟹ suspect
        │                                    │
   the lingering MARK ──► (scarf off, or snatched) ──► W.believes.mark.X ⟹ W.believes.vampire.X
        │                                    │
   GOSSIP (rumor) spreads the belief ────────┤
                                             ▼
                                    SUSPICION accrues (derived, defeasible)
                                             │
                    ┌────────────────────────┴───────────────────────┐
              SNATCH a suspect                              (elimination — NEXT slice:
              mark present → confirmed                       accuse / kill / priest-cure)
              no mark      → SLANDER penalty
```

## The utility layer (what the planner reconciles)

**Vampire** (inherits blood-hunger + sate-hunger from the skeleton; adds):
- `conceal("vampire.<self>", k)` — intrinsic cost of being believed a vampire. This is the whole
  engine of caution: it makes an undisguised bite (which tells the victim) *worse* than spending a
  turn to disguise first, and makes an uncovered mark worse than wearing a scarf. The depth-2
  lookahead is what lets it foresee "bite in the open → the victim believes `vampire.me`."
- the same **stay-warm** and **church-faux-pas** terms every villager has (so its scarf choices read
  as ordinary — until church).

**Villager** (inherits the skeleton's home-anchor + sate-hunger; adds):
- **stay-warm** — a mild want satisfied by `wearing.<self>.scarf` (the winter substrate).
- **church-faux-pas** — a cost for `wearing.<self>.scarf` while on holy ground, outweighing warmth
  there for an honest villager.
- **a disreputable indulgence** (held by at least one) — a want to reach a den / tryst, paired with
  a want to conceal it → they disguise en route, making disguise ambiguous.
- (Fear-driven investigation — snatching on suspicion — is present as the `snatch` affordance with
  its slander penalty, but *acting* on a confirmed vampire is the elimination slice.)

## Prax realisation (design level)

Reuses, directly: `witness::observable`/`witnessed` (the bite becomes an event), `rumor` (gossip),
`repute::standing_unless` (the slander penalty; suspicion as derived defeasible standing),
`deceit::conceal` (the vampire's and the philanderer's concealment wants), the sight/perception
clock and scope, beliefs, and the schedule.

New facts (illustrative): `appears.X!<name>` (apparent identity; `!someone` when disguised);
`disguised.X`; `wearing.X.scarf`; `W.believes.bit.<apparent>.<victim>`; `W.believes.vampire.X`
(derived suspicion); `W.believes.mark.X`; `onHolyGround.X` (in church); `slander.Snatcher.X` (a
disproven snatch → standing penalty); den/tryst facts for the indulgence.

New practices/actions (illustrative): `disguise` / `drop disguise` (anyone); `wear scarf` /
`remove scarf` (anyone); `snatch` (expose a suspect — reveals a mark or inflicts slander); the
**den/tryst** indulgence practice with the tempted villager's action; and the `feed` action gains
its `observable` wrapper keyed on `appears`. New axioms: witnessed-bite ⟹ suspicion; mark-seen ⟹
suspicion; the mark-evidence channel gated by `wearing.scarf`; the church-scarf faux-pas signal.

## Success criteria & validation (behavioural — no frozen oracle)

1. `type_check` clean; the world builds and runs.
2. **Per-mechanic tests** (each with its feature): a witnessed undisguised bite makes the victim
   believe `vampire.<biter>`; the same bite while disguised makes them believe only
   `bit.someone.<victim>` (no `vampire.<biter>`); a scarf suppresses the mark channel; a scarf in
   church is flagged while a scarf in the cold is not; a snatch that finds a mark confirms, one that
   finds none inflicts `slander`; a disreputable villager disguises without being a vampire; gossip
   carries a bite belief to an absent villager.
3. **The crux — emergent caution.** In a seeded planner run the vampire, valuing concealment,
   **disguises before feeding** rather than biting in the open, and keeps its mark covered — with no
   rule commanding it. This is the whole point of the slice; if depth-2 is too shallow to surface it,
   that is the finding to resolve (tune the `conceal` weight; the want is principled, its magnitude
   reuses the village `conceal` scale).
4. **Ambiguity holds.** At least one run produces a **disguise false-positive** (a masked innocent
   snatched to a scandal, not a mark) or a **scarf false-positive** (a cold villager scarfed for
   warmth, not a mark) — proof that the tells are inferences, not proofs.
5. **No elimination.** Suspicion forms and spreads but nothing yet kills or cures; the endings are
   unchanged from the skeleton. (Confirm the vampire still wins — it is now merely *harder to spot*.)

## Open risks

- **Depth-2 sufficiency for concealment.** The crux (#3 above). The `conceal` want gives the vampire
  a *direct* one-step-visible cost (bite → victim believes), so depth 2 should suffice — but the
  disguise-then-bite plan is two actions, exactly at the horizon. Resolve empirically; if it needs a
  nudge, the lever is the `conceal` weight, not a deeper search.
- **Scarf/faux-pas balance.** The church-scarf signal only survives if warmth < faux-pas indoors and
  warmth > 0 outdoors. Two inequalities to satisfy with principled weights; tune against behaviour.
- **Slander calibration.** Too high and no one ever snatches (marks never exposed); too low and the
  village harasses innocents. Tuned against the stress distribution once elimination gives snatching
  a payoff — here it is present but low-stakes.

## Scope out (later slices)

Elimination (accuse → kill / priest-cure) and the lethal stakes that make suspicion *deadly*; scale
to ~30; mass-run mining; play tuning. This slice is the information layer and the vampire's hiding.
