// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fallible allocation helpers (v1.2 review delta #1: "OOM handling").
//!
//! Rust's ordinary allocation paths (`Vec::push` growth, `Box::new`, …)
//! **abort the process** on allocation failure; they cannot return an error.
//! The v1.2 review corrected the design accordingly: every large-data growth
//! path in the engine must go through [`Vec::try_reserve`]/`try_reserve_exact`
//! so exhaustion surfaces as [`Error::AllocationFailure`] instead of a
//! process abort. This is what keeps "one worker's OOM must not take down the
//! node" achievable for `tpt-mathr-search`.
//!
//! The helpers here are the single sanctioned conversion point from
//! [`alloc::collections::TryReserveError`] to the engine error model:
//!
//! | `TryReserveError`      | [`Error`]                                |
//! |------------------------|------------------------------------------|
//! | `CapacityOverflow`     | `ResourceExhausted { kind: Memory }`     |
//! | `AllocError`           | `AllocationFailure`                      |
//!
//! `CapacityOverflow` means the *requested size* cannot exist on this
//! platform (`> isize::MAX` bytes) — a resource limit of physics, not a
//! transient shortage, so it maps to `ResourceExhausted`. `AllocError` is the
//! allocator actually refusing — transient `AllocationFailure`.
//!
//! Later phases (symbolic arenas, scratch-buffer pools) replace the
//! *mechanism* underneath these helpers, not their signatures; arithmetic
//! code never calls the allocator directly (see `docs/architecture.md`).

use alloc::vec::Vec;
use core::mem;

use crate::error::{Error, ResourceKind};

/// Mirror the allocator's capacity-overflow rule without unstable APIs:
/// a request is impossible iff its byte size cannot be addressed on this
/// platform (`> isize::MAX`). We pre-check so the engine can distinguish
/// "request can never exist" ([`Error::ResourceExhausted`]) from "allocator
/// refused a possible request" ([`Error::AllocationFailure`]) — the
/// distinction `TryReserveErrorKind` would otherwise provide.
const fn request_impossible(bytes_per_element: usize, cap: usize) -> bool {
    match bytes_per_element.checked_mul(cap) {
        None => true,
        Some(total) => total > isize::MAX as usize,
    }
}

/// Convert the allocator's failure modes into the engine error model.
/// Impossibility is pre-checked by the callers, so any error reaching this
/// point is a genuine allocator refusal.
const fn map_try_reserve(_: alloc::collections::TryReserveError) -> Error {
    Error::AllocationFailure
}

/// Build a `Vec<T>` with capacity for exactly `cap` elements, fallibly.
///
/// No elements are allocated-then-dropped: memory is reserved without
/// constructing elements.
///
/// # Errors
///
/// [`Error::AllocationFailure`] if the allocator refuses;
/// [`Error::ResourceExhausted`] if the request exceeds platform capacity.
pub fn try_with_capacity<T>(cap: usize) -> Result<Vec<T>, Error> {
    if request_impossible(mem::size_of::<T>(), cap) {
        return Err(Error::ResourceExhausted {
            kind: ResourceKind::Memory,
        });
    }
    let mut v = Vec::new();
    v.try_reserve_exact(cap).map_err(map_try_reserve)?;
    Ok(v)
}

/// Grow `v` by `additional` slots, fallibly (amortised growth).
///
/// This is the engine's canonical `reserve_limbs`-style helper: arithmetic
/// code calls it *before* writing a computed range, so a failure aborts the
/// operation with `Err` and leaves every [`BigInt`-like value] in a valid
/// state (a failed `try_reserve` leaves the vector unchanged).
///
/// # Errors
///
/// [`Error::AllocationFailure`] / [`Error::ResourceExhausted`] as mapped
/// above; `v` is unchanged in both cases.
pub fn reserve<T>(v: &mut Vec<T>, additional: usize) -> Result<(), Error> {
    if request_impossible(mem::size_of::<T>(), additional) {
        return Err(Error::ResourceExhausted {
            kind: ResourceKind::Memory,
        });
    }
    v.try_reserve(additional).map_err(map_try_reserve)
}

/// Like [`reserve`] but allocates exactly `additional` slots (no geometric
/// over-allocation). Prefer [`reserve`] in hot paths; use this for
/// fixed-shape results where the final size is known.
///
/// # Errors
///
/// As [`reserve`].
pub fn reserve_exact<T>(v: &mut Vec<T>, additional: usize) -> Result<(), Error> {
    if request_impossible(mem::size_of::<T>(), additional) {
        return Err(Error::ResourceExhausted {
            kind: ResourceKind::Memory,
        });
    }
    v.try_reserve_exact(additional).map_err(map_try_reserve)
}

/// Reserve additional capacity for `v` and then extend it to `new_len` with
/// `value`, fallibly.
///
/// On failure `v` is unchanged. On success `v.len() == new_len`.
///
/// # Errors
///
/// As [`reserve`].
pub fn resize_with_reserve<T: Clone>(
    v: &mut Vec<T>,
    new_len: usize,
    value: T,
) -> Result<(), Error> {
    let additional = new_len.saturating_sub(v.len());
    reserve(v, additional)?;
    v.resize(new_len, value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{vec, vec::Vec};

    // --- Simulated allocation failure -------------------------------------

    // A request that cannot exist on any platform: its byte size exceeds
    // isize::MAX, so the allocator could never address it. The pre-check
    // makes this deterministic everywhere (no reliance on the unstable
    // TryReserveErrorKind distinction).
    #[test]
    fn impossible_capacity_maps_to_resource_exhausted() {
        let err = reserve::<u8>(&mut Vec::new(), usize::MAX).unwrap_err();
        assert_eq!(
            err,
            Error::ResourceExhausted {
                kind: ResourceKind::Memory
            }
        );

        // 2^63-1 elements of u64: (2^63-1) * 8 > isize::MAX on 64-bit.
        let err = try_with_capacity::<u64>(isize::MAX as usize).unwrap_err();
        assert_eq!(
            err,
            Error::ResourceExhausted {
                kind: ResourceKind::Memory
            }
        );

        // On 32-bit the same rule fires through the byte-size multiplication
        // (usize::MAX/2 elements * 4 bytes overflows usize), so both limb
        // configurations are covered by construction.
    }

    // Genuine allocator refusals (TryReserveError::AllocError) map to
    // Error::AllocationFailure through the same call site; they cannot be
    // triggered deterministically on every platform (Linux overcommit may
    // grant absurd-but-addressable reservations), so that branch is pinned
    // by inspection here and exercised by the fuzz/soak harnesses.

    // --- Success and no-op-on-failure guarantees ---------------------------

    #[test]
    fn reserve_success_and_unchanged_on_failure() {
        let mut v = vec![1_u32, 2, 3];
        reserve(&mut v, 10).expect("small reserve succeeds");
        assert_eq!(v.len(), 3);

        // A failed reserve must leave the vector untouched.
        let before = v.clone();
        let _ = reserve(&mut v, usize::MAX);
        assert_eq!(v, before, "failed reserve must not mutate");
        assert_eq!(v.len(), 3);
    }

    #[test]
    fn resize_with_reserve_extends_or_leaves_unchanged() {
        let mut v = vec![7_u8; 3];
        resize_with_reserve(&mut v, 5, 9).expect("small resize succeeds");
        assert_eq!(v, vec![7, 7, 7, 9, 9]);

        let before = v.clone();
        let _ = resize_with_reserve(&mut v, usize::MAX, 0);
        assert_eq!(v, before, "failed resize must not mutate");
    }

    #[test]
    fn shrink_is_a_no_op_for_resize_helper() {
        let mut v = vec![1_u8, 2, 3, 4];
        resize_with_reserve(&mut v, 2, 0).expect("shrinking never allocates");
        assert_eq!(v, vec![1, 2]);
    }
}
