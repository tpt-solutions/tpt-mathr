// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cooperative progress context threading limits and cancellation through
//! long operations (v1.2 deltas A1/A2/A7).
//!
//! Two contracts from `docs/invariants.md` are enforced here:
//!
//! * **I9** — limits are honoured before the budget is crossed; exceeding
//!   one is `Err(ResourceExhausted)`, never a value produced outside it.
//! * **I10-adjacent** — cancellation is cooperative: checked at safety
//!   points, signalled via `Err(Cancelled)`.
//!
//! Operations accept a `&Progress`. The *pure* value API passes
//! [`Progress::UNLIMITED`] (allocation stays fallible — v1.2 delta A1 —
//! but no budgets apply); entry points for untrusted inputs construct a
//! `Progress` from their own budgets and token and pass it down.

use tpt_mathr_base::{CancellationToken, Error, ResourceKind, ResourceLimits};

/// Shared, cheaply cloneable run context: budgets plus an optional
/// cancellation token. Long operations poll it at safety points.
#[derive(Debug, Clone, Default)]
pub struct Progress {
    limits: ResourceLimits,
    cancel: Option<CancellationToken>,
}

impl Progress {
    /// The no-budgets, no-token context used by the pure value API.
    pub const UNLIMITED: Progress = Progress {
        limits: ResourceLimits::DEFAULT,
        cancel: None,
    };

    /// Build a context from explicit limits.
    #[must_use]
    pub const fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            cancel: None,
        }
    }

    /// Attach a cancellation token.
    #[must_use]
    pub fn with_cancellation(mut self, token: CancellationToken) -> Self {
        self.cancel = Some(token);
        self
    }

    /// The configured limits.
    #[must_use]
    pub const fn limits(&self) -> &ResourceLimits {
        &self.limits
    }

    /// The attached token, if any.
    #[must_use]
    pub const fn cancellation(&self) -> Option<&CancellationToken> {
        self.cancel.as_ref()
    }

    /// Safety point: `Err(Cancelled)` if the attached token has fired.
    ///
    /// # Errors
    ///
    /// [`Error::Cancelled`].
    #[inline]
    pub fn check_cancelled(&self) -> Result<(), Error> {
        match &self.cancel {
            Some(t) => t.check(),
            None => Ok(()),
        }
    }

    /// Safety point for an integer about to occupy `bits` bits (I9: checked
    /// *before* the allocation that would create it).
    ///
    /// # Errors
    ///
    /// [`Error::ResourceExhausted`] with kind `IntegerBits`.
    #[inline]
    pub fn check_integer_bits(&self, bits: u64) -> Result<(), Error> {
        self.limits.check_integer_bits(bits)
    }

    /// Safety point for a buffer about to hold `bytes` bytes (I9).
    ///
    /// # Errors
    ///
    /// [`Error::ResourceExhausted`] with kind `Memory`.
    #[inline]
    pub fn check_memory(&self, bytes: u64) -> Result<(), Error> {
        self.limits.check_memory(bytes)
    }

    /// Convenience: any-limit exhaustion error (used where the precise kind
    /// is decided by the caller's earlier check).
    #[must_use]
    pub const fn exhausted(kind: ResourceKind) -> Error {
        Error::ResourceExhausted { kind }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlimited_has_no_budgets_and_never_cancels() {
        assert!(Progress::UNLIMITED.check_cancelled().is_ok());
        assert!(Progress::UNLIMITED.check_integer_bits(u64::MAX).is_ok());
        assert!(Progress::UNLIMITED.check_memory(u64::MAX).is_ok());
        assert!(Progress::UNLIMITED.cancellation().is_none());
    }

    #[test]
    fn budgets_and_token_are_enforced() {
        let p = Progress::new(ResourceLimits::restrictive())
            .with_cancellation(CancellationToken::new());

        let err = p.check_integer_bits(4096).unwrap_err();
        assert_eq!(
            err,
            Error::ResourceExhausted {
                kind: ResourceKind::IntegerBits
            }
        );

        assert!(p.check_integer_bits(64).is_ok());
        assert!(p.check_cancelled().is_ok());

        p.cancellation().unwrap().cancel();
        assert_eq!(p.check_cancelled(), Err(Error::Cancelled));
    }
}
