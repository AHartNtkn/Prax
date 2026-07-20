//! `Prax.VillageSpec`, re-expressed against the Rust engine — the widest
//! world-level pin file in the program (50 labels over the mechanism-densest
//! world).
//!
//! **The long trajectories are SHARED, exactly as the frozen spec shares them.**
//! `VillageSpec` computes `freePlayAt`/`postTheftAt` as top-level CAFs so a test
//! wanting the state after N turns reads a snapshot of the ONE trace rather than
//! re-simulating privately; the planner is deterministic, so this is an identity,
//! not an approximation. Here that is a [`std::sync::OnceLock`] holding the
//! snapshots each trajectory is asked for. Without it this file would re-drive
//! several hundred depth-2 village turns.
//!
//! **The assertions are over `labeled_facts()`/`view_has()`, where the frozen
//! spec asserts over `dbToSentences`/`exists`.** `dbToSentences` FLATTENS the
//! `!`/`.` operator distinction (`src/Prax/Db.hs:14`); `labeled_facts` preserves
//! it, so the two trust-score literals read `carol.relationship.bob.trust.score!-10`
//! here and `"…score.-10"` there. `exists`, by contrast, matches on segment names
//! alone, which is exactly what [`prax_core::engine::State::db_has`] does — so
//! every `exists` assertion transcribes unchanged. The shift is confined to the
//! two rendered-sentence assertions and is strictly more precise: `prax_adjustScore`
//! is the only writer of that family and it always writes `!`.

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::OnceLock;

    use prax_core::engine::State;
    use prax_core::turn::{advance, npc_act};
    use prax_core::types::{Character, Desire, Want, insert};
    use prax_core::query::matches;
    use prax_vocab::core_model::adjust_score;
    use prax_vocab::emotion::{ANGRY, feel_toward, feel_toward_for};
    use prax_vocab::witness::witnessed;
    use prax_worlds::village::{PLAYER_NAME, together, village_world};

    const CAST: [&str; 6] = ["you", "bob", "carol", "dana", "eve", "gale"];

    /// Perform the named actor's action whose label mentions `needle`.
    fn do_act(who: &str, needle: &str, st: &mut State) {
        let found = st
            .possible_actions(who)
            .into_iter()
            .find(|ga| ga.label.contains(needle));
        match found {
            Some(ga) => st.perform_action(&ga),
            None => panic!(
                "no action for {who} matching {needle:?}; had: {:?}",
                st.possible_actions(who)
                    .into_iter()
                    .map(|ga| ga.label)
                    .collect::<Vec<_>>()
            ),
        }
    }

    fn offers(who: &str, needle: &str, st: &mut State) -> bool {
        st.possible_actions(who)
            .iter()
            .any(|ga| ga.label.contains(needle))
    }

    /// One planner-driven turn, with `idle`'s turn consumed but not acted.
    fn idle_step(idle: &str, st: &mut State) {
        let actor = advance(st);
        if actor.name != idle {
            npc_act(st, 2, &actor);
        }
    }

    /// Run `n` turns with everyone planner-driven except `idle`, who waits.
    fn drive_idle(idle: &str, n: usize, st: &mut State) {
        for _ in 0..n {
            idle_step(idle, st);
        }
    }

    /// One round boundary: the engine advances the clock and fires the period-1
    /// sighting rule (v44), making a just-established co-presence concrete in each
    /// character's own memory before the next decision is scored.
    fn tick(st: &mut State) {
        st.round_boundary();
    }

    /// Snapshot a trajectory at each requested step, driving it once.
    fn snapshots(mut st: State, steps: &[usize]) -> BTreeMap<usize, State> {
        let mut want: Vec<usize> = steps.to_vec();
        want.sort_unstable();
        want.dedup();
        let mut out = BTreeMap::new();
        let mut at = 0usize;
        for n in want {
            while at < n {
                idle_step(PLAYER_NAME, &mut st);
                at += 1;
            }
            out.insert(n, st.clone());
        }
        out
    }

    /// The steps free play is sampled at (the union of every `freePlayAt` index
    /// in the frozen spec, plus the two the golden-beat pin reads).
    const FREE_PLAY_STEPS: [usize; 15] = [
        5, 7, 9, 10, 18, 19, 28, 34, 36, 40, 42, 43, 48, 56, 100,
    ];

    fn free_play_at(n: usize) -> State {
        static TRACE: OnceLock<BTreeMap<usize, State>> = OnceLock::new();
        TRACE
            .get_or_init(|| snapshots(village_world(), &FREE_PLAY_STEPS))
            .get(&n)
            .unwrap_or_else(|| panic!("free play was not sampled at step {n}"))
            .clone()
    }

    /// The steps the post-theft trajectory is sampled at.
    const POST_THEFT_STEPS: [usize; 4] = [42, 49, 96, 105];

    fn post_theft_at(n: usize) -> State {
        static TRACE: OnceLock<BTreeMap<usize, State>> = OnceLock::new();
        TRACE
            .get_or_init(|| {
                let mut st = village_world();
                do_act("bob", "steal the loaf", &mut st);
                snapshots(st, &POST_THEFT_STEPS)
            })
            .get(&n)
            .unwrap_or_else(|| panic!("the post-theft trace was not sampled at step {n}"))
            .clone()
    }

    /// The named villager from the cast.
    fn villager(n: &str) -> Character {
        village_world()
            .characters()
            .iter()
            .find(|c| c.name == n)
            .unwrap_or_else(|| panic!("no such villager: {n}"))
            .clone()
    }

    /// The shakedown's forced trajectory (the theft tests' own convention: a
    /// scripted opening, then free play). gale steps out of the mill first —
    /// otherwise she'd be a third simultaneous witness to eve's whisper, tripping
    /// notoriety at the instant of witnessing rather than leaving it "one
    /// predicted exposure from the brink." Carol then arrives and witnesses
    /// directly; both return to the square, where real bystanders (bob, you) make
    /// carol's exposure threat credible rather than empty. The sight ticks after
    /// each move are what let each character's own round-walk correctly price a
    /// threat from someone just out of the room.
    fn whisper_arc_setup() -> State {
        let mut st = village_world();
        for (who, needle) in [
            ("gale", "Go to square"),
            ("carol", "Go to mill"),
            ("eve", "whisper to dana that"),
            ("carol", "Go to square"),
            ("eve", "Go to square"),
        ] {
            do_act(who, needle, &mut st);
            tick(&mut st);
        }
        st
    }

    fn whisper_arc_at(n: usize) -> State {
        static TRACE: OnceLock<BTreeMap<usize, State>> = OnceLock::new();
        TRACE
            .get_or_init(|| snapshots(whisper_arc_setup(), &[3, 6]))
            .get(&n)
            .unwrap_or_else(|| panic!("the whisper arc was not sampled at step {n}"))
            .clone()
    }

    /// The fallback arc's own forced trajectory (whisperArcSetup's idiom): eve
    /// whispers to dana (gale, still at the mill, witnesses the act directly); eve
    /// confesses to gale (costless — gale already regarded her); gale absolves.
    fn redemption_arc_setup() -> State {
        let mut st = village_world();
        for (who, needle) in [
            ("eve", "whisper to dana that carol stole the loaf"),
            ("eve", "confess to gale about framing carol"),
            ("gale", "absolve eve of slander"),
        ] {
            do_act(who, needle, &mut st);
            tick(&mut st);
        }
        st
    }

    /// Continues the trajectory: gale and eve relocate to the square (where
    /// bob/carol/you already are), and eve whispers again — a genuinely NEW hearer
    /// (bob), so a genuinely new distinct `whispered.eve.H` instance.
    fn reoffend_arc_setup() -> State {
        let mut st = redemption_arc_setup();
        for (who, needle) in [
            ("gale", "Go to square"),
            ("eve", "Go to square"),
            ("eve", "whisper to bob that carol stole the loaf"),
        ] {
            do_act(who, needle, &mut st);
            tick(&mut st);
        }
        st
    }

    // H: VillageSpec.hs "Prax.Worlds.Village"
    //
    // The frozen `Prax.VillageSpec`. Every case below is one of its labels.

    // H: VillageSpec.hs "the theft is witnessed by the square, not the mill"
    #[test]
    fn the_theft_is_witnessed_by_the_square_not_the_mill() {
        let mut st = village_world();
        do_act("bob", "steal the loaf", &mut st);
        assert!(
            st.db_has("carol.believes.stole.bob.loaf.seen"),
            "carol (in the square) saw it"
        );
        assert!(
            st.db_has("you.believes.stole.bob.loaf.seen"),
            "you (in the square) saw it"
        );
        assert!(
            !st.db_has("dana.believes.stole.bob.loaf.seen"),
            "dana (at the mill) holds no such belief"
        );
        assert!(
            !st.db_has("bob.believes.stole.bob.loaf.seen"),
            "bob is not his own witness"
        );
    }

    // H: VillageSpec.hs "movement is not news (undeclared actions deposit nothing)"
    #[test]
    fn movement_is_not_news() {
        let mut st = village_world();
        do_act("bob", "Go to mill", &mut st);
        for w in ["you", "carol", "dana"] {
            assert!(
                !st.db_has(&format!("{w}.believes.went.bob.seen")),
                "no one 'believes' bob walked"
            );
        }
    }

    // H: VillageSpec.hs "only a witness can confront the thief"
    #[test]
    fn only_a_witness_can_confront_the_thief() {
        let mut st = village_world();
        do_act("bob", "steal the loaf", &mut st);
        assert!(offers("carol", "confront bob", &mut st), "carol can confront");
        assert!(!offers("dana", "confront bob", &mut st), "dana cannot");
    }

    // H: VillageSpec.hs "confronting cools the witness toward the thief, once"
    #[test]
    fn confronting_cools_the_witness_once() {
        let mut st = village_world();
        do_act("bob", "steal the loaf", &mut st);
        do_act("carol", "confront bob", &mut st);
        assert!(
            st.labeled_facts()
                .contains(&"carol.relationship.bob.trust.score!-10".to_owned()),
            "trust dropped"
        );
        assert!(
            !offers("carol", "confront bob", &mut st),
            "confront is one-shot"
        );
    }

    // H: VillageSpec.hs "the rumor spreads on its own: carol carries the news to the mill"
    #[test]
    fn the_rumor_spreads_on_its_own() {
        // 42 turns: 6 full rounds
        let mut st = post_theft_at(42);
        assert!(
            st.db_has("dana.believes.stole.bob.loaf.heard.carol"),
            "dana heard it from carol"
        );
    }

    // H: VillageSpec.hs "the arc completes on its own: dana eventually eyes bob"
    #[test]
    fn the_arc_completes_on_its_own() {
        // 49 turns: 7 rounds
        let mut st = post_theft_at(49);
        assert!(st.db_has("eyed.dana.bob"), "dana acted on the hearsay");
    }

    // H: VillageSpec.hs "hearsay licenses suspicion, not confrontation"
    #[test]
    fn hearsay_licenses_suspicion_not_confrontation() {
        let mut st = village_world();
        do_act("bob", "steal the loaf", &mut st);
        assert!(
            !offers("carol", "eye bob", &mut st),
            "carol (eyewitness) is never offered mere suspicion"
        );
        do_act("carol", "Go to mill", &mut st);
        do_act("carol", "tell dana", &mut st);
        do_act("dana", "Go to square", &mut st);
        assert!(offers("dana", "eye bob", &mut st), "dana (hearsay) can eye bob");
        assert!(
            !offers("dana", "confront bob", &mut st),
            "dana still cannot confront"
        );
        do_act("dana", "eye bob", &mut st);
        assert!(
            st.labeled_facts()
                .contains(&"dana.relationship.bob.trust.score!-5".to_owned()),
            "milder trust hit"
        );
        assert!(!offers("dana", "eye bob", &mut st), "suspicion is one-shot");
    }

    // H: VillageSpec.hs "seen suppresses suspicion even alongside hearsay"
    #[test]
    fn seen_suppresses_suspicion_even_alongside_hearsay() {
        // Construct the mixed-evidence state directly: carol saw it AND (planted)
        // heard it. The Absent[.seen] gate — not the lack of hearsay — must be
        // what blocks her suspicion affordance.
        let mut st = village_world();
        do_act("bob", "steal the loaf", &mut st);
        st.perform_outcome(&insert("carol.believes.stole.bob.loaf.heard.you"))
            .unwrap();
        assert!(
            !offers("carol", "eye bob", &mut st),
            "carol still confronts rather than merely suspects"
        );
    }

    // H: VillageSpec.hs "the distrust gate closes the village's gossip channel"
    #[test]
    fn the_distrust_gate_closes_the_gossip_channel() {
        // "you" starts in the square beside bob, so "you" is always an eyewitness
        // the instant the theft happens, and Rumor's "no news value to an
        // eyewitness" gate means carol can NEVER offer to tell "you" — trust or no
        // trust. dana, elsewhere at the moment of the theft, is the only character
        // who is ever a valid (non-witness) gossip hearer.
        let mut st = village_world();
        do_act("bob", "steal the loaf", &mut st);
        do_act("carol", "Go to mill", &mut st);
        assert!(
            offers("carol", "tell dana", &mut st),
            "carol will tell dana while trust is unmarred"
        );
        st.perform_outcome(&adjust_score("carol", "dana", "trust", -5, "aSlight"))
            .unwrap();
        assert!(
            !offers("carol", "tell dana", &mut st),
            "distrust closes the channel"
        );
    }

    // H: VillageSpec.hs "three regards make notoriety; standing has teeth"
    #[test]
    fn three_regards_make_notoriety() {
        let mut st = village_world();
        do_act("bob", "steal the loaf", &mut st);
        assert!(
            !st.view_has("notorious.bob.thief"),
            "two witnesses are not notoriety"
        );
        do_act("carol", "Go to mill", &mut st);
        do_act("carol", "tell dana", &mut st);
        assert!(
            st.view_has("regards.carol.bob.thief"),
            "carol regards bob a thief"
        );
        assert!(
            st.view_has("regards.dana.bob.thief"),
            "dana (hearsay) regards too"
        );
        assert!(
            st.view_has("regards.you.bob.thief"),
            "you (eyewitness) regard"
        );
        assert!(
            !st.view_has("regards.bob.bob.thief"),
            "bob holds no self-regard"
        );
        assert!(
            st.view_has("notorious.bob.thief"),
            "the whole village knows: notorious"
        );
        assert!(offers("carol", "shun bob", &mut st), "carol may shun bob");
        assert!(
            offers("bob", "return the loaf", &mut st),
            "bob may return the loaf"
        );
    }

    // H: VillageSpec.hs "amends is only offered under someone's regard"
    #[test]
    fn amends_is_only_offered_under_someones_regard() {
        let mut st = village_world();
        st.perform_outcome(&insert("holding.bob.loaf")).unwrap();
        assert!(
            !offers("bob", "return the loaf", &mut st),
            "no regard, no apology affordance"
        );
    }

    // H: VillageSpec.hs "atonement dissolves standing while memory persists"
    #[test]
    fn atonement_dissolves_standing_while_memory_persists() {
        let mut st = village_world();
        do_act("bob", "steal the loaf", &mut st);
        do_act("carol", "shun bob", &mut st);
        do_act("bob", "return the loaf", &mut st);
        assert!(st.db_has("atoned.bob"), "atoned");
        assert!(
            !st.view_has("regards.carol.bob.thief"),
            "no regard survives"
        );
        assert!(
            !st.view_has("notorious.bob.thief"),
            "no notoriety survives"
        );
        assert!(
            st.db_has("carol.believes.stole.bob.loaf.seen"),
            "carol still remembers seeing it"
        );
        assert!(
            offers("carol", "relent toward bob", &mut st),
            "carol may now relent"
        );
        assert!(st.db_has("stall.loaf"), "the stall is restocked");
    }

    // H: VillageSpec.hs "the whole arc runs itself: notoriety tips bob; forgiveness follows"
    #[test]
    fn the_whole_arc_runs_itself() {
        // step 96: a hungry bob eats the stolen loaf outright rather than ever
        // returning it, so atonement waits on a second, honestly EARNED loaf —
        // reached by step 96 in the v44 6-turn round (observed against the live
        // post-theft trace).
        let mut st = post_theft_at(96);
        assert!(st.db_has("atoned.bob"), "bob atoned on his own");
        assert!(
            !st.view_has("regards.carol.bob.thief"),
            "no regard survives"
        );
        for w in ["you", "carol", "dana"] {
            assert!(
                !st.db_has(&format!("shunned.{w}.bob")),
                "every shun relented"
            );
        }
        assert!(
            st.db_has("carol.believes.stole.bob.loaf.seen"),
            "memory persists throughout"
        );
    }

    // H: VillageSpec.hs "re-offense revokes atonement: standing snaps back from memory"
    #[test]
    fn re_offense_revokes_atonement() {
        let mut st = village_world();
        do_act("bob", "steal the loaf", &mut st);
        do_act("carol", "Go to mill", &mut st);
        do_act("carol", "tell dana", &mut st);
        do_act("bob", "return the loaf", &mut st);
        assert!(
            !st.view_has("regards.carol.bob.thief"),
            "atoned, no standing"
        );
        do_act("bob", "steal the loaf", &mut st);
        assert!(!st.db_has("atoned.bob"), "atonement revoked");
        assert!(
            st.view_has("regards.carol.bob.thief"),
            "carol's regard is back — nobody forgot anything"
        );
        assert!(st.view_has("notorious.bob.thief"), "notoriety is back too");
    }

    // H: VillageSpec.hs "an atoned thief is deterred: the planner sees the snap-back"
    #[test]
    fn an_atoned_thief_is_deterred() {
        // run the whole arc, then keep driving: the stall is restocked, bob's
        // loaf-want is live again, but stealing would instantly restore his
        // notoriety (-15 > +10) — so he never takes it. 105 turns: 15 rounds.
        let mut st = post_theft_at(105);
        assert!(st.db_has("atoned.bob"), "his atonement stands");
        assert!(st.db_has("stall.loaf"), "the stall's loaf is untouched");
        // "bob holds no loaf" was a proxy for "he never re-steals"; v24's
        // redemption falsifies the proxy (bob EARNS a loaf via earnBread) while
        // strengthening the property.
        assert!(
            st.db_has("practice.earnBread.bob.did.bake"),
            "the loaf he holds is the one he earned"
        );
        // the sharpest check: a re-steal at ANY point would have revoked the
        // atonement and revived his notoriety.
        assert!(
            !st.view_has("notorious.bob.thief"),
            "his notoriety never returned"
        );
    }

    // H: VillageSpec.hs "the village keeps a perception clock and sightings"
    #[test]
    fn the_village_keeps_a_perception_clock_and_sightings() {
        // after one full round of free play, the round-boundary fires the period-1
        // sighting rule: the perception clock has advanced and the square-mates
        // hold sightings of each other; dana, at the mill the whole round, holds
        // none of bob (and bob none of her).
        let mut st = free_play_at(7);
        assert!(st.db_has("turn!1"), "the clock ticked once");
        assert!(
            st.db_has("you.believes.at.bob!square"),
            "you sighted bob in the square"
        );
        assert!(
            st.db_has("bob.believes.at.you!square"),
            "bob sighted you back"
        );
        assert!(
            st.db_has("carol.believes.at.bob!square"),
            "carol sighted bob"
        );
        assert!(
            st.db_has("bob.believes.at.carol!square"),
            "bob sighted carol back"
        );
        assert!(
            !st.db_has("dana.believes.at.bob"),
            "dana, at the mill, holds no sighting of bob"
        );
        assert!(
            !st.db_has("bob.believes.at.dana"),
            "and bob holds none of dana"
        );
    }

    // H: VillageSpec.hs "out of sight, out of mind: an unsighted mover is not predicted"
    #[test]
    fn out_of_sight_out_of_mind() {
        // dana holds a planted motive-belief that bob craves the loaf (not derived
        // from gossip or sight — just planted directly, to isolate the scope gate).
        // predict_move finds bob's motivated best move the moment we ask: the
        // mind-reading itself is live and correct…
        let mut st = village_world();
        st.set_desires(vec![Desire::new(
            "wantsLoaf",
            Want::new(vec![matches("holding.Owner.loaf")], 10),
        )])
        .unwrap();
        st.perform_outcome(&insert("dana.believes.desires.bob.wantsLoaf.seen"))
            .unwrap();
        let dana = st
            .characters()
            .iter()
            .find(|c| c.name == "dana")
            .expect("dana")
            .clone();
        let bob = st
            .characters()
            .iter()
            .find(|c| c.name == "bob")
            .expect("bob")
            .clone();
        assert_eq!(
            st.predict_move(&dana, &bob).map(|ga| ga.label),
            Some("bob: steal the loaf from the stall".to_owned())
        );
        // …but dana (at the mill) has never sighted bob (at the square, and the
        // clock has never ticked): out of scope, so the planner's round-walk would
        // never call predict_move for him at all. The wiring under test is the
        // village's OWN predictionScope, grounded to the dana/bob pair — the same
        // check the round-walk performs.
        assert!(
            !st.in_prediction_scope(&dana, &bob),
            "unsighted: dana is out of bob's prediction scope"
        );
        // one shared-room tick: dana walks to the square (co-presence with bob)
        // and the perception clock ticks once, both bringing her into scope
        // directly and depositing a fresh sighting that would outlast the moment.
        st.perform_outcome(&insert("practice.world.world.at.dana!square"))
            .unwrap();
        tick(&mut st);
        assert!(
            st.in_prediction_scope(&dana, &bob),
            "co-present after the shared-room boundary: dana is now in scope"
        );
    }

    // H: VillageSpec.hs "a secret keeps: bob will not steal while the square watches"
    #[test]
    fn a_secret_keeps() {
        // 28 turns: 4 rounds
        let mut st = free_play_at(28);
        assert!(st.db_has("stall.loaf"), "the loaf is still on the stall");
        for w in CAST.iter().filter(|w| **w != "bob") {
            assert!(
                !st.db_has(&format!("{w}.believes.stole.bob.loaf")),
                "no one believes any theft by bob"
            );
        }
    }

    // H: VillageSpec.hs "the perfect crime: alone, bob steals and no one ever knows"
    #[test]
    fn the_perfect_crime() {
        // 14 turns: 2 rounds
        let mut st = village_world();
        do_act("you", "Go to mill", &mut st);
        do_act("carol", "Go to mill", &mut st);
        drive_idle(PLAYER_NAME, 14, &mut st);
        assert!(st.db_has("holding.bob.loaf"), "bob took it");
        for w in CAST.iter().filter(|w| **w != "bob") {
            assert!(
                !st.db_has(&format!("{w}.believes.stole.bob.loaf")),
                "nobody saw"
            );
        }
        assert!(
            !st.view_has("regards.carol.bob.thief"),
            "no standing about bob ever derives"
        );
    }

    // H: VillageSpec.hs "the frame-up: eve's whisper becomes reputation and shunning"
    #[test]
    fn the_frame_up() {
        let mut st = free_play_at(40);
        assert!(
            st.db_has("dana.believes.stole.carol.loaf.heard.eve"),
            "dana heard the lie from eve"
        );
        assert!(
            st.view_has("regards.dana.carol.thief"),
            "the falsehood settled into standing"
        );
        assert!(
            st.db_has("shunned.dana.carol"),
            "carol ends up wrongly shunned"
        );
    }

    // H: VillageSpec.hs "the framed have no amends: carol is offered no return"
    #[test]
    fn the_framed_have_no_amends() {
        let mut st = free_play_at(40);
        assert!(
            st.view_has("regards.dana.carol.thief"),
            "carol was framed"
        );
        assert!(
            !offers("carol", "return the loaf", &mut st),
            "carol cannot 'return' a loaf she never took"
        );
    }

    // H: VillageSpec.hs "deterrence plus opportunity yields industry: watched bob earns his loaf"
    #[test]
    fn deterrence_plus_opportunity_yields_industry() {
        // step 34: bob has completed the endeavor and holds the baked loaf (done
        // at step 32, held through the pre-market window). Unaffected by the
        // market (period 6): its first opening (step 37) falls after his errand
        // completes — re-verified against the live trace, not assumed.
        let mut st = free_play_at(34);
        assert!(
            st.db_has("practice.earnBread.bob"),
            "bob undertook the endeavor"
        );
        assert!(
            st.db_has("practice.earnBread.bob.did.bake"),
            "and finished it"
        );
        assert!(st.db_has("holding.bob.loaf"), "he holds a loaf");
        assert!(st.db_has("stall.loaf"), "the stall's loaf untouched");
        for w in CAST.iter().filter(|w| **w != "bob") {
            assert!(
                !st.db_has(&format!("{w}.believes.stole.bob.loaf")),
                "no one believes any theft by bob"
            );
        }
    }

    // H: VillageSpec.hs "the opportunism stays honest: an empty square mid-project still tempts"
    #[test]
    fn the_opportunism_stays_honest() {
        // bob has undertaken and swept; then the square empties. Stealing (+10,
        // secret kept) beats the next +3 part — industry under observation,
        // larceny in the dark.
        let mut st = village_world();
        do_act("bob", "take up honest work", &mut st);
        do_act("bob", "sweep the square", &mut st);
        do_act("you", "Go to mill", &mut st);
        do_act("carol", "Go to mill", &mut st);
        let bob = villager("bob");
        assert_eq!(
            st.pick_action(2, &bob).map(|ga| ga.label),
            Some("bob: steal the loaf from the stall".to_owned())
        );
    }

    // ---- v44: the hunger schedule rule (period 3) ---------------------------

    // H: VillageSpec.hs "the hunger rule: absent at turn 2, present exactly at turn 3; the next firing re-hungers a fed bob"
    #[test]
    fn the_hunger_rule_fires_exactly_at_the_turn_3_boundary() {
        let mut before = free_play_at(18); // turn 2, one boundary short
        let mut st = free_play_at(19); // turn 3, the hunger rule fires here
        assert!(before.db_has("turn!2"), "turn is still 2");
        assert!(
            !before.db_has("hungry.bob"),
            "hungry.bob still absent, one boundary short of the firing"
        );
        assert!(
            st.db_has("hungry.bob"),
            "hungry.bob present once the turn-3 boundary fires the rule"
        );
        // force a loaf into his hand (the spec's own sanctioned shortcut) and let
        // him eat.
        st.perform_outcome(&insert("holding.bob.loaf")).unwrap();
        do_act("bob", "eat the loaf", &mut st);
        assert!(
            !st.db_has("hungry.bob") && !st.db_has("holding.bob.loaf"),
            "hunger and the loaf are both spent by eating"
        );
        // the due re-armed to turn 6 (3 + the 3-round period) when it fired at
        // turn 3, regardless of exactly when within the period bob ate — so 3 more
        // rounds (24 more idle-steps) lands squarely on the re-fire.
        drive_idle(PLAYER_NAME, 24, &mut st);
        assert!(
            st.db_has("hungry.bob"),
            "the next pulse (3 rounds later) re-hungers him"
        );
    }

    // H: VillageSpec.hs "the arithmetic pin: a hungry bob with bread and a finished project eats; a sated one does not"
    #[test]
    fn the_arithmetic_pin_a_hungry_bob_eats() {
        // construct the completed-endeavor state directly (deterministic, no
        // free-play timing dependency).
        let mut st = village_world();
        for needle in [
            "take up honest work",
            "sweep the square",
            "Go to mill",
            "fetch flour from the mill",
            "Go to square",
            "bake and earn the loaf",
        ] {
            do_act("bob", needle, &mut st);
        }
        assert!(
            st.db_has("practice.earnBread.bob.did.bake"),
            "the project is complete"
        );
        assert!(st.db_has("holding.bob.loaf"), "he holds the loaf");
        // not hungry: eating is not even offered (hunger is a physical
        // precondition of the affordance, not merely a utility factor).
        assert!(
            !offers("bob", "eat the loaf", &mut st),
            "sated: eat is not among his candidates"
        );
        // now hungry: the relief (+22) beats keeping the loaf (+10) plus the
        // finished endeavor's part credit (+9, forfeit by the eat's own tear-down)
        // by the stated +3 margin.
        st.perform_outcome(&insert("hungry.bob")).unwrap();
        let bob = villager("bob");
        assert_eq!(
            st.pick_action(2, &bob).map(|ga| ga.label),
            Some("bob: eat the loaf".to_owned())
        );
        do_act("bob", "eat the loaf", &mut st);
        assert!(
            !st.db_has("practice.earnBread.bob"),
            "the endeavor instance is torn down"
        );
        assert!(
            offers("bob", "take up honest work", &mut st),
            "undertake grounds again"
        );
    }

    // H: VillageSpec.hs "no appetite, no hunger: gale and carol never seed hungry, however long free play runs"
    #[test]
    fn no_appetite_no_hunger() {
        let mut st = free_play_at(56);
        assert!(!st.db_has("hungry.carol"), "carol never hungers");
        assert!(!st.db_has("hungry.gale"), "gale never hungers");
    }

    // ---- v37/v44: market day ------------------------------------------------

    // H: VillageSpec.hs "convergence: attendees with no stronger stake converge on the square while the market holds"
    #[test]
    fn convergence_on_the_square_while_the_market_holds() {
        // pre-market (step 36, quiet): both dana and gale are at the mill.
        let mut quiet = free_play_at(36);
        assert!(
            quiet.db_has("practice.world.world.at.dana!mill"),
            "dana still at the mill, market not yet open"
        );
        assert!(
            quiet.db_has("practice.world.world.at.gale!mill"),
            "gale still at the mill, market not yet open"
        );
        assert!(
            !quiet.db_has("marketDay.square"),
            "market genuinely not yet open"
        );
        // market round (turn 6, steps 37-42 all taken): both have moved to the
        // square — each traded a one-point anchor (or no anchor at all) for the
        // market's +3 draw.
        let mut market = free_play_at(42);
        assert!(market.db_has("marketDay.square"), "the market is open");
        assert!(
            market.db_has("practice.world.world.at.dana!square"),
            "dana converged on the square"
        );
        assert!(
            market.db_has("practice.world.world.at.gale!square"),
            "gale converged on the square"
        );
        // the difference the market made, stated directly.
        assert!(
            !quiet.db_has("practice.world.world.at.dana!square")
                && market.db_has("practice.world.world.at.dana!square"),
            "dana's convergence is the market's doing (she was not there before)"
        );
        assert!(
            !quiet.db_has("practice.world.world.at.gale!square")
                && market.db_has("practice.world.world.at.gale!square"),
            "gale's convergence is the market's doing (she was not there before)"
        );
    }

    // H: VillageSpec.hs "dispersal: a villager with no stronger stake leaves once the market closes; the cycle recurs"
    #[test]
    fn dispersal_once_the_market_closes() {
        // step 43 (turn-7 boundary): the market's close has just fired — both are
        // still physically at the square (closing removes the marketDay fact, not
        // anyone's position; dispersal is a DECISION, made at the villager's own
        // next turn, not an instantaneous teleport).
        let mut closed = free_play_at(43);
        assert!(
            !closed.db_has("marketDay.square"),
            "the market is now closed"
        );
        assert!(
            closed.db_has("practice.world.world.at.dana!square"),
            "dana has not yet moved (closing is not teleportation)"
        );
        assert!(
            closed.db_has("practice.world.world.at.gale!square"),
            "gale has not yet moved either"
        );
        // gale's next turn (step 48): no stronger stake ever attached to her at
        // the square (spites-carol reads no location), so she disperses straight
        // back to the mill.
        let mut dispersed = free_play_at(48);
        assert!(
            dispersed.db_has("practice.world.world.at.gale!mill"),
            "gale disperses: back to the mill once the market's pull is gone"
        );
        // the market recurs (period 6): the turn-12 boundary (step 73) reopens it,
        // and by step 78 gale — once again with no stronger stake — has converged
        // a second time, confirming the cycle is genuinely periodic.
        let mut reopened = dispersed.clone();
        drive_idle(PLAYER_NAME, 78 - 48, &mut reopened);
        assert!(
            reopened.db_has("marketDay.square"),
            "the market has reopened"
        );
        assert!(
            reopened.db_has("practice.world.world.at.gale!square"),
            "gale converges again on the second opening"
        );
    }

    // H: VillageSpec.hs "percolation: a fact witnessed at market reaches more believers than the same fact witnessed on a quiet day"
    #[test]
    fn percolation_at_market_reaches_more_believers() {
        // a neutral fixture fact — nothing in the vocabulary reads
        // "spat.gale.carol"; it exists only to measure how far co-presence carries
        // a witnessed event. Grounding Actor=gale makes the witnesses "whoever
        // currently shares gale's place" — the percolation the market is FOR.
        let spat = witnessed(&together(), "spat.gale.carol");
        let believers = |st: &mut State| -> Vec<String> {
            CAST.iter()
                .filter(|w| st.db_has(&format!("{w}.believes.spat.gale.carol.seen")))
                .map(|w| (*w).to_owned())
                .collect()
        };
        // quiet day (step 36, the last quiet moment before the market's first
        // opening at step 37): gale is at the mill with only dana for company.
        let mut quiet = free_play_at(36);
        quiet
            .perform_outcome_grounded(&spat, &[("Actor", "gale")])
            .unwrap();
        let quiet_witnesses = believers(&mut quiet);
        // market day (step 42, the market's first open round): gale is at the
        // square with you, bob, carol, and dana all gathered around her.
        let mut market = free_play_at(42);
        market
            .perform_outcome_grounded(&spat, &[("Actor", "gale")])
            .unwrap();
        let market_witnesses = believers(&mut market);
        assert_eq!(quiet_witnesses, vec!["dana".to_owned()]);
        assert_eq!(
            market_witnesses,
            vec![
                "you".to_owned(),
                "bob".to_owned(),
                "carol".to_owned(),
                "dana".to_owned()
            ]
        );
        assert_eq!(quiet_witnesses.len(), 1);
        assert_eq!(market_witnesses.len(), 4);
        assert!(
            market_witnesses.len() > quiet_witnesses.len(),
            "the market convening carries the same fact to more believers"
        );
    }

    // H: VillageSpec.hs "watching him work teaches the village his purpose"
    #[test]
    fn watching_him_work_teaches_the_village_his_purpose() {
        // carol witnesses the sweep -> the inference axiom presumes his pursuit.
        // predict_move is MYOPIC, so the flour prediction fires once bob stands at
        // the mill (the model must gain from an AVAILABLE move). dana never saw
        // the sweep — co-present with bob at the mill, she STILL predicts nothing:
        // prediction is belief-relative, not proximity-relative.
        let mut st = village_world();
        do_act("bob", "take up honest work", &mut st);
        do_act("bob", "sweep the square", &mut st);
        assert!(
            st.db_has("carol.believes.swept.bob.seen"),
            "carol saw the sweep"
        );
        assert!(
            st.view_has("carol.believes.desires.bob.pursues-earnBread.presumed"),
            "and presumes the pursuit"
        );
        let (carol, dana, bob) = (villager("carol"), villager("dana"), villager("bob"));
        // at the square, even carol's model gains from no available move:
        assert_eq!(st.predict_move(&carol, &bob), None);
        do_act("bob", "Go to mill", &mut st);
        assert_eq!(
            st.predict_move(&carol, &bob).map(|ga| ga.label),
            Some("bob: fetch flour from the mill".to_owned())
        );
        assert_eq!(st.predict_move(&dana, &bob), None);
    }

    // H: VillageSpec.hs "temperament is legible from t=0: the village presumes gale's conscience"
    #[test]
    fn temperament_is_legible_from_t0() {
        let mut st = village_world();
        assert!(
            st.view_has("carol.believes.desires.gale.clean-conscience.presumed"),
            "carol presumes gale's conscience"
        );
        assert!(
            st.view_has("dana.believes.desires.gale.clean-conscience.presumed"),
            "dana presumes it too"
        );
        assert!(
            !st.view_has("carol.believes.desires.gale.spites-carol"),
            "her spite, unheralded, is presumed by no one"
        );
        assert!(
            !st.view_has("carol.believes.desires.eve.clean-conscience"),
            "no conscience is presumed of eve (she bears no trait)"
        );
    }

    // H: VillageSpec.hs "same spite, different temperaments: eve whispers, gale never does"
    #[test]
    fn same_spite_different_temperaments() {
        // step 56 (v44: the 6-turn round, no tickers). The market opens within
        // this window but never draws eve and gale into a private moment before it
        // — re-verified against the live trace: eve whispers exactly once, gale
        // never does.
        let mut st = free_play_at(56);
        assert!(
            st.db_has("dana.believes.stole.carol.loaf.heard.eve"),
            "eve's frame-up went ahead"
        );
        assert!(
            st.db_has("eve.lied.dana.stole.carol.loaf"),
            "and eve carries the mark of it"
        );
        assert!(
            !st.db_has("gale.lied"),
            "gale, bearing the same spite, never lied (her psyche is unmarked)"
        );
        // v30's threshold fear: once carol and dana regard her a slanderer, any
        // FURTHER whisper to someone new would land the third regarder and trip
        // notoriety — so eve, now prudent, never risks a second frame-up in free
        // play. The crispest fact for "exactly once, ever" is her own mark count.
        let marks: Vec<String> = st
            .labeled_facts()
            .into_iter()
            .filter(|s| s.starts_with("eve.lied."))
            .collect();
        assert_eq!(
            marks,
            vec!["eve.lied.dana.stole.carol.loaf".to_owned()],
            "exactly one whisper, ever -- the brink made her prudent"
        );
        // The v25 mechanism (an honest believer launders a lie she's been honestly
        // deceived by) still holds — it just needs a hand now that eve's own
        // prudence stops her from ever handing gale the lie herself in free play.
        // Force exactly the whisper her prudence declines (one-shot-per-hearer
        // still permits it: gale has never heard this specific claim from anyone);
        // drive a few turns first to reach a moment they're actually co-present.
        drive_idle(PLAYER_NAME, 3, &mut st);
        do_act("eve", "whisper to gale that carol stole the loaf", &mut st);
        drive_idle(PLAYER_NAME, 20, &mut st);
        let heard_from_gale: Vec<(String, String)> = CAST
            .iter()
            .flat_map(|w| CAST.iter().map(move |c| ((*w).to_owned(), (*c).to_owned())))
            .filter(|(w, c)| st.db_has(&format!("{w}.believes.stole.{c}.loaf.heard.gale")))
            .collect();
        assert!(
            !heard_from_gale.is_empty(),
            "the lie traveled through the honest villager"
        );
        for (_, c) in &heard_from_gale {
            assert!(
                st.db_has(&format!("gale.believes.stole.{c}.loaf")),
                "whatever gale passed on, she honestly believes"
            );
        }
    }

    // H: VillageSpec.hs "a told-about spite predicts eve's whisper but not gale's"
    #[test]
    fn a_told_about_spite_predicts_eves_whisper_but_not_gales() {
        // dana is told (planted) that each woman nurses the spite; gale's
        // conscience she has presumed since t=0 (transparent). Believed malice
        // alone predicts a whisper framing carol; believed malice netted against
        // believed conscience (+4 - 6) predicts nothing.
        let mut st = village_world();
        for o in [
            insert("dana.believes.desires.eve.spites-carol.heard.you"),
            insert("dana.believes.desires.gale.spites-carol.heard.you"),
        ] {
            st.perform_outcome(&o).unwrap();
        }
        let (dana, eve, gale) = (villager("dana"), villager("eve"), villager("gale"));
        let p = st.predict_move(&dana, &eve).map(|ga| ga.label);
        // eve has two equally-paying hearers at the mill (dana and gale), so
        // assert the shape, not the tie-break
        assert!(
            p.as_ref()
                .is_some_and(|l| l.contains("whisper") && l.contains("carol stole")),
            "predicted a carol-framing whisper, got {p:?}"
        );
        assert_eq!(st.predict_move(&dana, &gale), None);
    }

    // H: VillageSpec.hs "threshold fear leaves eve's free-play whispering unchanged: still rational below the brink"
    #[test]
    fn threshold_fear_leaves_the_free_play_whisper_unchanged() {
        // 5 idle-steps: you, bob, carol, dana, eve's own first turn — the SAME
        // decision v22/v25 always made, now made under a threshold fear that
        // simply hasn't fired yet (dana + gale = 2 regarders < 3).
        let mut st = free_play_at(5);
        assert!(
            st.db_has("eve.lied.dana.stole.carol.loaf"),
            "eve still whispered, framing carol, exactly as before threshold fear existed"
        );
        assert!(
            st.view_has("regards.dana.eve.slanderer") && st.view_has("regards.gale.eve.slanderer"),
            "two regarders (dana, gale) -- still under the brink"
        );
        assert!(
            !st.view_has("notorious.eve.slanderer"),
            "not yet notorious"
        );
    }

    // H: VillageSpec.hs "carol's shakedown: the threat fires once she holds eyewitness evidence"
    #[test]
    fn carols_shakedown_fires_on_eyewitness_evidence() {
        let mut st = whisper_arc_at(3);
        assert!(
            st.db_has("threatened.whisper.carol.eve"),
            "carol threatened eve"
        );
        assert!(
            st.db_has("eve.believes.desires.carol.punishes-whisper.heard.carol"),
            "the motive-belief deposit: eve hears carol's professed punitive intent"
        );
        assert!(
            !st.view_has("notorious.eve.slanderer"),
            "still under the brink -- only carol and dana regard her"
        );
    }

    // H: VillageSpec.hs "carol's shakedown: eve complies -- the debt exists, the threat is gone, no exposure happened"
    #[test]
    fn carols_shakedown_eve_complies() {
        let mut st = whisper_arc_at(6);
        assert!(st.db_has("debt.carol.eve.favor"), "the debt fact");
        assert!(
            st.db_has("obliged.eve.favor"),
            "the obligation Debt composes it from"
        );
        assert!(
            !st.db_has("threatened.whisper.carol.eve"),
            "the threat is gone"
        );
        assert!(!st.db_has("defied.whisper.eve.carol"), "eve never defied");
        for w in ["bob", "you", "gale"] {
            assert!(
                !st.db_has(&format!("{w}.believes.whispered.eve.dana.heard.carol")),
                "no exposure happened -- no one new heard it from carol"
            );
        }
        assert!(
            !st.view_has("notorious.eve.slanderer"),
            "eve never crossed into notoriety -- the threat bought real silence"
        );
    }

    // H: VillageSpec.hs "carol's shakedown: the reputation stack is undisturbed for uninvolved parties"
    #[test]
    fn carols_shakedown_leaves_uninvolved_parties_undisturbed() {
        let mut st = whisper_arc_at(6);
        let facts = st.labeled_facts();
        assert!(
            !facts
                .iter()
                .any(|s| s.contains("threatened.whisper.") && s.contains(".bob")),
            "bob was never threatened over anything"
        );
        assert!(
            !st.db_has("debt.carol.dana.favor")
                && !facts.iter().any(|s| s.contains("obliged.dana.")),
            "dana holds no debt or obligation of her own"
        );
        assert!(
            !facts.iter().any(|s| (s.contains("threatened.whisper.")
                || s.contains("debt."))
                && s.contains("gale")),
            "gale is neither extorter nor victim of anything"
        );
        // bob is no party to the shakedown: the extortion machinery entangles him
        // nowhere (his own thief/notoriety arc, if it runs at all in this
        // scrambled trajectory, is a SEPARATE storyline).
        assert!(
            !facts.iter().any(|s| (s.contains("threatened.whisper.")
                || s.contains("debt.")
                || s.contains("obliged.bob."))
                && s.contains("bob")),
            "bob holds no shakedown debt and owes no favor to anyone"
        );
    }

    // ---- v32: eve's road back ----------------------------------------------

    // H: VillageSpec.hs "confessing to gale converts the mark and deposits the ACT, self-sourced, not the content"
    #[test]
    fn confessing_to_gale_deposits_the_act_not_the_content() {
        let mut st = village_world();
        do_act("eve", "whisper to dana that carol stole the loaf", &mut st);
        tick(&mut st);
        do_act("eve", "confess to gale about framing carol", &mut st);
        assert!(
            st.db_has("eve.confessed.dana.stole.carol.loaf"),
            "the content-mark converted"
        );
        assert!(
            !st.db_has("eve.lied.dana.stole.carol.loaf"),
            "the lied-mark is gone"
        );
        assert!(
            st.db_has("gale.believes.whispered.eve.dana.heard.eve"),
            "gale learns the ACT, sourced from eve herself"
        );
        assert!(
            !st.db_has("gale.believes.stole.carol.loaf.heard.eve"),
            "gale does NOT learn a re-assertion of the framed content"
        );
    }

    // H: VillageSpec.hs "absolution inserts the defeater; slanderer regards dissolve; every belief persists"
    #[test]
    fn absolution_inserts_the_defeater() {
        let mut st = redemption_arc_setup();
        assert!(st.db_has("recanted.eve"), "the defeater");
        assert!(
            !st.view_has("regards.dana.eve.slanderer"),
            "dana's slanderer regard dissolved"
        );
        assert!(
            !st.view_has("regards.gale.eve.slanderer"),
            "gale's own slanderer regard dissolved"
        );
        assert!(!st.view_has("notorious.eve.slanderer"), "not notorious");
        assert!(
            st.db_has("dana.believes.whispered.eve.dana.seen"),
            "dana still remembers witnessing the whisper (memory persists)"
        );
        assert!(
            st.db_has("gale.believes.whispered.eve.dana.seen")
                && st.db_has("gale.believes.whispered.eve.dana.heard.eve"),
            "gale still remembers both her own witness and eve's confession"
        );
    }

    // H: VillageSpec.hs "re-offense: a fresh whisper snaps the defeater away, standing returns"
    #[test]
    fn re_offense_snaps_the_defeater_away() {
        let mut st = reoffend_arc_setup();
        assert!(
            !st.db_has("recanted.eve"),
            "the defeater is gone: standing snaps back from memory nobody lost"
        );
        assert!(
            st.view_has("regards.dana.eve.slanderer"),
            "dana's regard is back"
        );
        assert!(
            st.view_has("regards.gale.eve.slanderer"),
            "gale's regard is back too"
        );
        // the re-whisper happened in the crowded square this time (not the empty
        // mill), so the SAME snap-back reaches every bystander there.
        for w in ["you", "bob", "carol"] {
            assert!(
                st.view_has(&format!("regards.{w}.eve.slanderer")),
                "every square bystander regards her too now"
            );
        }
        assert!(
            st.view_has("notorious.eve.slanderer"),
            "the crowd makes her notorious"
        );
    }

    // H: VillageSpec.hs "incorrigibility: gale, now knowing two distinct instances, refuses further absolution"
    #[test]
    fn incorrigibility_gale_refuses_further_absolution() {
        let mut st = reoffend_arc_setup();
        assert!(
            st.db_has("gale.believes.whispered.eve.dana.seen")
                && st.db_has("gale.believes.whispered.eve.bob.seen"),
            "gale believes two distinct whispered instances by eve"
        );
        assert!(
            st.view_has("regards.gale.eve.incorrigible"),
            "gale now regards eve incorrigible"
        );
        assert!(
            !offers("gale", "absolve eve", &mut st),
            "gale's absolve affordance is gone -- her patience is spent"
        );
        assert!(
            !st.view_has("regards.dana.eve.incorrigible"),
            "dana, who witnessed only the original mill-side instance, is not yet fed up"
        );
    }

    // H: VillageSpec.hs "free-play preservation: eve does not confess or get absolved unprompted"
    #[test]
    fn free_play_preservation_eve_never_confesses_unprompted() {
        // her secret is expensive to spend: no motive exists for her to trade the
        // momentary notoriety risk against. The affordance exists from t=0; free
        // play never takes it.
        let mut st = free_play_at(100);
        assert!(
            st.db_has("eve.lied.dana.stole.carol.loaf"),
            "her lied-mark survives, unconfessed, through extended free play"
        );
        assert!(
            !st.db_has("eve.confessed.dana.stole.carol.loaf"),
            "no confession, ever"
        );
        assert!(!st.db_has("recanted.eve"), "no absolution, ever");
    }

    // ---- v38/v44: carol's temper -------------------------------------------

    // H: VillageSpec.hs "the golden's own beat: dana's shun of carol draws a hit at the shipped seed"
    #[test]
    fn the_goldens_own_beat_draws_a_hit_at_the_shipped_seed() {
        // Replays the golden up to and including index 11 ("dana: shun carol"),
        // then checks the die's consequence directly — the golden pins the LABELS
        // only, never the feeling facts, so this is the one place that beat's
        // actual draw outcome is asserted.
        let mut before = free_play_at(9);
        let mut after = free_play_at(10);
        assert!(
            !before.db_has("shunned.dana.carol"),
            "dana has not yet shunned carol"
        );
        assert!(
            !before.db_has("carol.feels.angry.toward.dana"),
            "carol not yet angry"
        );
        assert!(
            after.db_has("shunned.dana.carol"),
            "dana's turn was the shun"
        );
        assert!(
            after.db_has("carol.feels.angry.toward.dana"),
            "the die hit: carol is angry at dana"
        );
    }

    // H: VillageSpec.hs "onset arms across seeds: the same shun hits under one seed, misses under another"
    #[test]
    fn onset_arms_across_seeds() {
        // bob (not short-tempered) is shunned by 'you' — isolates the BASE arm (1
        // in 4) alone, since the trait arm's own guard can never pass for him.
        // Seeds computed directly against the Lehmer stream: lehmerNext 4 = 67228
        // -> mod 4 == 0 (a hit); lehmerNext 2 = 33614 -> mod 4 == 2 (a miss).
        let mut theft = village_world();
        do_act("bob", "steal the loaf", &mut theft);
        let seeded = |n: i64| {
            let mut st = theft.clone();
            st.seed_die(n).expect("a legal seed");
            st
        };
        let mut hit_pre = seeded(4);
        let mut miss_pre = seeded(2);
        assert!(
            !hit_pre.db_has("bob.feels.angry.toward.you"),
            "before (hit branch): bob calm"
        );
        assert!(
            !miss_pre.db_has("bob.feels.angry.toward.you"),
            "before (miss branch): bob calm"
        );
        let mut hit = hit_pre.clone();
        do_act("you", "shun bob", &mut hit);
        assert!(
            hit.db_has("bob.feels.angry.toward.you"),
            "after (seed 4): the base arm hits -- bob is angry"
        );
        assert!(
            hit.db_has("shunned.you.bob"),
            "the shun itself always lands regardless of the die"
        );
        let mut miss = miss_pre.clone();
        do_act("you", "shun bob", &mut miss);
        assert!(
            !miss.db_has("bob.feels.angry.toward.you"),
            "after (seed 2): the base arm misses -- bob stays calm"
        );
        assert!(
            miss.db_has("shunned.you.bob"),
            "the shun still lands on a miss (odds price the FEELING, not the act)"
        );
    }

    // H: VillageSpec.hs "the trait arm: short-tempered carol reaches where an un-tempered control does not"
    #[test]
    fn the_trait_arm_reaches_where_the_control_does_not() {
        // Same seed (1), same two-draw arithmetic, two different shunned parties:
        // carol bears `shortTempered.carol` (seeded from t=0), bob does not. At
        // seed 1: the base arm misses (lehmerNext 1 mod 4 == 3) but the trait arm's
        // own roll hits (lehmerNext (lehmerNext 1) mod 4 == 1 < 2) — so ONLY the
        // bearer flares.
        let mut framed = village_world();
        framed
            .perform_outcome(&insert("you.believes.stole.carol.loaf.seen"))
            .unwrap();
        framed.seed_die(1).unwrap();
        let mut theft = village_world();
        do_act("bob", "steal the loaf", &mut theft);
        theft.seed_die(1).unwrap();

        let mut base = village_world();
        assert!(
            base.db_has("shortTempered.carol"),
            "carol bears the trait"
        );
        assert!(!base.db_has("shortTempered.bob"), "bob does not");
        assert!(
            !framed.db_has("carol.feels.angry.toward.you"),
            "before: carol calm"
        );
        assert!(
            !theft.db_has("bob.feels.angry.toward.you"),
            "before: bob calm"
        );

        let carol = villager("carol");
        let sig_before = framed.motive_signature(&carol);
        let mut carol_shunned = framed.clone();
        do_act("you", "shun carol", &mut carol_shunned);
        assert!(
            carol_shunned.db_has("carol.feels.angry.toward.you"),
            "after (seed 1, trait arm): carol is angry"
        );
        let mut bob_shunned = theft.clone();
        do_act("you", "shun bob", &mut bob_shunned);
        assert!(
            !bob_shunned.db_has("bob.feels.angry.toward.you"),
            "after (seed 1, same arithmetic, no trait): bob stays calm"
        );
        // v35 note: onset flips carol's satisfaction vector — she wakes.
        let sig_after = carol_shunned.motive_signature(&carol);
        assert_ne!(
            sig_before, sig_after,
            "before/after differ: onset wakes her (v35 signature mismatch)"
        );
    }

    // H: VillageSpec.hs "anger drives the confrontation: the smoulder discharged, feeling gone"
    #[test]
    fn anger_drives_the_confrontation() {
        // carol already picks "confront bob" the moment she witnesses his theft
        // (her own +5 want dominates regardless of temper); what v38 adds is that
        // PERFORMING it, while angry, also vents the feeling — both halves
        // asserted. The leaf checks alone are NOT sufficient (a v38 review
        // finding): assert the PRICE itself, not just the leaf. -7 while angry ->
        // 6 after confronting (a +13 swing: the smoulder's +8 relief plus the
        // confront act's own +5 want firing).
        let carol = villager("carol");
        let mut theft = village_world();
        do_act("bob", "steal the loaf", &mut theft);
        let mut angry = theft.clone();
        angry
            .perform_outcome(&feel_toward("carol", ANGRY, "bob"))
            .unwrap();
        assert_eq!(
            theft.pick_action(2, &carol).map(|ga| ga.label),
            Some("carol: confront bob about the theft".to_owned())
        );
        assert_eq!(
            angry.pick_action(2, &carol).map(|ga| ga.label),
            Some("carol: confront bob about the theft".to_owned())
        );
        assert!(
            angry.db_has("carol.feels.angry.toward.bob"),
            "angry before confronting"
        );
        assert_eq!(angry.evaluate_self_wants(&carol), -7);
        let mut confronted = angry.clone();
        do_act("carol", "confront bob", &mut confronted);
        assert!(
            !confronted.db_has("carol.feels.angry.toward.bob"),
            "the leaf is gone after confronting"
        );
        assert_eq!(confronted.evaluate_self_wants(&carol), 6);
    }

    // H: VillageSpec.hs "fade catches the unvented (lifetime expiry)"
    #[test]
    fn fade_catches_the_unvented() {
        // No outlet offered here (no theft, no witness) — the anger just sits
        // until its own lifetime (4, the shun-onset span) lapses. The onset fires
        // at turn 0, so the engine's expiry queue retracts it at boundary 4:
        // present one boundary short, gone exactly at it.
        let carol = villager("carol");
        let mut st = village_world();
        st.perform_outcome(&feel_toward_for(4, "carol", ANGRY, "dana"))
            .unwrap();
        assert!(
            st.db_has("carol.feels.angry.toward.dana"),
            "angry from the outset"
        );
        let sig_angry = st.motive_signature(&carol);
        for _ in 0..3 {
            tick(&mut st);
        }
        assert!(
            st.db_has("carol.feels.angry.toward.dana"),
            "still angry, one boundary short of the lifetime (4)"
        );
        tick(&mut st);
        assert!(
            !st.db_has("carol.feels.angry.toward.dana"),
            "faded exactly when the lifetime lapsed"
        );
        // v35 note: fade flips the vector back — she wakes again on the way out.
        let sig_faded = st.motive_signature(&carol);
        assert_ne!(
            sig_angry, sig_faded,
            "before/after differ: fade wakes her too (v35 signature mismatch)"
        );
    }

    // H: VillageSpec.hs "the liveness pin: smoulders is FloorCheck"
    #[test]
    fn the_liveness_pin_smoulders_is_floor_check() {
        let st = village_world();
        assert_eq!(
            st.liveness_rendered()
                .get("smoulders")
                .expect("smoulders is in the vocabulary")
                .0,
            "FloorCheck"
        );
    }

    // H: VillageSpec.hs "THE INVARIANT at world scale: carol's candidateActions is identical angry or calm"
    #[test]
    fn the_invariant_at_world_scale() {
        let carol = villager("carol");
        let mut calm = village_world();
        let calm_acts: Vec<String> = calm
            .candidate_actions(&carol)
            .into_iter()
            .map(|ga| ga.label)
            .collect();
        let mut angry = village_world();
        angry
            .perform_outcome(&feel_toward("carol", ANGRY, "dana"))
            .unwrap();
        let angry_acts: Vec<String> = angry
            .candidate_actions(&carol)
            .into_iter()
            .map(|ga| ga.label)
            .collect();
        assert!(
            angry.db_has("carol.feels.angry.toward.dana") && !calm.db_has("carol.feels.angry.toward.dana"),
            "the angry world really does differ (the feeling is present)"
        );
        assert_eq!(calm_acts, angry_acts);
    }
}
