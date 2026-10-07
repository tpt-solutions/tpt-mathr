// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! [`BigRat`] — exact rational arithmetic (Phase 4).
//!
//! # Representation (invariants: `docs/invariants.md`, section `BigRat`)
//!
//! A rational is a signed numerator ([`BigInt`]) over a strictly positive
//! denominator ([`BigUint`]), stored **always reduced** (`gcd(|num|, den)
//! == 1`). Sign lives in the numerator; zero is `0/1` and canonical, so
//! equality is component-wise and there is exactly one representation per
//! value.
//!
//! # Rounding conventions
//!
//! Division is **truncated** (matches the integer layer, I5). [`BigRat::round`]
//! rounds **half away from zero** — the convention of Rust's primitive
//! floats; this is normative and pinned by tests (Python's `round` is
//! half-even and must not be used as its reference).

use alloc::string::String;

use tpt_mathr_base::error::Error;

use crate::bigint::{BigInt, Sign};
use crate::biguint::BigUint;
use crate::progress::Progress;
use crate::radix::{MAX_RADIX, MIN_RADIX};

/// An exact rational number: reduced `num / den` with `den > 0`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BigRat {
    num: BigInt,
    den: BigUint,
}

impl BigRat {
    /// The zero rational (`0/1`).
    #[must_use]
    pub fn zero() -> Self {
        Self {
            num: BigInt::zero(),
            den: BigUint::one(),
        }
    }

    /// The one rational (`1/1`).
    #[must_use]
    pub fn one() -> Self {
        Self {
            num: BigInt::one(),
            den: BigUint::one(),
        }
    }

    /// `true` iff the value is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.num.is_zero()
    }

    /// Construct from numerator and denominator, reducing to canonical
    /// form. Arguments are consumed; the reduced components are derived
    /// from them.
    ///
    /// # Errors
    ///
    /// [`Error::DivisionByZero`] for a zero denominator; allocation, limits,
    /// cancellation from the reduction gcd.
    pub fn new(num: BigInt, den: BigUint, progress: &Progress) -> Result<Self, Error> {
        if den.is_zero() {
            return Err(Error::DivisionByZero);
        }
        if num.is_zero() {
            return Ok(Self::zero());
        }
        // The gcd divides both magnitudes exactly; the truncated-division
        // remainders are therefore zero by construction (I6).
        let (sign, num_mag) = num.into_parts();
        let den_val = BigUint::from_limbs(den.into_limbs());
        let g = num_mag.gcd(&den_val, progress)?;
        let (num_mag, _) = num_mag.div_rem(&g, progress)?;
        let (den_mag, _) = den_val.div_rem(&g, progress)?;
        Ok(Self {
            num: BigInt::from_sign_and_magnitude(sign, num_mag),
            den: den_mag,
        })
    }

    /// The numerator (carries the sign).
    #[must_use]
    pub fn numerator(&self) -> &BigInt {
        &self.num
    }

    /// The denominator (strictly positive, coprime to the numerator).
    #[must_use]
    pub fn denominator(&self) -> &BigUint {
        &self.den
    }

    /// Negation (cheap: sign flip only, reduction unchanged).
    #[must_use]
    pub fn neg(&self) -> Self {
        Self {
            num: self.num.neg(),
            den: self.den.clone(),
        }
    }

    /// Absolute value.
    #[must_use]
    pub fn abs(&self) -> Self {
        Self {
            num: self.num.abs(),
            den: self.den.clone(),
        }
    }

    /// Multiplicative inverse.
    ///
    /// # Errors
    ///
    /// [`Error::DivisionByZero`] for zero.
    pub fn inv(&self) -> Result<Self, Error> {
        if self.is_zero() {
            return Err(Error::DivisionByZero);
        }
        Ok(Self {
            num: match self.num.sign() {
                Sign::Positive => BigInt::from_sign_and_magnitude(Sign::Positive, self.den.clone()),
                Sign::Negative => BigInt::from_sign_and_magnitude(Sign::Negative, self.den.clone()),
            },
            den: self.num.magnitude().clone(),
        })
    }

    /// Addition.
    ///
    /// # Errors
    ///
    /// Allocation, limits, cancellation.
    pub fn add(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        // a/b + c/d = (a·d + c·b) / (b·d)
        let ad = self.num.mul(&bigint_of(&other.den), progress)?;
        let cb = other.num.mul(&bigint_of(&self.den), progress)?;
        let bd = self.den.mul(&other.den, progress)?;
        let num = ad.add(&cb, progress)?;
        Self::new(num, bd, progress)
    }

    /// Subtraction.
    ///
    /// # Errors
    ///
    /// Allocation, limits, cancellation.
    pub fn checked_sub(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        let ad = self.num.mul(&bigint_of(&other.den), progress)?;
        let cb = other.num.mul(&bigint_of(&self.den), progress)?;
        let bd = self.den.mul(&other.den, progress)?;
        let num = ad.checked_sub(&cb, progress)?;
        Self::new(num, bd, progress)
    }

    /// Multiplication.
    ///
    /// # Errors
    ///
    /// Allocation, limits, cancellation.
    pub fn mul(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        let ac = self.num.mul(&other.num, progress)?;
        let bd = self.den.mul(&other.den, progress)?;
        Self::new(ac, bd, progress)
    }

    /// Division: `(a/b) / (c/d) = (a/b) · (d/c)` via the exact inverse, so
    /// the divisor's sign is carried by construction.
    ///
    /// # Errors
    ///
    /// [`Error::DivisionByZero`] for a zero divisor; allocation, limits,
    /// cancellation otherwise.
    pub fn div(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        if other.is_zero() {
            return Err(Error::DivisionByZero);
        }
        self.mul(&other.inv()?, progress)
    }

    /// Truncated remainder: `self - trunc(self/other)·other` (I5 analogue
    /// over rationals; `sign(rem) == sign(self)`, `|rem| < |other|`).
    ///
    /// # Errors
    ///
    /// [`Error::DivisionByZero`] for a zero divisor; allocation, limits,
    /// cancellation otherwise.
    pub fn rem(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        let q = self.div(other, progress)?;
        let q_trunc = Self::from(trunc_to_bigint(&q));
        let prod = q_trunc.mul(other, progress)?;
        self.checked_sub(&prod, progress)
    }

    /// Comparison.
    ///
    /// # Errors
    ///
    /// Allocation, limits, cancellation (cross-multiplication).
    pub fn cmp_rat(&self, other: &Self, progress: &Progress) -> Result<core::cmp::Ordering, Error> {
        // a/b vs c/d  <=>  ad vs cb (b, d > 0)
        let ad = self.num.mul(&bigint_of(&other.den), progress)?;
        let cb = other.num.mul(&bigint_of(&self.den), progress)?;
        Ok(ad.cmp(&cb))
    }

    /// Floor: greatest integer `<= self`.
    ///
    /// # Errors
    ///
    /// Allocation, limits, cancellation.
    pub fn floor(&self, progress: &Progress) -> Result<BigInt, Error> {
        let (q, r) = self.num.div_rem(&bigint_of(&self.den), progress)?;
        // Truncated q is the floor when the remainder is zero or the value
        // was already integral / positive; for a negative non-integral value
        // the floor is one below the truncation.
        let needs_adjust = !r.is_zero() && self.num.sign() == Sign::Negative;
        if needs_adjust {
            q.checked_sub(&BigInt::one(), progress)
        } else {
            Ok(q)
        }
    }

    /// Ceil: least integer `>= self`.
    ///
    /// # Errors
    ///
    /// Allocation, limits, cancellation.
    pub fn ceil(&self, progress: &Progress) -> Result<BigInt, Error> {
        self.neg().floor(progress).map(|f| f.neg())
    }

    /// Round to nearest integer, **ties away from zero** (normative; see
    /// the module docs).
    ///
    /// # Errors
    ///
    /// Allocation, limits, cancellation.
    pub fn round(&self, progress: &Progress) -> Result<BigInt, Error> {
        // Truncate, then classify the fractional part |r|/den against 1/2:
        // 2·|r| > den → move one step away from zero; == → tie, away from
        // zero; < → stay at the truncation.
        let (q, r) = self.num.div_rem(&bigint_of(&self.den), progress)?;
        let twice_rem = r.magnitude().shl(1, progress)?;
        match twice_rem.cmp(&self.den) {
            core::cmp::Ordering::Greater | core::cmp::Ordering::Equal => {
                if self.num.sign() == Sign::Negative {
                    q.checked_sub(&BigInt::one(), progress)
                } else {
                    q.add(&BigInt::one(), progress)
                }
            }
            core::cmp::Ordering::Less => Ok(q),
        }
    }

    /// Render as `num/den` in `radix` (`den` omitted when 1, `-` prefix on
    /// the whole fraction for negatives).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidRadix`], allocation, limits, cancellation.
    pub fn to_str_radix(&self, radix: u32, progress: &Progress) -> Result<String, Error> {
        if !(MIN_RADIX..=MAX_RADIX).contains(&radix) {
            return Err(Error::InvalidRadix {
                radix: radix.clamp(0, u8::MAX as i32 as u32) as u8,
            });
        }
        let num = self.num.to_str_radix(radix, progress)?;
        if self.den == BigUint::one() {
            Ok(num)
        } else {
            let den = self.den.to_str_radix(radix, progress)?;
            let mut s = String::with_capacity(num.len() + den.len() + 1);
            s.push_str(&num);
            s.push('/');
            s.push_str(&den);
            Ok(s)
        }
    }

    /// Parse `num/den` or bare `num` in `radix`; the result is reduced.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidRadix`], [`Error::Parse`], [`Error::DivisionByZero`]
    /// for a zero denominator, allocation/limits/cancellation.
    pub fn from_str_radix(s: &str, radix: u32, progress: &Progress) -> Result<Self, Error> {
        if !(MIN_RADIX..=MAX_RADIX).contains(&radix) {
            return Err(Error::InvalidRadix {
                radix: radix.clamp(0, u8::MAX as i32 as u32) as u8,
            });
        }
        match s.split_once('/') {
            None => Ok(Self::from(BigInt::from_str_radix(s, radix, progress)?)),
            Some((n, d)) => {
                if d.is_empty() || n.is_empty() {
                    return Err(Error::Parse(tpt_mathr_base::ParseError::empty()));
                }
                let num = BigInt::from_str_radix(n, radix, progress)?;
                let den = BigUint::from_str_radix(d, radix, progress)?;
                Self::new(num, den, progress)
            }
        }
    }

    /// Full invariant check: denominator positive, fraction reduced, zero
    /// canonical (`0/1`).
    ///
    /// # Panics
    ///
    /// Naming the violated invariant.
    pub fn assert_invariants(&self) {
        assert!(!self.den.is_zero(), "BigRat violated: zero denominator");
        if self.is_zero() {
            assert!(
                self.num.magnitude().is_zero() && self.den == BigUint::one(),
                "BigRat violated: zero must be canonical 0/1"
            );
            return;
        }
        let g = self
            .num
            .magnitude()
            .gcd(&self.den, &Progress::UNLIMITED)
            .expect("gcd allocates");
        assert!(
            g == BigUint::one(),
            "BigRat violated: fraction not reduced (gcd = {g})"
        );
    }
}

/// View a `BigUint` as a non-negative `BigInt` without copying semantics.
fn bigint_of(v: &BigUint) -> BigInt {
    BigInt::from(v.clone())
}

/// Truncate a rational (as numerator/denominator) to its integer part.
fn trunc_to_bigint(q: &BigRat) -> BigInt {
    let (t, _r) = q
        .num
        .div_rem(&bigint_of(&q.den), &Progress::UNLIMITED)
        .expect("denominator is positive by invariant");
    t
}

impl From<BigInt> for BigRat {
    fn from(num: BigInt) -> Self {
        if num.is_zero() {
            Self::zero()
        } else {
            Self {
                num,
                den: BigUint::one(),
            }
        }
    }
}

impl From<BigUint> for BigRat {
    fn from(v: BigUint) -> Self {
        Self::from(BigInt::from(v))
    }
}

impl From<i64> for BigRat {
    fn from(v: i64) -> Self {
        Self::from(BigInt::from(v))
    }
}

impl core::fmt::Display for BigRat {
    /// Decimal rendering `num` or `num/den`. Convenience over
    /// [`BigRat::to_str_radix`]; allocation failure panics here (the
    /// fallible path is `to_str_radix`).
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(
            &self
                .to_str_radix(10, &Progress::UNLIMITED)
                .expect("Display: use to_str_radix for the fallible path"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    fn r(num: i64, den: u64) -> BigRat {
        BigRat::new(BigInt::from(num), BigUint::from(den), &Progress::UNLIMITED).unwrap()
    }

    fn ri(v: i64) -> BigRat {
        BigRat::from(v)
    }

    #[test]
    fn construction_reduces_and_canonicalises() {
        assert_eq!(r(2, 4), r(1, 2));
        assert_eq!(r(-6, 4), r(6, 4).neg());
        assert_eq!(r(0, 5), BigRat::zero());
        assert_eq!(BigRat::zero().denominator(), &BigUint::one());
        assert_eq!(r(6, 4).numerator(), &BigInt::from(3_i64));
        assert_eq!(r(6, 4).denominator(), &BigUint::from(2_u8));
        r(6, 4).assert_invariants();
        // Zero denominator is refused, never wrapped:
        assert_eq!(
            BigRat::new(BigInt::one(), BigUint::zero(), &Progress::UNLIMITED).unwrap_err(),
            Error::DivisionByZero
        );
    }

    #[test]
    fn arithmetic_field_identities() {
        let p = Progress::UNLIMITED;
        let a = r(1, 3);
        let b = r(1, 6);
        assert_eq!(a.add(&b, &p).unwrap(), r(1, 2));
        assert_eq!(a.checked_sub(&b, &p).unwrap(), r(1, 6));
        assert_eq!(a.mul(&b, &p).unwrap(), r(1, 18));
        assert_eq!(a.div(&b, &p).unwrap(), r(2, 1));
        assert_eq!(ri(5).rem(&r(3, 2), &p).unwrap(), r(1, 2));
        assert_eq!(r(-5, 1).rem(&r(3, 2), &p).unwrap(), r(-1, 2));
        assert_eq!(a.add(&a.neg(), &p).unwrap(), BigRat::zero());
        assert_eq!(a.mul(&a.inv().unwrap(), &p).unwrap(), BigRat::one());
        assert_eq!(
            ri(1).div(&BigRat::zero(), &p).unwrap_err(),
            Error::DivisionByZero
        );
        assert_eq!(BigRat::zero().inv().unwrap_err(), Error::DivisionByZero);
    }

    #[test]
    fn comparisons_cross_multiply() {
        use core::cmp::Ordering;
        let p = Progress::UNLIMITED;
        assert_eq!(r(1, 3).cmp_rat(&r(1, 2), &p).unwrap(), Ordering::Less);
        assert_eq!(r(-1, 3).cmp_rat(&r(1, 2), &p).unwrap(), Ordering::Less);
        assert_eq!(r(-1, 2).cmp_rat(&r(-1, 3), &p).unwrap(), Ordering::Less);
        assert_eq!(r(2, 4).cmp_rat(&r(1, 2), &p).unwrap(), Ordering::Equal);
        assert_eq!(r(1, 3).cmp_rat(&r(1, 2), &p).unwrap(), Ordering::Less);
    }

    #[test]
    fn floor_ceil_round_conventions() {
        let p = Progress::UNLIMITED;
        assert_eq!(r(7, 2).floor(&p).unwrap(), BigInt::from(3_i64));
        assert_eq!(r(7, 2).ceil(&p).unwrap(), BigInt::from(4_i64));
        assert_eq!(r(-7, 2).floor(&p).unwrap(), BigInt::from(-4_i64));
        assert_eq!(r(-7, 2).ceil(&p).unwrap(), BigInt::from(-3_i64));
        // Round: ties AWAY from zero (normative).
        assert_eq!(r(7, 2).round(&p).unwrap(), BigInt::from(4_i64));
        assert_eq!(r(-7, 2).round(&p).unwrap(), BigInt::from(-4_i64));
        assert_eq!(r(5, 2).round(&p).unwrap(), BigInt::from(3_i64));
        assert_eq!(r(-5, 2).round(&p).unwrap(), BigInt::from(-3_i64));
        assert_eq!(r(1, 3).round(&p).unwrap(), BigInt::zero());
        assert_eq!(r(2, 3).round(&p).unwrap(), BigInt::one());
        assert_eq!(r(1, 2).round(&p).unwrap(), BigInt::one());
        assert_eq!(r(-1, 2).round(&p).unwrap(), BigInt::from(-1_i64));
        // Integers are their own floor/ceil/round:
        assert_eq!(ri(5).floor(&p).unwrap(), BigInt::from(5_i64));
        assert_eq!(ri(-5).ceil(&p).unwrap(), BigInt::from(-5_i64));
    }

    #[test]
    fn display_and_parse_round_trip() {
        let p = Progress::UNLIMITED;
        assert_eq!(r(1, 2).to_string(), "1/2");
        assert_eq!(r(-3, 4).to_string(), "-3/4");
        assert_eq!(ri(7).to_string(), "7");
        assert_eq!(BigRat::zero().to_string(), "0");

        for text in ["1/2", "-3/4", "22/7", "7", "0", "-1/3"] {
            let v = BigRat::from_str_radix(text, 10, &p).unwrap();
            v.assert_invariants();
            assert_eq!(v.to_str_radix(10, &p).unwrap(), text);
        }
        let hex = BigRat::from_str_radix("-a/10", 16, &p).unwrap();
        assert_eq!(hex, r(-5, 8));
        assert_eq!(hex.to_str_radix(16, &p).unwrap(), "-5/8");
        // Malformed:
        assert!(BigRat::from_str_radix("/2", 10, &p).is_err());
        assert!(BigRat::from_str_radix("1/", 10, &p).is_err());
        assert_eq!(
            BigRat::from_str_radix("1/0", 10, &p).unwrap_err(),
            Error::DivisionByZero
        );
    }

    #[test]
    fn invariants_hold_under_random_operations() {
        let mut rng = tpt_mathr_base::testing::TestRng::new(0x0BE5);
        let p = Progress::UNLIMITED;
        for _ in 0..200 {
            let a = r(rng.next_u64() as i64, rng.next_u64() | 1);
            let b = r(rng.next_u64() as i64, rng.next_u64() | 1);
            for v in [
                a.add(&b, &p).unwrap(),
                a.mul(&b, &p).unwrap(),
                a.div(&b, &p).unwrap(),
            ] {
                v.assert_invariants();
            }
            // (a/b)/(c/d) * (c/d) == a/b
            assert_eq!(a.div(&b, &p).unwrap().mul(&b, &p).unwrap(), a);
        }
    }
}
