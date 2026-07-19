//! `WorldError` — construction-guard failures as `Result<_, WorldError>`
//! (thiserror; loud). Engine-invariant breaches panic instead; a detected
//! contradiction stays a queryable fact, never an error here (PLAN.md, Errors).
