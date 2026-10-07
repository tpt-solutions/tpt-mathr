# BigFloat

> Status: skeleton (Phase 0). Content lands with the corresponding roadmap
> phase; the section structure below is fixed by spec section 47 so every
> algorithm is documented uniformly.

Arbitrary-precision floating-point arithmetic.

## Scope

Sign / significand / exponent / precision representation; rounding modes
(Nearest, TowardZero, TowardPositive, TowardNegative); core arithmetic,
sqrt, powers, exp, log, trigonometric functions as maturity permits;
deterministic results independent of thread count and SIMD path.

## Mathematical definition

TODO(phase-6).

## Algorithm

TODO(phase-6).

## Complexity

TODO(phase-6). No complexity claims are made before measurement.

## Numerical constraints

TODO(phase-6); resource limits that apply: `ResourceLimits` fields
documented in `tpt-mathr-base`.

## Implementation

TODO(phase-6); unsafe policy: [unsafe-policy.md](unsafe-policy.md);
invariants: [invariants.md](invariants.md).

## Testing strategy

TODO(phase-6); stack per spec section 30: unit, property,
differential, metamorphic, fuzz, Miri.

## Verification strategy

TODO(phase-6); what evidence would make a result from this layer
independently checkable.
