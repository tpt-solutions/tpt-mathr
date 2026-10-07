// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Differential corpus for the Phase 3 paths vs Python (dev-only):
//! `pow_mod`, `mod_inverse`, Miller–Rabin primality, and factorisation.
//! Python's `pow(a, e, m)` / `pow(a, -1, m)` are the references; primality
//! and factorisation are cross-checked with pure-Python Miller–Rabin in the
//! interpreter preamble (sympy is not assumed).

#[path = "common/mod.rs"]
mod common;

use common::{skip_note, Python};
use tpt_mathr_arith::{factorise, mod_inverse, pow_mod, BigInt, BigUint, Primality, Progress};
use tpt_mathr_base::testing::TestRng;

const P: Progress = Progress::UNLIMITED;

fn bu(v: u64) -> BigUint {
    BigUint::from(v)
}

fn rand_value(rng: &mut TestRng, max_limbs: usize) -> (BigUint, String) {
    let limbs: Vec<u64> = (0..rng.below_usize(max_limbs + 1)).map(|_| rng.next_u64()).collect();
    let v = tpt_mathr_arith::test_support::from_u64_limbs(&limbs);
    (v.clone(), format!("0x{}", v.to_str_radix(16, &P).unwrap()))
}

#[test]
fn differential_pow_mod_vs_python() {
    let Some(py) = Python::discover() else {
        skip_note("differential_phase3::differential_pow_mod_vs_python");
        return;
    };
    let mut rng = TestRng::new(0x0D03_0001);
    let mut exprs = Vec::new();
    let mut ours = Vec::new();

    for _ in 0..80 {
        let (mut m, _) = rand_value(&mut rng, 4);
        if m < bu(2) {
            m = bu(1_000_000_007);
        }
        let m_lit = format!("0x{}", m.to_str_radix(16, &P).unwrap());
        let (b, b_lit) = rand_value(&mut rng, 8);
        let e = rng.below(512);
        // Negative bases too (Python semantics: canonical residue).
        let neg = if rng.chance(1, 3) { "-" } else { "" };
        let want = pow_mod(
            &if neg == "-" {
                BigInt::from(b).neg()
            } else {
                BigInt::from(b)
            },
            &bu(e),
            &m,
            &P,
        )
        .unwrap();
        exprs.push(format!("pow({neg}{b_lit}, {e}, {m_lit})"));
        ours.push(want.to_str_radix(10, &P).unwrap());
    }

    // Huge-exponent case: a^(p-1) mod p for a random prime-shaped modulus
    // (2^127 - 1 is prime; Fermat ⇒ 1 for a ∤ p).
    let p127 = bu(2).pow(127, &P).unwrap().checked_sub(&bu(1), &P).unwrap();
    let lit = format!("0x{}", p127.to_str_radix(16, &P).unwrap());
    for base in [3_u64, 5, 7, 1 << 40] {
        let want = pow_mod(&BigInt::from(base), &p127, &p127, &P).unwrap();
        exprs.push(format!("pow({base}, {lit}, {lit})"));
        ours.push(want.to_str_radix(10, &P).unwrap());
    }

    let got = py.eval_bigint_batch(&exprs).expect("python batch eval");
    for (i, (expr, want)) in exprs.iter().zip(ours.iter()).enumerate() {
        assert_eq!(&got[i], want, "pow_mod case {i}: {expr}");
    }
}

#[test]
fn differential_mod_inverse_vs_python() {
    let Some(py) = Python::discover() else {
        skip_note("differential_phase3::differential_mod_inverse_vs_python");
        return;
    };
    let mut rng = TestRng::new(0x0D03_0002);
    let mut exprs = Vec::new();
    let mut ours = Vec::new();

    for _ in 0..60 {
        let (mut m, _) = rand_value(&mut rng, 3);
        if m < bu(3) {
            m = bu(1_000_000_007);
        }
        if m.rem(&bu(2), &P).unwrap() == BigUint::zero() {
            m = m.add(&bu(1), &P).unwrap();
        }
        let (a, _) = rand_value(&mut rng, 3);
        let a = a.rem(&m, &P).unwrap();
        // Ensure invertibility: gcd(a, m) == 1 else resample with a+1.
        let a = match mod_inverse(&BigInt::from(a.clone()), &m, &P) {
            Ok(_) => a,
            Err(_) => a.add(&bu(1), &P).unwrap(),
        };
        let inv = match mod_inverse(&BigInt::from(a.clone()), &m, &P) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let m_lit = format!("0x{}", m.to_str_radix(16, &P).unwrap());
        let a_lit = format!("0x{}", a.to_str_radix(16, &P).unwrap());
        exprs.push(format!("pow({a_lit}, -1, {m_lit})"));
        ours.push(inv.to_str_radix(10, &P).unwrap());
    }

    let got = py.eval_bigint_batch(&exprs).expect("python batch eval");
    for (i, (expr, want)) in exprs.iter().zip(ours.iter()).enumerate() {
        assert_eq!(&got[i], want, "mod_inverse case {i}: {expr}");
    }
}

#[test]
fn differential_primality_vs_python_mr() {
    let Some(py) = Python::discover() else {
        skip_note("differential_phase3::differential_primality_vs_python_mr");
        return;
    };
    let mut rng = TestRng::new(0x0D03_0003);

    // A mix of primes, composites, Carmichaels, and strong pseudoprimes.
    let mut candidates: Vec<BigUint> = Vec::new();
    for v in [
        2_u64, 3, 5, 7, 97, 561, 1105, 1729, 2465, 2821, 6601, 7919, 104729,
        2_147_483_647, 3215031751, 3825123056546413051,
    ] {
        candidates.push(bu(v));
        // A value above the 2^64 limb range with known MR behaviour:
        // 2^89 - 1 is prime (probable above the deterministic bound).
        candidates.push(bu(2).pow(89, &P).unwrap().checked_sub(&bu(1), &P).unwrap());
        // Strong pseudoprime to bases 2..~ with a factor 318665857834031151167461:
        // use its known 2-limb shape directly.
        candidates.push(
            BigUint::from_str_radix(
                "318665857834031151167461",
                10,
                &P,
            )
            .unwrap(),
        );
    }
    for _ in 0..40 {
        let (v, _) = rand_value(&mut rng, 4);
        // force odd: an even value is made odd by +1
        let v = if v.rem(&bu(2), &P).unwrap() == BigUint::zero() {
            v.add(&bu(1), &P).unwrap()
        } else {
            v
        };
        candidates.push(v);
    }

    let mut exprs = Vec::new();
    for n in &candidates {
        // Pure-Python strong Miller–Rabin with the same 12-base set.
        let lit = format!("0x{}", n.to_str_radix(16, &P).unwrap());
        exprs.push(format!("__mr({lit})"));
    }
    let script_preamble = "def __mr(n):\n    if n < 2: return 'Composite'\n    for p in [2,3,5,7,11,13,17,19,23,29,31,37]:\n        if n == p: return 'Prime'\n        if n % p == 0: return 'Composite'\n    d = n - 1\n    s = 0\n    while d % 2 == 0:\n        d //= 2\n        s += 1\n    for a in [2,3,5,7,11,13,17,19,23,29,31,37]:\n        xv = pow(a, d, n)\n        if xv in (1, n - 1): continue\n        for _ in range(s - 1):\n            xv = xv * xv % n\n            if xv == n - 1: break\n        else:\n            return 'Composite'\n    return 'Prime'";

    let got = py.eval_with_preamble(script_preamble, &exprs).expect("python mr batch");
    for (i, (n, line)) in candidates.iter().zip(got.iter()).enumerate() {
        let verdict = n.is_probable_prime(&P).unwrap();
        let want = match verdict {
            Primality::Prime | Primality::ProbablePrime => "Prime",
            Primality::Composite => "Composite",
        };
        assert_eq!(line, want, "primality mismatch for {n:?} (case {i})");
    }
}

#[test]
fn differential_factorisation_vs_python_verification() {
    let Some(py) = Python::discover() else {
        skip_note("differential_phase3::differential_factorisation_vs_python_verification");
        return;
    };
    let mut rng = TestRng::new(0x0D03_0004);
    let mut ns: Vec<BigUint> = vec![
        bu(1),
        bu(2),
        bu(360),
        bu(561),
        bu(2).pow(40, &P).unwrap(),
        (bu(2).pow(61, &P).unwrap().checked_sub(&bu(1), &P).unwrap())
            .mul(&bu(2).pow(31, &P).unwrap().checked_sub(&bu(1), &P).unwrap(), &P)
            .unwrap(),
        bu(9_007_199_254_740_881), // (2^53-1)/... actually 9007199254740881 is prime
    ];
    for _ in 0..10 {
        // Bound random inputs to ~64 bits: Pollard rho has no speed
        // promises (spec 5.1), and a random 128+-bit semiprime with two
        // large prime factors can take astronomically long to split. Small
        // factors are what the phase-3 helpers guarantee to handle.
        let (v, _) = rand_value(&mut rng, 1);
        let v = if v.rem(&bu(2), &P).unwrap() == BigUint::zero() {
            v.add(&bu(1), &P).unwrap()
        } else {
            v
        };
        ns.push(v);
    }

    let mut exprs = Vec::new();
    let mut ours = Vec::new();
    for n in &ns {
        let factors = factorise(n, &P).unwrap();
        // Render verification expression: product == n AND every factor
        // passes MR.
        let mut prod_terms = Vec::new();
        let mut factor_checks = Vec::new();
        for f in &factors {
            let lit = f.prime.to_str_radix(10, &P).unwrap();
            prod_terms.push(format!("{lit}**{}", f.exponent));
            factor_checks.push(format!(
                "__mr({lit}) in ('Prime',)"
            ));
        }
        let n_lit = format!("0x{}", n.to_str_radix(16, &P).unwrap());
        let product = if prod_terms.is_empty() {
            "1".to_string()
        } else {
            prod_terms.join("*")
        };
        let checks = if factor_checks.is_empty() {
            "True".to_string()
        } else {
            factor_checks.join(" and ")
        };
        exprs.push(format!("({product} == {n_lit}) and ({checks})"));
        ours.push(n.to_str_radix(10, &P).unwrap());
    }

    let script_preamble = "def __mr(n):\n    if n < 2: return 'Composite'\n    for p in [2,3,5,7,11,13,17,19,23,29,31,37]:\n        if n == p: return 'Prime'\n        if n % p == 0: return 'Composite'\n    d = n - 1\n    s = 0\n    while d % 2 == 0:\n        d //= 2\n        s += 1\n    for a in [2,3,5,7,11,13,17,19,23,29,31,37]:\n        xv = pow(a, d, n)\n        if xv in (1, n - 1): continue\n        for _ in range(s - 1):\n            xv = xv * xv % n\n            if xv == n - 1: break\n        else:\n            return 'Composite'\n    return 'Prime'";

    let got = py.eval_with_preamble(script_preamble, &exprs).expect("python verify batch");
    for (i, line) in got.iter().enumerate() {
        assert_eq!(line, "True", "factorisation verification failed for {}", ours[i]);
    }
}
