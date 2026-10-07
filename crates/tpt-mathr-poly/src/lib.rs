//! High-performance polynomial arithmetic over exact coefficients (spec §11).
//!
//! Planned contents (Phase 5): `Polynomial<T>`, `SparsePolynomial<T>`,
//! `FormalPowerSeries<T>`; add/sub/mul/div/gcd/evaluate/differentiate/
//! integrate/compose/interpolate; adaptive multiplication with
//! benchmark-derived thresholds; `max_polynomial_degree` enforcement.
//!
//! `tpt-mathr-poly` is the computational substrate *underneath*
//! `tpt-math-symbolic`, not an independent CAS (spec §3.3). Status: not yet
//! implemented; see docs/spec.md Phase 5.

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

extern crate alloc;

/// Crate version, mirroring the workspace release.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
