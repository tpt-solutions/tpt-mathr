// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Limb representation (private; v1.2 delta A3).
//!
//! The limb width is a **private** platform-conditional detail: `u64` on
//! 64-bit targets, `u32` on 32-bit targets and wasm32. The wider
//! intermediate type used for carries, products, and division steps is
//! `DoubleLimb` (`u128` / `u64` respectively), with a signed twin for
//! Knuth-D borrows. Nothing in this module is reachable from the public
//! API — callers see `BigUint`/`BigInt`, never `BigInt<Limb>`.

/// The limb element type: `u64` where the platform multiplies it natively,
/// `u32` where a 64-bit product would be emulated (32-bit, wasm32).
#[cfg(target_pointer_width = "64")]
pub(crate) type Limb = u64;

#[cfg(target_pointer_width = "32")]
pub(crate) type Limb = u32;

/// Wider intermediate for limb products, carries, and division estimates.
#[cfg(target_pointer_width = "64")]
pub(crate) type DoubleLimb = u128;

#[cfg(target_pointer_width = "32")]
pub(crate) type DoubleLimb = u64;

/// Signed twin of [`DoubleLimb`] for multiply-subtract borrows (Knuth D).
#[cfg(target_pointer_width = "64")]
pub(crate) type SignedDouble = i128;

#[cfg(target_pointer_width = "32")]
pub(crate) type SignedDouble = i64;

/// Bits per limb.
#[cfg(target_pointer_width = "64")]
pub(crate) const LIMB_BITS: usize = 64;

#[cfg(target_pointer_width = "32")]
pub(crate) const LIMB_BITS: usize = 32;

/// `2^LIMB_BITS - 1` as a `Limb`.
pub(crate) const LIMB_MAX: Limb = Limb::MAX;

/// The zero limb.
pub(crate) const LIMB_ZERO: Limb = 0;

/// The one limb.
pub(crate) const LIMB_ONE: Limb = 1;

/// Widen a u64 into a Limb (truncating on 32-bit targets). Used by the
/// feature-gated `test_support::from_u64_limbs`, hence invisible to dead-code
/// analysis in default builds.
#[inline]
#[allow(dead_code, reason = "used by cfg(feature = \"testing\") test_support")]
pub(crate) const fn limb_from_u64_lossy(v: u64) -> Limb {
    v as Limb
}

/// Join `(high, low)` limbs into a `DoubleLimb`.
#[inline]
#[allow(dead_code, reason = "used by tests now and Phase 3 kernel paths later")]
pub(crate) const fn dl_join(hi: Limb, lo: Limb) -> DoubleLimb {
    ((hi as DoubleLimb) << LIMB_BITS) | (lo as DoubleLimb)
}

/// Number of limbs needed to represent `bits` bits.
#[inline]
#[allow(dead_code, reason = "used by tests now and Phase 3 kernel paths later")]
pub(crate) const fn limbs_for_bits(bits: u64) -> usize {
    (bits as usize).div_ceil(LIMB_BITS)
}

/// Widen a limb to `u64` (the limb-free representation used by conversions
/// and rendering).
#[inline]
#[allow(
    dead_code,
    reason = "wired into conversions incrementally; tests exercise it"
)]
pub(crate) const fn limb_to_u64(v: Limb) -> u64 {
    #[cfg(target_pointer_width = "64")]
    {
        v
    }
    #[cfg(target_pointer_width = "32")]
    {
        // Widening cast; `From` is not const-callable here yet.
        v as u64
    }
}

/// Split a `DoubleLimb` into `(high, low)` limbs.
#[inline]
pub(crate) const fn dl_split(v: DoubleLimb) -> (Limb, Limb) {
    #[cfg(target_pointer_width = "64")]
    {
        ((v >> 64) as Limb, v as Limb)
    }
    #[cfg(target_pointer_width = "32")]
    {
        ((v >> 32) as Limb, v as Limb)
    }
}

/// Full product of two limbs as a `DoubleLimb`.
#[inline]
pub(crate) const fn widening_mul(a: Limb, b: Limb) -> DoubleLimb {
    (a as DoubleLimb) * (b as DoubleLimb)
}

/// Number of leading zero bits of a limb (`LIMB_BITS` for zero).
#[inline]
pub(crate) const fn leading_zeros(v: Limb) -> u32 {
    v.leading_zeros()
}

/// Number of trailing zero bits of a limb (`LIMB_BITS` for zero).
#[inline]
pub(crate) const fn trailing_zeros(v: Limb) -> u32 {
    v.trailing_zeros()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limb_width_matches_target() {
        if cfg!(target_pointer_width = "64") {
            assert_eq!(LIMB_BITS, 64);
            assert_eq!(core::mem::size_of::<Limb>(), 8);
            assert_eq!(core::mem::size_of::<DoubleLimb>(), 16);
        } else {
            assert_eq!(LIMB_BITS, 32);
            assert_eq!(core::mem::size_of::<Limb>(), 4);
            assert_eq!(core::mem::size_of::<DoubleLimb>(), 8);
        }
    }

    #[test]
    fn split_join_round_trip() {
        let v = widening_mul(LIMB_MAX, LIMB_MAX);
        let (hi, lo) = dl_split(v);
        assert_eq!(dl_join(hi, lo), v);
        // (B-1)^2 = (B-2)·B + 1: the high limb is B-2 on either width.
        assert_eq!(hi, LIMB_MAX - 1);
        let (h2, l2) = dl_split(dl_join(LIMB_MAX, 0));
        assert_eq!((h2, l2), (LIMB_MAX, 0));
    }

    #[test]
    fn limb_to_u64_is_exact_on_both_widths() {
        assert_eq!(limb_to_u64(0), 0);
        #[allow(clippy::unnecessary_cast, reason = "the cast is the point on 32-bit")]
        let expected = LIMB_MAX as u64;
        assert_eq!(limb_to_u64(LIMB_MAX), expected);
    }

    #[test]
    fn bits_to_limbs() {
        assert_eq!(limbs_for_bits(1), 1);
        assert_eq!(limbs_for_bits(LIMB_BITS as u64), 1);
        assert_eq!(limbs_for_bits(LIMB_BITS as u64 + 1), 2);
    }

    #[test]
    fn zero_limb_has_full_width_counts() {
        assert_eq!(leading_zeros(0), LIMB_BITS as u32);
        assert_eq!(trailing_zeros(0), LIMB_BITS as u32);
        assert_eq!(leading_zeros(1), LIMB_BITS as u32 - 1);
    }
}
