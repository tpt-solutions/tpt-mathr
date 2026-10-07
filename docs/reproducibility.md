# Reproducibility

> Status: skeleton (Phase 0). Content lands with the corresponding roadmap
> phase; the section structure below is fixed by spec section 47 so every
> algorithm is documented uniformly.

What must be recorded for a result to be reproduced.

## Scope

The record required to reproduce any engine result: inputs, version,
precision, rounding mode, algorithm/backend configuration, CPU feature
selection, seed, and work-unit partition; how the engine pins each of
these and how search jobs persist them.

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
