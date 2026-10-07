// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Multiplication architecture (spec §7): strategy-based, thresholds to be
//! benchmark-derived, no hard-coded universal winner.
//!
//! [`MulStrategy`] is an internal abstraction over limb-slice
//! multiplication. [`select_strategy`] picks an implementation by operand
//! size; the current constants are *provisional engineering defaults* and
//! are explicitly scheduled for replacement by measured thresholds
//! (Phase 3, `docs/benchmarks.md`). The strategy names are recorded in
//! benchmark output via [`MulStrategy::name`].

use alloc::{vec, vec::Vec};

use tpt_mathr_base::alloc_helpers;
use tpt_mathr_base::error::Error;

use crate::limb::{DoubleLimb, LIMB_BITS, LIMB_ZERO, Limb, SignedDouble, widening_mul};
use crate::bigint::{BigInt, Sign};
use crate::biguint::BigUint;
use crate::progress::Progress;

/// Operand length (in limbs) at which Karatsuba is preferred over
/// schoolbook. Provisional; benchmark-derived thresholds replace this in
/// Phase 3 (no hard-coded universal winner — spec §7).
pub(crate) const KARATSUBA_THRESHOLD: usize = 32;

/// Operand length at which Toom-Cook-3 is preferred over Karatsuba.
/// Provisional pending the benchmark matrix (spec §7).
pub(crate) const TOOM3_THRESHOLD: usize = 256;

/// Operand length at which the NTT path is preferred over Toom-Cook-3.
/// Provisional pending the benchmark matrix (spec §7).
pub(crate) const NTT_THRESHOLD: usize = 4096;

/// Internal limb-slice multiplication strategy.
///
/// `name` feeds the benchmark records (Phase 3, docs/benchmarks.md); it is
/// unused until the benchmark harness wires strategy reporting, hence the
/// dead-code allowance.
#[allow(
    dead_code,
    reason = "name() is consumed by benchmark records from Phase 3"
)]
pub(crate) trait MulStrategy {
    /// Stable identifier used in benchmark records and debug output.
    fn name(&self) -> &'static str;

    /// Full product of two *normalised* little-endian limb slices,
    /// returned normalised. Implementations may assume both slices are
    /// non-empty and free of trailing zero limbs.
    ///
    /// # Errors
    ///
    /// Allocation failure, or the `progress` budget errors.
    fn multiply(&self, lhs: &[Limb], rhs: &[Limb], progress: &Progress)
    -> Result<Vec<Limb>, Error>;
}

/// Choose a strategy for the given operand lengths (thresholds provisional
/// pending the Phase 3 benchmark matrix).
pub(crate) fn select_strategy(a_len: usize, b_len: usize) -> &'static dyn MulStrategy {
    let small = a_len.min(b_len);
    if small >= NTT_THRESHOLD {
        &crate::ntt::Ntt
    } else if small >= TOOM3_THRESHOLD {
        &Toom3
    } else if small >= KARATSUBA_THRESHOLD {
        &Karatsuba
    } else {
        &Schoolbook
    }
}

/// Classical O(n·m) schoolbook multiplication.
pub(crate) struct Schoolbook;

impl MulStrategy for Schoolbook {
    fn name(&self) -> &'static str {
        "schoolbook"
    }

    fn multiply(
        &self,
        lhs: &[Limb],
        rhs: &[Limb],
        _progress: &Progress,
    ) -> Result<Vec<Limb>, Error> {
        let mut out = alloc_helpers::try_with_capacity(lhs.len() + rhs.len())?;
        out.resize(lhs.len() + rhs.len(), LIMB_ZERO);
        for (i, &a) in lhs.iter().enumerate() {
            if a == LIMB_ZERO {
                continue;
            }
            let mut carry: DoubleLimb = 0;
            for (j, &b) in rhs.iter().enumerate() {
                // Fits: 2 products + 2 carries < 2^(2·LIMB_BITS).
                let t = widening_mul(a, b) + (out[i + j] as DoubleLimb) + carry;
                out[i + j] = t as Limb;
                carry = t >> LIMB_BITS;
            }
            out[i + rhs.len()] = carry as Limb;
        }
        Ok(normalise_limbs(out))
    }
}

/// Karatsuba: three half-size products instead of four, O(n^1.585).
pub(crate) struct Karatsuba;

impl MulStrategy for Karatsuba {
    fn name(&self) -> &'static str {
        "karatsuba"
    }

    fn multiply(
        &self,
        lhs: &[Limb],
        rhs: &[Limb],
        progress: &Progress,
    ) -> Result<Vec<Limb>, Error> {
        // Below threshold recurse to schoolbook (also the base case). A
        // size imbalance beyond 2x defeats Karatsuba's splitting: the long
        // side's low+high sum would recreate the original problem size and
        // recurse forever, so fall back to schoolbook instead.
        if lhs.len().min(rhs.len()) < KARATSUBA_THRESHOLD
            || lhs.len().max(rhs.len()) > 2 * lhs.len().min(rhs.len())
        {
            return Schoolbook.multiply(lhs, rhs, progress);
        }
        let m = lhs.len().max(rhs.len()).div_ceil(2);
        let (a_lo, a_hi) = split_at(lhs, m);
        let (b_lo, b_hi) = split_at(rhs, m);

        progress.check_cancelled()?;

        let z0 = self.multiply(a_lo, b_lo, progress)?;
        let z2 = self.multiply(a_hi, b_hi, progress)?;
        let sa = add_limbs(a_lo, a_hi)?;
        let sb = add_limbs(b_lo, b_hi)?;
        let z1_full = self.multiply(&sa, &sb, progress)?;
        let z1 = sub_limbs(&sub_limbs(&z1_full, &z0)?, &z2)?;

        // result = z0 + z1·B^m + z2·B^(2m)
        let out_len = lhs.len() + rhs.len();
        let mut out = alloc_helpers::try_with_capacity::<Limb>(out_len + 1)?;
        out.resize(out_len, LIMB_ZERO);
        add_shifted(&mut out, &z0, 0);
        add_shifted(&mut out, &z1, m);
        add_shifted(&mut out, &z2, 2 * m);
        Ok(normalise_limbs(out))
    }
}

/// Toom-Cook-3: five half-size products instead of Karatsuba's three,
/// O(n^1.465). Evaluation points 0, 1, -1, 2, ∞; interpolation divides by
/// small constants only (2 and 3), all exact (Bodrato's formulation).
pub(crate) struct Toom3;

impl MulStrategy for Toom3 {
    fn name(&self) -> &'static str {
        "toom3"
    }

    fn multiply(
        &self,
        lhs: &[Limb],
        rhs: &[Limb],
        progress: &Progress,
    ) -> Result<Vec<Limb>, Error> {
        // Imbalance guard (same rationale as Karatsuba): at max == 3·min
        // the split leaves a1/a2 empty and the v(1) sub-product recreates
        // the original problem size, recursing forever. 3:1 and beyond is
        // delegated to Karatsuba (which itself falls back to schoolbook);
        // rebalancing unbalanced multiplies is a recorded Phase 3 TODO.
        if lhs.len().min(rhs.len()) < TOOM3_THRESHOLD
            || lhs.len().max(rhs.len()) > 2 * lhs.len().min(rhs.len())
        {
            return Karatsuba.multiply(lhs, rhs, progress);
        }
        let m = lhs.len().max(rhs.len()).div_ceil(3);
        let (a0, rest) = split_at(lhs, m);
        let (a1, a2) = split_at(rest, m);
        let (b0, rest) = split_at(rhs, m);
        let (b1, b2) = split_at(rest, m);

        progress.check_cancelled()?;

        // All evaluations run through BigInt so the -1 point and the
        // interpolation share one signed code path.
        let g = |s: &[Limb]| BigInt::from(BigUint::from_limbs(s.to_vec()));

        // v(0) and v(inf) use unsigned recursive dispatch.
        let v0 = BigInt::from(BigUint::from_limbs(self.multiply(a0, b0, progress)?));
        let vinf = BigInt::from(BigUint::from_limbs(self.multiply(a2, b2, progress)?));

        // v(1) = (a0+a1+a2)(b0+b1+b2)
        let v1 = g(a0)
            .add(&g(a1), progress)?
            .add(&g(a2), progress)?
            .mul(&g(b0).add(&g(b1), progress)?.add(&g(b2), progress)?, progress)?;
        // v(2) = (a0+2a1+4a2)(b0+2b1+4b2)
        let two = BigInt::from(2_i64);
        let four = BigInt::from(4_i64);
        let v2 = g(a0)
            .add(&g(a1).mul(&two, progress)?, progress)?
            .add(&g(a2).mul(&four, progress)?, progress)?
            .mul(
                &g(b0)
                    .add(&g(b1).mul(&two, progress)?, progress)?
                    .add(&g(b2).mul(&four, progress)?, progress)?,
                progress,
            )?;
        // v(-1) = (a0-a1+a2)(b0-b1+b2); checked_sub on BigInt is signed.
        let v_neg1 = g(a0)
            .checked_sub(&g(a1), progress)?
            .add(&g(a2), progress)?
            .mul(
                &g(b0)
                    .checked_sub(&g(b1), progress)?
                    .add(&g(b2), progress)?,
                progress,
            )?;

        // Interpolation (Bodrato; divisions by 2 and 3 are exact):
        //   A = v1-v0-vinf = c1+c2+c3
        //   B = v-1-v0-vinf = -c1+c2-c3
        //   c2 = (A+B)/2,  D = (A-B)/2 = c1+c3
        //   C = v2-v0-16vinf = 2c1+4c2+8c3
        //   E = (C-2(A+B))/2 = c1+4c3
        //   c3 = (E-D)/3,  c1 = D-c3
        let eval_a = v1.checked_sub(&v0, progress)?.checked_sub(&vinf, progress)?;
        let eval_b = v_neg1.checked_sub(&v0, progress)?.checked_sub(&vinf, progress)?;
        let c2 = eval_a.add(&eval_b, progress)?.div_small_exact(2, progress)?;
        let d = eval_a
            .checked_sub(&eval_b, progress)?
            .div_small_exact(2, progress)?;
        let sixteen = BigInt::from(16_i64);
        let c_big = v2
            .checked_sub(&v0, progress)?
            .checked_sub(&vinf.mul(&sixteen, progress)?, progress)?;
        let e = c_big
            .checked_sub(&eval_a.add(&eval_b, progress)?.mul(&two, progress)?, progress)?
            .div_small_exact(2, progress)?;
        let c3 = e.checked_sub(&d, progress)?.div_small_exact(3, progress)?;
        let c1 = d.checked_sub(&c3, progress)?;

        // Recombine at x = B^m: c0 + c1x + c2x^2 + c3x^3 + c4x^4.
        let x = BigInt::from(BigUint::one().shl(m as u64 * LIMB_BITS as u64, progress)?);
        let mut acc = v0;
        let mut place = x.clone();
        for coef in [&c1, &c2, &c3] {
            acc = acc.add(&coef.mul(&place, progress)?, progress)?;
            place = place.mul(&x, progress)?;
        }
        acc = acc.add(&vinf.mul(&place, progress)?, progress)?;
        debug_assert!(
            acc.sign() != Sign::Negative,
            "toom3 recombination must be non-negative"
        );
        Ok(acc.magnitude().as_limbs().to_vec())
    }
}

/// Split at limb index `m` (empty high part when the slice is shorter).
fn split_at(v: &[Limb], m: usize) -> (&[Limb], &[Limb]) {
    if m >= v.len() {
        (v, &[])
    } else {
        v.split_at(m)
    }
}

/// Limb-slice addition (internal; fallible allocation only).
pub(crate) fn add_limbs(a: &[Limb], b: &[Limb]) -> Result<Vec<Limb>, Error> {
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let mut out = alloc_helpers::try_with_capacity(long.len() + 1)?;
    let mut carry = LIMB_ZERO;
    // Index-based: same-position indexing with a tail default.
    #[allow(clippy::needless_range_loop)]
    for i in 0..long.len() {
        let s = (long[i] as DoubleLimb)
            + (short.get(i).copied().unwrap_or(LIMB_ZERO) as DoubleLimb)
            + (carry as DoubleLimb);
        out.push(s as Limb);
        carry = (s >> LIMB_BITS) as Limb;
    }
    if carry != 0 {
        out.push(carry);
    }
    Ok(out)
}

/// Limb-slice subtraction `a - b`; caller guarantees `a >= b`.
pub(crate) fn sub_limbs(a: &[Limb], b: &[Limb]) -> Result<Vec<Limb>, Error> {
    debug_assert!(a.len() >= b.len(), "sub_limbs precondition");
    let mut out = alloc_helpers::try_with_capacity(a.len())?;
    let mut borrow: SignedDouble = 0;
    for (i, &x) in a.iter().enumerate() {
        let y = b.get(i).copied().unwrap_or(LIMB_ZERO) as SignedDouble;
        let d = (x as SignedDouble) - y - borrow;
        out.push(d as Limb);
        borrow = SignedDouble::from(d < 0);
    }
    debug_assert_eq!(borrow, 0, "sub_limbs requires a >= b");
    Ok(normalise_limbs(out))
}

/// `out += add` aligned at `offset` limbs (out must be long enough).
fn add_shifted(out: &mut [Limb], add: &[Limb], offset: usize) {
    let mut carry = LIMB_ZERO;
    for (i, &x) in add.iter().enumerate() {
        let pos = offset + i;
        let s = (out[pos] as DoubleLimb) + (x as DoubleLimb) + (carry as DoubleLimb);
        out[pos] = s as Limb;
        carry = (s >> LIMB_BITS) as Limb;
    }
    let mut pos = offset + add.len();
    while carry != LIMB_ZERO {
        let s = (out[pos] as DoubleLimb) + (carry as DoubleLimb);
        out[pos] = s as Limb;
        carry = (s >> LIMB_BITS) as Limb;
        pos += 1;
    }
}

/// Trim trailing zero limbs (I1).
pub(crate) fn normalise_limbs(mut limbs: Vec<Limb>) -> Vec<Limb> {
    while limbs.last() == Some(&LIMB_ZERO) {
        limbs.pop();
    }
    limbs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biguint::BigUint;

    fn limbs_of(v: u64) -> Vec<Limb> {
        BigUint::from(v).as_limbs().to_vec()
    }

    fn value_of(limbs: &[Limb]) -> BigUint {
        BigUint::from_limbs(limbs.to_vec())
    }

    #[test]
    fn schoolbook_small() {
        let p = Progress::UNLIMITED;
        let out = Schoolbook
            .multiply(&limbs_of(123), &limbs_of(456), &p)
            .unwrap();
        assert_eq!(value_of(&out), BigUint::from(123_u64 * 456));
        // Zero operands never reach strategies via BigUint::mul, but the
        // slices-level contract (normalised in/out) still holds:
        let out = Schoolbook.multiply(&[], &limbs_of(9), &p).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn toom3_single_case_isolation() {
        let p = Progress::UNLIMITED;
        let mut rng = tpt_mathr_base::testing::TestRng::new(0x1234);
        let a: Vec<Limb> = (0..256).map(|_| rng.next_u64()).collect();
        let b: Vec<Limb> = (0..256).map(|_| rng.next_u64()).collect();
        let got = Toom3.multiply(&a, &b, &p).unwrap();
        let want = Schoolbook.multiply(&a, &b, &p).unwrap();
        assert_eq!(value_of(&got), value_of(&want));
    }

    #[test]
    fn toom3_agrees_across_many_shapes() {
        let mut rng = tpt_mathr_base::testing::TestRng::new(0x007003);
        let p = Progress::UNLIMITED;
        for len_a in [255_usize, 256, 257, 384, 511, 512, 513] {
            for len_b in [1_usize, 3, 128, 255, 256, 257, 384, 512, 513] {
                let a: Vec<Limb> = (0..len_a).map(|_| rng.next_u64()).collect();
                let b: Vec<Limb> = (0..len_b).map(|_| rng.next_u64()).collect();
                let via_toom3 = value_of(&Toom3.multiply(&a, &b, &p).unwrap());
                let via_ref = value_of(&Schoolbook.multiply(&a, &b, &p).unwrap());
                assert_eq!(
                    via_toom3, via_ref,
                    "toom3 mismatch at {len_a}x{len_b} limbs
A={a:?}
B={b:?}"
                );
            }
        }
    }

    #[test]
    fn strategies_agree_across_sizes() {
        let mut rng = tpt_mathr_base::testing::TestRng::new(0xA11CE);
        let p = Progress::UNLIMITED;
        for len_a in [1, 2, 3, 31, 32, 33, 64, 65, 100] {
            for len_b in [1, 31, 32, 33, 80] {
                let a: Vec<Limb> = (0..len_a)
                    .map(|_| rng.below(u64::MAX).max(1) as Limb)
                    .collect();
                let b: Vec<Limb> = (0..len_b)
                    .map(|_| rng.below(u64::MAX).max(1) as Limb)
                    .collect();
                let via_schoolbook = value_of(&Schoolbook.multiply(&a, &b, &p).unwrap());
                let via_karatsuba = value_of(&Karatsuba.multiply(&a, &b, &p).unwrap());
                assert_eq!(
                    via_schoolbook, via_karatsuba,
                    "strategy disagreement at {len_a}x{len_b} limbs"
                );
                let via_select =
                    value_of(&select_strategy(len_a, len_b).multiply(&a, &b, &p).unwrap());
                assert_eq!(via_select, via_schoolbook);
            }
        }
    }

    #[test]
    fn karatsuba_matches_schoolbook_on_known_big_product() {
        let p = Progress::UNLIMITED;
        // (2^64)^2 = 2^128: cross-multiplication boundary case.
        let two_64 = BigUint::from(1_u128 << 64);
        let sq = Karatsuba
            .multiply(two_64.as_limbs(), two_64.as_limbs(), &p)
            .unwrap();
        assert_eq!(
            value_of(&sq),
            BigUint::from(1_u128 << 127)
                .add(&BigUint::from(1_u128 << 127), &p)
                .unwrap()
        );
    }
}
