//! The condition language over `unify`: the authoring `Condition` (string
//! operands) and its cooked runtime mirror (interned operands). Match/Not/Eq/
//! Neq/Cmp/Calc/Count/Subquery/Or/Absent/Exists, evaluated left to right
//! threading bindings (the list-monad nondeterminism of unification).
