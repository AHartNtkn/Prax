//! The six `owed: S7` rows whose SUBJECT is `villageWorld` — the last of the
//! stage's deferrals, discharged here as NATIVE Rust assertions over
//! [`prax_worlds::village::village_world`].
//!
//! Five are `RelevanceSpec` rows (the improvable table, the state field, delta
//! relevance, monotone-insert classification, and the liveness field); one is
//! `LoopSpec`'s v37-wake end-to-end. All six are removed from `conformance/KILLED.md`
//! as they land here.
//!
//! **They assert NAMED CONTENT, and that is the whole point** [S7 design D-I1].
//! `worldshape village` does NOT discharge them: a worldshape diff asserts only
//! equal-to-frozen, which would make the Haskell the contract (inverting the
//! program's authority), would evaporate at cut-over, and is the wrong net — a
//! misclassified floor that the frozen ALSO misclassifies passes a diff and fails
//! here. The worldshape table dump stays as an early net that localizes a table
//! divergence to shape-time; these are the contract.

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use prax_core::turn::{advance, npc_act};
    use prax_core::types::Desire;
    use prax_worlds::village::village_world;

    /// The liveness table's rendered variant tag for `name`.
    fn liveness_tag(
        tbl: &BTreeMap<String, (String, Vec<Vec<String>>)>,
        name: &str,
    ) -> (String, Vec<Vec<String>>) {
        tbl.get(name)
            .unwrap_or_else(|| panic!("no liveness entry for {name:?}; had {:?}", tbl.keys()))
            .clone()
    }

    // H: RelevanceSpec.hs "the village table: conscience live, spite and pursuit live"
    #[test]
    fn the_village_improvable_table_conscience_spite_and_pursuit() {
        let st = village_world();
        let tbl = st.improvables();
        // v32: confess's own outcome list Deletes exactly the "lied"-shaped mark
        // clean-conscience's condition matches (the village's confessWhisper
        // authors that delete) — a conscience-only believed model CAN now be
        // improved (predicting a confession relieves it), so the table correctly
        // flips from the pre-v32 "never improvable" finding.
        //
        // Performance consequence, recorded rather than papered over:
        // `predict_move`'s v26 pre-filter skips grounding/scoring a mover's
        // candidates entirely when EVERY desire in the predictor's believed model
        // is un-improvable. Gale's "honest" trait is presumed by every character
        // from t=0 (`transparent`), so a conscience-only model of gale used to hit
        // that skip for free. Now that clean-conscience (and its v32 sibling
        // conscience-remembers) are improvable, that skip no longer fires for her.
        assert!(
            tbl.contains(&"clean-conscience".to_owned()),
            "clean-conscience is improvable; table = {tbl:?}"
        );
        // spites-carol counts DERIVED regards facts (standingUnless's head):
        // conservatively improvable, so eve's predicted whisper stays live.
        assert!(
            tbl.contains(&"spites-carol".to_owned()),
            "spites-carol is improvable; table = {tbl:?}"
        );
        // pursuit counts base did-facts the part actions Insert.
        assert!(
            tbl.contains(&"pursues-earnBread".to_owned()),
            "pursues-earnBread is improvable; table = {tbl:?}"
        );
    }

    // H: RelevanceSpec.hs "the state carries the table and setDesires rebuilds it"
    #[test]
    fn the_state_carries_the_table_and_set_desires_rebuilds_it() {
        let mut st = village_world();
        let field = st.improvables().to_vec();
        assert_eq!(
            field,
            st.improvable_desires_recomputed(),
            "the village's cached improvables field must equal the module computation \
             over its own compiled tables — a difference is a stale rebuild"
        );
        let narrowed: Vec<Desire> = st
            .desires_src()
            .iter()
            .filter(|d| d.name == "spites-carol")
            .cloned()
            .collect();
        assert_eq!(narrowed.len(), 1, "spites-carol is in the vocabulary");
        st.set_desires(narrowed).expect("narrowing the vocabulary");
        assert!(
            !st.improvables().contains(&"pursues-earnBread".to_owned()),
            "narrowed vocabulary narrows the table; had {:?}",
            st.improvables()
        );
    }

    // H: RelevanceSpec.hs "delta relevance against the village's axioms"
    #[test]
    fn delta_relevance_against_the_villages_axioms() {
        let mut st = village_world();
        assert!(
            !st.relevant_delta("practice.world.world.at.bob!square")
                .unwrap(),
            "movement commutes with closure (fast path)"
        );
        assert!(
            st.relevant_delta("you.believes.stole.bob.loaf.seen").unwrap(),
            "a witness deposit is relevant (standingUnless reads believes)"
        );
        assert!(
            st.relevant_delta("atoned.bob").unwrap(),
            "an atonement is relevant (it defeats standing)"
        );
        assert!(
            !st.relevant_delta("stall.loaf").unwrap(),
            "the stall's stock is not"
        );
    }

    // H: RelevanceSpec.hs "monotone-insert classification against the village"
    #[test]
    fn monotone_insert_classification_against_the_village() {
        let mut st = village_world();
        assert!(
            st.cont_monotone(),
            "the village's axioms admit the continuation tier"
        );
        assert!(
            st.monotone_insert("you.believes.stole.bob.loaf.seen")
                .unwrap(),
            "a witness deposit grows monotonically"
        );
        assert!(
            !st.monotone_insert("atoned.bob").unwrap(),
            "atonement defeats standing: full reclose"
        );
        assert!(
            !st.monotone_insert("practice.world.world.at.bob!square")
                .unwrap(),
            "an exclusion insert never takes the continuation"
        );
    }

    // H: RelevanceSpec.hs "the village's liveness field: floors for consciences, classes for the rest"
    #[test]
    fn the_villages_liveness_field() {
        let mut st = village_world();
        let tbl = st.liveness_rendered();
        assert_eq!(liveness_tag(&tbl, "clean-conscience").0, "FloorCheck");
        assert_eq!(liveness_tag(&tbl, "conscience-remembers").0, "FloorCheck");
        // pursues-earnBread's condition is a did-fact every part action inserts
        // (practice.earnBread.Owner.did.P) — action-insertable, so no conjunct
        // qualifies as a gate.
        assert_eq!(liveness_tag(&tbl, "pursues-earnBread").0, "AlwaysLive");
        // spites-carol's condition (regards.W.carol.thief) is standingUnless's own
        // axiom head — conservatively excluded from gating.
        assert_eq!(liveness_tag(&tbl, "spites-carol").0, "AlwaysLive");
        // punishes-whisper's top-level conjuncts are an Or (never a gate
        // candidate) and a belief-Match that expose's own outcome inserts —
        // action-insertable, so again no qualifying conjunct remains.
        assert_eq!(liveness_tag(&tbl, "punishes-whisper").0, "AlwaysLive");
        // v36: suffers-hunger is a negative desire (-22) — FloorCheck
        // unconditionally, so a sated bob's pair-skip against it fires between
        // meals.
        assert_eq!(liveness_tag(&tbl, "suffers-hunger").0, "FloorCheck");
        // v37: drawn-to-market's first conjunct (marketDay.square) is clock-moved
        // only — no authored action ever inserts it, so it qualifies as the sole
        // gate; the second conjunct (practice.world.world.at.Owner!square) is
        // action-insertable ("Go to [Place]" inserts exactly this shape) and so
        // never qualifies — confirmed, not assumed, against the computed table.
        assert_eq!(
            liveness_tag(&tbl, "drawn-to-market"),
            (
                "GateCheck".to_owned(),
                vec![vec!["marketDay.square".to_owned()]]
            )
        );
        // The frozen case's closing clause: `liveness villageWorld == livenessOf
        // villageWorld`. The cached FIELD must equal the module COMPUTATION, or
        // the seven assertions above are pinning a stale table.
        assert_eq!(
            tbl,
            st.liveness_recomputed(),
            "the liveness field matches the module computation -- a difference \
             means the field went stale against its own rebuild"
        );
    }

    /// The frozen `idleStep`: one turn of free play with `you` sitting out —
    /// `VillageSpec`'s own `driveIdle`, which the v37-wake case drives the world
    /// with.
    fn idle_step(st: &mut prax_core::engine::State, idle: &str) {
        let actor = advance(st);
        if actor.name != idle {
            npc_act(st, 2, &actor);
        }
    }

    // H: LoopSpec.hs "the v37 wake, end to end: a standing intention holds through quiet rounds, the market's open flips the live-desire set and wakes fresh deliberation to the square, the close disperses it"
    #[test]
    fn the_v37_wake_end_to_end() {
        // gale, driven through the village's own free play (the v35+v37+v44
        // integration — the real village, not a fixture): her only wants are
        // spites-carol (no locational content) and drawn-to-market, so her
        // location choices are read straight off the market's own clock. v44: the
        // ticker characters are gone, so the village's round is 6 turns and the
        // engine fires the schedule at each round boundary (at the wrap). The
        // market (period 6) opens at the turn-6 boundary (step 37) and closes at
        // the turn-7 boundary (step 43) — both OBSERVED against the live trace,
        // not assumed.
        let mut st = village_world();
        let gale = st
            .characters()
            .iter()
            .find(|c| c.name == "gale")
            .expect("no such villager: gale")
            .clone();

        for _ in 0..36 {
            idle_step(&mut st, "you");
        }
        // QUIET (step 36, one boundary short of the open): her standing intention
        // still matches her current signature — the quiescence holds, and
        // drawn-to-market is not yet in her live set.
        let sig_quiet = st.motive_signature(&gale);
        let intent_quiet = st
            .intention_of("gale")
            .expect("gale holds no standing intention yet");
        assert_eq!(
            intent_quiet.basis, sig_quiet,
            "the standing intention holds through the quiet turn"
        );
        assert!(
            !sig_quiet.live_desires.contains(&"drawn-to-market".to_owned()),
            "drawn-to-market is not yet live; live = {:?}",
            sig_quiet.live_desires
        );

        // THE OPEN (step 37, the turn-6 boundary): the market's insert flips
        // drawn-to-market's gate live — the live-desire SET component of gale's
        // signature changes, asserted as before/after/diff.
        idle_step(&mut st, "you");
        let sig_open = st.motive_signature(&gale);
        assert!(st.db_has("marketDay.square"), "the market is open");
        assert!(
            sig_open.live_desires.contains(&"drawn-to-market".to_owned()),
            "drawn-to-market is now live; live = {:?}",
            sig_open.live_desires
        );
        assert_ne!(
            sig_quiet.live_desires, sig_open.live_desires,
            "the live-desire component actually changed (before /= after)"
        );
        // her prior intention is still the one on file (she has not acted since)
        // — and it no longer matches: the wake has fired.
        assert_ne!(
            st.intention_of("gale").expect("gale's intention").basis,
            sig_open,
            "her standing intention's basis no longer matches: she is woken"
        );

        // her NEXT turn within the open round (step 42) re-deliberates and picks
        // the square — the wake's CONSEQUENCE, not merely its trigger.
        for _ in 37..42 {
            idle_step(&mut st, "you");
        }
        assert_eq!(
            st.intention_of("gale")
                .expect("gale's intention")
                .act
                .map(|ga| ga.label),
            Some("gale: Go to square".to_owned())
        );

        // THE CLOSE (step 43, the turn-7 boundary): drawn-to-market's gate shuts
        // again — the live-desire set flips back, and her market-turn intention
        // (stored woken and live) no longer matches.
        idle_step(&mut st, "you");
        let sig_closed = st.motive_signature(&gale);
        assert!(
            !st.db_has("marketDay.square"),
            "the market is closed"
        );
        assert!(
            !sig_closed
                .live_desires
                .contains(&"drawn-to-market".to_owned()),
            "drawn-to-market is dead again; live = {:?}",
            sig_closed.live_desires
        );
        assert_ne!(
            sig_open.live_desires, sig_closed.live_desires,
            "the live-desire component changed back (open /= closed)"
        );
        assert_ne!(
            st.intention_of("gale").expect("gale's intention").basis,
            sig_closed,
            "her market-turn intention no longer matches: woken again, by the close"
        );

        // her next turn (step 48) disperses her back to the mill.
        for _ in 43..48 {
            idle_step(&mut st, "you");
        }
        assert_eq!(
            st.intention_of("gale")
                .expect("gale's intention")
                .act
                .map(|ga| ga.label),
            Some("gale: Go to mill".to_owned())
        );
    }
}
