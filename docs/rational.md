# BigRat

> Status: skeleton (Phase 0). Content lands with the corresponding roadmap
> phase; the section structure below is fixed by spec section 47 so every
> algorithm is documented uniformly.

Exact rational arithmetic.

## Scope

`BigRat` with BigInt numerator and BigUint denominator, always reduced,
denominator positive; arithmetic, comparison, floor/ceil/round, numerator
and denominator accessors; the `tpt-math-exact` backend integration.

## Mathematical definition

TODO(phase-4).

## Algorithm

TODO(phase-4).

## Complexity

TODO(phase-4). No complexity claims are made before measurement.

## Numerical constraints

TODO(phase-4); resource limits that apply: `ResourceLimits` fields
documented in `tpt-mathr-base`.

## Implementation

TODO(phase-4); unsafe policy: [unsafe-policy.md](unsafe-policy.md);
invariants: [invariants.md](invariants.md).

## Testing strategy

TODO(phase-4); stack per spec section 30: unit, property,
differential, metamorphic, fuzz, Miri.

## Verification strategy

TODO(phase-4); what evidence would make a result from this layer
independently checkable.
