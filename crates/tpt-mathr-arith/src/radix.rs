// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Radix parsing and formatting for radix 2–36 (invariant I4).
//!
//! Both directions are chunked at the largest radix-power that fits a limb
//! (`radix^k`), giving O(n²/k) limb operations — the straightforward
//! school conversion. Divide-and-conquer conversion (subquadratic) is a
//! Phase 3 optimisation once benchmarks justify it; the *output* of this
//! module is already canonical: no leading zeros, lowercase `a–z` (I4).
//!
//! Parsing is an untrusted-input entry point, so it takes `&Progress` and
//! enforces `max_integer_bits` as the value grows (I9).

use alloc::string::String;
use alloc::vec::Vec;

use tpt_mathr_base::alloc_helpers;
use tpt_mathr_base::error::{Error, ParseError};

use crate::bigint::{BigInt, Sign};
use crate::biguint::BigUint;
use crate::division;
use crate::limb::{DoubleLimb, LIMB_MAX, Limb};
use crate::progress::Progress;

/// Smallest supported radix.
pub const MIN_RADIX: u32 = 2;
/// Largest supported radix (alphabet `0-9`, `a-z`).
pub const MAX_RADIX: u32 = 36;

/// Cancellation check cadence for the conversion loops.
const CANCEL_EVERY: u64 = 128;

fn check_radix(radix: u32) -> Result<(), Error> {
    if (MIN_RADIX..=MAX_RADIX).contains(&radix) {
        Ok(())
    } else {
        Err(Error::InvalidRadix {
            radix: radix.clamp(0, u8::MAX as u32) as u8,
        })
    }
}

/// Digit value for a byte in the given radix, or `None`.
fn digit_value(b: u8, radix: u32) -> Option<u32> {
    let v = match b {
        b'0'..=b'9' => u32::from(b - b'0'),
        b'a'..=b'z' => u32::from(b - b'a') + 10,
        b'A'..=b'Z' => u32::from(b - b'A') + 10,
        _ => return None,
    };
    if v < radix { Some(v) } else { None }
}

/// Largest digit-chunk length `k` with `radix^k <= LIMB_MAX`, and the
/// corresponding `radix^k`.
fn chunk_params(radix: u32) -> (usize, Limb) {
    let r = DoubleLimb::from(radix);
    let mut digits = 0_usize;
    let mut power: DoubleLimb = 1;
    loop {
        let next = power * r;
        if next > LIMB_MAX as DoubleLimb || next < power {
            break;
        }
        power = next;
        digits += 1;
    }
    (digits, power as Limb)
}

impl BigUint {
    /// Parse a non-negative integer in `radix` (underscores are *not*
    /// accepted; input is exactly the digit string).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidRadix`], [`Error::Parse`] (empty or invalid
    /// character, position reported), [`Error::ResourceExhausted`] when the
    /// value exceeds `max_integer_bits`, plus allocation errors.
    pub fn from_str_radix(s: &str, radix: u32, progress: &Progress) -> Result<Self, Error> {
        check_radix(radix)?;
        let bytes = s.as_bytes();
        if bytes.is_empty() {
            return Err(ParseError::empty().into());
        }
        let (chunk_digits, chunk_base) = chunk_params(radix);

        // Leading partial chunk so every later multiply is by chunk_base.
        let first = bytes.len() % chunk_digits.max(1);
        let mut acc = BigUint::zero();
        let mut pos = 0_usize;
        for &b in &bytes[..first] {
            let d =
                digit_value(b, radix).ok_or(Error::Parse(ParseError::invalid_character(pos)))?;
            acc = acc
                .mul(&BigUint::from(radix), progress)?
                .add(&BigUint::from(u64::from(d)), progress)?;
            progress.check_integer_bits(acc.bit_len() + 1)?;
            pos += 1;
        }

        while pos < bytes.len() {
            progress.check_cancelled()?;
            // One full chunk.
            let mut chunk: u64 = 0;
            for &b in &bytes[pos..pos + chunk_digits] {
                let d = digit_value(b, radix)
                    .ok_or(Error::Parse(ParseError::invalid_character(pos)))?;
                chunk = chunk * u64::from(radix) + u64::from(d);
                pos += 1;
            }
            acc = acc
                .mul(&BigUint::from(chunk_base), progress)?
                .add(&BigUint::from(chunk), progress)?;
            progress.check_integer_bits(acc.bit_len() + 1)?;
        }
        acc.assert_invariants();
        Ok(acc)
    }

    /// Render in `radix`, lowercase digits, no leading zeros (I4).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidRadix`], allocation, limits, cancellation.
    ///
    /// # Panics
    ///
    /// Only if an internal invariant breaks while assembling the output
    /// (UTF-8 by construction from the ASCII digit table).
    pub fn to_str_radix(&self, radix: u32, progress: &Progress) -> Result<String, Error> {
        check_radix(radix)?;
        if self.is_zero() {
            return Ok(String::from("0"));
        }
        let (chunk_digits, chunk_base) = chunk_params(radix);
        let mut chunks: Vec<Limb> = Vec::new();
        let mut rest: Vec<Limb> = self.as_limbs().to_vec();
        let mut iterations = 0_u64;
        while !rest.is_empty() {
            iterations += 1;
            if iterations % CANCEL_EVERY == 0 {
                progress.check_cancelled()?;
            }
            let (q, r) = division::div_rem_small(&rest, chunk_base)?;
            chunks.push(r);
            rest = q;
        }

        // Upper bound: every chunk is chunk_digits characters.
        let mut out =
            alloc_helpers::try_with_capacity::<u8>(chunks.len() * chunk_digits.max(1) + 1)?;
        // Chunks are little-endian; render high to low, zero-padding all but
        // the most significant chunk.
        let mut first = true;
        for &chunk in chunks.iter().rev() {
            let mut digits = [0_u8; 64]; // chunk_digits <= 64 by construction
            let mut n = 0_usize;
            let mut v = crate::limb::limb_to_u64(chunk);
            loop {
                digits[n] = DIGITS[(v % u64::from(radix)) as usize];
                n += 1;
                v /= u64::from(radix);
                if v == 0 {
                    break;
                }
            }
            if first {
                first = false;
            } else {
                // Zero-pad every chunk except the most significant one so
                // digit positions stay aligned.
                #[allow(
                    clippy::same_item_push,
                    reason = "padding is definitionally a run of identical bytes"
                )]
                for _ in n..chunk_digits {
                    out.push(b'0');
                }
            }
            for d in digits[..n].iter().rev() {
                out.push(*d);
            }
        }
        // ASCII by construction of DIGITS.
        Ok(String::from_utf8(out).expect("radix digits are ASCII"))
    }
}

const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";

impl BigInt {
    /// Parse a signed integer in `radix`: optional leading `-` or `+`, then
    /// digits. Canonical zero for all-zero input regardless of sign (I3).
    ///
    /// # Errors
    ///
    /// As [`BigUint::from_str_radix`].
    pub fn from_str_radix(s: &str, radix: u32, progress: &Progress) -> Result<Self, Error> {
        if let Some(rest) = s.strip_prefix('-') {
            if rest.is_empty() {
                return Err(ParseError::empty().into());
            }
            let mag = BigUint::from_str_radix(rest, radix, progress)?;
            return Ok(Self::from_sign_and_magnitude(Sign::Negative, mag));
        }
        let rest = s.strip_prefix('+').unwrap_or(s);
        if rest.is_empty() {
            return Err(ParseError::empty().into());
        }
        Ok(Self::from(BigUint::from_str_radix(rest, radix, progress)?))
    }

    /// Render in `radix` with a leading `-` for negative values (I4).
    ///
    /// # Errors
    ///
    /// As [`BigUint::to_str_radix`].
    pub fn to_str_radix(&self, radix: u32, progress: &Progress) -> Result<String, Error> {
        let mag = self.magnitude().to_str_radix(radix, progress)?;
        if self.sign() == Sign::Negative && !self.is_zero() {
            let mut s = String::with_capacity(mag.len() + 1);
            s.push('-');
            s.push_str(&mag);
            Ok(s)
        } else {
            Ok(mag)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use tpt_mathr_base::error::ParseErrorKind;

    fn bu_from(s: &str, radix: u32) -> BigUint {
        BigUint::from_str_radix(s, radix, &Progress::UNLIMITED).unwrap()
    }

    #[test]
    fn parse_known_values() {
        assert_eq!(bu_from("0", 10), BigUint::zero());
        assert_eq!(bu_from("255", 10), BigUint::from(255_u8));
        assert_eq!(bu_from("ff", 16), BigUint::from(255_u8));
        assert_eq!(bu_from("FF", 16), BigUint::from(255_u8));
        assert_eq!(bu_from("101", 2), BigUint::from(5_u8));
        assert_eq!(bu_from("zz", 36), BigUint::from(1_295_u32));
    }

    #[test]
    fn parse_rejects_garbage() {
        for (s, radix) in [
            ("", 10),
            ("12x4", 10),
            ("2", 2),
            ("-", 10),
            ("+", 10),
            ("--5", 10),
        ] {
            let err = BigUint::from_str_radix(s, radix, &Progress::UNLIMITED).unwrap_err();
            assert!(
                matches!(err, Error::Parse(_)),
                "expected Parse error for {s:?}/{radix}, got {err}"
            );
        }
        let err = BigUint::from_str_radix("12x4", 10, &Progress::UNLIMITED).unwrap_err();
        match err {
            Error::Parse(pe) => {
                assert_eq!(pe.kind, ParseErrorKind::InvalidCharacter);
                assert_eq!(pe.position, Some(2));
            }
            other => panic!("wrong error {other}"),
        }
        let err = BigUint::from_str_radix("1", 1, &Progress::UNLIMITED).unwrap_err();
        assert_eq!(err, Error::InvalidRadix { radix: 1 });
        let err = BigUint::from_str_radix("1", 37, &Progress::UNLIMITED).unwrap_err();
        assert_eq!(err, Error::InvalidRadix { radix: 37 });
    }

    #[test]
    fn round_trip_all_radices_i4() {
        let p = Progress::UNLIMITED;
        let values = [
            BigUint::zero(),
            BigUint::one(),
            BigUint::from(255_u8),
            BigUint::from(u64::MAX),
            BigUint::from(u64::MAX)
                .mul(&BigUint::from(u64::MAX), &p)
                .unwrap(),
            BigUint::from(1_u128).shl(300, &p).unwrap(),
        ];
        for v in values {
            for radix in MIN_RADIX..=MAX_RADIX {
                let s = v.to_str_radix(radix, &p).unwrap();
                assert!(!s.starts_with('0') || s == "0", "leading zero in {s}");
                assert!(
                    s.bytes().all(|b| digit_value(b, radix).is_some()),
                    "invalid digit for radix {radix} in {s}"
                );
                let back = BigUint::from_str_radix(&s, radix, &p).unwrap();
                assert_eq!(back, v, "round trip failed radix {radix}: {s}");
            }
        }
    }

    #[test]
    fn chunk_boundaries_are_exact() {
        // Values around the per-limb chunk size exercise the partial-chunk
        // and padding paths.
        let p = Progress::UNLIMITED;
        let mut v = BigUint::one();
        for _ in 0..200 {
            for radix in [2_u32, 3, 10, 16, 36] {
                let s = v.to_str_radix(radix, &p).unwrap();
                assert_eq!(BigUint::from_str_radix(&s, radix, &p).unwrap(), v);
            }
            v = v
                .mul(&BigUint::from(7_u8), &p)
                .unwrap()
                .add(&BigUint::from(3_u8), &p)
                .unwrap();
        }
    }

    #[test]
    fn signed_parse_and_format() {
        let p = Progress::UNLIMITED;
        assert_eq!(
            BigInt::from_str_radix("-255", 10, &p).unwrap(),
            BigInt::from(-255_i64)
        );
        assert_eq!(
            BigInt::from_str_radix("+7", 10, &p).unwrap(),
            BigInt::from(7_i64)
        );
        assert_eq!(
            BigInt::from_str_radix("-0", 10, &p).unwrap(),
            BigInt::zero(),
            "canonical zero from signed parse"
        );
        assert_eq!(
            BigInt::from(-123_456_789_i64).to_str_radix(16, &p).unwrap(),
            "-75bcd15"
        );
        assert_eq!(BigInt::zero().to_str_radix(2, &p).unwrap(), "0");
        assert!(BigInt::from_str_radix("-", 10, &p).is_err());
        assert!(BigInt::from_str_radix("+", 10, &p).is_err());
    }

    #[test]
    fn limits_apply_to_parsing() {
        let tight = Progress::new(tpt_mathr_base::ResourceLimits::restrictive());
        // 2^128 fits the 1024-bit restrictive budget:
        assert!(
            BigUint::from_str_radix(
                "115792089237316195423570985008687907853269984665640564039457584007913129639936",
                10,
                &tight,
            )
            .is_ok()
        );
        // 2^2048 does not:
        let huge = alloc::format!("1{}", "0".repeat(2048));
        let err = BigUint::from_str_radix(&huge, 2, &tight).unwrap_err();
        assert!(
            err.is_resource_exhausted(),
            "2^2048 must exceed the bit budget"
        );
    }

    #[test]
    fn display_via_radix() {
        assert_eq!(format!("{}", BigUint::from(255_u8)), "255");
        assert_eq!(format!("{}", BigInt::from(-255_i64)), "-255");
    }
}
