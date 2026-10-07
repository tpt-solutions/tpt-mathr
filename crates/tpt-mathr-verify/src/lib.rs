//! Independent certificate checkers and assurance profiles (spec §15-§16).
//!
//! Planned contents (Phase 8): the certificate model and serialisation,
//! local checkers forming a small trusted kernel (a checker never converts
//! resource exhaustion into acceptance), assurance profiles (Fast, Checked,
//! Certified, Formal, KernelVerified), Lean/Coq export.
//!
//! Status: not yet implemented; see docs/spec.md Phase 8.

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

extern crate alloc;

/// Crate version, mirroring the workspace release.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
