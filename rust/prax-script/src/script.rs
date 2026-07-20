//! The authored `Script`: scenes, casts, beats, and junctions — the value a
//! builder or a decoded JSON document produces before compilation.
//!
//! Frozen reference: `src/Prax/Script.hs`'s AST and smart constructors, mirrored
//! field for field. This is the AUTHORING family (ARCHITECTURE's stance): plain
//! `String`s and `Vec`s, no interning, converted only inside
//! [`crate::compile::compile`].
//!
//! **Every constructor here is INFALLIBLE** (S4's builders-build-values rule),
//! and that is not a convenience. The frozen [`goto`] documents explicitly that
//! its hygiene guard lives at `compile`, not at construction, *because a
//! `Junction` can be built by the smart constructor, by a raw record literal, or
//! by JSON decode*. Validation sits at the one consumption point, uniformly over
//! all three routes; a constructor-level guard would be bypassable through the
//! other two.

use prax_core::query::{CmpOp, Condition, cmp};
use prax_core::types::{Outcome, Want};

/// A whole playtext: a cast, a set of scenes, and the scene to open on
/// (`Prax.Script.Script`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    pub cast: Vec<CastMember>,
    pub scenes: Vec<Scene>,
    /// The id of the opening scene.
    pub start: String,
}

impl Script {
    /// A playtext opening on `start`, with no cast and no scenes yet.
    pub fn new(start: impl Into<String>) -> Script {
        Script {
            cast: Vec::new(),
            scenes: Vec::new(),
            start: start.into(),
        }
    }
    /// Set the cast (declaration order is load-bearing: it is the order the
    /// `character.<c>` facts are asserted and the order the roster enumerates).
    #[must_use]
    pub fn cast(mut self, cast: impl IntoIterator<Item = CastMember>) -> Script {
        self.cast = cast.into_iter().collect();
        self
    }
    /// Set the scenes (declaration order is load-bearing: it is the outer loop
    /// of both the compiled action list and the compiled `story` clause list).
    #[must_use]
    pub fn scenes(mut self, scenes: impl IntoIterator<Item = Scene>) -> Script {
        self.scenes = scenes.into_iter().collect();
        self
    }
}

/// A character in the cast, with its desires written as FOL [`Want`]s
/// (`Prax.Script.CastMember`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastMember {
    pub name: String,
    /// Marks the player-controlled character.
    pub playable: bool,
    pub desires: Vec<Want>,
    /// Personality tags → `trait.<who>.<t>` facts.
    pub traits: Vec<String>,
}

/// One scene: the unit of grouping (`Prax.Script.Scene`). Its beats are
/// available only while it is current; its junctions are the ways it can end or
/// hand off to another scene.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scene {
    pub id: String,
    /// Narration shown on entering the scene. INERT AUTHORED DATA [D-I8]:
    /// `compile` never reads it, `flow_chart` never reads it, no CLI arm prints
    /// it, and the frozen tree is the same. It exists to survive the JSON
    /// round-trip and the AST equality — which is why it is neither wired into
    /// some output (a divergence the differential would catch only if it reached
    /// stdout) nor dropped as dead (which would break both
    /// `decode(encode(x)) == x` and the byte-compatibility of
    /// `examples/play.json`, whose scenes emit `"opening"` unconditionally).
    pub opening: String,
    /// Facts asserted when the scene becomes current.
    pub setup: Vec<Outcome>,
    pub beats: Vec<Beat>,
    pub junctions: Vec<Junction>,
}

impl Scene {
    /// Set the scene's opening narration.
    #[must_use]
    pub fn opening(mut self, text: impl Into<String>) -> Scene {
        self.opening = text.into();
        self
    }
    /// Set the entry outcomes.
    #[must_use]
    pub fn setup(mut self, outs: impl IntoIterator<Item = Outcome>) -> Scene {
        self.setup = outs.into_iter().collect();
        self
    }
    /// Set the scene's beats (declaration order is the compiled action order).
    #[must_use]
    pub fn beats(mut self, beats: impl IntoIterator<Item = Beat>) -> Scene {
        self.beats = beats.into_iter().collect();
        self
    }
    /// Set the scene's junctions (declaration order is the compiled clause
    /// order — the authored-order tiebreak the frozen suite pins four ways).
    #[must_use]
    pub fn junctions(mut self, js: impl IntoIterator<Item = Junction>) -> Scene {
        self.junctions = js.into_iter().collect();
        self
    }
}

/// A beat: a line of dialogue or an affordance a character may take within the
/// scene (`Prax.Script.Beat`). `speaker: Some(c)` restricts it to character `c`
/// (a Prompter quip is spoken by a named actor); `None` leaves it open to any
/// cast member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Beat {
    pub label: String,
    pub speaker: Option<String>,
    pub when: Vec<Condition>,
    pub effects: Vec<Outcome>,
}

/// A junction: a route out of a scene (`Prax.Script.Junction`), fired by the
/// compiled `story` schedule rule at a round boundary as soon as `when` holds
/// AND (if `after` is set) at least that many engine rounds have elapsed since
/// the scene was entered. `to: Some(s)` transitions to scene `s`; `None` ends
/// the story with `name` as the ending key.
///
/// `when` is always 100% author content — it never carries spliced machinery, so
/// `compile` validates it uniformly with the same v40 hygiene guard `setup`
/// gets, regardless of how the `Junction` was built. The timeout machinery (the
/// patience marker scene entry arms and the story clause reads as `Not`) is
/// expanded from `after` at compile time, so it never appears in author-visible
/// data at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Junction {
    pub name: String,
    pub to: Option<String>,
    pub when: Vec<Condition>,
    /// Fire only >= this many rounds after scene entry.
    pub after: Option<i64>,
}

// ---- smart constructors ----------------------------------------------------

/// A non-player cast member with no desires (`Prax.Script.member`; override with
/// [`wanting`]).
pub fn member(n: impl Into<String>) -> CastMember {
    CastMember {
        name: n.into(),
        playable: false,
        desires: Vec::new(),
        traits: Vec::new(),
    }
}

/// The player-controlled cast member (`Prax.Script.player`).
pub fn player(n: impl Into<String>) -> CastMember {
    CastMember {
        playable: true,
        ..member(n)
    }
}

/// Give a cast member desires (`Prax.Script.wanting`). APPENDS, as the frozen
/// does, so it composes with [`concerned_with`] in either order.
#[must_use]
pub fn wanting(mut c: CastMember, ws: impl IntoIterator<Item = Want>) -> CastMember {
    c.desires.extend(ws);
    c
}

/// Sketch a character's **concerns** (`Prax.Script.concernedWith`): each
/// `(dimension, weight)` appends a want that the character be regarded
/// positively on that dimension — `+weight` for every OTHER whose evaluation of
/// them on `dimension` is above zero (the natural positive/negative boundary;
/// the weight is author-supplied, so no magic constants). Composes with
/// [`wanting`].
#[must_use]
pub fn concerned_with<S: Into<String>>(
    mut c: CastMember,
    pairs: impl IntoIterator<Item = (S, i32)>,
) -> CastMember {
    let name = c.name.clone();
    for (dim, w) in pairs {
        let dim = dim.into();
        c.desires.push(Want::new(
            vec![
                Condition::Match(format!("Other.relationship.{name}.{dim}.score!N")),
                Condition::Neq("Other".to_owned(), name.clone()),
                cmp(CmpOp::Gt, "N", "0"),
            ],
            w,
        ));
    }
    c
}

/// Give a character personality **traits** (`Prax.Script.withTraits`) — stored
/// as queryable `trait.<who>.<t>` facts (usable in preconditions). They are
/// deliberately NOT compiled to behaviour: no source specifies a trait→desire
/// mapping.
#[must_use]
pub fn with_traits<S: Into<String>>(
    mut c: CastMember,
    ts: impl IntoIterator<Item = S>,
) -> CastMember {
    c.traits.extend(ts.into_iter().map(Into::into));
    c
}

/// An empty scene with the given id (`Prax.Script.scene`); fill it with the
/// fluent setters.
pub fn scene(sid: impl Into<String>) -> Scene {
    Scene {
        id: sid.into(),
        opening: String::new(),
        setup: Vec::new(),
        beats: Vec::new(),
        junctions: Vec::new(),
    }
}

/// A beat open to any cast member (`Prax.Script.beat`).
pub fn beat(label: impl Into<String>, when: Vec<Condition>, effects: Vec<Outcome>) -> Beat {
    Beat {
        label: label.into(),
        speaker: None,
        when,
        effects,
    }
}

/// A beat spoken by a named character (`Prax.Script.quip`).
pub fn quip(
    speaker: impl Into<String>,
    label: impl Into<String>,
    when: Vec<Condition>,
    effects: Vec<Outcome>,
) -> Beat {
    Beat {
        label: label.into(),
        speaker: Some(speaker.into()),
        when,
        effects,
    }
}

/// A transition junction (`Prax.Script.goto`): hand off to `to` when `when`
/// holds. `when` is validated at [`crate::compile::compile`] time — uniformly
/// with every other junction, however built — rather than here.
pub fn goto(name: impl Into<String>, to: impl Into<String>, when: Vec<Condition>) -> Junction {
    Junction {
        name: name.into(),
        to: Some(to.into()),
        when,
        after: None,
    }
}

/// An ending junction (`Prax.Script.ending`): the ending key is `name`.
pub fn ending(name: impl Into<String>, when: Vec<Condition>) -> Junction {
    Junction {
        name: name.into(),
        to: None,
        when,
        after: None,
    }
}

/// A **timed transition** (`Prax.Script.after`): hand off to `to` once `n`
/// rounds have elapsed in the current scene (Prompter's timeout transition).
pub fn after(name: impl Into<String>, n: i64, to: impl Into<String>) -> Junction {
    Junction {
        name: name.into(),
        to: Some(to.into()),
        when: Vec::new(),
        after: Some(n),
    }
}

/// A **timeout ending** (`Prax.Script.timeout`): end the story with key `name`
/// after `n` rounds in the scene (Prompter's `timeout_conclusion`).
pub fn timeout(name: impl Into<String>, n: i64) -> Junction {
    Junction {
        name: name.into(),
        to: None,
        when: Vec::new(),
        after: Some(n),
    }
}

/// The player-controlled character: the first cast member marked playable
/// (`Prax.Script.scriptPlayer`, whose frozen form is an `error`).
///
/// This function has NO caller inside the workspace at S8, and that is the
/// FROZEN shape, not an oversight [D-I4]: the frozen worlds define `playerName`
/// as a plain string literal (`Play.hs:31`, `Audience.hs:22`), and
/// `scriptPlayer`'s only frozen consumer is `app/Main.hs`'s `play <file>.json`
/// arm — the CLI, which is S9. The surface is prax-script's, so it lands with
/// prax-script and is pinned here.
///
/// # Errors
/// [`prax_core::error::WorldError::NoPlayableCastMember`] if no cast member is
/// playable.
pub fn script_player(scr: &Script) -> Result<&str, prax_core::error::WorldError> {
    scr.cast
        .iter()
        .find(|c| c.playable)
        .map(|c| c.name.as_str())
        .ok_or(prax_core::error::WorldError::NoPlayableCastMember)
}

#[cfg(test)]
mod tests {
    use super::*;

    // H: ScriptSpec.hs "a character sketch compiles concerns to wants and traits to facts"
    #[test]
    fn a_character_sketch_compiles_concerns_to_wants_and_traits_to_facts() {
        let cm = with_traits(concerned_with(member("vain"), [("beauty", 50)]), ["proud"]);
        assert_eq!(
            cm.desires,
            vec![Want::new(
                vec![
                    Condition::Match("Other.relationship.vain.beauty.score!N".to_owned()),
                    Condition::Neq("Other".to_owned(), "vain".to_owned()),
                    cmp(CmpOp::Gt, "N", "0"),
                ],
                50
            )]
        );
        assert_eq!(cm.traits, vec!["proud".to_owned()]);
    }

    /// NATIVE PIN — no frozen label. The frozen `scriptPlayer` is an `error`, so
    /// observing its failure needs `try`/`evaluate` and no frozen test does it.
    /// The Rust makes it a `Result` (S4's loud-error rule) and the S9 CLI is the
    /// consumer that will `?` it — so the contract is pinned here, at its home,
    /// rather than left to be discovered at S9.
    ///
    /// REDDENS UNDER: returning the first cast member regardless of `playable`,
    /// or defaulting to an empty name instead of an error.
    #[test]
    fn script_player_takes_the_first_playable_member_and_is_loud_without_one() {
        let scr = Script::new("s").cast([member("a"), player("b"), player("c")]);
        assert_eq!(script_player(&scr), Ok("b"));
        let none = Script::new("s").cast([member("a")]);
        assert!(script_player(&none).is_err());
    }

    /// NATIVE PIN — no frozen label. `wanting`/`concerned_with`/`with_traits`
    /// all APPEND in the frozen source, which is what lets `audience`'s duke be
    /// written `member "duke" \`concernedWith\` … \`withTraits\` …`. A port that
    /// assigned instead of appending would silently drop the earlier call, and
    /// `audience`'s shape would still look right because the duke has exactly
    /// one of each.
    ///
    /// REDDENS UNDER: replacing either `extend`/`push` with an assignment.
    #[test]
    fn the_sketch_combinators_append_rather_than_replace() {
        let c = with_traits(
            with_traits(
                concerned_with(wanting(member("x"), [Want::new(vec![], 1)]), [("favor", 10)]),
                ["a"],
            ),
            ["b"],
        );
        assert_eq!(c.desires.len(), 2, "the plain want AND the concern's want");
        assert_eq!(c.traits, vec!["a".to_owned(), "b".to_owned()]);
    }
}
