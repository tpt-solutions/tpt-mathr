// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! FFI status codes and the total error → status mapping (spec §19, §42).
//!
//! The C ABI (Phase 11) returns a [`TptStatus`] from every function and
//! delivers results through out-parameters (`TptStatus f(..., T** out)`).
//! This module is the *only* place where engine errors
//! ([`crate::error::Error`]) become status codes, so the boundary contract
//! stays auditable:
//!
//! | [`Error`](crate::error::Error)         | [`TptStatus`]                     |
//! |----------------------------------------|-----------------------------------|
//! | `Parse(_)`                             | `InvalidArgument`                 |
//! | `Overflow`, `Underflow`                | `InvalidArgument`                 |
//! | `DivisionByZero`                       | `InvalidArgument`                 |
//! | `InvalidPrecision`, `InvalidRadix`     | `InvalidArgument`                 |
//! | `UnsupportedOperation`                 | `InvalidArgument`                 |
//! | `AllocationFailure`                    | `OutOfMemory`                     |
//! | `ResourceExhausted { .. }`             | `ResourceExhausted`               |
//! | `Cancelled`                            | `Cancelled`                       |
//! | `Inconclusive`                         | `Inconclusive`                    |
//! | `VerificationFailure`                  | `VerificationFailure`             |
//! | `InvalidCertificate`                   | `VerificationFailure`             |
//!
//! `NullArgument` has no `Error` source: it is produced by the FFI layer
//! itself when a required pointer parameter is null. `InternalError` has no
//! `Error` source either: it is the defensive catch-all for invariant
//! violations that must never occur (and is also what an unhandled
//! `#[non_exhaustive]` future variant maps to).
//!
//! # Guarantee
//!
//! [`Error::ResourceExhausted`](crate::error::Error::ResourceExhausted) and
//! [`Error::Inconclusive`](crate::error::Error::Inconclusive) map to their own
//! non-zero codes and can therefore never be presented to a caller as
//! success, `false`, or a verified result — pinning the spec §33 rule at the
//! ABI boundary. The mapping is pinned by tests.

/// Status code returned across the C ABI (spec §19).
///
/// `#[non_exhaustive]` is intentionally **not** used: C consumers switch on
/// the numeric values exhaustively, and adding a variant is a documented
/// ABI revision, not silent growth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum TptStatus {
    /// Operation completed; the out-parameter (if any) is valid.
    Ok = 0,
    /// A required pointer argument was null.
    NullArgument = 1,
    /// An argument was structurally invalid (bad radix, division by zero,
    /// out-of-range conversion, unsupported operation, malformed input).
    InvalidArgument = 2,
    /// The allocator refused a request (engine: `Error::AllocationFailure`).
    OutOfMemory = 3,
    /// A configured resource budget was exhausted. **Not** a negative
    /// result: the caller asked for a definite answer and ran out of budget.
    ResourceExhausted = 4,
    /// The operation observed its cancellation token fire.
    Cancelled = 5,
    /// The operation completed without producing a definite verdict.
    /// **Not** a negative result and **not** a verification.
    Inconclusive = 6,
    /// A verification obligation failed, or a certificate was rejected.
    VerificationFailure = 7,
    /// Defensive catch-all: an invariant that must never be violated was.
    InternalError = 8,
}

impl TptStatus {
    /// The numeric value passed across the ABI.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }

    /// `true` only for [`TptStatus::Ok`]. The single sanctioned success
    /// test at the ABI boundary.
    #[must_use]
    pub const fn is_ok(self) -> bool {
        matches!(self, TptStatus::Ok)
    }

    /// Stable, human-readable name for error-detail APIs.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            TptStatus::Ok => "ok",
            TptStatus::NullArgument => "null_argument",
            TptStatus::InvalidArgument => "invalid_argument",
            TptStatus::OutOfMemory => "out_of_memory",
            TptStatus::ResourceExhausted => "resource_exhausted",
            TptStatus::Cancelled => "cancelled",
            TptStatus::Inconclusive => "inconclusive",
            TptStatus::VerificationFailure => "verification_failure",
            TptStatus::InternalError => "internal_error",
        }
    }

    /// Map an engine error to its ABI status code.
    ///
    /// This function is the audited boundary of the error model; see the
    /// [module documentation](self) for the full table.
    #[allow(
        clippy::match_same_arms,
        reason = "the mapping is a documentation table; identical bodies are the point"
    )]
    #[must_use]
    pub const fn from_error(err: &crate::error::Error) -> Self {
        use crate::error::{Error, ParseErrorKind};
        match err {
            Error::Parse(e) => match e.kind {
                ParseErrorKind::ExceedsLimit => TptStatus::ResourceExhausted,
                _ => TptStatus::InvalidArgument,
            },
            Error::Overflow
            | Error::Underflow
            | Error::DivisionByZero
            | Error::InvalidPrecision
            | Error::UnsupportedOperation => TptStatus::InvalidArgument,
            Error::InvalidRadix { .. } => TptStatus::InvalidArgument,
            Error::AllocationFailure => TptStatus::OutOfMemory,
            Error::ResourceExhausted { .. } => TptStatus::ResourceExhausted,
            Error::Cancelled => TptStatus::Cancelled,
            Error::Inconclusive => TptStatus::Inconclusive,
            Error::VerificationFailure | Error::InvalidCertificate => {
                TptStatus::VerificationFailure
            } // Exhaustive by construction: Error is #[non_exhaustive] for
              // *downstream* users only; inside this crate a new variant is a
              // compile error here, forcing an audited mapping decision.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{Error, ParseError, ResourceKind};

    #[test]
    fn codes_are_stable_and_non_zero_for_errors() {
        let all = [
            TptStatus::Ok,
            TptStatus::NullArgument,
            TptStatus::InvalidArgument,
            TptStatus::OutOfMemory,
            TptStatus::ResourceExhausted,
            TptStatus::Cancelled,
            TptStatus::Inconclusive,
            TptStatus::VerificationFailure,
            TptStatus::InternalError,
        ];
        for (i, s) in all.iter().enumerate() {
            let idx = u32::try_from(i).expect("fewer than u32::MAX status codes");
            assert_eq!(s.code(), idx, "status codes are ABI: order is frozen");
        }
        assert!(TptStatus::Ok.is_ok());
        for s in &all[1..] {
            assert!(!s.is_ok());
            assert_ne!(s.code(), 0);
        }
    }

    #[test]
    fn mapping_table_is_exactly_as_documented() {
        let cases = [
            (
                Error::Parse(ParseError::invalid_character(0)),
                TptStatus::InvalidArgument,
            ),
            (
                Error::Parse(ParseError::empty()),
                TptStatus::InvalidArgument,
            ),
            (
                Error::Parse(ParseError::exceeds_limit()),
                TptStatus::ResourceExhausted,
            ),
            (Error::Overflow, TptStatus::InvalidArgument),
            (Error::Underflow, TptStatus::InvalidArgument),
            (Error::DivisionByZero, TptStatus::InvalidArgument),
            (Error::InvalidPrecision, TptStatus::InvalidArgument),
            (
                Error::InvalidRadix { radix: 40 },
                TptStatus::InvalidArgument,
            ),
            (Error::AllocationFailure, TptStatus::OutOfMemory),
            (
                Error::ResourceExhausted {
                    kind: ResourceKind::Memory,
                },
                TptStatus::ResourceExhausted,
            ),
            (Error::VerificationFailure, TptStatus::VerificationFailure),
            (Error::UnsupportedOperation, TptStatus::InvalidArgument),
            (Error::InvalidCertificate, TptStatus::VerificationFailure),
            (Error::Inconclusive, TptStatus::Inconclusive),
            (Error::Cancelled, TptStatus::Cancelled),
        ];
        for (err, want) in cases {
            assert_eq!(TptStatus::from_error(&err), want, "mapping of {err}");
        }
    }

    // The ABI boundary half of the spec §33 guarantee.
    #[test]
    fn exhaustion_and_inconclusive_never_become_ok_or_verdicts() {
        let inconclusive_flavoured = [
            Error::ResourceExhausted {
                kind: ResourceKind::ExecutionTime,
            },
            Error::ResourceExhausted {
                kind: ResourceKind::Memory,
            },
            Error::Inconclusive,
        ];
        for err in inconclusive_flavoured {
            let status = TptStatus::from_error(&err);
            assert!(!status.is_ok(), "{err} must not map to OK");
            assert_ne!(
                status,
                TptStatus::VerificationFailure,
                "{err} is not a verdict"
            );
            assert_eq!(err.definite_refutation(), None);
        }
    }
}
