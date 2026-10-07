// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared infrastructure for the `tpt-mathr` engine.
//!
//! `tpt-mathr-base` is the dependency-free foundation every other
//! `tpt-mathr` crate builds on. It hosts the cross-cutting concerns that the
//! specification (§33, and the v1.2 review deltas) requires to be uniform
//! across the whole engine:
//!
//! * [`error::Error`] — the engine-wide error model. [`Error::ResourceExhausted`]
//!   and [`Error::Inconclusive`] can **never** be coerced into a negative or
//!   verified result; see the [result-semantics guarantee](error#result-semantics-guarantee).
//! * [`limits::ResourceLimits`] — explicit budgets (memory, integer bits,
//!   polynomial degree, search depth, certificate size, execution time) so a
//!   worker discovers a budget overrun *before* consuming the machine.
//! * [`cancel::CancellationToken`] — cheap, cloneable, cooperative
//!   cancellation for long operations and distributed search.
//! * [`alloc_helpers`] — fallible allocation helpers built on
//!   `try_reserve`; large-data paths never rely on infallible growth.
//! * [`status::TptStatus`] — the FFI status-code enum and the total
//!   error → status mapping used at the ABI boundary.
//! * [`time`] (behind the `std` feature) — cooperative wall-clock deadlines.
//!
//! The crate is `no_std + alloc`; the `std` feature only adds the hosted
//! time source. It has **no external dependencies** (spec §44).

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs, missing_debug_implementations)]
#![warn(clippy::pedantic)]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod alloc_helpers;
pub mod cancel;
pub mod error;
pub mod limits;
pub mod status;
#[cfg(feature = "std")]
pub mod time;

/// Deterministic RNG helpers for property/metamorphic tests across the
/// workspace. Test scaffolding only: never part of the engine API.
#[cfg(feature = "testing")]
pub mod testing;

pub use cancel::CancellationToken;
pub use error::{Error, ParseError, ParseErrorKind, ResourceKind};
pub use limits::ResourceLimits;
pub use status::TptStatus;
