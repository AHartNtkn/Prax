//! Forward-chaining derivation: domain axioms `body → head` closed to a
//! fixpoint (the paper's canonical model `m(G,A)`). Heads are `meet`-ed into the
//! model so a forced exclusive slot yields `⊥` — a detected contradiction, never
//! a silent overwrite. Closure is a view (the base stays the source of truth),
//! so a conclusion whose premise is retracted simply disappears.
