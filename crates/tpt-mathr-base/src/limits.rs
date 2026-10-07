// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Explicit resource budgets (v1.2 review delta #1: "add resource limits").
//!
//! A mathematical engine that searches enormous spaces must fail *fast and
//! legibly* when a job is bigger than its budget. A worker in a distributed
//! search should not have to discover it consumed 64 GB of RAM before
//! failing; it should hit a configured [`ResourceLimits`] check and return
//! [`Error::ResourceExhausted`].
//!
//! # Cooperative enforcement
//!
//! Limits are **cooperative**: they are ordinary values passed explicitly to
//! operations that accept them. The engine contains no hidden global state
//! (spec §2.5), so there is no process-global limit either. Enforcement
//! points follow one rule:
//!
//! > Every *growth path* — a place where a buffer, exponent, degree, or
//! > recursion depth is about to increase — consults the limits in scope
//! > before growing, and every *entry point fed by untrusted input*
//! > (parsers, deserialisers, network/FFI messages) requires limits to be
//! > in scope.
//!
//! Time is special: measuring elapsed time needs a clock, which is a hosted
//! capability. The [`ResourceLimits`] struct always *stores*
//! `max_execution_time` (a [`core::time::Duration`], no_std-safe), while the
//! *checking* of a running deadline lives behind the `std` feature in
//! [`crate::time::Deadline`].

use core::time::Duration;

use crate::error::{Error, ResourceKind};

/// Explicit budgets for engine operations. `None` means "no configured
/// limit"; the [`Default`] implementation is fully unlimited.
///
/// Construction uses the builder pattern so new fields can be added without
/// breaking callers:
///
/// ```
/// use core::time::Duration;
/// use tpt_mathr_base::ResourceLimits;
///
/// let limits = ResourceLimits::default()
///     .with_max_integer_bits(1 << 20)
///     .with_max_memory_bytes(256 * 1024 * 1024)
///     .with_max_execution_time(Duration::from_secs(30));
/// assert_eq!(limits.max_integer_bits(), Some(1 << 20));
/// ```
#[allow(
    clippy::struct_field_names,
    reason = "`max_*` field names are the spec's vocabulary (v1.2 delta: add resource limits)"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResourceLimits {
    max_memory_bytes: Option<u64>,
    max_integer_bits: Option<u64>,
    max_polynomial_degree: Option<u64>,
    max_search_depth: Option<u64>,
    max_certificate_size_bytes: Option<u64>,
    max_execution_time: Option<Duration>,
}

impl ResourceLimits {
    /// The unlimited limit set as a `const`, usable in `const` contexts
    /// where [`Default::default`] cannot appear.
    pub const DEFAULT: Self = Self {
        max_memory_bytes: None,
        max_integer_bits: None,
        max_polynomial_degree: None,
        max_search_depth: None,
        max_certificate_size_bytes: None,
        max_execution_time: None,
    };

    /// A limit set that denies anything non-trivial — useful for tests that
    /// exercise the exhaustion paths.
    #[must_use]
    pub fn restrictive() -> Self {
        Self::default()
            .with_max_integer_bits(1024)
            .with_max_memory_bytes(64 * 1024)
            .with_max_polynomial_degree(256)
            .with_max_search_depth(64)
            .with_max_certificate_size_bytes(64 * 1024)
            .with_max_execution_time(Duration::from_millis(100))
    }

    /// Set the total memory budget in bytes.
    #[must_use]
    pub const fn with_max_memory_bytes(mut self, bytes: u64) -> Self {
        self.max_memory_bytes = Some(bytes);
        self
    }

    /// Set the maximum size (in bits) of any integer operand.
    #[must_use]
    pub const fn with_max_integer_bits(mut self, bits: u64) -> Self {
        self.max_integer_bits = Some(bits);
        self
    }

    /// Set the maximum polynomial degree.
    #[must_use]
    pub const fn with_max_polynomial_degree(mut self, degree: u64) -> Self {
        self.max_polynomial_degree = Some(degree);
        self
    }

    /// Set the maximum search depth.
    #[must_use]
    pub const fn with_max_search_depth(mut self, depth: u64) -> Self {
        self.max_search_depth = Some(depth);
        self
    }

    /// Set the maximum serialised certificate size in bytes.
    #[must_use]
    pub const fn with_max_certificate_size_bytes(mut self, bytes: u64) -> Self {
        self.max_certificate_size_bytes = Some(bytes);
        self
    }

    /// Set the wall-clock execution-time budget.
    #[must_use]
    pub const fn with_max_execution_time(mut self, time: Duration) -> Self {
        self.max_execution_time = Some(time);
        self
    }

    /// The memory budget, if configured.
    #[must_use]
    pub const fn max_memory_bytes(&self) -> Option<u64> {
        self.max_memory_bytes
    }

    /// The integer-bit budget, if configured.
    #[must_use]
    pub const fn max_integer_bits(&self) -> Option<u64> {
        self.max_integer_bits
    }

    /// The polynomial-degree budget, if configured.
    #[must_use]
    pub const fn max_polynomial_degree(&self) -> Option<u64> {
        self.max_polynomial_degree
    }

    /// The search-depth budget, if configured.
    #[must_use]
    pub const fn max_search_depth(&self) -> Option<u64> {
        self.max_search_depth
    }

    /// The certificate-size budget, if configured.
    #[must_use]
    pub const fn max_certificate_size_bytes(&self) -> Option<u64> {
        self.max_certificate_size_bytes
    }

    /// The execution-time budget, if configured.
    #[must_use]
    pub const fn max_execution_time(&self) -> Option<Duration> {
        self.max_execution_time
    }

    /// Check that an integer of `bits` bits is within budget.
    ///
    /// # Errors
    ///
    /// [`Error::ResourceExhausted`] with [`ResourceKind::IntegerBits`] when
    /// the budget is exceeded.
    pub fn check_integer_bits(&self, bits: u64) -> Result<(), Error> {
        match self.max_integer_bits {
            Some(max) if bits > max => Err(Error::ResourceExhausted {
                kind: ResourceKind::IntegerBits,
            }),
            _ => Ok(()),
        }
    }

    /// Check that `bytes` more memory is within budget.
    ///
    /// # Errors
    ///
    /// [`Error::ResourceExhausted`] with [`ResourceKind::Memory`] when the
    /// budget is exceeded.
    pub fn check_memory(&self, bytes: u64) -> Result<(), Error> {
        match self.max_memory_bytes {
            Some(max) if bytes > max => Err(Error::ResourceExhausted {
                kind: ResourceKind::Memory,
            }),
            _ => Ok(()),
        }
    }

    /// Check that a polynomial of degree `degree` is within budget.
    ///
    /// # Errors
    ///
    /// [`Error::ResourceExhausted`] with [`ResourceKind::PolynomialDegree`]
    /// when the budget is exceeded.
    pub fn check_polynomial_degree(&self, degree: u64) -> Result<(), Error> {
        match self.max_polynomial_degree {
            Some(max) if degree > max => Err(Error::ResourceExhausted {
                kind: ResourceKind::PolynomialDegree,
            }),
            _ => Ok(()),
        }
    }

    /// Check that descending to search `depth` is within budget.
    ///
    /// # Errors
    ///
    /// [`Error::ResourceExhausted`] with [`ResourceKind::SearchDepth`] when
    /// the budget is exceeded.
    pub fn check_search_depth(&self, depth: u64) -> Result<(), Error> {
        match self.max_search_depth {
            Some(max) if depth > max => Err(Error::ResourceExhausted {
                kind: ResourceKind::SearchDepth,
            }),
            _ => Ok(()),
        }
    }

    /// Check that a certificate of `bytes` bytes is within budget.
    ///
    /// # Errors
    ///
    /// [`Error::ResourceExhausted`] with [`ResourceKind::CertificateSize`]
    /// when the budget is exceeded.
    pub fn check_certificate_size(&self, bytes: u64) -> Result<(), Error> {
        match self.max_certificate_size_bytes {
            Some(max) if bytes > max => Err(Error::ResourceExhausted {
                kind: ResourceKind::CertificateSize,
            }),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_unlimited() {
        let l = ResourceLimits::default();
        assert_eq!(l.max_integer_bits(), None);
        assert_eq!(l.max_memory_bytes(), None);
        assert!(l.check_integer_bits(u64::MAX).is_ok());
        assert!(l.check_memory(u64::MAX).is_ok());
        assert!(l.check_polynomial_degree(u64::MAX).is_ok());
        assert!(l.check_search_depth(u64::MAX).is_ok());
        assert!(l.check_certificate_size(u64::MAX).is_ok());
    }

    #[test]
    fn checks_report_the_exceeded_kind() {
        let l = ResourceLimits::default()
            .with_max_integer_bits(100)
            .with_max_memory_bytes(1_000)
            .with_max_polynomial_degree(10)
            .with_max_search_depth(5)
            .with_max_certificate_size_bytes(500);

        for (res, kind) in [
            (l.check_integer_bits(101), ResourceKind::IntegerBits),
            (l.check_memory(1_001), ResourceKind::Memory),
            (
                l.check_polynomial_degree(11),
                ResourceKind::PolynomialDegree,
            ),
            (l.check_search_depth(6), ResourceKind::SearchDepth),
            (l.check_certificate_size(501), ResourceKind::CertificateSize),
        ] {
            let err = res.expect_err("over-budget request must fail");
            assert_eq!(err, Error::ResourceExhausted { kind });
            // The guarantee: exhaustion is an Err, never a value.
            assert!(err.is_resource_exhausted());
            assert!(!err.is_inconclusive());
        }

        // Boundary: exactly at the limit is allowed.
        assert!(l.check_integer_bits(100).is_ok());
        assert!(l.check_memory(1_000).is_ok());
    }

    #[test]
    fn restrictive_preset_sets_every_budget() {
        let l = ResourceLimits::restrictive();
        assert!(l.max_integer_bits().is_some());
        assert!(l.max_memory_bytes().is_some());
        assert!(l.max_polynomial_degree().is_some());
        assert!(l.max_search_depth().is_some());
        assert!(l.max_certificate_size_bytes().is_some());
        assert!(l.max_execution_time().is_some());
    }
}
