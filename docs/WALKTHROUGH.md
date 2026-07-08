# Walkthrough: understanding `prax` v1 by playing the bar

This guide walks you through the demo world and, as you go, points out exactly which engine
feature each thing exercises. By the end you'll have seen every capability in v1 in action.

Run it:

```sh
cabal run prax
```

You control **`you`**. The other two characters act on their own each turn. Turn order is
round-robin: `you`, then `ada`, then `bex`, repeating. On your turn you pick a numbered action,
or `m` to wait (let others act), or `q` to quit.

> Menu **numbers shift** as options appear and disappear, so this guide names actions rather
> than numbering them. Pick the option whose text matches.
>
> Note: `m` (pass your turn) and the in-world "Wait a moment" action do the same nothing, so the
> player menu hides "Wait a moment" — use `m`. NPCs still have it (they need a "do nothing"
> option so an idle agent isn't forced to wander).

---

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
  → features: asymmetric role evaluation (§X). code: `Prax.Core`.

*(Left to try later, per `docs/LEDGER.md`: public "bonds" via `setBond`, plus beliefs and
conversation.)*

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

*(Deferred, per `docs/LEDGER.md`: the player *as* the DM, richer metalevel repertoire and pacing,
and a generic event stream the director could watch.)*

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

---

## Feature coverage map

Everything implemented in v1, where it lives, and how the demo shows it:

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

If the tables and scene lines don't convince you a feature is really doing what's claimed, the
same behaviours are asserted in the test suite (`cabal test`): see `Prax.QuerySpec`,
`Prax.EngineSpec`, `Prax.PlannerSpec`, `Prax.CoreSpec` (emotions/relationships), `Prax.ReactionsSpec`
(reactions, norms, norm-avoidance), `Prax.BeliefsSpec` (per-agent & false beliefs), `Prax.ConversationSpec`
(speaker turns, topics, one-shot quips), `Prax.ArcSpec` (arc stages), `Prax.BarSpec` (drunkenness +
bell + warmth/mood gates + greeting chain + tipping + rumours + a driven conversation + the director
+ arcs), and `Prax.LoopSpec` (a deterministic 20-turn replay: greet → serve → take-offense → buy,
the director turning two friends against each other, and bex settling into belonging anyway).

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

The bar exercises the whole engine including the v2 core model, v3 reactions & norms, v4 beliefs,
v5 conversation, a v6 story manager, v7 character arcs, and a v8 first-order query grammar
(`∀`/`∃`/`∨`/`→`), but the engine is still deliberately smaller than Versu.
Not yet built (see `docs/LEDGER.md`): public "bonds" in play, richer norms & eviction, a generic
"react to any action" event bus, quantified/nested beliefs, multi-party conversation, the player as
DM, a verified dramatic episode, QA tooling (stress-test / inspector / save-load), and a text
authoring language. Those are the next milestones.
