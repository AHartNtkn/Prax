//! The v44 authoring surface for engine time (`Prax.Schedule`): the combinators
//! that BUILD recurring rules and lifetimes. Firing itself is the engine's — the
//! round boundary ([`crate::engine::State::round_boundary`]) advances the clock,
//! fires due expiries (a fact with lifetime `n` is present rounds
//! `onset..onset+n-1` and gone at the boundary; expiries fire before rules), then
//! due rules in declaration order. A world installs its schedule with
//! [`crate::engine::State::set_schedule`], beside `set_desires`/`define_practices`.
//!
//! The [`ScheduleRule`] TYPE lives in [`crate::types`] (one home, no re-export);
//! this module is the combinator surface only. These are pure value-builders — a
//! `lasts`/`gathering` that cannot mean anything is a loud [`WorldError`], never a
//! silent nonsense rule.

use crate::error::WorldError;
use crate::query::Condition;
use crate::types::{Outcome, ScheduleRule};

/// Wrap a plain insert with a lifetime (`Prax.Schedule.lasts`): the asserted fact
/// lives `n` round boundaries, then the engine retracts it (the ONE expiry
/// mechanism, [`Outcome::InsertFor`]). Loud on anything but an `Insert` — a
/// lifetime on a `Delete`/`Call`/`ForEach`/`Roll` has no meaning. NOTE the retract
/// takes the path's whole subtree, so lifetimes belong on leaf facts.
pub fn lasts(n: i64, o: Outcome) -> Result<Outcome, WorldError> {
    match o {
        Outcome::Insert(s) => Ok(Outcome::InsertFor(n, s)),
        other => Err(WorldError::LifetimeOnNonInsert {
            outcome: format!("{other:?}"),
        }),
    }
}

/// A recurring convening (`Prax.Schedule.gathering`): ONE rule whose open effects
/// fire every `period` boundaries and whose asserted facts live `duration`
/// boundaries (the close rule of the v37 design is subsumed by expiry — one
/// mechanism for a temporary fact). Requires `0 < duration < period`: a gathering
/// that never opens, or one whose opening never lapses before the next, is not a
/// gathering.
pub fn gathering(
    name: impl Into<String>,
    period: i64,
    duration: i64,
    open_outs: Vec<Outcome>,
) -> Result<ScheduleRule, WorldError> {
    let name = name.into();
    if duration < 1 || duration >= period {
        return Err(WorldError::GatheringDuration {
            name,
            period,
            duration,
        });
    }
    let then = open_outs
        .into_iter()
        .map(|o| lasts(duration, o))
        .collect::<Result<Vec<_>, WorldError>>()?;
    Ok(ScheduleRule::new(name, period).clause(Vec::<Condition>::new(), then))
}

/// Perception as a period-1 recurring rule (`Prax.Schedule.sightRule`): the
/// authored sighting template, stamped with the clock read as a fact (`turn!Now`).
/// `Now`/`Seer`/`Seen`/`Spot` are its CONTRACT variables — the template binds them
/// and the outcomes read them straight back out — so, like every other rule body,
/// only the `Prax` namespace and `Actor` are forbidden in the authored `sighting`
/// ([`crate::engine::State::set_schedule`] enforces it).
pub fn sight_rule(sighting: Vec<Condition>) -> ScheduleRule {
    let mut when = sighting;
    when.push(Condition::Neq("Seer".to_owned(), "Seen".to_owned()));
    when.push(Condition::Match("turn!Now".to_owned()));
    ScheduleRule::new("sight", 1).clause(
        when,
        [
            Outcome::Insert("Seer.believes.at.Seen!Spot".to_owned()),
            Outcome::Insert("Seer.believes.atSince.Seen!Now".to_owned()),
        ],
    )
}

#[cfg(test)]
mod tests {
    //! The Schedule authoring combinators' own mechanics. `gathering`'s
    //! construction guards and firing are pinned by ScheduleRuleSpec (in
    //! conformance); these cover `lasts`'s only-Insert law and `sight_rule`'s
    //! template shape, which the frozen tests exercise through SightSpec (owned by
    //! S6) rather than a dedicated combinator label.
    use super::*;
    use crate::types::{delete, insert};

    #[test]
    fn lasts_wraps_an_insert_as_insert_for() {
        assert_eq!(
            lasts(3, insert("mood!a")).unwrap(),
            Outcome::InsertFor(3, "mood!a".to_owned())
        );
    }

    #[test]
    fn lasts_rejects_a_non_insert() {
        assert!(matches!(
            lasts(3, delete("mood.a")),
            Err(WorldError::LifetimeOnNonInsert { .. })
        ));
    }

    #[test]
    fn gathering_maps_lasts_over_the_open_outcomes() {
        let r = gathering("fair", 3, 1, vec![insert("marketDay.now")]).unwrap();
        assert_eq!(r.name, "fair");
        assert_eq!(r.period, 3);
        assert_eq!(
            r.body,
            vec![(
                Vec::<Condition>::new(),
                vec![Outcome::InsertFor(1, "marketDay.now".to_owned())]
            )]
        );
    }

    #[test]
    fn gathering_rejects_duration_at_or_over_period() {
        assert!(matches!(
            gathering("fair", 3, 3, vec![insert("x")]),
            Err(WorldError::GatheringDuration { .. })
        ));
        assert!(matches!(
            gathering("fair", 3, 0, vec![insert("x")]),
            Err(WorldError::GatheringDuration { .. })
        ));
    }

    #[test]
    fn sight_rule_stamps_the_clock_and_the_believes_outcomes() {
        let r = sight_rule(vec![Condition::Match("at.Seer!Spot".to_owned())]);
        assert_eq!(r.name, "sight");
        assert_eq!(r.period, 1);
        let (when, then) = &r.body[0];
        // the authored sighting, then the contract guards appended in order.
        assert_eq!(
            when,
            &vec![
                Condition::Match("at.Seer!Spot".to_owned()),
                Condition::Neq("Seer".to_owned(), "Seen".to_owned()),
                Condition::Match("turn!Now".to_owned()),
            ]
        );
        assert_eq!(
            then,
            &vec![
                Outcome::Insert("Seer.believes.at.Seen!Spot".to_owned()),
                Outcome::Insert("Seer.believes.atSince.Seen!Now".to_owned()),
            ]
        );
    }
}
