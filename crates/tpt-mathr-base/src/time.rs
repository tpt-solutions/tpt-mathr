// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cooperative wall-clock deadlines (requires the `std` feature).
//!
//! `ResourceLimits::max_execution_time` is stored even in `no_std`, but
//! *measuring* elapsed time needs a clock. This module provides the hosted
//! check: a [`Deadline`] pairs the configured budget with
//! [`std::time::Instant`] and answers one question — is the budget spent?
//!
//! Distributed search workers use the same primitive with a budget injected
//! per work unit; there is deliberately no global timer (no hidden global
//! state, spec §2.5).

use std::time::Instant;

use crate::error::{Error, ResourceKind};
use crate::limits::ResourceLimits;

/// A running wall-clock budget. Cheap to create, `Copy`, and checked
/// cooperatively at operation safety points.
#[derive(Debug, Clone, Copy)]
pub struct Deadline {
    start: Instant,
    budget: Option<core::time::Duration>,
}

impl Deadline {
    /// Start a deadline from `limits`' execution-time budget. Unset budget →
    /// a deadline that never expires.
    #[must_use]
    pub fn from_limits(limits: &ResourceLimits) -> Self {
        Self {
            start: Instant::now(),
            budget: limits.max_execution_time(),
        }
    }

    /// Start a deadline with an explicit budget.
    #[must_use]
    pub fn after(budget: core::time::Duration) -> Self {
        Self {
            start: Instant::now(),
            budget: Some(budget),
        }
    }

    /// Elapsed time since the deadline started.
    #[must_use]
    pub fn elapsed(&self) -> core::time::Duration {
        self.start.elapsed()
    }

    /// `true` when the budget is spent (or was never configured).
    #[must_use]
    pub fn expired(&self) -> bool {
        match self.budget {
            Some(budget) => self.elapsed() >= budget,
            None => false,
        }
    }

    /// Cooperative check point: `Ok(())` while time remains,
    /// `Err(ResourceExhausted { kind: ExecutionTime })` once spent.
    ///
    /// # Errors
    ///
    /// [`Error::ResourceExhausted`] with [`ResourceKind::ExecutionTime`].
    pub fn check(&self) -> Result<(), Error> {
        if self.expired() {
            Err(Error::ResourceExhausted {
                kind: ResourceKind::ExecutionTime,
            })
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::time::Duration;

    #[test]
    fn unset_budget_never_expires() {
        let d = Deadline::from_limits(&ResourceLimits::default());
        assert!(!d.expired());
        assert_eq!(d.check(), Ok(()));
    }

    #[test]
    fn explicit_budget_expires() {
        let d = Deadline::after(Duration::from_millis(0));
        assert!(d.expired());
        let err = d.check().unwrap_err();
        assert_eq!(
            err,
            Error::ResourceExhausted {
                kind: ResourceKind::ExecutionTime
            }
        );
        assert!(err.is_resource_exhausted());
        assert!(!err.is_inconclusive());
    }

    #[test]
    fn from_limits_uses_execution_time_budget() {
        let limits = ResourceLimits::default().with_max_execution_time(Duration::from_secs(1));
        let d = Deadline::from_limits(&limits);
        assert!(!d.expired());
        assert_eq!(d.budget, Some(Duration::from_secs(1)));
    }

    #[test]
    fn zero_budget_expires_immediately() {
        assert!(Deadline::after(Duration::ZERO).expired());
    }
}
