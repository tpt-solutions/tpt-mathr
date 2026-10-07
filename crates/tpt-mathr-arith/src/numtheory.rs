// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Number-theoretic primitives (Phase 2 scope): `gcd`, `extended_gcd`,
//! `lcm`, `pow`, `isqrt` — with **no speed promises** (spec §5.1:
//! factorisation must never be promised universally fast, and these land
//! before the Phase 3 optimisations).

pub use crate::bigint::ExtendedGcd;
use tpt_mathr_base::error::Error;

use crate::biguint::BigUint;
use crate::division;
use crate::progress::Progress;

/// Cancel-check cadence for loops without natural safety points.
const CANCEL_EVERY: u64 = 64;

impl BigUint {
    /// `self^exponent` by binary exponentiation.
    ///
    /// # Errors
    ///
    /// Budgets (integer-bits and memory limits are checked per
    /// multiplication), cancellation, allocation.
    pub fn pow(&self, exponent: u64, progress: &Progress) -> Result<Self, Error> {
        if exponent == 0 {
            return Ok(Self::one());
        }
        if self.is_zero() {
            return Ok(Self::zero());
        }
        let mut base = self.clone();
        let mut acc = Self::one();
        let mut e = exponent;
        let mut steps = 0_u64;
        while e > 0 {
            steps += 1;
            if steps % CANCEL_EVERY == 0 {
                progress.check_cancelled()?;
            }
            if e & 1 == 1 {
                acc = acc.mul(&base, progress)?;
            }
            e >>= 1;
            if e > 0 {
                base = base.mul(&base, progress)?;
            }
        }
        acc.assert_invariants();
        Ok(acc)
    }

    /// Greatest common divisor (binary GCD / Stein's algorithm). Always
    /// non-negative; `gcd(x, 0) == x`, `gcd(0, 0) == 0` (I6).
    ///
    /// # Errors
    ///
    /// Allocation, limits, cancellation.
    ///
    /// # Panics
    ///
    /// Only from the debug invariant assertions, which are unreachable for
    /// values produced by this crate's constructors.
    pub fn gcd(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        if self.is_zero() {
            return Ok(other.clone());
        }
        if other.is_zero() {
            return Ok(self.clone());
        }
        let shift = self
            .trailing_zeros()
            .unwrap()
            .min(other.trailing_zeros().unwrap());
        let mut a = self.shr(self.trailing_zeros().unwrap(), progress)?;
        let mut b = other.shr(other.trailing_zeros().unwrap(), progress)?;
        let mut iterations = 0_u64;
        loop {
            iterations += 1;
            if iterations % CANCEL_EVERY == 0 {
                progress.check_cancelled()?;
            }
            if a > b {
                core::mem::swap(&mut a, &mut b);
            }
            // b is odd-or-zero; subtract the odd twin to get an even value.
            b = b.checked_sub(&a, progress)?;
            if b.is_zero() {
                break;
            }
            b = b.shr(b.trailing_zeros().unwrap(), progress)?;
        }
        let g = a.shl(shift, progress)?;
        g.assert_invariants();
        Ok(g)
    }

    /// Least common multiple; `lcm(0, x) == 0` (I6).
    ///
    /// # Errors
    ///
    /// Allocation, limits, cancellation.
    pub fn lcm(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        if self.is_zero() || other.is_zero() {
            return Ok(Self::zero());
        }
        let g = self.gcd(other, progress)?;
        // Divide first to keep intermediates small: self/g * other.
        let (q, _r) = division::div_rem(self.as_limbs(), g.as_limbs(), progress)?;
        let q = Self::from_limbs(q);
        q.mul(other, progress)
    }

    /// Integer square root: `floor(sqrt(self))` by Newton iteration with an
    /// exact initial bound (I7).
    ///
    /// # Errors
    ///
    /// Allocation, limits, cancellation.
    pub fn isqrt(&self, progress: &Progress) -> Result<Self, Error> {
        if self.is_zero() {
            return Ok(Self::zero());
        }
        // Initial guess >= sqrt(self): 2^ceil(bit_len / 2) satisfies
        // 2^bit_len > self, so 2^ceil(bit_len/2) >= sqrt(self).
        let bits = self.bit_len();
        let guess = Self::one().shl(bits.div_ceil(2), progress)?;
        let mut x = guess;
        let mut iterations = 0_u64;
        loop {
            iterations += 1;
            if iterations % CANCEL_EVERY == 0 {
                progress.check_cancelled()?;
            }
            // y = (x + self / x) / 2
            let (q, _r) = division::div_rem(self.as_limbs(), x.as_limbs(), progress)?;
            let q = Self::from_limbs(q);
            let sum = q.add(&x, progress)?;
            let y = sum.shr(1, progress)?;
            if y >= x {
                x.assert_invariants();
                return Ok(x);
            }
            x = y;
        }
    }
}

impl BigUint {
    /// Extended gcd on magnitudes: returns `(g, x, y)` with
    /// `a·x + b·y = g`, `g >= 0`, computed by the iterative extended
    /// Euclidean algorithm over signed intermediates. For the fully signed
    /// version see [`BigInt::extended_gcd`](crate::BigInt::extended_gcd).
    ///
    /// # Errors
    ///
    /// Allocation, limits, cancellation.
    pub fn extended_gcd(&self, other: &Self, progress: &Progress) -> Result<ExtendedGcd, Error> {
        crate::bigint::BigInt::from(self.clone())
            .extended_gcd(&crate::bigint::BigInt::from(other.clone()), progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bigint::BigInt;
    use tpt_mathr_base::ResourceLimits;

    fn bu(v: u64) -> BigUint {
        BigUint::from(v)
    }

    #[test]
    fn pow_basics() {
        let p = Progress::UNLIMITED;
        assert_eq!(bu(5).pow(0, &p).unwrap(), BigUint::one());
        assert_eq!(bu(0).pow(5, &p).unwrap(), BigUint::zero());
        assert_eq!(bu(5).pow(1, &p).unwrap(), bu(5));
        assert_eq!(bu(2).pow(10, &p).unwrap(), bu(1024));
        assert_eq!(bu(3).pow(5, &p).unwrap(), bu(243));
        let big = bu(2).pow(64, &p).unwrap();
        assert_eq!(big, BigUint::from(1_u128 << 64));
    }

    #[test]
    fn gcd_matches_definition_i6() {
        let p = Progress::UNLIMITED;
        assert_eq!(bu(12).gcd(&bu(18), &p).unwrap(), bu(6));
        assert_eq!(bu(18).gcd(&bu(12), &p).unwrap(), bu(6));
        assert_eq!(bu(0).gcd(&bu(7), &p).unwrap(), bu(7));
        assert_eq!(bu(7).gcd(&bu(0), &p).unwrap(), bu(7));
        assert_eq!(bu(0).gcd(&bu(0), &p).unwrap(), BigUint::zero());
        assert_eq!(bu(1).gcd(&bu(999_999), &p).unwrap(), bu(1));
        // gcd divides both operands:
        let a = bu(2 * 3 * 5 * 7 * 11);
        let b = bu(2 * 3 * 5 * 13 * 17);
        let g = a.gcd(&b, &p).unwrap();
        assert_eq!(g, bu(30));
        // Powers of two exercise the shift paths:
        assert_eq!(bu(1 << 20).gcd(&bu(1 << 14), &p).unwrap(), bu(1 << 14));
    }

    #[test]
    fn gcd_randomised_identity() {
        let mut rng = tpt_mathr_base::testing::TestRng::new(0x00C0_FFEE);
        let p = Progress::UNLIMITED;
        for _ in 0..200 {
            let a = bu(rng.next_u64() | 1);
            let b = bu(rng.next_u64() | 1);
            let g = a.gcd(&b, &p).unwrap();
            // Both a and b are divisible by g:
            let (_, ra) = crate::division::div_rem(a.as_limbs(), g.as_limbs(), &p).unwrap();
            let (_, rb) = crate::division::div_rem(b.as_limbs(), g.as_limbs(), &p).unwrap();
            assert!(ra.is_empty() && rb.is_empty(), "gcd must divide both");
        }
    }

    #[test]
    #[allow(
        clippy::many_single_char_names,
        reason = "identity variables a, b, g, l"
    )]
    fn lcm_matches_i6() {
        let p = Progress::UNLIMITED;
        assert_eq!(bu(4).lcm(&bu(6), &p).unwrap(), bu(12));
        assert_eq!(bu(0).lcm(&bu(9), &p).unwrap(), BigUint::zero());
        let a = bu(12);
        let b = bu(18);
        let prod = a.mul(&b, &p).unwrap();
        let g = a.gcd(&b, &p).unwrap();
        let l = a.lcm(&b, &p).unwrap();
        assert_eq!(g.mul(&l, &p).unwrap(), prod);
    }

    #[test]
    fn isqrt_matches_i7() {
        let p = Progress::UNLIMITED;
        assert_eq!(BigUint::zero().isqrt(&p).unwrap(), BigUint::zero());
        assert_eq!(bu(1).isqrt(&p).unwrap(), bu(1));
        assert_eq!(bu(3).isqrt(&p).unwrap(), bu(1));
        assert_eq!(bu(4).isqrt(&p).unwrap(), bu(2));
        assert_eq!(bu(8).isqrt(&p).unwrap(), bu(2));
        assert_eq!(bu(9).isqrt(&p).unwrap(), bu(3));
        assert_eq!(bu(2_147_483_647).isqrt(&p).unwrap(), bu(46_340));
        let sq = bu(1_000_000).mul(&bu(1_000_000), &p).unwrap();
        assert_eq!(sq.isqrt(&p).unwrap(), bu(1_000_000));
        assert_eq!(
            sq.checked_sub(&bu(1), &p).unwrap().isqrt(&p).unwrap(),
            bu(999_999)
        );
    }

    #[test]
    fn limits_and_cancellation_apply() {
        let token = tpt_mathr_base::CancellationToken::new();
        let p = Progress::new(ResourceLimits::DEFAULT).with_cancellation(token.clone());
        token.cancel();
        assert_eq!(
            bu(2).pow(100, &p).unwrap_err(),
            tpt_mathr_base::Error::Cancelled
        );

        let tight = Progress::new(ResourceLimits::restrictive());
        // 2^100 fits the 1024-bit budget; 2^10000 does not.
        assert!(bu(2).pow(100, &tight).is_ok());
        let err = bu(2).pow(10_000, &tight).unwrap_err();
        assert!(err.is_resource_exhausted());
    }

    #[test]
    #[allow(clippy::many_single_char_names, reason = "Bezout (g, x, y) notation")]
    fn magnitude_extended_gcd_bezout_identity() {
        // Coefficients are signed, so the identity check is signed too.
        let p = Progress::UNLIMITED;
        for (a, b) in [(12_u64, 18_u64), (18, 12), (17, 5), (1, 1), (0, 7), (7, 0)] {
            let ext = bu(a).extended_gcd(&bu(b), &p).unwrap();
            assert_eq!(ext.gcd.sign(), crate::bigint::Sign::Positive, "gcd >= 0");
            let sa = crate::bigint::BigInt::from(a);
            let sb = crate::bigint::BigInt::from(b);
            let lhs = sa
                .mul(&ext.x, &p)
                .unwrap()
                .add(&sb.mul(&ext.y, &p).unwrap(), &p)
                .unwrap();
            assert_eq!(lhs, ext.gcd, "a*x + b*y == gcd for ({a}, {b})");
        }
    }

    #[test]
    fn bigint_extended_gcd_bezout_identity() {
        let p = Progress::UNLIMITED;
        let a = BigInt::from(240_i64);
        let b = BigInt::from(46_i64);
        let ext = a.extended_gcd(&b, &p).unwrap();
        assert_eq!(ext.gcd, BigInt::from(2_i64));
        let lhs = a
            .mul(&ext.x, &p)
            .unwrap()
            .add(&b.mul(&ext.y, &p).unwrap(), &p)
            .unwrap();
        assert_eq!(lhs, ext.gcd, "a*x + b*y == gcd");
    }
}
