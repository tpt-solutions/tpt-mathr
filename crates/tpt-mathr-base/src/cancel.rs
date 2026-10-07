// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cooperative cancellation (v1.2 review delta #6).
//!
//! Long-running engine operations — searches, big multiplications, rewrite
//! sweeps — accept a [`CancellationToken`] and check it at their internal
//! safety points. Cancellation is *cooperative*: nothing is preempted; a
//! token only flips an atomic flag that running code polls. When a cancelled
//! operation unwinds, it returns [`Error::Cancelled`] — an `Err`, never a
//! result value, so cancellation can never be mistaken for a negative or
//! verified answer.
//!
//! The token is a cheap `Arc`-shared flag: cloning it is one atomic
//! increment, checking it is one atomic load. Coordinator-side cancellation
//! (v1.2 §"cancellation": worker B finds a counterexample, coordinator
//! cancels A, C, D) is expressed by sharing one token with every worker.
//!
//! ```
//! use tpt_mathr_base::CancellationToken;
//!
//! let token = CancellationToken::new();
//! let worker = token.clone();
//!
//! assert!(!token.is_cancelled());
//! token.cancel();
//! // The worker observes cancellation through its clone:
//! assert_eq!(worker.check(), Err(tpt_mathr_base::Error::Cancelled));
//! ```

use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::error::Error;

/// Shared state behind every clone of a [`CancellationToken`].
#[derive(Debug, Default)]
struct CancelState {
    flag: AtomicBool,
}

/// A cheap, cloneable handle for cooperatively cancelling engine operations.
///
/// See the [module documentation](self) for the architectural contract.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    state: Arc<CancelState>,
}

impl CancellationToken {
    /// Create a token in the not-cancelled state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Cancel every operation holding a clone of this token.
    ///
    /// Idempotent and infallible: cancelling twice is a no-op, and calling
    /// it on an already-cancelled token is harmless. Operations observe
    /// cancellation through [`is_cancelled`](Self::is_cancelled) or
    /// [`check`](Self::check), not through this call.
    pub fn cancel(&self) {
        self.state.flag.swap(true, Ordering::AcqRel);
    }

    /// `true` once [`cancel`](Self::cancel) has been called on this token or
    /// any of its clones.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.state.flag.load(Ordering::Acquire)
    }

    /// Cooperative check point: `Ok(())` while running, `Err(Cancelled)` once
    /// cancelled. This is what long operations call at their safety points.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Cancelled`] when the token has been cancelled.
    #[must_use = "the check result must be propagated"]
    pub fn check(&self) -> Result<(), Error> {
        if self.is_cancelled() {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }

    /// Like [`check`](Self::check) but returns the flag instead of an error,
    /// for hot paths where branch shape matters more than diagnostics.
    #[must_use]
    pub fn poll(&self) -> bool {
        !self.is_cancelled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_token_is_not_cancelled() {
        let t = CancellationToken::new();
        assert!(!t.is_cancelled());
        assert_eq!(t.check(), Ok(()));
        assert!(t.poll());
    }

    #[test]
    fn cancel_is_shared_across_clones_and_idempotent() {
        let coordinator = CancellationToken::new();
        let a = coordinator.clone();
        let b = coordinator.clone();

        coordinator.cancel();
        coordinator.cancel(); // idempotent

        assert!(a.is_cancelled());
        assert!(b.is_cancelled());
        assert_eq!(a.check(), Err(Error::Cancelled));
        assert!(!a.poll());
    }

    #[test]
    fn cancelling_a_clone_cancels_the_coordinator() {
        let coordinator = CancellationToken::new();
        let worker = coordinator.clone();
        worker.cancel();
        assert!(coordinator.is_cancelled());
    }

    #[test]
    fn cancel_matches_only_cancelled_error() {
        let t = CancellationToken::new();
        t.cancel();
        let err = t.check().unwrap_err();
        assert_eq!(err, Error::Cancelled);
        assert!(err.is_cancelled());
        assert!(!err.is_inconclusive());
        assert!(!err.is_resource_exhausted());
    }
}
