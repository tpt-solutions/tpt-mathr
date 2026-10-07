// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Differential corpus: `tpt-mathr-arith` vs Python `int` on randomly
//! generated large operands (spec §30, §45; todo Phase 2).
//!
//! Run with:
//!
//! ```text
//! cargo test -p tpt-mathr-arith --features differential-tests,testing
//! ```
//!
//! Python is the *reference implementation* (never a runtime dependency).
//! Each case builds a Python expression over signed hex literals, ships a
//! whole corpus through one interpreter process, and compares the decimal
//! renderings against the engine's `to_str_radix(10)`. Seeds print on
//! failure for exact reproduction.

#[path = "common/mod.rs"]
mod common;

use common::{Python, skip_note};
use tpt_mathr_arith::test_support::from_u64_limbs;
use tpt_mathr_arith::{BigInt, BigUint, Progress};
use tpt_mathr_base::testing::TestRng;

const P: Progress = Progress::UNLIMITED;

/// Random u64 limb vector (possibly empty = zero).
fn rand_limbs(rng: &mut TestRng, max_limbs: usize) -> Vec<u64> {
    let n = rng.below_usize(max_limbs + 1);
    (0..n).map(|_| rng.next_u64()).collect()
}

/// Random `BigUint` plus its signed hex literal for Python.
fn rand_hex_value(rng: &mut TestRng, max_limbs: usize) -> (BigUint, String) {
    let limbs = rand_limbs(rng, max_limbs);
    let v = from_u64_limbs(&limbs);
    // Canonical lowercase hex, no leading zeros (I4) — directly usable as a
    // Python literal with the 0x prefix.
    let hex = format!("0x{}", v.to_str_radix(16, &P).unwrap());
    (v, hex)
}

fn rand_signed_hex(rng: &mut TestRng, max_limbs: usize) -> (BigInt, String) {
    let (v, hex) = rand_hex_value(rng, max_limbs);
    let b = BigInt::from(v);
    if rng.chance(1, 2) {
        (b.neg(), format!("(-{hex})"))
    } else {
        (b, hex)
    }
}

/// Edge values with exact hex forms.
fn edge_cases() -> Vec<(BigInt, String)> {
    let mut v = Vec::new();
    for raw in [
        "0x0",
        "0x1",
        "0xff",
        "0x100",
        "0xffffffffffffffff",
        "0x10000000000000000",
        "0xffffffffffffffffffffffffffffffff",
        "0x80000000000000000000000000000000",
        "0x7fffffffffffffffffffffffffffffff",
    ] {
        // Our parser takes bare digits; the Python literal keeps the 0x.
        let digits = &raw[2..];
        let parsed = BigInt::from_str_radix(digits, 16, &P).unwrap();
        v.push((parsed.clone(), raw.to_string()));
        v.push((parsed.neg(), format!("(-{raw})")));
    }
    v
}

#[test]
fn differential_arithmetic_corpus_vs_python() {
    let Some(py) = Python::discover() else {
        skip_note("differential_bigint::differential_arithmetic_corpus_vs_python");
        return;
    };
    let mut rng = TestRng::new(0x0D1F_0001);
    let mut cases: Vec<(String, String)> = Vec::new(); // (python expr, expected)

    // --- add / sub / mul ------------------------------------------------
    for _ in 0..250 {
        let (a, ha) = rand_signed_hex(&mut rng, 20);
        let (b, hb) = rand_signed_hex(&mut rng, 20);
        cases.push((
            format!("{ha} + {hb}"),
            a.add(&b, &P).unwrap().to_str_radix(10, &P).unwrap(),
        ));
        cases.push((
            format!("{ha} - {hb}"),
            a.checked_sub(&b, &P).unwrap().to_str_radix(10, &P).unwrap(),
        ));
        cases.push((
            format!("{ha} * {hb}"),
            a.mul(&b, &P).unwrap().to_str_radix(10, &P).unwrap(),
        ));
    }
    for (a, ha) in edge_cases() {
        for (b, hb) in edge_cases() {
            cases.push((
                format!("{ha} + {hb}"),
                a.add(&b, &P).unwrap().to_str_radix(10, &P).unwrap(),
            ));
            cases.push((
                format!("{ha} * {hb}"),
                a.mul(&b, &P).unwrap().to_str_radix(10, &P).unwrap(),
            ));
        }
    }

    // --- truncated divrem (Python floored -> engine truncated) -----------
    let divrem_lambda = "(lambda a,b: (lambda q,r: (str(q+1), str(r-b)) if r != 0 and (a < 0) != (b < 0) else (str(q), str(r)))(*divmod(a,b)))";
    for _ in 0..120 {
        let (a, ha) = rand_signed_hex(&mut rng, 24);
        let (b, hb) = rand_signed_hex(&mut rng, 10);
        if b.is_zero() {
            continue;
        }
        let (q, r) = a.div_rem(&b, &P).unwrap();
        cases.push((
            format!("{divrem_lambda}({ha},{hb})"),
            format!(
                "('{}', '{}')",
                q.to_str_radix(10, &P).unwrap(),
                r.to_str_radix(10, &P).unwrap()
            ),
        ));
    }

    // --- gcd (math.gcd is sign-agnostic, matching engine semantics) ------
    for _ in 0..80 {
        let (a, ha) = rand_signed_hex(&mut rng, 16);
        let (b, hb) = rand_signed_hex(&mut rng, 16);
        let g = a.gcd(&b, &P).unwrap();
        cases.push((
            format!("math.gcd({ha},{hb})"),
            g.to_str_radix(10, &P).unwrap(),
        ));
    }

    // --- pow --------------------------------------------------------------
    for _ in 0..60 {
        let (a, ha) = rand_signed_hex(&mut rng, 4);
        let e = rng.below(25);
        let p = a.pow(e, &P).unwrap();
        cases.push((format!("{ha} ** {e}"), p.to_str_radix(10, &P).unwrap()));
    }

    // --- isqrt (non-negative only) ----------------------------------------
    for _ in 0..60 {
        let (a, ha) = rand_hex_value(&mut rng, 20);
        let s = a.isqrt(&P).unwrap();
        cases.push((format!("math.isqrt({ha})"), s.to_str_radix(10, &P).unwrap()));
    }

    // Ship the corpus in one batch and compare in order.
    let exprs: Vec<String> = cases.iter().map(|(e, _)| e.clone()).collect();
    let got = py.eval_bigint_batch(&exprs).expect("python batch eval");
    assert_eq!(
        got.len(),
        cases.len(),
        "python returned the wrong number of lines"
    );

    for (i, ((expr, want), line)) in cases.iter().zip(got.iter()).enumerate() {
        // Python prints tuples with spaces: "('q', 'r')" vs our build.
        let want_norm = want.replace(", ", ",");
        let got_norm = line.replace(", ", ",");
        assert_eq!(
            got_norm, want_norm,
            "case {i} mismatch\n  expr: {expr}\n  python: {line}\n  engine: {want}\n  seed: 0x0D1F_0001"
        );
    }
}

#[test]
fn differential_decimal_round_trip_vs_python() {
    let Some(py) = Python::discover() else {
        skip_note("differential_bigint::differential_decimal_round_trip_vs_python");
        return;
    };
    let mut rng = TestRng::new(0x0D1F_0002);
    let mut exprs = Vec::new();
    let mut ours = Vec::new();
    for _ in 0..120 {
        let (a, ha) = rand_signed_hex(&mut rng, 30);
        let decimal = a.to_str_radix(10, &P).unwrap();
        // Python re-parses our decimal rendering; equality with the hex
        // literal proves both directions.
        exprs.push(format!("str(int(\"{decimal}\")) == str({ha})"));
        ours.push(decimal);
    }
    let got = py.eval_bigint_batch(&exprs).expect("python batch eval");
    for (i, line) in got.iter().enumerate() {
        assert_eq!(line, "True", "round trip failed for {}", ours[i]);
    }
}

#[test]
fn differential_hex_round_trip_vs_python() {
    let Some(py) = Python::discover() else {
        skip_note("differential_bigint::differential_hex_round_trip_vs_python");
        return;
    };
    let mut rng = TestRng::new(0x0D1F_0003);
    let mut exprs = Vec::new();
    let mut ours = Vec::new();
    for _ in 0..120 {
        let (a, _) = rand_signed_hex(&mut rng, 30);
        let hex = a.magnitude().to_str_radix(16, &P).unwrap();
        let decimal = a.to_str_radix(10, &P).unwrap();
        // hex(…) yields 0x… / -0x…; compare against our magnitude rendering
        // (lowercase, no leading zeros — I4) with the sign reattached.
        exprs.push(format!("hex(int(\"{decimal}\"))"));
        ours.push(if a.is_zero() {
            "0x0".to_string()
        } else if a.sign() == tpt_mathr_arith::Sign::Negative {
            format!("-0x{hex}")
        } else {
            format!("0x{hex}")
        });
    }
    let got = py.eval_bigint_batch(&exprs).expect("python batch eval");
    for (i, line) in got.iter().enumerate() {
        assert_eq!(line, &ours[i], "hex rendering mismatch case {i}");
    }
}
