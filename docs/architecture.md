# Architecture

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
