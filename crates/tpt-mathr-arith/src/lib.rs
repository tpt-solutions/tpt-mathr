// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native Rust arbitrary-precision arithmetic (spec §5.1).
//!
//! This crate is the engine's integer layer:
//!
//! * [`BigUint`] — non-negative integers, canonical normalised limbs;
//! * [`BigInt`] — sign + magnitude, canonical zero, truncated division;
//! * modular arithmetic, primality — arriving with Phase 3;
//! * [`Progress`] — the limits + cancellation context threaded through
//!   every fallible operation.
//!
//! # Architectural requirements (v1.2 deltas)
//!
//! * **Fallible allocation.** Every growth path allocates through
//!   `try_reserve`-based helpers; allocation failure is
//!   [`Error::AllocationFailure`](tpt_mathr_base::Error::AllocationFailure)/[`Error::ResourceExhausted`](tpt_mathr_base::Error::ResourceExhausted), never a
//!   process abort, and a failed operation leaves inputs untouched (I8).
//! * **Explicit budgets.** Operations accept a `&Progress`; untrusted
//!   inputs go through entry points that enforce `ResourceLimits` (I9).
//!   The pure value API passes [`Progress::UNLIMITED`].
//! * **Cooperative cancellation.** Long operations poll the token at
//!   safety points and unwind via [`Error::Cancelled`](tpt_mathr_base::Error::Cancelled).
//! * **No public limb exposure.** The limb width is private and
//!   platform-conditional (u64 on 64-bit, u32 on 32-bit/wasm32); the
//!   public types are `BigUint`/`BigInt`, never `BigInt<Limb>` (delta A3).
//! * **Determinism.** No hidden global state; identical inputs give
//!   identical results (I10).
//!
//! # Example
//!
//! ```
//! use tpt_mathr_arith::{BigInt, Progress};
//!
//! let p = Progress::UNLIMITED;
//! let a = BigInt::from_str_radix("123456789012345678901234567890", 10, &p)?;
//! let b = a.mul(&a, &p)?;
//! assert_eq!(b.to_str_radix(10, &p)?.len(), 59);
//!
//! // Budgets make oversized work fail loudly instead of consuming the machine:
//! let tight = Progress::new(tpt_mathr_base::ResourceLimits::restrictive());
//! assert!(a.pow(1000, &tight).unwrap_err().is_resource_exhausted());
//! # Ok::<(), tpt_mathr_base::Error>(())
//! ```

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs, missing_debug_implementations)]
#![warn(clippy::pedantic)]
// Limb arithmetic is cast-dense by nature: buffer indices are `usize`,
// products and carries live in `DoubleLimb`, and limbs are `u64`/`u32`
// depending on target. Every cast site is bounded by a buffer size or a
// mask; per-site annotations would repeat this paragraph ~30 times. The
// lossless/lossy distinction is still enforced for anything *outside*
// limb arithmetic (conversions to primitives use TryFrom + errors).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    reason = "limb arithmetic casts are bounded by buffer sizes and masks by construction"
)]

extern crate alloc;

// The test harness links std even in a no_std crate.
#[cfg(test)]
extern crate std;

pub mod bigint;
pub mod biguint;
mod convert;
mod division;
mod limb;
pub mod modular;
pub mod ntt;
pub mod numtheory;
pub mod primality;
pub mod progress;
pub mod radix;
pub mod rational;
mod strategy;

pub use bigint::{BigInt, ExtendedGcd, Sign};
pub use biguint::BigUint;
pub use progress::Progress;
pub use radix::{MAX_RADIX, MIN_RADIX};
pub use modular::{mod_inverse, pow_mod, ModInt};
pub use primality::{factorise, Primality};
pub use rational::BigRat;

/// Test-support entry points (feature `testing`, doc-hidden): reference
/// implementations for differential checks of optimised paths. Never part
/// of the public API contract; never enabled by default.
#[cfg(feature = "testing")]
#[doc(hidden)]
pub mod test_support {
    use crate::biguint::BigUint;
    use crate::progress::Progress;
    use crate::strategy::MulStrategy;

    /// Multiply via the schoolbook reference strategy, bypassing the
    /// threshold dispatch. Used by integration tests to pin the strategy
    /// table to the reference path.
    /// Build a `BigUint` from little-endian u64 limbs (normalised).
    /// Test scaffolding for random value generation; on 32-bit targets the
    /// u64 values are the natural two-limb splits.
    #[must_use]
    pub fn from_u64_limbs(limbs: &[u64]) -> BigUint {
        BigUint::from_limbs(
            limbs
                .iter()
                .copied()
                .map(crate::limb::limb_from_u64_lossy)
                .collect(),
        )
    }

    /// Multiply via the schoolbook reference strategy, bypassing the
    /// threshold dispatch. Used by integration tests to pin the strategy
    /// table to the reference path.
    #[must_use]
    pub fn mul_schoolbook(a: &BigUint, b: &BigUint) -> BigUint {
        let limbs = crate::strategy::Schoolbook
            .multiply(a.as_limbs(), b.as_limbs(), &Progress::UNLIMITED)
            .expect("schoolbook reference multiply allocates");
        BigUint::from_limbs(limbs)
    }
}

/// Crate version, mirroring the workspace release.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn public_types_do_not_leak_limb_generics() {
        // Delta A3: the public API is `BigUint`/`BigInt`, not `BigInt<Limb>`.
        let v = crate::BigInt::from(7_i64);
        assert_eq!(v.to_i64().unwrap(), 7);
    }
}
