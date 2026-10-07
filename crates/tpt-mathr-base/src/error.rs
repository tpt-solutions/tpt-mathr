// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `tpt-mathr` error model (spec §33).
//!
//! Every fallible operation in the engine returns `Result<T, Error>`. The
//! enum is deliberately flat and `#[non_exhaustive]`: new variants may be
//! added as the engine grows, downstream code must match on the ones it
//! understands and route the rest through a catch-all.
//!
//! # Result-semantics guarantee
//!
//! Two variants have a special, architecture-wide contract (spec §33, §34):
//!
//! * [`Error::ResourceExhausted`] — a budget (memory, time, depth, size) was
//!   hit before a definite answer was reached.
//! * [`Error::Inconclusive`] — the method ran to completion but the evidence
//!   it gathered does not settle the question.
//!
//! Neither may ever be *interpreted* as `false`, "no result", or "verified".
//! This crate enforces that contract at three levels:
//!
//! 1. **Types.** Both variants exist only as payloads of `Err`. There is no
//!    `From<Error>` conversion into any boolean/verdict type, and a search or
//!    verification API that wants to report a verdict must use its own
//!    taxonomy (see `tpt-mathr-search`/`tpt-mathr-verify`), into which
//!    `Error` never auto-converts.
//! 2. **Mapping.** [`crate::status::TptStatus::from_error`] maps both to
//!    dedicated non-zero status codes, never to `TPT_OK`.
//! 3. **Introspection.** [`Error::is_inconclusive`] and
//!    [`Error::is_resource_exhausted`] let callers detect these cases
//!    explicitly; [`Error::definite_refutation`] returns `Some(false)` only
//!    for errors that genuinely are definite negative evidence.
//!
//! Simulated allocation failure is exercised in the crate tests: allocation
//! helpers convert `TryReserveError` into [`Error::AllocationFailure`] /
//! [`Error::ResourceExhausted`] and never panic.

use core::fmt;

/// The engine-wide error type (spec §33).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// A string could not be parsed as a number. See [`ParseError`].
    Parse(ParseError),
    /// A value did not fit the destination representation (e.g. a conversion
    /// into a fixed-width primitive whose range was exceeded).
    Overflow,
    /// A value underflowed the destination representation (e.g. an exponent
    /// limit in a floating-point type).
    Underflow,
    /// Division (or modulo) by zero.
    DivisionByZero,
    /// An operation was requested at a precision the type cannot represent.
    InvalidPrecision,
    /// A radix outside the supported range (2–36) was requested.
    InvalidRadix {
        /// The offending radix.
        radix: u8,
    },
    /// A fallible allocation failed. Produced only by the helpers in
    /// [`crate::alloc_helpers`]; ordinary `Vec`/`Box` growth would abort the
    /// process instead, so the engine's large-data paths never use it.
    AllocationFailure,
    /// A configured resource budget was exhausted before a definite answer
    /// was reached. See the [result-semantics guarantee](#result-semantics-guarantee):
    /// this is **not** a negative result.
    ResourceExhausted {
        /// Which configured budget was hit.
        kind: ResourceKind,
    },
    /// A verification obligation could not be discharged: the evidence does
    /// not establish the claim.
    VerificationFailure,
    /// The requested operation is not supported by this build/configuration.
    UnsupportedOperation,
    /// A certificate failed structural validation before its semantics could
    /// be checked.
    InvalidCertificate,
    /// The operation completed but its evidence does not settle the question.
    /// See the [result-semantics guarantee](#result-semantics-guarantee):
    /// this is **not** a negative result and **not** a verification.
    Inconclusive,
    /// The operation observed its [`crate::CancellationToken`] fire.
    Cancelled,
}

/// Which configured budget a [`Error::ResourceExhausted`] hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResourceKind {
    /// The memory budget (`ResourceLimits::max_memory_bytes`) was exceeded.
    Memory,
    /// The integer-bit budget (`ResourceLimits::max_integer_bits`) was exceeded.
    IntegerBits,
    /// The polynomial-degree budget was exceeded.
    PolynomialDegree,
    /// The search-depth budget was exceeded.
    SearchDepth,
    /// The certificate-size budget was exceeded.
    CertificateSize,
    /// The execution-time budget was exceeded.
    ExecutionTime,
}

/// Why a parse failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseErrorKind {
    /// The input string was empty or contained no digits.
    Empty,
    /// A character outside the requested radix was encountered.
    InvalidCharacter,
    /// The parsed value exceeds the configured input-size budget.
    ExceedsLimit,
}

/// Detail for [`Error::Parse`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseError {
    /// Why the parse failed.
    pub kind: ParseErrorKind,
    /// Byte offset of the offending input, when known.
    pub position: Option<usize>,
}

impl ParseError {
    /// Convenience constructor for an invalid character at `position`.
    #[must_use]
    pub const fn invalid_character(position: usize) -> Self {
        Self {
            kind: ParseErrorKind::InvalidCharacter,
            position: Some(position),
        }
    }

    /// Convenience constructor for an empty input.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            kind: ParseErrorKind::Empty,
            position: None,
        }
    }

    /// Convenience constructor for an input that exceeded a configured limit.
    #[must_use]
    pub const fn exceeds_limit() -> Self {
        Self {
            kind: ParseErrorKind::ExceedsLimit,
            position: None,
        }
    }
}

impl Error {
    /// `true` for [`Error::ResourceExhausted`].
    ///
    /// Callers checking for this case are asserting "I need a definite
    /// answer and ran out of budget", never "the answer is no".
    #[must_use]
    pub const fn is_resource_exhausted(&self) -> bool {
        matches!(self, Error::ResourceExhausted { .. })
    }

    /// `true` for [`Error::Inconclusive`].
    ///
    /// Callers checking for this case are asserting "the evidence does not
    /// settle the question", never "the answer is no" and never "verified".
    #[must_use]
    pub const fn is_inconclusive(&self) -> bool {
        matches!(self, Error::Inconclusive)
    }

    /// `true` for [`Error::Cancelled`].
    #[must_use]
    pub const fn is_cancelled(&self) -> bool {
        matches!(self, Error::Cancelled)
    }

    /// Definite negative evidence, if this error constitutes any.
    ///
    /// Returns `Some(false)` only when this error *refutes* a claim; returns
    /// `None` for every inconclusive flavour. This is the single sanctioned
    /// way to convert an error into a boolean verdict, and it makes the
    /// "resource exhaustion is not false" rule a type-checked decision
    /// instead of a convention.
    ///
    /// Currently no variant carries definite negative evidence: a parse
    /// error, an overflow, or a cancelled computation refute nothing. The
    /// method exists so that later phases (verification, search) extend it in
    /// one reviewed place rather than sprinkling `match` arms.
    #[must_use]
    pub const fn definite_refutation(&self) -> Option<bool> {
        let _ = self;
        None
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse(e) => match e.kind {
                ParseErrorKind::Empty => write!(f, "parse error: empty input"),
                ParseErrorKind::InvalidCharacter => match e.position {
                    Some(pos) => write!(f, "parse error: invalid character at byte {pos}"),
                    None => write!(f, "parse error: invalid character"),
                },
                // Exhaustive: a new ParseErrorKind is a compile error here.
                ParseErrorKind::ExceedsLimit => {
                    write!(f, "parse error: input exceeds configured limit")
                }
            },
            Error::Overflow => write!(f, "overflow"),
            Error::Underflow => write!(f, "underflow"),
            Error::DivisionByZero => write!(f, "division by zero"),
            Error::InvalidPrecision => write!(f, "invalid precision"),
            Error::InvalidRadix { radix } => write!(f, "invalid radix {radix} (supported: 2-36)"),
            Error::AllocationFailure => write!(f, "allocation failed"),
            Error::ResourceExhausted { kind } => write!(f, "resource exhausted: {}", kind.name()),
            Error::VerificationFailure => write!(f, "verification failed"),
            Error::UnsupportedOperation => write!(f, "unsupported operation"),
            Error::InvalidCertificate => write!(f, "invalid certificate"),
            Error::Inconclusive => write!(f, "inconclusive: evidence does not settle the question"),
            Error::Cancelled => write!(f, "operation cancelled"),
        }
    }
}

impl ResourceKind {
    /// Stable, human-readable name for diagnostics and logs.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            ResourceKind::Memory => "memory",
            ResourceKind::IntegerBits => "integer bits",
            ResourceKind::PolynomialDegree => "polynomial degree",
            ResourceKind::SearchDepth => "search depth",
            ResourceKind::CertificateSize => "certificate size",
            ResourceKind::ExecutionTime => "execution time",
        }
    }
}

impl core::error::Error for Error {}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Error::Parse(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn inconclusive_and_exhausted_are_detectable() {
        assert!(Error::Inconclusive.is_inconclusive());
        assert!(!Error::Inconclusive.is_resource_exhausted());
        assert!(
            Error::ResourceExhausted {
                kind: ResourceKind::Memory
            }
            .is_resource_exhausted()
        );
        assert!(
            !Error::ResourceExhausted {
                kind: ResourceKind::Memory
            }
            .is_inconclusive()
        );
        assert!(Error::Cancelled.is_cancelled());
    }

    // The core guarantee of spec §33, pinned as a test: the two
    // inconclusive-flavoured errors must never carry a boolean verdict.
    #[test]
    fn resource_exhausted_and_inconclusive_never_map_to_false_or_verified() {
        let exhausteds = [
            ResourceKind::Memory,
            ResourceKind::IntegerBits,
            ResourceKind::PolynomialDegree,
            ResourceKind::SearchDepth,
            ResourceKind::CertificateSize,
            ResourceKind::ExecutionTime,
        ]
        .map(|kind| Error::ResourceExhausted { kind });

        for err in exhausteds
            .into_iter()
            .chain(core::iter::once(Error::Inconclusive))
        {
            assert_eq!(
                err.definite_refutation(),
                None,
                "{err} must never be interpretable as a definite result"
            );
        }
    }

    #[test]
    fn display_is_stable_and_informative() {
        assert_eq!(Error::DivisionByZero.to_string(), "division by zero");
        assert_eq!(
            Error::InvalidRadix { radix: 1 }.to_string(),
            "invalid radix 1 (supported: 2-36)"
        );
        assert_eq!(
            Error::Parse(ParseError::invalid_character(7)).to_string(),
            "parse error: invalid character at byte 7"
        );
        assert!(Error::Inconclusive.to_string().contains("inconclusive"));
    }
}
