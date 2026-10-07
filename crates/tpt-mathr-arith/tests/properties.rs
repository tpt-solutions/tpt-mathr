// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Property tests: algebraic identities over randomly generated operands
//! (spec §30 "property tests"), using the dependency-free deterministic RNG
//! from `tpt-mathr-base::testing`. A failure is reproducible: the seeds are
//! printed in the panic message.

use tpt_mathr_arith::{BigInt, BigUint, Progress};
use tpt_mathr_base::testing::TestRng;

const P: Progress = Progress::UNLIMITED;

/// Random `BigUint` with up to `max_limbs` limbs, normalised (never zero
/// unless `allow_zero`).
fn rand_biguint(rng: &mut TestRng, max_limbs: usize, allow_zero: bool) -> BigUint {
    let n = rng.below_usize(max_limbs) + 1;
    let limbs: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
    let v = tpt_mathr_arith::test_support::from_u64_limbs(&limbs);
    if allow_zero || !v.is_zero() {
        v
    } else {
        BigUint::one()
    }
}

fn rand_bigint(rng: &mut TestRng, max_limbs: usize) -> BigInt {
    let v = rand_biguint(rng, max_limbs, true);
    if rng.chance(1, 2) {
        BigInt::from(v)
    } else {
        BigInt::from(v).neg()
    }
}

#[test]
fn biguint_add_commutative_and_associative() {
    let mut rng = TestRng::new(0x0C01);
    for i in 0..300 {
        let a = rand_biguint(&mut rng, 8, true);
        let b = rand_biguint(&mut rng, 8, true);
        let c = rand_biguint(&mut rng, 8, true);
        assert_eq!(
            a.add(&b, &P).unwrap(),
            b.add(&a, &P).unwrap(),
            "commutativity, case {i}"
        );
        let ab_c = a.add(&b, &P).unwrap().add(&c, &P).unwrap();
        let a_bc = a.add(&b.add(&c, &P).unwrap(), &P).unwrap();
        assert_eq!(ab_c, a_bc, "associativity, case {i}");
    }
}

#[test]
fn biguint_mul_commutative_associative_distributive() {
    let mut rng = TestRng::new(0x0C02);
    for i in 0..150 {
        let a = rand_biguint(&mut rng, 6, true);
        let b = rand_biguint(&mut rng, 6, true);
        let c = rand_biguint(&mut rng, 6, true);
        assert_eq!(
            a.mul(&b, &P).unwrap(),
            b.mul(&a, &P).unwrap(),
            "mul commutativity {i}"
        );
        let ab_c = a.mul(&b, &P).unwrap().mul(&c, &P).unwrap();
        let a_bc = a.mul(&b.mul(&c, &P).unwrap(), &P).unwrap();
        assert_eq!(ab_c, a_bc, "mul associativity {i}");
        let left = a.mul(&b.add(&c, &P).unwrap(), &P).unwrap();
        let right = a
            .mul(&b, &P)
            .unwrap()
            .add(&a.mul(&c, &P).unwrap(), &P)
            .unwrap();
        assert_eq!(left, right, "distributivity {i}");
    }
}

#[test]
fn biguint_sub_add_identity() {
    let mut rng = TestRng::new(0x0C03);
    for i in 0..300 {
        let a = rand_biguint(&mut rng, 8, false);
        let b = rand_biguint(&mut rng, 8, false);
        let (lo, hi) = if a < b { (&a, &b) } else { (&b, &a) };
        let d = hi.checked_sub(lo, &P).unwrap();
        assert_eq!(lo.add(&d, &P).unwrap(), *hi, "a + (b - a) == b, case {i}");
    }
}

#[test]
fn division_identity_i5_everywhere() {
    let mut rng = TestRng::new(0x0C04);
    for i in 0..300 {
        let a = BigInt::from(rand_biguint(&mut rng, 10, true));
        let mut b = BigInt::from(rand_biguint(&mut rng, 5, false));
        if rng.chance(1, 2) {
            b = b.neg();
        }
        let (q, r) = a.div_rem(&b, &P).unwrap();
        // I5: a == b*q + r; sign(r) == sign(a) or r == 0; |r| < |b|.
        let back = b.mul(&q, &P).unwrap().add(&r, &P).unwrap();
        assert_eq!(back, a, "division identity, case {i}");
        assert!(r.is_zero() || r.sign() == a.sign(), "remainder sign {i}");
        assert!(r.magnitude() < b.magnitude(), "remainder bound {i}");
    }
}

#[test]
fn gcd_divides_both_operands() {
    let mut rng = TestRng::new(0x0C05);
    for i in 0..200 {
        let a = rand_biguint(&mut rng, 8, true);
        let b = rand_biguint(&mut rng, 8, true);
        let g = a.gcd(&b, &P).unwrap();
        for (v, name) in [(&a, "a"), (&b, "b")] {
            let (_, r) = v.div_rem(&g, &P).unwrap();
            assert!(r.is_zero(), "gcd must divide {name}, case {i}");
        }
        // Commutativity of gcd:
        assert_eq!(g, b.gcd(&a, &P).unwrap(), "gcd commutativity {i}");
    }
}

#[test]
fn lcm_times_gcd_identity() {
    let mut rng = TestRng::new(0x0C06);
    for i in 0..100 {
        let a = rand_biguint(&mut rng, 5, true);
        let b = rand_biguint(&mut rng, 5, true);
        if a.is_zero() || b.is_zero() {
            continue;
        }
        let g = a.gcd(&b, &P).unwrap();
        let l = a.lcm(&b, &P).unwrap();
        assert_eq!(
            g.mul(&l, &P).unwrap(),
            a.mul(&b, &P).unwrap(),
            "|a*b| == gcd*lcm, case {i}"
        );
    }
}

#[test]
fn pow_by_squaring_equals_repeated_multiplication() {
    let mut rng = TestRng::new(0x0C07);
    for i in 0..60 {
        let base = rand_biguint(&mut rng, 3, false);
        let e = rng.below(40);
        let fast = base.pow(e, &P).unwrap();
        let mut slow = BigUint::one();
        for _ in 0..e {
            slow = slow.mul(&base, &P).unwrap();
        }
        assert_eq!(fast, slow, "pow identity, case {i} (e={e})");
    }
}

#[test]
fn isqrt_floor_property_i7() {
    let mut rng = TestRng::new(0x0C08);
    for i in 0..150 {
        let n = rand_biguint(&mut rng, 6, true);
        let s = n.isqrt(&P).unwrap();
        // s^2 <= n < (s+1)^2
        let s2 = s.mul(&s, &P).unwrap();
        let s1 = s.add(&BigUint::one(), &P).unwrap();
        let s12 = s1.mul(&s1, &P).unwrap();
        assert!(s2 <= n, "s^2 <= n, case {i}");
        assert!(n < s12, "n < (s+1)^2, case {i}");
    }
}

#[test]
fn signed_negation_involutive() {
    let mut rng = TestRng::new(0x0C09);
    for i in 0..200 {
        let a = rand_bigint(&mut rng, 6);
        assert_eq!(a.neg().neg(), a, "double negation, case {i}");
        // a - (-a) == 2a, and a + (-a) == 0:
        let two_a = a.add(&a, &P).unwrap();
        let a_neg_a = a.checked_sub(&a.neg(), &P).unwrap();
        assert_eq!(a_neg_a, two_a, "a - (-a) == 2a, case {i}");
        assert!(
            a.add(&a.neg(), &P).unwrap().is_zero(),
            "a + (-a) == 0, case {i}"
        );
    }
}

#[test]
fn mul_dispatch_paths_agree_with_schoolbook() {
    // The strategy table must be observationally equivalent to the
    // reference schoolbook path across operand-size combinations,
    // including the Karatsuba recursion boundaries (I10 determinism).
    let mut rng = TestRng::new(0x00CA);
    for i in 0..40 {
        let a = rand_biguint(&mut rng, 90, false);
        let b = rand_biguint(&mut rng, 90, false);
        let via_api = a.mul(&b, &P).unwrap();
        let via_ref = tpt_mathr_arith::test_support::mul_schoolbook(&a, &b);
        assert_eq!(via_api, via_ref, "dispatch vs schoolbook, case {i}");
    }
}
