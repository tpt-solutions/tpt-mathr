# Big integers (BigUint / BigInt)

> Status: skeleton (Phase 0). Content lands with the corresponding roadmap
> phase; the section structure below is fixed by spec section 47 so every
> algorithm is documented uniformly.

Arbitrary-precision integers.

## Scope

`BigUint` and `BigInt`: canonical representations, addition, subtraction,
comparison, multiplication strategies (schoolbook, Karatsuba, Toom-Cook,
NTT/FFT), division/remainder (Knuth D), gcd/extended gcd/lcm, pow, isqrt,
parsing and formatting for radix 2-36, primitive conversions.

## Mathematical definition

TODO(phase-2).

## Algorithm

TODO(phase-2).

## Complexity

TODO(phase-2). No complexity claims are made before measurement.

## Numerical constraints

TODO(phase-2); resource limits that apply: `ResourceLimits` fields
documented in `tpt-mathr-base`.

## Implementation

TODO(phase-2); unsafe policy: [unsafe-policy.md](unsafe-policy.md);
invariants: [invariants.md](invariants.md).

## Testing strategy

TODO(phase-2); stack per spec section 30: unit, property,
differential, metamorphic, fuzz, Miri.

## Verification strategy

TODO(phase-2); what evidence would make a result from this layer
independently checkable.
