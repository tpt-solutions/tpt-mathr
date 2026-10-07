// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! [`BigInt`] — signed arbitrary-precision integers.
//!
//! # Representation (invariants I2, I3)
//!
//! Sign plus magnitude ([`BigUint`]). Zero is canonical: an empty magnitude
//! with [`Sign::Positive`]; `(-0)` cannot be constructed or arise from any
//! operation. Division is **truncated** (`/` and `%` match Rust's
//! primitives; invariant I5).

use tpt_mathr_base::error::Error;

use crate::biguint::BigUint;
use crate::progress::Progress;

/// The sign of a [`BigInt`]. Zero is always `Positive` (I3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(
    clippy::exhaustive_enums,
    reason = "a two-variant sign is the whole point; exhaustiveness at call sites is desired"
)]
pub enum Sign {
    /// Value > 0, or zero (canonical zero always carries this sign).
    Positive,
    /// Value < 0.
    Negative,
}

impl core::ops::Not for Sign {
    type Output = Sign;
    fn not(self) -> Sign {
        match self {
            Sign::Positive => Sign::Negative,
            Sign::Negative => Sign::Positive,
        }
    }
}

/// A signed arbitrary-precision integer: [`Sign`] + [`BigUint`] magnitude.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BigInt {
    sign: Sign,
    magnitude: BigUint,
}

impl BigInt {
    /// Zero (canonical: `Sign::Positive`, empty magnitude).
    #[must_use]
    pub fn zero() -> Self {
        Self {
            sign: Sign::Positive,
            magnitude: BigUint::zero(),
        }
    }

    /// One.
    #[must_use]
    pub fn one() -> Self {
        Self::from(BigUint::one())
    }

    /// `true` iff zero (I2/I3).
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.magnitude.is_zero()
    }

    /// The sign; always `Positive` for zero (I3).
    #[must_use]
    pub const fn sign(&self) -> Sign {
        self.sign
    }

    /// The absolute value, shared (no copy).
    #[must_use]
    pub fn magnitude(&self) -> &BigUint {
        &self.magnitude
    }

    /// Construct from sign and magnitude, normalising zero (I3).
    #[must_use]
    pub fn from_sign_and_magnitude(sign: Sign, magnitude: BigUint) -> Self {
        if magnitude.is_zero() {
            Self::zero()
        } else {
            Self { sign, magnitude }
        }
    }

    /// Consume a canonical residue (non-negative by contract) into its
    /// magnitude. Internal helper for the modular layer.
    pub(crate) fn into_residue_magnitude(self) -> BigUint {
        debug_assert!(self.sign == Sign::Positive, "residue must be canonical");
        self.magnitude
    }

    /// Decompose into `(sign, magnitude)`, consuming `self`.
    #[must_use]
    pub fn into_parts(self) -> (Sign, BigUint) {
        (self.sign, self.magnitude)
    }

    /// Absolute value as an owned `BigInt`.
    #[must_use]
    pub fn abs(&self) -> Self {
        Self::from(self.magnitude.clone())
    }

    /// Negation.
    #[must_use]
    pub fn neg(&self) -> Self {
        if self.is_zero() {
            Self::zero()
        } else {
            Self {
                sign: !self.sign,
                magnitude: self.magnitude.clone(),
            }
        }
    }

    /// Full invariant check (I3 via the constructor; magnitude invariants
    /// delegated to `BigUint::assert_invariants`).
    ///
    /// # Panics
    ///
    /// With a message naming the violated invariant (I2/I3/I1).
    pub fn assert_invariants(&self) {
        if self.magnitude.is_zero() {
            assert_eq!(
                self.sign,
                Sign::Positive,
                "I3 violated: zero must be Positive"
            );
        }
        self.magnitude.assert_invariants();
    }

    /// Addition.
    ///
    /// # Errors
    ///
    /// Allocation, limits, cancellation (see [`BigUint::add`]).
    pub fn add(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        let out = match (self.sign, other.sign) {
            (Sign::Positive, Sign::Positive) => {
                Self::from(self.magnitude.add(&other.magnitude, progress)?)
            }
            (Sign::Negative, Sign::Negative) => {
                Self::from(self.magnitude.add(&other.magnitude, progress)?).neg_into()
            }
            // Opposite signs: subtract magnitudes; larger magnitude wins.
            _ => match self.magnitude.cmp(&other.magnitude) {
                core::cmp::Ordering::Equal => Self::zero(),
                core::cmp::Ordering::Greater => Self::from_sign_and_magnitude(
                    self.sign,
                    self.magnitude.checked_sub(&other.magnitude, progress)?,
                ),
                core::cmp::Ordering::Less => Self::from_sign_and_magnitude(
                    other.sign,
                    other.magnitude.checked_sub(&self.magnitude, progress)?,
                ),
            },
        };
        out.assert_invariants();
        Ok(out)
    }

    /// Subtraction (`self - other`); total on signed values.
    ///
    /// # Errors
    ///
    /// Allocation, limits, cancellation.
    pub fn checked_sub(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        self.add(&other.neg(), progress)
    }

    /// Negation in place.
    fn neg_into(mut self) -> Self {
        if !self.is_zero() {
            self.sign = !self.sign;
        }
        self
    }

    /// Multiplication.
    ///
    /// # Errors
    ///
    /// Allocation, limits (bit budget checked against the operand sizes),
    /// cancellation.
    pub fn mul(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        if self.is_zero() || other.is_zero() {
            return Ok(Self::zero());
        }
        let mag = self.magnitude.mul(&other.magnitude, progress)?;
        let out = Self::from_sign_and_magnitude(
            if self.sign == other.sign {
                Sign::Positive
            } else {
                Sign::Negative
            },
            mag,
        );
        out.assert_invariants();
        Ok(out)
    }

    /// Truncated division: `(q, r)` with `self == q·other + r`,
    /// `sign(r) == sign(self)`, `|r| < |other|` (I5).
    ///
    /// # Errors
    ///
    /// [`Error::DivisionByZero`] for a zero divisor; allocation, limits,
    /// cancellation otherwise.
    pub fn div_rem(&self, other: &Self, progress: &Progress) -> Result<(Self, Self), Error> {
        if other.is_zero() {
            return Err(Error::DivisionByZero);
        }
        if self.is_zero() {
            return Ok((Self::zero(), Self::zero()));
        }
        let (q, r) = self.magnitude.div_rem(&other.magnitude, progress)?;
        let q_sign = if self.sign == other.sign {
            Sign::Positive
        } else {
            Sign::Negative
        };
        let out = (
            Self::from_sign_and_magnitude(q_sign, q),
            Self::from_sign_and_magnitude(self.sign, r),
        );
        out.0.assert_invariants();
        out.1.assert_invariants();
        Ok(out)
    }

    /// Truncated quotient. See [`div_rem`](Self::div_rem) for semantics.
    ///
    /// # Errors
    ///
    /// As [`div_rem`](Self::div_rem).
    pub fn div(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        self.div_rem(other, progress).map(|(q, _)| q)
    }

    /// Truncated remainder (`sign(r) == sign(self)`). See
    /// [`div_rem`](Self::div_rem).
    ///
    /// # Errors
    ///
    /// As [`div_rem`](Self::div_rem).
    pub fn rem(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        self.div_rem(other, progress).map(|(_, r)| r)
    }

    /// Gcd of absolute values: always non-negative (I6).
    ///
    /// # Errors
    ///
    /// Allocation, limits, cancellation.
    pub fn gcd(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        Ok(Self::from(self.magnitude.gcd(&other.magnitude, progress)?))
    }

    /// Extended gcd: `(g, x, y)` with `self·x + other·y == g`, `g >= 0`.
    ///
    /// The coefficients satisfy the identity exactly (pinned by tests and
    /// the differential corpus). Classic iterative extended Euclid on the
    /// magnitudes, with truncated division.
    ///
    /// # Errors
    ///
    /// Allocation, limits, cancellation.
    pub fn extended_gcd(&self, other: &Self, progress: &Progress) -> Result<ExtendedGcd, Error> {
        let mut old_r = self.clone();
        let mut r = other.clone();
        let mut old_s = Self::one();
        let mut s = Self::zero();
        let mut old_t = Self::zero();
        let mut t = Self::one();
        let mut iterations = 0_u64;
        while !r.is_zero() {
            iterations += 1;
            if iterations % 64 == 0 {
                progress.check_cancelled()?;
            }
            let (q, next_r) = old_r.div_rem(&r, progress)?;

            let next_s = old_s.checked_sub(&s.mul(&q, progress)?, progress)?;
            let next_t = old_t.checked_sub(&t.mul(&q, progress)?, progress)?;

            old_r = r;
            r = next_r;
            old_s = s;
            s = next_s;
            old_t = t;
            t = next_t;
        }
        // Truncated division keeps the dividend's sign in the remainders, so
        // negative operands can leave a negative gcd. Negating the whole
        // Bezout triple preserves a*x + b*y == g and restores g >= 0 (I6).
        if old_r.sign() == Sign::Negative {
            return Ok(ExtendedGcd {
                gcd: old_r.neg(),
                x: old_s.neg(),
                y: old_t.neg(),
            });
        }
        Ok(ExtendedGcd {
            gcd: old_r,
            x: old_s,
            y: old_t,
        })
    }

    /// `self^exponent` (`exponent >= 0`).
    ///
    /// # Errors
    ///
    /// Allocation, limits, cancellation.
    pub fn pow(&self, exponent: u64, progress: &Progress) -> Result<Self, Error> {
        let mag = self.magnitude.pow(exponent, progress)?;
        // The sign alternates exactly for negative bases with odd exponents;
        // zero stays canonical through from_sign_and_magnitude.
        // (-a)^e has the base's sign only for odd e; zero canonicalises in
        // from_sign_and_magnitude.
        let negative = self.sign == Sign::Negative && exponent % 2 == 1;
        let sign = if negative {
            Sign::Negative
        } else {
            Sign::Positive
        };
        Ok(Self::from_sign_and_magnitude(sign, mag))
    }

    /// Divide by a small unsigned constant, asserting exactness. Internal
    /// helper for interpolation schemes (Toom-Cook), whose divisions by
    /// small constants are provably exact.
    pub(crate) fn div_small_exact(
        &self,
        d: u64,
        progress: &Progress,
    ) -> Result<Self, Error> {
        let dv = Self::from(BigUint::from(d));
        let (q, r) = self.div_rem(&dv, progress)?;
        debug_assert!(r.is_zero(), "exact division precondition");
        Ok(q)
    }

    /// Integer square root of a **non-negative** value (I7).
    ///
    /// # Errors
    ///
    /// [`Error::UnsupportedOperation`] for negative values (never a wrapped
    /// result); allocation, limits, cancellation otherwise.
    pub fn isqrt(&self, progress: &Progress) -> Result<Self, Error> {
        if self.sign == Sign::Negative {
            return Err(Error::UnsupportedOperation);
        }
        Ok(Self::from(self.magnitude.isqrt(progress)?))
    }
}

/// Result of [`BigInt::extended_gcd`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedGcd {
    /// `gcd(a, b) >= 0`.
    pub gcd: BigInt,
    /// Bezout coefficient with `a·x + b·y == gcd`.
    pub x: BigInt,
    /// Bezout coefficient with `a·x + b·y == gcd`.
    pub y: BigInt,
}

// --- display ---------------------------------------------------------------

impl core::fmt::Display for BigInt {
    /// Decimal rendering with `-` for negatives. Convenience over
    /// [`BigInt::to_str_radix`]; the `fmt::Display` signature is infallible,
    /// so an allocation failure here panics — computational paths should
    /// use `to_str_radix` and handle [`Error`] properly.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(
            &self
                .to_str_radix(10, &Progress::UNLIMITED)
                .expect("Display: use to_str_radix for the fallible path"),
        )
    }
}

// --- comparison -----------------------------------------------------------

impl Ord for BigInt {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        use core::cmp::Ordering;
        match (self.sign, other.sign) {
            (Sign::Positive, Sign::Negative) => Ordering::Greater,
            (Sign::Negative, Sign::Positive) => Ordering::Less,
            // Same signs rank by magnitude; negative order inverts it.
            (Sign::Positive, Sign::Positive) => self.magnitude.cmp(&other.magnitude),
            (Sign::Negative, Sign::Negative) => other.magnitude.cmp(&self.magnitude),
        }
    }
}

impl PartialOrd for BigInt {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// --- conversions ----------------------------------------------------------

impl From<BigUint> for BigInt {
    fn from(magnitude: BigUint) -> Self {
        Self::from_sign_and_magnitude(Sign::Positive, magnitude)
    }
}

impl From<u64> for BigInt {
    fn from(v: u64) -> Self {
        Self::from(BigUint::from(v))
    }
}

macro_rules! from_signed_primitives {
    ($($t:ty),*) => {$(
        impl From<$t> for BigInt {
            fn from(v: $t) -> Self {
                // unsigned_abs handles $t::MIN (no negation overflow);
                // the canonical-zero rule lives in from_sign_and_magnitude.
                let sign = if v < 0 { Sign::Negative } else { Sign::Positive };
                Self::from_sign_and_magnitude(sign, BigUint::from(v.unsigned_abs()))
            }
        }
    )*};
}
from_signed_primitives!(i8, i16, i32, i64, i128, isize);

impl BigInt {
    /// Narrow to `u64` if the value is non-negative and fits.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`].
    pub fn to_u64(&self) -> Result<u64, Error> {
        match self.sign {
            Sign::Negative => Err(Error::Overflow),
            Sign::Positive => self.magnitude.to_u64(),
        }
    }

    /// Narrow to `i64` if the value fits.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`].
    pub fn to_i64(&self) -> Result<i64, Error> {
        let mag = self.magnitude.to_u64()?;
        match self.sign {
            Sign::Positive => i64::try_from(mag).map_err(|_| Error::Overflow),
            Sign::Negative => {
                // The i128 intermediate makes -(2^63) representable, which
                // the u64 domain alone cannot express.
                i64::try_from(-i128::from(mag)).map_err(|_| Error::Overflow)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biguint::BigUint;
    use tpt_mathr_base::ResourceLimits;

    fn bi(v: i64) -> BigInt {
        BigInt::from(v)
    }

    fn bu(v: u64) -> BigUint {
        BigUint::from(v)
    }

    fn i64_gcd(mut a: i64, mut b: i64) -> i64 {
        while b != 0 {
            let t = a % b;
            a = b;
            b = t;
        }
        a.abs()
    }

    #[test]
    fn canonical_zero_i3() {
        assert_eq!(bi(0).sign(), Sign::Positive);
        assert!(bi(0).is_zero());
        // Constructing with an explicit negative sign still canonicalises:
        let z = BigInt::from_sign_and_magnitude(Sign::Negative, BigUint::zero());
        assert_eq!(z.sign(), Sign::Positive);
        assert_eq!(z.neg(), BigInt::zero());
        // -0 + 0 == 0, never -0:
        assert_eq!(
            bi(-5).add(&bi(5), &Progress::UNLIMITED).unwrap(),
            BigInt::zero()
        );
        assert_eq!(
            bi(-5).mul(&bi(0), &Progress::UNLIMITED).unwrap(),
            BigInt::zero()
        );
    }

    #[test]
    fn min_i128_and_negation() {
        let min = BigInt::from(i128::MIN);
        assert_eq!(min.sign(), Sign::Negative);
        assert!(min.neg().to_i64().is_err());
        assert_eq!(min.neg().sign(), Sign::Positive);
    }

    #[test]
    fn signed_add_sub_across_signs() {
        let p = Progress::UNLIMITED;
        for (a, b) in [
            (5_i64, 3_i64),
            (-5, 3),
            (5, -3),
            (-5, -3),
            (-5, 5),
            (0, -7),
            (-7, 0),
        ] {
            let sum = bi(a).add(&bi(b), &p).unwrap();
            assert_eq!(sum.to_i64().unwrap(), a + b, "add {a}+{b}");
            let diff = bi(a).checked_sub(&bi(b), &p).unwrap();
            assert_eq!(diff.to_i64().unwrap(), a - b, "sub {a}-{b}");
        }
        // Out-of-i64-range results stay exact on magnitude:
        let big = bi(i64::MIN).add(&bi(-1), &p).unwrap();
        assert_eq!(big.sign(), Sign::Negative);
        assert_eq!(
            big.magnitude(),
            &BigUint::from(1_u64 << 63).add(&BigUint::one(), &p).unwrap(),
            "-(2^63 + 1)"
        );
        let diff = bi(i64::MIN).checked_sub(&bi(i64::MAX), &p).unwrap();
        assert_eq!(diff.sign(), Sign::Negative);
        assert_eq!(diff.magnitude(), &bu(u64::MAX), "-(2^64 - 1)");
    }

    #[test]
    fn signed_mul_div_rem_i5() {
        let p = Progress::UNLIMITED;
        for (a, b) in [
            (7_i64, 2_i64),
            (-7, 2),
            (7, -2),
            (-7, -2),
            (6, 3),
            (-6, 3),
            (0, 5),
        ] {
            let (q, r) = bi(a).div_rem(&bi(b), &p).unwrap();
            // I5: a == b*q + r, sign(r) == sign(a), |r| < |b|
            let back = bi(b).mul(&q, &p).unwrap().add(&r, &p).unwrap();
            assert_eq!(back, bi(a), "identity for {a}/{b}");
            assert!(
                r.is_zero() || r.sign() == bi(a).sign(),
                "remainder sign for {a}/{b}"
            );
            assert!(
                r.magnitude() < &bu(b.unsigned_abs()),
                "remainder bound for {a}/{b}: {r:?}"
            );
            assert_eq!(q.to_i64().unwrap(), a / b);
            assert_eq!(r.to_i64().unwrap(), a % b);
        }
        assert_eq!(
            bi(1).div_rem(&BigInt::zero(), &p).unwrap_err(),
            Error::DivisionByZero
        );
    }

    #[test]
    fn ordering_across_signs() {
        assert!(bi(-2) < bi(-1));
        assert!(bi(-1) < bi(0));
        assert!(bi(0) < bi(1));
        assert!(bi(-5) < bi(5));
        let huge_neg = BigInt::from_sign_and_magnitude(Sign::Negative, bu(u64::MAX));
        assert!(huge_neg < bi(1));
        assert!(bi(1) < huge_neg.abs());
    }

    #[test]
    fn pow_sign_rules() {
        let p = Progress::UNLIMITED;
        assert_eq!(bi(-2).pow(3, &p).unwrap(), bi(-8));
        assert_eq!(bi(-2).pow(2, &p).unwrap(), bi(4));
        assert_eq!(bi(-2).pow(0, &p).unwrap(), bi(1));
        assert_eq!(
            bi(2).pow(64, &p).unwrap().magnitude(),
            &bu(1).shl(64, &p).unwrap()
        );
    }

    #[test]
    fn isqrt_refuses_negatives() {
        let err = bi(-4).isqrt(&Progress::UNLIMITED).unwrap_err();
        assert_eq!(err, Error::UnsupportedOperation);
        assert_eq!(bi(16).isqrt(&Progress::UNLIMITED).unwrap(), bi(4));
    }

    #[test]
    fn gcd_is_absolute_and_extgcd_identity_holds() {
        let p = Progress::UNLIMITED;
        assert_eq!(bi(-12).gcd(&bi(18), &p).unwrap(), bi(6));
        for (a, b) in [
            (240_i64, 46_i64),
            (-240, 46),
            (240, -46),
            (-240, -46),
            (0, 7),
            (7, 0),
            (1, 1),
        ] {
            let ext = bi(a).extended_gcd(&bi(b), &p).unwrap();
            assert!(ext.gcd.sign() == Sign::Positive, "gcd non-negative");
            let lhs = bi(a)
                .mul(&ext.x, &p)
                .unwrap()
                .add(&bi(b).mul(&ext.y, &p).unwrap(), &p)
                .unwrap();
            assert_eq!(lhs, ext.gcd, "Bezout identity for ({a}, {b})");
            assert_eq!(ext.gcd, bi(i64_gcd(a, b)), "gcd value for ({a}, {b})");
        }
    }

    #[test]
    fn limits_apply() {
        let tight = Progress::new(ResourceLimits::restrictive());
        // 2^64 squared is only 128 bits - inside the restrictive budget.
        let big = BigInt::from(u64::MAX);
        assert!(big.mul(&big, &tight).is_ok());
        // ~6400 bits - far beyond the 1024-bit restrictive budget.
        let err = big.pow(100, &tight).unwrap_err();
        assert!(err.is_resource_exhausted());
    }
}
