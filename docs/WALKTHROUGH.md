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
(`dm`), emergent derivation (`feud`), the static checker (`check`), the Prompter compilation
features (`audience`), and the sandbox where who sees, who's told, and what that settles
into decides how people treat you — including a villain who manages what's known, both his own
secret and someone else's fabricated one (`village` — witnessing, gossip, reputation, and
deception).

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
produce, and (looking two moves ahead through its own future choices) sees that going to the bar
enables ordering the beer it wants. ada, seeing an outstanding order (which she dislikes),
fulfills it. This is the heart of Versu's "strong autonomy": practices only *offer* actions; the
agents choose.
→ code: `Prax.Planner` (wants, `scoreActions`/`pickAction` lookahead — the old `worldValue` this
guide used to name here no longer exists; see the note below), `Prax.Loop` (turn taking).

> **How lookahead treats *other* people (v23).** What you just watched is a special case: the
> bar's cast has no authored believed-desire vocabulary (`Prax.Minds`), so bex's lookahead is
> entirely about its own future moves, exactly as described above. In general the planner's
> lookahead also imagines one round of *other* characters' moves — but only a character the actor
> currently holds a *belief* about wanting something (`Prax.Minds`), and only if the actor
> currently believes that character is around to act (co-present now, or sighted recently enough
> — `Prax.Sight`, an authored *prediction scope*). A mind nobody has told you about, or a person
> whose whereabouts you don't know, is imagined standing still — never as conveniently helping
> your plan along. `prax village` (§22–24 below) is where this actually bites; the bar never
> exercises it.

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

- **NPCs respect norms on their own.** bex is given a strong aversion to stiffing
  (`Want [violationOf "bex" "stiffedTheBartender"] (-40)`) plus a small liking for tipping, so
  when served it **tips** rather than walking out. This isn't a foreseen future: "Leave ada's tab
  unpaid" inserts the violation fact itself, so bex's own −40 already condemns it in the immediate,
  no-lookahead evaluation (`scoreActions 0`) — confirmed live: tipping scores 13.0 against
  stiffing's −30.0 with lookahead depth 0, before the planner has looked ahead at all. bex never
  needs to *predict* ada's disapproval; it has a strong opinion of its own about breaking norms.
  That's the paper's "strong desire to respect norms" falling out of ordinary utility evaluation,
  no special rule engine, and (as of v23) no prediction of anyone else's reaction required.
  → features: norm avoidance via a large negative self-`Want`, scored at depth 0 — `Prax.Planner`
  `evaluate`/`scoreActions`.

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

### 22. Witnessing — who knows what (`prax village`) (v19)

Everything so far has one shared fact database everyone reads. `Prax.Witness` breaks that: what a
character comes to *believe* now depends on where they were standing when something happened.

```sh
cabal run prax -- village
```

> *"You are a villager. What you see — and what you miss — decides what you can do."*

bob, carol, and you start in the square; dana and eve are off at the mill. dana stays there all
game — she has her own +1 want to be at the mill (the same anchoring idiom that keeps bob loitering
near the stall: an idle character needs a place it wants to be, or it drifts on a tie-break).

**A note before you press anything.** In an earlier round of this world (v19–21), the honest
demonstration here was to wait a beat and watch bob, who wants the loaf, take it. As of v22, that
no longer happens: bob now conceals his theft (§25 covers the mechanism), and the square, with
carol and you standing in it, is exactly the audience he's concealing it *from*. He will simply
wait, for as many beats as you care to press. The cleanest way to see witnessing fire today is to
steal the loaf yourself:

```
-------------------- scene --------------------
  - bob is at the square
  - carol is at the square
  - dana is at the mill
  - eve is at the mill
  - you is at the square
Your move (you):
  1) you: steal the loaf from the stall
  2) you: whisper to carol that bob stole the loaf
  3) you: whisper to bob that carol stole the loaf
  4) you: whisper to bob that dana stole the loaf
  5) you: whisper to carol that dana stole the loaf
  6) you: whisper to bob that eve stole the loaf
  7) you: whisper to carol that eve stole the loaf
  8) you: Go to mill
  m) wait and let others act
  s) save    q) quit
> > you: steal the loaf from the stall
  bob: Wait a moment
  carol: confront you about the theft
  dana: Wait a moment
  eve: whisper to dana that carol stole the loaf
```

(Two things already visible that weren't here at v19's landing: the `whisper to …` options, and
eve's own move — v22's deception layer, live in this same world from turn one. Both are §25's
subject, not this section's; ignore them for now. eve, at the mill the whole time, isn't a witness
to anything in the square either — the same reason dana isn't.)

carol, standing right there, confronted you the instant you took it — the same beat, because
witnessing is checked at the moment of the act, not on a later turn — and comes to *regard* you a
thief on the strength of it, a *derived-reputation* affordance this section isn't about yet (§24
covers it):

```
  - bob regards you as a thief
  - carol regards you as a thief
```

dana never gets `confront` — not on this turn, not once she's told secondhand (§23), and not even
later, once she's standing right next to you in the square (checked directly against her own
`possibleActions` at that point: it's never offered). Witnessing is fixed at the moment of the
act, not by later proximity or later belief: `carol.believes.stole.you.loaf.seen` is asserted the
instant you steal — and note who *doesn't* get a belief either: you don't. `observable`'s `ForEach`
excludes the actor from its own witness deposit (the same reason `bob.believes.stole.bob.loaf.seen`
never held in the old bob-steals demo) — you aren't a witness to your own hand, and so, as it turns
out, you can't later `tell` anyone what "you saw," because you never saw it either. dana holds no
such belief, ever — only a *heard* one, once carol reaches her (§23) — and `confront` is gated on
`saw`, not `heard`.

The theft is wrapped in `observable together "stole.Actor.loaf"`; the plain `Go to [Place]` /
`Wait a moment` actions in the same world are not — so **movement is not news**: nobody, not even
someone standing next to you the whole time, comes to believe `went.you`, because no author
declared it an event. Observability is a property the world author states about an action, not
something the engine infers from watching it execute — the same action could be authored to *look*
like something else entirely (cover stories/misdirection about one's own deeds stay a banked
future tier; §25 covers the deception the vocabulary *does* support today — concealing a real deed
and lying about one that never happened).

Underneath, this is `ForEach [Condition] [Outcome]`, the outcome-language quantifier v8 never
gave: `Insert`/`Delete` act on one sentence, `Call` dispatches to one function case, but "for every
co-present character, deposit a belief" needs to range over a whole query's worth of bindings at
once. `ForEach` takes a **snapshot** — it queries all bindings before applying any sub-outcome — so
depositing carol's belief can't change who else counts as a witness mid-fold. `Prax.Witness.observable`
is the one built-in use of it: it appends
`ForEach (copresence ++ [Neq "Witness" "Actor"]) [Insert <the belief>]` to an action's outcomes,
where `copresence` is a *world-supplied* template (the village's `together` relates two characters
sharing an `at` fact) — the engine itself has no notion of place.

→ code: `Prax.Engine` (`ForEach`, `performOutcome`), `Prax.Witness` (`observable`/`saw`),
`Prax.Worlds.Village`; asserted in `Prax.WitnessSpec`, `Prax.VillageSpec`.

### 23. Rumor — the news travels (`prax village`) (v20)

`Prax.Rumor` closes the loop §22 left open: a belief someone holds — witnessed or already
secondhand — can now be *told* to a co-present hearer.

The rest of this tour (§§23–24) goes back to following *bob*, not you: v21's reputation arc ends
in an NPC's own remorse and self-deterrence, which only an autonomous character can exhibit, so it
needs an NPC thief. As of v22, though, getting bob to actually take the loaf in free play doesn't
happen on its own anymore (§22, §25): he conceals, and carol — who used to wander off on a
tie-break before her first decision even arrived — now holds her own +1 square-anchor, added in
v22 for exactly this reason (with no theft guaranteed on turn one anymore, her first turns are
zero-utility ties, and an idle character with no anchor drifts on the tie-break, same idiom as
bob's and dana's). The two anchors together mean the square never genuinely empties in autonomous
play, so bob never bites — §25 demonstrates this directly, at length.

So what follows is **forced**: bob's theft triggered directly (`doAct`, the exact technique
`Prax.VillageSpec`'s tests use — precisely because free play can no longer reach this state), then
the same production loop the CLI itself runs (`Prax.Loop.advance`/`npcAct`) driven headlessly for
real, additional rounds. This is not `cabal run prax -- village` — it's the identical engine code,
reachable interactively via `cabal repl exe:prax`, real output captured directly from a live
session, not fabricated or reused from any report:

```
-- forced: bob steals (doAct) --
  - bob is at the square
  - carol is at the square
  - dana is at the mill
  - eve is at the mill
  - you is at the square
  - carol regards bob as a thief
  - you regards bob as a thief
```

A round is six turns now (you, bob, carol, dana, eve, and the bodiless sight ticker — §25 notes
where the ticker came from). carol confronted bob the instant she witnessed it, same mechanism as
§22. eve, independently and for her own reasons (§25 — not this story), whispers a lie about
*carol* to dana in the very same round; the two plots run concurrently in one world, unprompted by
each other:

```
  you: (you wait)
  bob: Wait a moment
  carol: confront bob about the theft
  dana: Wait a moment
  eve: whisper to dana that carol stole the loaf
```

Round 2: carol shuns bob outright, still in the square — and, a side effect of eve's whisper
reaching her the round before, dana shuns carol too, for something carol never did:

```
  you: (you wait)
  bob: Wait a moment
  carol: shun bob
  dana: shun carol
  eve: Go to square
```

Round 3: carol sets off looking for a hearer — she has a want that others hear the truth about
bob's theft *from her* (`Want [carol.believes.stole.bob.loaf, Other.believes.stole.bob.loaf.heard.
carol] 5`) — and the nearest one, eve, has just walked into the square:

```
  you: (you wait)
  bob: Wait a moment
  carol: tell eve that bob stole the loaf
  dana: Go to square
  eve: whisper to bob that carol stole the loaf
```

Round 5 (30 steps in), carol reaches dana specifically and tells her too — a second, independent
source, since `Other` in her want is satisfied by any hearer, not one in particular — and dana,
hearsay in hand, acts on it the same round:

```
  you: (you wait)
  bob: Wait a moment
  carol: tell dana that bob stole the loaf
  dana: eye bob with suspicion
  eve: Go to mill
```

confirmed directly: `dana.believes.stole.bob.loaf.heard.carol` holds true from here on. `gossip`
only requires teller and hearer to be co-present — the mechanism itself doesn't route anyone toward
anyone — but carol's own want reliably puts her wherever an uninformed hearer is, so *she* does the
travelling, exactly as in the pre-v22 village.

**The saw/heard affordance asymmetry.** dana, hearsay-only, gets `eye [Thief] with suspicion` — a
milder trust hit (−5, reason `heardOfTheft`) — but never `confront`, which stays gated on `saw`
(−10, reason `sawTheft`): hearsay doesn't license "I saw you." The asymmetry cuts the other way too:
`VillageSpec`'s "hearsay licenses suspicion, not confrontation" confirms carol, the eyewitness, is
never offered mere suspicion — `eye` is gated on `heard "Actor" "<event>"` **and**
`Absent [Match "Actor.believes.<event>.seen"]`, so seeing subsumes hearing for the milder act.

**Sourced-hearsay vocabulary.** Provenance is no longer the single exclusive value v19 shipped
(`!seen`) — it's multi-valued: `<W>.believes.<event>.seen` for direct witness, one
`<W>.believes.<event>.heard.<source>` edge per teller, coexisting under the same `believes.<event>`
node. A witness who is *also* told keeps their `.seen` edge rather than losing it to an overwriting
`!heard` — the capture bug the v19 review flagged and banked for this round. Each further teller
adds another `.heard.<source>` edge, so corroboration is just counting distinct sources; `heard`
itself is a boolean `Exists` over that subtree, so two sources still yield one row in the menu, not
a duplicate tell.

**The distrust gate.** `gossip`'s world-supplied gate (worlds add their own extra conditions, the
way `Prax.Witness`'s `together` co-presence template is world-supplied) is "you don't gossip with
someone you distrust":

```haskell
Absent [ Match "Actor.relationship.Hearer.trust.score!TrustScore", Cmp Lt "TrustScore" "0" ]
```

The spec sketch named this variable `V`; the shipped code calls it `TrustScore`, because the
village practice's own role is *also* named `V` (`roles = ["V"]` in `Prax.Worlds.Village`) — reusing
`V` here would have silently captured that binding instead of introducing a fresh condition
variable, a real bug caught before it shipped rather than a stylistic choice. With no trust score
recorded, gossip flows freely; once trust drops below zero the tell disappears from the menu, same
as any other gated affordance.

→ code: `Prax.Rumor` (`gossip`/`heard`), `Prax.Worlds.Village`; asserted in `Prax.RumorSpec`,
`Prax.VillageSpec`.

### 24. Reputation — standing, notoriety, atonement, and a thief who learns (`prax village`) (v21)

`Prax.Repute` closes the loop §§22–23 opened: evidence that reached someone — witnessed or only
heard — now *settles into what they think of you*, a derived standing that shapes behaviour
without anyone storing a reputation fact. Continuing straight on from §23's forced session (same
`cabal repl exe:prax` technique, same running world — bob's theft was forced there because free
play no longer reaches this state; see §22/§25):

The instant carol (an eyewitness) believed the theft, she also came to *regard* bob a thief — a
fact nobody wrote, derived from her belief — and standing already had teeth: `shun bob` was
available to carol on the very same gate (`regardedAs "Actor" "T" "thief"`) that would offer the
player `shun bob` too, that same beat (round 1, already shown in §23).

By round 3, carol has told eve, who — hearsay-only — comes to regard bob too: §23's saw/heard
asymmetry (suspicion, not confrontation) carries straight into standing, since hearsay is evidence
enough to *regard* exactly as it's evidence enough to *believe*. With carol (witness), you
(witness), and eve (hearsay from carol) all regarding bob a thief, the third regard tips
`notoriety "thief" 3` — "the whole village knows" — and bob's want against `notorious.bob.thief`
(−15) now outweighs the loaf in his hands (+10). One round later, he returns it, the same round
carol relents (dana never regards *bob* at all in this run — eve's lie reached her first, so her
only regard is the wrongful one, §25):

```
  you: (you wait)
  bob: return the loaf with apologies
  carol: relent toward bob
  dana: eye carol with suspicion
  eve: whisper to you that carol stole the loaf
```

(dana and eve's lines here are entirely eve's frame-up of carol, §25 — the two stories are
threaded through the same six-turn rounds, and both are real, unstaged output from the one run.)

**Atonement, not amnesia.** Every line derived from bob's own theft vanishes at once — not because
anyone forgot, but because their only support (the absence of `atoned.bob`) is gone. Checked
directly against the same session: `atoned.bob` holds; `regards.carol.bob.thief` and
`notorious.bob.thief` are both false in the closed view; and — the point of the exercise —
`carol.believes.stole.bob.loaf.seen` is still **true**, exactly as before. Nobody's belief moved;
`standingUnless`'s defeater dissolved the *derivations*, on the same read, while the belief that
supports them (when the defeater is later revoked) sits untouched. This is `VillageSpec`'s
"atonement dissolves standing while memory persists," and it's not something a plain scene render
shows directly (the scene prints derived standing, not raw beliefs) — which is exactly why this
walkthrough checks the underlying facts instead of just the narration, here and below.

**Deterrence: the stall stays stocked.** Driving the same forced session onward past round 6
(round 6 confirms every shun toward bob has relented) and out to round 15 (90 steps — the same
budget `VillageSpec`'s "an atoned thief is deterred" test drives), the picture doesn't change:
bob's `steal the loaf from the stall` is still on his own menu, unrefused by any gate —

```
bob's steal-the-loaf option still on the menu, unrefused: True
stall.loaf present: True
bob holds no loaf: True
```

— and he simply never takes it, for the whole remaining run. `stall.loaf` really is still there,
and bob's `Want [Match "holding.bob.loaf"] 10` is live again (atonement deleted his *holding*, not
his want). The deed's own outcome deletes `atoned.Actor` as part of the very same outcome that
reinstates his holding, so standing and notoriety are already snapped back — the same −15 he'd
only just paid off — in the state stealing itself produces, with *no lookahead at all*: re-checked
live against the current planner (v22 concealment and the v23 round-walk both landed since this
number was first measured), `scoreActions` at depth 0 still ranks `bob: steal the loaf from the
stall` at exactly **−4.0** against `bob: Wait a moment`'s **+1.0** — unchanged, because concealment
is a *reward-if-unseen* want (it simply fails to add its +12 once the square is watched, which it
always is here; it isn't an extra penalty on top of notoriety's −15). This isn't a foreseen future;
it's an immediate consequence of the action, exactly like §8's stiffing aversion. An unatoned bob
was tipped into atoning; an atoned bob, seeing that consequence right in front of him, doesn't
re-offend. `VillageSpec` pins both facts — "re-offense revokes atonement: standing snaps back from
memory" forces a second theft by hand and asserts the regards return, and "an atoned thief is
deterred: the planner sees the snap-back" drives 90 autonomous turns past the atonement and asserts
the stall stays stocked (the full arc itself is driven for 60 turns in "the whole arc runs
itself").

The shun/relent/tell options never resurface either — with no live regard, the wants that drove
them (carol and dana's shun-want is *conditioned* on `regardedAs`, per the design spec, so it
evaporates with the regard rather than fighting a stale shun to a tie-break) simply have nothing
left to pursue. The full mechanism — `standing`, `standingUnless`, `regardedAs`, `notoriety`, and
the world's choice to key bob's shame on notoriety rather than any one regarder's contempt — is
documented in `docs/specs/2026-07-10-v21-repute-design.md`.

→ code: `Prax.Repute` (`standing`/`standingUnless`/`regardedAs`/`notoriety`), `Prax.Worlds.Village`
(`shun`, `return the loaf with apologies`, `relent`, `villageAxioms`); asserted in
`Prax.ReputeSpec`, `Prax.VillageSpec`.

### 25. Secrets & deception — a villain, and an honest injustice (`prax village`) (v22)

`Prax.Deceit` adds the adversarial layer §§22–24's information stack (witnessing → rumor →
reputation) was always going to need: agents who *manage* what is known, rather than just
carrying it. Two mechanisms, both authored as ordinary wants and an ordinary action — no stealth
system, no lie-detection engine, nothing new in `Prax.Engine`:

- **`conceal`** is a want that nobody believe some deed (`Absent [Anyone.believes.<event>]`). It
  needs no enforcement of its own — the planner's lookahead already simulates the v19 witness
  deposits before choosing an action, so an agent who values the secret simply never scores the
  witnessed version of the theft as highly as the unwitnessed one. Waiting for privacy falls out
  of ordinary utility maximization.
- **`lie`** mirrors v20's `gossip`, inverted twice: the speaker must hold **no** evidence of the
  event (that absence is what makes it a lie, and it's also the action's own undoing — the instant
  the liar hears their own lie told back to them, they acquire evidence, the `lie` action's gate
  closes, and plain `gossip` takes its place, seamlessly), and the fabricated subject is bound from
  a world-supplied *fabrication* condition (whom you could plausibly frame) rather than from a
  belief. The effect it inserts — `<Hearer>.believes.<event>.heard.<Actor>` — is *identical* to
  `gossip`'s. That identity is the whole design: the deceived hold real hearsay, structurally
  indistinguishable from the genuine article, and the entire v20/v21 machinery (retelling,
  corroboration, standing, notoriety, shunning) runs on the falsehood unmodified.

**bob conceals.** `conceal "stole.bob.loaf" 12` — worth more than the loaf itself (+10). §22
already showed the consequence: press `m` as many times as you like with anyone still minding the
square, and bob just waits.

```sh
cabal run prax -- village
```

Ten presses, player never leaving the square:

```
-------------------- scene --------------------
  - bob is at the square
  - carol is at the square
  - dana is at the mill
  - eve is at the mill
  - you is at the square
Your move (you):
  1) you: steal the loaf from the stall
  2) you: whisper to carol that bob stole the loaf
  3) you: whisper to bob that carol stole the loaf
  4) you: whisper to bob that dana stole the loaf
  5) you: whisper to carol that dana stole the loaf
  6) you: whisper to bob that eve stole the loaf
  7) you: whisper to carol that eve stole the loaf
  8) you: Go to mill
  m) wait and let others act
  s) save    q) quit
>   bob: Wait a moment
```

`bob: Wait a moment` repeats on every one of the ten presses — the stall stays stocked the whole
capture. **Walking away yourself is not enough**, either. Leave for the mill on your very first
move and keep pressing `m`:

```
-------------------- scene --------------------
  - bob is at the square
  - carol is at the square
  - dana is at the mill
  - eve is at the mill
  - you is at the square
Your move (you):
  1) you: steal the loaf from the stall
  2) you: whisper to carol that bob stole the loaf
  3) you: whisper to bob that carol stole the loaf
  4) you: whisper to bob that dana stole the loaf
  5) you: whisper to carol that dana stole the loaf
  6) you: whisper to bob that eve stole the loaf
  7) you: whisper to carol that eve stole the loaf
  8) you: Go to mill
  m) wait and let others act
  s) save    q) quit
> > you: Go to mill
  bob: Wait a moment
```

— and across every one of seven further presses, bob still just waits. The reason is v22's other
landed change (§23): carol now holds her own +1 square-anchor, so she never leaves either, and bob
is still watched. §2 of the design spec put it plainly once this was discovered live: *bob waits as
long as **anyone** minds the square — the bread is safe exactly as long as someone is watching.*
The "walk to the mill and it empties" beat from earlier rounds' expectations is **dead in free
play**; the only way to see the actual perfect crime — the square genuinely empty, bob taking the
loaf, and nobody ever forming a belief about it — is the scripted route `VillageSpec`'s tests use
(`carol` moved out by hand, same as `you`), which is exactly what `Prax.DeceitSpec`'s minimal
fixture and `VillageSpec`'s "the perfect crime: alone, bob steals and no one ever knows" case both
demonstrate and pass. The stall surviving your absence isn't a bug in the demo — it's the mechanic
working exactly as designed: concealment isn't about the player, it's about *anyone watching*.

**eve joins, and frames carol.** eve starts at the mill — placement matters: she must not witness
the scripted thefts v19–21's own tests force, so their two-witness arithmetic stays intact — with
one authored want, `Want [Match "regards.W.carol.thief"] 4`: she wants carol ill-regarded, per
head, and doesn't care how. `lie` gives her the means. Continuing the very first capture above (ten
`m` presses, player present throughout), her whisper campaign is visible from the first beat:

```
>   bob: Wait a moment
  carol: Wait a moment
  dana: Wait a moment
  eve: whisper to dana that carol stole the loaf
```

— and `dana regards carol as a thief` derives on the very same beat, a fact nobody wrote, from a
claim nobody witnessed. The very next round, dana acts on it:

```
>   bob: Wait a moment
  carol: Wait a moment
  dana: shun carol
  eve: Go to square
```

eve keeps moving to wherever an untold villager is and keeps whispering — to bob, then to you —
until, four presses in, the third regard tips notoriety over *carol*, exactly as it did over bob in
§24, on exactly the same machinery:

```
-------------------- scene --------------------
  - bob is at the square
  - carol is at the square
  - dana is at the square
  - eve is at the square
  - you is at the square
  - dana is shunning carol
  - bob regards carol as a thief
  - dana regards carol as a thief
  - you regards carol as a thief
  - carol is notorious as a thief
```

Nobody in this scene did anything wrong except eve. carol never went near the stall. And yet the
regard, the shunning, and the notoriety are all real, derived facts, indistinguishable — to
`Prax.Repute`, and to every villager but eve — from the ones bob earned honestly in §24. That's the
design's central claim from the spec, borne out live: *fabrication planted ordinary
`.heard.<liar>` hearsay ... the lie propagates as truth because hearsay and fabrication are
indistinguishable to everyone but the liar.*

**The injustice is honest: carol has no recourse.** eve's frame-up doesn't depend on bob at all —
it's driven here from a fresh, *unforced* `villageWorld` (exactly `VillageSpec`'s own setup:
`driveIdle "you" 40 villageWorld`), so the same cascade shown above runs on its own, without
anyone needing to force anything. Checking carol's own `possibleActions` directly at that point
(`cabal repl exe:prax`, live):

```
-- carol's menu after the frame-up cascade (40 driven turns) --
carol: steal the loaf from the stall
carol: whisper to eve that bob stole the loaf
carol: whisper to you that bob stole the loaf
carol: whisper to bob that dana stole the loaf
carol: whisper to eve that dana stole the loaf
carol: whisper to you that dana stole the loaf
carol: whisper to bob that eve stole the loaf
carol: whisper to you that eve stole the loaf
carol: whisper to bob that you stole the loaf
carol: whisper to eve that you stole the loaf
carol: Go to mill
carol: Wait a moment
```

No `return the loaf with apologies` — and there never will be. That action's own precondition is
`Match "holding.Actor.loaf"`, and carol never held one; amends requires the thing she never took.
This isn't a missing feature — the vocabulary has no notion of *ground truth* an accusation could
be checked against, so there is nothing for anyone (carol included) to point to that would clear
her name. `Prax.VillageSpec`'s "the framed have no amends: carol is offered no return" pins exactly
this. Exculpation would need an event record — something actions could be checked against — banked
for a future round (`docs/LEDGER.md`'s backlog), not faked here with a shortcut that would make the
injustice ring false. The player has the identical `whisper`/`lie` affordance eve does (visible in
every menu throughout this section) — nothing stops *you* from framing someone too, or from
clearing carol's name by simply not believing eve's lie, which changes nothing about what everyone
else now believes.

→ code: `Prax.Deceit` (`conceal`/`lie`), `Prax.Worlds.Village`; asserted in `Prax.DeceitSpec`,
`Prax.VillageSpec`.

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
| Quantified outcomes (`ForEach`) + authored witnessing | `Prax.Engine` / `Prax.Witness` | `prax village`: carol (co-present) believes bob's theft and can confront him; dana (elsewhere) doesn't |
| Gossip / sourced hearsay (`gossip`/`heard`, multi-valued `.seen`/`.heard.<source>` provenance) | `Prax.Rumor` | `prax village`: carol tells dana what she saw; hearsay licenses suspicion, not confrontation |
| `standing`/`standingUnless`/`regardedAs`/`notoriety` (derived reputation, base-fact atonement defeater) | `Prax.Repute` | `prax village`: three regards tip `notorious.bob.thief`; atonement dissolves every regard while the belief persists; re-offense revokes it and an atoned bob is deterred from a restocked stall |
| Secrets & deception (`conceal`/`lie`) | `Prax.Deceit` | `prax village`: bob conceals his theft and waits out a watched square; eve frames carol, and the lie cascades into real shunning and notoriety with no recourse for the framed |

If the tables and scene lines don't convince you a feature is really doing what's claimed, the
same behaviours are asserted in the test suite (`cabal test`, 267 tests). Part I: `Prax.QuerySpec`,
`Prax.EngineSpec`, `Prax.PlannerSpec` + `Prax.MindsSpec` (wants/utility/lookahead, now a round-walk
over believed minds — `predictMove`, `charDesires`, `professed`/`conventional`), `Prax.CoreSpec`
(emotions/relationships), `Prax.ReactionsSpec` (reactions, norms, norm-avoidance), `Prax.BeliefsSpec`
(per-agent & false beliefs), `Prax.ConversationSpec` (speaker turns, topics, one-shot quips),
`Prax.ArcSpec` (arc stages), `Prax.DeonticSpec` (□, discharge, breach, contrary-to-duty),
`Prax.BarSpec`, and `Prax.LoopSpec` (a deterministic 25-turn replay — the bar's cast now includes
the bodiless sight ticker). Part II: `Prax.IntrigueSpec` (death + branching endings, incl. the
confidant/victim `predictMove` split), `Prax.StressSpec`, `Prax.PersistSpec` (save/resume),
`Prax.ScriptSpec` + `Prax.Script.JsonSpec` (scene layer + JSON, incl. memories/timed junctions/sketches
and the `audience`), `Prax.DirectorSpec` (player-as-DM), `Prax.ELSpec` + `Prax.DeriveSpec` (the
exclusion-logic lattice and forward chaining), `Prax.TypeCheckSpec`, `Prax.WitnessSpec` +
`Prax.VillageSpec` + `Prax.RumorSpec` + `Prax.SightSpec` (`ForEach` witnessing, co-presence, the
confront affordance, sourced hearsay and the gossip gate, and the perception ticker/sightings that
gate whose moves get predicted), `Prax.ReputeSpec` (derived standing, the base-fact
atonement defeater, and notoriety at threshold — `VillageSpec`'s later cases carry the same
mechanisms through the full autonomous arc, the re-offense snap-back, and the resulting
deterrence), and `Prax.DeceitSpec` (`conceal`'s shape and its watched/unwatched planner probe,
`lie`'s no-evidence gate, self-framing and subject-is-hearer exclusions, one-shot-per-hearer, and
hearing-your-own-lie-back replacing `lie` with `gossip` — `VillageSpec`'s later cases carry the
same mechanisms into the full village: the stall staying stocked while watched, the perfect crime
once the square is genuinely empty, and eve's frame-up cascading to shunning with no recourse).

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
  *categorical* norm enforcement, and quantified/nested beliefs (which Versu itself couldn't
  represent). These extend past what Versu did; the LEDGER's "Future ideas" section tracks them.
  (Runtime want injection needs no separate mechanism — a want gated on a fact is injectable by
  inserting the fact — and `Prax.Minds`, v23, gives named desires a believable, tellable form on
  top of the plain `Want` this doc's Part I covers.)

The larger arc, per the LEDGER, is to grow this from a faithful reproduction into an emergent
social-sim substrate that can be embedded in other games (sandboxes, roguelikes) — the `feud`
sandbox (§19) is the first step in that direction.
