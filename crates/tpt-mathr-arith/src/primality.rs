// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Primality testing and factorisation helpers (Phase 3).
//!
//! **No speed promises** (spec §5.1): these are correctness-first
//! primitives with documented scopes.
//!
//! # Search is not proof (spec §2.3)
//!
//! The Miller–Rabin test is a *search for witnesses*; its verdicts are
//! reported honestly via [`Primality`]:
//!
//! * [`Primality::Composite`] — proven composite (a witness was found).
//! * [`Primality::ProbablePrime`] — no witness found among the fixed base
//!   set. This is **not** a proof of primality for `n ≥ 3.3·10²⁴` (below
//!   that bound the base set is deterministic, see [`is_probable_prime`]).
//! * [`Primality::Prime`] — proven by construction (tiny values, exact
//!   small-prime table membership).
//!
//! Resource exhaustion and cancellation surface as `Err` — never as
//! "composite" (I9, and the spec §33 rule that exhaustion is not a
//! negative result).

use alloc::{vec, vec::Vec};

use tpt_mathr_base::error::Error;

use crate::biguint::BigUint;
use crate::bigint::{BigInt, Sign};
use crate::modular::pow_mod;
use crate::progress::Progress;

/// Miller–Rabin base set: the first 12 primes. Deterministic (no
/// counterexamples exist) for all `n < 3,317,044,064,679,887,385,961,981`
/// (≈ 3.3·10²⁴, Sorenson & Webster); above that, a `ProbablePrime` verdict
/// is probabilistic with error ≤ 4^(-12) per the standard bound.
pub(crate) const MR_BASES: &[u64] = &[2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];

/// Small-prime trial-division table: covers every prime below 256. Used to
/// prefilter candidates and to prove small factors exactly.
pub(crate) const SMALL_PRIMES: &[u64] = &[
    2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89,
    97, 101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179, 181, 191,
    193, 197, 199, 211, 223, 227, 229, 233, 239, 241, 251,
];

/// The verdict of a primality search, kept honest per spec §2.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Primality {
    /// Proven prime (tiny domain or exact small-prime match).
    Prime,
    /// No Miller–Rabin witness found; **not** a proof for large `n`.
    ProbablePrime,
    /// Proven composite (a witness or an explicit factor was found).
    Composite,
}

/// Classify `n` by strong base-`a` Miller–Rabin: `Ok(true)` iff `a` is a
/// witness that `n` is composite. Internal shared step.
fn mr_is_witness(a: &BigUint, n: &BigUint, d: &BigUint, s: u64, progress: &Progress) -> Result<bool, Error> {
    // x = a^d mod n
    let mut x = pow_mod(&BigInt::from(a.clone()), d, n, progress)?;
    if x == BigUint::one() || x == n.checked_sub(&BigUint::one(), progress)? {
        return Ok(false); // not a witness
    }
    for _ in 0..s.saturating_sub(1) {
        progress.check_cancelled()?;
        x = x.mul(&x, progress)?.rem(n, progress)?;
        if x == n.checked_sub(&BigUint::one(), progress)? {
            return Ok(false);
        }
    }
    Ok(true) // composite witness
}

/// Decompose `n - 1 = 2^s · d` with `d` odd. `n` must be odd and > 2.
fn decompose(n_minus_1: &BigUint, progress: &Progress) -> Result<(u64, BigUint), Error> {
    let s = n_minus_1.trailing_zeros().expect("n odd ⇒ n-1 even and nonzero");
    let d = n_minus_1.shr(s, progress)?;
    Ok((s, d))
}

impl BigUint {
    /// Miller–Rabin classification of `n ≥ 0` with the fixed base set
    /// (see [`MR_BASES`]). Verdicts are honest: `ProbablePrime` is not a
    /// proof above the deterministic bound.
    ///
    /// # Errors
    ///
    /// Allocation, limits, cancellation.
    pub fn is_probable_prime(&self, progress: &Progress) -> Result<Primality, Error> {
        if self < &BigUint::from(2_u8) {
            return Ok(Primality::Composite); // 0 and 1 are neither
        }
        // Exact small-value handling.
        for &p in SMALL_PRIMES {
            let pv = BigUint::from(p);
            if self == &pv {
                return Ok(Primality::Prime);
            }
            let (_, r) = self.div_rem(&pv, progress)?;
            if r.is_zero() {
                return Ok(Primality::Composite);
            }
        }
        let n_minus_1 = self.checked_sub(&BigUint::one(), progress)?;
        let (s, d) = decompose(&n_minus_1, progress)?;
        for &a in MR_BASES {
            // Every base is < n here: values below the small-prime table's
            // range were resolved exactly above (any surviving n > 251
            // exceeds every base in the set).
            if mr_is_witness(&BigUint::from(a), self, &d, s, progress)? {
                return Ok(Primality::Composite);
            }
        }
        if self.bit_len() <= 83 {
            // 2^83 > 3.3·10^24: the base set is deterministic here.
            Ok(Primality::Prime)
        } else {
            Ok(Primality::ProbablePrime)
        }
    }
}

/// One factor found by [`factorise`]: prime (probable above the
/// deterministic bound) and its multiplicity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimeFactor {
    /// The prime (exact below the deterministic MR bound; probable above).
    pub prime: BigUint,
    /// Its multiplicity in the factorisation.
    pub exponent: u32,
}

/// Factor a positive integer into prime powers via trial division plus
/// Pollard's rho (Brent's cycle variant). Returns the factors sorted by
/// prime; empty for `n = 1`.
///
/// **No speed promises** (spec §5.1): worst-case behaviour for adversarial
/// inputs is not characterised; budget via [`Progress`].
///
/// # Errors
///
/// [`Error::DivisionByZero`] for `n = 0`; allocation, limits, cancellation
/// otherwise.
pub fn factorise(n: &BigUint, progress: &Progress) -> Result<Vec<PrimeFactor>, Error> {
    if n.is_zero() {
        return Err(Error::DivisionByZero);
    }
    let mut out: Vec<PrimeFactor> = Vec::new();
    let mut todo: Vec<BigUint> = vec![n.clone()];
    let mut iterations = 0_u64;

    while let Some(v) = todo.pop() {
        if v == BigUint::one() {
            continue;
        }
        iterations += 1;
        if iterations % 16 == 0 {
            progress.check_cancelled()?;
        }
        // Trial division by small primes first (cheap, exact).
        let mut v = v;
        for &p in SMALL_PRIMES {
            let pv = BigUint::from(p);
            let (q, r) = v.div_rem(&pv, progress)?;
            if r.is_zero() {
                let mut e = 1_u32;
                let mut v2 = q;
                loop {
                    let (q2, r2) = v2.div_rem(&pv, progress)?;
                    if r2.is_zero() {
                        v2 = q2;
                        e += 1;
                    } else {
                        break;
                    }
                }
                push_factor(&mut out, pv, e);
                v = v2;
            }
        }
        if v == BigUint::one() {
            continue;
        }
        // Primality of the surviving cofactor must be checked
        // unconditionally: stripping small primes does NOT make the rest
        // prime (e.g. 347·p for a large probable-prime p).
        if v.is_probable_prime(progress)? != Primality::Composite {
            push_factor(&mut out, v, 1);
            continue;
        }
        // Pollard rho with deterministic pseudo-random coefficients.
        let f = pollard_rho(&v, progress)?;
        if f == BigUint::one() || f == v {
            // The full coefficient schedule failed to split v: report
            // honestly rather than risk a wrong factorisation
            // (`Inconclusive` is an Err and never a verdict — spec §33).
            return Err(Error::Inconclusive);
        }
        let (q, _) = v.div_rem(&f, progress)?;
        todo.push(f);
        todo.push(q);
    }
    out.sort_by(|a, b| a.prime.cmp(&b.prime));
    Ok(out)
}

fn push_factor(out: &mut Vec<PrimeFactor>, prime: BigUint, exponent: u32) {
    if let Some(existing) = out.iter_mut().find(|f| f.prime == prime) {
        existing.exponent += exponent;
    } else {
        out.push(PrimeFactor { prime, exponent });
    }
}

/// Pollard's rho (Brent) with fixed coefficient schedule. Returns a
/// non-trivial factor of the (odd, composite, > 1) input.
fn pollard_rho(n: &BigUint, progress: &Progress) -> Result<BigUint, Error> {
    let n = n.clone();
    let one = BigUint::one();
    let two = BigUint::from(2_u8);
    for &c0 in [1_u64, 3, 5, 7, 11, 13, 23, 41].iter() {
        let c = BigUint::from(c0);
        let mut x = two.clone();
        let mut y = two.clone();
        let mut d = one.clone();
        let mut iterations = 0_u64;
        while d == one {
            iterations += 1;
            if iterations % 64 == 0 {
                progress.check_cancelled()?;
            }
            // Floyd cycle: x ← x²+c, y ← f(f(y))
            x = step(&x, &c, &n, progress)?;
            y = {
                let y1 = step(&y, &c, &n, progress)?;
                step(&y1, &c, &n, progress)?
            };
            let diff = if x >= y {
                x.checked_sub(&y, progress)?
            } else {
                y.checked_sub(&x, progress)?
            };
            let g = diff.gcd(&n, progress)?;
            if g != one {
                d = g;
            }
        }
        if d != n {
            return Ok(d);
        }
        // d == n: retry with the next coefficient.
    }
    // Exhausted the coefficient schedule without a split.
    Err(Error::Inconclusive)
}

fn step(x: &BigUint, c: &BigUint, n: &BigUint, progress: &Progress) -> Result<BigUint, Error> {
    let x2 = x.mul(x, progress)?;
    let x2c = x2.add(c, progress)?;
    x2c.rem(n, progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bu(v: u64) -> BigUint {
        BigUint::from(v)
    }

    #[test]
    fn primality_known_values() {
        let p = Progress::UNLIMITED;
        for v in [2_u64, 3, 5, 7, 97, 7919, 2_147_483_647, 672_804_213_107_21] {
            assert_eq!(bu(v).is_probable_prime(&p).unwrap(), Primality::Prime, "{v}");
        }
        for v in [0_u64, 1, 4, 561, 1_105, 1_729, 2_147_483_646, 3825123056546413051] {
            // 561/1105/1729: Carmichael; 3825123056546413051: strong
            // pseudoprime to the first several bases — the 12-base set
            // catches all of these as composites.
            assert_eq!(bu(v).is_probable_prime(&p).unwrap(), Primality::Composite, "{v}");
        }
        // Probable (not proven) domain: a large known prime.
        let mersenne = bu(2).pow(127, &p).unwrap().checked_sub(&bu(1), &p).unwrap();
        assert_eq!(mersenne.is_probable_prime(&p).unwrap(), Primality::ProbablePrime);
    }

    #[test]
    fn factorisation_reconstructs_n() {
        let p = Progress::UNLIMITED;
        for (n, want) in [
            (1_u64, Vec::new()),
            (2, vec![(2_u64, 1_u32)]),
            (360, vec![(2, 3), (3, 2), (5, 1)]),
            (1024, vec![(2, 10)]),
            (561, vec![(3, 1), (11, 1), (17, 1)]),
            (9973, vec![(9973, 1)]),
            // Primorial 23#: 2·3·5·7·11·13·17·19·23.
            (223_092_870, vec![(2, 1), (3, 1), (5, 1), (7, 1), (11, 1), (13, 1), (17, 1), (19, 1), (23, 1)]),
            // (2^31 - 1)² : square of a Mersenne prime.
            (4_611_686_014_132_420_609, vec![(2_147_483_647, 2)]),
        ] {
            let n = bu(n);
            let factors = factorise(&n, &p).unwrap();
            let got: Vec<(u64, u32)> = factors
                .iter()
                .map(|f| (f.prime.to_u64().unwrap(), f.exponent))
                .collect();
            assert_eq!(got, want, "factorisation of {n}");
        }
    }

    #[test]
    fn factorisation_semiprime_with_large_factors() {
        let p = Progress::UNLIMITED;
        // (2^61 - 1)·(2^31 - 1): both Mersenne primes.
        let a = bu(2).pow(61, &p).unwrap().checked_sub(&bu(1), &p).unwrap();
        let b = bu(2).pow(31, &p).unwrap().checked_sub(&bu(1), &p).unwrap();
        let n = a.mul(&b, &p).unwrap();
        let factors = factorise(&n, &p).unwrap();
        let mut got: Vec<BigUint> = factors.into_iter().map(|f| f.prime).collect();
        got.sort();
        let mut want = vec![b, a];
        want.sort();
        assert_eq!(got, want);
    }

    #[test]
    fn zero_input_is_rejected() {
        assert_eq!(
            factorise(&BigUint::zero(), &Progress::UNLIMITED).unwrap_err(),
            Error::DivisionByZero
        );
    }
}
