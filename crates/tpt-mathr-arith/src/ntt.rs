// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! NTT-based multiplication (Phase 3): the number-theoretic transform over
//! three NTT-friendly primes with CRT reconstruction.
//!
//! # Scheme
//!
//! Each u64 limb is split into two 32-bit halves; the operand becomes a
//! little-endian vector of 32-bit digits. The convolution of those digit
//! vectors is evaluated exactly over three primes
//! (`998244353`, `754974721`, `167772161` — each with a large power of two
//! dividing `p−1`), pointwise-multiplied, inverted, and reconstructed by
//! CRT into `u128` coefficients, then carry-propagated back into limbs.
//!
//! # Exactness envelope (documented limit)
//!
//! A convolution coefficient is at most `n·(2³²−1)² < n·2⁶⁴` for `n` digit
//! pairs; CRT over the three primes is exact while that stays below
//! `p₁·p₂·p₃ ≈ 3.35·10²⁶`, i.e. `n < 1.8·10⁷` digits. The transform length
//! is additionally capped at `2²³` points (the smallest prime's root
//! order), i.e. operands up to 4·2²³/2 = ~33 MiB. Beyond that the dispatch
//! layer falls back to Toom-Cook — the cap is part of the strategy
//! contract, never a silent wraparound.
//!
//! No performance claims are attached until measured (Phase 3 milestone
//! policy); thresholds are provisional.

use alloc::{vec, vec::Vec};

use tpt_mathr_base::error::Error;

use crate::limb::{Limb, LIMB_BITS};
use crate::strategy::{MulStrategy, NTT_THRESHOLD, Schoolbook};
use crate::progress::Progress;

/// The three NTT primes and their primitive 2-power roots.
const NTT_PRIMES: [(u64, u64); 3] = [
    (998_244_353, 3),      // 119·2²³ + 1, root 3
    (754_974_721, 11),     // 45·2²⁴ + 1, root 11
    (167_772_161, 3),      // 5·2²⁵ + 1, root 3
];

/// Maximum transform length (points) per prime's root order.
const MAX_LEN: usize = 1 << 23;

/// Modular multiplication mod a ≤ 2⁶² prime fits `u128`.
#[inline]
const fn mul_mod(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128) * (b as u128) % (m as u128)) as u64
}

/// Modular exponentiation.
fn pow_mod_small(base: u64, exp: u64, m: u64) -> u64 {
    let mut result = 1_u64;
    let mut b = base % m;
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            result = mul_mod(result, b, m);
        }
        b = mul_mod(b, b, m);
        e >>= 1;
    }
    result
}

/// In-place iterative Cooley–Tukey NTT. `len` must be a power of two
/// dividing the prime's root order; `invert` applies the inverse transform
/// (including the `len⁻¹` scaling).
fn ntt_in_place(values: &mut [u64], root: u64, m: u64, invert: bool) {
    let len = values.len();
    // Bit-reversal permutation.
    let mut j = 0_usize;
    for i in 1..len {
        let mut bit = len >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            values.swap(i, j);
        }
    }
    // Butterflies. `root` must be a primitive `len`-th root of unity; the
    // block step is root^(len/span).
    let mut span = 2_usize;
    while span <= len {
        let half = span / 2;
        let w_len = pow_mod_small(root, (len / span) as u64, m);
        for start in (0..len).step_by(span) {
            let mut w = 1_u64;
            for k in 0..half {
                let u = values[start + k];
                let v = mul_mod(values[start + k + half], w, m);
                values[start + k] = (u + v) % m;
                values[start + k + half] = (u + m - v) % m;
                w = mul_mod(w, w_len, m);
            }
        }
        span *= 2;
    }
    if invert {
        // Scale by len⁻¹ and use the inverse root (caller passes it in).
        let len_inv = pow_mod_small(len as u64, m - 2, m);
        for v in values.iter_mut() {
            *v = mul_mod(*v, len_inv, m);
        }
    }
}

/// NTT strategy: convolves 32-bit half-limbs over three primes.
pub(crate) struct Ntt;

impl MulStrategy for Ntt {
    fn name(&self) -> &'static str {
        "ntt"
    }

    fn multiply(
        &self,
        lhs: &[Limb],
        rhs: &[Limb],
        progress: &Progress,
    ) -> Result<Vec<Limb>, Error> {
        // Below the dispatch threshold, hand back to Toom-3 (also the
        // safety net for odd shapes).
        if lhs.len().min(rhs.len()) < crate::strategy::NTT_THRESHOLD {
            return crate::strategy::Toom3.multiply(lhs, rhs, progress);
        }

        // Split into 32-bit half digits.
        let to_digits32 = |v: &[Limb]| -> Vec<u32> {
            let mut out = Vec::with_capacity(v.len() * 2);
            for &l in v {
                out.push(l as u32);
                out.push((l >> (LIMB_BITS / 2)) as u32);
            }
            out
        };
        let a = to_digits32(lhs);
        let b = to_digits32(rhs);

        // Transform length: next power of two ≥ a.len + b.len.
        let need = a.len() + b.len();
        let mut conv_len = 1_usize;
        while conv_len < need {
            conv_len *= 2;
        }
        // Documented envelope (module docs): beyond MAX_LEN the caller must
        // not select NTT; guard anyway.
        if conv_len > MAX_LEN {
            return crate::strategy::Toom3.multiply(lhs, rhs, progress);
        }

        progress.check_memory((conv_len as u64) * 8)?;

        let digits = self.convolve(&a, &b, conv_len, progress)?;

        // Carry-propagate the base-2³² convolution into base-2^64 limbs.
        let mut out: Vec<Limb> = Vec::with_capacity(conv_len / 2 + 2);
        let mut carry: u128 = 0;
        for i in (0..conv_len).step_by(2) {
            let v = digits[i] + (digits.get(i + 1).copied().unwrap_or(0) << 32) + carry;
            out.push(v as Limb);
            carry = v >> LIMB_BITS;
        }
        while carry != 0 {
            out.push(carry as Limb);
            carry >>= LIMB_BITS;
        }
        Ok(crate::strategy::normalise_limbs(out))
    }
}

impl Ntt {
    /// Exact base-2³² convolution of two digit vectors at transform length
    /// `conv_len` (must be a power of two ≥ a.len + b.len − 1 and ≤ the
    /// root-order cap). Public within the crate for direct testing.
    pub(crate) fn convolve(
        &self,
        a: &[u32],
        b: &[u32],
        conv_len: usize,
        progress: &Progress,
    ) -> Result<Vec<u128>, Error> {
        let mut crt_rows: Vec<Vec<u64>> = Vec::with_capacity(NTT_PRIMES.len());
        for &(p, g) in NTT_PRIMES.iter() {
            let mut fa = vec![0_u64; conv_len];
            let mut fb = vec![0_u64; conv_len];
            for (i, &d) in a.iter().enumerate() {
                fa[i] = d as u64;
            }
            for (i, &d) in b.iter().enumerate() {
                fb[i] = d as u64;
            }
            let root = pow_mod_small(g, (p - 1) / conv_len as u64, p);
            let root_inv = pow_mod_small(root, p - 2, p);
            ntt_in_place(&mut fa, root, p, false);
            ntt_in_place(&mut fb, root, p, false);
            for i in 0..conv_len {
                fa[i] = mul_mod(fa[i], fb[i], p);
            }
            ntt_in_place(&mut fa, root_inv, p, true);
            crt_rows.push(fa);
            progress.check_cancelled()?;
        }

        let (p1, _) = NTT_PRIMES[0];
        let (p2, _) = NTT_PRIMES[1];
        let (p3, _) = NTT_PRIMES[2];
        let m12 = (p1 as u128) * (p2 as u128);
        let inv_p1_mod_p2 = pow_mod_small(p1 % p2, p2 - 2, p2);
        let inv_m12_mod_p3 = pow_mod_small((m12 % p3 as u128) as u64, p3 - 2, p3);

        let mut digits = vec![0_u128; conv_len];
        for i in 0..conv_len {
            let r1 = crt_rows[0][i] as u128;
            let r2 = crt_rows[1][i] as u128;
            let r3 = crt_rows[2][i] as u128;
            // x = r1 + p1·k2, k2 = (r2 − r1)·p1⁻¹ mod p2.
            let k2 = ((r2 + p2 as u128 * 3 - r1) % p2 as u128) * (inv_p1_mod_p2 as u128)
                % p2 as u128;
            let x12 = r1 + (p1 as u128) * k2;
            // x = x12 + m12·k3, k3 = (r3 − x12)·m12⁻¹ mod p3.
            let x12m = x12 % p3 as u128;
            let k3 = ((r3 + p3 as u128 * 3 - x12m) % p3 as u128)
                * (inv_m12_mod_p3 as u128)
                % p3 as u128;
            digits[i] = x12 + m12 * k3;
        }
        Ok(digits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biguint::BigUint;
    use crate::progress::Progress;
    use alloc::vec;

    const P: Progress = Progress::UNLIMITED;

    fn value_of(limbs: &[Limb]) -> BigUint {
        BigUint::from_limbs(limbs.to_vec())
    }

    #[test]
    fn ntt_convolve_small_exact() {
        // [1,2,3] ⋆ [4,5,6] = [4, 13, 28, 27, 18]
        let digits = Ntt.convolve(&[1, 2, 3], &[4, 5, 6], 8, &P).unwrap();
        assert_eq!(&digits[..5], &[4, 13, 28, 27, 18]);
        assert!(digits[5..].iter().all(|&d| d == 0));
        // Maximal 32-bit digits stress the CRT bound.
        let big = [u32::MAX; 4];
        let digits = Ntt.convolve(&big, &big, 16, &P).unwrap();
        // Reference: (2^32-1) ⋆ itself = [M², 2M², ...] with M = 2^32-1
        // (coefficients up to 4M² need more than u64 — u128 exactly).
        let mm = u128::from(u32::MAX) * u128::from(u32::MAX);
        for (i, want) in [mm, 2 * mm, 3 * mm, 4 * mm, 3 * mm, 2 * mm, mm]
            .iter()
            .enumerate()
        {
            assert_eq!(digits[i], *want, "coef {i}");
        }
    }

    #[test]
    fn ntt_pow_root_sanity() {
        // root of full order: 3^((p-1)/2) ≡ -1 mod 998244353.
        let (p, g) = NTT_PRIMES[0];
        assert_eq!(mul_mod(pow_mod_small(g, (p - 1) / 2, p), 1, p), p - 1);
    }

    #[test]
    fn ntt_matches_schoolbook_across_sizes() {
        let mut rng = tpt_mathr_base::testing::TestRng::new(0x00C0FFEE);
        // Sizes exercise the dispatch boundary without paying debug-build
        // schoolbook costs at full scale; full-scale runs go through
        // `cargo test --release` (CI) and the benchmark matrix.
        for &len in &[
            NTT_THRESHOLD - 1,
            NTT_THRESHOLD,
            NTT_THRESHOLD + 1,
            NTT_THRESHOLD + 33,
        ] {
            let a: Vec<Limb> = (0..len).map(|_| rng.next_u64()).collect();
            let b: Vec<Limb> = (0..len).map(|_| rng.next_u64()).collect();
            let via_ntt = Ntt.multiply(&a, &b, &P).unwrap();
            let via_ref = Schoolbook.multiply(&a, &b, &P).unwrap();
            assert_eq!(
                value_of(&via_ntt),
                value_of(&via_ref),
                "NTT mismatch at {len} limbs"
            );
        }
    }

    #[test]
    fn ntt_handles_maximal_digits() {
        // All-ones limbs maximise convolution coefficients and carries.
        let len = NTT_THRESHOLD + 5;
        let a: Vec<Limb> = vec![Limb::MAX; len];
        let via_ntt = Ntt.multiply(&a, &a, &P).unwrap();
        let via_ref = Schoolbook.multiply(&a, &a, &P).unwrap();
        assert_eq!(value_of(&via_ntt), value_of(&via_ref));
    }

    #[test]
    fn ntt_delegates_below_threshold() {
        let small = [1_u64, 2, 3];
        let out = Ntt.multiply(&small, &small, &P).unwrap();
        assert_eq!(value_of(&out), value_of(&Schoolbook.multiply(&small, &small, &P).unwrap()));
    }
}
