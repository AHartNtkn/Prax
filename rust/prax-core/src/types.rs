//! The two type families and the boundary between them: the string-surfaced
//! authoring AST (`Condition`/`Outcome`/`Practice`/…, what the builder DSL
//! produces) and the interned runtime types (`Cond`/`Effect`/`CompiledPath`).
//! The conversion is the single compile choke point ([`crate::engine`]); there
//! is no runtime mirror duality to keep in sync.
