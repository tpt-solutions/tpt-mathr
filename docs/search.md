# Search

> Status: skeleton (Phase 0). Content lands with the corresponding roadmap
> phase; the section structure below is fixed by spec section 47 so every
> algorithm is documented uniformly.

Reproducible, cancellable, certificate-producing search.

## Scope

`SearchProblem`/`SearchEngine`; the result taxonomy (Found,
NotFoundWithinBounds, Exhausted, Inconclusive, Verified, Counterexample);
deterministic work-unit identity; resumable jobs and persisted metadata;
backends (native threads / distributed / GPU / WASM workers); cancellation
and limits; worker failure isolation.

## Mathematical definition

TODO(phase-9).

## Algorithm

TODO(phase-9).

## Complexity

TODO(phase-9). No complexity claims are made before measurement.

## Numerical constraints

TODO(phase-9); resource limits that apply: `ResourceLimits` fields
documented in `tpt-mathr-base`.

## Implementation

TODO(phase-9); unsafe policy: [unsafe-policy.md](unsafe-policy.md);
invariants: [invariants.md](invariants.md).

## Testing strategy

TODO(phase-9); stack per spec section 30: unit, property,
differential, metamorphic, fuzz, Miri.

## Verification strategy

TODO(phase-9); what evidence would make a result from this layer
independently checkable.
