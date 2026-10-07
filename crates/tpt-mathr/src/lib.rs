// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Thin facade over the `tpt-mathr` engine crates.
//!
//! Users see one crate with a stable, small API surface; the implementation
//! is a workspace of focused crates that may reorganise freely. Re-exports
//! are feature-flagged so minimal builds only pay for what they use:
//!
//! ```text
//! tpt-mathr = { version = "0.1", features = ["arith"] }
//! ```
//!
//! The public API never exposes limb types, arenas, or parallelism
//! (standing decision): callers see `BigInt`, not `BigInt<u64>`.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]

pub use tpt_mathr_base::{CancellationToken, Error, ResourceLimits, TptStatus};

/// Arbitrary-precision integers and modular arithmetic (feature `arith`).
#[cfg(feature = "arith")]
pub use tpt_mathr_arith as arith;

/// Polynomial arithmetic (feature `poly`).
#[cfg(feature = "poly")]
pub use tpt_mathr_poly as poly;

/// Proof-aware symbolic rewriting (feature `symbolic`).
#[cfg(feature = "symbolic")]
pub use tpt_mathr_symbolic as symbolic;

/// Mathematical search (feature `search`).
#[cfg(feature = "search")]
pub use tpt_mathr_search as search;

/// Certificate checking (feature `verify`).
#[cfg(feature = "verify")]
pub use tpt_mathr_verify as verify;

/// Crate version, mirroring the workspace release.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn base_types_are_always_available() {
        let limits = crate::ResourceLimits::default().with_max_integer_bits(1024);
        assert!(limits.check_integer_bits(512).is_ok());
        assert!(!crate::CancellationToken::new().is_cancelled());
        assert!(crate::TptStatus::Ok.is_ok());
    }
}
