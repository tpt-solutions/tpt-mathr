//! Reproducible, cancellable, certificate-producing search (spec §13-§14).
//!
//! Planned contents (Phase 9): `SearchProblem`/`SearchEngine` traits, the
//! result taxonomy (Found / NotFoundWithinBounds / Exhausted / Inconclusive
//! / Verified / Counterexample — "not found" is never "proved absent"),
//! deterministic work-unit identity, resumable jobs, execution backends.
//!
//! Status: not yet implemented; see docs/spec.md Phase 9.

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

extern crate alloc;

/// Crate version, mirroring the workspace release.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
