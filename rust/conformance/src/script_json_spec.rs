//! `Prax.Script.JsonSpec`, re-expressed against the Rust codec: the AST round
//! trip, the per-constructor round trips, the malformed-input rejection, the
//! `after` field, the compile guard reached THROUGH the JSON authoring surface,
//! the `memories` rejection, and the shipped `examples/play.json`.
//!
//! **The nets here are FILE-driven, not frozen-encoder-driven**, and that is the
//! point of the file. `examples/play.json` survives the cut-over; the frozen
//! encoder does not. So the durable net is not "the Rust encoder agrees with the
//! Haskell encoder" — it is "the shipped file IS the Rust encoder's own output",
//! which stays meaningful after `src/` is deleted.
//!
//! Two facts make that net strong, and both are MEASURED rather than argued:
//! the frozen `dump-play` output is byte-identical to `examples/play.json`
//! (2122 bytes), and so is the Rust one. The file is simultaneously the DECODE
//! fixture and the ENCODE golden, so the "adjust the file until the decoder
//! passes" move corrupts both nets at once and cannot pass either.

#[cfg(test)]
mod tests {
    // H: Script/JsonSpec.hs "Prax.Script.Json"
    //
    // The frozen `Prax.Script.JsonSpec` group.
    use prax_core::query::{CalcOp, Condition, calc, matches, neq};
    use prax_core::types::{Outcome, insert};
    use prax_script::compile::{compile, current_scene_of};
    use prax_script::json::{decode_script, encode_script};
    use prax_script::script::timeout;
    use prax_worlds::play::play_script;

    /// The shipped example, embedded from the repo. `cargo test`'s working
    /// directory is the CRATE directory, not the repo root, so the frozen
    /// spec's relative `BL.readFile "examples/play.json"` has no direct Rust
    /// twin [M-4] — resolving at compile time removes the question entirely.
    const SHIPPED_PLAY_JSON: &str = include_str!("../../../examples/play.json");

    // H: Script/JsonSpec.hs "a play-script round-trips through JSON exactly"
    #[test]
    fn a_play_script_round_trips_through_json_exactly() {
        // structural equality: encode then decode is the identity on the AST,
        // so the compiled world (and every ending it reaches) is identical too
        assert_eq!(
            decode_script(encode_script(&play_script()).as_bytes()),
            Ok(play_script())
        );
    }

    // H: Script/JsonSpec.hs "the decoded script still compiles to a runnable world"
    #[test]
    fn the_decoded_script_still_compiles_to_a_runnable_world() {
        let scr = decode_script(encode_script(&play_script()).as_bytes()).expect("decodes");
        let mut st = compile(&scr).expect("compiles");
        assert_eq!(current_scene_of(&mut st).as_deref(), Some("confidence"));
    }

    // H: Script/JsonSpec.hs "malformed JSON reports an error rather than failing silently"
    #[test]
    fn malformed_json_reports_an_error_rather_than_failing_silently() {
        let e = decode_script(br#"{ "start": 3 }"#).expect_err("expected an error");
        println!("{e}");
    }

    // H: Script/JsonSpec.hs "every CalcOp round-trips through JSON, Mod included"
    #[test]
    fn every_calc_op_round_trips_through_json_mod_included() {
        let cs: Vec<Condition> = [CalcOp::Add, CalcOp::Sub, CalcOp::Mul, CalcOp::Mod]
            .into_iter()
            .map(|op| calc("R", op, "17", "5"))
            .collect();
        assert_eq!(round_trip_conditions(&cs), cs);
    }

    // H: Script/JsonSpec.hs "a ForEach outcome round-trips through JSON"
    #[test]
    fn a_for_each_outcome_round_trips_through_json() {
        let o = Outcome::ForEach(
            vec![matches("at.Witness!P"), neq("Witness", "Actor")],
            vec![insert("Witness.believes.stole.Actor.loaf.seen")],
        );
        assert_eq!(round_trip_outcomes(std::slice::from_ref(&o)), vec![o]);
    }

    // H: Script/JsonSpec.hs "an InsertFor outcome round-trips through JSON"
    #[test]
    fn an_insert_for_outcome_round_trips_through_json() {
        let o = Outcome::InsertFor(3, "mood!a".to_owned());
        assert_eq!(round_trip_outcomes(std::slice::from_ref(&o)), vec![o]);
    }

    // H: Script/JsonSpec.hs "a Roll outcome (the drama die, v50) round-trips through JSON"
    #[test]
    fn a_roll_outcome_round_trips_through_json() {
        let o = Outcome::Roll(
            1,
            4,
            vec![matches("shortTempered.T")],
            vec![insert("T.feels.angry.toward.Actor")],
        );
        assert_eq!(round_trip_outcomes(std::slice::from_ref(&o)), vec![o]);
    }

    // H: Script/JsonSpec.hs "a timed junction's \"after\" field round-trips through JSON"
    #[test]
    fn a_timed_junctions_after_field_round_trips_through_json() {
        let j = timeout("gaveUp", 5);
        let scr = prax_script::script::Script::new("a")
            .scenes([prax_script::script::scene("a").junctions([j.clone()])]);
        assert_eq!(
            decode_script(encode_script(&scr).as_bytes()).expect("decodes").scenes[0].junctions,
            vec![j]
        );
    }

    // H: Script/JsonSpec.hs "decoding+compiling the reviewer's JSON repro rejects the Prax-namespaced goto condition (guard-trigger, at the JSON authoring surface FromJSON Junction decodes straight into)"
    #[test]
    fn decoding_and_compiling_the_json_repro_rejects_the_prax_namespaced_goto() {
        let json = br#"{ "start": "a",
              "cast": [ { "name": "p", "playable": true } ],
              "scenes": [
                { "id": "a", "junctions": [
                    { "name": "go", "to": "b", "when":
                        [ { "match": "chapter!PraxNow" } ] } ] },
                { "id": "b" } ] }"#;
        let scr = decode_script(json).expect("the document decodes");
        // the guard lives at the CONSUMPTION point, so every authoring route —
        // smart constructor, raw record, JSON decode — hits the same one
        assert!(matches!(
            compile(&scr).err(),
            Some(prax_core::error::WorldError::ReservedVarClash { .. })
        ));
    }

    // H: Script/JsonSpec.hs "a scene JSON carrying a removed \"memories\" field is rejected loudly, not silently ignored"
    #[test]
    fn a_scene_json_carrying_a_removed_memories_field_is_rejected_loudly() {
        let json = br#"{ "start": "a",
              "cast": [ { "name": "p", "playable": true } ],
              "scenes": [
                { "id": "a", "memories": [ "artus.confided" ] } ] }"#;
        let err = decode_script(json).expect_err("expected decoding to fail");
        assert!(
            err.contains("memories"),
            "the error should name the removed memories feature: {err}"
        );

        // DIV-5, asserted rather than merely written down. The frozen guard is
        // `isJust <$> (o .:? "memories")` and aeson maps an explicit `null` to
        // `Nothing`, so the frozen decoder ACCEPTS this document (measured:
        // `Right (Script {…, sceneId = "a", …})`) — a hole in a guard whose only
        // purpose is loudness, at the spelling an author is most likely to leave
        // behind when half-deleting a field. The Rust fires on the KEY.
        let with_null = br#"{ "start": "a", "cast": [], "scenes": [ { "id": "a", "memories": null } ] }"#;
        let err = decode_script(with_null)
            .expect_err("DIV-5: the Rust guard fires on the key, null included");
        assert!(err.contains("memories"), "{err}");
    }

    // H: Script/JsonSpec.hs "the shipped examples/play.json decodes and compiles"
    #[test]
    fn the_shipped_examples_play_json_decodes_and_compiles() {
        let scr = decode_script(SHIPPED_PLAY_JSON.as_bytes())
            .expect("examples/play.json no longer decodes");
        let mut st = compile(&scr).expect("examples/play.json no longer compiles");
        assert!(
            current_scene_of(&mut st).is_some(),
            "compiles to a scene-bearing world"
        );
    }

    /// NATIVE PIN — the shipped file IS the Rust encoder's own re-emission, byte
    /// for byte. This is [R7]'s claim 6 and the whole basis of the `dump-play`
    /// cut-over equality criterion; nothing frozen pins it, because the frozen
    /// suite never encodes the shipped file.
    ///
    /// The trailing newline is the CLI's, not the encoder's [I-4]: the frozen
    /// `dump-play` arm is `BLC.putStrLn (encodeScript …)`, so `encode_script`
    /// produces 2121 bytes and the file is 2122. Measured on both engines:
    /// `cabal run -v0 prax -- dump-play | diff - examples/play.json` is empty,
    /// and so is the Rust equivalent.
    ///
    /// REDDENS UNDER: any key-order change in the encoder (including a
    /// workspace-wide `serde_json/preserve_order`), any conditional-key change,
    /// and any edit to `examples/play.json` that is not matched by an edit to
    /// `prax_worlds::play::play_script`.
    #[test]
    fn the_shipped_file_is_the_encoders_own_output_byte_for_byte() {
        let emitted = format!("{}\n", encode_script(&play_script()));
        assert_eq!(
            emitted.len(),
            SHIPPED_PLAY_JSON.len(),
            "byte length: encoder {} vs file {}",
            emitted.len(),
            SHIPPED_PLAY_JSON.len()
        );
        assert_eq!(emitted, SHIPPED_PLAY_JSON);
        assert_eq!(SHIPPED_PLAY_JSON.len(), 2122);
    }

    /// NATIVE PIN — the file also decodes to EXACTLY the world's script, not
    /// merely to something that compiles. Together with the pin above this is
    /// the "loads unchanged" claim in full: the file is the encoder's output AND
    /// the decoder's input maps it back to the same value.
    ///
    /// REDDENS UNDER: a decoder that silently drops a field the encoder emits
    /// (`opening` is the one nothing else would catch — it is read by nothing,
    /// so only an equality assertion sees it go).
    #[test]
    fn the_shipped_file_decodes_to_exactly_the_worlds_own_script() {
        assert_eq!(
            decode_script(SHIPPED_PLAY_JSON.as_bytes()),
            Ok(play_script())
        );
    }

    /// Round-trip a condition list through a whole document (the frozen spec
    /// round-trips bare `Condition`/`Outcome` values, which have no standalone
    /// Rust codec: the schema is a script schema).
    fn round_trip_conditions(cs: &[Condition]) -> Vec<Condition> {
        let scr = prax_script::script::Script::new("a").scenes([
            prax_script::script::scene("a")
                .junctions([prax_script::script::ending("e", cs.to_vec())]),
        ]);
        decode_script(encode_script(&scr).as_bytes())
            .expect("decodes")
            .scenes[0]
            .junctions[0]
            .when
            .clone()
    }

    fn round_trip_outcomes(os: &[Outcome]) -> Vec<Outcome> {
        let scr = prax_script::script::Script::new("a")
            .scenes([prax_script::script::scene("a").setup(os.to_vec())]);
        decode_script(encode_script(&scr).as_bytes())
            .expect("decodes")
            .scenes[0]
            .setup
            .clone()
    }
}
