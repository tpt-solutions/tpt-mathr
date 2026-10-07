# Arithmetic invariants

Spec section 32 requires every invariant below to be documented and tested.
`tpt-mathr-arith` provides a debug/test-only invariant checker
(`assert_invariants`) that validates the full list; property tests assert
that no public operation breaks it. This file defines the invariants
normatively; implementations must reference them by name in tests.

## I1 - `BigUint` limb normalisation

The limb vector of a `BigUint` is little-endian and contains **no trailing
zero limbs**. The most significant limb, if any, is non-zero.

## I2 - Canonical zero

Zero has exactly one representation: the empty limb vector. There is no
"negative zero", no "zero with stale limbs", no sign-carrying zero.

## I3 - `BigInt` canonical sign

A zero-magnitude `BigInt` always carries sign `Positive`; `(-0)` cannot be
constructed or arise from any operation.

## I4 - Radix-string round trip

For every radix `2..=36`, `parse_radix(to_radix_string(x, r), r) == x`, and
`to_radix_string` never emits leading zeros, a sign for `BigUint`, or
characters outside the radix alphabet (`0-9`, then `a-z`); parsing accepts
upper and lower case.

## I5 - Division identity

For all `BigInt` `a`, `b != 0`: `a == b * trunc(a / b) + rem(a / b)` with
`sign(rem) == sign(a)` and `|rem| < |b|` (truncated division, matching
Rust's `/` and `%` on primitives).

## I6 - gcd well-formedness

`gcd(a, b) >= 0`; `gcd(a, 0) == |a|`; `gcd(a, b)` divides both `a` and `b`;
`gcd(0, 0) == 0`. `lcm(a, b) * gcd(a, b) == |a * b|` whenever both are
defined. `extended_gcd(a, b)` yields `(g, x, y)` with `a*x + b*y == g`.

## I7 - `pow` and `isqrt` correctness

`a.pow(0) == 1`; `a.pow(n) == a * a.pow(n-1)` (tested via properties, not
recursion); `isqrt(x) == floor(sqrt(x))`, i.e. `isqrt(x)^2 <= x <
(isqrt(x)+1)^2`; `isqrt` of a negative `BigInt` is an error (never a
wrapped value).

## I8 - Fallible growth leaves values valid

After any failed operation (`AllocationFailure`, `ResourceExhausted`,
`Cancelled`), every observable value is in a valid state satisfying
I1-I3 - operations either succeed completely or leave their inputs
unchanged.

## I9 - Limits are honoured, limits are errors

An operation run under `ResourceLimits` that would exceed a budget returns
`Err(ResourceExhausted)` **before** allocating past the budget; it never
returns a value produced outside the budget.

## I10 - Determinism

For identical inputs, any operation returns identical outputs regardless of
thread count, SIMD dispatch path, or prior allocations.

## Modular arithmetic (Phase 3)

`ModInt` is always in the canonical range `[0, m)`; the modulus is never
zero (nor one, for operations requiring invertibility); Montgomery/Barrett
implementations are differential-tested against plain modular reduction on
every code path, including the unsafe/SIMD ones.

## BigRat (Phase 4)

Denominator is strictly positive; the fraction is always fully reduced;
`BigRat::zero` is `0/1`; sign lives in the numerator.

## BigFloat (Phase 6)

Representation validity: significand normalised to the configured
precision; exponent within configured limits; NaN never equals anything
including itself; signed zero exists and `(-0) + (+0) == (+0)` in
round-to-nearest; results are independent of thread count and SIMD path
(I10).

## Certificate model (Phase 8)

A certificate is a value that an independent checker accepts or rejects;
checkers never accept on resource exhaustion (`ResourceExhausted` is an
error, not an acceptance); malformed certificates are
`Error::InvalidCertificate` before any semantic check runs.

## Search results (Phase 9)

The taxonomy `Found / NotFoundWithinBounds / Exhausted / Inconclusive /
Verified / Counterexample` is exhaustive; "not found within bounds" is
never reported as "proved absent"; worker success reports are never trusted
without a certificate.
