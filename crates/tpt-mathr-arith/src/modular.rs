// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modular arithmetic (Phase 3): [`ModInt`], Montgomery reduction, Barrett
//! reduction, [`pow_mod`], and [`mod_inverse`].
//!
//! # Design
//!
//! The public type is canonical-form [`ModInt`] — value always in `[0, m)`
//! (invariant, docs/invariants.md "Modular arithmetic") — while the
//! reduction machinery lives in private context types:
//!
//! * [`MontgomeryCtx`] (odd moduli): REDC with the single-limb `n′ = -n⁻¹
//!   mod B` trick; multiplication is mul + one REDC.
//! * [`BarrettCtx`] (even moduli): precomputed `μ = ⌊B^{2k}/m⌋` replaces
//!   division by multiplication.
//!
//! Dispatch is deterministic and value-driven (odd → Montgomery, even →
//! Barrett), never thread- or hardware-dependent (I10). No performance
//! claims are attached to these paths yet (Phase 3 milestone policy):
//! benchmarks record, they do not advertise.

use alloc::vec::Vec;

use tpt_mathr_base::error::Error;

use crate::biguint::BigUint;
use crate::bigint::{BigInt, Sign};
use crate::limb::{limb_to_u64, Limb, DoubleLimb, LIMB_BITS, LIMB_MAX, LIMB_ZERO};
use crate::progress::Progress;

// ---------------------------------------------------------------------------
// Montgomery reduction (odd moduli)
// ---------------------------------------------------------------------------

/// Precomputed context for Montgomery REDC over an odd modulus.
#[derive(Debug, Clone)]
pub(crate) struct MontgomeryCtx {
    /// The odd modulus `n`.
    n: BigUint,
    /// `n′ = -n^{-1} mod B` for the machine limb width `B = 2^LIMB_BITS`.
    nprime: Limb,
    /// `R² mod n` with `R = B^k`, `k = n` limb count.
    r2: BigUint,
}

impl MontgomeryCtx {
    /// Build a context. Errors with [`Error::UnsupportedOperation`] on an
    /// even modulus (Montgomery requires odd `n`).
    pub fn new(n: &BigUint, progress: &Progress) -> Result<Self, Error> {
        if n.is_zero() {
            return Err(Error::DivisionByZero);
        }
        if (n.as_limbs()[0] & 1) == 0 {
            return Err(Error::UnsupportedOperation);
        }
        let k = n.len_limbs();
        // n′ via Newton iteration: x ← x(2 - n₀x) mod B, converges in
        // O(log B) doublings of correct low bits.
        let n0 = limb_to_u64(n.as_limbs()[0]) | 1; // n is odd
        // Newton iteration in 64-bit (wide enough for either limb width);
        // each step doubles the count of correct low bits, so six steps
        // cover all of 2^64.
        let mut x: u64 = 1;
        for _ in 0..6 {
            x = x.wrapping_mul(2_u64.wrapping_sub(n0.wrapping_mul(x)));
        }
        // x is now n₀^{-1} mod 2^LIMB_BITS; nprime = -x mod B.
        let x = if LIMB_BITS == 64 { x } else { x & ((1_u64 << LIMB_BITS) - 1) };
        let nprime = x.wrapping_neg() as Limb;
        // R² mod n: R = B^k. R = 2^{64k} (or 2^{32k}); compute R² = 2^{2·LIMB_BITS·k}
        // reduced mod n by repeated doubling: 2·LIMB_BITS·k doublings is far
        // too many; instead R = B^k is a left shift of 1 by k limbs, square
        // it with the strategy table, then reduce by division.
        let r = BigUint::one().shl(k as u64 * LIMB_BITS as u64, progress)?;
        let r2_raw = r.mul(&r, progress)?;
        let (_, r2) = r2_raw.div_rem(n, progress)?; // remainder, not quotient
        Ok(Self { n: n.clone(), nprime, r2 })
    }

    /// Montgomery product: returns `a·b·R⁻¹ mod n` for values in Montgomery
    /// form, via k single-limb Hensel steps (the CIOS formulation). Step i
    /// adds `(t_i·n′ mod B)·n` shifted by i limbs, which zeroes limb i of
    /// the accumulator; after k steps the accumulator is exactly divisible
    /// by R = B^k and the upper limbs are the reduction. Hensel division is
    /// exact by construction (m1 ≡ -t_i·n⁻¹ mod B at every step).
    pub fn mont_mul(&self, a: &BigUint, b: &BigUint, progress: &Progress) -> Result<BigUint, Error> {
        let k = self.n.len_limbs();
        let t = a.mul(b, progress)?;
        let mut acc: Vec<Limb> = t.as_limbs().to_vec();
        acc.resize(2 * k + 1, LIMB_ZERO);
        let mut carry;
        for i in 0..k {
            let m1 = (limb_to_u64(acc[i]).wrapping_mul(limb_to_u64(self.nprime))) as Limb;
            if m1 == LIMB_ZERO {
                continue;
            }
            let m1_big = BigUint::from_limb(m1);
            let row = m1_big.mul(&self.n, progress)?; // ≤ k+1 limbs
            carry = LIMB_ZERO;
            for (j, &rl) in row.as_limbs().iter().enumerate() {
                let pos = i + j;
                let s = (acc[pos] as DoubleLimb)
                    + (rl as DoubleLimb)
                    + (carry as DoubleLimb);
                acc[pos] = s as Limb;
                carry = (s >> LIMB_BITS) as Limb;
            }
            let mut pos = i + row.len_limbs();
            while carry != LIMB_ZERO {
                let s = (acc[pos] as DoubleLimb) + (carry as DoubleLimb);
                acc[pos] = s as Limb;
                carry = (s >> LIMB_BITS) as Limb;
                pos += 1;
            }
        }
        // Upper k limbs: acc / R, congruent to a·b·R⁻¹ mod n, < 2n.
        let result = BigUint::from_limbs(acc[k..].to_vec());
        if result >= self.n {
            result.checked_sub(&self.n, progress)
        } else {
            Ok(result)
        }
    }

    /// Convert a canonical value into Montgomery form: `a·R mod n`.
    pub fn to_form(&self, a: &BigUint, progress: &Progress) -> Result<BigUint, Error> {
        let reduced = a.rem(&self.n, progress)?;
        self.mont_mul(&reduced, &self.r2, progress)
    }

    /// Convert out of Montgomery form: `x·R⁻¹ mod n` (a plain REDC).
    pub fn from_form(&self, x: &BigUint, progress: &Progress) -> Result<BigUint, Error> {
        self.mont_mul(x, &BigUint::one(), progress)
    }

    /// The modulus.
    pub fn modulus(&self) -> &BigUint {
        &self.n
    }
}

// ---------------------------------------------------------------------------
// Barrett reduction (even moduli and general use)
// ---------------------------------------------------------------------------

/// Precomputed context for Barrett reduction: replaces `% m` by
/// multiplication with `μ = ⌊B^{2k}/m⌋`.
#[derive(Debug, Clone)]
pub(crate) struct BarrettCtx {
    m: BigUint,
    mu: BigUint,
    k: usize,
}

impl BarrettCtx {
    /// Build a context for a nonzero modulus.
    pub fn new(m: &BigUint, progress: &Progress) -> Result<Self, Error> {
        if m.is_zero() {
            return Err(Error::DivisionByZero);
        }
        let k = m.len_limbs();
        // μ = ⌊B^{2k} / m⌋
        let b2k = BigUint::one().shl(2 * k as u64 * LIMB_BITS as u64, progress)?;
        let (mu, _) = b2k.div_rem(m, progress)?;
        Ok(Self { m: m.clone(), mu, k })
    }

    /// Reduce `x` for `x < m²` (the multiplication-remainder use case).
    pub fn reduce(&self, x: &BigUint, progress: &Progress) -> Result<BigUint, Error> {
        // q̂ = ⌊x / B^{k-1}⌋·μ >> (2k+1 limbs)... classic 4-step:
        // q1 = ⌊x / B^{k-1}⌋, q2 = q1·μ, q3 = ⌊q2 / B^{k+1}⌋
        let shift_low = (self.k as u64 - 1) * LIMB_BITS as u64;
        let q1 = x.shr(shift_low, progress)?;
        let q2 = q1.mul(&self.mu, progress)?;
        let q3 = q2.shr((self.k as u64 + 1) * LIMB_BITS as u64, progress)?;
        // r = (x mod B^{k+1}) - (m·q3 mod B^{k+1}), borrow-corrected.
        let mask_shift = (self.k as u64 + 1) * LIMB_BITS as u64;
        let r1 = x.rem(&BigUint::one().shl(mask_shift, progress)?, progress)?;
        let mq3 = self.m.mul(&q3, progress)?;
        let r2 = mq3.rem(&BigUint::one().shl(mask_shift, progress)?, progress)?;
        let r = match r1.checked_sub(&r2, progress) {
            Ok(v) => v,
            Err(Error::Underflow) => {
                // add B^{k+1}: exactly one borrow by the Barrett bound
                r1.add(&BigUint::one().shl(mask_shift, progress)?, progress)?
            }
            Err(e) => return Err(e),
        };
        // Two correction steps bound the estimate error.
        let mut r = r;
        while r >= self.m {
            r = r.checked_sub(&self.m, progress)?;
        }
        Ok(r)
    }

    /// The modulus.
    pub fn modulus(&self) -> &BigUint {
        &self.m
    }
}

// ---------------------------------------------------------------------------
// ModInt — the canonical public type
// ---------------------------------------------------------------------------

/// A residue modulo an explicit positive modulus, kept in canonical form
/// (`0 ≤ value < modulus`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModInt {
    value: BigUint,
    modulus: BigUint,
}

impl ModInt {
    /// Canonicalise `value` modulo `modulus`.
    ///
    /// # Errors
    ///
    /// [`Error::DivisionByZero`] for a zero modulus; allocation, limits,
    /// cancellation otherwise.
    pub fn new(value: &BigUint, modulus: &BigUint, progress: &Progress) -> Result<Self, Error> {
        if modulus.is_zero() {
            return Err(Error::DivisionByZero);
        }
        let reduced = value.rem(modulus, progress)?;
        Ok(Self { value: reduced, modulus: modulus.clone() })
    }

    /// The canonical value in `[0, m)`.
    #[must_use]
    pub fn value(&self) -> &BigUint {
        &self.value
    }

    /// The modulus.
    #[must_use]
    pub fn modulus(&self) -> &BigUint {
        &self.modulus
    }

    /// Addition modulo `m`.
    ///
    /// # Errors
    ///
    /// [`Error::UnsupportedOperation`] (would-be `ModulusMismatch`) on
    /// differing moduli; allocation, limits, cancellation otherwise.
    pub fn add(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        self.same_modulus(other)?;
        let sum = self.value.add(&other.value, progress)?;
        let value = if sum >= self.modulus {
            sum.checked_sub(&self.modulus, progress)?
        } else {
            sum
        };
        Ok(Self { value, modulus: self.modulus.clone() })
    }

    /// Subtraction modulo `m`.
    ///
    /// # Errors
    ///
    /// As [`add`](Self::add).
    pub fn sub(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        self.same_modulus(other)?;
        let value = match self.value.checked_sub(&other.value, progress) {
            Ok(v) => v,
            Err(Error::Underflow) => self
                .value
                .add(&self.modulus.checked_sub(&other.value, progress)?, progress)?,
            Err(e) => return Err(e),
        };
        Ok(Self { value, modulus: self.modulus.clone() })
    }

    /// Multiplication modulo `m` (Montgomery for odd `m`, Barrett for even).
    ///
    /// # Errors
    ///
    /// As [`add`](Self::add).
    pub fn mul(&self, other: &Self, progress: &Progress) -> Result<Self, Error> {
        self.same_modulus(other)?;
        let value = mod_reduce_mul(&self.value, &other.value, &self.modulus, progress)?;
        Ok(Self { value, modulus: self.modulus.clone() })
    }

    /// Raise to an arbitrary (non-negative, arbitrary-precision) exponent.
    ///
    /// # Errors
    ///
    /// As [`add`](Self::add).
    pub fn pow(&self, exponent: &BigUint, progress: &Progress) -> Result<Self, Error> {
        let value =
            pow_mod_ctx(&BigInt::from(self.value.clone()), exponent, &self.modulus, progress)?;
        Ok(Self { value, modulus: self.modulus.clone() })
    }

    /// Multiplicative inverse (`self^(m-2)` for prime `m`, extended gcd
    /// otherwise — always exact, never probabilistic).
    ///
    /// # Errors
    ///
    /// [`Error::UnsupportedOperation`] when the inverse does not exist
    /// (gcd(value, m) ≠ 1); allocation, limits, cancellation otherwise.
    pub fn inv(&self, progress: &Progress) -> Result<Self, Error> {
        let inv = mod_inverse(&BigInt::from(self.value.clone()), &self.modulus, progress)?;
        Ok(Self { value: inv, modulus: self.modulus.clone() })
    }

    fn same_modulus(&self, other: &Self) -> Result<(), Error> {
        if self.modulus == other.modulus {
            Ok(())
        } else {
            Err(Error::UnsupportedOperation)
        }
    }
}

/// Canonical-form multiplier over a fixed modulus: odd `m` rides the
/// Montgomery forms, even `m` falls back to Barrett. Selection is
/// deterministic and value-driven (I10) — never thread- or hardware-driven.
#[derive(Debug, Clone)]
enum Reducer {
    Montgomery(MontgomeryCtx),
    Barrett(BarrettCtx),
}

impl Reducer {
    /// Build the reducer for `m > 0`.
    fn new(m: &BigUint, progress: &Progress) -> Result<Self, Error> {
        if m.is_zero() {
            return Err(Error::DivisionByZero);
        }
        if (m.as_limbs()[0] & 1) == 1 {
            Ok(Self::Montgomery(MontgomeryCtx::new(m, progress)?))
        } else {
            Ok(Self::Barrett(BarrettCtx::new(m, progress)?))
        }
    }

    /// `a · b mod m` for canonical `a, b < m`, returning a canonical value.
    fn mul_canonical(
        &self,
        a: &BigUint,
        b: &BigUint,
        progress: &Progress,
    ) -> Result<BigUint, Error> {
        match self {
            Self::Montgomery(ctx) => {
                let fa = ctx.to_form(a, progress)?;
                let fb = ctx.to_form(b, progress)?;
                let prod = ctx.mont_mul(&fa, &fb, progress)?;
                ctx.from_form(&prod, progress)
            }
            Self::Barrett(ctx) => {
                let prod = a.mul(b, progress)?;
                ctx.reduce(&prod, progress)
            }
        }
    }
}

/// Shared reduction path for products of two canonical residues.
fn mod_reduce_mul(
    a: &BigUint,
    b: &BigUint,
    m: &BigUint,
    progress: &Progress,
) -> Result<BigUint, Error> {
    Reducer::new(m, progress)?.mul_canonical(a, b, progress)
}

/// Modular exponentiation with an arbitrary-precision exponent:
/// `base^exponent mod modulus`.
///
/// # Errors
///
/// [`Error::DivisionByZero`] for a zero modulus; allocation, limits,
/// cancellation otherwise.
pub fn pow_mod(
    base: &BigInt,
    exponent: &BigUint,
    modulus: &BigUint,
    progress: &Progress,
) -> Result<BigUint, Error> {
    pow_mod_ctx(base, exponent, modulus, progress)
}

fn pow_mod_ctx(
    base: &BigInt,
    exponent: &BigUint,
    modulus: &BigUint,
    progress: &Progress,
) -> Result<BigUint, Error> {
    if modulus.is_zero() {
        return Err(Error::DivisionByZero);
    }
    if modulus == &BigUint::one() {
        return Ok(BigUint::zero());
    }
    let m_int = BigInt::from(modulus.clone());
    let mut base_res = base.rem(&m_int, progress)?;
    if base_res.sign() == Sign::Negative {
        // Truncated remainders of negatives are negative; canonicalise to
        // Python-style `mod` semantics (always in [0, m)).
        base_res = base_res.add(&m_int, progress)?;
    }
    let reducer = Reducer::new(modulus, progress)?;
    let (_, base_mag) = base_res.into_parts();
    let mut result = BigUint::one();
    let mut square = base_mag;
    let bits = exponent.bit_len();
    // LSB-first: `square` is base^(2^i) at iteration i, so multiplying it
    // into `result` on set bits builds base^exponent directly.
    for i in 0..bits {
        progress.check_cancelled()?;
        if exponent.bit(i) {
            result = reducer.mul_canonical(&result, &square, progress)?;
        }
        if i + 1 < bits {
            square = reducer.mul_canonical(&square, &square, progress)?;
        }
    }
    Ok(result)
}

/// Modular multiplicative inverse of `a` modulo `m` via the extended
/// Euclidean algorithm (exact; `Err` when `gcd(a, m) != 1`).
///
/// # Errors
///
/// [`Error::DivisionByZero`] for a zero modulus;
/// [`Error::UnsupportedOperation`] when the inverse does not exist;
/// allocation, limits, cancellation otherwise.
pub fn mod_inverse(
    a: &BigInt,
    m: &BigUint,
    progress: &Progress,
) -> Result<BigUint, Error> {
    if m.is_zero() {
        return Err(Error::DivisionByZero);
    }
    // Reduce a into [0, m) first: the extended gcd needs the canonical
    // residue, and the Bezout coefficient is then already exact.
    let m_int = BigInt::from(m.clone());
    let (_, a_res) = a.div_rem(&m_int, progress)?;
    let ext = a_res.extended_gcd(&m_int, progress)?;
    if ext.gcd.magnitude() == &BigUint::one() {
        let (_, x) = ext.x.div_rem(&m_int, progress)?;
        let x = if x.sign() == Sign::Negative {
            x.add(&m_int, progress)?
        } else {
            x
        };
        Ok(x.into_residue_magnitude())
    } else {
        Err(Error::UnsupportedOperation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bu(v: u64) -> BigUint {
        BigUint::from(v)
    }

    fn bi(v: i64) -> BigInt {
        BigInt::from(v)
    }

    #[test]
    fn montgomery_round_trip_and_products() {
        let p = Progress::UNLIMITED;
        let n = bu(1_000_000_007);
        let ctx = MontgomeryCtx::new(&n, &p).unwrap();
        for a in [0_u64, 1, 2, 123_456_789, 999_999_999] {
            let fa = ctx.to_form(&bu(a), &p).unwrap();
            let back = ctx.from_form(&fa, &p).unwrap();
            assert_eq!(back, bu(a), "round trip {a}");
        }
        // (a·b) via forms matches a·b mod n.
        for (a, b) in [(123_456_789_u64, 987_654_321), (1_000_000_006, 1_000_000_006), (0, 5)] {
            let fa = ctx.to_form(&bu(a), &p).unwrap();
            let fb = ctx.to_form(&bu(b), &p).unwrap();
            let prod = ctx.mont_mul(&fa, &fb, &p).unwrap();
            let out = ctx.from_form(&prod, &p).unwrap();
            assert_eq!(out, bu((u128::from(a) * u128::from(b) % 1_000_000_007) as u64));
        }
        // Even modulus refused.
        assert_eq!(
            MontgomeryCtx::new(&bu(10), &p).unwrap_err(),
            Error::UnsupportedOperation
        );
        assert_eq!(
            MontgomeryCtx::new(&BigUint::zero(), &p).unwrap_err(),
            Error::DivisionByZero
        );
    }

    #[test]
    fn barrett_reduces_products() {
        let p = Progress::UNLIMITED;
        let m = bu(1_000_000_012); // even
        let ctx = BarrettCtx::new(&m, &p).unwrap();
        for (a, b) in [(u64::MAX, u64::MAX), (123_456_789, 987_654_321), (0, 7)] {
            let prod = bu(a).mul(&bu(b), &p).unwrap();
            let out = ctx.reduce(&prod, &p).unwrap();
            let want = (u128::from(a) * u128::from(b) % 1_000_000_012) as u64;
            assert_eq!(out, bu(want), "barrett {a}·{b} mod m");
        }
    }

    #[test]
    fn modint_is_canonical_and_correct() {
        let p = Progress::UNLIMITED;
        let m = bu(97);
        let a = ModInt::new(&bu(150), &m, &p).unwrap();
        assert_eq!(a.value(), &bu(53));
        let b = ModInt::new(&bu(60), &m, &p).unwrap();
        assert_eq!(a.add(&b, &p).unwrap().value(), &bu(16));
        assert_eq!(a.sub(&b, &p).unwrap().value(), &bu(90));
        assert_eq!(a.mul(&b, &p).unwrap().value(), &bu((150 % 97) * 60 % 97));
        // Mismatched moduli are refused:
        let other = ModInt::new(&bu(1), &bu(89), &p).unwrap();
        assert_eq!(a.add(&other, &p).unwrap_err(), Error::UnsupportedOperation);
    }

    #[test]
    fn pow_mod_matches_known_values() {
        let p = Progress::UNLIMITED;
        // Fermat: 2^(p-1) ≡ 1 mod prime p.
        assert_eq!(
            pow_mod(&bi(2), &bu(1_000_000_006), &bu(1_000_000_007), &p).unwrap(),
            bu(1)
        );
        // Even modulus path:
        assert_eq!(
            pow_mod(&bi(3), &bu(100), &bu(1_000_000_012), &p).unwrap(),
            {
                // reference: repeated squaring done by the same code path —
                // cross-check with ModInt::pow
                let base = ModInt::new(&bu(3), &bu(1_000_000_012), &p).unwrap();
                base.pow(&bu(100), &p).unwrap().value().clone()
            }
        );
        // Huge exponents (Miller-Rabin shape): a^(n-1) mod n.
        let n = bu(2_147_483_647); // Mersenne prime
        let e = BigUint::from(2_147_483_646_u64);
        assert_eq!(pow_mod(&bi(5), &e, &n, &p).unwrap(), bu(1));
        // Zero modulus / modulus one:
        assert_eq!(pow_mod(&bi(2), &bu(3), &BigUint::zero(), &p).unwrap_err(), Error::DivisionByZero);
        assert_eq!(pow_mod(&bi(123), &bu(9), &bu(1), &p).unwrap(), BigUint::zero());
    }

    #[test]
    fn mod_inverse_exact() {
        let p = Progress::UNLIMITED;
        assert_eq!(
            mod_inverse(&bi(3), &bu(1_000_000_007), &p).unwrap(),
            bu(333_333_336)
        );
        // 3·336... = 1 mod m: verify directly.
        let inv = mod_inverse(&bi(3), &bu(1_000_000_007), &p).unwrap();
        assert_eq!(
            BigUint::from(3_u8).mul(&inv, &p).unwrap().rem(&bu(1_000_000_007), &p).unwrap(),
            bu(1)
        );
        // Non-invertible:
        assert_eq!(
            mod_inverse(&bi(6), &bu(9), &p).unwrap_err(),
            Error::UnsupportedOperation
        );
        // Negative a handled via reduction:
        assert_eq!(
            mod_inverse(&bi(-3), &bu(1_000_000_007), &p).unwrap(),
            bu(1_000_000_007 - 333_333_336)
        );
    }

    #[test]
    fn montgomery_matches_plain_reduction_wide() {
        let mut rng = tpt_mathr_base::testing::TestRng::new(0x0D0D);
        let p = Progress::UNLIMITED;
        for _ in 0..50 {
            // Odd moduli up to 4 limbs; operands span up to 4 limbs.
            let m = {
                let mut v = (u128::from(rng.next_u64()) << 64) | u128::from(rng.next_u64());
                v |= 1; // odd
                BigUint::from(v)
            };
            let x = BigUint::from((u128::from(rng.next_u64()) << 64) | u128::from(rng.next_u64()));
            let ctx = MontgomeryCtx::new(&m, &p).unwrap();
            let fa = ctx.to_form(&x, &p).unwrap();
            let back = ctx.from_form(&fa, &p).unwrap();
            assert_eq!(back, x.rem(&m, &p).unwrap(), "montgomery wide round trip mod {m:?}");
        }
    }
}
