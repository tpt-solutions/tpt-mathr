// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Differential corpus for `BigRat` vs Python `fractions.Fraction`
//! (Phase 4; dev-only). Run with:
//!
//! ```text
//! cargo test -p tpt-mathr-arith --features differential-tests,testing
//! ```
//!
//! Python's `Fraction` auto-reduces, so component comparison is exact.
//! `round()` differs by convention (Python: half-even; engine: half away
//! from zero, normative in `rational.rs`), so the rounding corpus uses an
//! explicit away-from-zero reference instead of `round()`.

#[path = "common/mod.rs"]
mod common;

use common::{Python, skip_note};
use tpt_mathr_arith::{BigInt, BigRat, BigUint, Progress};
use tpt_mathr_base::testing::TestRng;

const P: Progress = Progress::UNLIMITED;

/// A random rational plus its Python-safe rendering (`Fraction(n, d)` with
/// a normalised positive denominator and signed numerator).
fn rand_rat(rng: &mut TestRng, max_bits: usize) -> (BigRat, String) {
    let n_limbs: Vec<u64> = (0..rng.below_usize(max_bits / 64 + 1))
        .map(|_| rng.next_u64())
        .collect();
    // At least one limb: the denominator must be positive.
    let d_limbs: Vec<u64> = (0..rng.below_usize(max_bits / 64 + 1) + 1)
        .map(|_| rng.next_u64() | 1) // odd, hence nonzero
        .collect();
    let num = BigInt::from(tpt_mathr_arith::test_support::from_u64_limbs(&n_limbs));
    let num = if rng.chance(1, 2) { num.neg() } else { num };
    let den = tpt_mathr_arith::test_support::from_u64_limbs(&d_limbs);
    let rat = BigRat::new(num.clone(), den.clone(), &P).expect("reduction");
    let lit = format!(
        "Fraction({num},{den})",
        num = num.to_str_radix(10, &P).unwrap(),
        den = den.to_str_radix(10, &P).unwrap()
    );
    (rat, lit)
}

fn render(r: &BigRat) -> (String, String) {
    (
        r.numerator().to_str_radix(10, &P).unwrap(),
        r.denominator().to_str_radix(10, &P).unwrap(),
    )
}

#[test]
fn differential_rational_arithmetic_vs_python() {
    let Some(py) = Python::discover() else {
        skip_note("differential_rational::differential_rational_arithmetic_vs_python");
        return;
    };
    let mut rng = TestRng::new(0x0F4C_0001);

    // Batch 1: add, sub, mul, div, rem over random pairs.
    let mut exprs = Vec::new();
    let mut expected = Vec::new();
    for _ in 0..80 {
        let (a, la) = rand_rat(&mut rng, 256);
        let (b, lb) = rand_rat(&mut rng, 128);

        let (an, ad) = render(&a.add(&b, &P).unwrap());
        exprs.push(format!("({la} + {lb})"));
        expected.push(format!("{an}/{ad}"));

        let (sn, sd) = render(&a.checked_sub(&b, &P).unwrap());
        exprs.push(format!("({la} - {lb})"));
        expected.push(format!("{sn}/{sd}"));

        let (mn, md) = render(&a.mul(&b, &P).unwrap());
        exprs.push(format!("({la} * {lb})"));
        expected.push(format!("{mn}/{md}"));

        if !b.is_zero() {
            let (dn, dd) = render(&a.div(&b, &P).unwrap());
            exprs.push(format!("({la} / {lb})"));
            expected.push(format!("{dn}/{dd}"));

            // Truncated remainder: r = a - trunc(a/b)·b. The dividend is
            // substituted literally (a quotient alone would compute
            // q - trunc(q)·b — a different quantity).
            let (rn, rd) = render(&a.rem(&b, &P).unwrap());
            exprs.push(format!("(lambda q: {la} - int(q) * {lb})({la} / {lb})"));
            expected.push(format!("{rn}/{rd}"));
        }
    }

    // Batch 2: floor/ceil and away-from-zero rounding.
    for _ in 0..60 {
        let (a, la) = rand_rat(&mut rng, 200);
        let f = a.floor(&P).unwrap();
        exprs.push(format!("math.floor({la})"));
        expected.push(f.to_str_radix(10, &P).unwrap());

        let c = a.ceil(&P).unwrap();
        exprs.push(format!("math.ceil({la})"));
        expected.push(c.to_str_radix(10, &P).unwrap());

        let r = a.round(&P).unwrap();
        exprs.push(format!(
            "(math.floor({la} + Fraction(1,2)) if {la} >= 0 else math.ceil({la} - Fraction(1,2)))"
        ));
        expected.push(r.to_str_radix(10, &P).unwrap());
    }

    let got = py.eval_bigint_batch(&exprs).expect("python batch eval");
    assert_eq!(got.len(), expected.len(), "batch lost lines");
    for (i, (expr, want)) in exprs.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            line_value(&got[i]),
            line_value(want),
            "case {i} mismatch\n  expr: {expr}\n  python: {}\n  engine: {want}",
            got[i]
        );
    }
}

/// Normalise Python's output for `Fraction` values ("n/d") and integers
/// (Python prints an integral Fraction bare, e.g. `0`, `3`).
fn line_value(line: &str) -> String {
    let s = line.replace(' ', "");
    if s.contains('/') { s } else { format!("{s}/1") }
}

#[test]
fn differential_rational_reduction_is_canonical() {
    // Python's Fraction constructor reduces exactly like `BigRat::new`
    // (gcd, positive denominator): feeding unreduced pairs through both and
    // comparing components pins the "always reduced" invariant externally.
    let Some(py) = Python::discover() else {
        skip_note("differential_rational::differential_rational_reduction_is_canonical");
        return;
    };
    let mut rng = TestRng::new(0x0F4C_0002);
    let mut exprs = Vec::new();
    let mut ours = Vec::new();
    for _ in 0..100 {
        // Deliberately unreduced inputs: a large common factor.
        let g = rng.next_u64() | 1;
        let n = (rng.next_u64() | 1).wrapping_mul(g);
        let d = (rng.next_u64() | 1).wrapping_mul(g);
        if d == 0 {
            continue; // wrapping product landed on zero — resample
        }
        let mag = tpt_mathr_arith::test_support::from_u64_limbs(&[n]);
        let num = if rng.chance(1, 2) {
            BigInt::from(mag).neg()
        } else {
            BigInt::from(mag)
        };
        let den = tpt_mathr_arith::test_support::from_u64_limbs(&[d]);

        let rat = BigRat::new(num.clone(), den.clone(), &P).expect("reduction");
        rat.assert_invariants();
        let (rn, rd) = render(&rat);

        let n_signed = num.to_str_radix(10, &P).unwrap();
        let d_signed = den.to_str_radix(10, &P).unwrap();
        exprs.push(format!(
            "str(Fraction({n_signed},{d_signed}).numerator) + '/' + str(Fraction({n_signed},{d_signed}).denominator)"
        ));
        ours.push(format!("{rn}/{rd}"));
    }
    let got = py.eval_bigint_batch(&exprs).expect("python batch eval");
    assert_eq!(got.len(), ours.len(), "batch lost lines");
    for (i, (line, want)) in got.iter().zip(ours.iter()).enumerate() {
        assert_eq!(line_value(line), *want, "reduction mismatch at case {i}");
    }
}
