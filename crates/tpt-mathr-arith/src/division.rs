// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Division and remainder.
//!
//! Two paths, selected by divisor shape:
//!
//! * single-limb divisor — short division via [`DoubleLimb`];
//! * multi-limb divisor — Knuth's Algorithm D (TAOCP 4.3.1), the
//!   normalised two-word estimate with the add-back correction, following
//!   the reference formulation in *Hacker's Delight* §9-1 (`divmnu`).
//!
//! The output quotient convention is **truncated** division (round toward
//! zero in magnitude; remainder carries the dividend's sign in the signed
//! layer), matching Rust's `/` and `%` on primitives — invariant I5.
//! Every path is differentially tested against Python `int` (floored
//! division mapped appropriately) and property-checked against
//! `a == q·b + r`, `|r| < |b|`.

use alloc::vec::Vec;

use tpt_mathr_base::alloc_helpers;
use tpt_mathr_base::error::Error;

use crate::limb::{DoubleLimb, LIMB_BITS, LIMB_MAX, LIMB_ZERO, Limb, leading_zeros};
use crate::progress::Progress;

/// `u / v` for a single-limb divisor; returns `(quotient limbs, remainder)`.
/// Both slices are normalised; `v != 0`.
pub(crate) fn div_rem_small(
    u: &[Limb],
    v: crate::limb::Limb,
) -> Result<(Vec<Limb>, crate::limb::Limb), Error> {
    debug_assert!(v != 0, "divisor zero is guarded by callers");
    let mut q = alloc_helpers::try_with_capacity(u.len())?;
    q.resize(u.len(), LIMB_ZERO);
    let mut rem: crate::limb::Limb = LIMB_ZERO;
    let denom = v as DoubleLimb;
    for i in (0..u.len()).rev() {
        let cur = ((rem as DoubleLimb) << LIMB_BITS) | (u[i] as DoubleLimb);
        q[i] = (cur / denom) as crate::limb::Limb;
        rem = (cur % denom) as crate::limb::Limb;
    }
    Ok((crate::strategy::normalise_limbs(q), rem))
}

/// Full division `u / v` on normalised little-endian limbs. Returns
/// `(quotient, remainder)` normalised. `v` must be non-empty and non-zero.
pub(crate) fn div_rem(
    u: &[Limb],
    v: &[Limb],
    progress: &Progress,
) -> Result<(Vec<Limb>, Vec<Limb>), Error> {
    debug_assert!(
        !v.is_empty() && *v.last().unwrap() != LIMB_ZERO,
        "normalised divisor"
    );
    if v.len() == 1 {
        let (q, r) = div_rem_small(u, v[0])?;
        return Ok((q, if r == 0 { Vec::new() } else { alloc::vec![r] }));
    }
    if u.len() < v.len() {
        return Ok((Vec::new(), u.to_vec()));
    }
    knuth_div_rem(u, v, progress)
}

/// Knuth Algorithm D. `u.len() >= v.len() >= 2`, both normalised.
/// Single-letter names are the textbook notation (TAOCP 4.3.1: dividend
/// `u`, divisor `v`, quotient `q`, lengths `m`/`n`, shift `s`, position
/// `j`); renaming would obscure the algorithm against its reference.
#[allow(
    clippy::many_single_char_names,
    reason = "textbook Algorithm D notation"
)]
fn knuth_div_rem(
    u: &[Limb],
    v: &[Limb],
    progress: &Progress,
) -> Result<(Vec<Limb>, Vec<Limb>), Error> {
    let n = v.len();
    let m = u.len() - n;

    progress.check_memory((m as u64 + n as u64 + 2) * (LIMB_BITS as u64 / 8))?;

    // D1: normalise so the divisor's top limb has its high bit set.
    let s = leading_zeros(v[n - 1]);
    let vn = shift_up_one(v, s)?;
    // `un` gets one extra high limb for the shifted-out bit.
    let mut un = alloc_helpers::try_with_capacity::<crate::limb::Limb>(u.len() + 1)?;
    if s == 0 {
        un.extend_from_slice(u);
        un.push(LIMB_ZERO);
    } else {
        let mut carry = LIMB_ZERO;
        for &limb in u {
            un.push((limb << s) | carry);
            carry = limb >> (LIMB_BITS as u32 - s);
        }
        un.push(carry);
    }

    // D2..D7: main loop over quotient limbs, high to low.
    let mut q = alloc_helpers::try_with_capacity::<crate::limb::Limb>(m + 1)?;
    q.resize(m + 1, LIMB_ZERO);
    let denom = vn[n - 1] as DoubleLimb;
    for j in (0..=m).rev() {
        if j % 64 == 0 {
            progress.check_cancelled()?;
        }

        // D3: estimate the quotient limb.
        let num = ((un[j + n] as DoubleLimb) << LIMB_BITS) | (un[j + n - 1] as DoubleLimb);
        let mut qhat = num / denom;
        let mut rhat = num % denom;
        loop {
            let second = qhat * (vn[n - 2] as DoubleLimb)
                > ((rhat << LIMB_BITS) | (un[j + n - 2] as DoubleLimb));
            if qhat > (LIMB_MAX as DoubleLimb) || second {
                qhat -= 1;
                rhat += denom;
                if rhat > (LIMB_MAX as DoubleLimb) {
                    break;
                }
            } else {
                break;
            }
        }

        // D4: multiply and subtract.
        let mut borrow: crate::limb::SignedDouble = 0;
        for i in 0..n {
            let p = qhat * (vn[i] as DoubleLimb);
            let t = (un[i + j] as crate::limb::SignedDouble)
                - borrow
                - ((p & (LIMB_MAX as DoubleLimb)) as crate::limb::SignedDouble);
            un[i + j] = t as crate::limb::Limb;
            borrow = ((p >> LIMB_BITS) as crate::limb::SignedDouble) - (t >> LIMB_BITS);
        }
        let t = (un[j + n] as crate::limb::SignedDouble) - borrow;
        un[j + n] = t as crate::limb::Limb;

        // D5/D6: the quotient limb; add back when we subtracted too much.
        if t < 0 {
            q[j] = qhat.wrapping_sub(1) as crate::limb::Limb;
            let mut carry: DoubleLimb = 0;
            for i in 0..n {
                let sum = (un[i + j] as DoubleLimb) + (vn[i] as DoubleLimb) + carry;
                un[i + j] = sum as crate::limb::Limb;
                carry = sum >> LIMB_BITS;
            }
            un[j + n] = (un[j + n] as crate::limb::Limb).wrapping_add(carry as crate::limb::Limb);
        } else {
            q[j] = qhat as crate::limb::Limb;
        }
    }

    // D8: denormalise the remainder.
    let rem_limbs = &un[..n];
    let remainder = if s == 0 {
        crate::strategy::normalise_limbs(rem_limbs.to_vec())
    } else {
        let mut r = alloc_helpers::try_with_capacity::<crate::limb::Limb>(n)?;
        let mut carry = LIMB_ZERO;
        for i in (0..n).rev() {
            let cur = (carry << (LIMB_BITS as u32 - s)) | (rem_limbs[i] >> s);
            r.push(cur);
            carry = rem_limbs[i] & ((1 << s) - 1);
        }
        r.reverse();
        crate::strategy::normalise_limbs(r)
    };

    Ok((crate::strategy::normalise_limbs(q), remainder))
}

/// Shift a limb slice up by `s` bits, `0 <= s < LIMB_BITS`, producing a
/// slice one limb longer (or identical when `s == 0`).
fn shift_up_one(v: &[Limb], s: u32) -> Result<Vec<Limb>, Error> {
    debug_assert!((0..LIMB_BITS as u32).contains(&s));
    if s == 0 {
        let mut out = alloc_helpers::try_with_capacity(v.len())?;
        out.extend_from_slice(v);
        return Ok(out);
    }
    let mut out = alloc_helpers::try_with_capacity::<crate::limb::Limb>(v.len() + 1)?;
    let mut carry = LIMB_ZERO;
    for &limb in v {
        out.push((limb << s) | carry);
        carry = limb >> (LIMB_BITS as u32 - s);
    }
    out.push(carry);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biguint::BigUint;
    use alloc::vec;

    fn value_of(limbs: &[Limb]) -> BigUint {
        BigUint::from_limbs(limbs.to_vec())
    }

    /// `u == q*v + r` and `r < v` (I5) on the limb level.
    #[allow(clippy::many_single_char_names, reason = "Algorithm D notation")]
    fn assert_div_invariant(u: &[Limb], v: &[Limb]) {
        let p = Progress::UNLIMITED;
        let (q, r) = div_rem(u, v, &p).unwrap();
        let uv = value_of(u);
        let vv = value_of(v);
        let qv = value_of(&q);
        let rv = value_of(&r);
        assert!(rv < vv, "remainder bound violated for {uv:?} / {vv:?}");
        let product = vv.mul(&qv, &p).unwrap().add(&rv, &p).unwrap();
        assert_eq!(
            uv, product,
            "division identity violated for {uv:?} / {vv:?}"
        );
    }

    #[test]
    fn small_divisor_paths() {
        assert_div_invariant(&[], &[3]);
        assert_div_invariant(&[7], &[3]);
        assert_div_invariant(&[LIMB_MAX], &[2]);
        assert_div_invariant(&[0, 1], &[2]); // 2^64 / 2
        assert_div_invariant(&[LIMB_MAX, LIMB_MAX], &[LIMB_MAX]);
        let (q, r) = div_rem_small(&[LIMB_MAX], LIMB_MAX).unwrap();
        assert_eq!((q, r), (vec![1], 0));
    }

    #[test]
    fn smaller_dividend_returns_zero_quotient() {
        let (q, r) = div_rem(&[5, 1], &[9, 9], &Progress::UNLIMITED).unwrap();
        assert!(q.is_empty());
        assert_eq!(value_of(&r), BigUint::from_limbs(vec![5, 1]));
    }

    #[test]
    #[allow(clippy::many_single_char_names, reason = "Algorithm D notation")]
    fn knuth_known_cases() {
        let p = Progress::UNLIMITED;
        // 2^128 / 2^64 == 2^64, remainder 0.
        let u = BigUint::from(1_u128).shl(128, &p).unwrap();
        let v = BigUint::from(1_u128).shl(64, &p).unwrap();
        let (q, r) = div_rem(u.as_limbs(), v.as_limbs(), &p).unwrap();
        assert_eq!(value_of(&q), BigUint::from(1_u128).shl(64, &p).unwrap());
        assert!(r.is_empty());

        // (2^128 - 1) / (2^64 - 1): (2^64-1)(2^64+1) == 2^128-1 exactly,
        // so the quotient is 2^64+1 with a zero remainder.
        let u = BigUint::from(u128::MAX);
        let v = BigUint::from(u64::MAX);
        let (q, r) = div_rem(u.as_limbs(), v.as_limbs(), &p).unwrap();
        assert_eq!(
            value_of(&q),
            BigUint::from(1_u128)
                .shl(64, &p)
                .unwrap()
                .add(&BigUint::one(), &p)
                .unwrap()
        );
        assert!(r.is_empty(), "exact division must leave no remainder");
    }

    #[test]
    fn knuth_randomised_identity_and_bounds() {
        let mut rng = tpt_mathr_base::testing::TestRng::new(0xB0B);
        for &vlen in &[2, 3, 5, 9, 17] {
            for &ulen in &[2, 4, 8, 16, 33] {
                if ulen < vlen {
                    continue;
                }
                // Normalised random operands: nonzero top limbs.
                let u: Vec<Limb> = (0..ulen)
                    .map(|i| {
                        if i + 1 == ulen {
                            rng.below(u64::MAX).max(1) as Limb
                        } else {
                            rng.next_u64() as Limb
                        }
                    })
                    .collect();
                let v: Vec<Limb> = (0..vlen)
                    .map(|i| {
                        if i + 1 == vlen {
                            rng.below(u64::MAX).max(1) as Limb
                        } else {
                            rng.next_u64() as Limb
                        }
                    })
                    .collect();
                assert_div_invariant(&u, &v);
            }
        }
    }

    #[test]
    fn cancellation_propagates() {
        let token = tpt_mathr_base::CancellationToken::new();
        let p =
            Progress::new(tpt_mathr_base::ResourceLimits::DEFAULT).with_cancellation(token.clone());
        token.cancel();
        let u: Vec<Limb> = (0..40).map(|i| i as Limb + 1).collect();
        let v: Vec<Limb> = vec![3, 5];
        let err = div_rem(&u, &v, &p).unwrap_err();
        assert_eq!(err, tpt_mathr_base::Error::Cancelled);
    }
}
