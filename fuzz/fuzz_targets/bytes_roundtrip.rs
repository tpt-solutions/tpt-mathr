// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fuzz: byte serialisation round trip (from_bytes_le ∘ to_bytes_le ==
//! identity) and decimal parse/format consistency on arbitrary input.

#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_mathr_arith::{BigUint, Progress};

fuzz_target!(|data: &[u8]| {
    const P: Progress = Progress::UNLIMITED;
    let v = BigUint::from_bytes_le(data, P).expect("bytes to value");
    let round = BigUint::from_bytes_le(&v.to_bytes_le(), P).expect("value to bytes to value");
    assert_eq!(round, v, "byte round trip");

    // Decimal rendering parses back to the same value.
    let decimal = v.to_str_radix(10, P).expect("decimal render");
    let reparsed = BigUint::from_str_radix(&decimal, 10, P).expect("decimal reparse");
    assert_eq!(reparsed, v);
});
