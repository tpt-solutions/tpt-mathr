// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fuzz: radix parsing must never panic, never accept garbage, and always
//! round-trip through formatting (I4) for every accepted input.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    let radix = 2 + (data[0] % 35) as u32; // 2..=36
    // The rest is the candidate digit string (ASCII-filtered to keep the
    // corpus focused on digit-shape bugs rather than encoding panics).
    let text: String = data[1..]
        .iter()
        .filter(|b| b.is_ascii_alphanumeric())
        .map(|b| *b as char)
        .collect();

    let progress = tpt_mathr_arith::Progress::UNLIMITED;
    if let Ok(v) = tpt_mathr_arith::BigUint::from_str_radix(&text, radix, progress) {
        // Accepted inputs must round-trip exactly.
        let rendered = v.to_str_radix(radix, progress).expect("format of accepted value");
        let reparsed = tpt_mathr_arith::BigUint::from_str_radix(&rendered, radix, progress)
            .expect("canonical rendering reparses");
        assert_eq!(reparsed, v, "round trip failed for {text:?} radix {radix}");
    }
});
