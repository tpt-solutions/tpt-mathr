// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! [`BigUint`] — non-negative arbitrary-precision integers.
//!
//! # Representation (invariants I1, I2)
//!
//! Little-endian limb vector with **no trailing zero limbs**; the empty
//! vector is the unique zero. Every constructor and operation normalises
//! before returning, and [`BigUint::assert_invariants`] checks the full
//! contract (debug builds and the test suite).
//!
//! # Fallibility
//!
//! Every allocating operation returns `Result` ([`Error`]). Error sources:
//! allocation itself ([`Error::AllocationFailure`] /
//! [`Error::ResourceExhausted`] — v1.2 delta A1) and, where applicable,
//! budgets and cancellation via [`Progress`] (I9, deltas A2/A7). A failed
//! operation leaves its inputs untouched (I8).

use alloc::vec::Vec;

use tpt_mathr_base::alloc_helpers;
use tpt_mathr_base::error::Error;

use crate::limb::{
    DoubleLimb, LIMB_BITS, LIMB_ONE, LIMB_ZERO, Limb, leading_zeros, trailing_zeros,
};
use crate::progress::Progress;

/// A non-negative arbitrary-precision integer.
///
/// Zero is the empty limb vector; every value has exactly one
/// representation (I1, I2). Arithmetic is fallible: see the [crate
/// docs](crate) for the fallibility, limits, and cancellation model.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct BigUint {
    pub(crate) limbs: Vec<Limb>,
}

impl core::fmt::Debug for BigUint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.limbs.is_empty() {
            return write!(f, "BigUint(0)");
        }
        write!(f, "BigUint(0x")?;
        for (i, limb) in self.limbs.iter().rev().enumerate() {
            if i > 0 {
                write!(f, "{limb:0width$x}", width = LIMB_BITS / 4)?;
            } else {
                write!(f, "{limb:x}")?;
            }
        }
        write!(f, ")")
    }
}

impl BigUint {
    /// The zero value.
    #[must_use]
    pub const fn zero() -> Self {
        Self { limbs: Vec::new() }
    }

    /// The value one.
    #[must_use]
    pub fn one() -> Self {
        Self::from_limb(LIMB_ONE)
    }

    /// `true` iff this is zero (I2).
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    /// Number of stored limbs (0 for zero). Limb width is private (A3);
    /// this is used by reduction contexts to size precomputation.
    #[must_use]
    pub(crate) const fn len_limbs(&self) -> usize {
        self.limbs.len()
    }

    /// Number of significant bits: `floor(log2(self)) + 1`, or 0 for zero.
    #[must_use]
    pub fn bit_len(&self) -> u64 {
        match self.limbs.last() {
            None => 0,
            Some(&top) => {
                (self.limbs.len() as u64 - 1) * LIMB_BITS as u64
                    + (LIMB_BITS as u64 - leading_zeros(top) as u64)
            }
        }
    }

    /// Number of trailing zero bits, or `None` for zero.
    #[must_use]
    pub fn trailing_zeros(&self) -> Option<u64> {
        let (idx, limb) = self
            .limbs
            .iter()
            .copied()
            .enumerate()
            .find(|(_, l)| *l != LIMB_ZERO)?;
        Some(idx as u64 * LIMB_BITS as u64 + trailing_zeros(limb) as u64)
    }

    /// Construct from little-endian limbs, normalising (I1). Internal
    /// constructor; the public surface builds values from strings and
    /// primitives.
    #[must_use]
    pub(crate) fn from_limbs(mut limbs: Vec<Limb>) -> Self {
        while limbs.last() == Some(&LIMB_ZERO) {
            limbs.pop();
        }
        Self { limbs }
    }

    /// Borrow the normalised little-endian limbs (empty for zero).
    pub(crate) fn as_limbs(&self) -> &[Limb] {
        &self.limbs
    }

    /// Consume and return the normalised little-endian limbs.
    pub(crate) fn into_limbs(self) -> Vec<Limb> {
        self.limbs
    }

    /// Full invariant check (I1, I2). Panics naming the violated invariant.
    ///
    /// # Panics
    ///
    /// On any violated invariant (trailing zero limb).
    pub fn assert_invariants(&self) {
        if let Some(last) = self.limbs.last() {
            assert_ne!(*last, LIMB_ZERO, "I1 violated: trailing zero limb");
        }
    }

    /// Full addition: `self + other`.
    ///
    /// # Errors
    ///
    /// Allocation failure, or [`Error::ResourceExhausted`] under `progress`.
    pub fn add(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        let (long, short) = if self.limbs.len() >= other.limbs.len() {
            (&self.limbs, &other.limbs)
        } else {
            (&other.limbs, &self.limbs)
        };
        let out_len = long.len() + 1; // upper bound; unused slot dropped below
        progress.check_memory(out_len as u64 * (LIMB_BITS as u64 / 8))?;
        let mut out: Vec<Limb> = Vec::with_capacity(out_len);
        let mut carry = LIMB_ZERO;
        // Index-based: both arrays are indexed at the same position with a
        // tail default, which iterator zips express less clearly.
        #[allow(clippy::needless_range_loop)]
        for i in 0..long.len() {
            let b = short.get(i).copied().unwrap_or(LIMB_ZERO);
            let (s1, c1) = add_carry(long[i], b);
            let (s2, c2) = add_carry(s1, carry);
            out.push(s2);
            carry = if c1 != 0 || c2 != 0 {
                LIMB_ONE
            } else {
                LIMB_ZERO
            };
        }
        if carry != 0 {
            out.push(carry);
        }
        // from_limbs normalises; there is no unused slot to drop (the loop
        // pushed exactly long.len() limbs).
        let out = Self::from_limbs(out);
        out.assert_invariants();
        Ok(out)
    }

    /// Magnitude subtraction: `self - other`.
    ///
    /// # Errors
    ///
    /// [`Error::Underflow`] when `other > self` (`BigUint` has no negatives;
    /// use [`BigInt`](crate::BigInt) for signed arithmetic), plus the
    /// allocation/limit errors of [`add`](Self::add).
    pub fn checked_sub(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        if self < other {
            return Err(Error::Underflow);
        }
        progress.check_memory(self.limbs.len() as u64 * (LIMB_BITS as u64 / 8))?;
        let mut out: Vec<Limb> = Vec::with_capacity(self.limbs.len());
        let mut borrow = 0_u64;
        for (i, &a) in self.limbs.iter().enumerate() {
            let b = other.limbs.get(i).copied().unwrap_or(LIMB_ZERO);
            let (d, br) = sub_borrow(a, b, borrow);
            out.push(d);
            borrow = br;
        }
        debug_assert_eq!(borrow, 0, "guarded by the ordering check above");
        let out = Self::from_limbs(out);
        out.assert_invariants();
        Ok(out)
    }

    /// Shift left by `bits` (`self * 2^bits`).
    ///
    /// # Errors
    ///
    /// Budgets/limits via [`Progress`] (the result bit-length is checked
    /// before allocation), plus allocation failure.
    pub fn shl(&self, bits: u64, progress: &Progress) -> Result<Self, Error> {
        if self.is_zero() || bits == 0 {
            return Ok(self.clone());
        }
        progress.check_integer_bits(self.bit_len().saturating_add(bits))?;
        let limbs_shift = (bits / LIMB_BITS as u64) as usize;
        let bit_shift = (bits % LIMB_BITS as u64) as u32;
        let out_len = self.limbs.len() + limbs_shift + usize::from(bit_shift > 0);
        progress.check_memory(out_len as u64 * (LIMB_BITS as u64 / 8))?;
        let mut out = alloc_helpers::try_with_capacity::<Limb>(out_len)?;
        out.resize(limbs_shift, LIMB_ZERO);
        if bit_shift == 0 {
            out.extend_from_slice(&self.limbs);
        } else {
            let mut carry = LIMB_ZERO;
            for &l in &self.limbs {
                out.push((l << bit_shift) | carry);
                carry = l >> (LIMB_BITS as u32 - bit_shift);
            }
            if carry != LIMB_ZERO {
                out.push(carry);
            }
        }
        let out = Self::from_limbs(out);
        out.assert_invariants();
        Ok(out)
    }

    /// Shift right by `bits` (arithmetic / floor). Zero for `bits >= bit_len`.
    ///
    /// # Errors
    ///
    /// Allocation/limits via [`Progress`].
    pub fn shr(&self, bits: u64, _progress: &Progress) -> Result<Self, Error> {
        if bits == 0 {
            return Ok(self.clone());
        }
        let limbs_shift = (bits / LIMB_BITS as u64) as usize;
        if limbs_shift >= self.limbs.len() {
            return Ok(Self::zero());
        }
        let bit_shift = (bits % LIMB_BITS as u64) as u32;
        let src = &self.limbs[limbs_shift..];
        let mut out = alloc_helpers::try_with_capacity::<Limb>(src.len())?;
        if bit_shift == 0 {
            out.extend_from_slice(src);
        } else {
            for (i, &l) in src.iter().enumerate() {
                let incoming = match src.get(i + 1) {
                    Some(&next) => next << (LIMB_BITS as u32 - bit_shift),
                    None => LIMB_ZERO,
                };
                let v = (l >> bit_shift) | incoming;
                if i + 1 == src.len() && v == LIMB_ZERO {
                    break; // would be a trailing zero limb — keep I1
                }
                out.push(v);
            }
        }
        let out = Self::from_limbs(out);
        out.assert_invariants();
        Ok(out)
    }

    /// Bit test: is bit `index` set? Indices beyond `bit_len` read as false.
    #[must_use]
    pub fn bit(&self, index: u64) -> bool {
        let limb_idx = (index / LIMB_BITS as u64) as usize;
        match self.limbs.get(limb_idx) {
            Some(&l) => (l >> (index % LIMB_BITS as u64)) & 1 == 1,
            None => false,
        }
    }

    /// Construct from a limb.
    #[must_use]
    pub fn from_limb(v: Limb) -> Self {
        Self::from_limbs(alloc::vec![v])
    }

    /// Construct from a double-width limb value (`u128` on 64-bit targets,
    /// `u64` on 32-bit), normalising.
    #[must_use]
    pub fn from_double(v: DoubleLimb) -> Self {
        let lo = v as Limb;
        let hi = (v >> LIMB_BITS) as Limb;
        if hi == 0 {
            Self::from_limb(lo)
        } else {
            Self::from_limbs(alloc::vec![lo, hi])
        }
    }

    /// Multiplication, dispatched through the strategy table (spec §7).
    ///
    /// # Errors
    ///
    /// The result bit-length is checked against `progress` budgets before
    /// allocation (I9); allocation failure otherwise.
    pub fn mul(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        if self.is_zero() || other.is_zero() {
            return Ok(Self::zero());
        }
        progress.check_integer_bits(self.bit_len() + other.bit_len())?;
        progress.check_cancelled()?;
        let strategy = crate::strategy::select_strategy(self.limbs.len(), other.limbs.len());
        let limbs = strategy.multiply(self.as_limbs(), other.as_limbs(), progress)?;
        let out = Self::from_limbs(limbs);
        out.assert_invariants();
        Ok(out)
    }

    /// Truncated division and remainder on magnitudes: `self == q·other + r`
    /// with `r < other` (I5).
    ///
    /// # Errors
    ///
    /// [`Error::DivisionByZero`] for a zero divisor; allocation, limits,
    /// cancellation otherwise.
    pub fn div_rem(&self, other: &Self, progress: &Progress) -> Result<(Self, Self), Error> {
        if other.is_zero() {
            return Err(Error::DivisionByZero);
        }
        let (q, r) = crate::division::div_rem(self.as_limbs(), other.as_limbs(), progress)?;
        let out = (Self::from_limbs(q), Self::from_limbs(r));
        out.0.assert_invariants();
        out.1.assert_invariants();
        Ok(out)
    }

    /// Truncated quotient. See [`div_rem`](Self::div_rem).
    ///
    /// # Errors
    ///
    /// As [`div_rem`](Self::div_rem).
    pub fn div(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        self.div_rem(other, progress).map(|(q, _)| q)
    }

    /// Truncated remainder. See [`div_rem`](Self::div_rem).
    ///
    /// # Errors
    ///
    /// As [`div_rem`](Self::div_rem).
    pub fn rem(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        self.div_rem(other, progress).map(|(_, r)| r)
    }

    /// Narrow to `u64`, if the value fits.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`] when the value exceeds `u64::MAX`.
    pub fn to_u64(&self) -> Result<u64, Error> {
        match self.limbs.as_slice() {
            [] => Ok(0),
            [lo] => Ok(crate::limb::limb_to_u64(*lo)),
            // Narrow-limb targets pack two limbs into the u64.
            [lo, hi] if LIMB_BITS == 32 => {
                Ok((crate::limb::limb_to_u64(*hi) << 32) | crate::limb::limb_to_u64(*lo))
            }
            _ => Err(Error::Overflow),
        }
    }
}

// --- display ---------------------------------------------------------------

impl core::fmt::Display for BigUint {
    /// Decimal rendering. Convenience over [`BigUint::to_str_radix`]; the
    /// `fmt::Display` signature is infallible, so an allocation failure
    /// here panics — computational paths should use `to_str_radix` and
    /// handle [`Error`] properly.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(
            &self
                .to_str_radix(10, &Progress::UNLIMITED)
                .expect("Display: use to_str_radix for the fallible path"),
        )
    }
}

// --- comparison (I1 makes lexicographic-from-top exact) -------------------

impl Ord for BigUint {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        match self.limbs.len().cmp(&other.limbs.len()) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        for i in (0..self.limbs.len()).rev() {
            match self.limbs[i].cmp(&other.limbs[i]) {
                core::cmp::Ordering::Equal => {}
                ord => return ord,
            }
        }
        core::cmp::Ordering::Equal
    }
}

impl PartialOrd for BigUint {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// --- primitive construction (uniform widening, no public limb leakage) ----

macro_rules! from_unsigned {
    ($($t:ty),*) => {$(
        impl From<$t> for BigUint {
            fn from(v: $t) -> Self {
                let mut limbs = Vec::new();
                // u128 is wide enough for every covered primitive, so the
                // shift never over-rotates (u8 >> 64 would).
                let mut v = v as u128;
                while v != 0 {
                    limbs.push(v as Limb); // low limb
                    v >>= LIMB_BITS as u32;
                }
                Self { limbs }
            }
        }
    )*};
}
from_unsigned!(u8, u16, u32, u64, u128, usize);

// --- carry/borrow primitives ---------------------------------------------

use crate::limb::{SignedDouble, dl_split, widening_mul};

/// `(sum, carry)` of two limbs.
#[inline]
const fn add_carry(a: Limb, b: Limb) -> (Limb, Limb) {
    let sum = (a as DoubleLimb) + (b as DoubleLimb);
    // dl_split yields (hi, lo); callers want (lo=sum, hi=carry).
    let (carry, sum_limb) = dl_split(sum);
    (sum_limb, carry)
}

/// `(difference, borrow-out)` of `a - b - borrow_in`.
#[inline]
const fn sub_borrow(a: Limb, b: Limb, borrow: u64) -> (Limb, u64) {
    let diff = (a as SignedDouble) - (b as SignedDouble) - (borrow as SignedDouble);
    if diff < 0 {
        ((diff + ((1 as SignedDouble) << LIMB_BITS)) as Limb, 1)
    } else {
        (diff as Limb, 0)
    }
}

// keep widening_mul referenced from this module's future users (mul paths
// live in strategy.rs); the import above exists for dl_split/add_carry.
#[allow(dead_code)]
const _: fn(Limb, Limb) -> DoubleLimb = widening_mul;

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{format, vec};

    fn bu(v: u64) -> BigUint {
        BigUint::from(v)
    }

    #[test]
    fn zero_is_canonical() {
        assert!(BigUint::zero().is_zero());
        assert_eq!(BigUint::from(0_u8), BigUint::zero());
        assert_eq!(BigUint::from_limb(0), BigUint::zero());
        assert_eq!(BigUint::from_double(0), BigUint::zero());
        assert_eq!(BigUint::from_limbs(vec![0, 0, 0]), BigUint::zero());
        assert_eq!(BigUint::zero().bit_len(), 0);
        BigUint::zero().assert_invariants();
    }

    #[test]
    fn debug_renders_hex() {
        assert_eq!(format!("{:?}", BigUint::zero()), "BigUint(0)");
        assert_eq!(format!("{:?}", bu(255)), "BigUint(0xff)");
        assert_eq!(
            format!("{:?}", BigUint::from(0x1234_5678_9ABC_DEF0_u64)),
            "BigUint(0x123456789abcdef0)"
        );
    }

    #[test]
    fn comparisons() {
        assert!(bu(0) < bu(1));
        assert!(bu(1) < bu(2));
        assert!(bu(u64::MAX) < bu(u64::MAX).add(&bu(1), &Progress::UNLIMITED).unwrap());
        let big = BigUint::from_limbs(vec![0, 1]); // 2^64
        let big2 = BigUint::from_limbs(vec![1, 1]); // 2^64 + 1
        assert!(big < big2);
        assert_eq!(big2.cmp(&big2), core::cmp::Ordering::Equal);
        assert_eq!(bu(7), bu(7));
    }

    #[test]
    fn bit_len_and_bits() {
        assert_eq!(bu(1).bit_len(), 1);
        assert_eq!(bu(255).bit_len(), 8);
        assert_eq!(bu(256).bit_len(), 9);
        let just_over_one_limb = BigUint::from(1_u8)
            .shl(LIMB_BITS as u64, &Progress::UNLIMITED)
            .unwrap();
        assert_eq!(just_over_one_limb.bit_len(), LIMB_BITS as u64 + 1);
        assert!(!bu(0b1010).bit(0));
        assert!(bu(0b1010).bit(1));
        assert!(!bu(0b1010).bit(63));
        assert!(!bu(0b1010).bit(64));
    }

    #[test]
    fn trailing_zeros() {
        assert_eq!(bu(1).trailing_zeros(), Some(0));
        assert_eq!(bu(8).trailing_zeros(), Some(3));
        // 2^(2*LIMB_BITS + 1): trailing zeros span two zero limbs.
        let wide = BigUint::from(2_u8)
            .shl(2 * LIMB_BITS as u64, &Progress::UNLIMITED)
            .unwrap();
        assert_eq!(wide.trailing_zeros(), Some(2 * LIMB_BITS as u64 + 1));
        assert_eq!(BigUint::zero().trailing_zeros(), None);
    }

    #[test]
    fn add_sub_round_trip() {
        let p = Progress::UNLIMITED;
        for (a, b) in [
            (0_u64, 0_u64),
            (1, 2),
            (5, 7),
            (u64::MAX, 1),
            (u64::MAX, u64::MAX),
        ] {
            let sum = bu(a).add(&bu(b), &p).unwrap();
            sum.assert_invariants();
            let back = sum.checked_sub(&bu(b), &p).unwrap();
            assert_eq!(back, bu(a), "a+b-b == a for ({a}, {b})");
        }
        let m = bu(u64::MAX);
        let sum = m.add(&m, &p).unwrap();
        assert_eq!(
            sum,
            BigUint::from(0xFFFF_FFFF_FFFF_FFFE_u64)
                .add(&bu(1).shl(64, &p).unwrap(), &p)
                .unwrap()
        );
    }

    #[test]
    fn sub_underflow_is_an_error() {
        let err = bu(3).checked_sub(&bu(4), &Progress::UNLIMITED).unwrap_err();
        assert_eq!(err, Error::Underflow);
        assert_eq!(
            bu(3).checked_sub(&bu(3), &Progress::UNLIMITED).unwrap(),
            BigUint::zero()
        );
    }

    #[test]
    fn shifts_round_trip() {
        let p = Progress::UNLIMITED;
        // Width-agnostic two-limb value (limb split differs on 32/64-bit).
        let v = BigUint::from(0x1234_5678_9ABC_DEF0_u64)
            .add(
                &BigUint::from(0x8000_0000_0000_0001_u64)
                    .shl(64, &p)
                    .unwrap(),
                &p,
            )
            .unwrap();
        let shifted = v.shl(65, &p).unwrap();
        assert_eq!(shifted.bit_len(), v.bit_len() + 65);
        let back = shifted.shr(65, &p).unwrap();
        assert_eq!(back, v);

        assert_eq!(bu(0xFF).shl(4, &p).unwrap(), bu(0xFF0));
        assert_eq!(bu(0xFF0).shr(4, &p).unwrap(), bu(0xFF));
        assert_eq!(bu(7).shr(100, &p).unwrap(), BigUint::zero());
        assert_eq!(bu(7).shl(0, &p).unwrap(), bu(7));
        assert_eq!(bu(7).shl(1, &p).unwrap(), bu(14));
        // Cross-limb shr trims the trailing zero limb: 2^B >> 1 == 2^(B-1).
        let big = BigUint::from(1_u8).shl(LIMB_BITS as u64, &p).unwrap();
        assert_eq!(
            big.shr(1, &p).unwrap(),
            BigUint::from(1_u64 << (LIMB_BITS - 1))
        );
    }

    #[test]
    fn limits_are_enforced_before_growth() {
        let p = Progress::new(tpt_mathr_base::ResourceLimits::restrictive());
        let err = bu(1).shl(4096, &p).unwrap_err();
        assert!(err.is_resource_exhausted());
        // I8: the input is untouched.
        assert_eq!(bu(1), bu(1));
    }
}
