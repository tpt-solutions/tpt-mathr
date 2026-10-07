// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Differential tests vs Python `int` (spec §30, §45; dev-only).
//!
//! Enabled with `--features differential-tests`; requires a Python 3
//! interpreter on PATH. Python is the *reference implementation*, never a
//! runtime dependency. Without the feature — or without an interpreter —
//! these tests skip with a printed note rather than fail: the in-tree unit
//! and property suites are the always-on floor.
//!
//! Phase 2 fills the real corpora (large generated operands for add, sub,
//! mul, divrem, gcd, pow, parse/format). The tests below validate the
//! harness itself so a broken bridge is caught before it can silently
//! "pass" a corpus.

#[path = "common/mod.rs"]
mod common;

use common::{Python, skip_note};

#[test]
fn harness_python_bridge_roundtrips_known_values() {
    let Some(py) = Python::discover() else {
        skip_note("differential::harness_python_bridge_roundtrips_known_values");
        return;
    };

    // Known-answer checks for the bridge itself (floor(1000*log10(2)) + 1 = 302 digits).
    assert_eq!(py.eval_bigint("2**1000").unwrap().len(), 302);
    assert_eq!(
        py.eval_bigint("(1 << 64) - 1").unwrap(),
        "18446744073709551615"
    );
    assert_eq!(
        py.eval_bigint("-(2**128)").unwrap(),
        "-340282366920938463463374607431768211456"
    );
    assert_eq!(py.eval_bigint("0b1011 * 0o17 + 0xff").unwrap(), "420");
}

#[test]
fn harness_batch_protocol_preserves_order() {
    let Some(py) = Python::discover() else {
        skip_note("differential::harness_batch_protocol_preserves_order");
        return;
    };

    let exprs: Vec<String> = (0..50).map(|i| format!("({i} ** 7) + 3")).collect();
    let got = py.eval_bigint_batch(&exprs).unwrap();
    assert_eq!(got.len(), exprs.len(), "batch protocol lost lines");
    for (i, line) in got.iter().enumerate() {
        let want = format!("{}", u64::pow(i as u64, 7) + 3);
        assert_eq!(line, &want, "batch order violated at {i}");
    }
}

#[test]
fn harness_rejects_broken_expressions_with_error() {
    let Some(py) = Python::discover() else {
        skip_note("differential::harness_rejects_broken_expressions_with_error");
        return;
    };

    assert!(py.eval_bigint("1 +").is_err(), "syntax error must surface");
    assert!(
        py.eval_bigint("1 // 0").is_err(),
        "runtime error must surface with stderr"
    );
}
