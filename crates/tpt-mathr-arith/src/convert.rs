// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Conversions between engine integers and Rust primitives.
//!
//! Widening conversions are total (`From`). Narrowing conversions are
//! fallible (`TryFrom`) and report [`Error::Overflow`] — never a wrapped
//! or truncated value (spec §33).

use alloc::vec::Vec;

use tpt_mathr_base::error::Error;

use crate::bigint::{BigInt, Sign};
use crate::biguint::BigUint;
use crate::limb::{LIMB_BITS, Limb};
use crate::progress::Progress;

macro_rules! try_from_biguint {
    ($($t:ty),*) => {$(
        impl TryFrom<&BigUint> for $t {
            type Error = Error;
            fn try_from(v: &BigUint) -> Result<Self, Error> {
                let wide = v.to_u64()?;
                <$t>::try_from(wide).map_err(|_| Error::Overflow)
            }
        }
        impl TryFrom<BigUint> for $t {
            type Error = Error;
            fn try_from(v: BigUint) -> Result<Self, Error> {
                <$t>::try_from(&v)
            }
        }
    )*};
}
try_from_biguint!(u8, u16, u32, u64, u128, usize);

macro_rules! try_from_bigint_signed {
    ($($t:ty),*) => {$(
        impl TryFrom<&BigInt> for $t {
            type Error = Error;
            fn try_from(v: &BigInt) -> Result<Self, Error> {
                let wide = v.to_i64()?;
                <$t>::try_from(wide).map_err(|_| Error::Overflow)
            }
        }
        impl TryFrom<BigInt> for $t {
            type Error = Error;
            fn try_from(v: BigInt) -> Result<Self, Error> {
                <$t>::try_from(&v)
            }
        }
    )*};
}
try_from_bigint_signed!(i8, i16, i32, i64, i128, isize);

macro_rules! try_from_bigint_unsigned {
    ($($t:ty),*) => {$(
        impl TryFrom<&BigInt> for $t {
            type Error = Error;
            fn try_from(v: &BigInt) -> Result<Self, Error> {
                if v.sign() == Sign::Negative {
                    return Err(Error::Overflow);
                }
                <$t>::try_from(v.magnitude()).map_err(|_| Error::Overflow)
            }
        }
        impl TryFrom<BigInt> for $t {
            type Error = Error;
            fn try_from(v: BigInt) -> Result<Self, Error> {
                <$t>::try_from(&v)
            }
        }
    )*};
}
try_from_bigint_unsigned!(u8, u16, u32, u64, u128, usize);

impl BigUint {
    /// Little-endian bytes, minimal length (no trailing zero bytes).
    /// Empty for zero. Useful for hashing, serialisation, and FFI bulk
    /// paths; the byte order is fixed API.
    #[must_use]
    pub fn to_bytes_le(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.as_limbs().len() * LIMB_BITS / 8);
        for limb in self.as_limbs() {
            out.extend_from_slice(&limb.to_le_bytes());
        }
        while out.last() == Some(&0) {
            out.pop();
        }
        out
    }

    /// Construct from little-endian bytes (any length; leading zero bytes
    /// are tolerated and normalised away).
    ///
    /// # Errors
    ///
    /// [`Error::ResourceExhausted`] under `progress` bit limits.
    pub fn from_bytes_le(bytes: &[u8], progress: &Progress) -> Result<Self, Error> {
        let limb_bytes = LIMB_BITS / 8;
        let n_limbs = bytes.len().div_ceil(limb_bytes);
        progress.check_memory(n_limbs as u64 * limb_bytes as u64)?;
        progress.check_integer_bits(bytes.len() as u64 * 8 + 1)?;
        let mut limbs: Vec<Limb> = Vec::with_capacity(n_limbs);
        for chunk in bytes.chunks(limb_bytes) {
            let mut limb = 0_u64;
            for (i, &b) in chunk.iter().enumerate() {
                limb |= u64::from(b) << (i * 8);
            }
            limbs.push(limb as Limb);
        }
        Ok(Self::from_limbs(limbs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrowing_reports_overflow_never_wraps() {
        assert_eq!(u8::try_from(&BigUint::from(255_u8)).unwrap(), 255);
        assert_eq!(
            u8::try_from(&BigUint::from(256_u16)).unwrap_err(),
            Error::Overflow
        );
        assert_eq!(i8::try_from(&BigInt::from(127_i64)).unwrap(), 127);
        assert_eq!(
            i8::try_from(&BigInt::from(128_i64)).unwrap_err(),
            Error::Overflow
        );
        assert_eq!(i8::try_from(&BigInt::from(-128_i64)).unwrap(), -128);
        assert_eq!(
            i8::try_from(&BigInt::from(-129_i64)).unwrap_err(),
            Error::Overflow
        );
        assert_eq!(
            u8::try_from(&BigInt::from(-1_i64)).unwrap_err(),
            Error::Overflow
        );
    }

    #[test]
    fn widening_is_total() {
        assert_eq!(BigUint::from(7_u8), BigUint::from(7_u64));
        assert_eq!(BigUint::from(u128::MAX).to_u64(), Err(Error::Overflow));
        assert_eq!(BigInt::from(-7_i8), BigInt::from(-7_i64));
        assert_eq!(BigInt::from(i64::MIN).sign(), Sign::Negative);
    }

    #[test]
    fn bytes_round_trip() {
        let p = Progress::UNLIMITED;
        for v in [0_u64, 1, 255, 256, u64::MAX] {
            let n = BigUint::from(v);
            let bytes = n.to_bytes_le();
            assert_eq!(BigUint::from_bytes_le(&bytes, &p).unwrap(), n);
        }
        let big = BigUint::from(1_u128).shl(200, &p).unwrap();
        assert_eq!(BigUint::from_bytes_le(&big.to_bytes_le(), &p).unwrap(), big);
        assert_eq!(BigUint::zero().to_bytes_le(), Vec::<u8>::new());
        assert_eq!(
            BigUint::from_bytes_le(&[0, 0, 0], &p).unwrap(),
            BigUint::zero()
        );
    }

    #[test]
    fn from_bytes_honours_limits() {
        let tight = Progress::new(tpt_mathr_base::ResourceLimits::restrictive());
        let bytes = [0xFF_u8; 4096];
        let err = BigUint::from_bytes_le(&bytes, &tight).unwrap_err();
        assert!(err.is_resource_exhausted());
    }
}
