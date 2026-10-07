# Fix Toom3 signed evaluation types in strategy.rs.
p = "crates/tpt-mathr-arith/src/strategy.rs"
s = open(p, encoding="utf-8").read()

old = """        let a_neg1 = as_big(a0).add(&as_big(a2), progress)?.checked_sub(&as_big(a1), progress)?;
        let b_neg1 = as_big(b0).add(&as_big(b2), progress)?.checked_sub(&as_big(b1), progress)?;"""
new = """        let a_neg1 = as_big(a0)
            .add(&as_big(a2), progress)?
            .checked_sub(&as_big(a1), progress)
            .map(|v| crate::bigint::BigInt::from(v));
        let b_neg1 = as_big(b0)
            .add(&as_big(b2), progress)?
            .checked_sub(&as_big(b1), progress)
            .map(|v| crate::bigint::BigInt::from(v));"""
assert old in s
s = s.replace(old, new)

old = """        let v_neg1 = match (a_neg1, b_neg1) {
            (Ok(a), Ok(b)) => crate::bigint::BigInt::from(a),
            (a, b) => {
                // At least one is negative: do the multiplication in BigInt.
                let sa = signed_value(a, as_big(a0).add(&as_big(a2), progress)?, as_big(a1));
                let sb = signed_value(b, as_big(b0).add(&as_big(b2), progress)?, as_big(b1));
                sa.mul(&sb, progress)?
            }
        };"""
new = """        let v_neg1: crate::bigint::BigInt = match (a_neg1, b_neg1) {
            (Ok(a), Ok(b)) => a.mul(&b, progress)?,
            (a, b) => {
                // At least one evaluation is negative: multiply in BigInt,
                // reconstructing each signed value from magnitude and sign.
                let signed = |v: Result<crate::bigint::BigInt, Error>,
                              pos: BigUint,
                              neg: BigUint| match v {
                    Ok(x) => x,
                    Err(_) => crate::bigint::BigInt::from(neg)
                        .checked_sub(&crate::bigint::BigInt::from(pos), &Progress::UNLIMITED)
                        .expect("signed evaluation allocates"),
                };
                let sa = signed(a, as_big(a0).add(&as_big(a2), progress)?, as_big(a1));
                let sb = signed(b, as_big(b0).add(&as_big(b2), progress)?, as_big(b1));
                sa.mul(&sb, progress)?
            }
        };"""
assert old in s
s = s.replace(old, new)

# remove the now-unused signed_value helper
import re
pattern = re.compile(r"/// Signed view of a product-side value.*?\n\}\n\n", re.DOTALL)
s, n = pattern.subn("", s, count=1)
assert n == 1

open(p, "w", encoding="utf-8", newline="\n").write(s)
print("ok")
