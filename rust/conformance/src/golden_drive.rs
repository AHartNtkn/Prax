//! The decision-sequence goldens: `Prax.GoldenDriveSpec`, re-expressed.
//!
//! One planner-driven turn per cast member per round; the named character idles.
//! Each turn contributes one line: `"<actor>: <label>"` for a performed action,
//! `"<actor>: -"` for idle/no move. The sequences ARE the planner's contract —
//! any change that perturbs a single decision fails here.
//!
//! The expected sequence is LOADED from `conformance/goldens/<name>.txt`
//! ([`crate::goldens::load`]) and never typed here [D-C3(b)]: the file is
//! extracted from the FROZEN tree and `scripts/golden-check.sh` holds it
//! byte-identical to the frozen literal, so "adjust the golden" is not a
//! reachable move. `no_inline_golden_literals` sweeps this file like every
//! other.
//!
//! The worlds arrive with their slices: intrigue at slice 2, bar at slice 3,
//! village at slice 4.

use prax_core::engine::State;
use prax_core::turn::{advance, npc_act};

/// `Prax.GoldenDriveSpec.driveLabels`: `n` turns, the named character idling.
pub fn drive_labels(st: &mut State, n: usize, idle: Option<&str>) -> Vec<String> {
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let actor = advance(st);
        let name = actor.name.clone();
        if Some(name.as_str()) == idle {
            out.push(format!("{name}: -"));
            continue;
        }
        let label = npc_act(st, 2, &actor).map_or_else(|| "-".to_owned(), |ga| ga.label);
        out.push(format!("{name}: {label}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goldens::load;

    // H: GoldenDriveSpec.hs "Prax.GoldenDrive (decision-sequence exactness)"
    //
    // The frozen `Prax.GoldenDriveSpec`. Its three world cases land one per
    // slice; this file grows with them.

    // H: GoldenDriveSpec.hs "intrigue: 12 turns, decision for decision"
    #[test]
    fn intrigue_twelve_turns_decision_for_decision() {
        let mut st = prax_worlds::intrigue::intrigue_world();
        let got = drive_labels(&mut st, 12, None);
        for (i, line) in got.iter().enumerate() {
            println!("{i:2}: {line}");
        }
        assert_eq!(
            got,
            load("intrigue-12"),
            "the intrigue decision sequence must match the frozen golden turn for turn"
        );
    }

    // H: GoldenDriveSpec.hs "bar: 12 turns, decision for decision"
    #[test]
    fn bar_twelve_turns_decision_for_decision() {
        // `driveLabels 12 Nothing barWorld`: no idler, so the director's four
        // turns show as `director: -` — it holds no available metalevel move
        // while the room is still cold. That is the shape difference from
        // `runNpcTicks`, which omits an idle turn entirely (see
        // `crate::loop_bar`).
        let mut st = prax_worlds::bar::bar_world();
        let got = drive_labels(&mut st, 12, None);
        for (i, line) in got.iter().enumerate() {
            println!("{i:2}: {line}");
        }
        assert_eq!(
            got,
            load("bar-12"),
            "the bar decision sequence must match the frozen golden turn for turn"
        );
    }
}
