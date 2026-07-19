//! Replay of the committed unit-fixture corpora
//! (`conformance/fixtures/{db,el,query,derive}.json`) against the Rust engine:
//! each recorded input is recomputed and checked byte-for-byte against the
//! frozen Haskell's recorded output. Populated as S1–S3 land the engine paths.
