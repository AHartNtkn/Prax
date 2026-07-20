//! Lowering an authored [`Script`] to a ready-to-run engine [`State`]: the
//! `beats` practice, the compiler-owned `story` schedule rule, the scene-entry
//! fold, and the five hygiene guards standing in front of all of it.
//!
//! Frozen reference: `src/Prax/Script.hs`'s `compile`, `flowChart`,
//! `currentSceneOf`. The frozen NESTS its builders (`setCharacters (…
//! (definePractices [beatsP] emptyState))`); the Rust builder API is `&mut`
//! setters, so the nesting inverts to a statement sequence read outside-in —
//! practices, functions, cast, the engine-door rule, then the setup fold.
//!
//! **The lowering orderings.** Seven are generated here and only ONE of them is
//! seen by a frozen pin, so the rest carry native pins in this file (S7 [C2]:
//! a net that only asserts equal-to-frozen evaporates at cut-over).
//!
//! | | ordering | frozen net |
//! |---|---|---|
//! | O1 | compiled action labels = scene decl × beat decl | none |
//! | O2 | `story` clauses = scene decl × junction decl, eager forward-only fold | FOUR `ScriptSpec` pins |
//! | O3 | scene entry = patience markers BEFORE the authored setup | none |
//! | O4 | compiled beat `when` conjunct order | none |
//! | O5 | story-clause GUARD conjunct order | none |
//! | O6 | story-clause BODY order: the scene switch before the destination's setup | none |
//! | O7 | the compile-time setup fold order | none needed — the pin says why |

use prax_core::engine::{State, door};
use prax_core::error::WorldError;
use prax_core::path::segment_names_checked;
use prax_core::query::{Condition, cond_sents, eq, matches, not_};
use prax_core::typecheck::{CURRENT_SCENE_PATH, SCENE_PATIENCE_FAMILY, scene_patience_path};
use prax_core::types::{
    Action, Character, Outcome, Practice, ScheduleRule, authored_var_clash, insert, outcome_sents,
};
use prax_vocab::core_model::core_fns;

use crate::script::{Beat, Junction, Scene, Script};

/// The id of the one practice a script compiles its beats into.
pub const BEATS_PRACTICE: &str = "beats";
/// The name of the one schedule rule a script compiles its junctions into.
pub const STORY_RULE: &str = "story";

/// The current-scene fact for a scene id: `currentScene!<sid>`. Literal-tailed,
/// so the slot is exclusive and at most one scene is ever current.
fn current_scene_fact(sid: &str) -> String {
    format!("{CURRENT_SCENE_PATH}!{sid}")
}

/// The id of the currently-active scene, if any (`Prax.Script.currentSceneOf`).
///
/// The frozen unifies `currentScene.S` and takes the first solution. Because
/// `currentScene` is written with `!`, the slot is EXCLUSIVE and there is at
/// most one child — so "first" is not an ordering commitment and no sort is
/// ported. `db_child_keys` is the same base-db probe the randtrace walk already
/// uses for `ending.E`.
pub fn current_scene_of(st: &mut State) -> Option<String> {
    st.db_child_keys(CURRENT_SCENE_PATH).into_iter().next()
}

/// Substitute a concrete speaker for every `[Actor]` token in a label
/// (`Prax.Script.bakeActor`).
///
/// The frozen is a hand-rolled left-to-right scan that never rescans the text it
/// inserted. `str::replace` has exactly those semantics — it also scans the
/// ORIGINAL left to right and never re-examines a replacement — so the two agree
/// on every input, including a speaker name that itself contains `[Actor]`.
/// Stated, and pinned below, rather than "simplified" into a `format!`, which
/// would only be equivalent for labels whose `[Actor]` sits at the front.
fn bake_actor(speaker: &str, label: &str) -> String {
    label.replace("[Actor]", speaker)
}

// ---- the five guards -------------------------------------------------------

/// Every authored condition/outcome list `compile` consumes, labelled with the
/// frozen guard's own site string.
///
/// The sweep is LIST-MAJOR, not scene-major: every scene's setup, THEN every
/// junction's `when`, THEN every beat condition, THEN every beat effect, THEN
/// every cast desire. A scene-by-scene walk reports a different offender on a
/// doubly-offending script, and the Rust error carries the site.
fn authored_sentence_sites(scr: &Script) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for s in &scr.scenes {
        out.push((format!("scene {:?}'s setup", s.id), outcome_sents(&s.setup)));
    }
    for s in &scr.scenes {
        for j in &s.junctions {
            out.push((
                format!("scene {:?}'s junction {:?} condition", s.id, j.name),
                cond_sents(&j.when),
            ));
        }
    }
    for s in &scr.scenes {
        for b in &s.beats {
            out.push((
                format!("scene {:?}'s beat {:?} condition", s.id, b.label),
                cond_sents(&b.when),
            ));
        }
    }
    for s in &scr.scenes {
        for b in &s.beats {
            out.push((
                format!("scene {:?}'s beat {:?} effect", s.id, b.label),
                outcome_sents(&b.effects),
            ));
        }
    }
    for c in &scr.cast {
        for w in &c.desires {
            out.push((
                format!("cast member {:?}'s desire", c.name),
                cond_sents(&w.when),
            ));
        }
    }
    out
}

/// The head segment of an authored sentence.
///
/// CHECKED split: the frozen `headSegment` is `pathNames`, which raises on a
/// trailing operator BEFORE any head is taken, so `"foo."` in an authored setup
/// dies as a malformed sentence at this guard rather than sailing past it. The
/// checked splitter is the one that keeps that error identity.
fn head_segment(s: &str) -> Result<Option<String>, WorldError> {
    Ok(segment_names_checked(s)?.into_iter().next())
}

/// The five construction guards, in the frozen order (`Prax.Script.compile`'s
/// guard chain, `Script.hs:273-292`). The ORDER is observable through WHICH
/// error a doubly-offending script gets, and so are the two within-guard
/// selection rules noted at their sites.
fn check_guards(scr: &Script) -> Result<(), WorldError> {
    // 1. Duplicate junction names within one scene.
    //
    // WITHIN-GUARD SELECTION: the frozen `repeated xs = [x | (x:_:_) <- group
    // (sort xs)]` reports the ALPHABETICALLY-FIRST duplicated name, not the
    // declaration-first one. Scenes are still walked in declaration order.
    for s in &scr.scenes {
        let mut names: Vec<&str> = s.junctions.iter().map(|j| j.name.as_str()).collect();
        names.sort_unstable();
        if let Some(dup) = names.windows(2).find(|w| w[0] == w[1]).map(|w| w[0]) {
            return Err(WorldError::DuplicateJunctionName {
                scene: s.id.clone(),
                junction: dup.to_owned(),
            });
        }
    }
    // 2. A timed junction with a delay below one round. Scene-major, then
    //    junction declaration order.
    for s in &scr.scenes {
        for j in &s.junctions {
            if let Some(n) = j.after
                && n < 1
            {
                return Err(WorldError::ZeroDelayJunction {
                    scene: s.id.clone(),
                    junction: j.name.clone(),
                    delay: n,
                });
            }
        }
    }
    // 3. An authored sentence headed by the compiler-owned patience family, in
    //    ANY of the five swept lists — walked LIST-MAJOR (above).
    //
    // REACH, stated exactly [D-I7]: this guard refuses any sentence
    // `outcome_sents`/`cond_sents` SEE. On the WRITE side that is complete —
    // `outcome_sents` covers `Insert`/`Delete`/`InsertFor` and recurses through
    // `ForEach` and `Roll` bodies. On the READ side it is not: `Count`'s
    // sentence operand and `Subquery`'s `set` operand are dropped by
    // `cond_sents`, so `{"count": ["N", "scenePatience.a.b"]}` is ACCEPTED. The
    // Rust REPRODUCES that read hole and does not fix it — a `Count`-aware sweep
    // would reject a script the frozen accepts, which is a differential
    // divergence dressed up as hygiene. If anyone wants the stricter contract it
    // is an S10 fork question, not an S8 convenience.
    for (site, sents) in authored_sentence_sites(scr) {
        for s in &sents {
            if head_segment(s)?.as_deref() == Some(SCENE_PATIENCE_FAMILY) {
                return Err(WorldError::ReservedFamilyAuthored {
                    site,
                    sentence: s.clone(),
                    family: SCENE_PATIENCE_FAMILY.to_owned(),
                });
            }
        }
    }
    // 4. A Prax-namespaced variable in a SCENE SETUP (the v40 hygiene boundary;
    //    setups splice into the story rule's clause bodies). ALL scenes' setups
    //    are checked before ANY junction `when` — two separate frozen guards
    //    with a definite order between them.
    for s in &scr.scenes {
        if let Some(v) = authored_var_clash(&[], &[], &s.setup)?.into_iter().next() {
            return Err(WorldError::ReservedVarClash {
                context: "Script.compile".to_owned(),
                var: v,
                extra: format!(" (scene {:?}'s setup)", s.id),
            });
        }
    }
    // 5. A Prax-namespaced variable in a JUNCTION `when` (they splice into the
    //    story rule's clause guards).
    for s in &scr.scenes {
        for j in &s.junctions {
            if let Some(v) = authored_var_clash(&[], &j.when, &[])?.into_iter().next() {
                return Err(WorldError::ReservedVarClash {
                    context: "Script.compile".to_owned(),
                    var: v,
                    extra: format!(" (scene {:?}'s junction {:?})", s.id, j.name),
                });
            }
        }
    }
    Ok(())
}

// ---- the lowering ----------------------------------------------------------

/// The compiled form of one beat of scene `sid`.
///
/// O4 — the `when` conjunct order — is `[currentScene!<sid>, character.Actor]`,
/// then the speaker gate if there is one, then the author's own conditions. The
/// order decides binding order at query time.
///
/// A quip is a SPECIFIC speaker's action, so its compiled id bakes the speaker
/// into the `[Actor]` slot: two speakers sharing the display text
/// `[Actor]: flatter the king` become distinct actions, and dispatch (which is
/// by action id) cannot cross them. The rendered label is unchanged, since
/// `[Actor]` would render to the speaker anyway.
fn compile_beat(sid: &str, b: &Beat) -> Action {
    let label = match &b.speaker {
        Some(spk) => bake_actor(spk, &b.label),
        None => b.label.clone(),
    };
    let mut when = vec![matches(current_scene_fact(sid)), matches("character.Actor")];
    if let Some(spk) = &b.speaker {
        when.push(eq("Actor", spk.clone()));
    }
    when.extend(b.when.iter().cloned());
    Action::new(label).when(when).then(b.effects.clone())
}

/// Scene entry (`Prax.Script.setupOf`): one patience marker per timed junction
/// of the scene, in junction declaration order, THEN the scene's authored setup
/// (O3).
///
/// A marker is `InsertFor n scenePatience.<sid>.<j>` — a plain literal insert
/// whose LIFETIME is the delay, retracted `n` boundaries later by the v44 expiry
/// schedule. Re-entry refreshes it (v44's supersession branch re-arms). This
/// runs on all three entry paths — compile-time start, transition, re-entry —
/// because every path threads through here.
fn setup_of(scenes: &[Scene], sid: &str) -> Vec<Outcome> {
    let Some(s) = scenes.iter().find(|s| s.id == sid) else {
        return Vec::new();
    };
    let mut out: Vec<Outcome> = s
        .junctions
        .iter()
        .filter_map(|j| {
            j.after
                .map(|n| Outcome::InsertFor(n, scene_patience_path(sid, &j.name)))
        })
        .collect();
    out.extend(s.setup.iter().cloned());
    out
}

/// One `story` clause for junction `j` of scene `sid`.
///
/// O5 — the GUARD order — is `[currentScene!<sid>, Absent[ending.E]]`, then the
/// author's own `when`, then (for a timed junction) the `Not` on the patience
/// marker. A timed junction fires when its patience has RUN OUT: the marker
/// armed at entry has expired, so `Not` holds. The v44 boundary order (expiries
/// before rules) retracts the marker at entry+n exactly when this clause first
/// becomes eligible.
///
/// O6 — the BODY order — puts the scene switch FIRST and the destination's entry
/// outcomes after it. Swap them and a cascading transition runs the
/// destination's setup while the source scene is still current.
fn story_clause(scenes: &[Scene], sid: &str, j: &Junction) -> (Vec<Condition>, Vec<Outcome>) {
    let mut guard = vec![
        matches(current_scene_fact(sid)),
        Condition::Absent(vec![matches("ending.E")]),
    ];
    guard.extend(j.when.iter().cloned());
    if j.after.is_some() {
        guard.push(not_(scene_patience_path(sid, &j.name)));
    }
    let body = match &j.to {
        Some(next) => {
            let mut b = vec![insert(current_scene_fact(next))];
            b.extend(setup_of(scenes, next));
            b
        }
        None => vec![insert(format!("ending!{}", j.name))],
    };
    (guard, body)
}

/// Compile a [`Script`] into a ready-to-run [`State`] (`Prax.Script.compile`).
///
/// Loud at the CONSUMPTION point — uniformly over every construction route
/// (smart constructor, raw record literal, or JSON decode) — on the five
/// authoring faults [`check_guards`] enumerates. Every frozen `error` in
/// `Script.hs` becomes a [`WorldError`] here.
///
/// **One panic remains reachable, and it is not this module's** [S-I6]: an
/// authored `Roll` in a scene setup (spellable through JSON) is executed by the
/// compile-time outcome fold against an unseeded die, and the engine `error`s on
/// that before it even evaluates the roll's guard — in `Engine.hs`, not
/// `Script.hs`. The Rust engine panics in the same place, for the same reason.
/// That panic is also the guarantee `worldshape`'s setup-roll scan cannot give
/// for a script world: a compiled script world's mere EXISTENCE proves its setup
/// consumed zero draws.
///
/// # Errors
/// [`WorldError`] for any of the five guards, or for a malformed authored path.
pub fn compile(scr: &Script) -> Result<State, WorldError> {
    check_guards(scr)?;

    // O1: scene declaration order × that scene's beat declaration order.
    let mut beats = Practice::new(BEATS_PRACTICE)
        .name("scene dialogue")
        .roles(["Stage"]);
    for s in &scr.scenes {
        for b in &s.beats {
            beats = beats.action(compile_beat(&s.id, b));
        }
    }

    // Junctions and endings are the world's own dynamics, not a character's
    // action, so they compile to ONE plain period-1 schedule rule the engine
    // fires silently at each round boundary. Each clause's own gates self-mask:
    // the transition's `currentScene` eviction masks same-scene doubles, and
    // `Absent ending` masks everything after an ending. The fold is eager,
    // forward-only and order-sensitive — a clause is re-queried against the
    // PRECEDING clause's post-state — so a transition can cascade straight into
    // a later-declared scene's junction in the same boundary, but never into an
    // earlier-declared one's, whose turn in the fold has already passed.
    //
    // It carries Prax-namespaced machinery (the destination's patience markers),
    // so it registers through the engine door, not `set_schedule`.
    let mut story = ScheduleRule::new(STORY_RULE, 1);
    for s in &scr.scenes {
        for j in &s.junctions {
            let (guard, body) = story_clause(&scr.scenes, &s.id, j);
            story = story.clause(guard, body);
        }
    }

    let mut st = State::new();
    st.define_practices([beats])?;
    st.define_functions(core_fns())?;
    st.set_characters(
        scr.cast
            .iter()
            .map(|c| {
                let mut ch = Character::new(c.name.clone());
                ch.wants = c.desires.clone();
                ch
            })
            .collect(),
    )?;
    door::register_engine_rules(&mut st, vec![story])?;

    // O7 — the compile-time setup fold: the stage instance, then one
    // `character.<c>` per cast member in declaration order, then the traits,
    // then the start scene, then the start scene's entry outcomes.
    let mut setup = vec![insert(format!("practice.{BEATS_PRACTICE}.stage"))];
    for c in &scr.cast {
        setup.push(insert(format!("character.{}", c.name)));
    }
    for c in &scr.cast {
        for t in &c.traits {
            setup.push(insert(format!("trait.{}.{}", c.name, t)));
        }
    }
    setup.push(insert(current_scene_fact(&scr.start)));
    setup.extend(setup_of(&scr.scenes, &scr.start));
    for o in &setup {
        st.perform_outcome(o)?;
    }
    Ok(st)
}

// ---- tooling ---------------------------------------------------------------

/// Render the scene graph as a Mermaid `graph TD` (`Prax.Script.flowChart`,
/// Prompter's auto-generated flow chart): a `start` node into the opening scene,
/// one node per scene, and a labelled edge per junction — to the target scene,
/// or to a terminal ending node.
pub fn flow_chart(scr: &Script) -> String {
    // Mermaid node ids must be identifier-like; display text stays in the labels.
    fn node_id(s: &str) -> String {
        s.chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect()
    }
    let mut lines = vec![
        "graph TD".to_owned(),
        format!("  _start((start)) --> {}", node_id(&scr.start)),
    ];
    for s in &scr.scenes {
        lines.push(format!("  {}[\"{}\"]", node_id(&s.id), s.id));
        for j in &s.junctions {
            let from = node_id(&s.id);
            lines.push(match &j.to {
                Some(to) => format!("  {from} -->|{}| {}", j.name, node_id(to)),
                None => format!(
                    "  {from} -->|{}| _end_{}(({}))",
                    j.name,
                    node_id(&j.name),
                    j.name
                ),
            });
        }
    }
    // `unlines`: a trailing newline after EVERY line, the last one included.
    let mut out = String::new();
    for l in lines {
        out.push_str(&l);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::{
        after, beat, ending, goto, member, player, quip, scene, timeout, with_traits,
    };

    /// A two-scene script whose every ordering is non-trivial: the scenes are
    /// declared out of alphabetical order, so are the beats and the junctions,
    /// and both scenes carry a timed junction and an authored setup.
    fn ordering_probe() -> Script {
        Script::new("zeta")
            .cast([player("p"), member("q")])
            .scenes([
                scene("zeta")
                    .setup([insert("zSetup")])
                    .beats([
                        quip("q", "[Actor]: zeta-second", vec![not_("done")], vec![]),
                        beat("zeta-first", vec![], vec![]),
                    ])
                    .junctions([
                        goto("zOut", "alpha", vec![matches("go")]),
                        after("zWait", 3, "alpha"),
                    ]),
                scene("alpha")
                    .setup([insert("aSetup")])
                    .beats([beat("alpha-only", vec![], vec![])])
                    .junctions([timeout("aEnd", 2)]),
            ])
    }

    /// NATIVE PIN (O1) — no frozen net at all. The design's §1.4 says the only
    /// net is `worldshape.shape.action_labels`; that understates it — the
    /// randtrace `candidates` list is emitted in NATIVE order and `pick` INDEXES
    /// it, so O1 also decides which action each seed selects. Both nets die at
    /// cut-over (the four committed goldens are bar/intrigue/loop-bar, none
    /// script-compiled), and no frozen pin sees it: every frozen `actionMatching`
    /// call site leaves exactly one match after the speaker gate.
    ///
    /// The assertion is the full label SEQUENCE, not a set — a membership
    /// assertion would not redden under the mutation this exists to catch.
    ///
    /// REDDENS UNDER: swapping the two nested loops (beats-major instead of
    /// scenes-major) in `compile`'s action list.
    #[test]
    fn o1_compiled_action_labels_are_scene_major_then_beat_declaration_order() {
        let st = compile(&ordering_probe()).expect("the probe compiles");
        let labels: Vec<&str> = st.practice_defs()[BEATS_PRACTICE]
            .actions
            .iter()
            .map(|a| a.name.as_str())
            .collect();
        assert_eq!(
            labels,
            ["q: zeta-second", "zeta-first", "alpha-only"],
            "scene declaration order outermost, beat declaration order within \
             (NOT alphabetical, NOT beats-major)"
        );
    }

    /// NATIVE PIN (O3) — the only net is `worldshape.bodies`, which asserts
    /// equal-to-frozen and dies at cut-over. Scene entry arms the patience
    /// markers BEFORE running the authored setup.
    ///
    /// The assertion is the emitted outcome SEQUENCE. A set assertion would not
    /// redden under the mutation, which is exactly the S7 [C2] trap.
    ///
    /// REDDENS UNDER: putting `s.setup` before the `InsertFor` markers in
    /// `setup_of`.
    #[test]
    fn o3_scene_entry_arms_patience_markers_before_the_authored_setup() {
        let scr = ordering_probe();
        assert_eq!(
            setup_of(&scr.scenes, "zeta"),
            vec![
                Outcome::InsertFor(3, "scenePatience.zeta.zWait".to_owned()),
                insert("zSetup"),
            ]
        );
    }

    /// NATIVE PIN (O4) — `worldshape.bodies` only. The compiled beat leads with
    /// the scene gate and the cast gate, then the speaker gate, then the
    /// author's own conditions. Order decides binding order at query time.
    ///
    /// REDDENS UNDER: moving `b.when` ahead of the `currentScene` match, or
    /// dropping the `character.Actor` gate (which is what stops a non-cast name
    /// speaking).
    #[test]
    fn o4_a_compiled_beat_gates_scene_then_cast_then_speaker_then_author() {
        let b = quip("q", "[Actor]: say", vec![not_("done")], vec![insert("said")]);
        let a = compile_beat("zeta", &b);
        assert_eq!(
            a.when,
            vec![
                matches("currentScene!zeta"),
                matches("character.Actor"),
                eq("Actor", "q"),
                not_("done"),
            ]
        );
        assert_eq!(a.then, vec![insert("said")]);
        // and a speaker-less beat carries no `Eq` gate at all
        let open = compile_beat("zeta", &beat("plain", vec![], vec![]));
        assert_eq!(
            open.when,
            vec![matches("currentScene!zeta"), matches("character.Actor")]
        );
    }

    /// NATIVE PIN (O5/O6) — `worldshape.bodies` only. The story clause's guard
    /// order and, more importantly, its BODY order: the `currentScene` switch
    /// comes FIRST and the destination's entry outcomes after it.
    ///
    /// REDDENS UNDER: emitting `setup_of(next)` before the
    /// `Insert currentScene!next` (which would run a destination's setup while
    /// the source scene is still current — silently wrong only on a cascade), or
    /// moving the `Not` patience conjunct ahead of the author's `when`.
    #[test]
    fn o5_o6_the_story_clause_guard_and_body_orders() {
        let scr = ordering_probe();
        let timed = &scr.scenes[0].junctions[1]; // after "zWait" 3 "alpha"
        let (guard, body) = story_clause(&scr.scenes, "zeta", timed);
        assert_eq!(
            guard,
            vec![
                matches("currentScene!zeta"),
                Condition::Absent(vec![matches("ending.E")]),
                // the author's own `when` would sit here (this junction has none)
                not_("scenePatience.zeta.zWait"),
            ]
        );
        assert_eq!(
            body,
            vec![
                insert("currentScene!alpha"),
                Outcome::InsertFor(2, "scenePatience.alpha.aEnd".to_owned()),
                insert("aSetup"),
            ],
            "the scene switch precedes the destination's entry outcomes"
        );

        // an author's own `when` sits between the two standing gates and the
        // patience `Not`
        let (untimed, end_body) = story_clause(&scr.scenes, "alpha", &scr.scenes[1].junctions[0]);
        assert_eq!(
            untimed,
            vec![
                matches("currentScene!alpha"),
                Condition::Absent(vec![matches("ending.E")]),
                not_("scenePatience.alpha.aEnd"),
            ]
        );
        assert_eq!(end_body, vec![insert("ending!aEnd")]);
    }

    /// NATIVE PIN (O7) — no net is NEEDED, and the pin records WHY rather than
    /// leaving the silence to be read as an oversight. The compile-time setup
    /// fold's order is unobservable: every outcome is a plain `Insert` of a
    /// distinct literal path, so any permutation reaches the same db, and a
    /// script world has no die, so no permutation can move an RNG stream either.
    /// What the pin does assert is the CONTENT.
    ///
    /// REDDENS UNDER: dropping the trait inserts, or the stage instance (without
    /// which no beat is ever enumerable).
    #[test]
    fn o7_the_compile_time_setup_fold_asserts_stage_cast_traits_and_scene_entry() {
        let scr = Script::new("s")
            .cast([with_traits(player("p"), ["bold", "shy"])])
            .scenes([scene("s").setup([insert("here")])]);
        let st = compile(&scr).expect("compiles");
        let facts = st.labeled_facts();
        for f in [
            "practice.beats.stage",
            "character.p",
            "trait.p.bold",
            "trait.p.shy",
            "currentScene!s",
            "here",
        ] {
            assert!(facts.contains(&f.to_owned()), "{f} in setup, got {facts:?}");
        }
    }

    /// NATIVE PIN — the compile GUARD ORDER and the two within-guard
    /// offender-selection rules. The frozen suite asserts only `isLeft` on every
    /// one of its eight guard cases, so the order is entirely unpinned
    /// frozen-side — yet it is observable the moment an error carries a site,
    /// which the Rust errors do.
    ///
    /// REDDENS UNDER: any reordering of the five guards; walking the patience
    /// sweep scene-major instead of list-major; or reporting the
    /// declaration-first duplicate junction name instead of the alphabetically
    /// first.
    #[test]
    fn the_five_compile_guards_fire_in_the_frozen_order() {
        // 1 before 2: a scene with BOTH a duplicate name and a zero delay.
        let dup_and_zero = Script::new("a")
            .cast([player("p")])
            .scenes([scene("a").junctions([timeout("dup", 0), ending("dup", vec![])])]);
        assert!(matches!(
            compile(&dup_and_zero),
            Err(WorldError::DuplicateJunctionName { .. })
        ));

        // WITHIN GUARD 1: the ALPHABETICALLY first duplicated name, not the
        // declaration-first one. `zed` is declared first and duplicated; so is
        // `abe`; the frozen `group . sort` reports `abe`.
        let two_dups = Script::new("a").cast([player("p")]).scenes([scene("a").junctions([
            ending("zed", vec![]),
            ending("abe", vec![]),
            ending("zed", vec![]),
            ending("abe", vec![]),
        ])]);
        assert_eq!(
            compile(&two_dups).err(),
            Some(WorldError::DuplicateJunctionName {
                scene: "a".to_owned(),
                junction: "abe".to_owned(),
            })
        );

        // 2 before 3: a zero-delay junction AND an authored patience read.
        let zero_and_patience = Script::new("a").cast([player("p")]).scenes([scene("a")
            .junctions([
                timeout("t", 0),
                ending("e", vec![matches("scenePatience.a.t")]),
            ])]);
        assert!(matches!(
            compile(&zero_and_patience),
            Err(WorldError::ZeroDelayJunction { .. })
        ));

        // 3 before 4: an authored patience write AND a Prax-namespaced setup var.
        let patience_and_clash = Script::new("a").cast([player("p")]).scenes([
            scene("a").setup([insert("scenePatience.a.x"), insert("mood.PraxNow")]),
        ]);
        assert!(matches!(
            compile(&patience_and_clash),
            Err(WorldError::ReservedFamilyAuthored { .. })
        ));

        // WITHIN GUARD 3: the sweep is LIST-MAJOR. Scene `a` (declared FIRST)
        // offends in its JUNCTION `when`; scene `b` offends in its SETUP. All
        // setups are swept before any junction `when`, so `b`'s setup is
        // reported — a scene-major walk would report `a`'s junction.
        let list_major = Script::new("a").cast([player("p")]).scenes([
            scene("a").junctions([goto("j", "b", vec![matches("scenePatience.a.j")])]),
            scene("b").setup([insert("scenePatience.b.k")]),
        ]);
        assert_eq!(
            compile(&list_major).err(),
            Some(WorldError::ReservedFamilyAuthored {
                site: "scene \"b\"'s setup".to_owned(),
                sentence: "scenePatience.b.k".to_owned(),
                family: "scenePatience".to_owned(),
            })
        );

        // 4 before 5: scene `a` (declared first) has a clean setup and a
        // clashing junction `when`; scene `b` has a clashing setup. ALL setups
        // are checked before ANY junction `when`, so `b`'s setup wins.
        let setup_before_junction = Script::new("a").cast([player("p")]).scenes([
            scene("a").junctions([goto("j", "b", vec![matches("chapter!PraxNow")])]),
            scene("b").setup([insert("flag.PraxE")]),
        ]);
        match compile(&setup_before_junction).expect_err("both sites clash") {
            WorldError::ReservedVarClash { var, extra, .. } => {
                assert_eq!(var, "PraxE");
                assert!(extra.contains("\"b\"'s setup"), "got {extra:?}");
            }
            other => panic!("expected a ReservedVarClash, got {other:?}"),
        }
    }

    /// NATIVE PIN — the `Count`/`Subquery` READ HOLE in guard 3, reproduced
    /// rather than fixed [D-I7]. `cond_sents` does not see `Count`'s sentence
    /// operand, so the frozen `compile` ACCEPTS a junction condition that counts
    /// over the reserved family. Fixing it in Rust would reject a script the
    /// frozen accepts — a differential divergence dressed as hygiene.
    ///
    /// REDDENS UNDER: extending the sweep to `Count`/`Subquery` set operands
    /// (which is the change this pin exists to stop being made silently).
    #[test]
    fn the_patience_sweep_reproduces_the_frozen_count_read_hole() {
        let counted = Script::new("a").cast([player("p")]).scenes([scene("a").junctions([
            ending(
                "e",
                vec![prax_core::query::count("N", "scenePatience.a.e")],
            ),
        ])]);
        assert!(
            compile(&counted).is_ok(),
            "the frozen guard does not see a Count's sentence operand, and \
             neither does this one -- a READ hole, and CG-1 is unaffected \
             because the WRITE side is complete"
        );
    }

    /// NATIVE PIN — `bake_actor`'s exact semantics. No frozen pin observes the
    /// rescan question: `ScriptSpec`'s speaker-dispatch case uses two ordinary
    /// names. The claim pinned is that `str::replace` IS the hand-rolled
    /// left-to-right never-rescan scan — including at the one input where a
    /// naive fixpoint replacement would diverge.
    ///
    /// REDDENS UNDER (both verified): rewriting `bake_actor` as a
    /// `starts_with` + `format!` prefix, which is wrong for the mid-label case;
    /// and looping `replace` to a fixpoint, which on the self-referential
    /// speaker does not merely differ — it does not TERMINATE, and the pin is
    /// what turns a hang into a named failure.
    #[test]
    fn bake_actor_replaces_every_occurrence_left_to_right_without_rescanning() {
        assert_eq!(bake_actor("q", "[Actor]: greet"), "q: greet");
        // not at the front, and more than once
        assert_eq!(
            bake_actor("q", "the [Actor] greets [Actor]"),
            "the q greets q"
        );
        // a speaker name containing the token: the inserted text is never
        // rescanned, so exactly one round of substitution happens
        assert_eq!(bake_actor("[Actor]x", "[Actor]!"), "[Actor]x!");
        // no token at all: verbatim
        assert_eq!(bake_actor("q", "plain label"), "plain label");
    }

    /// NATIVE PIN — `flow_chart`'s WHOLE rendered string. The frozen pin asserts
    /// only that seven substrings appear, and `flow` output is a stated
    /// cut-over equality criterion, so seven `contains` calls are not the net
    /// this needs. The exact string for a script with a transition, an ending
    /// and a non-identifier scene id is pinned instead.
    ///
    /// REDDENS UNDER: changing indentation, the arrow syntax, the `_end_` node
    /// prefix, the `unlines` trailing newline, or the non-alphanumeric →
    /// underscore node-id mangling.
    #[test]
    fn flow_chart_renders_the_exact_mermaid_graph() {
        let scr = Script::new("act one").scenes([
            scene("act one").junctions([goto("toB", "act-two", vec![])]),
            scene("act-two").junctions([ending("done", vec![])]),
        ]);
        assert_eq!(
            flow_chart(&scr),
            "graph TD\n\
             \x20 _start((start)) --> act_one\n\
             \x20 act_one[\"act one\"]\n\
             \x20 act_one -->|toB| act_two\n\
             \x20 act_two[\"act-two\"]\n\
             \x20 act_two -->|done| _end_done((done))\n"
        );
    }

    /// NATIVE PIN — the two-door registration and its provenance record. The
    /// compiled `story` rule goes in through the COMPILER door, so its name is
    /// recorded in `engine_rule_names` (v53) and the S9 checker will exempt it;
    /// an authored rule of the same shape would not be.
    ///
    /// REDDENS UNDER: routing the story rule through `set_schedule` instead
    /// (which also rejects it outright, since a transition body writes the
    /// reserved patience family).
    #[test]
    fn the_story_rule_arrives_through_the_compiler_door_with_provenance() {
        let st = compile(&ordering_probe()).expect("compiles");
        assert_eq!(st.engine_rule_names(), ["story"]);
        assert_eq!(st.schedule_src().len(), 1);
        assert_eq!(st.schedule_src()[0].name, STORY_RULE);
        assert_eq!(st.schedule_src()[0].period, 1);
        // one clause per (scene, junction) in declaration × declaration order
        assert_eq!(st.schedule_src()[0].body.len(), 3);
    }

    /// NATIVE PIN — a script world NEVER seeds the die, and that is the
    /// guarantee standing in for `worldshape`'s setup-roll scan, which is
    /// structurally BLIND here: it scans practice INIT outcomes, and the
    /// compiled `beats` practice has none — a script's setup is a compile-time
    /// fold the state does not retain, so the check passes VACUOUSLY. (The
    /// emitted `"setup_rolls_zero": true` is a hardcoded literal on both sides;
    /// it is a comment in the document, never evidence.)
    ///
    /// The substitute is STRONGER than the scan it replaces: since the engine
    /// errors on a `Roll` against an unseeded die before evaluating the roll's
    /// guard, a compiled script world's existence proves its setup consumed ZERO
    /// draws — including through routes a static scan cannot see.
    ///
    /// REDDENS UNDER: seeding the die inside `compile`.
    #[test]
    fn a_script_world_has_no_die_so_its_setup_provably_drew_nothing() {
        let st = compile(&ordering_probe()).expect("compiles");
        assert_eq!(st.rng_seed(), None);
        assert!(
            st.practice_defs()[BEATS_PRACTICE]
                .init_outcomes
                .is_empty(),
            "the compiled beats practice has no init outcomes, which is why the \
             frozen setup-roll scan is vacuous for script worlds"
        );
    }
}
