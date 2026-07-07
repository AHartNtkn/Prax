# Versu — architecture notes

Distilled from primary sources for the `prax` reimplementation. The richest source is the
IEEE paper, read directly (page cites below are to it).

**Primary source:** Evans & Short, "Versu—A Simulationist Storytelling System," *IEEE
Transactions on Computational Intelligence and AI in Games* 6(2):113–130, 2014.
PDF: https://cs.uky.edu/~sgware/reading/papers/evans2014versu.pdf · IEEE:
https://ieeexplore.ieee.org/document/6648395/ (won the IEEE CIS Outstanding Paper Award).

Other primary: Emily Short's blog (https://emshort.blog/category/versu/), esp.
[Introducing Versu](https://emshort.blog/2013/02/14/introducing-versu/),
[Conversation Implementation](https://emshort.blog/2013/02/26/versu-conversation-implementation/),
[Mailbag: Writing for Versu](https://emshort.blog/2017/05/18/mailbag-writing-for-versu/).

---

## 1. Two object types + a logic DB + a decision loop

> "Our simulation is built up out of two types of objects: agents and social practices." (p.116)

Architecture (Fig. 3, p.117): three authored input files (Social Practice File, World
Initialization File, Character File) are parsed and used to populate a central **database**
(the whole world state). A **Decision Maker** feeds an agent the affordances offered by the
practices it participates in; the agent chooses; an **Action Executor** mutates the DB; loop
repeats. Player choice uses the same pipeline except affordances go to the UI instead of the DM.

## 2. Exclusion logic / Praxis (the database)

- "the simulation state is entirely determined by a set of sentences in a modal logic. … There
  are no objects, or pointers, as traditionally conceived." (p.118)
- Claimed advantages: **Visibility** (nothing hidden), **Debuggability** (logical breakpoints:
  which practice made a fact true), **Serializability**. (p.118)
- Grammar (p.119): literals `X ::= S | S.X | S!X`. `.` = ordinary (possibly multi-valued)
  descent; `!` = exclusion: "B is the only way in which A is the case" (single-valued slot).
- Facts form a **trie**; a shared prefix is an object; deleting the prefix deletes the subtree
  "in one fell swoop" (`brown.` removes all facts about Brown). A prefix is "the Praxis
  equivalent of an object." (p.119)
- **Automatic cleanup via `!`:** switching `p!a` → `p!b` (insert `p!b`) auto-removes all of
  state a's local data `p!a.*` "according to the update rules for exclusion logic." (p.119)
  → single-valued edges garbage-collect stale state; STRIPS `remove` clauses become unnecessary.
- Strongly, implicitly typed (cf. ML/Haskell); exclusion info feeds the type checker
  (`agent.sex!gender` ⇒ an agent has exactly one gender). (p.120)

## 3. Social practices

- "A social practice is a hierarchical collection of affordances, providing various options to
  its participants (who are characterized solely in terms of the roles they are playing)." (p.121)
- **Constitutive (not regulative):** "every affordance is contained within a practice and is
  only available if that practice is instantiated." This is how the infinite-choice problem is
  solved — an agent only sees affordances from practices it's in. (p.120)
- Declared with `process`; instantiated by asserting a sentence. Verbatim (p.121):
  ```
  process.greet.X(agent).Y(agent)
    action "Greet"
      preconditions
        X.in!L and Y.in!L
      postconditions
        text "[X] says 'Hi' to [Y obj]"
  end
  ```
  Asserting `process.greet.jack.jill` activates it with `[jack/X, jill/Y]`.
- **Role-agnostic** (any cast can fill the roles) → high replayability; an n-role episode has
  2^n player/NPC assignments. (p.115)
- **Concurrent:** many practices run at once; an agent's options are the *union* of affordances
  from all practices it's in. Practices hold arbitrary persistent state and can spawn practices.
- Action language beyond PDDL: disjunctive preconditions, negation-as-failure, nested ∀/∃
  preconditions, numeric expressions, domain axioms, conditional effects. (p.121)
- **Norms:** norm-violating actions get postconditions that mark the violation; agents have
  strong desires to respect norms; a violation spawns a subpractice offering reactions
  (disapprove/forgive/anger/evict). (p.121)

## 4. Action selection / autonomy

- **Strong autonomy:** "It is always the individual agent who decides what to do, using
  utility-based reactive action selection." Practices only suggest. (Abstract, p.113)
- **Forward-chaining apply-evaluate-undo:** "they actually execute the results of the action …
  evaluate this future world state with respect to their desires … then undo the consequences."
  Undo is cheap because primitives are "efficiently undoable." (p.122)
- **Utility = Σ satisfied desires.** "A want is a desire to make a sentence true, and that
  sentence can be any sentence of exclusion logic." Each want has a numeric modifier; every
  *separate instantiation* (binding) adds its modifier again. (p.122)
- The DM/story manager "is just a particular type of practice"; reactive, hand-authored per
  episode, can even be played by a human. (p.118, p.127)
- Player UI: menu of affordances; two buttons **act** / **more** (more = let NPCs proceed). (p.113)

## 5. Core model (emotion / relationship / belief)

The channel through which otherwise-isolated practices communicate. (p.124)
- **Emotions:** Ekman-based, single slot, new overrides old, remembers previous mood + target +
  triggering event. (p.116, p.124)
- **Relationships = role evaluation** (Sacks' membership categorization): "how well is y playing
  role R, in x's eyes?" Evaluations are **multiple and asymmetric**, stored with a reason:
  `Agent.relationship.Evaluated.role!Value!Explanation`. Acquired by hard-coding, by interpreting
  actions, or by hearing/believing others. (p.122–123)
- **Public relationship state:** a single symmetric long-term stance (friends/lovers/enemies);
  changes require active choice from both; never imposed unilaterally on the player. (p.123, p.125)
- **Beliefs:** world state shared by default; per-issue individual beliefs only where false
  beliefs/disagreement are wanted. Could NOT represent quantified/nested beliefs. (p.124, p.129)
- **Reactivity is a practice:** each action spawns a reacting practice proposing responses; a
  response is itself an action that can spawn further reactions. (p.123–124)
- **Character arcs:** internal high-level choices (interiority); true transformation (choosing
  against one's own desires) is player-only. (p.124)
- **Conversation:** speaker + topics; dialogue lines are **quips** (text template + effects on
  social/emotional state), possibly shared across speakers, tagged with topics. (p.125)

## 6. Notable claimed capabilities (targets for the ledger)

- Replayable via role/character separation (same story, many perspectives/assignments).
- Reusable role-agnostic practices (vs Façade's hard-coded joint behaviors).
- Emergent relationships the author never explicitly wrote.
- Concurrent practices → layered, subtextual behavior.
- Scale: >1000 parameterized actions authored in a year; >300 desires. (p.128–129)
- Mixed authorial control: separates decision-making (agent) from coordination + continuity
  (practice). (Fig. 4, p.128)
- Flagship authored title: **Blood & Laurels** (ancient Rome), built on the engine.

## 7. Authoring tool: Prompter

Built on top of Praxis; writers create scenes/dialogue in a play-script format marked up with
emotional/evaluative effects, compiled to Praxis. Cut authoring of a branching 20-min episode
from ~1–2 months to <1 week. (p.130; Short's Mailbag post.)

## Not verified / to close later

- Full formal semantics + decision procedure of exclusion logic live in the paywalled DEON 2010
  paper (Evans, "Introducing Exclusion Logic as a Deontic Logic"). The IEEE paper only
  *describes* the update rules. Needed before formalizing deontic `should` / norm-conflict.
- No public Praxis grammar or Versu source (IP retained by Linden Lab).
