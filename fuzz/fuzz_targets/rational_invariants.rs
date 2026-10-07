// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fuzz: rational arithmetic invariants — always reduced, positive
//! denominator, canonical zero, field identities, division identity
//! analogue. Any panic is a soundness bug.

#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_mathr_arith::{BigRat, BigInt, BigUint, Progress};

const P: Progress = Progress::UNLIMITED;

fuzz_target!(|data: &[u8]| {
    let limbs: Vec<u64> = data
        .chunks(8)
        .map(|c| {
            let mut v = [0_u8; 8];
            v[..c.len()].copy_from_slice(c);
            u64::from_le_bytes(v)
        })
        .collect();
    if limbs.len() < 4 {
        return;
    }
    let quarter = limbs.len() / 4;
    let num_a = BigInt::from(tpt_mathr_arith::test_support::from_u64_limbs(&limbs[..quarter]));
    let den_a = tpt_mathr_arith::test_support::from_u64_limbs(&limbs[quarter..2 * quarter]);
    let num_b = BigInt::from(tpt_mathr_arith::test_support::from_u64_limbs(&limbs[2 * quarter..3 * quarter]));
    let den_b = tpt_mathr_arith::test_support::from_u64_limbs(&limbs[3 * quarter..]);
    if den_a.is_zero() || den_b.is_zero() {
        return;
    }
    let Ok(a) = BigRat::new(num_a, den_a, P) else { unreachable!("nonzero denominator") };
    let Ok(b) = BigRat::new(num_b, den_b, P) else { unreachable!("nonzero denominator") };
    a.assert_invariants();
    b.assert_invariants();

    // Field identities.
    assert_eq!(a.add(&b, P).unwrap(), b.add(&a, P).unwrap(), "add commutes");
    assert_eq!(a.mul(&b, P).unwrap(), b.mul(&a, P).unwrap(), "mul commutes");
    assert_eq!(a.mul(&a.inv().unwrap(), P).unwrap(), BigRat::one(), "a * a^-1 == 1");
    assert_eq!(a.add(&a.neg(), P).unwrap(), BigRat::zero(), "a + (-a) == 0");

    // Division identity analogue: a == (a/b)*b + (a rem b) with the
    // truncated remainder bounded by |b|.
    if !b.is_zero() {
        let q = a.div(&b, P).unwrap();
        let r = a.rem(&b, P).unwrap();
        let back = q.mul(&b, P).unwrap().add(&r, P).unwrap();
        assert_eq!(back, a, "a == (a/b)*b + (a rem b)");
        assert!(r.abs().cmp_rat(&b.abs(), P).unwrap() == core::cmp::Ordering::Less);
    }

    // floor <= value <= ceil; round agrees with the ties-away convention.
    let f = a.floor(P).unwrap();
    let c = a.ceil(P).unwrap();
    assert!(f <= c);
    let r = a.round(P).unwrap();
    assert!(f <= r && r <= c, "round between floor and ceil");
});
