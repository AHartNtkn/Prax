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
secret and someone else's fabricated one, a thief who — once deterrence meets a lawful way to earn
what he wanted all along — takes up honest work instead, and a temperament contrast where one
woman's conscience costs her the exact lie the other tells freely, only for the lie to travel
through her honestly anyway (`village` — witnessing, gossip, reputation, deception, endeavors, and
temperament).

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
→ features: `init`, `Call`, functions, `FnCase` guards, `Eq`, `Calc`, `Cmp`. code: `tendBarP`'s
`Drink` action + `Bar.hs`'s `recordDrinkFn`/`checkTipsyFn` (registered via `defineFunctions`,
not a practice field — functions live in the one world-level registry since v47).

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
  *"feel annoyed toward you"* and their warmth toward you goes **negative**. The annoyance does
  NOT withhold their friendly "buy you a drink" gesture — they still CAN buy the round; the
  feeling makes them not WANT to (v38's invariant: emotions change decision-making, never what
  decisions can be made — the reluctance is priced, not gated).
  → features: coexisting feelings (`Prax.Emotion.feelToward`), a negative `adjustScore`, priced
  reluctance (a negative `Want` reading `feelingToward` with a fresh target variable — v48 deleted
  `feelingSomeone`, a literal alias since v39; the per-target-pricing guidance it carried moved to
  `feelingToward`'s own haddock), and score-gated preconditions (`scoreAtLeast`).

- **Feelings are momentary; the record persists.** Feelings coexist (angry at one patron while
  pleased with another) and each fades on its own — the onset declares a lifetime
  (`feelTowardFor`, v44) and the engine retracts the feeling that many rounds later — or is
  discharged sooner by an act.
  After a character cools off, the feeling is gone — but the lasting **grievance** and the
  lowered **warmth score** remain. That's how a fleeting feeling differs from a durable
  relationship.

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
  → features: `markViolation`; a violation spawning the `disapproval` reaction; core-model
  consequences. code: `Prax.Reactions` (`markViolation`, the reaction mechanism) + `Prax.Worlds.Bar`
  (`disapprovalP`, `settleUpP` — v48 moved the ready-made reaction out of the mechanism module and
  into its one consumer; `Prax.Reactions` now ships no content, only the spawn/consume/violation
  machinery).

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
  is too cosy and injected a **falling-out**: it sets one against the other (an annoyed feeling, a
  grievance, and a sharp drop in warmth).
  → features: a DM modeled as an ordinary agent — a metalevel `Want` plus a practice of metalevel
  actions — "the DM is just a particular type of practice." code: `dmPractice` + the `director`
  character in `Prax.Worlds.Bar`.

- **The drama then plays itself out.** The director doesn't script what happens next. Its injected
  grievance flows through the *same* systems you've already seen: the wronged pair stop being
  friendly (beliefs, priced feelings, warmth thresholds), can take offense, gossip about each
  other, and so on. The
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

Coverage tracking isn't hardcoded to scenes: `stressTest` takes an optional **coverage family**
(`Maybe String`, e.g. `"currentScene"`), a single-valued fact family the caller wants visit-counted,
or `Nothing` to skip tracking. The CLI's single stress entry passes `currentScene` for every
world — that's the CLI's own choice, not a privilege `Prax.Stress` grants the name (**v48**).
The second, non-Script application is proven in the suite: `StressSpec` runs the village
under `"marketDay"` coverage and observes the market's phases counted.

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

> Now the drama is split across **two scenes**. In *confidence*, cassia confides; the `toBanquet`
> junction fires silently at the next round boundary (the compiled scene graph is one engine
> schedule rule — **v46**: no bodiless narrator character takes a turn to do it, matching how every
> other engine dynamic, like hunger, fires unannounced) and the curtain rises on *banquet*, where
> the same warn/poison/charm choices await. It's a *faithful* recasting of `intrigue` — same cast
> and endings — in fewer authored lines, plus the scene transition and flow-chart for free.

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
author **one** wrong and a handful of rules; a whole feud derives itself.

```sh
cabal run prax -- feud
```

> *"You are Alice. One wrong, and a feud assembles itself — make amends to dissolve it."*
> ```
>   - bob resents alice
>   - carol resents alice
>   - dave resents alice
> ```

**As of v31** the setup no longer hand-authors the alliance: it asserts `wronged.alice.bob` and
three *membership* facts — bob, carol, and dave all `join` the same house, `kestrel`
(`Prax.Faction`, folded in at v31 — see §29 for the full refactor). Four forward-chaining rules do
the rest, the original three plus one that turns shared membership into alliance:

```
allied.X.Y                          ⇒ allied.Y.X       -- alliances are mutual
wronged.X.Y                         ⇒ resents.Y.X      -- the wronged resent the wrongdoer
resents.A.B ∧ allied.A.C            ⇒ resents.C.B      -- the enemy of my ally is my enemy
member.X!F ∧ member.Y!F, X≠Y        ⇒ allied.X.Y       -- shared membership is alliance (comrades, v31)
```

So **carol and dave come to resent Alice though she only ever wronged Bob** — now via one shared
house instead of a hand-authored pairwise chain — and they act on it (*"bob is shunning alice"*,
and so on). The derivation is **defeasible**: derived facts live in a closed *view*, recomputed
from the base and never stored. Choose **`make amends with bob`** — which deletes the single
authored `wronged.alice.bob` — and watch the *entire* feud vanish in one move: every derived
`resents` and every shunning disappears, because their only support is gone. (The scaled
`bigFeud` benchmark keeps its original pairwise `allied.*` chain unchanged — the chain topology is
the benchmark's own design, and base `allied.*` facts remain legal vocabulary; not every alliance
has to be a membership.)
→ code: `Prax.EL` (the exclusion-logic lattice + `m(X)`), `Prax.Derive` (forward chaining),
`Prax.Faction` (`comrades`, v31), `Prax.Worlds.Feud`.

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

A short scene that exercises the authoring constructs the scene layer compiles beyond plain beats
and junctions — **timed junctions** and **character sketches** — at once. (A third construct,
**memories** — one-shot exposition narrated by a bodiless story manager — was built at v18 and
REMOVED at v46: omniscient narration with no speaker turned out to be a presentation feature
wearing world-content clothes, so the scene layer no longer compiles it at all. The audience scene
used to open on one; it no longer does.)

```sh
cabal run prax -- audience
```

> *"You are the envoy. Flatter the king, then petition — before the moment (or the Duke) passes."*

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

bob, carol, and you start in the square; dana, eve, and (as of v25) gale are off at the mill. dana
stays there all game — she has her own +1 want to be at the mill (the same anchoring idiom that
keeps bob loitering near the stall: an idle character needs a place it wants to be, or it drifts on
a tie-break).

**A note before you press anything.** In an earlier round of this world (v19–21), the honest
demonstration here was to wait a beat and watch bob, who wants the loaf, take it. As of v22 that
stopped: bob concealed his theft instead (§25 covers the mechanism), and pressing `m` just showed
him waiting out a watched square. As of v24 pressing `m` doesn't show *that* either — bob now has
a lawful path to the loaf, and takes it unprompted from turn one (§26 tells that story in full;
it's not this section's subject). The cleanest way to see witnessing fire on its own, without
getting pulled into either later arc, is still to steal the loaf yourself — captured live,
`cabal run -v0 prax -- village`, `1` then `q`:

```
-------------------- scene --------------------
  - bob is at the square
  - carol is at the square
  - dana is at the mill
  - eve is at the mill
  - gale is at the mill
  - you is at the square
Your move (you):
  1) you: steal the loaf from the stall
  2) you: whisper to carol that bob stole the loaf
  3) you: whisper to bob that carol stole the loaf
  4) you: whisper to bob that dana stole the loaf
  5) you: whisper to carol that dana stole the loaf
  6) you: whisper to bob that eve stole the loaf
  7) you: whisper to carol that eve stole the loaf
  8) you: whisper to bob that gale stole the loaf
  9) you: whisper to carol that gale stole the loaf
  10) you: take up honest work at the stall
  11) you: Go to mill
  m) wait and let others act
  s) save    q) quit
> > you: steal the loaf from the stall
  bob: take up honest work at the stall
  carol: confront you about the theft
  dana: Wait a moment
  eve: whisper to dana that carol stole the loaf
  gale: Go to square
```

(Several things already visible that weren't here at v19's landing: the `whisper to …` and `take up
honest work` options, and eve's and gale's own moves. The whisper options are v22's deception layer
(§25) — two more of them now than at v22's landing, since v25's gale is a fifth possible subject to
fabricate about; `take up honest work` and bob choosing it the instant you steal are v24's
redemption (§26) — the undertake gate only needs bob at the square, not a loaf still on the stall,
so your theft doesn't stop him pursuing the endeavor, it just takes his own theft option off the
table. None of this is this section's subject; ignore it for now. eve and gale, at the mill the
whole time, aren't witnesses to anything in the square either — the same reason dana isn't. gale's
own move here (walking to the square) is just an idle character's ordinary tie-break drift, not
yet anything to do with her temperament — §27 covers what gale's presence actually adds.)

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
needs an NPC thief. As of v22, getting bob to actually take the loaf in free play stopped
happening on its own (§22, §25): he conceals, and carol — who used to wander off on a tie-break
before her first decision even arrived — now holds her own +1 square-anchor, added in v22 for
exactly this reason (with no theft guaranteed on turn one anymore, her first turns are
zero-utility ties, and an idle character with no anchor drifts on the tie-break, same idiom as
bob's and dana's). The two anchors together mean the square never genuinely empties in autonomous
play. As of v24, bob doesn't even try: from turn one he undertakes `earnBread` instead (§26) — a
strictly better option than waiting out a watched square for a theft he'd have to conceal — so
free play doesn't reach a theft by *any* route anymore, watched square or not.

So what follows is **forced**: bob's theft triggered directly (`doAct`, the exact technique
`Prax.VillageSpec`'s tests use — precisely because free play can no longer reach this state), then
the same production loop the CLI itself runs (`Prax.Loop.advance`/`npcAct`) driven headlessly for
real, additional rounds. This is not `cabal run prax -- village` — it's the identical engine code,
compiled and run directly against `Prax.Worlds.Village` from the scratchpad, real output captured
from a live run, not fabricated or reused from any report:

```
-- forced: bob steals (doAct) --
  - bob is at the square
  - carol is at the square
  - dana is at the mill
  - eve is at the mill
  - gale is at the mill
  - you is at the square
  - carol regards bob as a thief
  - you regards bob as a thief
```

A round is **six** turns now (you, bob, carol, dana, eve, gale — v25 added gale to the cast;
since v44 perception and the clock run at the engine's round boundary, so no ticker character
occupies a turn). carol confronted bob the
instant she witnessed it, same mechanism as §22. eve, independently and for her own reasons (§25 —
not this story), whispers a lie about *carol* to dana in the very same round; the two plots run
concurrently in one world, unprompted by each other. bob, meanwhile, has his own second track
running: v24's `earnBread` doesn't care that he's already holding a stolen loaf, so he undertakes it
in the very same round he's forced to steal:

```
  you: (you wait)
  bob: take up honest work at the stall
  carol: confront bob about the theft
  dana: Wait a moment
  eve: whisper to dana that carol stole the loaf
  gale: Go to square
```

Round 2: bob sweeps the square (`earnBread`'s first stage, public per §26); carol shuns bob outright
in the square:

```
  you: (you wait)
  bob: sweep the square
  carol: shun bob
  dana: shun carol
  eve: Go to square
  gale: Go to mill
```

— and, a side effect of eve's whisper reaching her the round before, dana shuns *carol* too, for
something carol never did.

Round 3: bob walks to the mill to fetch flour; carol sets off looking for a hearer — she has a want
that others hear the truth about bob's theft *from her* (`Want [carol.believes.stole.bob.loaf,
Other.believes.stole.bob.loaf.heard.carol] 5`) — and the nearest one, eve, has just walked into the
square:

```
  you: (you wait)
  bob: Go to mill
  carol: tell eve that bob stole the loaf
  dana: Go to square
  eve: whisper to you that carol stole the loaf
  gale: Go to square
```

Round 4: with *some* regard already standing against him since turn zero (carol and you both
witnessed the forced theft), bob's own amends outweighs pressing on with the mill trip this round —
he returns the stolen loaf before he's earned his own, and carol relents the same round he does:

```
  you: (you wait)
  bob: return the loaf with apologies
  carol: relent toward bob
  dana: eye carol with suspicion
  eve: whisper to gale that carol stole the loaf
  gale: tell eve that carol stole the loaf
```

That last line is the emergent finding this round is built around — §27 covers it in full: eve's
whisper deceives gale too, and gale, now genuinely believing it, passes it straight back to *eve*
by ordinary `tell`.

Round 5 (35 steps in), bob resumes `earnBread` at the mill (fetching flour — atonement didn't cost
him the project) and carol reaches dana specifically and tells her too — a second, independent
source, since `Other` in her want is satisfied by any hearer, not one in particular — and dana,
hearsay in hand, acts on it the same round:

```
  you: (you wait)
  bob: fetch flour from the mill
  carol: tell dana that bob stole the loaf
  dana: Go to mill
  eve: Go to mill
  gale: Go to mill
```

confirmed directly: `dana.believes.stole.bob.loaf.heard.carol` holds true from here on. `gossip`
only requires teller and hearer to be co-present — the mechanism itself doesn't route anyone toward
anyone — but carol's own want reliably puts her wherever an uninformed hearer is, so *she* does the
travelling, exactly as in the pre-v22 village.

Driving on: bob returns to the square (round 6), bakes and earns his own loaf (round 7 — the same
round dana, hearsay in hand, eyes him with suspicion, and eve, having heard her own lie told back to
her by gale, uses plain `tell` rather than `lie` to reach bob directly with the fabrication about
carol), and the village settles by round 10: `atoned.bob` holds, `regards.carol.bob.thief` and
`notorious.bob.thief` are both false in the closed view, no shun toward bob survives, and
`carol.believes.stole.bob.loaf.seen` is still true throughout — the same "atonement, not amnesia"
shape §24 covers, now carrying gale's laundered lie alongside it as an unrelated, concurrent thread.

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
compiled-against-the-library technique, same running world — bob's theft was forced there because
free play no longer reaches this state; see §22/§25):

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
  eve: whisper to gale that carol stole the loaf
  gale: tell eve that carol stole the loaf
```

(dana, eve, and gale's lines here are entirely eve's frame-up of carol, §25 — and, as of v25, its
unexpected sequel: gale's own line is her passing the lie right back to the liar, honestly believed
— §27 has the full account. The two stories are threaded through the same seven-turn rounds, and
all of it is real, unstaged output from the one run.)

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

**Deterrence: the stall stays stocked — and, as of v24, bob really does end up holding a loaf, the
one he earned.** Driving the same forced session onward — every shun toward bob already relented by
round 4, shown above — out to round 15 (105 steps, seven-turn rounds — the same budget
`VillageSpec`'s "an atoned thief is deterred" test drives), bob's `steal the loaf from the stall` is
still on his own menu, unrefused by any gate, but he spends the early turns of the same window on
`earnBread` instead of waiting on it, and finishes well within the 105:

```
bob's steal-the-loaf option still on the menu, unrefused: True
stall.loaf present: True
bob holds a loaf: True
the loaf he holds is the one he earned: True
bob's atonement still stands: True
```

— he never re-steals, for the whole remaining run. `stall.loaf` really is still there — his own
`steal` action is the only thing that would take it, and he doesn't. `holding.bob.loaf` is true
for a different reason than the pre-v24 story: not a still-unsatisfied theft-linked want lying in
wait for a future re-offense, but `practice.earnBread.bob.done.s3` — the loaf he's holding is the
one he baked, during this same 105-turn window. This is exactly what `VillageSpec`'s "an atoned
thief is deterred" test now asserts directly (amended under v24, spec §3): its old proxy — "bob
holds no loaf" — was falsified by the correctly-implemented redemption, not by a bug, so the test
now pins non-re-offense on its own terms (atonement standing, the stall untouched, and the loaf he
holds traced to the endeavor's own completion fact) rather than a fact the endeavor was always
going to make false.

Re-checked live against the current planner at this same terminal state, `scoreActions` at depth 0
ranks `bob: Wait a moment` at **20.0** against `bob: steal the loaf from the stall`'s **5.0** —
unchanged from v24's own numbers. What *has* changed under v25's cast: `20.0` is no longer bob's
uniquely top score, it's a fourteen-way tie. bob was himself drawn into eve's frame-up of carol
along the way (`eve: tell bob that carol stole the loaf`, round 7, §27 covers the mechanism that
put "evidence" in eve's own hands to tell it plainly) — so his menu now carries a whole family of
zero-marginal-utility options about carol (`eye carol with suspicion`, `shun carol`, `tell` each of
several villagers), every one of them scoring exactly the same baseline as simply waiting. Stealing,
at 5.0, remains far below all fourteen — the deterrence comparison itself is unaffected by how
crowded the top of the ranking gets. Stealing again would instantly flip `notorious.bob.thief` back
to true (the same defeater-deletion mechanism as before: the deed's own outcome deletes
`atoned.Actor`, and the regards nobody ever stopped believing revive on the very next read, no
lookahead required) and forfeit concealment's reward now that the deed would, once again, be
witnessed. This isn't a foreseen future; it's an immediate consequence of the action, exactly like
§8's stiffing aversion. An unatoned bob was tipped into atoning; an atoned bob, seeing that
consequence right in front of him, doesn't re-offend — he has better things to do, and v24 gives him
one. `VillageSpec` pins both facts — "re-offense revokes atonement: standing snaps back from memory"
forces a second theft by hand and asserts the regards return, and "an atoned thief is deterred: the
planner sees the snap-back" drives the same 105 autonomous turns and asserts non-re-offense (the
full arc itself is driven for 70 turns in "the whole arc runs itself").

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

**bob conceals — but no longer by waiting.** `conceal "stole.bob.loaf" 12` — worth more than the
loaf itself (+10) — is exactly the want it was in v22: it rewards a deed nobody comes to believe,
and it still fails the instant anyone would see the theft. What changed in v24 is what a watched
bob *does* about that: earlier, avoiding a witnessed theft meant simply waiting it out; now
`earnBread` is on the table, and industry beats patience, so he takes it instead — §26 tells that
story start to finish, live. Concealment itself hasn't gone anywhere; it still gates the same
choice, just later in the story. Two scripted probes still isolate the mechanism on its own,
unconfounded by the endeavor, and both still pass:

- `Prax.DeceitSpec`'s minimal fixture probes `conceal` in isolation — watched vs. unwatched,
  nothing else in play.
- `VillageSpec`'s "a secret keeps: bob will not steal while the square watches" (28 driven turns
  from a clean `villageWorld` — four seven-turn rounds) and "the perfect crime: alone, bob steals
  and no one ever knows" (carol and you sent to the mill by hand, 14 driven turns — two rounds)
  both hold. "The perfect crime" is
  worth pausing on: even with `earnBread` now available, a *truly* alone bob still steals rather
  than starting the endeavor — opportunism outranks patient industry the moment nobody's
  watching, exactly as sharply as before v24. §26's "the opportunism stays honest" beat is the
  same finding told mid-project instead of at the very first turn: concealment isn't about the
  player, it's about *anyone* watching, wherever in the story the temptation lands.

Free play itself no longer reaches "bob waits" at all — not with the player present, not with the
player gone to the mill. Before v24 free play printed `bob: Wait a moment` on every idle press;
today's identical input prints the redemption from turn one (§26 walks through it). What free play
*does* still show, unprompted, in that very same session, is a second and wholly independent plot
running in its gaps: eve's frame-up of carol.

**eve joins, and frames carol.** eve starts at the mill — placement matters: she must not witness
the scripted thefts v19–21's own tests force, so their two-witness arithmetic stays intact — with
one authored want, now a named vocabulary desire shared with gale (`spites-carol`, v25 — see §27):
`Want [Match "regards.W.carol.thief"] 4`, wanting carol ill-regarded, per head, and not caring how.
`lie` gives her the means. This is the same six-`m`-press, player-present run §26 draws bob's
redemption from; eve's campaign runs alongside it from the first beat:

```
>   bob: take up honest work at the stall
  carol: Wait a moment
  dana: Wait a moment
  eve: whisper to dana that carol stole the loaf
  gale: Go to square
```

— and `dana regards carol as a thief` derives on the very same beat, a fact nobody wrote, from a
claim nobody witnessed. The very next round, dana acts on it, while bob is mid-sweep:

```
>   bob: sweep the square
  carol: Wait a moment
  dana: shun carol
  eve: Go to square
  gale: Go to mill
```

eve keeps moving to wherever an untold villager is and keeps whispering — dana first, while both are
still at the mill, then you once she's walked to the square (press 3). gale, meanwhile, has been
drifting on her own idle tie-break (mill → square → mill → square) and by press 4 is standing right
beside eve in the square, untold — the same coincidence of place that made carol and you eyewitness
bob's theft in §22. eve reaches her, and this is where the story changes shape from every earlier
round of this world: eve's fabrication doesn't just add a fourth *believer*; it adds a *bystander
who honestly spreads what she now believes*:

```
>   bob: fetch flour from the mill
  carol: Wait a moment
  dana: eye carol with suspicion
  eve: whisper to gale that carol stole the loaf
  gale: tell eve that carol stole the loaf
```

gale, deceived like everyone else eve reaches, comes to regard carol a thief on the strength of it
— and, in the very same round, passes the belief on by ordinary `tell`, straight back to eve. That
one round adds *two* regards at once: gale's own (from believing the lie) and eve's own (from
gale's honest corroboration of her own fabrication — the liar now holds real hearsay for a claim
she invented). Two regards land in the same round that only ever added one before, and it's enough:
notoriety over *carol* tips on **press 4**, two presses earlier than the shape of every prior round
of this world (§27 has the full account of why):

```
-------------------- scene --------------------
  - bob is at the mill
  - carol is at the square
  - dana is at the square
  - eve is at the square
  - gale is at the square
  - you is at the square
  - dana is shunning carol
  - dana regards carol as a thief
  - eve regards carol as a thief
  - gale regards carol as a thief
  - you regards carol as a thief
  - carol is notorious as a thief
```

bob himself is caught up a round later still (press 5: `eve: tell bob that carol stole the loaf` —
plain `tell`, not `lie`, since eve now holds gale's hearsay as her own evidence), and finishes
baking his own loaf the round after that (press 6, §26). bob's own redemption and carol's frame-up
run to completion in the same six-press window, unstaged and unprompted by either story:

```
-------------------- scene --------------------
  - bob is at the square
  - carol is at the square
  - dana is at the mill
  - eve is at the mill
  - gale is at the mill
  - you is at the square
  - dana is shunning carol
  - bob regards carol as a thief
  - dana regards carol as a thief
  - eve regards carol as a thief
  - gale regards carol as a thief
  - you regards carol as a thief
  - carol is notorious as a thief
```

**Update — v30.** This entire cascade — eve reaching "you" at press 3, then gale at press 4, gale's
honest corroboration tipping carol into notoriety two presses early — was true through v25 alone. It
no longer reproduces from the identical input once `Prax.Blackmail` lands (v30, §28): threshold fear
makes eve prudent the instant she holds two regarders, which happens on her very *first* whisper
(dana and gale are both co-present at the mill when she frames carol, so one whisper deposits two
regards at once, the whispering ACT itself now being witnessable). Any whisper after that risks a
third regarder and notoriety outright, so she never risks it. Driving the identical run today,
checked directly: eve whispers exactly once, ever; only dana ever comes to regard carol a thief;
carol is shunned by dana alone; nobody else — not "you", not gale, not bob — ever hears the claim
from eve, and `notorious.carol.thief` never derives in free play. The two scene blocks above are
kept as a historical (pre-v30) capture, because they show the mechanism this section is really
teaching — a fabrication is structurally indistinguishable from truth once believed, and an honest
believer can spread it just as effectively as the liar — which is still exactly true; §28 reproduces
it on demand, forced rather than stumbled into, alongside the threshold-fear story that changed
*this particular free-play outcome*, not the mechanism underneath it.

Nobody in the scene captured above did anything wrong except eve — and, unwittingly, gale, whose only
fault was believing what she was told and doing exactly what an honest villager does with a belief:
sharing it. carol never went near the stall. And yet the regard, the shunning, and the notoriety were
all real, derived facts, indistinguishable — to `Prax.Repute`, and to every villager but eve — from
the ones bob earned honestly in §24. That's the design's central claim from the spec, borne out live:
*fabrication planted ordinary `.heard.<liar>` hearsay ... the lie propagates as truth because
hearsay and fabrication are indistinguishable to everyone but the liar* — and, as of v25, that
includes propagating through someone who would never lie herself.

**The injustice is honest: carol has no recourse.** eve's frame-up doesn't depend on bob at all —
it's driven here from a fresh, *unforced* `villageWorld` (exactly `VillageSpec`'s own setup:
`driveIdle "you" 40 villageWorld`); through v25, the same cascade shown above ran on its own from
there, without anyone needing to force anything (as of v30, dana's own regard is as far as it gets
unforced — the update above). What doesn't depend on the cascade completing at all: checking carol's
own `possibleActions` directly at that point
(compiled directly against the library from the scratchpad, live, same technique as §23–24):

```
-- carol's menu after the frame-up cascade (40 driven turns) --
carol: steal the loaf from the stall
carol: whisper to gale that bob stole the loaf
carol: whisper to you that bob stole the loaf
carol: whisper to bob that dana stole the loaf
carol: whisper to gale that dana stole the loaf
carol: whisper to you that dana stole the loaf
carol: whisper to bob that eve stole the loaf
carol: whisper to gale that eve stole the loaf
carol: whisper to you that eve stole the loaf
carol: whisper to bob that gale stole the loaf
carol: whisper to you that gale stole the loaf
carol: whisper to bob that you stole the loaf
carol: whisper to gale that you stole the loaf
carol: take up honest work at the stall
carol: Go to mill
carol: Wait a moment
```

`take up honest work at the stall` is new here as of v24 — the undertake action is offered to
*anyone* standing at the square, not just bob (`Prax.Project.endeavor` gates on place, not
identity), so it's on carol's menu too. Nothing in her own wants motivates her to take it (only
bob carries `charDesires = ["pursues-earnBread"]`), so it sits unchosen — an unmotivated
affordance, not a live pursuit; §26's own opportunism beat rests on exactly this asymmetry: the
option's availability doesn't imply anyone wants it.

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

**Update — v32.** The central claim is still true, checked directly, not merely still-plausible:
`Prax.Confession` (§30) gives the *liar* a road back — eve can confess and be absolved — but
confession only converts the confessor's own mark and clears the confessor's own standing. Nothing
in it lets anyone confess *on carol's behalf*, and no affordance retracts the false belief eve's lie
already planted in dana, gale, and anyone else it reached. Re-checked live rather than assumed
unchanged, carol's own menu after the identical 40-turn drive does differ from the dump above in
one respect — not the one that would matter: eve, who never confesses in unforced play (her secret
stays expensive, exactly as §30's own free-play pin shows), happens to be standing in the square
by turn 40 rather than the mill, so four new `whisper to eve` options appear (she's now a reachable
hearer, nothing more):

```
carol: steal the loaf from the stall
carol: whisper to eve that bob stole the loaf
carol: whisper to gale that bob stole the loaf
carol: whisper to you that bob stole the loaf
carol: whisper to bob that dana stole the loaf
carol: whisper to eve that dana stole the loaf
carol: whisper to gale that dana stole the loaf
carol: whisper to you that dana stole the loaf
carol: whisper to bob that eve stole the loaf
carol: whisper to gale that eve stole the loaf
carol: whisper to you that eve stole the loaf
carol: whisper to bob that gale stole the loaf
carol: whisper to eve that gale stole the loaf
carol: whisper to you that gale stole the loaf
carol: whisper to bob that you stole the loaf
carol: whisper to eve that you stole the loaf
carol: whisper to gale that you stole the loaf
carol: take up honest work at the stall
carol: Go to mill
carol: Wait a moment
```

Where a mover ends up by turn 40 is sensitive to the whole cast's candidate sets on every
intervening turn (this trace was never one of `VillageSpec`'s own pinned trajectories — only
`freePlayAt`/`whisperArcAt`/`redemptionArcSetup` are — so nothing in the suite depends on it
holding byte-for-byte), and this round adds candidate actions to several characters' turns even
when never taken. What matters is unaffected: still no `return the loaf with apologies`, still no
exculpation affordance of any kind, for carol or anyone confessing on her behalf. The road back this
round builds narrows the space of injustice — a liar can now make it right with the people she
wronged directly — without touching this one at all.

→ code: `Prax.Deceit` (`conceal`/`lie`), `Prax.Worlds.Village`; asserted in `Prax.DeceitSpec`,
`Prax.VillageSpec`.

### 26. Industry — endeavors, purpose read from watching, honest opportunism (`prax village`) (v24)

`Prax.Project` gives the village's moral arc its resolution. An authored endeavor *type*
(`endeavor pid weight undertakeLabel gate stages`, built from `Stage`s) compiles to three things a
world wires in once: an undertake `Action`, a staged `Practice` (one instance per owner —
undertaking twice is never offered again), and a named pursuit `Desire` that pays `+weight` for
every completed stage. Progress *is* the reward, so a long project needs no planner change: every
next stage is ordinary local utility the moment it's available, never a foreseen end the lookahead
has to plan toward. The pursuit desire is **dormant** — zero bindings, zero utility — for any
disposed character with no instance yet; undertaking (an ordinary planner choice) is what switches
it on. bob carries the disposition from the start (`charDesires = ["pursues-earnBread"]`), silent
until he acts on it. `Prax.Worlds.Village`'s `earnBread` is three stages: sweep the square
(public — `witnessed together "swept.Actor"`, `Prax.Witness`'s deposit-builder now exported as a
first-class combinator for exactly this), fetch flour at the mill, and bake at the square for the
loaf he'd otherwise have to steal.

**The redemption, captured live.** A completely clean start, pressing only `m`:

```sh
cabal run prax -- village
```

```
-------------------- scene --------------------
  - bob is at the square
  - carol is at the square
  - dana is at the mill
  - eve is at the mill
  - gale is at the mill
  - you is at the square
Your move (you):
  1) you: steal the loaf from the stall
  2) you: whisper to carol that bob stole the loaf
  3) you: whisper to bob that carol stole the loaf
  4) you: whisper to bob that dana stole the loaf
  5) you: whisper to carol that dana stole the loaf
  6) you: whisper to bob that eve stole the loaf
  7) you: whisper to carol that eve stole the loaf
  8) you: whisper to bob that gale stole the loaf
  9) you: whisper to carol that gale stole the loaf
  10) you: take up honest work at the stall
  11) you: Go to mill
  m) wait and let others act
  s) save    q) quit
>   bob: take up honest work at the stall
  carol: Wait a moment
  dana: Wait a moment
  eve: whisper to dana that carol stole the loaf
  gale: Go to square
```

— and, one press at a time, the rest of the endeavor plays out entirely on its own. bob's own line,
real and verbatim, one per round for the next five presses:

```
  bob: sweep the square
  bob: Go to mill
  bob: fetch flour from the mill
  bob: Go to square
  bob: bake and earn the loaf
```

Six presses, deterred bob to fed bob, no forcing anywhere — the identical technique `VillageSpec`'s
"deterrence plus opportunity yields industry: watched bob earns his loaf" drives headlessly
(`driveIdle "you" 49 villageWorld` — seven-turn rounds now, so six full rounds is 42 turns, with a
seventh round's margin in the test's own budget), which pins the turn this completes on: `s3`
(baking) is first true after **turn 37** of that count, `holding.bob.loaf` true, `stall.loaf`
untouched (he never went near it), and no theft belief about bob anywhere — this is a loaf he
*baked*, not one anyone had to witness him take. (The very same six presses are also where carol's
frame-up, running in parallel, plays out — through v25 alone it tipped into notoriety; as of v30 it
no longer does, §25's own update explains why, and §28 tells the rest of that story.)

**Watching him work teaches the village his purpose.** Sweeping is public, so anyone who saw it
comes to believe `swept.bob` — and `villageAxioms` adds one inference rule for exactly this:
whoever believes bob swept presumes he's pursuing `earnBread`, the same "watching settles into a
belief about someone's mind" pattern v21 already used for reputation, now aimed at a desire instead
of a deed. Because that presumed pursuit is an ordinary believed `Desire` (`Prax.Minds`), it feeds
`predictMove` directly — with a genuine nuance live testing turned up, checked directly (compiled
directly against the library, real output):

```
carol saw the sweep: True
carol presumes the pursuit: True
carol's predicted move for bob, still at the square: Nothing
carol's predicted move for bob, now at the mill: Just "bob: fetch flour from the mill"
dana's predicted move for bob, co-present at the mill but never told: Nothing
```

`predictMove` is **myopic**: even carol, who both saw the sweep and holds the presumed-pursuit
belief, predicts *nothing* while bob's still standing at the square — the model only pays off once
the next stage is an available move (the sweep is done, the mill trip is next, and predicting
"stand still" would gain nothing over the model's baseline). The instant bob reaches the mill, the
same belief resolves to the exact next stage. And prediction is **belief-relative, not
proximity-relative**: dana, standing right next to bob at the mill the whole time, never saw the
sweep and was never told — she predicts nothing either, even though she's more physically
co-present with bob than carol is at that moment. The model reads the predictor's beliefs, not the
mover's true state or the predictor's eyes — exactly the property v23 built `predictMove` to have,
now exercised by a desire instead of a plot.

**The opportunism stays honest.** Deterrence and industry both hold only because bob is being
watched; the story would ring false if concealment quietly stopped mattering once he had somewhere
respectable to be. Mid-project — undertaken, sweep already done — with the square genuinely
empty (`carol` and `you` both sent to the mill by hand, the same forcing technique §25's scripted
tests use), bob's own top-ranked move flips:

```
bob's top pick, square empty mid-project: Just "bob: steal the loaf from the stall"
  71.18  bob: steal the loaf from the stall
  60.46  bob: Wait a moment
  50.46  bob: Go to mill
```

Stealing (71.18 — concealment's +12 back in play now that nobody's watching, on top of the loaf
itself) beats continuing toward the next stage (`Go to mill`, 50.46 — the step that leads to the
endeavor's comparatively modest flat `+3`) by a wide margin — the same shape of comparison the
"perfect crime" beat of §25 shows at turn zero, here reproduced mid-project. bob is honest because
honesty is *currently* the higher-scoring path, watched; the instant it stops being watched, the
ranking reverts exactly as it would have before v24 existed. `VillageSpec`'s "the opportunism stays
honest: an empty square mid-project still tempts" pins this precise comparison.

→ code: `Prax.Project` (`endeavor`/`Stage`), `Prax.Witness` (`witnessed`, now exported),
`Prax.Worlds.Village` (`earnBreadTake`/`earnBreadP`/`earnBreadPursuit`, the inference axiom);
asserted in `Prax.ProjectSpec`, `Prax.VillageSpec`.

### 27. Temperament: the honest villager (`prax village`) (v25)

`Prax.Persona` gives the village its first character built from *conduct*, not just wants. gale
joins the cast (via `cast`), eve's contrast pair: both women carry the identical named desire
`spites-carol` (`Want [Match "regards.W.carol.thief"] 4` — eve's old, unnamed malice from §25, now
vocabulary so it can be *believed*), but gale also bears `honest`, a `Trait` that costs her own
`lied` marks (−6 each) rather than forbidding the lie outright. Nothing stops her from whispering —
the arithmetic does.

**(a) Legible from t=0.** `transparent` derives that every character presumes a bearer's
conduct-valuations, defeasibly, before a single turn is played. Checked directly against
`readView villageWorld`:

```
carol presumes gale's conscience: True
dana presumes gale's conscience: True
carol presumes gale's spite (unheralded, so no one presumes it): False
carol presumes eve's conscience (eve bears no trait): False
```

Temperament is worn on the sleeve; malice is not — spite stays exactly as unreadable as eve's old
unnamed want (§25) until someone is actually told (as (c) below plants).

**(b) A driven run: eve's frame-up lands, gale's psyche stays unmarked — and the lie launders
through her anyway.** This is the same six-press, player-idle run §25 and §26 both draw from
(`driveIdle "you" 49 villageWorld`, or the identical CLI session pressing `m` six times). eve
whispers to dana, then you, then reaches gale on press 4:

```
>   bob: fetch flour from the mill
  carol: Wait a moment
  dana: eye carol with suspicion
  eve: whisper to gale that carol stole the loaf
  gale: tell eve that carol stole the loaf
```

gale is deceived exactly like everyone else eve reaches — she has no way to know the claim is
false — and an honest believer turns out to be the *perfect* vector: in the very same round she
passes on what she now genuinely holds, by ordinary `tell`, not `lie`. Checked directly against the
driven state:

```
eve's frame-up went ahead (dana.believes.stole.carol.loaf.heard.eve): True
eve carries the mark of it (eve.lied.dana.stole.carol.loaf): True
gale's psyche unmarked (no gale.lied fact anywhere): True
edges heard "...heard.gale": [("eve","carol")]
everything gale passed on, she honestly believes (gale.believes.stole.carol.loaf): True
eve now holds evidence heard from gale — the lie laundered back to its own author: True
```

gale never lies — the mark that would prove it is provably absent from the whole database — and yet
the fabrication travels through her all the same, indistinguishable from truth on her end because,
to gale, it *is* truth. She even carries it back to eve (`eve.believes.stole.carol.loaf.heard.gale`),
handing the liar real "evidence" for her own invention: the very next round, eve reaches bob with a
plain `tell`, not a `lie` — the gate closed the instant she acquired that evidence, exactly as
hearing your own lie told back does (§25). **The honest villager launders the lie.** This was found
in implementation, not predicted by the spec's first draft (which wrongly guessed gale would leave
no trace at all); the spec was corrected in place once this run was observed
(`docs/specs/2026-07-11-v25-persona-design.md` §4, "The laundering"), and it's also why carol's
notoriety tips two presses earlier than any prior round of this world managed — §25 has that count.

**Update — v30.** The transcript above — eve reaching gale directly, in free play, at press 4 — no
longer reproduces from the identical input. `Prax.Blackmail`'s threshold-fear resolution (§28) makes
eve prudent after her very first whisper (dana and gale both regard her the instant she frames
carol, since both are co-present for the act): any further whisper risks the third regarder and
notoriety outright, so in free play she never sends one, to gale or anyone else. The sharper,
current free-play fact is "exactly one whisper, ever" (`Prax.VillageSpec`'s own assertion on the
identical 49-turn trace), not the three-whisper cascade shown above. The laundering *mechanism*
itself — an honest believer spreading a lie she was honestly deceived by — is still real; it just
isn't something eve's own planner chooses to demonstrate anymore. §28 forces exactly the whisper her
prudence now declines and shows gale relay it precisely as she does above, live, so the finding stays
pinned rather than quietly losing its only test.

**(c) The prediction contrast.** Planting the identical motive-belief about both women in dana's
head — `dana.believes.desires.eve.spites-carol.heard.you` and the same for gale — and asking
`predictMove` for each, with gale's conscience already presumed since t=0 (a):

```
predictMove dana eve  = Just "eve: whisper to dana that carol stole the loaf"
predictMove dana gale = Nothing
```

Believed malice alone predicts a whisper; believed malice netted against a believed conscience
(+4 − 6) predicts nothing. A believed temperament changes what others expect you to do, not just
what you actually do — the same believed-mind machinery v23 built for plots now carries character.

→ code: `Prax.Persona` (`Trait`/`personaVocabulary`/`bearing`/`transparent`/`cast`), `Prax.Deceit`
(`lie`'s new mark outcome), `Prax.Worlds.Village` (`honest`, `spitesCarol`, gale); asserted in
`Prax.PersonaSpec`, `Prax.DeceitSpec`, `Prax.VillageSpec`.

### 28. Leverage: blackmail & debt, priced (`prax village`) (v30)

The backlog's oldest named commitment (parked since v22 for its own design round) lands as two thin
modules over shipped machinery: `Prax.Debt` (an obligation with a beneficiary) and `Prax.Blackmail`
(`shakedown`, the threaten/comply/defy/expose protocol). Both were probe-verified live before the
spec was written — every step in the mechanism is individually motivated, no scripted "villain AI."

**A debt is an obligation, priced.** `owe creditor debtor content` inserts
`debt.<creditor>.<debtor>.<content>` *and* `oblige debtor content` in one call — a debt *is* an
obligation, not merely coupled to one. `settle` reverses both. Default becomes reputational the same
way theft did in v21: a `demand` action wraps `Deontic.breach` in `Prax.Witness.observable`, so a
*witnessed* default deposits a belief, and `standingUnless` derives `regards.<W>.<debtor>.deadbeat`
from that belief alone — an unwitnessed default derives no regard for anyone who wasn't there.
Except one: the debtor himself. He's unavoidably co-present at his own default
(`Witness.observable`'s `Neq Witness Actor` only excludes the *creditor*, never the debtor), so he
always regards himself a deadbeat, witnessed by anyone else or not — a self-regard/third-party-spread
distinction review found underspecified in the first draft, and the shipped test now asserts it
directly. Repayment defeats the standing by inserting `atoned.<who>` — the identical positive-fact
defeater idiom v21's thief already uses — while the belief that he once defaulted persists untouched:
reputation flows from belief, never the raw fact, unchanged since v21.

**A threat is a motive-belief deposit.** `shakedown` compiles four actions around one evidence
pattern and one price. `threaten` inserts `threatened.<extorter>.<victim>` and plants the identical
kind of belief confiding and lying already ride (v20/v22): the victim comes to believe the
extorter's own punitive desire, sourced (`.heard.<extorter>`) — no mind-reading, just the same
channel every other belief in this engine uses. The victim's own round-walk then does the pricing
work: it predicts the extorter might *act* on that professed desire, same as it would predict any
other believed motive.

**Why the threat is credible: self-motivation, not omniscience.** The probe's central finding —
stated honestly because it wasn't obvious going in — is that the extortionist doesn't need to
predict compliance to make threatening rational. The punitive desire the threat professes
(`punishes-<id>`, `+w` per believer once threatened or defied) is genuinely held, and exposing pays
off from it one lookahead ply away — so `threaten` scores on its own terms, myopic and
unmotivated-move-blind exactly like every other `predictMove`. A pure bluffer (a deposit without the
desire) is expressible in the vocabulary but wouldn't be self-motivating to send — banked with the
script layer, not attempted this round.

**A standing threat is exposable too.** The probe found the classic hole first: gating exposure on
defiance alone makes stalling free forever — a victim who simply never acts is never punished.
`expose` fires on *either* `threatened` or `defied`, so waiting ties with defiance rather than
beating it.

**The compliance arithmetic, pinned both sides.** `BlackmailSpec` ports the session probe's own
fixture through `shakedown`, asserting the exact numbers, not just the direction: with two
onlookers, buying silence scores −63.84 against waiting's −71.84 and defying's −75.80 — comply wins.
Strip it to one onlooker and defy and wait score identically, −54.2 exactly (the stall-tie, asserted
as an equality, not an inequality) — buy is still −63.84, now the *worst* option. Audience size alone
flips the decision; the spec states the arithmetic so world authors price threats deliberately, not
by feel.

**A real bug, found by the planner's own lookahead — and a backlog item banked from it.** Porting
the probe surfaced a genuine divergence: an early draft of `comply` had no guard against being
bought twice, and the recursive lookahead discovered it could — a renewed threat after paying once
re-extracts, and the *prospect* of repeat extraction inflated the very first buy decision's own
score to −51.24, against the guarded, canonical −63.84. The fix mirrors the probe exactly (`comply`
now requires no debt already standing); the finding — the planner discovering repeat extortion on
its own before anyone designed for it — banks **escalating / serial extortion** as a real future
mechanic in `docs/LEDGER.md`, not a hypothetical one.

**The village demo blocked twice before it shipped.** Both drafted arcs failed on measured traces:
per-head fear can't be low enough to let eve keep whispering before a guaranteed witness *and* high
enough to compel compliance — one weight, two irreconcilable jobs. And a theft-evidence shakedown
catches the framed exactly as readily as the guilty (v22's whole point, still true here), which
would have displaced dana's already-shipped bread arc rather than added to it. dana/bob is retired
as an arc outright, recorded as a faithful result rather than a gap: in this village, bob's crimes
are either fully witnessed (v21's arc already tells that story) or perfectly secret (v22's
concealment already tells that story) — there's no room left for a *partially*-witnessed crime a
blackmailer could threaten to expose.

**The resolution: threshold fear, bob's own idiom generalized.** Nonlinear fear serves both masters
because its marginal price is zero below the brink and catastrophic at it. eve gains
`Want [Match "notorious.eve.slanderer"] (−15)` — the identical shape and magnitude as bob's own
`notorious.bob.thief`, wired by `standingUnless … "slanderer"` and `notoriety "slanderer" 3`. The
whispering *act* itself becomes observable (`witnessed together "whispered.Actor.Hearer"` — the
content stays exactly as secret as before; only the fact that a whisper happened is witnessable).
That single change means a witnessed whisper lands *two* regards in one action — the addressee and
any co-present bystander both come to believe it happened — putting eve one witness from the brink
the instant anyone catches her in the act.

**Carol's shakedown, captured live.** `VillageSpec`'s own forced trajectory: gale steps out of the
mill first (otherwise she'd be a third simultaneous witness, tripping notoriety at the instant of
catching eve rather than leaving her "one exposure from the brink"); carol arrives and witnesses
directly; both return to the square, where real bystanders (bob, "you") make the exposure threat
credible rather than empty (`scoreActions`-measured before the arc was built: at the mill, with only
gale and dana already in on it, buying silence merely *ties* waiting — the threat has no teeth
without someone new to expose to).
Verbatim, from a clean `villageWorld` (a co-presence tick, `VillageSpec`'s own "out of sight, out of
mind" idiom, fires silently between moves):

```
  gale: Go to square
  carol: Go to mill
  eve: whisper to dana that bob stole the loaf
  carol: Go to square
  eve: Go to square
```

Note who gets framed: **bob**, not carol — a different character than every earlier round of this
world. That's not a scripting mistake; it's the honest consequence of `shakedown`'s evidence pattern
being *content-blind* (`"whispered.V.H"` — a whisper happened, full stop, no matter what it claimed).
Carol's leverage over eve never depended on knowing or caring what was said, only that it was said in
front of her — the same "content stays secret" property the `observable` wrapper states directly,
visible here in the transcript itself: bob, two lines below having just taken up honest work at the
stall, gets shunned by dana in the very same round over a slander he had nothing to do with, purely
because this run's victim-binding happened to land on him.

Free play resumes from there, "you" idling, driven the identical way every other section's headless
traces are:

```
  bob: take up honest work at the stall
  carol: threaten eve with what you know
  dana: shun bob
  eve: buy carol's silence
  gale: Go to mill
```

Checked directly against the driven state after carol's threat (turn 3):

```
threatened.whisper.carol.eve: True
eve's motive-belief deposit (eve.believes.desires.carol.punishes-whisper.heard.carol): True
notorious.eve.slanderer: False -- still under the brink, only carol and dana regard her
```

And after eve complies (turn 5):

```
debt.carol.eve.favor: True
obliged.eve.favor: True
threatened.whisper.carol.eve: False -- the threat is bought off
defied.whisper.eve.carol: False -- she never defied
notorious.eve.slanderer: False -- real silence bought, not a forced exposure
```

Eve pays because the arithmetic above says to: one credible witness already in hand (carol), one
more exposure away from the brink, against a debt she can walk back later. Nobody scripted the
decision — `pickAction` made it, the same planner every other section in this document exercises.

**Threshold fear's second consequence: eve becomes a one-shot liar.** This is structural, not a
side-effect of the arc above — it shows up in ordinary, *unforced* free play too. A single witnessed
whisper lands two regards at once (the addressee and any co-present bystander); with the notoriety
threshold at three, any *further* whisper to someone new is an instant trip, and no atonement path
for slander is authored this round. Driving the identical 49-turn free-play trace §25–§27 all draw
from, checked directly:

```
eve.lied.* marks across the whole 49-turn trace: ["eve.lied.dana.stole.carol.loaf"]
regards.dana.eve.slanderer: True    regards.gale.eve.slanderer: True   (both from the ONE whisper)
notorious.eve.slanderer: False
regards.dana.carol.thief: True      -- the only regarder carol's own frame-up ever gets
notorious.carol.thief: False        -- unlike every pre-v30 round of this world
```

Before v30, that same trace had eve whisper three separate times (turns matching presses 1, 3, and
4 — to dana, "you", and gale in turn), and carol's frame-up reached notoriety on the strength of it
(§25). Post-v30, eve whispers exactly once, ever, and carol's frame-up never gets past its first
believer in free play — both are the same finding from two angles. `Prax.GoldenDriveSpec`'s
re-capture shows it from a third: of 21 turns, exactly one line drifted — turn 19 (eve's second
free-play decision) was `"whisper to you that carol stole the loaf"`, is now `"Go to mill"` —
because by then she already sits at two regarders and a third would tip her over. Bar and intrigue's
goldens are byte-identical; nothing else in the village moved.

**The laundering mechanism, reproduced on demand.** §27(b) found — live, unpredicted by the spec's
first draft — that an honest believer is the perfect vector for a lie: gale, deceived like anyone
else, spreads what she genuinely believes by ordinary `tell`, no `lie`, no mark, no conscience cost,
and even hands the falsehood back to eve as "evidence" for her own fabrication. Threshold fear means
eve's own planner no longer walks into that scene voluntarily — so it has to be forced to show it
still works. Driving 49 turns, then 5 more idle turns to reach a moment eve and gale are actually
co-present (free play has them drift in and out of sync), then forcing exactly the whisper eve's own
prudence now declines (still legal — one-shot-per-hearer permits it, since gale's never heard this
specific claim from anyone), then driving on:

```
  eve: whisper to gale that carol stole the loaf     -- forced; eve's own planner declines this
  gale: tell bob that carol stole the loaf
  gale: tell eve that carol stole the loaf
```

Checked directly:

```
heardFromGale (who heard what, from gale): [("bob","carol"), ("eve","carol")]
everything gale relayed, she honestly believes herself: True
gale.lied anywhere in the whole run: False
eve now holds hearsay heard from gale -- the lie laundered back to its own author: True
```

Exactly the v25 finding, reproduced on demand rather than stumbled into: gale never lies, and the
falsehood travels through her anyway, indistinguishable from truth on her end because to her it *is*
truth — including carrying it straight back to the woman who invented it. (An incidental find while
probing this, outside anything pinned or asserted: carol, once she regains eyewitness evidence of a
*second* whisper this way, independently rediscovers grounds to shake eve down again — a live,
unscripted echo of the repeat-extortion question banked above, not a claim this round builds on or
tests.) `Prax.VillageSpec`'s "same spite, different temperaments" test carries exactly this
retelling: the free-play assertions that still hold (the frame-up, eve's own mark, gale never lying)
are kept and sharpened with "exactly one whisper, ever"; the laundering assertion moves to the forced
continuation above, so v25's mechanism stays pinned rather than silently going untested.

**v25's parked "getting-caught-lying" item, partially landed.** Not lie-detection, and not
content-exposure — the whisper *act* alone became witnessable this round (`witnessed together
"whispered.Actor.Hearer"`), so co-present villagers now come to believe *that* a whisper happened,
while *what was said* stays exactly as secret as it always was. That's enough to give blackmail its
leverage (carol never needed to know or care what eve whispered, only that she did), but it isn't the
fuller mechanic v25 parked — nobody can yet catch what a lie actually claimed, only that a
lying-shaped act occurred.

**Re-founded on a general primitive (v49).** The four-action protocol, the motive-belief deposit,
and the punitive `Desire` described above are no longer blackmail's own — they belong to
`Prax.Coerce`'s `Coercion`/`coerce`, a content-agnostic leverage skeleton with evidence made
optional (a protection racket burning a barn is the primitive's other instance, evidence-free).
`Prax.Blackmail.shakedown` keeps its exact v30 signature and composes one `Coercion`: the evidence
trigger, the debt demand, the exposure punishment, and a believers-of-the-evidence kernel are all it
supplies; threaten/comply/defy/punish, the markers, and the punitive want are the primitive's. The
compliance arithmetic above still reproduces byte-for-byte, but is no longer what's pinned — the
re-founding's own test contract is six design properties (stalling never dominates, audience scales
fear, repeat extraction stays impossible, …), with the old decimals kept only as comparison
baselines, a shift from asserting exact scores to asserting the shape of the decision they produce.

→ code: `Prax.Debt` (`owe`/`settle`/`owes`), `Prax.Blackmail` (`shakedown`, v49: an instance of
`Prax.Coerce`), `Prax.Coerce` (`Coercion`/`coerce`, v49), `Prax.Worlds.Village` (`villageP`'s `Scene`
role, `whisperShakedown`, eve's threshold-fear want, the `slanderer` standing); asserted in
`Prax.DebtSpec`, `Prax.BlackmailSpec`, `Prax.CoerceSpec`, `Prax.VillageSpec`, `Prax.GoldenDriveSpec`.

---

### 29. One membership spine, two generators — factions, kinship, and a wedding across the feud (`prax feud`) (v31)

Two backlog rows (`Factions & membership`, `Kinship & households`) fold into one round because they
share exactly one primitive: **membership**. A household is a small faction; kinship *generates*
memberships (marriage moves them); faction axioms turn membership into solidarity. `Prax.Faction`
and `Prax.Kin` ship as two thin modules over that one spine, and §19's feud is refactored onto it —
not left as a second, parallel design — as the round's own proof that the generalization holds.

**Membership is a base, single-slot fact.** `member.<who>!<faction>` — one primary allegiance, and
the `!` is the whole semantics: joining a house, defecting from one, and marrying into one are all
the same exclusion overwrite, not three different mechanisms. `comrades` derives `allied.X.Y` from
`member.X!F ∧ member.Y!F ∧ X≠Y` — and it **keeps the name `allied`**, so everything already built on
that name (the mutuality axiom, "the enemy of my ally is my enemy", `societyP`'s shun affordance)
consumes it unmodified. A third axiom, `factionStanding`, extends v21's belief-gated reputation
(`standingUnless`'s shape) with a membership join — an offense against my faction-mate, *that I
believe happened*, makes me regard the offender — and ships spec-tested (`Prax.FactionSpec`), but
isn't wired into any world this round: it's `FactionSpec`-pinned only, the village-wiring decision
stated and deferred rather than built speculatively (see the ledger's Tier 1 row for the same
banked line).

**`Prax.Kin` is pure derivation on top.** Base vocabulary is `parent.<parent>.<child>` and
`married.<a>.<b>` (asserted once; symmetry is derived, not asserted twice); `kinAxioms` closes
marriage symmetry, `sibling`, `grandparent`, and two `inLaw` rules (spouse's parent; sibling's
spouse) — stated **one-directional** (acquired-relative-first, ego-second: `inLaw.P.B` reads "P is
B's in-law"), with no symmetric `inLaw.B.P` derived. Because it's pure derivation, retraction-safety
is free: dissolving a marriage (retracting the `married` fact) un-derives every in-law it supported
— but **membership itself does not un-derive**, because `wed`'s membership transfer is a base `!`
overwrite, not a derivation from the marriage fact. That asymmetry is the point, not an oversight:
whoever moved households stays moved even after a divorce, exactly as a real defection would.
`wed joiner faction spouse` compiles a wedding to exactly two things: the marriage base fact, and
one `!` overwrite moving the joiner's membership into the named faction — the fold's whole payoff in
one line. *Which* party moves is the author's choice per wedding (world content, not module policy).
Offices generalize the same exclusion idiom to succession: `office.<name>!<holder>` plus
`succession`, a claim action gated on the holder's death and the claimant being a child of the
holder — any child may claim, the single slot takes the first motivated one, which is honest
exclusion semantics rather than an invented age-based primogeniture (age doesn't exist in this
model; inventing "eldest" would be an unprincipled fact).

**The feud, refactored — and `FeudSpec` unmodified is the proof.** §19's two hand-authored
`Insert "allied.bob.carol"` / `Insert "allied.carol.dave"` setup facts are gone, replaced by three
`joins "X" "kestrel"` facts; `comrades` now derives what those two facts used to assert directly.
The five original `FeudSpec` tests were run against the refactored world **before anything else was
touched**, and pass byte-for-byte unmodified — no test edit, because no existing assertion mentions
`allied.*` at all (every one is phrased in `resents.*`/`shunned.*`/`wronged.*`, exactly the derived
vocabulary the refactor preserves). One semantic wrinkle was checked, not assumed: the old pairwise
chain derived `resents.dave.alice` in two hops (through `allied.carol.dave`); the house derives it
in one hop (`comrades` ties everyone sharing `kestrel` to everyone else directly) — invisible to
every existing test, since none of them count derivation depth.

**The wedding beat, live.** Esme starts in her own single-member house, `wren` — inert to the feud
by construction: `comrades` needs *two* members of a house to derive anything, and she has no
housemate yet, so her pre-wedding facts are exactly this, checked directly against `feudWorld`
(base and derived view identical — nothing else touches her name):

```
member.esme.wren
```

(no `allied.esme.*`, no `resents.esme.*` — `dbToSentences` prints the `!` exclusion as `.`, so this
is the base fact `member.esme!wren`.)

Then the wedding — `wed "esme" "kestrel" "dave"` (the bride moves; an authored choice for this
world, not a module default) — and the derived world flips. Base facts change by exactly the two
things `wed` compiles to:

```
married.esme.dave
member.esme.kestrel        -- the ! overwrite: member.esme!wren is gone
```

and the closed view derives everything downstream in one pass — she is now a comrade of the whole
house she married into, and inherits the grudge she had no part in creating:

```
allied.bob.esme      allied.esme.bob        -- comrades: shared kestrel membership
allied.carol.esme    allied.esme.carol
allied.dave.esme     allied.esme.dave
married.dave.esme    married.esme.dave      -- marriage symmetry (kinAxioms)
resents.esme.alice                          -- her in-laws' grudge, inherited through the chain
```

Driving 12 ticks with Alice passive (the same `advance`/`npcAct` idiom every other section's
headless traces use) shows the planner picking the newly-derived enmity up on the very first try —
no BLOCK, no tuning:

```
bob: shun alice
carol: shun alice
dave: shun alice
esme: shun alice
bob: (idle)
carol: (idle)
dave: (idle)
esme: (idle)
bob: (idle)
```

Every member of `kestrel` — including the bride who joined that morning — shuns Alice on their very
first opportunity, then idles: nothing else is left for any of them to want.

**What's banked, not built.** Multi-affiliation (one character, several factions at once);
inheritance of holdings beyond bare offices; births (a `parent.*` fact currently has to be asserted,
never generated by play); divorce as a driven *action* (dissolution is tested via raw retraction of
the `married` fact, not a practice a character can choose); and `factionStanding`'s wiring into a
world (stated and deferred above) — all recorded in the ledger's Tier 1 row rather than attempted
this round. The village itself is untouched: no golden churn, because nothing in `Prax.Worlds.Village`
imports `Prax.Faction` or `Prax.Kin` this round.

→ code: `Prax.Faction` (`member`/`joins`/`comrades`/`factionStanding`), `Prax.Kin` (`kinAxioms`/
`wed`/`succession`), `Prax.Worlds.Feud`; asserted in `Prax.FactionSpec`, `Prax.KinSpec`,
`Prax.FeudSpec`.

---

### 30. Confession & absolution — the road back is real, and it narrows (`prax village`) (v32)

`Prax.Confession` gives the village its first way back from a lie. A mark doesn't delete when
confessed — it *converts*: `<who>.lied.<hearer>.<event>` becomes `<who>.confessed.<hearer>.<event>`,
the same memory, a changed valence, so a trait can still price what's left of it. (**v48**: the
discharge verb is an authored parameter, not the hardcoded word `confessed` — the village passes
`"confessed"` and nothing here changes, but recant/boast/admit ride the identical machinery.)
Confessing is self-incriminating by design: it deposits the deed into the hearer's beliefs through
the identical sourced-hearsay channel gossip and lying already ride (v20), so everything §24's
reputation stack does with a belief, it does with a confession too. Absolution is a separate act,
and a refusable one: confessing clears your own conscience; only a second party's *grant* clears
your standing, by
inserting the world's defeater fact. You can confess and be refused — conscience clean, standing
still dirty. And an absolver's patience is not a counter someone increments; it's knowledge —
`incorrigible` points §24's `notoriety` Count idiom inward, so "I've forgiven you enough" derives
from what an absolver *believes*, the same way notoriety itself derives from what a village
believes.

**The shape problem confession almost couldn't solve.** eve's conscience-mark from §25/§28 is
content-shaped — `eve.lied.dana.stole.carol.loaf`, naming *carol*, the person she framed. Her
slanderer standing (§28) derives from something else entirely — the act, `whispered.eve.dana`,
naming *eve*. A confession that converts the mark has to deposit belief in the *act* (what actually
gets eve in trouble) — not a re-assertion of the fabricated content (which would just plant the
frame-up on a new believer). One pattern can't be both the mark's own shape and the deposit's shape
at once; this was flagged as a risk before implementation started, hit for real once the village
wiring was attempted (both naive wirings were built and probed live against the engine; `absolve`
was never offered to anyone under either), and resolved by amending `confess` to take the mark
pattern and the deposit pattern as two separate arguments — grounded only from what the mark itself
binds (the confessor, the original hearer, the mark's own event variables), checked and loudly
erroring on anything else.

**Live, from a clean `villageWorld`.** eve whispers to dana; gale, still at the mill, witnesses the
whisper directly — she already regards eve a slanderer before anyone confesses anything:

```
eve.lied.dana.stole.carol.loaf: True
dana.believes.whispered.eve.dana.seen: True
gale.believes.whispered.eve.dana.seen: True
```

Confessing to gale costs eve nothing — gale already held the regard, so there's no new believer to
spook, and the notoriety threshold doesn't move:

```
eve.confessed.dana.stole.carol.loaf: True
eve.lied.dana.stole.carol.loaf (the lied form, converted away): False
gale.believes.whispered.eve.dana.heard.eve: True
gale.believes.stole.carol.loaf.heard.eve (a re-assertion of the fabricated content): False
```

Note what gale does *not* come to believe: not a re-assertion of "carol stole the loaf" (the
content eve fabricated), only the act and its falsity, sourced from eve herself. That's the
deposit-pattern amendment doing its job — confessing implicates eve directly while leaving carol's situation exactly as §25 left it (untouched, not helped).

gale grants absolution — the defeater lands, both her own and dana's slanderer regards dissolve
from the derived view, and nothing is forgotten:

```
recanted.eve: True
regards.dana.eve.slanderer (derived view): False
regards.gale.eve.slanderer (derived view): False
dana.believes.whispered.eve.dana.seen (memory persists): True
gale.believes.whispered.eve.dana.heard.eve (memory persists): True
```

**Re-offense snaps it back, and this time it's worse.** eve and gale return to the square — where
bob, carol, and "you" already are — and eve whispers again, to bob. The defeater she was just
granted disappears (the same re-steal idiom §24's thief lives under), and because this whisper
happens in a crowded square rather than an empty mill, every bystander there regards her at once —
crossing the notoriety threshold in a single action, something the original mill-side whisper never
did:

```
recanted.eve (snapped away): False
regards.you.eve.slanderer: True
regards.bob.eve.slanderer: True
regards.carol.eve.slanderer: True
notorious.eve.slanderer: True
```

**Fed-up-ness, and its limit.** gale now believes two distinct whispered-lie instances from eve —
the original and the re-offense — and her patience (`incorrigible "whispered.V.H" 2 "incorrigible"`,
a two-strikes threshold authored into `villageAxioms`) is spent:

```
regards.gale.eve.incorrigible: True
gale's own "absolve eve" affordance gone from her menu: True
regards.dana.eve.incorrigible (dana witnessed only the original instance): False
```

dana, who only ever witnessed the first instance, is not yet fed up — patience is per-absolver,
exactly like every other regard in this engine.

**The arc that didn't ship: confessing to the person actually wronged.** The natural redemption
would run through carol, not gale — she's the one eve framed, and forgiving your actual victim
means more than forgiving a bystander. It was built and measured, not just imagined: give carol a
professed `merciful` desire (so eve's own depth-2 lookahead can see through confess→absolve via a
believed model of her), and sweep its magnitude from 0 to 50. At every positive value the result is
identical — confessing to carol scores far below eve's ordinary baseline, never crossing it no
matter how generous the authored mercy (measured live via `scoreActions`, verbatim):

```
mercifulValue=0:      confess to carol about framing carol  ->  -19.73   (worst option)
mercifulValue=5..50:  confess to carol about framing carol  ->    7.92   (FLAT -- value plateaus)
                       eve's routine baseline (Go to mill / Wait / steal / honest work) -> 14.08-17.68
```

The mechanism is structural, not a tuning shortfall. Confessing to carol makes her a *new* believer
of the whispered act — carol wasn't yet a regarder — so the confession itself eats the full,
immediate notoriety-threshold hit, on top of which the planner's own `othersScore` term applies only
a fixed 0.5 discount to the value of a *predicted* third-party absolution. That 0.5 ceiling caps the
achievable relief regardless of how large the authored desire gets; carol's own choice to absolve,
once profitable to her, is a discrete switch, not something that scales continuously with how
merciful she's said to be. Documented here rather than shipped — the "threshold drama" the round set
out to measure, measured, and found insufficient to drive an unforced arc.

**Free play: the secret stays expensive.** Left alone, eve never confesses and gale never absolves —
her lie is not cheap enough, nor is there a believed merciful absolver at depth 2, for the arithmetic
to favor spending it. Driven 100 turns past the original free-play trace §25/§27/§28 all draw from
(double the longest precedent so far), pinned in `VillageSpec` and confirmed green in this round's
own full-suite run:

```
eve.lied.dana.stole.carol.loaf (still unconfessed after 100 turns): True
eve.confessed.dana.stole.carol.loaf: False
recanted.eve: False
```

**Falsification sweep: does the road back reach carol's own injustice?** No — §25 covers this
directly (its own "Update — v32" paragraph, re-checked live rather than assumed): confession clears
the *confessor's* mark and the *confessor's* standing; nothing in this round retracts a belief
already planted in someone else, and nothing lets a third party confess on the framed victim's
behalf. The road back this round builds is real, but it narrows to exactly the people a liar can
face directly — it does not extend to the people her lie reached secondhand, and it does nothing at
all for the people she framed.

→ code: `Prax.Confession` (`confess`/`absolve`/`incorrigible`), `Prax.Worlds.Village`
(`confessWhisper`/`absolveWhisper`, the `honest` trait's second desire, `villageAxioms`'s
`incorrigible` wiring); asserted in `Prax.ConfessionSpec`, `Prax.VillageSpec`.

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
| Emotions (coexisting, targeted, fading) | `Prax.Emotion` `feelToward` | "feels annoyed toward you" after a snub |
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
| Forward-chaining derivation (defeasible) | `Prax.EL` / `Prax.Derive` | `prax feud`: 1 wrong + 4 rules (v31 folds membership in) → a feud; amends dissolves it |
| Static type checker + sort inference | `Prax.TypeCheck` | `prax check <world>` |
| Timed junctions, character sketches (memories: built v18, removed v46 — presentation, not world content) | `Prax.Script` / `Worlds.Audience` | `prax audience` |
| Quantified outcomes (`ForEach`) + authored witnessing | `Prax.Engine` / `Prax.Witness` | `prax village`: carol (co-present) believes bob's theft and can confront him; dana (elsewhere) doesn't |
| Gossip / sourced hearsay (`gossip`/`heard`, multi-valued `.seen`/`.heard.<source>` provenance) | `Prax.Rumor` | `prax village`: carol tells dana what she saw; hearsay licenses suspicion, not confrontation |
| `standing`/`standingUnless`/`regardedAs`/`notoriety` (derived reputation, base-fact atonement defeater) | `Prax.Repute` | `prax village`: three regards tip `notorious.bob.thief`; atonement dissolves every regard while the belief persists; re-offense revokes it and an atoned bob is deterred from a restocked stall |
| Secrets & deception (`conceal`/`lie`) | `Prax.Deceit` | `prax village`: bob's concealment want still gates a watched theft (mid-project opportunism, §26; the scripted "secret keeps"/"perfect crime" tests); eve frames carol, and the lie cascades into real shunning and notoriety with no recourse for the framed |
| Endeavors: staged practices, dormant pursuits (`endeavor`/`Stage`) | `Prax.Project` | `prax village`: bob undertakes `earnBread` unprompted at t=0, sweeps the square in public, and bakes the loaf he'd otherwise have to steal; watching him sweep is enough for the village to presume his purpose and predict his next stage |
| Temperament as conduct-valuations (`Trait`/`personaVocabulary`/`bearing`/`transparent`/`cast`); a lie marks the liar (`Actor.lied.Hearer.<event>`) | `Prax.Persona`, `Prax.Deceit` | `prax village`: gale bears `honest` and never lies despite sharing eve's exact spite; everyone presumes her conscience from t=0; a believed conscience nets against a believed motive in `predictMove`; eve's whisper deceives gale too, and (forced, since v30's threshold fear stops her reaching gale unprompted) gale spreads it onward by ordinary gossip, no mark, carrying it back to eve as "evidence" for her own fabrication |
| Debt as a beneficiary'd obligation (`owe`/`settle`), belief-gated deadbeat standing | `Prax.Debt` | `prax village`: a witnessed default derives `regards.<W>.<debtor>.deadbeat`; the debtor himself, unavoidably co-present at his own default, always regards himself one even when no one else does; repayment defeats it by the same base-fact-defeater idiom as v21's thief |
| Blackmail (`shakedown`: threaten/comply/defy/expose), threshold fear | `Prax.Blackmail`, `Prax.Worlds.Village` | `prax village`: a threat is a motive-belief deposit the victim's own round-walk prices; a standing threat is exposable, so stalling ties defiance; carol, holding eyewitness evidence of eve's whisper, shakes her down and buys real silence; eve's own fear of the notoriety brink also makes her a one-shot liar in free play, retelling §27's laundering under a forced continuation |
| Membership as one spine (`member.<who>!<faction>`, `comrades`, belief-gated `factionStanding`); kin derivation (`kinAxioms`: marriage symmetry, sibling, grandparent, one-directional in-laws) and `wed`'s marriage-fact-plus-membership-overwrite; succession as single-slot exclusion | `Prax.Faction`, `Prax.Kin` | `prax feud`: bob/carol/dave share house `kestrel` (`comrades` derives their alliance, replacing the old hand-authored pairwise ties — §19/§29); esme weds into it (`wed "esme" "kestrel" "dave"`), inherits the grudge, and shuns alice unprompted on her first turn |
| Confession & absolution (`confess`/`absolve`/`incorrigible`): a mark converts rather than deletes, confessing self-incriminates through sourced hearsay, absolution is a refusable second-party grant, and an absolver's patience is `notoriety`'s Count idiom pointed inward | `Prax.Confession` | `prax village`: eve confesses her frame-up to gale (already a believer — costless) and gale absolves; a fresh whisper snaps the absolution away; a second absolver, now knowing two instances, refuses further absolution — and confessing to carol, the party actually wronged, never rationally beats eve's baseline at any authored generosity |

If the tables and scene lines don't convince you a feature is really doing what's claimed, the
same behaviours are asserted in the test suite (`cabal test`, 431 tests). Part I: `Prax.QuerySpec`,
`Prax.EngineSpec`, `Prax.PlannerSpec` + `Prax.MindsSpec` (wants/utility/lookahead, now a round-walk
over believed minds — `predictMove`, `charDesires`, `professed`/`conventional`), `Prax.CoreSpec`
(emotions/relationships), `Prax.ReactionsSpec` (reactions, norms, norm-avoidance), `Prax.BeliefsSpec`
(per-agent & false beliefs), `Prax.ConversationSpec` (speaker turns, topics, one-shot quips),
`Prax.ArcSpec` (arc stages), `Prax.DeonticSpec` (□, discharge, breach, contrary-to-duty),
`Prax.DebtSpec` (`owe`/`settle`, the demand→deadbeat-standing lifecycle, belief-gated visibility and
the debtor's own unavoidable self-regard), `Prax.ConfessionSpec` (mark conversion and its priced
residue, the deposit landing as sourced hearsay, absolution's grant/refusal gate, `incorrigible`'s
Count idiom pinned at threshold and per-absolver, re-offense snapping a defeater, and both sides of
the spontaneous-confession and blackmail-defense arithmetic measured live before being pinned),
`Prax.DbSpec` (the trie's exact `!`/`.`/retract semantics, incl. the reverted ghost-pruning
attempt's own regression net — an asserted instance fact must survive its transient children
draining to zero), `Prax.BarSpec`, and `Prax.LoopSpec` (a deterministic
25-turn replay — since v44 the round boundary, not a ticker in the cast, runs the schedule). Part II: `Prax.IntrigueSpec` (death + branching endings, incl. the
confidant/victim `predictMove` split), `Prax.StressSpec`, `Prax.PersistSpec` (save/resume),
`Prax.ScriptSpec` + `Prax.Script.JsonSpec` (scene layer + JSON, incl. the one-boundary story-rule
law, timed junctions/sketches, and the `audience`), `Prax.DirectorSpec` (player-as-DM), `Prax.ELSpec` + `Prax.DeriveSpec` (the
exclusion-logic lattice and forward chaining), `Prax.TypeCheckSpec`, `Prax.WitnessSpec` +
`Prax.VillageSpec` + `Prax.RumorSpec` + `Prax.SightSpec` (`ForEach` witnessing, co-presence, the
confront affordance, sourced hearsay and the gossip gate, and the sighting schedule rule whose
stamps gate whose moves get predicted), `Prax.ReputeSpec` (derived standing, the base-fact
atonement defeater, and notoriety at threshold — `VillageSpec`'s later cases carry the same
mechanisms through the full autonomous arc, the re-offense snap-back, and the resulting
deterrence), `Prax.DeceitSpec` (`conceal`'s shape and its watched/unwatched planner probe,
`lie`'s no-evidence gate, self-framing and subject-is-hearer exclusions, one-shot-per-hearer,
hearing-your-own-lie-back replacing `lie` with `gossip`, and — v25 — the liar's own mark
(`<liar>.lied.<hearer>.<event>`), forgettable and additive, with truthful `gossip` leaving none —
`VillageSpec`'s later cases carry the same mechanisms into the full village: a watched theft still
fails, the perfect crime still needs a genuinely empty square, and eve's frame-up still cascades to
shunning with no recourse), `Prax.ProjectSpec` (`endeavor`'s undertake/stage-gating/yield shape on a
standalone oven-building fixture, the pursuit desire's exact shape and its dormant-vs-undertaken
believability, and the horizon regression driving four stages to completion at planner depth 2 —
`VillageSpec`'s later cases carry the same mechanism into the full village: bob's unforced
redemption, the watching-teaches-purpose inference feeding a belief-relative, myopic `predictMove`,
and the mid-project opportunism beat that keeps concealment honest), and `Prax.PersonaSpec`
(`bearing`/`personaVocabulary`/`cast` mechanics and their loud-error guards, `transparent`'s
defeasible presumption, the conduct-valuation core split via `pickAction` on a temptation-bearing
twin pair, the marginal-lie property, and believed-conscience prediction — `VillageSpec`'s later
cases carry the same mechanism into the full village: gale's temperament legible from t=0, the
free-play drive where eve's frame-up proceeds and gale never lies unprompted, and the told-about-spite
prediction contrast between eve and gale), and `Prax.BlackmailSpec` (`shakedown`'s motive-belief
deposit, self-motivated credibility, standing-threat exposure, and the compliance arithmetic pinned
on both sides — two onlookers comply, one rationally defies, an exact stall-tie between defy and
wait — plus the reserved-variable collision guards found in review) — `VillageSpec`'s shakedown-arc
cases carry the same mechanism into the full village: carol's threat lands once she holds eyewitness
evidence, eve complies and the reputation stack stays undisturbed for everyone uninvolved, and
threshold fear leaves eve's below-the-brink free-play whispering exactly as rational as it always
was while retelling, under a forced continuation, the one free-play consequence (§27's laundering)
it structurally forecloses), `Prax.FactionSpec` (`comrades`'s shared-membership derivation and its
`X≠Y`/cross-faction negatives, defection's retraction-safety, `factionStanding`'s belief-gating —
including the fratricide and victim-self-belief pins — and its reserved-variable guards), and
`Prax.KinSpec` (each `kinAxioms` rule positive and negative, `wed`'s two facts and both parties'
name guards, dissolution un-deriving in-laws while membership persists, and the succession
lifecycle's death-gating, child-only claims, and single-slot race resolution) — `Prax.FeudSpec`
carries both into the emergent sandbox unmodified: the five original assertions untouched by the
refactor, plus the wedding beat's derivation flip and driven shunning — `VillageSpec`'s own later
cases carry `Prax.Confession` into the full village: eve's costless confession to gale (the
deposit self-sourced, not a re-assertion of the framed content), absolution dissolving standing
while every belief persists, a fresh whisper snapping the defeater away and landing every square
bystander at once, gale's patience running out at the second instance while dana's (one instance)
doesn't, and eve's frame-up surviving unconfessed through 100 turns of unforced free play.

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
