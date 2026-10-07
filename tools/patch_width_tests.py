# Patch: width-agnostic test values (Limb = u64 or u32).
import re

p = "crates/tpt-mathr-arith/src/biguint.rs"
s = open(p, encoding="utf-8").read()

old = """        assert_eq!(bu(8).trailing_zeros(), Some(3));
        assert_eq!(BigUint::from_limbs(vec![0, 2]).trailing_zeros(), Some(65));
        assert_eq!(BigUint::zero().trailing_zeros(), None);"""
new = """        assert_eq!(bu(8).trailing_zeros(), Some(3));
        // 2^(2*LIMB_BITS + 1): trailing zero spans two zero limbs.
        let wide = BigUint::from(2_u8).shl(2 * LIMB_BITS as u64 + 1, &Progress::UNLIMITED).unwrap();
        assert_eq!(wide.trailing_zeros(), Some(2 * LIMB_BITS as u64 + 1));
        assert_eq!(BigUint::zero().trailing_zeros(), None);"""
assert old in s
s = s.replace(old, new)

old = """        assert_eq!(bu(256).bit_len(), 9);
        assert_eq!(BigUint::from_limbs(vec![0, 1]).bit_len(), 65);"""
new = """        assert_eq!(bu(256).bit_len(), 9);
        let just_over_one_limb = BigUint::from(1_u8).shl(LIMB_BITS as u64, &Progress::UNLIMITED).unwrap();
        assert_eq!(just_over_one_limb.bit_len(), LIMB_BITS as u64 + 1);"""
assert old in s
s = s.replace(old, new)

old = """        // Cross-limb shr trims the trailing zero limb: 2^64 >> 1 == 2^63.
        let big = BigUint::from_limbs(vec![0, 1]);
        assert_eq!(big.shr(1, &p).unwrap(), BigUint::from(1_u64 << 63));"""
new = """        // Cross-limb shr trims the trailing zero limb: 2^B >> 1 == 2^(B-1).
        let big = BigUint::from(1_u8).shl(LIMB_BITS as u64, &p).unwrap();
        assert_eq!(big.shr(1, &p).unwrap(), BigUint::from(1_u64 << (LIMB_BITS - 1)));"""
assert old in s
s = s.replace(old, new)

open(p, "w", encoding="utf-8", newline="\n").write(s)
print("biguint.rs patched")

p = "crates/tpt-mathr-arith/src/numtheory.rs"
s = open(p, encoding="utf-8").read()
old = """        let big = bu(2).pow(64, &p).unwrap();
        assert_eq!(big, BigUint::from_limbs(vec![0, 1]));"""
new = """        let big = bu(2).pow(64, &p).unwrap();
        assert_eq!(big, BigUint::from(1_u128 << 64));"""
assert old in s
s = s.replace(old, new)
open(p, "w", encoding="utf-8", newline="\n").write(s)
print("numtheory.rs patched")

p = "crates/tpt-mathr-arith/src/strategy.rs"
s = open(p, encoding="utf-8").read()
old = """        // (2^64)^2 = 2^128: cross-multiplication boundary case.
        let two_64 = BigUint::from_limbs(vec![0, 1]);"""
new = """        // (2^64)^2 = 2^128: cross-multiplication boundary case.
        let two_64 = BigUint::from(1_u128 << 64);"""
assert old in s
s = s.replace(old, new)
open(p, "w", encoding="utf-8", newline="\n").write(s)
print("strategy.rs patched")
