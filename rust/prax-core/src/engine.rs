//! The interpreter: discovering an actor's affordances (`possibleActions`) and
//! applying an action's effects (`performAction`), plus the retable heir that
//! rebuilds the derived tables (cooked defs, footprints, liveness) in lockstep
//! with any vocabulary change. Pure state transformations — the planner
//! speculatively applies and discards. A contradiction surfaces as a queryable
//! fact; an engine-invariant breach panics.
