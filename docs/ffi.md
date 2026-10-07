# FFI

> Status: skeleton (Phase 0). Content lands with the corresponding roadmap
> phase; the section structure below is fixed by spec section 47 so every
> algorithm is documented uniformly.

C ABI, Python, and language interoperability.

## Scope

The C ABI contract: opaque handles, `TptStatus f(..., T** out)`, explicit
free functions, no panics across the boundary, ABI versioning, error-detail
API; PyO3 bindings; header generation (cbindgen as a build-time tool).

## Mathematical definition

TODO(phase-11).

## Algorithm

TODO(phase-11).

## Complexity

TODO(phase-11). No complexity claims are made before measurement.

## Numerical constraints

TODO(phase-11); resource limits that apply: `ResourceLimits` fields
documented in `tpt-mathr-base`.

## Implementation

TODO(phase-11); unsafe policy: [unsafe-policy.md](unsafe-policy.md);
invariants: [invariants.md](invariants.md).

## Testing strategy

TODO(phase-11); stack per spec section 30: unit, property,
differential, metamorphic, fuzz, Miri.

## Verification strategy

TODO(phase-11); what evidence would make a result from this layer
independently checkable.
