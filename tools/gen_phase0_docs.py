# One-shot generator for the Phase 0 documentation set.
import os

DOCS = {}

DOCS["architecture.md"] = '''# Architecture

**Project:** tpt-mathr - The Proof Telescope
**Spec:** v1.2.0 (see [spec.md](spec.md), the reconciled canonical specification)
**Status:** living document; decisions recorded here are binding for implementation.

## 1. Purpose and position

`tpt-mathr` is the high-performance, arbitrary-precision, proof-aware
numerical engine for the TPT ecosystem. `tpt-math` remains the general
mathematical foundation; `tpt-mathr` provides the machinery underneath it:

> compute enormous mathematical objects, search enormous mathematical
> spaces, generate evidence, and independently verify the resulting claims.

The controlling design principles (spec section 2) are: reuse before
reimplementation; mathematical APIs separate from arithmetic backends;
**search is not proof**; fast components may be untrusted while small
checkers are trusted; no hidden global mathematical state; determinism;
pure Rust core with a minimal audited unsafe boundary.

## 2. Crate topology

```text
                         tpt-mathr (facade)
                             |
     +-----------+-----------+------------+-----------+
     |           |           |            |           |
    arith       poly      symbolic     search     verify
     |           |           |            |           |
     |           +-----+-----+            |           |
     +---------+---------+                  |           |
               |                            |           |
              base <------------------------+-----------+
```

(`tpt-mathr-ffi` wraps the engine for C/Python/WASM; `tpt-mathr-cli` is a
hosted consumer. Dependency edges are defined in the workspace manifests.)

Crate-by-crate rationale:

* **`tpt-mathr-base`** - the shared substrate every crate depends on: the
  error model, `ResourceLimits`, `CancellationToken`, fallible allocation
  helpers, FFI status codes. It exists so that the cross-cutting contracts
  (fallible allocation, resource budgets, cancellation, "exhaustion is not
  a result") are written once, tested once, and honoured everywhere. It is
  `no_std + alloc` with a `std`-feature time source and has **zero
  dependencies**.
* **`tpt-mathr-arith`** - the foundational new component: `BigUint`,
  `BigInt`, rationals, floats, modular arithmetic, number theory. No
  external runtime dependency; GMP/Python are dev-only references.
* **`tpt-mathr-poly`** - polynomial arithmetic over exact coefficients, the
  computational substrate under `tpt-math-symbolic`, not an independent CAS.
* **`tpt-mathr-symbolic`** - proof-aware rewriting with packed e-graph
  storage; only what cannot live in `tpt-math-symbolic` (interface review
  decides, Phase 7).
* **`tpt-mathr-search`** - reproducible, cancellable, resumable search with
  certificate output and an explicit result taxonomy.
* **`tpt-mathr-verify`** - the small trusted kernel: certificate checkers,
  assurance profiles, Lean/Coq export.
* **`tpt-mathr-ffi` / `tpt-mathr-cli` / `tpt-mathr` (facade)** -
  interoperability, command-line access, and the single-crate user
  experience with feature-flagged re-exports.

Deliberately removed (spec section 4): `tpt-mathr-core`, `tpt-mathr-compute`,
`tpt-mathr-sandbox` - their responsibilities belong elsewhere.

## 3. Recorded decisions

### D1 - Dual licence MIT OR Apache-2.0 (supersedes v1.1 "MIT only")

Every crate is `license = "MIT OR Apache-2.0"`; source files carry the SPDX
header. Contribution terms are defined in CONTRIBUTING.md. Dependencies
must be MIT-compatible; Apache-2.0-only and copyleft licences are excluded
from shipped crates (see [dependency-audit.md](dependency-audit.md)).

### D2 - The v1.2 review deltas (normative)

The v1.2 review (reconciled into [spec.md](spec.md)) changed the
architecture in eight ways; all are implemented or encoded in the design:

1. **Fallible allocation, not OOM-abort.** Rust aborts on allocation
   failure through ordinary growth paths. All large-data growth goes through
   `try_reserve`-based helpers in `tpt-mathr-base::alloc_helpers`;
   `allocator-api2` is deferred until a demonstrated need (standing
   decision).
2. **Explicit resource limits.** `ResourceLimits` carries budgets for
   memory, integer bits, polynomial degree, search depth, certificate size,
   and execution time; checks are cooperative and return
   `Error::ResourceExhausted`.
3. **Private, platform-conditional limbs.** `Limb = u64` on 64-bit targets,
   `u32` on 32-bit/wasm32, with `DoubleLimb = u128`/`u64` intermediates.
   The public API is `BigInt`, never `BigInt<Limb>` - a hard requirement.
4. **Symbolic allocation is two problems.** Arithmetic uses reusable
   buffers; the symbolic layer uses arenas for short-lived rewrite
   candidates and *packed indexed storage* (`Vec<EClass>`/`Vec<ENode>`,
   `EClassId`/`ENodeId`) for the long-lived e-graph. The e-graph is not
   arena-allocated.
5. **WASM baseline is single-threaded.** A threaded target
   (atomics + shared memory + `wasm-bindgen-rayon`) is a later, optional
   execution backend with documented cross-origin-isolation requirements -
   a deployment concern, not core arithmetic.
6. **FFI: status + opaque handle + out-parameter.** Canonical ABI is
   `TptStatus f(..., T** out)` with explicit free functions; no struct
   returns across the C ABI; no panics across the boundary.
7. **Cooperative cancellation.** `CancellationToken` (cheap clone, atomic,
   no_std-safe) is checked at safety points; coordinator-driven cancellation
   is token sharing.
8. **Parallelism is measured, never assumed.** Algorithm-level and
   workload-level parallelism are distinguished; benchmarks measure 1/2/4/8+
   threads; no linear-scaling assumption is baked into any design.

### D3 - Public API surface

Thin and implementation-free: users see `BigInt`, `BigRat`, `BigFloat`,
`Polynomial`. Limb types, `MulStrategy` selection, arenas, scratch pools,
and execution backends are private. Addition of any generic parameter to a
public mathematical type requires an architecture-decision record here.

### D4 - Result semantics (spec sections 33/34)

`ResourceExhausted` and `Inconclusive` are `Err` payloads that can never be
coerced into `false`/verified outcomes; enforcement is typed
(`Error::definite_refutation` returns `Some` only for genuine refutations),
mapped (distinct non-zero `TptStatus` codes), and pinned by tests. Search
APIs must keep `NotFoundWithinBounds` distinct from `Exhausted` and from
proved absence.

### D5 - MSRV and toolchain

Stable Rust only; MSRV **1.85** (first release with Edition 2024), pinned
stable toolchain in `rust-toolchain.toml`, nightly reserved for Miri/fuzz
tooling.

### D6 - Testing strategy stack

Unit -> property (dependency-free RNG) -> differential (Python `int`
in-tree; GMP harness where licensing/CI permits, dev-only) -> metamorphic
-> fuzz -> Miri (unsafe kernel) -> conformance (BigFloat vs MPFR,
dev-only). See [spec.md](spec.md) section 30 and [benchmarks.md](benchmarks.md).

## 4. Relationship to existing TPT repositories (spec section 3)

Every integration begins with an **interface review** of the target
repository; `tpt-mathr` must not duplicate or replace existing APIs:

| Repository | Relationship |
|---|---|
| `tpt-math` | owns the public mathematical abstractions; `tpt-mathr` is a backend |
| `tpt-math-exact` | `tpt-mathr-arith` becomes an optional backend behind a feature flag (Phase 4) |
| `tpt-math-symbolic` | public symbolic layer; `tpt-mathr` supplies coefficients, canonicalisation, certificates |
| `tpt-formal` / `tpt-telos` | verification substrate; verification obligations flow *to* them |
| `tpt-solver` | its core/check split is reused conceptually (untrusted search, trusted check) |
| `tpt-compute` | compute scheduling; no second generic compute platform |
| `tpt-gpu` | GPU kernels via TPTIR only; no second GPU runtime |
| `tpt-syntaxis` | discovery/orchestration delegates heavy computation to `tpt-mathr` |

## 5. Security and trust architecture

Trust is isolated: complicated fast components (SIMD kernels, parallel
search, heuristics) are *untrusted* producers; small exact components
(exact arithmetic, certificate checkers, invariant checkers) form the
trusted kernel. [unsafe-policy.md](unsafe-policy.md) governs the `unsafe`
boundary; [verification.md](verification.md) will define the trusted
computing base for certificates (Phase 8).
'''

DOCS["invariants.md"] = '''# Arithmetic invariants

Spec section 32 requires every invariant below to be documented and tested.
`tpt-mathr-arith` provides a debug/test-only invariant checker
(`assert_invariants`) that validates the full list; property tests assert
that no public operation breaks it. This file defines the invariants
normatively; implementations must reference them by name in tests.

## I1 - `BigUint` limb normalisation

The limb vector of a `BigUint` is little-endian and contains **no trailing
zero limbs**. The most significant limb, if any, is non-zero.

## I2 - Canonical zero

Zero has exactly one representation: the empty limb vector. There is no
"negative zero", no "zero with stale limbs", no sign-carrying zero.

## I3 - `BigInt` canonical sign

A zero-magnitude `BigInt` always carries sign `Positive`; `(-0)` cannot be
constructed or arise from any operation.

## I4 - Radix-string round trip

For every radix `2..=36`, `parse_radix(to_radix_string(x, r), r) == x`, and
`to_radix_string` never emits leading zeros, a sign for `BigUint`, or
characters outside the radix alphabet (`0-9`, then `a-z`); parsing accepts
upper and lower case.

## I5 - Division identity

For all `BigInt` `a`, `b != 0`: `a == b * trunc(a / b) + rem(a / b)` with
`sign(rem) == sign(a)` and `|rem| < |b|` (truncated division, matching
Rust's `/` and `%` on primitives).

## I6 - gcd well-formedness

`gcd(a, b) >= 0`; `gcd(a, 0) == |a|`; `gcd(a, b)` divides both `a` and `b`;
`gcd(0, 0) == 0`. `lcm(a, b) * gcd(a, b) == |a * b|` whenever both are
defined. `extended_gcd(a, b)` yields `(g, x, y)` with `a*x + b*y == g`.

## I7 - `pow` and `isqrt` correctness

`a.pow(0) == 1`; `a.pow(n) == a * a.pow(n-1)` (tested via properties, not
recursion); `isqrt(x) == floor(sqrt(x))`, i.e. `isqrt(x)^2 <= x <
(isqrt(x)+1)^2`; `isqrt` of a negative `BigInt` is an error (never a
wrapped value).

## I8 - Fallible growth leaves values valid

After any failed operation (`AllocationFailure`, `ResourceExhausted`,
`Cancelled`), every observable value is in a valid state satisfying
I1-I3 - operations either succeed completely or leave their inputs
unchanged.

## I9 - Limits are honoured, limits are errors

An operation run under `ResourceLimits` that would exceed a budget returns
`Err(ResourceExhausted)` **before** allocating past the budget; it never
returns a value produced outside the budget.

## I10 - Determinism

For identical inputs, any operation returns identical outputs regardless of
thread count, SIMD dispatch path, or prior allocations.

## Modular arithmetic (Phase 3)

`ModInt` is always in the canonical range `[0, m)`; the modulus is never
zero (nor one, for operations requiring invertibility); Montgomery/Barrett
implementations are differential-tested against plain modular reduction on
every code path, including the unsafe/SIMD ones.

## BigRat (Phase 4)

Denominator is strictly positive; the fraction is always fully reduced;
`BigRat::zero` is `0/1`; sign lives in the numerator.

## BigFloat (Phase 6)

Representation validity: significand normalised to the configured
precision; exponent within configured limits; NaN never equals anything
including itself; signed zero exists and `(-0) + (+0) == (+0)` in
round-to-nearest; results are independent of thread count and SIMD path
(I10).

## Certificate model (Phase 8)

A certificate is a value that an independent checker accepts or rejects;
checkers never accept on resource exhaustion (`ResourceExhausted` is an
error, not an acceptance); malformed certificates are
`Error::InvalidCertificate` before any semantic check runs.

## Search results (Phase 9)

The taxonomy `Found / NotFoundWithinBounds / Exhausted / Inconclusive /
Verified / Counterexample` is exhaustive; "not found within bounds" is
never reported as "proved absent"; worker success reports are never trusted
without a certificate.
'''

DOCS["unsafe-policy.md"] = '''# unsafe policy

**Baseline: safe Rust.** The default for every crate is
`#![forbid(unsafe_code)]`. The spec guarantee (section 2.7) is:

> Pure Rust mathematical implementation with a minimal, explicitly audited
> unsafe boundary.

## Where unsafe is permitted

Exactly three places, by standing decision:

1. **The limb/SIMD kernel module in `tpt-mathr-arith`** (Phase 3) - a single
   module, `unsafe` confined to it, with scalar safe reference
   implementations kept and *always* selectable.
2. **`tpt-mathr-ffi`** - the C ABI boundary (opaque handles, pointer
   validity, `catch_unwind` fencing).
3. **WASM glue in `tpt-mathr-ffi`** - `wasm-bindgen` marshalling.

Nothing else may introduce `unsafe`. A fourth location requires an ADR in
[architecture.md](architecture.md) before implementation.

## Current state

**The entire workspace is `#![forbid(unsafe_code)]`** (as of Phase 0-2).
The kernel module and FFI boundary do not exist yet; when they land, the
crate-level forbid in `arith` moves to module level, and the ffi crate uses
module-level forbids on its safe wrapper modules with `unsafe` confined to
the ABI modules.

## Rules for every `unsafe` block

1. A `// SAFETY:` comment naming the invariant that makes the call sound
   and who maintains it.
2. A scalar/naive safe reference path exists in the same crate, and tests
   exercise **both** paths and compare them (differential coverage).
3. Miri runs over the containing crate test suite (CI `miri` job) - the
   harness exists from Phase 0 so the first unsafe line lands under Miri.
4. Fuzzing covers the public entry points that reach the unsafe path with
   adversarial inputs (sizes, alignment-sensitive lengths, empty inputs).
5. No `unsafe` may *widen visibility*: soundness conditions must be
   enforceable inside the module that contains the block; if a caller
   could break it, the API is wrong.
6. Transmute-free preference: `unsafe` is for intrinsics, raw pointers at
   ABI/SIMD boundaries, and initialised-memory contracts - never for type
   punning that safe alternatives can express.

## Runtime dispatch discipline (Phase 3)

SIMD paths are selected by CPU capability detection at runtime. The
determinism requirement (I10) applies: every dispatch path must produce
bit-identical results; the differential harness pins this by running
operations across all available paths and comparing outputs exactly.
'''

DOCS["dependency-audit.md"] = '''# Dependency audit

Standing decision: dependencies must be MIT-compatible (MIT, MIT/Apache-2.0
dual, BSD, ISC, Zlib, Unlicense/CC0). **No Apache-2.0-only dependencies in
shipped crates. No GPL/LGPL/MPL in shipped crates.** GMP / MPFR / FLINT /
Python are dev-dependency/benchmark-only, never runtime.

Procedure: every proposed dependency gets a row here (crate, version,
licence expression, purpose, transitive-risk notes, decision) *before* it
enters any manifest. `deny.toml` enforces the allow-list; this file records
the human decision. The audit is repeated at release hardening (Phase 13).

## Current state: zero runtime dependencies

The workspace deliberately ships **no external dependencies** - the core is
`std`/`alloc`-only (spec section 44). All support tooling (property-testing
RNG, test serialisation) is written in-tree until a dependency clears this
audit.

## Audited candidates

| Crate | Licence | Purpose | Decision |
|---|---|---|---|
| `proptest` | MIT OR Apache-2.0 | property testing | **Deferred.** In-tree deterministic RNG property harness is sufficient for Phase 2; revisit when shrinking matters. |
| `bumpalo` | MIT OR Apache-2.0 | symbolic arena allocation | **Approved in principle** for Phase 7 symbolic temporaries; re-audit exact version at adoption. |
| `rayon` | MIT OR Apache-2.0 | parallel execution backend (search) | **Approved in principle** for Phase 9 as one swappable backend behind an execution-backend abstraction; core stays dependency-free. |
| `pyo3` | MIT OR Apache-2.0 (transitives include `target-lexicon` under Apache-2.0 WITH LLVM-exception) | Python bindings | **Conditionally approved** for Phase 11, `ffi` crate only, feature-gated; the LLVM-exception variant is accepted for ffi/dev only, never core. |
| `wasm-bindgen` | MIT OR Apache-2.0 | WASM bindings | **Conditionally approved** for Phase 11, `ffi` crate only, feature-gated. |
| `cbindgen` | MPL-2.0 | C header generation | **Not shippable as a dependency** (MPL excluded); approved as a build-time tool only (CI-installed binary), generating headers from our code; alternative is a hand-maintained header reviewed against the Rust ABI. |
| `rug` / `gmp-mpfr-sys` | LGPL/GPL | GMP/MPFR differential + benchmark references | **Dev/bench only**, feature-gated, never default, never published in the default feature set; CI environment permitting. |
| `criterion` | MIT OR Apache-2.0 | benchmark harness | **Approved** for `benchmarks/` (dev/bench only). Re-audit exact version and transitives at adoption. |
| `libfuzzer-sys` | MIT OR Apache-2.0 (links LLVM libFuzzer: Apache-2.0 WITH LLVM-exception) | fuzz targets | **Approved** for `fuzz/` only - a separate workspace, never part of the published crates. |

## Release-time checklist (Phase 13)

- `cargo deny check` clean at the shipped feature set.
- No Apache-2.0-only, GPL, LGPL, MPL, or AGPL crate in any shipped
  dependency graph (`cargo tree -e no-dev` output recorded here).
- GMP/MPFR/Python-gated features verified absent from default builds
  (`cargo tree --no-default-features` output recorded here).
'''

for name, body in DOCS.items():
    path = os.path.join("docs", name)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(body)
    print("wrote", path)
