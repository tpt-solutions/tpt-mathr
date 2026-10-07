//! Proof-aware symbolic rewriting with packed e-graph storage (spec §12).
//!
//! Planned contents (Phase 7): proof-aware expression representation,
//! canonical forms, interned nodes with stable IDs, a long-lived e-graph in
//! packed indexed storage (`Vec<EClass>`/`Vec<ENode>`), arena allocation for
//! short-lived rewrite candidates, and rewrite certificates.
//!
//! Status: not yet implemented; see docs/spec.md Phase 7.

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

extern crate alloc;

/// Crate version, mirroring the workspace release.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
