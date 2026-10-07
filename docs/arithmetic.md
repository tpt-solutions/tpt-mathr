# Arithmetic

> Status: skeleton (Phase 0). Content lands with the corresponding roadmap
> phase; the section structure below is fixed by spec section 47 so every
> algorithm is documented uniformly.

The shared arithmetic layers: limb model, strategies, limits.

## Scope

The limb representation (private, platform-conditional: u64 on 64-bit,
u32 on 32-bit/wasm32), the `MulStrategy` abstraction with benchmark-derived
thresholds, fallible allocation discipline, scratch-buffer reuse, and the
determinism contract that binds every layer.

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
