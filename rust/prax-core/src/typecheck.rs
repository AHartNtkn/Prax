//! The static well-formedness checker: unbound variables, exclusion-cardinality
//! clashes, dangling references, dead conditions, reserved-family touches, an
//! unseeded die, unclosed obligations, unmotivated coercions. Sound and
//! declaration-free (no false positives), so the report is trustworthy.
