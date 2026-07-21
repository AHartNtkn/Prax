//! `Prax.PersistSpec`, re-expressed as BEHAVIORAL pins over the Rust serde-JSON
//! save format ([`prax_core::persist`]).
//!
//! **There is NO persist differential** ([R4]): the Rust format is serde JSON
//! with its own version tag (`prax-rs-state v1`), not the frozen line-format
//! (`prax-state v4`) — nothing loads a cross-engine save, so reproducing the
//! frozen bytes would be pointless. The net is the Rust-internal round-trip law
//! plus the frozen `PersistSpec` re-expressed as behavior over this format.
//!
//! **The name domain, not the id domain.** Rust's per-state interners make a
//! direct cross-lineage `Sym`/`GroundedAction`/`CompiledPath` compare unsound
//! ([S-I1]): a value saved under one interner and reloaded under a fresh one
//! carries the SAME content but potentially different ids. So the frozen's
//! cross-lineage equalities (`intentions reloaded == intentions mid`, `reloAct
//! == contAct`) re-express in the name domain: `serialize` is by-name and
//! content-canonical, so `serialize(reload) == serialize(mid)` IS the exact
//! round-trip statement, and the end-to-end resume pin asserts the reuse
//! predicate WITHIN the reloaded lineage (sound), which is where the re-intern
//! hazard actually lives and must discharge.

#[cfg(test)]
mod tests {
    // H: PersistSpec.hs "Prax.Persist"
    //
    // The frozen `Prax.Persist` test group.
    use prax_core::engine::State;
    use prax_core::path::tokenize;
    use prax_core::persist::{FORMAT_VERSION, PersistError, deserialize_state, serialize_state};
    use prax_core::turn::{npc_act, run_npc_ticks};
    use prax_core::types::{Character, Outcome, ScheduleRule, insert, insert_for};
    use prax_worlds::intrigue::intrigue_world;

    /// A mid-episode intrigue state (Cassia has confided; Marcus now knows the
    /// plot) — the frozen `mid = snd (runNpcTicks 2 3 intrigueWorld)`.
    fn mid() -> State {
        let mut st = intrigue_world();
        run_npc_ticks(&mut st, 2, 3);
        st
    }

    /// The same state saved and reloaded onto a fresh copy of the world.
    fn reloaded() -> State {
        deserialize_state(&serialize_state(&mid()), intrigue_world()).expect("round trip")
    }

    fn character(st: &State, name: &str) -> Character {
        st.characters()
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("no {name} in world"))
            .clone()
    }

    // H: PersistSpec.hs "save/load round-trips the fact database and cursor exactly"
    #[test]
    fn round_trips_the_fact_database_and_cursor_exactly() {
        let mid = mid();
        let reloaded = deserialize_state(&serialize_state(&mid), intrigue_world()).unwrap();
        // labeled_facts is by name (interner-independent), so this compare is sound.
        assert_eq!(reloaded.labeled_facts(), mid.labeled_facts());
        assert_eq!(reloaded.cursor(), mid.cursor());
    }

    // H: PersistSpec.hs "save/load round-trips an asserted interior node with transient children (marks included)"
    #[test]
    fn round_trips_an_asserted_interior_node_with_transient_children() {
        // A spawned practice instance is an asserted fact that ALSO parents
        // transient children; the assertedness mark must survive serialization
        // or the instance reloads as mere scaffolding. Inject one raw (the frozen
        // `insertAll` onto the db, not a perform tier), round-trip, assert full
        // by-name db equality plus that the instance re-emits as its own fact.
        let mut with_instance = mid();
        with_instance.with_db(|interner, db| {
            let mut d = db.clone();
            for s in [
                "practice.tendBar.bar.ada",
                "practice.tendBar.bar.ada.customer.you",
            ] {
                d = d.insert(&tokenize(interner, s).expect("valid instance path"));
            }
            d
        });
        let reloaded_inst =
            deserialize_state(&serialize_state(&with_instance), intrigue_world()).unwrap();
        assert_eq!(reloaded_inst.labeled_facts(), with_instance.labeled_facts());
        assert!(
            reloaded_inst
                .labeled_facts()
                .contains(&"practice.tendBar.bar.ada".to_owned()),
            "the asserted instance re-emits as its own fact after reload"
        );
    }

    // H: PersistSpec.hs "save/load round-trips standing intentions exactly"
    #[test]
    fn round_trips_standing_intentions_exactly() {
        // "Exactly" in the name domain: serialize is by-name, so equal serialized
        // forms ARE equal content — the sound reading of the frozen `intentions
        // reloaded == intentions mid` under per-state interners.
        let mid = mid();
        let reloaded = deserialize_state(&serialize_state(&mid), intrigue_world()).unwrap();
        assert_eq!(serialize_state(&reloaded), serialize_state(&mid));
        assert!(
            !mid.intentions_map().is_empty(),
            "the mid state must actually carry standing intentions, or this pin is vacuous"
        );
    }

    // H: PersistSpec.hs "a reloaded standing intention is served without re-deliberating"
    #[test]
    fn a_reloaded_standing_intention_is_served_without_re_deliberating() {
        // Two halves, both after a FRESH interner (the [S-I1] re-intern hazard).
        //
        // (1) The frozen assertion, in the sound label domain: the reloaded state
        // makes the SAME decision as the original (`reloAct == contAct`, label ==
        // "bide your time"). Marcus RE-deliberates here — his stored basis
        // predates learning the plot — so this half proves decision-fidelity, not
        // reuse. Cross-lineage `GroundedAction` id-equality is unsound under
        // per-state interners, so the compare is by LABEL.
        let mut mid_clone = mid();
        let mut mid_reloaded = reloaded();
        let marcus = character(&mid_reloaded, "marcus");
        let cont = npc_act(&mut mid_clone, 2, &marcus).map(|g| g.label);
        let relo = npc_act(&mut mid_reloaded, 2, &marcus).map(|g| g.label);
        assert_eq!(cont, Some("marcus: bide your time".to_owned()));
        assert_eq!(relo, cont, "the reloaded state makes the same decision");

        // (2) The genuine [S-I1] discharge the label promises: a character whose
        // stored intention IS reused after reload. `artus`' basis still equals his
        // freshly computed MotiveSignature AND his stored GroundedAction is still
        // offered — BOTH equalities holding after the re-intern is exactly what
        // makes npc_act reuse the intention WITHOUT re-deliberating. A broken
        // re-intern (new sym ids in the reloaded ga's bindings) would drop it out
        // of the freshly computed candidates and this `still_offered` reddens.
        let mut reloaded2 = reloaded();
        let artus = character(&reloaded2, "artus");
        let intent = reloaded2
            .intention_of("artus")
            .expect("artus carries a standing intention after reload");
        let sig = reloaded2.motive_signature(&artus);
        assert_eq!(
            intent.basis, sig,
            "the MotiveSignature survives the re-intern (basis == fresh sig)"
        );
        let ga = intent.act.expect("artus' stored intention has an action");
        assert!(
            reloaded2.candidate_actions(&artus).contains(&ga),
            "the stored GroundedAction is still offered after the re-intern \
             (content-canonical equality under the reloaded interner)"
        );
    }

    // H: PersistSpec.hs "a reloaded session continues identically (Marcus can still warn)"
    #[test]
    fn a_reloaded_session_continues_identically() {
        let mut reloaded = reloaded();
        let ga = reloaded
            .possible_actions("marcus")
            .into_iter()
            .find(|g| g.label.contains("warn artus"))
            .expect("marcus can warn artus after reload");
        reloaded.perform_action(&ga);
        assert!(
            reloaded.db_has("ending.loyalty"),
            "reaches the loyalty ending after reload"
        );
    }

    // H: PersistSpec.hs "the serialized form is human-readable, label-faithful facts"
    #[test]
    fn the_serialized_form_is_human_readable_label_faithful_facts() {
        let text = serialize_state(&mid());
        // the value edge is single-valued, so it round-trips with its `!` label
        assert!(
            text.contains("marcus.believes.plotAgainst.artus!yes"),
            "carries the belief Marcus formed, label-faithful"
        );
        // human-readable + carries the cursor field
        assert!(text.contains("\"cursor\""), "has a cursor field");
    }

    // --- v43: the save-format version header ---------------------------------

    // H: PersistSpec.hs "v43: the save-format version header (previously latent: a save from another era misparsed silently)"
    // (the frozen subgroup label; the cases below carry its content)

    // H: PersistSpec.hs "the serialized form's first line is the format version tag"
    #[test]
    fn the_serialized_form_carries_the_format_version_tag() {
        // The frozen asserts the first LINE is the tag; the Rust format is JSON,
        // so the tag is the `version` field — the format's identity. A doc whose
        // version differs is rejected (the other cases), so the tag is load-bearing.
        let text = serialize_state(&mid());
        assert!(text.contains(FORMAT_VERSION), "the version tag is present");
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v.get("version").and_then(|x| x.as_str()), Some(FORMAT_VERSION));
    }

    // H: PersistSpec.hs "a header with no cursor line is a loud, malformed-save error"
    #[test]
    fn a_header_with_no_cursor_is_a_loud_malformed_error() {
        let err = deserialize_state(
            &format!("{{\"version\":\"{FORMAT_VERSION}\"}}"),
            intrigue_world(),
        )
        .unwrap_err();
        assert!(matches!(err, PersistError::Malformed { .. }), "got {err:?}");
        assert!(err.to_string().contains("malformed save"));
    }

    // H: PersistSpec.hs "an unsupported format version (prax-state v0) is a loud, version-mismatch error"
    #[test]
    fn an_unsupported_format_version_v0_is_a_loud_version_mismatch() {
        assert_unsupported("prax-state v0");
    }

    // H: PersistSpec.hs "a save with no header at all is a loud, malformed-save error"
    #[test]
    fn a_save_with_no_header_is_a_loud_malformed_error() {
        let err = deserialize_state("", intrigue_world()).unwrap_err();
        assert!(matches!(err, PersistError::Malformed { .. }), "got {err:?}");
    }

    // H: PersistSpec.hs "the previous format version (prax-state v1) is now rejected loudly"
    #[test]
    fn the_previous_format_version_v1_is_rejected_loudly() {
        // The frozen's "prax-state v1" is a FOREIGN tag to Rust's "prax-rs-state
        // v1" (different namespace, no collision), so it rejects as unsupported.
        assert_unsupported("prax-state v1");
    }

    // H: PersistSpec.hs "the immediately-prior format version (prax-state v2) is rejected under v46's v3 bump"
    #[test]
    fn the_format_version_v2_is_rejected() {
        assert_unsupported("prax-state v2");
    }

    // H: PersistSpec.hs "the immediately-prior format version (prax-state v3) is rejected under v50's v4 bump"
    #[test]
    fn the_format_version_v3_is_rejected() {
        assert_unsupported("prax-state v3");
    }

    /// A save carrying `tag` as its version (any foreign era) must reject loudly
    /// as an unsupported format — the frozen rejection-ladder stance.
    fn assert_unsupported(tag: &str) {
        let err = deserialize_state(
            &format!("{{\"version\":\"{tag}\",\"cursor\":0}}"),
            intrigue_world(),
        )
        .unwrap_err();
        assert!(
            matches!(err, PersistError::UnsupportedVersion { .. }),
            "expected unsupported-format rejection for {tag:?}, got {err:?}"
        );
        assert!(err.to_string().contains("unsupported save format"));
    }

    // --- v44: the schedule's runtime half (dues + the expiry queue) ----------

    // H: PersistSpec.hs "v44: the schedule's runtime half (per-rule dues + the expiry queue) round-trips"
    // (the frozen subgroup label; the cases below carry its content)

    /// A world declaring one schedule rule (so a reloaded due has a rule to
    /// re-associate to). `set_schedule` start-sates it one period out.
    fn beat_world() -> State {
        let mut st = State::new();
        st.set_schedule(vec![ScheduleRule::new("beat", 3)]).unwrap();
        st
    }

    // H: PersistSpec.hs "populated dues and expiries survive save/load; dues re-associate by name"
    #[test]
    fn populated_dues_and_expiries_survive_save_load() {
        // Drive the runtime so dues (set_schedule start-sate) and an expiry
        // (an armed InsertFor) are actually populated — no direct field poke.
        let mut populated = beat_world();
        populated.perform_outcome(&insert_for(6, "mood!a")).unwrap();
        assert!(
            !populated.schedule_dues().is_empty(),
            "the schedule rule start-sates a due"
        );
        assert!(
            !populated.expiries_rendered().is_empty(),
            "the InsertFor arms an expiry"
        );
        let reloaded = deserialize_state(&serialize_state(&populated), beat_world()).unwrap();
        assert_eq!(reloaded.schedule_dues(), populated.schedule_dues());
        // expiries compare by rendered name (CompiledPath has no cross-lineage Ord).
        assert_eq!(reloaded.expiries_rendered(), populated.expiries_rendered());
    }

    // H: PersistSpec.hs "a due naming a rule the reloaded world does not declare is a loud error"
    #[test]
    fn a_due_naming_an_undeclared_rule_is_a_loud_error() {
        // A save carrying a due for "ghost", reloaded onto a world (intrigue)
        // that declares no such rule.
        let save = format!(
            "{{\"version\":\"{FORMAT_VERSION}\",\"cursor\":0,\"intentions\":{{}},\
             \"dues\":{{\"ghost\":3}},\"expiries\":{{}},\"facts\":[]}}"
        );
        let err = deserialize_state(&save, intrigue_world()).unwrap_err();
        assert!(
            matches!(err, PersistError::UnknownScheduleRule { .. }),
            "got {err:?}"
        );
        assert!(err.to_string().contains("unknown schedule rule"));
    }

    // --- v50: the drama die's stream position (rngseed) ----------------------

    // H: PersistSpec.hs "v50: the drama die's stream position (rngseed) round-trips"
    // (the frozen subgroup label; the cases below carry its content)

    // H: PersistSpec.hs "a seeded, mid-stream state round-trips its rngseed exactly"
    #[test]
    fn a_seeded_mid_stream_state_round_trips_its_rngseed() {
        let mut st = State::new();
        st.seed_die(1988).unwrap();
        st.perform_outcome(&Outcome::Roll(1, 2, vec![], vec![insert("hit.mark")]))
            .unwrap();
        let reloaded = deserialize_state(&serialize_state(&st), State::new()).unwrap();
        assert_eq!(reloaded.rng_seed(), st.rng_seed());
    }

    // H: PersistSpec.hs "an unseeded state emits no rngseed line"
    #[test]
    fn an_unseeded_state_emits_no_rngseed() {
        assert!(
            !serialize_state(&mid()).contains("rng_seed"),
            "no rng_seed field for an unseeded (intrigue) save"
        );
    }

    // H: PersistSpec.hs "mid-stream save/resume continues the stream identically"
    #[test]
    fn mid_stream_save_resume_continues_the_stream_identically() {
        let mut st1 = State::new();
        st1.seed_die(1988).unwrap();
        st1.perform_outcome(&Outcome::Roll(1, 2, vec![], vec![])).unwrap();
        let reloaded = deserialize_state(&serialize_state(&st1), State::new()).unwrap();
        let mut cont_direct = st1.clone();
        cont_direct.perform_outcome(&Outcome::Roll(1, 2, vec![], vec![])).unwrap();
        let mut cont_reload = reloaded;
        cont_reload.perform_outcome(&Outcome::Roll(1, 2, vec![], vec![])).unwrap();
        assert_eq!(cont_reload.rng_seed(), cont_direct.rng_seed());
    }

    // --- the banked round-trip law (ARCHITECTURE) ----------------------------

    use proptest::prelude::*;

    proptest! {
        /// `deserialize(serialize(st)) == st` in the name domain, over states
        /// carrying intentions (random tick count), a seed, and armed facts. The
        /// name-domain equality is the sound form under per-state interners:
        /// serialize is content-canonical, so serialize idempotence IS the law.
        #[test]
        fn deserialize_of_serialize_is_the_identity(ticks in 0i32..6, seed in 1i64..2147483562) {
            let mut st = intrigue_world();
            st.seed_die(seed).unwrap();
            run_npc_ticks(&mut st, 2, ticks);
            let once = serialize_state(&st);
            let twice = serialize_state(&deserialize_state(&once, intrigue_world()).unwrap());
            prop_assert_eq!(once, twice);
        }
    }
}

