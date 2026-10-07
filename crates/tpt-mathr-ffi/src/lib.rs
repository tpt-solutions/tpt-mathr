//! C ABI, Python and WASM bindings (spec §18-§21).
//!
//! The ABI contract (v1.2 delta #5): opaque handles, canonical
//! `TptStatus f(..., T** out)` out-parameters, explicit free functions, no
//! panics across the boundary, ABI versioning. Implemented in Phase 11.
//!
//! Status: skeleton only.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
// This crate is the one sanctioned `unsafe` boundary (with future WASM
// glue); every `unsafe` block carries a `// SAFETY:` comment and is covered
// by docs/unsafe-policy.md. The safe wrapper modules forbid unsafe locally.

/// Crate version, mirroring the workspace release.
pub mod version {
    /// Crate version, mirroring the workspace release.
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
}
