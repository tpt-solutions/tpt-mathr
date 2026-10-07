# Verification

> Status: skeleton (Phase 0). Content lands with the corresponding roadmap
> phase; the section structure below is fixed by spec section 47 so every
> algorithm is documented uniformly.

Certificates, checkers, assurance profiles, proof export.

## Scope

The certificate model and serialisation; local checkers as the trusted
kernel; independent verification of FFT/NTT product candidates;
verification-obligation generation; assurance profiles (Fast, Checked,
Certified, Formal, KernelVerified); Lean export; the trusted computing base
definition.

## Mathematical definition

TODO(phase-8).

## Algorithm

TODO(phase-8).

## Complexity

TODO(phase-8). No complexity claims are made before measurement.

## Numerical constraints

TODO(phase-8); resource limits that apply: `ResourceLimits` fields
documented in `tpt-mathr-base`.

## Implementation

TODO(phase-8); unsafe policy: [unsafe-policy.md](unsafe-policy.md);
invariants: [invariants.md](invariants.md).

## Testing strategy

TODO(phase-8); stack per spec section 30: unit, property,
differential, metamorphic, fuzz, Miri.

## Verification strategy

TODO(phase-8); what evidence would make a result from this layer
independently checkable.
