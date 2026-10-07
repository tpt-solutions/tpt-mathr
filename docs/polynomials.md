# Polynomials

> Status: skeleton (Phase 0). Content lands with the corresponding roadmap
> phase; the section structure below is fixed by spec section 47 so every
> algorithm is documented uniformly.

Polynomial arithmetic over exact coefficients.

## Scope

`Polynomial<T>`, `SparsePolynomial<T>`, `FormalPowerSeries<T>`;
add/sub/mul/div/gcd/evaluate/differentiate/integrate/compose/interpolate;
adaptive multiplication with benchmark-justified thresholds; factorisation
with scoped, documented limits; degree limits via `ResourceLimits`.

## Mathematical definition

TODO(phase-5).

## Algorithm

TODO(phase-5).

## Complexity

TODO(phase-5). No complexity claims are made before measurement.

## Numerical constraints

TODO(phase-5); resource limits that apply: `ResourceLimits` fields
documented in `tpt-mathr-base`.

## Implementation

TODO(phase-5); unsafe policy: [unsafe-policy.md](unsafe-policy.md);
invariants: [invariants.md](invariants.md).

## Testing strategy

TODO(phase-5); stack per spec section 30: unit, property,
differential, metamorphic, fuzz, Miri.

## Verification strategy

TODO(phase-5); what evidence would make a result from this layer
independently checkable.
