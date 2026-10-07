# Benchmarks

> Status: skeleton (Phase 0). Content lands with the corresponding roadmap
> phase; the section structure below is fixed by spec section 47 so every
> algorithm is documented uniformly.

How performance is measured, recorded, and claimed.

## Scope

The benchmark contract: recorded hardware, compiler, flags, operand size,
operation, threads, backend, and version for every published number;
operand matrix 32 B to 100 MiB; parallel-scaling runs at 1/2/4/8+ threads;
comparisons vs num-bigint, GMP, Python int only with full metadata; no
claims without data.

## Mathematical definition

TODO(phase-3).

## Algorithm

TODO(phase-3).

## Complexity

TODO(phase-3). No complexity claims are made before measurement.

## Numerical constraints

TODO(phase-3); resource limits that apply: `ResourceLimits` fields
documented in `tpt-mathr-base`.

## Implementation

TODO(phase-3); unsafe policy: [unsafe-policy.md](unsafe-policy.md);
invariants: [invariants.md](invariants.md).

## Testing strategy

TODO(phase-3); stack per spec section 30: unit, property,
differential, metamorphic, fuzz, Miri.

## Verification strategy

TODO(phase-3); what evidence would make a result from this layer
independently checkable.
