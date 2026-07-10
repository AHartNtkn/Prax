# v22 — Secrets & deception (`Prax.Deceit`)

Backlog item **⤷K Secrets & deception** (`docs/LEDGER.md`, "Sandbox extension backlog"). The
information stack (witnessing v19 → rumor v20 → reputation v21) gets its adversarial layer:
agents who *manage* what is known — concealing their own deeds, and fabricating others'.

Both mechanisms were verified empirically against the live village before this spec was written:

- **Stealth is emergent from one want.** A concealment want (`Absent [anyone believes <deed>]`)
  made bob *wait* while carol and you stood in the square, steal the moment it emptied, and leave
  no beliefs behind — the planner's lookahead already simulates the v19 witness deposits, so
  avoiding witnesses falls out of utility. No stealth system exists or is needed.
- **The frame-up cascade runs on existing machinery.** Given a malice want, the planner *chose*
  to lie; the fabrication planted ordinary `.heard.<liar>` hearsay, v21 derived
  `regards.<hearer>.carol.thief` from it, and the deceived hearer was immediately offered
  `tell … that carol stole the loaf` — the lie propagates as truth because hearsay and
  fabrication are indistinguishable to everyone but the liar.

## 1. API (`Prax.Deceit`)

```haskell
-- | A desire that nobody believe <event> — the planner then avoids witnesses
-- by itself (lookahead simulates the witness deposits). The weight is authored
-- character: how much is the secret worth?
conceal :: String -> Int -> Want
-- conceal event k = Want [ Absent [ Match ("Anyone.believes." ++ event) ] ] k
```

- Reserved variable: **`Anyone`** (the `Regarder`/`Witness`/`Hearer` convention) — the event
  must not use it. The event may use no variables at all (a concrete secret, the common case) or
  variables bound by nothing — but an unbound variable would be a bug, so `conceal` requires a
  **variable-free** event (loud `error` otherwise; the v21 defeater-scope guard's precedent).
  A concealment want quantifies over *observers*, not deeds.

```haskell
-- | An action: assert an event you have NO evidence for, to a co-present
-- hearer, binding the fabricated subject from world-supplied conditions.
lie :: CoPresence      -- who can be told (as in Prax.Witness/Rumor)
    -> [Condition]     -- the world's gate (as in gossip; may be [])
    -> [Condition]     -- fabrication: binds the pattern's variables (whom you COULD frame)
    -> String          -- event pattern, e.g. "stole.Culprit.loaf" (first variable = subject)
    -> String          -- action label, e.g. "[Actor]: whisper to [Hearer] that [Culprit] stole the loaf"
    -> Action
```

`lie copresence gate fabrication pat label` builds `action label conds [effect]` with `subject` =
the pattern's first variable (loud `error` if none — the `gossip` convention):

```haskell
conds =
  fabrication                              -- binds the pattern's variables from the world
  ++ [ Neq subject "Actor"                 -- framing yourself is a confession, not a lie
     , Absent [ Match (beliefAbout "Actor" pat) ] ]   -- no evidence: this is what makes it a lie
  ++ copresenceForHearer                   -- Witness→Hearer rename, as in gossip
  ++ [ Neq "Hearer" "Actor"
     , Neq "Hearer" subject                -- you don't tell carol that carol stole
     , Absent [ Match (beliefAbout "Hearer" pat ++ ".seen") ]   -- an eyewitness knows what they saw
     , Not (beliefAbout "Hearer" pat ++ ".heard.Actor") ]       -- one-shot per hearer
  ++ gate

effect = Insert (beliefAbout "Hearer" pat ++ ".heard.Actor")
```

The effect is **identical to gossip's** — that is the design's core claim: the deceived hold
real hearsay evidence and the whole v20/v21 stack (retelling, corroboration, standing,
notoriety, shunning) runs on the falsehood unmodified. Consequences that fall out and are
tested, not authored:

- If the liar ever *hears their own lie back*, they acquire evidence — the `lie` action
  disappears (its `Absent` gate fails) and plain `gossip` appears in its place, seamlessly.
- A lie about a deed someone actually witnessed differently is still tellable (beliefs about
  distinct events don't exclude each other) — contradictory reputations can coexist, per
  observer, which is correct: nobody in-world holds ground truth.

## 2. Demo: the village gains a villain

- **bob conceals**: `conceal "stole.bob.loaf" 12` (authored: the bread is worth +10; not being
  thought a thief is worth more). Layering: concealment *prevents* (he waits for an empty
  square); the v21 shame want *responds* (if seen anyway, notoriety tips him into amends).
  Player-facing stealth: stand in the square and the bread never vanishes; walk to the mill and
  it does.
- **eve joins** (starts at the **mill** — placement matters: she must not witness the scripted
  thefts that v19–21 tests force, keeping their two-witness arithmetic intact). Her motivation
  is authored malice: `Want [ Match "regards.W.carol.thief" ] 4` — she wants carol ill-regarded,
  per head. The village's `lie` declaration fabricates over `stole.Culprit.loaf` with
  fabrication = any located character.
- **The injustice is honest**: framed carol is regarded, then shunned — and has **no recourse**:
  the amends action requires `holding.Actor.loaf`, and she never took one. The walkthrough says
  this plainly. Exculpation needs in-world ground truth (an event record actions could be checked
  against) — banked as a future item with the calendar/deed-token note, not paved over here.
- The player gets the same `lie` affordance (whisper campaigns are available to you too).

## 3. Interaction with existing tests (verified reasoning, to be re-verified empirically)

- All v19–21 arc tests force the theft via `doAct`, so bob's concealment (a *pre-theft* deterrent
  in autonomous play) does not affect them. His post-theft behavior is unchanged: once witnessed,
  the concealment want is broken in every branch, contributing a constant.
- eve acts in autonomous drives, so long-drive captures change (the frame-up interleaves) — the
  docs re-capture, and v21's bob-specific assertions (`regards.carol.bob.thief`, `shunned.*.bob`)
  are unaffected by carol-directed facts.
- The "three regards make notoriety" arithmetic holds: eve at the mill sees neither scripted
  theft; witnesses stay you+carol, hearsay stays dana.

## 4. Tests (TDD)

- `DeceitSpec` (minimal inline fixture, like RumorSpec): `conceal` shape (+ loud error on a
  variable-containing event); a concealer picks the deed only when unobserved (pickAction both
  ways — the probe, as a regression test); `lie` plants `.heard.<liar>`; the no-evidence gate
  (a speaker WITH evidence is offered gossip-shaped truth-telling, not this lie); self-framing
  excluded; subject never the hearer; one-shot per hearer; hearing your own lie back replaces
  lie with gossip (the `Absent` gate closes); loud error on a subject-less pattern.
- `VillageSpec` additions: bob never steals from t=0 autonomous play while the player stands in
  the square (stall intact, no beliefs, N turns); after you and carol leave, bob steals unseen
  (holding, no beliefs, no regards ever); eve frames carol in autonomous play (someone comes to
  hold `…stole.carol.loaf.heard.eve`, `regards.<w>.carol.thief` derives, carol ends up shunned);
  framed carol is offered no amends (`return the loaf` absent from her menu).
- Regression: all v19–21 tests green unmodified (per §3); every world `prax check` clean.

## 5. Out of scope (parked deliberately)

- **Blackmail (v23)**: obligation-under-threat composes Deontic + Rumor, but its leverage model
  (exclusivity of knowledge; why the blackmailer withholds rather than gossips) deserves its own
  design round.
- Ground-truth event records + exculpation/lie-detection (banked with deed tokens/calendar).
- Cover stories, misdirection about one's own deeds (needs the looks-like/is gap `observable`
  reserved in v19 — a later tier).
