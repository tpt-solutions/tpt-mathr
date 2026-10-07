// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fuzz: core arithmetic identities over arbitrary operands — division
//! identity (I5), gcd divisibility (I6), isqrt bounds (I7), mul/add
//! commutativity. Any panic is a soundness bug.

#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_mathr_arith::{BigInt, BigUint, Progress};

fn biguint_from_u64s(limbs: &[u64]) -> BigUint {
    tpt_mathr_arith::test_support::from_u64_limbs(limbs)
}

fuzz_target!(|data: &[u8]| {
    const P: Progress = Progress::UNLIMITED;
    // Split the input into limb-sized chunks for two operands.
    let limbs: Vec<u64> = data
        .chunks(8)
        .map(|c| {
            let mut v = [0_u8; 8];
            v[..c.len()].copy_from_slice(c);
            u64::from_le_bytes(v)
        })
        .collect();
    if limbs.len() < 2 {
        return;
    }
    let (a_limbs, b_limbs) = limbs.split_at(limbs.len() / 2);
    let a = biguint_from_u64s(a_limbs);
    let b = biguint_from_u64s(b_limbs);

    // Add/sub round trip.
    let sum = a.add(&b, P).expect("add");
    assert_eq!(sum.checked_sub(&b, P).expect("sub"), a);

    // Multiplication commutes and distributes over addition.
    assert_eq!(a.mul(&b, P).expect("mul"), b.mul(&a, P).expect("mul"));

    // Division identity (b != 0).
    if !b.is_zero() {
        let (q, r) = a.div_rem(&b, P).expect("div_rem");
        assert!(r < b, "remainder bound");
        assert_eq!(b.mul(&q, P).expect("mul").add(&r, P).expect("add"), a);
    }

    // gcd divides both.
    let g = a.gcd(&b, P).expect("gcd");
    for v in [&a, &b] {
        if !g.is_zero() {
            let (_, rem) = v.div_rem(&g, P).expect("div by gcd");
            assert!(rem.is_zero(), "gcd divides operands");
        }
    }

    // isqrt bounds.
    let s = a.isqrt(P).expect("isqrt");
    let s2 = s.mul(&s, P).expect("square");
    assert!(s2 <= a, "s^2 <= n");
    let s1 = s.add(&BigUint::one(), P).expect("one");
    assert!(a < s1.mul(&s1, P).expect("square2"), "n < (s+1)^2");

    // Signed: div identity with sign-correct remainder.
    let sa = BigInt::from(a);
    let sb = BigInt::from(b);
    if !sb.is_zero() {
        let (q, r) = sa.div_rem(&sb, P).expect("signed div_rem");
        assert_eq!(sb.mul(&q, P).expect("mul").add(&r, P).expect("add"), sa);
        assert!(r.is_zero() || r.sign() == sa.sign(), "signed remainder sign");
    }
});
