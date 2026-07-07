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

If the tables and scene lines don't convince you a feature is really doing what's claimed, the
same behaviours are asserted in the test suite (`cabal test`): see `Prax.QuerySpec`,
`Prax.EngineSpec`, `Prax.PlannerSpec`, `Prax.BarSpec` (drunkenness + bell), and `Prax.LoopSpec`
(a deterministic 12-turn replay of the emergent order→fulfill→serve arc).

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

## What v1 does *not* yet model

The bar exercises the whole engine, but the engine is deliberately smaller than Versu. Not yet
built (see `docs/LEDGER.md`): emotions, role-evaluation relationships, beliefs, reactions as
auto-spawned practices, a story-manager practice, the full first-order query grammar
(`∀`/`∃`/`∨`/`→`), and a text authoring language. Those are the next milestones.
