# Walkthrough: understanding `prax` by playing it

This guide is in two parts.

**Part I — the bar** walks you through the default demo world and, as you go, points out exactly
which engine feature each thing exercises. The bar exercises the whole *engine core*: the
exclusion-logic database, practices, the utility planner, the core model (emotions/relationships),
reactions & norms, beliefs, conversation, the drama manager, character arcs, first-order queries,
and — wired straight into "settling up" — the deontic obligation layer.

**Part II — beyond the bar** is a shorter tour of the capabilities the bar *doesn't* show, each in
its own world or tool: a branching dramatic episode with a death (`intrigue`), the QA tooling
(`stress`), save/resume, scene-authored drama (`play`, `flow`), playing the drama manager yourself
(`dm`), emergent derivation (`feud`), the static checker (`check`), and the Prompter compilation
features (`audience`).

Start with the bar:

```sh
cabal run prax
```

You control **`you`**. The other two characters act on their own each turn. Turn order is
round-robin: `you`, then `ada`, then `bex`, repeating. On your turn you pick a numbered action,
or `m` to wait (let others act), `s` to save, or `q` to quit.

> Menu **numbers shift** as options appear and disappear, so this guide names actions rather
> than numbering them. Pick the option whose text matches.
>
> Note: `m` (pass your turn) and the in-world "Wait a moment" action do the same nothing, so the
> player menu hides "Wait a moment" — use `m`. NPCs still have it (they need a "do nothing"
> option so an idle agent isn't forced to wander).

---

# Part I — the bar (the engine core)

## The world at a glance

**Places:** `entrance` ⇄ `bar` (connected both ways).

**Cast:**
- `you` — the player. No built-in desires; you decide.
- `ada` — the bartender. Wants: *don't leave orders outstanding* (utility −5 each) and *stay at
  the bar* (+1). So she tends the bar and serves people without being told to.
- `bex` — a patron. Wants: *a beer on order* (+4), *a beer in hand* (+9), *be at the bar* (+1).
  So bex will walk in, order a beer, and settle once served.

**Practices** (the reusable social situations):
- `world` — locations and movement (`Go to`, plus a universal `Wait a moment` no-op used by
  idle NPCs; hidden from your menu since `m` already passes your turn).
- `greet` — greeting a co-located character (once each direction).
- `patron` — marks someone a patron; seeds their drink counter when created.
- `tendBar` — order → fulfill → drink, getting tipsy, plus a "busy bar" bell.

---

## Guided playthrough

### 1. Watch autonomy before you do anything

On your first few turns, just press **`m`** (wait) three or four times and watch the narration.
You'll see something like:

```
  ada: Greet you
  bex: Go to bar
  ...
  bex: Order beer
  ada: Fulfill bex's order
```

**What this demonstrates:** *utility-based autonomous action selection with lookahead.* Nobody
scripted bex to walk to the bar and order — bex evaluates the world that each action would
produce, and (looking two moves ahead) sees that going to the bar enables ordering the beer it
wants. ada, seeing an outstanding order (which she dislikes), fulfills it. This is the heart of
Versu's "strong autonomy": practices only *offer* actions; the agents choose.
→ code: `Prax.Planner` (wants, `worldValue` lookahead), `Prax.Loop` (turn taking).

### 2. Move around

On your turn, choose **`you: Go to bar`**.

**What this demonstrates:** the `world` practice's movement action. Its precondition matches
where you are (`at.you!OtherPlace`) and a connection (`connected.OtherPlace.Place`); its effect
`Insert practice.world.world.at.you!Place` moves you. Because `at` is an **exclusion (`!`) slot**,
inserting the new location automatically removes the old one — no explicit delete needed.
→ features: `Match`, `Insert`, the corrected `!` semantics. code: `Prax.Worlds.Bar` `worldP`,
`Prax.Db.insert`.

### 3. Greet someone

At the bar with others present, choose **`you: Greet ada`** (or `bex`).

**What this demonstrates:** a precondition that both parties are co-located and that you haven't
already greeted them (`Not practice.greet.World.greeted.you.ada`), plus `Neq` to stop you
greeting yourself. Greet once and the option disappears (the `Not` now fails).
→ features: `Match`, `Neq`, `Not`, negation-as-failure. code: `greetP`.

### 4. Order a drink and get served

Choose **`you: Order beer`**. Then take your turn again (or press `m`) — on her turn **ada will
fulfill your order on her own**, and the scene will show *"you has a beer in hand."*

**What this demonstrates:**
- The **order → fulfill** hand-off between two roles of a **multi-role practice**
  (`tendBar` has roles `Place` and `Bartender`). Ordering writes
  `customer.you!order!beer`; fulfilling deletes the order and writes `customer.you!beverage!beer`.
- ada serving *without being told* — her want *"no outstanding orders"* (−5) means the world
  where your order is gone scores higher, so `Fulfill` is her best move.
- `dataFacts`: the four beverages (`beer`, `cider`, …) come from the practice's static data,
  which is why you get exactly those four order options.
→ features: multi-role practices, `Match`/`Eq`, `Insert`/`Delete`, `dataFacts`, wants driving an
NPC. code: `tendBarP` (Order/Fulfill), `ada`'s wants.

### 5. Drink it — and drink another to get tipsy

Choose **`you: Drink the beer`**. Then order another beer, get it served, and drink again. After
the **second** alcoholic drink the scene shows *"you is looking tipsy."*

**What this demonstrates — the richest single mechanic:**
- **`init`-on-spawn:** when the world was created, `Insert practice.patron.you` spawned a patron
  instance, and the practice's `init` seeded `practice.patron.you.drinks!0`. That counter exists
  only because of spawn-time initialization.
- **`Call` + functions with guarded cases:** `Drink` doesn't compute anything itself; it calls
  `recordDrink`. That function has two cases — the first fires only for `alcoholic` drinks
  (`Eq Kind alcoholic`), the second is a no-op fallback.
- **`Calc`:** the alcoholic case increments the counter (`Calc M add N 1`) and writes it back.
- **nested `Call` + `Cmp`:** it then calls `checkTipsy`, whose single case fires only when the
  counter reaches the threshold (`Cmp gte M 2`), inserting `person.you.tipsy`.

Order a *soda* (non-alcoholic) and drink it: your counter won't move — that's the fallback case.
→ features: `init`, `Call`, functions, `FnCase` guards, `Eq`, `Calc`, `Cmp`. code: `tendBarP`
`functions` + the `Drink` action.

### 6. Make the bar busy — the bell

Get **two** people to be customers at once (you've ordered, and bex has too — bex usually orders
early on its own). Watch ada's turns: once there are two or more customers, **ada rings the bell
on her own** and the scene shows *"the bar is busy — ada rang the bell."* (You can also confirm
it only happens with two: early on, with just one customer, the bell never rings.)

**What this demonstrates:** an action precondition that **counts** things:
- a **`Subquery`** gathers the set of customers (`Crowd`),
- **`Count`** turns that set into a number (`NumCust`),
- **`Cmp gte NumCust 2`** gates the action,
- and `Not …rang` makes it fire only once.
→ features: `Subquery`, `Count`, `Cmp`. code: `tendBarP` "Ring the bell" action.

### 7. Feelings & relationships — the core model (v2)

Characters now build up an emotional and relational interior as they interact, and that interior
changes what they do. Watch the scene's new lines (`… feels … toward …`, `…'s warmth toward …`).

- **Warm up to someone.** On your turn, **`Greet ada`**, then **order a beer** and let ada serve
  you. Each of those raises your `warmth` toward ada (greeting +10, being served +8). Watch the
  scene: *"you's warmth toward ada"* climbs from 10 to 18.
  → features: numeric relationship evaluation. code: `Prax.Core` `adjustScore`; wired into
  `greetP`/`tendBarP` in `Prax.Worlds.Bar`.

- **A relationship creating a new goal.** Once your warmth toward ada crosses 15, a brand-new
  option appears: **`Buy ada a drink`** — an affordance that literally did not exist until you'd
  warmed up to her. This is the point of the core model: relationships open up behaviour.
  → features: relationship-gated precondition (`Prax.Core` `scoreAtLeast` = `Match` + `Cmp`).

- **Snub someone and watch them cool.** When an NPC greets you and you **don't greet back**, on a
  later turn they *"Take offense at you ignoring your greeting."* The scene then shows they
  *"feel annoyed toward you"* and their warmth toward you goes **negative**. An `annoyed` mood (and
  the cooled warmth) then **withholds** their friendly "buy you a drink" gesture.
  → features: single-slot mood override (`setMood`, the `!` operator), a negative `adjustScore`,
  and mood-/score-gated preconditions (`Not …mood!annoyed…`, `scoreAtLeast`).

- **Emotions are momentary; the record persists.** A mood is single-slot — a new feeling
  overrides the old one, and the previous mood is kept as `priorMood`. So after a character is
  cheered back up, the *mood* is no longer "annoyed", but the lasting **grievance** and the
  lowered **warmth score** remain. That's how a fleeting feeling differs from a durable relationship.

- **Feelings are asymmetric.** Because warmth is directional, you'll routinely see one character
  warmer than the other (e.g. `bex's warmth toward ada: 38` while `ada's warmth toward bex: 30`),
  and both NPCs cold toward a player who never reciprocates.
  → features: asymmetric role evaluation. code: `Prax.Core`.

*(Beliefs and conversation come next, in §9–§10; public "bonds" via `setBond` show up as the
`lovers` romance in §14.)*

### 8. Reactions & norms (v3)

Actions now provoke *responses*, and the bar has a social rule with teeth. Watch the scene's
`… hasn't returned …'s greeting`, `… owes … a tip`, and `… broke a norm …` lines.

- **A greeting is a two-part exchange.** When someone greets you, the scene notes you *"hasn't
  returned"* their greeting, and your menu gains responses that didn't exist a moment ago:
  **`Greet … back`** (mutual warmth) or **`Rebuff …`** (both cool). Greeting *back* is the reaction
  consuming itself — not a fresh greeting.
  → features: a reaction spawned by an action; a response that consumes it. code: `Prax.Reactions`
  `spawnReaction`/`endReaction`; `respondGreetP` in `Prax.Worlds.Bar`.

- **Ignore a greeting and it comes back on you.** If you *don't* respond, the greeter can — on
  their turn — **take offense that you ignored them**, leaving a grievance and cooling toward you.
  (In the NPC replay, ada does exactly this to the always-silent player.)

- **A norm with consequences.** Order a drink and get served: the scene shows you now *owe ada a
  tip*. Choose **`Tip ada`** (she warms to you) or **`Leave ada's tab unpaid`**. Stiff her and the
  scene marks *"you broke a norm (stiffedTheBartender)"*; on her next turn ada **disapproves**, and
  her warmth toward you drops sharply.
  → features: `markViolation`; a violation spawning the ready-made `disapproval` reaction;
  core-model consequences. code: `Prax.Reactions` + `settleUpP`.

- **NPCs respect norms on their own.** bex is given a strong aversion to stiffing plus a small
  liking for tipping, so when served it **tips** rather than walking out — the planner sees that the
  violation→disapproval future scores far worse. That's the paper's "strong desire to respect
  norms" falling out of ordinary utility evaluation, no special rule engine.
  → features: norm avoidance via `Prax.Planner` lookahead + a large negative `Want`.

- **"Owes a tip" is a real obligation (v14).** That tip isn't just a reaction — being served
  raises a first-class **deontic □**: `obliged.you.(you.tipped.ada)` (the scene's *"you owes ada a
  tip"*). **Tipping** *discharges* the duty (it's met and closed); **stiffing** *breaches* it and,
  because the original duty can no longer be met, raises a **reparative □□** — a contrary-to-duty
  obligation to make amends. The planner pursues obligations because a small `Want` values fulfilled
  duties; conflicting duties collapse to ⊥ under the `!` exclusion and are left for utility to
  resolve. So the same "settle up" you saw as a norm is, underneath, Evans' exclusion-logic deontic
  logic (DEON 2010).
  → features: `oblige`/`discharge`/`breach`/`obligeReparative` (`Prax.Deontic`); wired into
  `settleUpP` in `Prax.Worlds.Bar`. Asserted in `Prax.DeonticSpec`.

### 9. Beliefs — what a character thinks may not be true (v4)

The world state is shared, but a character can hold a private belief about a specific issue that
diverges from the truth — and act on the belief, not the fact. In the bar this shows up as
believed grudges.

- **Plant a rumour.** If you're *cross with* someone (e.g. you rebuffed ada, so you're annoyed at
  her), then — while she's **out of the room** — you can **`Warn [someone] that ada resents them`**.
  That plants a belief in the hearer: the scene will read *"… believes ada resents them."* The
  claim needn't be true; ada may actually like them.
  → features: a per-agent belief formed by telling. code: `Prax.Beliefs` `believe`; the "Warn …"
  action in `Prax.Worlds.Bar` (gated on real annoyance + the subject's absence, so nobody gossips
  idly).

- **A false belief overrides real feeling.** A character who believes someone resents them **won't
  greet or buy a drink for** that person — even if their actual `warmth` is high. Belief beats
  fact for driving behaviour (Versu's whole point in modelling beliefs separately).
  → features: belief-gated preconditions (`Not …believes.resentedBy…`).

- **Beliefs are private and can disagree.** The rumour changes only the hearer's mind; others (and
  the truth) are untouched — two characters can hold opposite beliefs about the same issue.

- **Evidence can change a mind.** If the supposedly-hostile person actually **greets** the
  believer, they can **`Realize [they] don't resent you after all`** and drop the false belief.
  → features: belief revision (`Prax.Beliefs` `forget`).

*(What v4 doesn't do, per `docs/LEDGER.md`: quantified/nested beliefs — "X believes that everyone
thinks …" — which Versu itself couldn't represent; and there's no single "believe-or-else-the-truth"
query operator, since that needs disjunction, a later item.)*

### 10. Conversation — quips, topics, and taking turns (v5)

Two characters who like each other can actually *talk*, and what they say changes the world.

- **Strike up a chat.** Once you're warm enough toward someone (the same threshold as buying a
  drink), **`Strike up a conversation with [them]`** appears. It opens on *small talk*; the scene
  shows *"you and ada are chatting (smallTalk)"*.
  → features: a conversation is a spawned practice with a *selected speaker* and a *topic*. code:
  `Prax.Conversation` `beginConversation`; the "Strike up …" action in `Prax.Worlds.Bar`.

- **Quips are lines with consequences, and you take turns.** Only the current **speaker** may
  quip, and only on the current **topic**; saying a quip applies its effect and hands the floor to
  the other person. On *small talk* you can make small talk (a little mutual warmth); steer to
  *rapport* and you can **compliment** them (raising their regard for you); steer to *gossip* and,
  if you're cross with a third party, you can **confide that they resent your companion** — a
  gossip quip that plants a (possibly-false) **belief**, exactly the v4 mechanic delivered through
  dialogue.
  → features: `quip` (speaker + topic gated, one-shot, passes the turn), `changeSubject`; effects
  reuse `Prax.Core` and `Prax.Beliefs`. "A response is just a normal action … the same planner."

- **You stay on topic until you change it.** Off-topic quips simply aren't offered; to say them you
  first steer the conversation there. That models conversational coherence without any special
  rule.

- **It emerges on its own, too.** Once the cast has warmed up, an idle character will strike up a
  chat with a friend (bounded to one conversation per pair) — so late in a run you'll see NPCs
  talking, complimenting, and even gossiping without you.

*(Deferred, per `docs/LEDGER.md`: multi-party conversations, a richer quip library, and keeping
participants engaged in the chat rather than wandering off mid-sentence.)*

### 11. The director — a story manager that shapes the drama (v6)

There is a fourth character you never see: **`director`**, Versu's Drama Manager. It has no body
(no location — it can't be greeted, and never orders a drink) and only *metalevel* desires about
the shape of the evening. It doesn't puppet anyone; it nudges the situation and lets the autonomous
cast react.

- **Watch for the beat.** Play (or press `m`) until the room has warmed up — two characters who
  genuinely like each other. Then, on one of its turns, the narration shows
  **`director: turn ada against bex to stir up the evening`**. The director has decided the evening
  is too cosy and injected a **falling-out**: it sets one against the other (an `annoyed` mood, a
  grievance, and a sharp drop in warmth).
  → features: a DM modeled as an ordinary agent — a metalevel `Want` plus a practice of metalevel
  actions — "the DM is just a particular type of practice." code: `dmPractice` + the `director`
  character in `Prax.Worlds.Bar`.

- **The drama then plays itself out.** The director doesn't script what happens next. Its injected
  grievance flows through the *same* systems you've already seen: the wronged pair stop being
  friendly (belief/mood/warmth gates), can take offense, gossip about each other, and so on. The
  director sets the spark; the autonomous characters supply the fire.

- **It knows when to stop.** The intervention fires once (a metalevel want it can satisfy just one
  way), so the director doesn't grind the room into endless conflict — exactly the "high-level
  director who does not like to micromanage."

*(You can also take this slot yourself — see §18, `prax dm`. Still deferred, per `docs/LEDGER.md`:
richer metalevel repertoire and pacing, and a generic event stream the director could watch.)*

### 12. Character arcs — an inner life that reshapes what you want (v7)

Practices give characters *external* choices; an **arc** is a character's *internal*, high-level
state — the through-line of their evening. Everyone arrives **`hopeful`** (watch the scene:
*"bex feels hopeful"*).

- **Watch bex find its place.** As bex warms to someone over the evening, once it feels genuinely
  fond (its own warmth crosses a threshold) it takes the beat **`bex: settle in, feeling you belong
  here`** and the scene turns to *"bex feels at home here"*. Its wants shift with the stage — a
  belonging bex is content to linger.
  → features: a stage-gated `Want` — advancing the arc changes what the character pursues. code:
  `Prax.Arc`; the `arc` practice + bex's arc wants in `Prax.Worlds.Bar`.

- **Arcs are robust to the drama.** Even when the director turns ada against bex, bex still settles
  into belonging — because bex's *own* warmth toward ada held. The arc reflects the character's
  interior, not just what's done to them.

- **True transformation is the player's alone.** Every hopeful patron is *offered* the downward
  move, **`give up on the evening, resigning yourself to solitude`** (→ lonely). But no NPC ever
  takes it: sliding into loneliness only forecloses the belonging they crave, with no way back, so
  the utility planner refuses it. Only a human — who isn't bound by the planner — would ever choose
  to change against their own desires. This is Versu's "true transformation … is only available to
  the player," and here it falls straight out of the architecture (NPCs maximize utility; the
  player picks from the menu).

*(Deferred, per `docs/LEDGER.md`: richer multi-stage arcs and arcs that feed back into the
director's pacing.)*

### 13. First-order queries (v8)

Everything above uses preconditions of the form "this fact holds" / "this fact doesn't." v8 added
the missing logical connectives so a precondition or a desire can be **disjunctive** (`Or`),
**negative-existential** (`Absent` — "there is no X such that…"), or **quantified** (`Exists`,
plus `forAll`/`implies` built on them). You've already seen them at work without naming them: the
bell's `Subquery`+`Count` gathers a set, and — in Part II — every scripted ending is frozen by
`Absent [Match "ending.E"]` ("no ending exists yet"). These are the grammar the later worlds lean
on. → code: `Prax.Query`; exercised throughout `Prax.QuerySpec`.

---

# Part II — beyond the bar (the rest of the system)

The bar is deliberately cosy. The remaining capabilities each get their own world or command, so
you can see them in isolation. Each is a one- or two-line invocation.

### 14. A death, and branching endings — `prax intrigue` (v9)

```sh
cabal run prax -- intrigue
```

> You are **Marcus, the poet.** *"The others act on their own."* On the first turn **cassia
> confides the plot** — *"cassia: confide the plot against artus to marcus"* — and your menu opens
> up:
> ```
>   1) marcus: warn artus that cassia means to kill them
>   2) marcus: poison artus with your own hand
>   3) marcus: warm to cassia's charms
> ```

Four outcomes, on the *same* engine you learned in the bar:

- **Do nothing** (press `m`). Cassia poisons Artus; he **dies and leaves the cast**, and the story
  ends **`THE END — betrayal`**. A character being *removed from play* — not just marked dead — is
  the v9 capability.
- **Warn Artus** → **`loyalty`** (the plot is foiled).
- **Poison Artus yourself** → **`complicity`**.
- **Warm to Cassia's charms** forms a `lovers` **bond** — a romance thread that runs *alongside* the
  ending logic rather than being one (you can romance her and still warn, or still let it run).

Once any ending is reached, `Absent [Match "ending.E"]` **freezes** every further affordance, so the
credits don't keep rolling. → code: `Prax.Worlds.Intrigue`; cast removal in `Prax.Engine`; asserted
in `Prax.IntrigueSpec`.

### 15. QA tooling — `prax stress` (v10)

Before shipping a world you want to know: does every ending actually reach? Any dead ends? The
stress-tester plays hundreds of seeded, all-AI games and reports.

```sh
cabal run prax -- stress intrigue
```
```
  endings:   [("complicity",100),("loyalty",100)]
  coverage:  4 distinct actions fired
  dead ends: 0
  no ending: 0 / 200 runs
```

Note that `betrayal` doesn't appear: betrayal is the *do-nothing* ending, and a utility-driven AI
always acts, so a purely autonomous cast never produces it — a real, useful signal about the world.
Its companion is the **inspector** `explain`, which answers "why *can't* this character do X right
now?" by walking the failed preconditions (used from tests and the REPL). → code: `Prax.Stress`,
`Prax.Inspect`.

### 16. Save & resume (v11)

Session state *is* the fact database, so saving is trivial. In any world, press **`s`** to write a
save file; then continue later:

```sh
cabal run prax -- intrigue resume
```

The world reloads exactly where you left it. → code: `Prax.Persist`.

### 17. Scene-authored drama — `prax play` and `prax flow` (v12)

The same Intrigue drama, but written as a **screenplay** instead of hand-built practices — a `CAST`
plus a graph of `scene`s, each with `beat`s (dialogue/affordances) and `junction`s (labelled routes
that end the story or move to the next scene), all `compile`d down to ordinary practices.

```sh
cabal run prax -- flow      # print the scene graph (Mermaid)
```
```
graph TD
  _start((start)) --> confidence
  confidence["confidence"]
  confidence -->|toBanquet| banquet
  banquet["banquet"]
  banquet -->|betrayal| _end_betrayal((betrayal))
  banquet -->|loyalty| _end_loyalty((loyalty))
  banquet -->|complicity| _end_complicity((complicity))
```

```sh
cabal run prax -- play
```

> Now the drama is split across **two scenes**. In *confidence*, cassia confides; a bodiless
> **narrator** (Versu's story manager) then fires *"(story) toBanquet"* on its own and the curtain
> rises on *banquet*, where the same warn/poison/charm choices await. It's a *faithful* recasting of
> `intrigue` — same cast and endings — in fewer authored lines, plus the scene transition and
> flow-chart for free.

Play-scripts round-trip through **readable JSON** (the editable authoring format, chosen over a
bespoke grammar):

```sh
cabal run prax -- dump-play                 # print the built-in play as JSON
cabal run prax -- play examples/play.json   # load and play an edited script
```
→ code: `Prax.Script`, `Prax.Script.Json`, `Prax.Worlds.Play`.

### 18. You *as* the drama manager — `prax dm` (v13)

In the bar the director was an NPC. Here **you occupy the drama-manager slot** — you have no body
and never order a drink; your menu is *authorial nudges* over an autonomous cast (ada, bex, cai).

```sh
cabal run prax -- dm
```

> *"You are the drama manager: nudge the autonomous cast (ada, bex, cai)."*
> ```
>   1) director: stir up a rivalry between ada and bex
>   …
>   7) director: kindle warmth between ada and bex
>   …
>  13) director: cast a pall over ada's evening
> ```

Pick a nudge and the cast plays out the consequences through the ordinary social machinery you
learned in Part I. A practice-bound player is offered only its practice's affordances — the
metalevel `direct` moves — nothing else. → code: `Prax.Worlds.Bar` `barDirectorWorld`;
`candidateActions` in `Prax.Engine`.

### 19. Emergent derivation — `prax feud` (v15)

This is the sandbox that points past authored interactive fiction toward emergent social sim. You
author **one** wrong and **three** rules; a whole feud derives itself.

```sh
cabal run prax -- feud
```

> *"You are Alice. One wrong, and a feud assembles itself — make amends to dissolve it."*
> ```
>   - bob resents alice
>   - carol resents alice
>   - dave resents alice
> ```

The setup asserts only `wronged.alice.bob`, `allied.bob.carol`, `allied.carol.dave`. Three
forward-chaining rules do the rest:

```
allied.X.Y                     ⇒ allied.Y.X       -- alliances are mutual
wronged.X.Y                    ⇒ resents.Y.X      -- the wronged resent the wrongdoer
resents.A.B ∧ allied.A.C       ⇒ resents.C.B      -- the enemy of my ally is my enemy
```

So **carol and dave come to resent Alice though she only ever wronged Bob**, and they act on it
(*"bob is shunning alice"*, and so on). The derivation is **defeasible**: derived facts live in a
closed *view*, recomputed from the base and never stored. Choose **`make amends with bob`** — which
deletes the single authored `wronged.alice.bob` — and watch the *entire* feud vanish in one move:
every derived `resents` and every shunning disappears, because their only support is gone.
→ code: `Prax.EL` (the exclusion-logic lattice + `m(X)`), `Prax.Derive` (forward chaining),
`Prax.Worlds.Feud`.

### 20. Static type-checking — `prax check` (v16–17)

Versu had an implicit type system; `prax check` makes it explicit and runs it over a world without
executing it.

```sh
cabal run prax -- check feud
```
```
well-formed: the feud (emergent sandbox)
```

It flags **unbound variables** (an effect/axiom-head variable no precondition can bind — a silent
no-op), **exclusion-cardinality clashes** (a relation asserted both `!` and `.`), and **dangling
references** (a `Call`/spawn of something undefined). On top of that it does **ML-style sort
inference**: declare each base sort's members and it infers every position's sort by unification and
rejects conflicts (e.g. a place where an agent belongs). Every shipped world checks clean. → code:
`Prax.TypeCheck`; asserted in `Prax.TypeCheckSpec`.

### 21. The Prompter compilation features — `prax audience` (v18)

A short scene that exercises the three authoring constructs the scene layer compiles beyond plain
beats and junctions — **memories**, **timed junctions**, and **character sketches** — all at once.

```sh
cabal run prax -- audience
```

> *"You are the envoy. Flatter the king, then petition — before the moment (or the Duke) passes."*

- **A memory.** The instant you're before the throne, a line of exposition fires once:
  *"(You recall the last envoy who displeased the king — exiled by dawn.)"* A **memory** is a
  one-shot narration fired the first time its trigger holds.
- **A character sketch.** The **Duke** has no hand-written desires — only a *concern* for standing
  (`concernedWith [("favor", …)]`) and a trait (`ambitious`). That concern compiles to a want, so he
  **courts the king unbidden** (*"duke: flatter the king"*) exactly once, then rests.
- **A timed junction.** If you **dawdle** (press `m`), the audience runs out of patience and ends
  **`dismissed`** once the scene clock passes its bound. **Flatter, then present your petition** in
  time and it ends **`granted`**. Time passes via a passive scene clock, added only because this
  script uses a timed junction.

→ code: `Prax.Script` (compilation), `Prax.Worlds.Audience`; asserted in `Prax.ScriptSpec`.

---

## Feature coverage map

Everything implemented, where it lives, and how to see it. The first block is the engine core (the
bar, Part I); the second is Part II, one row per world/tool.

| Feature | Code | Seen when you… |
|---|---|---|
| Trie DB, `.` descent | `Prax.Db` | (everywhere — all state) |
| Exclusion `!` (corrected) | `Prax.Db.insert` | move (`at!Place` replaces old), order/serve |
| Unify / pattern match | `Prax.Db.unify` | every action precondition |
| `Match` | `Prax.Query` | any action being available |
| `Not` (negation as failure) | `Prax.Query` | greet vanishing after greeting |
| `Eq` (also assignment) | `Prax.Query` | Fulfill (`Eq Actor Bartender`) |
| `Neq` | `Prax.Query` | greet (can't greet yourself) |
| `Cmp` (`lt/lte/gt/gte`) | `Prax.Query` | tipsy threshold; the bell |
| `Calc` (`add/sub/mul`) | `Prax.Query` | drink counter incrementing |
| `Count` | `Prax.Query` | the bell (counting customers) |
| `Subquery` | `Prax.Query` | the bell (gathering the crowd) |
| `Insert` / `Delete` | `Prax.Engine` | ordering, serving, moving |
| `Call` + functions + `FnCase` | `Prax.Engine` | getting tipsy (`recordDrink`→`checkTipsy`) |
| Practice spawning + `init` | `Prax.Engine` | patron drink counters seeded at start |
| Single- & multi-role practices | `Prax.Types`/`Bar` | `world` (1 role), `tendBar` (2 roles) |
| `dataFacts` | `Prax.Engine` | the four beverage choices |
| Wants / utility / lookahead | `Prax.Planner` | bex walking in to order; ada serving |
| Round-robin loop + CLI menu | `Prax.Loop` / `app/Main` | the whole session |
| Emotions (mood, target/cause, prior) | `Prax.Core` `setMood` | "feels annoyed toward you" after a snub |
| Relationship evaluation (numeric, asymmetric) | `Prax.Core` `adjustScore` | "warmth toward …" climbing/cooling |
| Relationship-gated affordance | `Prax.Core` `scoreAtLeast` | "Buy … a drink" appearing once warm |
| Reactions (spawned practices + response chains) | `Prax.Reactions` | greet → "Greet back"/"Rebuff"; take-offense |
| Norms (violation-marking + disapproval) | `Prax.Reactions` | stiff the tab → "broke a norm" → ada disapproves |
| Norm avoidance in the planner | `Prax.Planner` + a `Want` | NPCs tip rather than stiff |
| Beliefs (per-agent, can be false) | `Prax.Beliefs` | a rumour → "… believes ada resents them" |
| Belief-gated behaviour / revision | `Prax.Beliefs` | a false belief suppresses friendliness; evidence dispels it |
| Conversation (speaker turns, topics, quips) | `Prax.Conversation` | "… are chatting (rapport)"; compliment / gossip quips |
| Story manager (DM) as a metalevel agent | `dmPractice` / `director` | "director: turn ada against bex to stir up the evening" |
| Character arcs (stage-gated wants) | `Prax.Arc` | "bex feels hopeful" → "at home here"; belonging beat |
| Player-only transformation (against desires) | `arc` practice | "give up …" offered but never taken by an NPC |
| First-order connectives (`Or`/`Absent`/`Exists`, `forAll`/`implies`) | `Prax.Query` | endings frozen by `Absent [ending.E]` |
| Deontic obligation (□), discharge/breach, contrary-to-duty (□□) | `Prax.Deontic` | bar: "owes ada a tip" → tip discharges / stiff breaches |
| Cast removal (a character dies and leaves) | `Prax.Engine` / `Intrigue` | `prax intrigue`: do nothing → Artus dies (betrayal) |
| Branching dramatic episode | `Prax.Worlds.Intrigue` | `prax intrigue`: warn/poison/romance → distinct endings |
| Stress-tester + inspector (`explain`) | `Prax.Stress` / `Prax.Inspect` | `prax stress intrigue` (endings, coverage, dead ends) |
| Save / resume (state = the fact DB) | `Prax.Persist` | press `s`; then `prax intrigue resume` |
| Scene-authoring layer (CAST + scenes/beats/junctions) | `Prax.Script` / `Worlds.Play` | `prax play`; `prax flow` prints the scene graph |
| Play-scripts round-trip through JSON | `Prax.Script.Json` | `prax dump-play`; `prax play examples/play.json` |
| Player as drama manager | `Bar.barDirectorWorld` / `candidateActions` | `prax dm`: your menu is authorial nudges |
| Forward-chaining derivation (defeasible) | `Prax.EL` / `Prax.Derive` | `prax feud`: 1 wrong + 3 rules → a feud; amends dissolves it |
| Static type checker + sort inference | `Prax.TypeCheck` | `prax check <world>` |
| Memories, timed junctions, character sketches | `Prax.Script` / `Worlds.Audience` | `prax audience` |

If the tables and scene lines don't convince you a feature is really doing what's claimed, the
same behaviours are asserted in the test suite (`cabal test`, 178 tests). Part I: `Prax.QuerySpec`,
`Prax.EngineSpec`, `Prax.PlannerSpec`, `Prax.CoreSpec` (emotions/relationships), `Prax.ReactionsSpec`
(reactions, norms, norm-avoidance), `Prax.BeliefsSpec` (per-agent & false beliefs), `Prax.ConversationSpec`
(speaker turns, topics, one-shot quips), `Prax.ArcSpec` (arc stages), `Prax.DeonticSpec` (□, discharge,
breach, contrary-to-duty), `Prax.BarSpec`, and `Prax.LoopSpec` (a deterministic 20-turn replay). Part
II: `Prax.IntrigueSpec` (death + branching endings), `Prax.StressSpec`, `Prax.PersistSpec` (save/resume),
`Prax.ScriptSpec` + `Prax.Script.JsonSpec` (scene layer + JSON, incl. memories/timed junctions/sketches
and the `audience`), `Prax.DirectorSpec` (player-as-DM), `Prax.ELSpec` + `Prax.DeriveSpec` (the
exclusion-logic lattice and forward chaining), and `Prax.TypeCheckSpec`.

---

## Things to try to build intuition

- **Never serve yourself:** order a beer, then keep pressing `m`. ada serves you because *she*
  wants no outstanding orders — you never asked.
- **Non-alcoholic vs alcoholic:** drink sodas all night; you never get tipsy (the guarded
  function case). Switch to beer/cider and the counter climbs.
- **Leave mid-order:** order a beer, then `Go to entrance` before ada serves you. She can't
  fulfill (her `Fulfill` requires you both at the same place) until you return.
- **Watch bex's arc end:** once bex has its beer, it stops chasing and just waits at the bar —
  its top want (`beer in hand`) is satisfied, so no action beats waiting.

## What is *not* yet modeled

As far as can be assessed from the available sources, the features of **Versu, Praxis, and Prompter**
are now reproduced — the sections above are the tour. What remains (see `docs/LEDGER.md`) is short
and honest:

- **The readable text playtext surface** of Prompter is *deliberately* not reproduced: editable
  scripts round-trip through JSON (§17) instead of a bespoke `.prompter` grammar. This is a chosen
  substitution, not a gap.
- **Free-text player input (#42, "Play What I Mean").** Typing a sentence and matching it to an
  affordance via an embedding model — a *beyond-Versu* capability that requires an external model,
  so it's a dependency to add, not a paper to work through.
- **Beyond-Versu extensions** noted for later: incremental view-maintenance for the derivation
  layer, hard priority tiers (Swaygent/Ensemble-style `forbidden`/`required` above utility) for
  *categorical* norm enforcement, quantified/nested beliefs (which Versu itself couldn't represent),
  and runtime want injection. These extend past what Versu did; the LEDGER's "Future ideas" section
  tracks them.

The larger arc, per the LEDGER, is to grow this from a faithful reproduction into an emergent
social-sim substrate that can be embedded in other games (sandboxes, roguelikes) — the `feud`
sandbox (§19) is the first step in that direction.
