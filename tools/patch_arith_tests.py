# Patch: fix extgcd sign bug + test expectations.
import io

# 1. bigint.rs: non-negative gcd + test fixes
p = "crates/tpt-mathr-arith/src/bigint.rs"
s = open(p, encoding="utf-8").read()

old = """            old_r = r;
            r = next_r;
            old_s = s;
            s = next_s;
            old_t = t;
            t = next_t;
        }
        Ok(ExtendedGcd { gcd: old_r, x: old_s, y: old_t })"""
new = """            old_r = r;
            r = next_r;
            old_s = s;
            s = next_s;
            old_t = t;
            t = next_t;
        }
        // Truncated division keeps the dividend's sign in the remainders, so
        // negative operands can leave a negative gcd. Negating the whole
        // Bezout triple preserves a*x + b*y == g and restores g >= 0 (I6).
        if old_r.sign() == Sign::Negative {
            return Ok(ExtendedGcd {
                gcd: old_r.neg(),
                x: old_s.neg(),
                y: old_t.neg(),
            });
        }
        Ok(ExtendedGcd { gcd: old_r, x: old_s, y: old_t })"""
assert old in s
s = s.replace(old, new)

old = """        for (a, b) in [
            (5_i64, 3_i64),
            (-5, 3),
            (5, -3),
            (-5, -3),
            (-5, 5),
            (0, -7),
            (-7, 0),
            (i64::MIN, i64::MAX),
        ] {
            let sum = bi(a).add(&bi(b), &p).unwrap();
            assert_eq!(sum.to_i64().unwrap_or(i64::MAX), a.saturating_add(b), "add {a}+{b}");
            let diff = bi(a).checked_sub(&bi(b), &p).unwrap();
            assert_eq!(diff.to_i64().unwrap_or(i64::MAX), a.saturating_sub(b), "sub {a}-{b}");
        }
        // Exact value that does not fit i64 is still correct on magnitude:
        let big = bi(i64::MIN).add(&bi(-1), &p).unwrap();
        assert_eq!(big.sign(), Sign::Negative);
        assert_eq!(big.magnitude(), &bu(u64::MAX).add(&bu(1).shl(63, &p).unwrap(), &p).unwrap().add(&bu(1), &p).unwrap());"""
new = """        for (a, b) in [
            (5_i64, 3_i64),
            (-5, 3),
            (5, -3),
            (-5, -3),
            (-5, 5),
            (0, -7),
            (-7, 0),
        ] {
            let sum = bi(a).add(&bi(b), &p).unwrap();
            assert_eq!(sum.to_i64().unwrap(), a + b, "add {a}+{b}");
            let diff = bi(a).checked_sub(&bi(b), &p).unwrap();
            assert_eq!(diff.to_i64().unwrap(), a - b, "sub {a}-{b}");
        }
        // Out-of-i64-range results stay exact on magnitude:
        let big = bi(i64::MIN).add(&bi(-1), &p).unwrap();
        assert_eq!(big.sign(), Sign::Negative);
        assert_eq!(
            big.magnitude(),
            &BigUint::from(1_u64 << 63).add(&BigUint::one(), &p).unwrap(),
            "-(2^63 + 1)"
        );
        let diff = bi(i64::MIN).checked_sub(&bi(i64::MAX), &p).unwrap();
        assert_eq!(diff.sign(), Sign::Negative);
        assert_eq!(diff.magnitude(), &bu(u64::MAX), "-(2^64 - 1)");"""
assert old in s
s = s.replace(old, new)

old = """            assert_eq!(r.sign(), bi(a).sign(), "remainder sign for {a}/{b}");"""
new = """            assert!(
                r.is_zero() || r.sign() == bi(a).sign(),
                "remainder sign for {a}/{b}"
            );"""
assert old in s
s = s.replace(old, new)

old = """    fn limits_apply() {
        let tight = Progress::new(ResourceLimits::restrictive());
        // 2^64 squared is only 128 bits - inside the restrictive budget.
        let big = BigInt::from(u64::MAX);
        assert!(big.mul(&big, &tight).is_ok());
        // ~6400 bits - far beyond the 1024-bit restrictive budget.
        let err = big.pow(100, &tight).unwrap_err();
        assert!(err.is_resource_exhausted());
    }"""
# the current file may still have the original; try both
old2 = """    fn limits_apply() {
        let tight = Progress::new(ResourceLimits::restrictive());
        let big = BigInt::from(u64::MAX);
        let err = big.mul(&big, &tight).unwrap_err();
        assert!(err.is_resource_exhausted());
    }"""
new2 = """    fn limits_apply() {
        let tight = Progress::new(ResourceLimits::restrictive());
        // 2^64 squared is only 128 bits - inside the restrictive budget.
        let big = BigInt::from(u64::MAX);
        assert!(big.mul(&big, &tight).is_ok());
        // ~6400 bits - far beyond the 1024-bit restrictive budget.
        let err = big.pow(100, &tight).unwrap_err();
        assert!(err.is_resource_exhausted());
    }"""
if old in s:
    s = s.replace(old, new)
else:
    assert old2 in s, "limits_apply not found"
    s = s.replace(old2, new2)

open(p, "w", encoding="utf-8", newline="\n").write(s)
print("bigint.rs patched")

# 2. division.rs: exact-division expectation
p = "crates/tpt-mathr-arith/src/division.rs"
s = open(p, encoding="utf-8").read()
old = """        // (2^128 - 1) / (2^64 - 1): quotient 2^64+1 remainder 2^64 - 2.
        let u = BigUint::from(u128::MAX);
        let v = BigUint::from(u64::MAX);
        let (q, r) = div_rem(u.as_limbs(), v.as_limbs(), &p).unwrap();
        assert_eq!(value_of(&q), BigUint::from(1_u128).shl(64, &p).unwrap().add(&BigUint::one(), &p).unwrap());
        assert_eq!(value_of(&r), BigUint::from(u64::MAX).checked_sub(&BigUint::one(), &p).unwrap());"""
new = """        // (2^128 - 1) / (2^64 - 1): (2^64-1)(2^64+1) == 2^128-1 exactly,
        // so the quotient is 2^64+1 with a zero remainder.
        let u = BigUint::from(u128::MAX);
        let v = BigUint::from(u64::MAX);
        let (q, r) = div_rem(u.as_limbs(), v.as_limbs(), &p).unwrap();
        assert_eq!(
            value_of(&q),
            BigUint::from(1_u128).shl(64, &p).unwrap().add(&BigUint::one(), &p).unwrap()
        );
        assert!(r.is_empty(), "exact division must leave no remainder");"""
assert old in s
s = s.replace(old, new)
open(p, "w", encoding="utf-8", newline="\n").write(s)
print("division.rs patched")

# 3. numtheory.rs: limits + magnitude extgcd test
p = "crates/tpt-mathr-arith/src/numtheory.rs"
s = open(p, encoding="utf-8").read()
old = """        let tight = Progress::new(ResourceLimits::restrictive());
        let err = bu(2).pow(100, &tight).unwrap_err();
        assert!(err.is_resource_exhausted());"""
new = """        let tight = Progress::new(ResourceLimits::restrictive());
        // 2^100 fits the 1024-bit budget; 2^10000 does not.
        assert!(bu(2).pow(100, &tight).is_ok());
        let err = bu(2).pow(10_000, &tight).unwrap_err();
        assert!(err.is_resource_exhausted());"""
assert old in s
s = s.replace(old, new)

old = """    #[test]
    fn extended_gcd_placeholder_never_lies() {
        // The magnitude extended_gcd routes through BigInt::extended_gcd in
        // the final implementation; until then it must not fabricate a
        // Bezout identity. (Test pins the honest-failure contract.)
        let p = Progress::UNLIMITED;
        let res = panic::catch_unwind(|| bu(12).extended_gcd(&bu(18), &p));
        assert!(res.is_err(), "placeholder must not pretend to succeed");
    }"""
new = """    #[test]
    fn magnitude_extended_gcd_bezout_identity() {
        let p = Progress::UNLIMITED;
        for (a, b) in [(12_u64, 18_u64), (18, 12), (17, 5), (1, 1), (0, 7), (7, 0)] {
            let ext = bu(a).extended_gcd(&bu(b), &p).unwrap();
            assert_eq!(ext.gcd.sign(), crate::bigint::Sign::Positive, "gcd >= 0");
            let lhs = bu(a)
                .mul(ext.x.magnitude(), &p).unwrap()
                .add(&bu(b).mul(ext.y.magnitude(), &p).unwrap(), &p).unwrap();
            assert_eq!(lhs, ext.gcd.magnitude(), "a*x + b*y == gcd for ({a}, {b})");
        }
    }"""
assert old in s
s = s.replace(old, new)

old = """        assert_eq!(lhs, ext.gcd, "a*x + b*y == gcd");
    }

    use std::panic;"""
new = """        assert_eq!(lhs, ext.gcd, "a*x + b*y == gcd");
    }"""
assert old in s
s = s.replace(old, new)
open(p, "w", encoding="utf-8", newline="\n").write(s)
print("numtheory.rs patched")

# 4. radix.rs: limits test
p = "crates/tpt-mathr-arith/src/radix.rs"
s = open(p, encoding="utf-8").read()
old = """        let err = BigUint::from_str_radix(
            "115792089237316195423570985008687907853269984665640564039457584007913129639936",
            10,
            &tight,
        )
        .unwrap_err();
        assert!(err.is_resource_exhausted(), "2^128 must exceed the restrictive bit budget");"""
new = """        // 2^128 fits the 1024-bit restrictive budget:
        assert!(BigUint::from_str_radix(
            "115792089237316195423570985008687907853269984665640564039457584007913129639936",
            10,
            &tight,
        )
        .is_ok());
        // 2^2048 does not:
        let huge = alloc::format!("1{}", "0".repeat(2048));
        let err = BigUint::from_str_radix(&huge, 2, &tight).unwrap_err();
        assert!(err.is_resource_exhausted(), "2^2048 must exceed the bit budget");"""
assert old in s
s = s.replace(old, new)
open(p, "w", encoding="utf-8", newline="\n").write(s)
print("radix.rs patched")
