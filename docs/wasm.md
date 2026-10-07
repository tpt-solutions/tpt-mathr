# WebAssembly

> Status: skeleton (Phase 0). Content lands with the corresponding roadmap
> phase; the section structure below is fixed by spec section 47 so every
> algorithm is documented uniformly.

Browser and WASM-runtime deployment.

## Scope

Single-threaded baseline; wasm-bindgen bindings for BigInt, BigRat,
selected BigFloat, polynomials; capability detection (SIMD, no filesystem
or timers assumed); the optional threaded target (atomics + shared memory +
wasm-bindgen-rayon) and its cross-origin-isolation deployment requirements.

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
