# Vampire Detection, Part 1 — the act channel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a bite *expose the biter* — a witnessed feed makes every co-present character (including the victim) believe the biter is a vampire — and give the vampire a principled reason to hide, so it emergently **disguises before feeding** to keep its identity concealed.

**Architecture:** Add three things to `rust/prax-worlds/src/vampire.rs`, composing existing `prax_vocab` combinators over the skeleton world: (1) the `feed` action becomes `witness::observable`, depositing a `bit` belief keyed on the biter's *apparent* identity; (2) an axiom bridges a believed bite to a believed vampire, and `rumor::gossip` spreads it; (3) a `disguise`/`drop disguise` pair flips the actor's apparent identity to `someone`, and every character holds `deceit::conceal("vampire.<self>")`, so the depth-2 planner disguises before biting. This is exactly the village's `observable`-steal + `conceal`-theft mechanism, re-skinned.

**Tech Stack:** Rust (workspace at `rust/Cargo.toml`); `prax-core` engine; `prax-vocab` combinators (`witness`, `deceit`, `rumor`, `beliefs`). Build/test with `cargo … --manifest-path rust/Cargo.toml -p prax-worlds --lib`.

## Global Constraints

- Only `rust/prax-worlds/src/vampire.rs` changes. No sibling-file edits; `git checkout --` any `cargo fmt` drift so only `vampire.rs` is committed.
- `type_check(&st)` MUST stay empty (`the_world_builds_and_is_well_formed`). A POSITIVE read of a fact nothing produces is a dead condition; a negation of an unproducible fact is fine.
- This is NEW content — no frozen oracle. Every mechanic ships with a committed, deterministic behavioural test; a "verified" claim the repo cannot reproduce is a fabrication.
- The world stays OFF the differential matrix (already true; do not add it to `ported()`/`S7_WORLDS`/`S8_WORLDS`).
- Beliefs use the `prax_vocab::beliefs` convention: `belief_about(who, issue) = "<who>.believes.<issue>"`; a witnessed deposit adds a `.seen` leaf. `conceal(event, k)` wants `absent(["Anyone.believes.<event>"])`, so the suspicion the vampire conceals must be a `<W>.believes.vampire.<X>` head (no `.seen`), derived by an axiom.
- The existing skeleton behaviour must not regress: the infection still closes to `ending.vampires` (`the_infection_runs_to_an_ending`), and the seeded feed tests still pass. A vampire that now spends turns disguising may reach the ending LATER — if `the_infection_runs_to_an_ending`'s 400-turn cap is exceeded, raise the cap (documented) rather than weakening the assertion.

---

## Interfaces this plan relies on (verified signatures)

- `prax_vocab::witness::observable(copresence: &Vec<Condition>, event: &str, act: Action) -> Action` — appends, to `act.then`, a deposit making every co-present `Witness` (≠ `Actor`) believe `<Witness>.believes.<event>.seen`. `event` may use the action's bound variables.
- `prax_vocab::witness::saw(who: &str, event: &str) -> Condition` — matches `<who>.believes.<event>.seen`.
- `prax_vocab::deceit::conceal(event: &str, k: i32) -> Result<Want, WorldError>` — `Want([absent(["Anyone.believes.<event>"])], k)`; `event` must be variable-free.
- `prax_vocab::rumor::gossip(copresence, gate: Vec<Condition>, pat: &str, label: &str) -> Result<Action, WorldError>` — a teller who believes `pat` tells a co-present hearer (never the subject, never an eyewitness, one-shot), depositing `<Hearer>.believes.<pat>.heard.<teller>`. `pat`'s first variable is the subject.
- `prax_vocab::beliefs::belief_about(who, issue) -> String`.
- From `prax_core` (already used in vampire.rs): `Axiom::new(when, head)`, `ScheduleRule`, `Practice`/`Action` builders, `Want::new(Vec<Condition>, i32)`, `matches`/`not_`/`neq`/`insert`, `State` setters.

Add these imports to `vampire.rs` as the tasks introduce them:
`use prax_vocab::witness::{observable, saw};`, `use prax_vocab::deceit::conceal;`, `use prax_vocab::rumor::gossip;`.

The bite-witnessing co-presence template (used by `observable` and `gossip`), define once near the top of the module:
```rust
/// Co-presence for a witnessed bite: the fixed `witness`/`rumor` variables
/// `Witness`/`Actor` (and `Hearer` via `as_role`) resolved against this world's
/// location facts. Same shape as [`sighting`], in `Witness`/`Actor` terms.
fn bite_witnessing() -> Vec<Condition> {
    vec![
        matches("practice.world.world.at.Actor!P"),
        matches("practice.world.world.at.Witness!P"),
    ]
}
```

---

### Task 1: A bite is witnessed — the victim and bystanders come to believe it happened

**Files:** Modify `rust/prax-worlds/src/vampire.rs` (`prey_practice`'s `feed` action; add `bite_witnessing`). Test: same `mod tests`.

**Interfaces:**
- Consumes: the skeleton `feed` action; `witness::observable`; `bite_witnessing()`.
- Produces: after a feed, every co-present character (incl. the victim) holds `<W>.believes.bit.<biter>.<victim>.seen`. Later tasks read this.

- [ ] **Step 1: Write the failing test**

```rust
    // H: detection spec "a witnessed bite makes co-present characters believe a bite occurred"
    #[test]
    fn a_bite_is_witnessed_by_the_victim_and_bystanders() {
        // mara + bram (victim) + cole (bystander) all at the mill
        let mut st = seeded_two_at("mara", "bram", "mill");
        st.perform_outcome(&insert("practice.world.world.at.cole!mill"))
            .expect("place cole at the mill");
        make_hungry(&mut st, "mara");
        let bite = ground_feed(&mut st, "mara", "bram");
        st.perform_action(&bite);
        // the victim believes mara bit them
        assert!(
            fact(&mut st, "bram.believes.bit.mara.bram.seen"),
            "the victim witnesses who bit them"
        );
        // a co-present bystander believes it too
        assert!(
            fact(&mut st, "cole.believes.bit.mara.bram.seen"),
            "a bystander witnesses the bite"
        );
        // the biter is not their own witness
        assert!(
            !fact(&mut st, "mara.believes.bit.mara.bram.seen"),
            "the actor is not their own witness"
        );
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml -p prax-worlds --lib vampire::tests::a_bite_is_witnessed`
Expected: FAIL — no `.believes.bit.` fact exists yet.

- [ ] **Step 3: Wrap the feed action in `observable`**

Add `use prax_vocab::witness::{observable, saw};` and `bite_witnessing()` (above). In `prey_practice`, wrap the existing `feed` action so it deposits a witnessed `bit.Actor.Prey` (biter NAMED for now; Task 3 makes it maskable):

```rust
fn prey_practice() -> Practice {
    Practice::new("prey")
        .name("Feeding")
        .roles(["Scene"])
        .action(observable(
            &bite_witnessing(),
            "bit.Actor.Prey",
            Action::new("[Actor]: feed on [Prey]")
                .when([
                    matches("vampire.Actor"),
                    matches("bloodHunger.Actor"),
                    not_("fed.Actor"),
                    matches("practice.world.world.at.Actor!Spot"),
                    matches("practice.world.world.at.Prey!Spot"),
                    neq("Actor", "Prey"),
                    not_("vampire.Prey"),
                    matches("turn!Now"),
                ])
                .then([
                    insert("mark.Prey.neck"),
                    insert("bittenOn.Prey!Now"),
                    insert_for(FEED_COOLDOWN, "fed.Actor"),
                    delete("bloodHunger.Actor"),
                ]),
        ))
}
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cargo test --manifest-path rust/Cargo.toml -p prax-worlds --lib vampire::tests::a_bite_is_witnessed`
Expected: PASS. Also run the whole `-p prax-worlds --lib` suite — still green (the observable wrapper only APPENDS a deposit; the skeleton feed effects are untouched).

- [ ] **Step 5: Commit**

```bash
git commit -am "vampire(detect): the feed action is witnessed — co-present characters believe the bite"
```

---

### Task 2: A believed bite becomes a believed vampire (the suspicion bridge)

**Files:** Modify `rust/prax-worlds/src/vampire.rs` (add an axiom; register it). Test: same `mod tests`.

**Interfaces:**
- Consumes: `<W>.believes.bit.<X>.<Y>.seen` (Task 1).
- Produces: derived belief `<W>.believes.vampire.<X>` — the head `conceal("vampire.<X>")` reads (Task 4) and `gossip` spreads (Task 5).

- [ ] **Step 1: Write the failing test**

```rust
    // H: detection spec "believing X bit Y ⟹ believing X is a vampire"
    #[test]
    fn a_witnessed_bite_makes_the_witness_suspect_the_biter() {
        let mut st = seeded_two_at("mara", "bram", "mill");
        make_hungry(&mut st, "mara");
        let bite = ground_feed(&mut st, "mara", "bram");
        st.perform_action(&bite);
        assert!(
            fact(&mut st, "bram.believes.vampire.mara"),
            "the bitten victim suspects mara is a vampire"
        );
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test … vampire::tests::a_witnessed_bite_makes_the_witness_suspect`
Expected: FAIL — `bram.believes.vampire.mara` is not derived.

- [ ] **Step 3: Add the suspicion axiom and register it**

Add the axiom fn (near `transformation`):
```rust
/// Suspicion, the act channel: whoever believes they saw `Biter` bite someone
/// believes `Biter` is a vampire — biting is how vampirism manifests, so a
/// witnessed biter is a suspected vampire. Derived (a belief head, no `.seen`),
/// so it dissolves if its supporting memory ever does and it is exactly the
/// `<W>.believes.vampire.<X>` shape [`conceal`] quantifies over. A disguised
/// bite (Task 3) binds `Biter = someone`, deriving the inert `…vampire.someone`.
fn bite_breeds_suspicion() -> Axiom {
    Axiom::new(
        vec![matches("Believer.believes.bit.Biter.Victim.seen")],
        ["Believer.believes.vampire.Biter"],
    )
}
```
Register it in `vampire_world()` — add it to the axiom set (keep `transformation` first):
```rust
    st.set_axioms(vec![transformation(), bite_breeds_suspicion()])
        .expect("vampire village axioms");
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cargo test … vampire::tests::a_witnessed_bite_makes_the_witness_suspect`
Expected: PASS. Run the whole `-p prax-worlds --lib` suite — still green, and `the_world_builds_and_is_well_formed` still asserts `type_check == []` (the axiom's body reads a produced `.believes.bit.` fact; its head is a belief, read by nothing unproducible).

- [ ] **Step 5: Commit**

```bash
git commit -am "vampire(detect): a witnessed bite breeds suspicion — believe bit ⟹ believe vampire"
```

---

### Task 3: Apparent identity — disguise masks the biter to "someone"

**Files:** Modify `rust/prax-worlds/src/vampire.rs` (add `appears.X!X` setup; a `disguise` practice with `disguise`/`drop disguise`; rewire the feed event to read the apparent identity). Test: same `mod tests`.

**Interfaces:**
- Consumes: the witnessed feed (Task 1); the suspicion axiom (Task 2).
- Produces: `appears.X!<name>` (a single-valued slot, `!X` by default, `!someone` while disguised); a `disguise`/`drop disguise` affordance. A disguised bite deposits `bit.someone.<victim>` — so no `vampire.<biter>` suspicion attaches.

- [ ] **Step 1: Write the failing test**

```rust
    // H: detection spec "a disguised bite records 'someone', not the biter's name"
    #[test]
    fn a_disguised_bite_masks_the_biter() {
        let mut st = seeded_two_at("mara", "bram", "mill");
        make_hungry(&mut st, "mara");
        // mara disguises: her apparent identity becomes "someone"
        let disg = find_action(&mut st, "mara", "put on a disguise");
        st.perform_action(&disg);
        assert!(fact(&mut st, "appears.mara!someone"), "disguise masks the apparent identity");
        // now she feeds while disguised
        let bite = ground_feed(&mut st, "mara", "bram");
        st.perform_action(&bite);
        // the victim believes SOMEONE bit them, not mara
        assert!(fact(&mut st, "bram.believes.bit.someone.bram.seen"), "the bite is attributed to 'someone'");
        assert!(!fact(&mut st, "bram.believes.bit.mara.bram.seen"), "the biter's name is not recorded");
        // and so no vampire-suspicion attaches to mara
        assert!(!fact(&mut st, "bram.believes.vampire.mara"), "a masked bite breeds no suspicion of mara");
    }
```

Add this test helper alongside `ground_feed` (it is `ground_feed` generalized to any action label):
```rust
    /// The offered action whose label contains `needle`, grounded off the real
    /// `possible_actions` enumeration. Panics (with the offer list) if absent.
    fn find_action(st: &mut State, actor: &str, needle: &str) -> GroundedAction {
        let had = labels(st, actor);
        st.possible_actions(actor)
            .into_iter()
            .find(|g| g.label.contains(needle))
            .unwrap_or_else(|| panic!("no action containing {needle:?} offered to {actor}; available: {had:?}"))
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test … vampire::tests::a_disguised_bite_masks_the_biter`
Expected: FAIL — no `put on a disguise` action, no `appears` fact.

- [ ] **Step 3: Add the apparent-identity slot, the disguise practice, and rewire the feed event**

3a. In `vampire_setup`, give every villager their default apparent identity (themselves). Build it from `HOMES` (the single roster source), chained with the existing setup facts:
```rust
        // Everyone appears as themselves until they disguise (a single-valued
        // slot the disguise practice flips to `someone`).
        // ...add to the vampire_setup vector:
        //   HOMES.iter().map(|(who, _)| insert(format!("appears.{who}!{who}")))
```
Concretely, extend `vampire_setup`'s returned vector by chaining
`HOMES.iter().map(|(who, _)| insert(format!("appears.{who}!{who}")))` alongside the position inserts.

3b. Add a `disguise` practice (a `Scene` singleton like `prey`) with two actions:
```rust
/// The general disguise affordance — available to ANYONE, not a vampire tell
/// (Part 2 gives it an innocent use). Disguising flips the actor's single-valued
/// `appears.Actor!` slot to `someone`; dropping it restores their name. A
/// witnessed act by a disguised actor is attributed to `someone`.
fn disguise_practice() -> Practice {
    Practice::new("disguise")
        .name("Disguise")
        .roles(["Scene"])
        .action(
            Action::new("[Actor]: put on a disguise")
                .when([matches("appears.Actor!Actor")]) // not already disguised
                .then([insert("appears.Actor!someone")]),
        )
        .action(
            Action::new("[Actor]: drop the disguise")
                .when([matches("appears.Actor!someone")])
                .then([insert("appears.Actor!Actor")]),
        )
}
```
Register the practice and spawn its singleton instance:
- In `vampire_world()`: `st.define_practices([world_practice(), prey_practice(), disguise_practice()])`.
- In `vampire_setup`: add `insert("practice.disguise.here")`.

3c. Rewire the feed's witnessed event to read the apparent identity. The `feed` action's `when` gains `matches("appears.Actor!Appears")`, and the `observable` event becomes `"bit.Appears.Prey"`:
```rust
        .action(observable(
            &bite_witnessing(),
            "bit.Appears.Prey",
            Action::new("[Actor]: feed on [Prey]")
                .when([
                    matches("vampire.Actor"),
                    matches("bloodHunger.Actor"),
                    not_("fed.Actor"),
                    matches("practice.world.world.at.Actor!Spot"),
                    matches("practice.world.world.at.Prey!Spot"),
                    neq("Actor", "Prey"),
                    not_("vampire.Prey"),
                    matches("appears.Actor!Appears"),
                    matches("turn!Now"),
                ])
                .then([
                    insert("mark.Prey.neck"),
                    insert("bittenOn.Prey!Now"),
                    insert_for(FEED_COOLDOWN, "fed.Actor"),
                    delete("bloodHunger.Actor"),
                ]),
        ))
```
`appears.Actor!Actor` (undisguised) binds `Appears` to the biter's name — so Task 1's and Task 2's tests, which expect `bit.mara.bram`, still hold. `appears.mara!someone` binds `Appears = someone`.

- [ ] **Step 4: Run it to verify it passes**

Run: `cargo test … vampire::tests::a_disguised_bite_masks_the_biter`
Then the whole `-p prax-worlds --lib` suite. Expected: PASS and green — Tasks 1–2's tests still pass (undisguised `Appears = biter`), and `type_check` stays empty (`appears.Actor!Appears` reads a produced slot).

- [ ] **Step 5: Commit**

```bash
git commit -am "vampire(detect): disguise masks the apparent identity — a masked bite reads 'someone'"
```

---

### Task 4: The vampire's concealment want — being believed a vampire is costly

**Files:** Modify `rust/prax-worlds/src/vampire.rs` (add a `conceal` want per villager; register it as a desire held by all). Test: same `mod tests`.

**Interfaces:**
- Consumes: the derived `<W>.believes.vampire.<X>` suspicion (Task 2).
- Produces: every character holds `conceal("vampire.<self>")` — the intrinsic cost that (with disguise, Task 3) drives the emergent hiding validated in Task 6.

- [ ] **Step 1: Write the failing test**

The want only bites once someone believes the secret, so test its effect on scoring: an undisguised, witnessed bite (which breeds `bram.believes.vampire.mara`) must make mara's realized state score WORSE than the same world where she stayed concealed. Assert via the planner's score of the two candidate first moves for a hungry mara who has a fresh victim: `put on a disguise` must outscore `feed on bram` at depth 2 (feeding undisguised forfeits concealment; disguising preserves it).

```rust
    // H: detection spec "the vampire conceals being believed a vampire, so it disguises first"
    #[test]
    fn a_hungry_vampire_prefers_to_disguise_before_feeding_in_the_open() {
        let mut st = seeded_two_at("mara", "bram", "mill");
        make_hungry(&mut st, "mara");
        let choice = st.pick_action(2, &character("mara"));
        let label = choice.map(|g| g.label).unwrap_or_default();
        assert!(
            label.contains("put on a disguise"),
            "a hungry vampire with an unconcealed identity disguises before biting in the open; got {label:?}"
        );
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test … vampire::tests::a_hungry_vampire_prefers_to_disguise`
Expected: FAIL — with no concealment want, mara just feeds (hunger −22 dominates; concealment is worth 0), so the pick is `feed on bram`.

- [ ] **Step 3: Add the concealment want to every villager**

Extend `vampire_cast` so each villager, alongside `sate-hunger` and the home anchor, holds a concealment want for their own vampirism. The `conceal` combinator returns a `Want`; use `.want(...)`:
```rust
    // inside the HOMES.iter().map(...) closure, add to each Character:
    //   .want(conceal(&format!("vampire.{who}"), CONCEAL_WEIGHT).expect("a villager's concealment want"))
```
Add the weight constant near the others, with its justification:
```rust
/// How much a villager values NOT being believed a vampire. Reuses the village
/// `conceal` scale (bob's `stole.bob.loaf` concealment is 12): strong enough
/// that forfeiting concealment outweighs a one-turn delay in sating hunger, so a
/// hungry vampire disguises before biting rather than biting in the open. Tuned
/// against behaviour in Task 6, starting from the proven village magnitude.
const CONCEAL_WEIGHT: i32 = 12;
```
Add `use prax_vocab::deceit::conceal;`. Update the `vampire_cast` doc comment to list the third want and note it is dormant for a human (nobody believes them a vampire) and load-bearing once they turn.

- [ ] **Step 4: Run it to verify it passes**

Run: `cargo test … vampire::tests::a_hungry_vampire_prefers_to_disguise`
Expected: PASS. If it does NOT pass, `CONCEAL_WEIGHT` is too low relative to the hunger/discount — raise it (this is the tuning the constant's comment anticipates) until the disguise-first pick emerges, and record the value that works. Then run the whole `-p prax-worlds --lib` suite — green (the seeded feed tests still pass: once disguised or with no witness-cost in play they still feed; and `a_hungry_vampire_with_prey_chooses_to_feed` from the skeleton may now pick `put on a disguise` first — if so, update THAT test to drive one more step and assert the feed follows, since disguise-then-feed is the correct new behaviour, not a regression).

- [ ] **Step 5: Commit**

```bash
git commit -am "vampire(detect): the concealment want — a vampire disguises before biting in the open"
```

---

### Task 5: Gossip spreads the suspicion

**Files:** Modify `rust/prax-worlds/src/vampire.rs` (add a `gossip` action to the `prey` or a `talk` practice). Test: same `mod tests`.

**Interfaces:**
- Consumes: `<W>.believes.bit.<X>.<Y>.seen` (Task 1) and `bite_witnessing()`.
- Produces: an absent villager can come to believe a bite via hearsay (`<Hearer>.believes.bit.<X>.<Y>.heard.<teller>`), which the Task-2 axiom then turns into suspicion.

- [ ] **Step 1: Write the failing test**

```rust
    // H: detection spec "suspicion spreads by gossip, not omniscience"
    #[test]
    fn a_witnessed_bite_spreads_by_gossip_to_an_absent_villager() {
        // cole witnesses the bite at the mill; later, cole and rosa are together
        // and cole gossips it to rosa, who was absent.
        let mut st = seeded_two_at("mara", "bram", "mill");
        st.perform_outcome(&insert("practice.world.world.at.cole!mill")).expect("cole at mill");
        make_hungry(&mut st, "mara");
        let bite = ground_feed(&mut st, "mara", "bram");
        st.perform_action(&bite);
        assert!(fact(&mut st, "cole.believes.bit.mara.bram.seen"), "cole witnessed it");
        // bring rosa to the mill with cole; cole tells her
        st.perform_outcome(&insert("practice.world.world.at.rosa!mill")).expect("rosa at mill");
        let tell = find_action(&mut st, "cole", "spread word");
        st.perform_action(&tell);
        assert!(
            fact_prefix(&mut st, "rosa.believes.bit.mara.bram.heard"),
            "rosa hears of the bite from cole"
        );
        // and reclosing derives rosa's suspicion via the Task-2 axiom
        reclose(&mut st);
        assert!(fact(&mut st, "rosa.believes.vampire.mara"), "hearsay breeds suspicion too");
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test … vampire::tests::a_witnessed_bite_spreads_by_gossip`
Expected: FAIL — no `spread word` action.

- [ ] **Step 3: Add the gossip action**

Add a `talk` practice (or extend an existing scene) carrying a `gossip` action over the bite belief. `gossip`'s `pat` first variable is the subject:
```rust
/// Word travels: a villager who believes they saw a bite tells a co-present
/// other (never the biter, never another eyewitness, once per hearer). The
/// hearsay then breeds suspicion through [`bite_breeds_suspicion`] just as an
/// eyewitness account does.
fn talk_practice() -> Practice {
    Practice::new("talk")
        .name("Talk")
        .roles(["Scene"])
        .action(
            gossip(&bite_witnessing(), Vec::new(), "bit.Subject.Victim", "[Actor]: spread word about [Subject]")
                .expect("the bite-gossip action"),
        )
}
```
Add `use prax_vocab::rumor::gossip;`. Register the practice in `define_practices` and spawn its singleton with `insert("practice.talk.here")` in `vampire_setup`.

- [ ] **Step 4: Run it to verify it passes**

Run: `cargo test … vampire::tests::a_witnessed_bite_spreads_by_gossip`
Then the whole suite. Expected: PASS and green; `type_check` empty.

- [ ] **Step 5: Commit**

```bash
git commit -am "vampire(detect): gossip spreads a witnessed bite to absent villagers"
```

---

### Task 6: Acceptance — the vampire hides in a real playthrough

**Files:** Modify `rust/prax-worlds/src/vampire.rs` (one acceptance `#[test]`; possibly adjust `CONCEAL_WEIGHT` and the skeleton acceptance cap). Test: same `mod tests`.

**Interfaces:**
- Consumes: everything above; the real game loop `advance` + `npc_act` at depth 2.

- [ ] **Step 1: Write the failing/acceptance test**

Drive the real loop; assert that across the run the vampire NEVER lets its identity attach to a bite — no character ever believes `vampire.mara` — because it disguises before feeding. (Undisguised, the victim would immediately believe it.)

```rust
    // H: detection spec "the crux — the vampire emergently disguises before feeding"
    #[test]
    fn the_vampire_conceals_itself_by_disguising_before_feeding() {
        use prax_core::turn::{advance, npc_act};
        let mut st = vampire_world();
        let mut fed_at_least_once = false;
        for _ in 0..400 {
            let actor = advance(&mut st);
            npc_act(&mut st, 2, &actor);
            if fact_prefix(&mut st, "bittenOn.") {
                fed_at_least_once = true;
            }
            // The invariant: no one ever pins a bite on mara by name.
            assert!(
                !st.labeled_view().iter().any(|f| f.ends_with(".believes.vampire.mara")),
                "mara must stay concealed — she disguises before biting"
            );
            if !st.db_child_keys("ending").is_empty() { break; }
        }
        assert!(fed_at_least_once, "the vampire did feed (concealed), not merely avoid feeding");
    }
```

- [ ] **Step 2: Run it**

Run: `cargo test … vampire::tests::the_vampire_conceals_itself_by_disguising`
Expected: initially it may FAIL if `CONCEAL_WEIGHT` is too low (mara bites undisguised and someone believes `vampire.mara`) — OR if the vampire over-values concealment and never feeds (the second assert catches this). Both are the tuning signal the spec's open risk #1 anticipates.

- [ ] **Step 3: Tune `CONCEAL_WEIGHT` empirically (if needed)**

If the vampire bites in the open, raise `CONCEAL_WEIGHT`; if it never feeds, the concealment is starving it — check that disguise-then-feed is actually reachable at depth 2 (it is: disguise is one action, feed the next) and lower toward the point where it feeds while concealed. Record the working value in the constant's comment. If NO value yields "feeds while concealed" at depth 2, STOP and report: depth-2 is insufficient for two-step concealment and the finding needs the designer (do not weaken the test).

- [ ] **Step 4: Verify the whole suite and the skeleton acceptance**

Run: `cargo test --manifest-path rust/Cargo.toml -p prax-worlds --lib`
Expected: all green, INCLUDING `the_infection_runs_to_an_ending`. If the vampire now takes longer to win (it spends turns disguising) and exceeds that test's 400-turn cap, raise the cap there with a comment (the loop still closes — concealment slows it, it does not stop it). Do NOT weaken any assertion.

- [ ] **Step 5: Commit**

```bash
git commit -am "vampire(detect): acceptance — the vampire hides, disguising before every bite"
```

---

## Self-Review

**Spec coverage (act-channel subset of the detection spec):** witnessed act → belief (T1) ✓; belief-of-bite ⟹ suspicion (T2) ✓; disguise = apparent-identity masking, `bit.someone` under disguise (T3) ✓; `conceal("vampire.self")` as the driver (T4) ✓; gossip spreads it (T5) ✓; the crux — emergent disguise-before-bite (T6) ✓. **Deferred to Part 2 (physical-evidence web):** the mark/scarf channel, winter/stay-warm, church faux-pas, snatch + slander, the disreputable-indulgence disguise ambiguity — and the omniscient-within-scope question those raise. Endings and the infection loop are unchanged; suspicion is not yet lethal.

**Placeholder scan:** every step carries the concrete fact/action/axiom. Two honest in-build tuning points are flagged, not hidden: `CONCEAL_WEIGHT` (T4/T6, a magnitude reusing the village `conceal` scale, tuned to yield the emergent behaviour — the same way the skeleton's `+1` anchor and `-22` hunger were validated) and the possible cap bump in `the_infection_runs_to_an_ending` (T6). The `find_action`/`fact_prefix` helpers are the existing `ground_feed`/`fact` idioms generalized.

**Type consistency:** fact shapes are consistent across tasks — `<W>.believes.bit.<Appears>.<Victim>.seen` (T1/T3), `<W>.believes.vampire.<X>` (T2, the exact shape `conceal` reads in T4), `appears.<X>!<name>` (T3), `bit.Subject.Victim` gossip pattern (T5). The feed event moves from `bit.Actor.Prey` (T1) to `bit.Appears.Prey` (T3) with `appears.Actor!Appears` bound in the same `when` — undisguised `Appears = Actor`, so T1/T2's assertions survive T3.

**Risk carried forward:** the crux (T6) is the spec's #1 open risk. The mechanism is the proven village `observable`+`conceal` pair, and disguise-then-feed sits exactly at depth 2, so it should surface; the lever if not is `CONCEAL_WEIGHT`, not a deeper search. Part 2's snatch/false-positive layer carries the omniscient-within-scope question, deliberately isolated here.
