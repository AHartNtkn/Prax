//! `Prax.AnalysisTableSpec`, re-expressed as NATIVE committed-literal pins.
//!
//! The frozen spec (`test/Prax/AnalysisTableSpec.hs`) renders every derived
//! analysis table a world carries — `contMonotone`, `improvables`, `liveness`,
//! `caresAbout`, `footprint`, `negFootprint`, `axiomHeads` — ONE LINE PER ENTRY
//! in the exact emission order the state holds them, and pins the whole
//! rendering per world against literals captured from the live pre-v41
//! (string-side) analyses. Order is part of the contract: the v41 cooked
//! rewrite had to reproduce the old walkers' emission order, not just their sets.
//!
//! **[R3]/[P1] — these are the DURABLE net, and for FIVE of the seven fields the
//! SOLE one.** Every field rendered already exists in `Compiled` (S4 built
//! footprint/neg_footprint/axiom_heads/cont_monotone; S6 built
//! improvables/liveness/cares_about). The differential channel `worldshape`
//! emits `axiom_heads` ALONE among these tables ([P1] correction —
//! improvables/liveness/caresAbout are emitted only by `fixtures planner` over a
//! SYNTHETIC corpus, never the shipped AnalysisTable worlds). So for
//! `improvables`, `liveness`, `caresAbout`, `footprint`, `negFootprint` the
//! committed literals below are the only net there is. They assert NAMED CONTENT
//! (`clean-conscience`, `spites-carol`, `pursues-earnBread`), so a worldshape
//! "equal-to-frozen" diff would invert the program's authority and evaporate at
//! cut-over; the native re-expression survives deletion. This is NOT a new
//! differential channel and needs no oracle extension.
//!
//! The literals are copied verbatim from the frozen spec. NEVER edit them to
//! match new output — a failure means the Rust `Compiled` tables diverge from
//! the frozen analyses, which is a bug to SURFACE, not to paper over.

#[cfg(test)]
mod tests {
    // H: AnalysisTableSpec.hs "Prax.AnalysisTable"
    //
    // The frozen `Prax.AnalysisTable` test group.
    use prax_core::engine::State;
    use prax_worlds::audience::audience_world;
    use prax_worlds::bar::{bar_director_world, bar_world};
    use prax_worlds::feud::feud_world;
    use prax_worlds::intrigue::intrigue_world;
    use prax_worlds::play::play_world;
    use prax_worlds::village::village_world;

    /// Reproduces the frozen `analysisTable :: PraxState -> [String]` renderer
    /// (`AnalysisTableSpec.hs:24-42`) over the Rust `Compiled` accessors: one
    /// line per entry, in the state's own emission order. `GateCheck` renders
    /// its gates' single `CMatch` path — the only shape `livenessOf` emits;
    /// anything else crashes loudly here, deliberately, exactly as the frozen
    /// `gate [CMatch p] = path p` does.
    fn analysis_table(st: &State) -> Vec<String> {
        let mut out = Vec::new();
        out.push(format!(
            "contMonotone: {}",
            if st.cont_monotone() { "True" } else { "False" }
        ));
        for n in st.improvables() {
            out.push(format!("improvable: {n}"));
        }
        for (n, (tag, gates)) in st.liveness_rendered() {
            let rendered = match tag.as_str() {
                "FloorCheck" => "FloorCheck".to_owned(),
                "AlwaysLive" => "AlwaysLive".to_owned(),
                "GateCheck" => {
                    let parts: Vec<String> = gates
                        .iter()
                        .map(|g| {
                            assert_eq!(
                                g.len(),
                                1,
                                "AnalysisTableSpec: unexpected gate shape (not a single CMatch): {g:?}"
                            );
                            g[0].clone()
                        })
                        .collect();
                    format!("GateCheck {}", parts.join(" | "))
                }
                other => panic!("AnalysisTableSpec: unexpected liveness tag: {other}"),
            };
            out.push(format!("liveness: {n} {rendered}"));
        }
        for (n, actions) in st.cares_about_table() {
            out.push(format!("caresAbout: {n} -> {}", actions.join("; ")));
        }
        for p in st.footprint_names() {
            out.push(format!("footprint: {p}"));
        }
        for p in st.neg_footprint_names() {
            out.push(format!("negFootprint: {p}"));
        }
        for p in st.axiom_head_names() {
            out.push(format!("axiomHead: {p}"));
        }
        out
    }

    // H: AnalysisTableSpec.hs "village"
    #[test]
    fn village() {
        assert_eq!(analysis_table(&village_world()), village_pin());
    }

    // H: AnalysisTableSpec.hs "bar"
    #[test]
    fn bar() {
        assert_eq!(analysis_table(&bar_world()), bar_pin());
    }

    // H: AnalysisTableSpec.hs "bar-director"
    #[test]
    fn bar_director() {
        assert_eq!(analysis_table(&bar_director_world()), bar_director_pin());
    }

    // H: AnalysisTableSpec.hs "intrigue"
    #[test]
    fn intrigue() {
        assert_eq!(analysis_table(&intrigue_world()), intrigue_pin());
    }

    // H: AnalysisTableSpec.hs "feud"
    #[test]
    fn feud() {
        assert_eq!(analysis_table(&feud_world()), feud_pin());
    }

    // H: AnalysisTableSpec.hs "audience"
    #[test]
    fn audience() {
        assert_eq!(analysis_table(&audience_world()), audience_pin());
    }

    // H: AnalysisTableSpec.hs "play"
    #[test]
    fn play() {
        assert_eq!(analysis_table(&play_world()), play_pin());
    }

    // ---- committed literals (verbatim from AnalysisTableSpec.hs) ------------

    fn village_pin() -> Vec<String> {
        [
            "contMonotone: True",
            "improvable: pursues-earnBread",
            "improvable: spites-carol",
            "improvable: punishes-whisper",
            "improvable: suffers-hunger",
            "improvable: drawn-to-market",
            "improvable: smoulders",
            "improvable: clean-conscience",
            "liveness: clean-conscience FloorCheck",
            "liveness: conscience-remembers FloorCheck",
            "liveness: drawn-to-market GateCheck marketDay.square",
            "liveness: punishes-whisper AlwaysLive",
            "liveness: pursues-earnBread AlwaysLive",
            "liveness: smoulders FloorCheck",
            "liveness: spites-carol AlwaysLive",
            "liveness: suffers-hunger FloorCheck",
            "caresAbout: bob -> [Actor]: sweep the square; [Actor]: fetch flour from the mill; [Actor]: bake and earn the loaf; [Actor]: steal the loaf from the stall; [Actor]: tell [Hearer] that [Culprit] stole the loaf; [Actor]: return the loaf with apologies; [Actor]: whisper to [Hearer] that [Culprit] stole the loaf; [Actor]: take up honest work at the stall; [Actor]: eat the loaf; [Actor]: Go to [Place]",
            "caresAbout: carol -> [Actor]: steal the loaf from the stall; [Actor]: confront [Thief] about the theft; [Actor]: tell [Hearer] that [Culprit] stole the loaf; [Actor]: shun [T]; [Actor]: relent toward [T]; [Actor]: whisper to [Hearer] that [Culprit] stole the loaf; [Actor]: confess to [Hearer] about framing [C]; [Actor]: threaten [V] with what you know; [Actor]: buy [E]'s silence; [Actor]: defy [E]; [Actor]: expose [V] to [Hearer]; [Actor]: Go to [Place]",
            "caresAbout: dana -> [Actor]: confront [Thief] about the theft; [Actor]: eye [Thief] with suspicion; [Actor]: shun [T]; [Actor]: relent toward [T]; [Actor]: Go to [Place]",
            "caresAbout: eve -> [Actor]: Go to [Place]",
            "caresAbout: gale -> [Actor]: whisper to [Hearer] that [Culprit] stole the loaf; [Actor]: confess to [Hearer] about framing [C]; [Actor]: Go to [Place]",
            "caresAbout: you -> ",
            "footprint: Regarder.believes.stole.Culprit.loaf",
            "footprint: atoned.Culprit",
            "footprint: regards.Regarder.Culprit.thief",
            "footprint: regards.W0.T.thief",
            "footprint: regards.W.T.thief",
            "footprint: notorious.T.thief",
            "footprint: Regarder.believes.swept.bob",
            "footprint: Regarder.believes.desires.bob.pursues-earnBread.presumed",
            "footprint: trait.M.T",
            "footprint: traitDesire.T.D",
            "footprint: character.P",
            "footprint: P.believes.desires.M.D.presumed",
            "footprint: Regarder.believes.whispered.V.H",
            "footprint: recanted.V",
            "footprint: regards.Regarder.V.slanderer",
            "footprint: regards.W0.T.slanderer",
            "footprint: regards.W.T.slanderer",
            "footprint: notorious.T.slanderer",
            "footprint: PraxW.believes.whispered.V.H0",
            "footprint: PraxW.believes.whispered.V.H",
            "footprint: regards.PraxW.V.incorrigible",
            "footprint: obliged.Obligor.Regarder.believes.swept.bob",
            "footprint: obliged.Obligor.Regarder.believes.desires.bob.pursues-earnBread.presumed",
            "footprint: obliged.Obligor.trait.M.T",
            "footprint: obliged.Obligor.traitDesire.T.D",
            "footprint: obliged.Obligor.character.P",
            "footprint: obliged.Obligor.P.believes.desires.M.D.presumed",
            "negFootprint: atoned.Culprit",
            "negFootprint: recanted.V",
            "axiomHead: regards.Regarder.Culprit.thief",
            "axiomHead: notorious.T.thief",
            "axiomHead: Regarder.believes.desires.bob.pursues-earnBread.presumed",
            "axiomHead: P.believes.desires.M.D.presumed",
            "axiomHead: regards.Regarder.V.slanderer",
            "axiomHead: notorious.T.slanderer",
            "axiomHead: regards.PraxW.V.incorrigible",
            "axiomHead: obliged.Obligor.Regarder.believes.desires.bob.pursues-earnBread.presumed",
            "axiomHead: obliged.Obligor.P.believes.desires.M.D.presumed",
            "axiomHead: contradiction",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn bar_pin() -> Vec<String> {
        [
            "contMonotone: True",
            "caresAbout: ada -> [Actor]: Disapprove of [Offender]; [Actor]: turn [X] against [Y] to stir up the evening; [Actor]: Greet [Other]; [Actor]: Strike up a conversation with [Other]; [Actor]: Greet [Greeter] back; [Actor]: Rebuff [Greeter]; [Actor]: Take offense that [Greeted] ignored your greeting; [Actor]: Order [Beverage]; [Actor]: Fulfill [Customer]'s order; [Actor]: Go to [Place]",
            "caresAbout: bex -> [Actor]: settle in, feeling you belong here; [Actor]: give up on the evening, resigning yourself to solitude; [Actor]: Disapprove of [Offender]; [Actor]: turn [X] against [Y] to stir up the evening; [Actor]: Greet [Other]; [Actor]: Strike up a conversation with [Other]; [Actor]: Buy [Other] a drink; [Actor]: Greet [Greeter] back; [Actor]: Rebuff [Greeter]; [Actor]: Take offense that [Greeted] ignored your greeting; [Actor]: Tip [Bartender]; [Actor]: Leave [Bartender]'s tab unpaid; [Actor]: Order [Beverage]; [Actor]: Fulfill [Customer]'s order; [Actor]: Drink the [Beverage]; [Actor]: Go to [Place]",
            "caresAbout: director -> [Actor]: turn [X] against [Y] to stir up the evening",
            "caresAbout: you -> ",
            "axiomHead: contradiction",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn bar_director_pin() -> Vec<String> {
        [
            "contMonotone: True",
            "caresAbout: ada -> [Actor]: stir up a rivalry between [X] and [Y]; [Actor]: Disapprove of [Offender]; [Actor]: Greet [Other]; [Actor]: Strike up a conversation with [Other]; [Actor]: Greet [Greeter] back; [Actor]: Rebuff [Greeter]; [Actor]: Take offense that [Greeted] ignored your greeting; [Actor]: Order [Beverage]; [Actor]: Fulfill [Customer]'s order; [Actor]: Go to [Place]",
            "caresAbout: bex -> [Actor]: settle in, feeling you belong here; [Actor]: give up on the evening, resigning yourself to solitude; [Actor]: stir up a rivalry between [X] and [Y]; [Actor]: Disapprove of [Offender]; [Actor]: Greet [Other]; [Actor]: Strike up a conversation with [Other]; [Actor]: Buy [Other] a drink; [Actor]: Greet [Greeter] back; [Actor]: Rebuff [Greeter]; [Actor]: Take offense that [Greeted] ignored your greeting; [Actor]: Tip [Bartender]; [Actor]: Leave [Bartender]'s tab unpaid; [Actor]: Order [Beverage]; [Actor]: Fulfill [Customer]'s order; [Actor]: Drink the [Beverage]; [Actor]: Go to [Place]",
            "caresAbout: cai -> [Actor]: settle in, feeling you belong here; [Actor]: give up on the evening, resigning yourself to solitude; [Actor]: stir up a rivalry between [X] and [Y]; [Actor]: Disapprove of [Offender]; [Actor]: Greet [Other]; [Actor]: Strike up a conversation with [Other]; [Actor]: Buy [Other] a drink; [Actor]: Greet [Greeter] back; [Actor]: Rebuff [Greeter]; [Actor]: Take offense that [Greeted] ignored your greeting; [Actor]: Order [Beverage]; [Actor]: Fulfill [Customer]'s order; [Actor]: Drink the [Beverage]; [Actor]: Go to [Place]",
            "caresAbout: director -> ",
            "axiomHead: contradiction",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn intrigue_pin() -> Vec<String> {
        [
            "contMonotone: True",
            "improvable: kill-artus",
            "liveness: kill-artus AlwaysLive",
            "caresAbout: artus -> ",
            "caresAbout: cassia -> [Actor]: slip poison into [Target]'s cup; [Actor]: poison [Target] with your own hand",
            "caresAbout: marcus -> ",
            "axiomHead: contradiction",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn feud_pin() -> Vec<String> {
        [
            "contMonotone: True",
            "caresAbout: alice -> [Actor]: shun [Target]",
            "caresAbout: bob -> [Actor]: shun [Target]",
            "caresAbout: carol -> [Actor]: shun [Target]",
            "caresAbout: dave -> [Actor]: shun [Target]",
            "caresAbout: esme -> [Actor]: shun [Target]",
            "footprint: allied.X.Y",
            "footprint: allied.Y.X",
            "footprint: wronged.X.Y",
            "footprint: resents.Y.X",
            "footprint: resents.A.B",
            "footprint: allied.A.C",
            "footprint: resents.C.B",
            "footprint: member.X.F",
            "footprint: member.Y.F",
            "footprint: allied.X.Y",
            "footprint: married.A.B",
            "footprint: married.B.A",
            "footprint: parent.P.X",
            "footprint: parent.P.Y",
            "footprint: sibling.X.Y",
            "footprint: parent.G.P",
            "footprint: parent.P.C",
            "footprint: grandparent.G.C",
            "footprint: married.A.B",
            "footprint: parent.P.A",
            "footprint: inLaw.P.B",
            "footprint: married.A.B",
            "footprint: sibling.A.S",
            "footprint: inLaw.S.B",
            "axiomHead: allied.Y.X",
            "axiomHead: resents.Y.X",
            "axiomHead: resents.C.B",
            "axiomHead: allied.X.Y",
            "axiomHead: married.B.A",
            "axiomHead: sibling.X.Y",
            "axiomHead: grandparent.G.C",
            "axiomHead: inLaw.P.B",
            "axiomHead: inLaw.S.B",
            "axiomHead: contradiction",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn audience_pin() -> Vec<String> {
        [
            "contMonotone: True",
            "caresAbout: duke -> envoy: flatter the king; duke: flatter the king",
            "caresAbout: envoy -> ",
            "caresAbout: king -> ",
            "axiomHead: contradiction",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn play_pin() -> Vec<String> {
        [
            "contMonotone: True",
            "caresAbout: artus -> ",
            "caresAbout: cassia -> cassia: slip poison into artus's cup; marcus: poison artus with your own hand",
            "caresAbout: marcus -> ",
            "axiomHead: contradiction",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }
}
