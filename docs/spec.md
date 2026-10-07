# Software Design Specification: tpt-mathr

**Project Name:** tpt-mathr
**Codename:** The Proof Telescope
**Document Version:** 1.2.0 (canonical reconciled edition)
**Status:** Architecture / Implementation Specification — single source of truth
**Language:** Rust, Edition 2024
**License:** dual MIT OR Apache-2.0 (v1.2 standing decision; the "Primary License: MIT" line below is superseded — see Amendment A0)
**Organisation:** TPT Solutions

> **Reconciliation note.** This document is the single canonical
> specification. It consists of (1) the base specification text, and (2) the
> normative **Amendments from the v1.2 review** at the end, which supersede
> the clauses they cite. It replaces the previously separate files
> `spec.txt` / `spec1.1.txt` (identical base texts) and `spec1.2.txt`
> (review deltas), which are preserved verbatim under `docs/history/` for
> provenance. Where this file and the historical files disagree, **this
> file governs**.

---



# 1. Executive Summary

`tpt-mathr` is the high-performance, arbitrary-precision, proof-aware numerical engine for the TPT mathematical computing ecosystem.

It is **not a replacement for `tpt-math`**.

`tpt-math` remains the general mathematical foundation containing numerical abstractions, exact mathematics, linear algebra, symbolic mathematics, geometry, graph theory, probability, statistics, optimisation, autodiff, signal processing and related mathematical primitives.

`tpt-mathr` provides the deeper numerical machinery required when ordinary mathematical computing is insufficient:

* native Rust arbitrary-precision integers;
* arbitrary-precision rationals;
* arbitrary-precision floating-point arithmetic;
* modular arithmetic;
* high-performance polynomial arithmetic;
* number-theoretic primitives;
* adaptive multiplication algorithms;
* CPU SIMD acceleration;
* deterministic arithmetic;
* reproducible numerical computation;
* proof-oriented arithmetic operations;
* certificate generation;
* high-performance mathematical search;
* language interoperability through outbound FFI;
* integration with TPT's formal verification ecosystem.

The architectural principle is:

> **`tpt-math` defines mathematical functionality; `tpt-mathr` provides high-performance and high-assurance computational machinery underneath it.**

The resulting ecosystem is:

```text
                         TPT Mathematical Ecosystem

                           ┌─────────────────┐
                           │  tpt-syntaxis   │
                           │ theorem         │
                           │ discovery       │
                           └────────┬────────┘
                                    │
                       ┌────────────┴────────────┐
                       │                         │
              ┌────────▼────────┐       ┌──────▼───────┐
              │    tpt-mathr    │       │  tpt-solver  │
              │ high precision  │       │ SAT / SMT    │
              │ numerical       │       │ certificates  │
              │ computation     │       └──────┬───────┘
              └────────┬────────┘              │
                       │                       │
              ┌────────▼────────┐              │
              │    tpt-math     │◄─────────────┘
              │ mathematical    │
              │ foundation      │
              └────────┬────────┘
                       │
             ┌─────────┴──────────┐
             │                    │
      ┌──────▼───────┐    ┌──────▼──────┐
      │ tpt-formal   │    │ tpt-compute │
      │ verification │    │ compute     │
      └──────┬───────┘    └──────┬──────┘
             │                   │
             └─────────┬─────────┘
                       │
                 ┌─────▼─────┐
                 │ tpt-telos │
                 │ formal    │
                 │ contracts │
                 └───────────┘
```

The ultimate goal is not merely faster arithmetic.

The goal is to provide the computational substrate through which TPT can:

> **compute enormous mathematical objects, search enormous mathematical spaces, generate evidence, and independently verify the resulting claims.**

---

# 2. Design Principles

`tpt-mathr` shall follow these principles.

## 2.1 Reuse Before Reimplementation

`tpt-mathr` MUST NOT recreate mathematical abstractions already provided by `tpt-math` unless there is a demonstrated architectural reason.

Existing TPT capabilities are consumed through stable interfaces.

Examples:

```text
tpt-math-numeric
tpt-math-exact
tpt-math-symbolic
tpt-math-linalg
tpt-math-graph
tpt-math-optimize
tpt-math-autodiff
...
```

`tpt-mathr` provides high-performance implementations and extensions where required.

---

## 2.2 Mathematical APIs Are Separate From Arithmetic Backends

The public mathematical API should not depend on the implementation details of the arithmetic engine.

Conceptually:

```text
Mathematical API
       │
       ▼
Arithmetic abstraction
       │
       ├── conventional backend
       │
       └── tpt-mathr backend
```

This permits users to write mathematical software without committing to a particular arbitrary-precision implementation.

---

## 2.3 Search Is Not Proof

A computation may discover a candidate result without establishing its correctness.

Therefore:

```text
Search
  ↓
Candidate
  ↓
Certificate / evidence
  ↓
Independent verification
  ↓
Accepted result
```

The architecture must never equate:

```text
"the algorithm says true"
```

with:

```text
"the result has been independently verified"
```

---

## 2.4 Fast Components May Be Untrusted

High-performance search components are allowed to be complicated.

Trust is isolated into small verification components.

This follows the same architectural principle already used by `tpt-solver`: its search engine is explicitly separated from its independent certificate checker.

The same philosophy applies here:

```text
                 UNTRUSTED
        ┌────────────────────────┐
        │ SIMD                    │
        │ parallel search         │
        │ heuristic algorithms    │
        │ GPU acceleration        │
        │ AI-generated candidates │
        └───────────┬────────────┘
                    │
                 result
                    │
                    ▼
                 TRUSTED
        ┌────────────────────────┐
        │ exact arithmetic       │
        │ certificate checker     │
        │ invariant checker      │
        │ proof assistant         │
        └────────────────────────┘
```

---

## 2.5 No Hidden Global Mathematical State

Core arithmetic shall avoid hidden process-global mutable state.

Objects should be independently owned and safely transferable between threads where their semantics permit.

The project MUST NOT claim that all arithmetic is inherently "lock-free".

Instead:

> Arithmetic operations are designed without hidden global mutable state and independent operations can be safely parallelised.

---

## 2.6 Determinism

Given:

* identical inputs;
* identical precision;
* identical rounding mode;
* identical algorithm configuration;

the mathematical result must be reproducible.

Parallel execution must not silently alter mathematical semantics.

---

## 2.7 Pure Rust Core

The mathematical core shall be implemented in Rust.

`unsafe` is permitted only where justified and must be isolated and audited.

Expected areas include:

```text
SIMD
limb operations
specialised memory operations
FFI boundaries
```

The desired guarantee is:

> **Pure Rust mathematical implementation with a minimal, explicitly audited unsafe boundary.**

---

# 3. Relationship to Existing TPT Repositories

`tpt-mathr` exists inside an already substantial mathematical ecosystem.

## 3.1 tpt-math

`tpt-math` is the general mathematical foundation.

It owns:

* numerical traits;
* exact mathematics;
* linear algebra;
* symbolic mathematics;
* geometry;
* graph theory;
* probability;
* statistics;
* optimisation;
* autodiff;
* signal processing;
* units and related mathematical abstractions.

`tpt-mathr` MUST NOT duplicate these APIs unnecessarily.

---

## 3.2 tpt-math-exact

`tpt-math-exact` is the most important existing integration point.

It already provides the exact-arithmetic abstraction.

`tpt-mathr-arith` becomes a potential high-performance implementation backend for that functionality.

Target architecture:

```text
tpt-math-exact
       │
       ├── existing/default backend
       │
       └── tpt-mathr backend
              │
              ├── BigUint
              ├── BigInt
              ├── BigRat
              └── BigFloat
```

The public API should remain stable where possible.

---

## 3.3 tpt-math-symbolic

Existing symbolic mathematics remains the public symbolic layer.

`tpt-mathr` supplies:

* larger coefficients;
* exact polynomial operations;
* arbitrary precision;
* high-performance canonicalisation;
* proof-aware rewrites;
* arithmetic certificates.

`tpt-mathr-symbolic` should therefore only be created where functionality cannot naturally belong in `tpt-math-symbolic`.

---

## 3.4 tpt-formal

`tpt-formal` provides the wider verification substrate.

`tpt-mathr` should consume it for mathematical verification interfaces rather than implementing another general verification framework.

---

## 3.5 tpt-telos

`tpt-telos` provides formal contract / verification infrastructure.

`tpt-mathr-verify` shall integrate with the actual current TPT verification architecture rather than assuming a new inline proof syntax.

Where appropriate:

```text
tpt-mathr
    ↓
verification obligation
    ↓
tpt-telos / tpt-formal
```

---

## 3.6 tpt-solver

`tpt-solver` provides SAT/SMT and certificate-backed constraint solving.

It already separates:

```text
tpt-solver-core
    search

tpt-solver-check
    trusted certificate checking
```

This architecture should be reused conceptually rather than recreated.

---

## 3.7 tpt-compute

`tpt-compute` provides general high-performance computational infrastructure.

`tpt-mathr` should use it for workloads requiring more sophisticated compute scheduling rather than implementing another generic compute platform.

---

## 3.8 tpt-gpu

`tpt-gpu` already provides hardware-agnostic GPU computation, TPTIR and GPU optimisation infrastructure. Its TPTIR layer is explicitly designed around SSA, tensors, memory hierarchy and progressive lowering.

`tpt-mathr` may eventually expose suitable arithmetic kernels through this infrastructure.

It must not create a second GPU runtime.

---

## 3.9 tpt-syntaxis

`tpt-syntaxis` is already the mathematical discovery/orchestration layer.

It currently combines:

```text
conjecture generation
        ↓
proof search
        ↓
Lean kernel verification
        ↓
proof minimisation
        ↓
audit
        ↓
publication
```

and supports symbolic, AI and hybrid discovery modes.

Therefore `tpt-mathr` should become a computational engine consumed by Syntaxis rather than attempting to replace it.

---

# 4. Repository Structure

The initial `tpt-mathr` workspace shall contain:

```text
tpt-mathr/
│
├── crates/
│
│   ├── tpt-mathr-arith/
│   │
│   ├── tpt-mathr-poly/
│   │
│   ├── tpt-mathr-symbolic/
│   │
│   ├── tpt-mathr-search/
│   │
│   ├── tpt-mathr-verify/
│   │
│   ├── tpt-mathr-ffi/
│   │
│   └── tpt-mathr-cli/
│
├── benchmarks/
├── tests/
├── examples/
├── docs/
├── proofs/
├── Cargo.toml
├── README.md
├── LICENSE
└── CHANGELOG.md
```

Three components from the original specification are deliberately removed:

```text
tpt-mathr-core
tpt-mathr-compute
tpt-mathr-sandbox
```

Their responsibilities belong elsewhere.

---

# 5. Crate Specifications

# 5.1 tpt-mathr-arith

## Purpose

Provide native Rust arbitrary-precision arithmetic.

This is the foundational new component.

## Responsibilities

### Integer types

```text
BigUint
BigInt
```

### Rational types

```text
BigRat
```

### Floating-point types

```text
BigFloat
```

### Modular arithmetic

```text
ModInt
Montgomery arithmetic
Barrett reduction
modular exponentiation
```

### Number theory

Potential primitives:

```text
gcd
extended_gcd
lcm
mod_inverse
integer_sqrt
isqrt_rem
pow
pow_mod
primality testing
factorisation helpers
```

Factorisation itself must not be promised as universally fast.

---

# 6. Limb Representation

The integer implementation shall use a limb-based representation.

Default limb:

```rust
type Limb = u64;
```

on architectures where that representation is appropriate.

Internal representation:

```text
BigUint
├── sign: none
├── limbs: Vec<Limb>
└── normalisation invariant
```

`BigInt`:

```text
BigInt
├── sign
└── magnitude: BigUint
```

The implementation must maintain canonical representations.

For example:

```text
zero
```

must have exactly one canonical representation.

---

# 7. Multiplication Architecture

Multiplication shall be strategy-based.

Conceptual interface:

```rust
trait MulStrategy {
    fn multiply(
        lhs: &[Limb],
        rhs: &[Limb],
    ) -> Vec<Limb>;
}
```

Candidate strategies:

```text
schoolbook
    ↓
Karatsuba
    ↓
Toom-Cook
    ↓
NTT / FFT
    ↓
advanced asymptotic multiplication
```

Thresholds shall be benchmark-derived.

The project MUST NOT hard-code the assumption that one algorithm is universally fastest.

---

# 8. SIMD Architecture

SIMD optimisation shall be isolated behind architecture-specific implementations.

Target families:

```text
x86_64
    AVX2
    AVX-512 where appropriate

aarch64
    NEON

WASM
    SIMD where supported
```

The implementation should use portable abstractions where practical and explicit architecture intrinsics where they provide measurable benefit.

CPU feature detection shall select implementations at runtime where appropriate.

Conceptually:

```text
BigUint operation
       │
       ▼
dispatch
       │
       ├── scalar
       ├── AVX2
       ├── AVX-512
       ├── NEON
       └── WASM SIMD
```

---

# 9. BigFloat

`BigFloat` is a major component and shall be specified independently from `BigInt`.

Representation:

```text
sign
significand
exponent
precision
```

Required features:

* configurable precision;
* explicit rounding mode;
* deterministic arithmetic;
* correctly specified special values;
* conversion to/from integer;
* conversion to/from binary floating point;
* decimal conversion;
* comparison;
* arithmetic;
* square root;
* powers;
* logarithms;
* exponentials;
* trigonometric functions where implementation maturity permits.

Initial rounding modes:

```text
Nearest
TowardZero
TowardPositive
TowardNegative
```

Additional modes may be added later.

The specification must explicitly define:

* overflow;
* underflow;
* exponent limits;
* NaN;
* infinity;
* signed zero;
* precision propagation;
* conversion semantics.

`tpt-mathr` MUST NOT claim MPFR compatibility until conformance tests demonstrate it.

---

# 10. BigRat

`BigRat` shall be represented using:

```text
numerator: BigInt
denominator: BigUint
```

with canonicalisation:

```text
gcd(|numerator|, denominator) == 1
denominator > 0
```

Operations:

```text
+
-
*
/
%
comparison
floor
ceil
round
numerator
denominator
```

Rational arithmetic must preserve exactness.

---

# 11. Polynomial Layer

## tpt-mathr-poly

This crate provides high-performance polynomial arithmetic where existing `tpt-math` polynomial functionality requires a deeper computational backend.

Potential types:

```text
Polynomial<T>
SparsePolynomial<T>
FormalPowerSeries<T>
```

Operations:

```text
add
sub
mul
div
gcd
evaluate
differentiate
integrate
compose
factor
interpolate
```

Multiplication strategy should be adaptive.

Potential backends:

```text
schoolbook
Karatsuba
Toom-Cook
NTT
FFT
```

`tpt-mathr-poly` should integrate with `tpt-math-symbolic` rather than becoming an independent symbolic algebra system.

---

# 12. tpt-mathr-symbolic

This crate is deliberately narrower than the original specification.

It provides advanced functionality that extends the existing `tpt-math-symbolic`.

Responsibilities may include:

* proof-aware expression representation;
* canonical forms;
* rewrite provenance;
* e-graph integration;
* rewrite certificates;
* exact coefficient support;
* algebraic normalisation;
* equivalence checking.

Example:

```text
Expression A
      │
      ▼
rewrite
      │
      ▼
Expression B
      │
      ▼
certificate
      │
      ▼
checker
```

Every optimisation that claims semantic equivalence should have a path to independent validation.

---

# 13. tpt-mathr-search

This replaces the original generic `tpt-mathr-compute`.

It is a mathematical search engine, not a general compute framework.

Responsibilities:

* counterexample search;
* parameter sweeps;
* conjecture exploration;
* numerical experimentation;
* symbolic search;
* exhaustive bounded search;
* Monte Carlo exploration;
* parallel search;
* resumable search;
* distributed work partitioning.

It shall expose a generic work model:

```rust
trait SearchProblem {
    type Candidate;
    type Result;
    type Error;

    fn generate(&self, state: &mut SearchState)
        -> Option<Self::Candidate>;

    fn evaluate(
        &self,
        candidate: Self::Candidate
    ) -> Result<Self::Result, Self::Error>;
}
```

Search results should be classified:

```text
Candidate
Counterexample
VerifiedResult
Inconclusive
Error
```

The system must distinguish:

```text
not found
```

from:

```text
proved absent
```

---

# 14. Parallel Search

`tpt-mathr-search` may use:

```text
Rayon
tpt-compute
tpt-gpu
distributed workers
```

depending on workload.

The mathematical semantics must not depend on execution order.

A parallel search should be partitionable:

```text
Search domain
      │
 ┌────┼────┬────┐
 ▼    ▼    ▼    ▼
W0   W1   W2   W3
 │    │    │    │
 └────┴────┴────┘
          │
          ▼
       results
```

Every work unit should have a deterministic identity where practical.

This enables:

* resume;
* deduplication;
* audit;
* distributed execution;
* reproducibility.

---

# 15. Certificate Architecture

Whenever practical, computational results should produce a certificate.

Examples:

```text
prime certificate
factorisation certificate
polynomial identity certificate
linear algebra certificate
SAT certificate
SMT certificate
counterexample witness
interval enclosure
exact rational witness
```

General model:

```text
expensive computation
        ↓
result + certificate
        ↓
small checker
        ↓
Accept / Reject / Inconclusive
```

The checker must never silently convert resource exhaustion into acceptance.

---

# 16. tpt-mathr-verify

This crate provides mathematical verification adapters.

It does NOT become a new general-purpose proof assistant.

Responsibilities:

* certificate generation;
* certificate validation;
* verification obligation generation;
* Lean export;
* Coq export where justified;
* SMT integration;
* `tpt-formal` integration;
* `tpt-telos` integration;
* proof object serialisation.

Architecture:

```text
tpt-mathr computation
        │
        ▼
verification obligation
        │
        ├── local checker
        ├── tpt-solver
        ├── tpt-formal
        ├── tpt-telos
        └── Lean
```

External solvers may be used as search engines.

Their output must not automatically become part of the trusted computing base.

This follows the existing TPT certificate architecture.

---

# 17. Lean Integration

Lean integration shall support:

```text
expression export
theorem statement export
proof/certificate export
proof replay
```

Example:

```text
tpt-mathr
    │
    │ theorem candidate
    ▼
Lean representation
    │
    ▼
Lean kernel
    │
    ├── accepted
    └── rejected
```

`tpt-syntaxis` remains responsible for higher-level theorem discovery and publication workflows. Its current architecture already treats the Lean kernel as the source of truth.

---

# 18. FFI Architecture

## tpt-mathr-ffi

The FFI is outbound.

`tpt-mathr` owns the mathematical implementation.

External languages consume it.

Architecture:

```text
                    Rust
                     │
          ┌──────────┼───────────┐
          ▼          ▼           ▼
       C ABI       PyO3        WASM
          │          │           │
          ▼          ▼           ▼
         C/C++     Python      JS/TS
```

---

# 19. C ABI

The C ABI shall use opaque handles.

Example:

```c
typedef struct TptBigInt TptBigInt;

TptBigInt*
tpt_bigint_from_str(
    const char* value,
    int radix
);

TptBigInt*
tpt_bigint_mul(
    const TptBigInt* lhs,
    const TptBigInt* rhs
);

void
tpt_bigint_free(
    TptBigInt* value
);
```

Requirements:

* opaque ownership;
* explicit allocation/free functions;
* no Rust layout exposed;
* ABI versioning;
* error codes;
* thread-safe ownership semantics;
* null handling;
* documented lifetime rules.

The C ABI must not expose internal structs.

---

# 20. Python API

PyO3 shall provide a Python-native interface.

Example:

```python
import tpt_mathr as tm

a = tm.BigInt("123456789012345678901234567890")
b = tm.BigInt("987654321987654321987654321")

c = a * b

print(c)
```

Python integration must prioritise:

* native arithmetic;
* predictable object ownership;
* efficient conversion;
* batch operations;
* zero unnecessary copying;
* wheel distribution.

The project must not claim superiority over NumPy/SymPy generally.

Performance claims must be benchmark-specific.

---

# 21. WebAssembly

WASM support should expose the deterministic arithmetic subset first.

Initial target:

```text
BigInt
BigRat
selected BigFloat
polynomials
exact algorithms
```

Browser execution must not require a server.

Possible consumers include:

* educational tools;
* mathematical visualisation;
* proof exploration;
* browser-based calculators;
* TPT web applications.

---

# 22. CLI

`tpt-mathr-cli` provides:

```text
tpt-mathr calc
tpt-mathr bigint
tpt-mathr rational
tpt-mathr float
tpt-mathr poly
tpt-mathr prime
tpt-mathr search
tpt-mathr verify
tpt-mathr benchmark
```

Example:

```bash
tpt-mathr bigint multiply \
    12345678901234567890 \
    98765432109876543210
```

Search:

```bash
tpt-mathr search \
    --problem counterexample \
    --range 1..100000000
```

Verification:

```bash
tpt-mathr verify certificate.json
```

Benchmark:

```bash
tpt-mathr benchmark arithmetic
```

---

# 23. Public API Stability

The following shall be considered core public types:

```text
BigUint
BigInt
BigRat
BigFloat
Polynomial
```

Internal implementation details such as:

```text
limb layout
multiplication thresholds
SIMD implementation
allocation strategy
FFT implementation
```

must remain private unless explicitly stabilised.

This permits performance evolution without breaking downstream applications.

---

# 24. Threading Model

Core arithmetic objects should be:

```text
Send
Sync
```

where their semantics permit.

No hidden global mutable state shall be required for normal arithmetic.

Parallelism belongs at the algorithm level:

```text
parallel polynomial multiplication
parallel search
parallel independent calculations
parallel certificate generation
```

The project MUST NOT promise:

> all arithmetic is lock-free.

Instead it promises:

> independent mathematical operations do not require shared mutable mathematical state.

---

# 25. Memory Model

Large-number operations can consume substantial memory.

Requirements:

* canonical representations;
* predictable ownership;
* no accidental copies;
* explicit borrowing APIs where beneficial;
* capacity reuse;
* configurable allocation strategy;
* optional scratch-space reuse;
* bounded algorithms where possible.

Future work may include:

```text
arena-backed temporary arithmetic
thread-local scratch buffers
memory pools
zero-copy FFI conversion
```

but these must not compromise ownership safety.

---

# 26. Reproducibility

A computation should be reproducible from:

```text
input
algorithm version
precision
rounding mode
backend
CPU feature configuration
search seed
search partition
```

Search jobs shall support persisted metadata:

```json
{
  "algorithm": "...",
  "version": "...",
  "seed": "...",
  "partition": "...",
  "precision": "...",
  "status": "..."
}
```

This enables independent replay.

---

# 27. Performance Strategy

Performance is a measured property, not an architectural assumption.

The project shall benchmark against appropriate reference implementations.

Potential references:

```text
num-bigint
GMP
MPFR
FLINT
Python int
SymPy
```

Benchmarks must identify:

* hardware;
* compiler;
* compiler flags;
* operand size;
* operation;
* thread count;
* backend;
* version.

---

# 28. Benchmark Matrix

Integer benchmarks:

```text
32 bits
64 bits
128 bits
1 KiB
4 KiB
16 KiB
64 KiB
256 KiB
1 MiB
10 MiB
100 MiB
```

Operations:

```text
add
subtract
multiply
divide
modulo
gcd
pow
pow_mod
comparison
conversion
```

Rational benchmarks:

```text
addition
multiplication
division
reduction
comparison
```

BigFloat:

```text
addition
multiplication
division
sqrt
exp
log
sin
cos
conversion
```

Polynomial:

```text
addition
multiplication
division
gcd
evaluation
composition
```

---

# 29. Performance Acceptance Criteria

No absolute claim such as:

> "beats GMP"

shall be a design requirement.

Instead each release shall publish benchmark data.

Milestones may include:

```text
M1:
correctness parity

M2:
competitive small/medium integer performance

M3:
competitive large integer multiplication

M4:
parallel polynomial scaling

M5:
large-number workloads demonstrating practical advantage

M6:
verified computation with acceptable overhead
```

Performance regressions greater than an agreed threshold should fail CI benchmarks where infrastructure permits.

---

# 30. Testing Strategy

Testing occurs at several levels.

## Unit tests

Every arithmetic primitive.

## Property tests

Examples:

```text
a + b == b + a

(a + b) + c == a + (b + c)

a * (b + c) == a*b + a*c

gcd(a,b) divides a
gcd(a,b) divides b
```

subject to mathematical domain requirements.

## Differential tests

Compare against established implementations.

For example:

```text
tpt BigInt
       vs
GMP
       vs
Python int
```

for generated inputs.

## Fuzzing

Fuzz:

* parsing;
* arithmetic;
* serialisation;
* FFI;
* certificates;
* polynomial operations.

## Metamorphic testing

Use mathematical identities to test results without requiring an external oracle.

---

# 31. Formal Verification Strategy

Not every line of the high-performance engine needs to be formally verified.

Instead identify a trusted kernel.

Potential trusted components:

```text
canonical arithmetic invariants
certificate checker
proof export correctness
critical limb operations
```

High-performance algorithms may be treated as untrusted producers of candidate results.

Example:

```text
FFT multiplication
       ↓
candidate product
       ↓
independent verification
       ↓
accepted product
```

This permits aggressive optimisation without expanding the trusted base unnecessarily.

---

# 32. Arithmetic Invariants

The implementation shall document and test invariants including:

```text
limbs are normalised
zero has canonical representation
no leading zero limbs
BigInt sign is canonical
BigRat denominator is positive
BigRat is reduced
BigFloat representation is valid
```

Critical invariants should have explicit verification tests.

---

# 33. Error Model

Errors must distinguish:

```text
ParseError
Overflow
Underflow
DivisionByZero
InvalidPrecision
InvalidRadix
AllocationFailure
VerificationFailure
UnsupportedOperation
InvalidCertificate
ResourceExhausted
Inconclusive
```

In particular:

```text
ResourceExhausted
```

MUST NOT be treated as:

```text
False
```

or:

```text
Verified
```

---

# 34. Search Result Semantics

Search APIs must distinguish:

```text
Found
NotFoundWithinBounds
Exhausted
Inconclusive
Verified
Counterexample
```

Example:

A search through:

```text
0 <= n <= 1,000,000
```

that finds no counterexample means:

> no counterexample was found within that domain.

It does not prove:

> no counterexample exists.

The API must make this distinction explicit.

---

# 35. Distributed Search

Distributed search is optional and belongs in `tpt-mathr-search`.

Architecture:

```text
                coordinator
                    │
        ┌───────────┼───────────┐
        ▼           ▼           ▼
     worker 0    worker 1    worker 2
        │           │           │
        └───────────┼───────────┘
                    ▼
              certificates
                    │
                    ▼
                verifier
```

Work units must be independently executable.

A worker must never be trusted simply because it reports success.

---

# 36. GPU Integration

GPU acceleration is not a Phase 1 requirement.

Where useful, arithmetic kernels may eventually target `tpt-gpu`.

Architecture:

```text
tpt-mathr
    │
    ▼
arithmetic operation
    │
    ├── CPU scalar
    ├── CPU SIMD
    └── GPU backend
             │
             ▼
          tpt-gpu
```

GPU results should be independently validated when correctness is critical.

`tpt-gpu` already provides a hardware-agnostic GPU stack and TPTIR compiler/runtime architecture, so `tpt-mathr` should consume that infrastructure rather than creating another GPU abstraction.

---

# 37. Integration With tpt-syntaxis

`tpt-syntaxis` should eventually be able to call:

```text
tpt-mathr-arith
tpt-mathr-poly
tpt-mathr-search
tpt-mathr-verify
```

for computationally expensive mathematical exploration.

Example:

```text
Syntaxis
   │
   ├── conjecture
   │
   ▼
tpt-mathr-search
   │
   ├── numerical exploration
   ├── symbolic exploration
   └── counterexample search
   │
   ▼
candidate
   │
   ▼
tpt-mathr-verify
   │
   ▼
Lean / tpt-formal / tpt-solver
   │
   ▼
verified theorem
```

Syntaxis remains responsible for discovery orchestration, human review, proof minimisation and publication. Its existing architecture already supports these responsibilities.

---

# 38. Integration With tpt-solver

`tpt-solver` can provide:

```text
SAT
SMT
LRA
constraint solving
certificate checking
```

to mathematical search.

Conversely, `tpt-mathr` can provide:

```text
BigInt
BigRat
polynomial arithmetic
exact numerical evaluation
```

to solver workloads.

Potential architecture:

```text
tpt-solver
      │
      └── exact arithmetic
              │
              ▼
         tpt-mathr-arith
```

and:

```text
tpt-mathr-search
       │
       └── constraint subproblem
                │
                ▼
           tpt-solver
```

---

# 39. Integration With tpt-formal and tpt-telos

Verification should be layered.

```text
Level 1
local mathematical checker

Level 2
tpt-solver certificate checker

Level 3
tpt-formal verification

Level 4
tpt-telos contracts

Level 5
Lean / other proof assistant
```

Not every computation needs the highest level.

The system should allow the user to select the desired assurance level.

Example:

```text
Fast
  arithmetic only

Certified
  certificate checked

Formal
  verification obligation generated

Kernel
  Lean-verified theorem
```

---

# 40. Assurance Profiles

`tpt-mathr` should eventually support:

```text
Fast
```

maximum performance, minimum verification overhead.

```text
Checked
```

result independently checked by a local checker.

```text
Certified
```

certificate emitted and independently replayable.

```text
Formal
```

proof obligation passed to the formal verification stack.

```text
KernelVerified
```

accepted by a proof assistant kernel.

This makes verification an explicit engineering choice rather than an all-or-nothing design.

---

# 41. Security

Security-sensitive boundaries include:

* C ABI;
* Python extension;
* WASM;
* certificate parsing;
* untrusted search results;
* distributed workers;
* external proof files.

Requirements:

* bounds checking;
* malformed-input rejection;
* no undefined behaviour at FFI boundaries;
* explicit ownership;
* denial-of-service limits where appropriate;
* resource budgets;
* fuzz testing.

---

# 42. FFI Error Handling

The C API MUST NOT panic across the ABI boundary.

Functions should return:

```text
status code
output handle
```

where necessary.

Example:

```c
TptStatus tpt_bigint_mul(
    const TptBigInt* lhs,
    const TptBigInt* rhs,
    TptBigInt** result
);
```

Possible statuses:

```text
TPT_OK
TPT_NULL_ARGUMENT
TPT_INVALID_ARGUMENT
TPT_OUT_OF_MEMORY
TPT_INTERNAL_ERROR
```

Detailed error information may be obtained through an explicit error API.

---

# 43. WASM Constraints

WASM builds must not assume:

* native threads;
* SIMD;
* filesystem;
* operating-system timers;
* native dynamic linking.

Capabilities must be detected.

The WASM API should expose deterministic operations independently of browser APIs.

---

# 44. Dependency Policy

The mathematical core should remain dependency-light.

Preferred:

```text
Rust standard library
alloc
small, audited mathematical dependencies where justified
```

External arbitrary-precision libraries MUST NOT be required by `tpt-mathr-arith` itself for its core implementation.

Reference implementations may be used in:

```text
benchmarks
test harnesses
development tools
```

but not as runtime dependencies of the native arithmetic engine.

---

# 45. No C Dependency Requirement

The core must not depend on:

```text
GMP
MPFR
FLINT
```

for normal execution.

Those systems may be used for:

```text
differential testing
benchmarking
validation
```

where licensing and CI environment permit.

This distinction is important:

> **Reference implementation ≠ runtime dependency.**

---

# 46. Licensing

The TPT implementation shall use:

```text
MIT
```

unless a specific crate has a documented reason for another compatible permissive licence.

Dependencies must comply with the TPT licensing policy.

Benchmark-only dependencies may have separate licensing constraints provided they do not become runtime dependencies.

---

# 47. Documentation

Documentation shall contain:

```text
architecture.md
arithmetic.md
bigint.md
bigfloat.md
rational.md
polynomials.md
simd.md
verification.md
search.md
ffi.md
wasm.md
benchmarks.md
reproducibility.md
```

Each major mathematical algorithm should document:

* mathematical definition;
* algorithm;
* complexity;
* numerical constraints;
* implementation;
* testing strategy;
* verification strategy.

---

# 48. Example Rust API

Example:

```rust
use tpt_mathr_arith::{BigInt, BigUint, BigRat};

let a = BigInt::parse("12345678901234567890", 10)?;
let b = BigInt::from(987654321_i64);

let product = &a * &b;

let numerator = BigInt::from(22);
let denominator = BigUint::from(7_u64);

let rational = BigRat::new(numerator, denominator)?;

println!("{product}");
println!("{rational}");
```

The exact final API may change during implementation.

---

# 49. Example Mathematical Search

```rust
use tpt_mathr_search::{SearchProblem, SearchEngine};

struct CounterexampleSearch;

impl SearchProblem for CounterexampleSearch {
    type Candidate = u64;
    type Result = bool;
    type Error = SearchError;

    fn generate(
        &self,
        state: &mut SearchState
    ) -> Option<Self::Candidate> {
        state.next_candidate()
    }

    fn evaluate(
        &self,
        candidate: Self::Candidate
    ) -> Result<bool, Self::Error> {
        Ok(test_property(candidate))
    }
}
```

The result must carry explicit search bounds and verification status.

---

# 50. Example Certificate Workflow

```text
calculate
   │
   ▼
candidate result
   │
   ▼
certificate
   │
   ▼
tpt-mathr checker
   │
   ├── Accept
   ├── Reject
   └── Inconclusive
```

A higher-level proof can then be generated:

```text
certificate
   │
   ▼
Lean / tpt-formal / tpt-telos
```

---

# 51. Development Roadmap

The original 12-month roadmap is replaced with capability-driven milestones.

## Phase 0 — Architecture

Deliver:

* repository;
* workspace;
* dependency policy;
* arithmetic invariants;
* API design;
* benchmark harness;
* differential-test harness.

No performance claims.

---

## Phase 1 — BigUint / BigInt

Implement:

```text
Limb
BigUint
BigInt
```

Operations:

```text
add
sub
mul
div
rem
gcd
comparison
conversion
```

Algorithms:

```text
schoolbook
Karatsuba
```

Milestone:

> Correctness against differential tests across a large generated corpus.

---

## Phase 2 — Performance Engine

Implement:

```text
Toom-Cook
SIMD addition/subtraction
adaptive multiplication
division optimisation
modular arithmetic
```

Benchmark against:

```text
num-bigint
GMP
Python int
```

Milestone:

> Demonstrate measured competitive performance on defined benchmark classes.

---

## Phase 3 — BigRat

Implement:

```text
BigRat
normalisation
arithmetic
comparison
conversion
```

Integrate with:

```text
tpt-math-exact
```

Milestone:

> `tpt-math-exact` can optionally use the `tpt-mathr` arithmetic backend.

---

## Phase 4 — Polynomial Engine

Implement:

```text
Polynomial
multiplication
division
gcd
evaluation
interpolation
```

Add:

```text
Karatsuba
Toom-Cook
NTT/FFT
```

where justified by benchmarks.

---

## Phase 5 — BigFloat

Implement:

```text
precision
rounding
special values
core arithmetic
sqrt
transcendentals
```

Develop conformance tests against reference implementations.

No compatibility claim until conformance is demonstrated.

---

## Phase 6 — Verification

Implement:

```text
certificate framework
local checkers
proof object formats
tpt-solver integration
tpt-formal integration
tpt-telos integration
Lean export
```

Milestone:

> At least one major mathematical computation can be independently checked from a compact certificate.

---

## Phase 7 — Search

Implement:

```text
bounded search
parallel search
resumable search
counterexample search
certificate output
```

Integrate with:

```text
tpt-compute
```

where appropriate.

---

## Phase 8 — Syntaxis Integration

Integrate:

```text
tpt-syntaxis
```

for:

```text
high-performance numerical exploration
counterexample discovery
large integer computation
polynomial computation
candidate theorem generation
```

Milestone:

> Syntaxis can delegate computationally intensive mathematical exploration to `tpt-mathr`.

---

## Phase 9 — FFI

Implement:

```text
C ABI
Python
WASM
```

Package:

```text
PyPI wheels
native libraries
WASM packages
```

Milestone:

> External languages can consume the same deterministic arithmetic engine.

---

## Phase 10 — Advanced Acceleration

Investigate:

```text
GPU arithmetic
distributed search
advanced multiplication
specialised number theory
large-scale certificate generation
```

Only implement where benchmark evidence justifies the complexity.

---

# 52. Initial Success Criteria

`tpt-mathr v1.2.x` is successful when:

1. `BigUint` is correct.
2. `BigInt` is correct.
3. `BigRat` is correct.
4. Core arithmetic is independently differential-tested.
5. SIMD acceleration is measurable.
6. Multiplication strategy selection is benchmark-driven.
7. `tpt-math-exact` can consume the new backend.
8. Mathematical certificates can be generated.
9. Certificates can be independently checked.
10. `tpt-solver` integration works where useful.
11. Lean export works for at least a defined subset.
12. `tpt-syntaxis` can call the engine.
13. C ABI is stable and documented.
14. Python wheels are reproducible.
15. WASM builds successfully.
16. Search results distinguish failure from inconclusiveness.
17. No hidden global mathematical state is required.
18. The core remains independent of GMP/MPFR/FLINT.

---

# 53. Explicit Non-Goals

The following are NOT goals of `tpt-mathr` v1:

### Not a replacement for all of tpt-math

`tpt-math` remains the general mathematical ecosystem.

### Not a proof assistant

Lean, Coq and TPT verification systems remain responsible for formal proof.

### Not a general SMT solver

`tpt-solver` owns that problem.

### Not a general compute runtime

`tpt-compute` owns that problem.

### Not a GPU runtime

`tpt-gpu` owns that problem.

### Not a theorem-discovery application

`tpt-syntaxis` owns that problem.

### Not an AI system

AI may drive search through external systems, but mathematical correctness cannot depend on AI.

### Not a sandbox

Execution sandboxing is a separate infrastructure concern.

---

# 54. Long-Term Architecture

The mature TPT mathematics ecosystem should eventually look approximately like:

```text
                            USER / APPLICATION
                                   │
                    ┌──────────────┴──────────────┐
                    │                             │
             tpt-syntaxis                    TPT applications
                    │                             │
             theorem discovery               domain mathematics
                    │                             │
                    └──────────────┬──────────────┘
                                   │
                              tpt-math
                                   │
                 ┌─────────────────┼─────────────────┐
                 │                 │                 │
          tpt-mathr           tpt-solver        tpt-formal
          arithmetic          constraints       verification
                 │                 │                 │
                 └─────────────────┼─────────────────┘
                                   │
                             tpt-telos
                                   │
                         formal contracts /
                         verification layer
                                   │
                              Lean / Coq
```

Compute infrastructure sits underneath:

```text
                 mathematical workloads
                         │
             ┌───────────┼───────────┐
             ▼           ▼           ▼
           CPU         tpt-gpu    distributed
         SIMD/threads               compute
```

---

# 55. Ultimate Vision

`tpt-mathr` is not intended to win a benchmark by replacing one existing arbitrary-precision library.

Its purpose is larger.

It should become the numerical engine that allows TPT to move from:

```text
mathematical idea
```

to:

```text
computation
      ↓
search
      ↓
candidate
      ↓
certificate
      ↓
independent verification
      ↓
formal proof
      ↓
human interpretation
```

The fundamental distinction is:

> **Computation finds things. Verification establishes what those findings mean.**

`tpt-mathr` supplies the computational depth.

`tpt-math` supplies the mathematical breadth.

`tpt-solver` supplies certified constraint reasoning.

`tpt-formal` and `tpt-telos` supply verification infrastructure.

`tpt-syntaxis` supplies mathematical discovery and orchestration.

Together they form a coherent TPT mathematical computing stack rather than a collection of overlapping repositories.

---

# 56. Final Architectural Principle

The project shall be guided by one rule:

> **Do not duplicate mathematical infrastructure merely because a new application needs it. Extend the lowest existing TPT layer that owns the concept, and create a new crate only when the capability represents a genuinely new abstraction or implementation boundary.**

For `tpt-mathr`, the genuinely new capability is:

> **high-performance native arbitrary-precision computation coupled to reproducible search and independently verifiable mathematical results.**

That is the reason for the repository.

Everything else should be composed from the existing TPT ecosystem wherever practical.


---

# Amendments from the v1.2 review (normative)

These amendments were adopted as standing decisions in the v1.2 review
(preserved in `docs/history/spec1.2.txt`). Where an amendment conflicts
with the base text above, **the amendment governs**. Each cites the clauses
it supersedes.

## A0 — Licensing (supersedes §1 header "Primary License: MIT" and §46)

The project is dual-licensed **MIT OR Apache-2.0**. Every crate declares
`license = "MIT OR Apache-2.0"`; every source file carries the SPDX header.
Dependency policy (§44) is unchanged, with the sharpened standing decision:
no Apache-2.0-only dependencies in shipped crates; no GPL/LGPL/MPL in
shipped crates; MIT-compatible permissive licences only (BSD, ISC, Zlib,
Unlicense/CC0 included).

## A1 — OOM handling: fallible allocation (supersedes §33 `AllocationFailure` as an ordinary error)

Rust's ordinary allocation paths abort the process on allocation failure;
they cannot return errors. `AllocationFailure` as a *recoverable* error is
therefore an architectural commitment, not a free consequence of the type
system:

* All large-data growth paths use `Vec::try_reserve()` /
  `try_reserve_exact()`, encapsulated behind internal allocation helpers
  (e.g. `reserve_limbs`), so the arithmetic implementation does not know
  which allocation mechanism is underneath.
* `allocator-api2` is deferred; start with `try_reserve`.

## A2 — Explicit resource limits (extends §33)

Beyond `AllocationFailure`, operations distinguish `ResourceExhausted`. An
explicit `ResourceLimits` structure carries the budgets:

```text
memory budget              max_memory_bytes
integer operand budget     max_integer_bits
polynomial degree budget   max_polynomial_degree
search depth budget        max_search_depth
certificate size budget    max_certificate_size_bytes
execution time budget      max_execution_time
```

A worker must not have to discover it consumed 64 GB before failing: limit
checks are cooperative, explicit, and return `ResourceExhausted` before the
budget is crossed. `ResourceExhausted` and `Inconclusive` MUST NOT be
treated as `False` or `Verified` (§33 unchanged and reinforced).

## A3 — Limb sizing (supersedes the unconditional §6 `type Limb = u64`)

```rust
#[cfg(target_pointer_width = "64")]
type Limb = u64;   // DoubleLimb = u128
#[cfg(target_pointer_width = "32")]
type Limb = u32;   // DoubleLimb = u64
```

* 32-bit/wasm32 builds use u32 limbs; the wider intermediate is
  `DoubleLimb`.
* The limb type is **private**. The public API is `BigInt`, never
  `BigInt<Limb>`. A `LimbConfig`-style trait may exist as an internal
  abstraction, but no generic limb parameter is exposed. This is a hard
  architectural requirement.
* Result: x86_64/aarch64 → u64 limbs; wasm32/armv7 → u32 limbs — behind one
  public `BigInt`.

## A4 — Symbolic allocation strategy (extends §12/§25)

Two distinct allocation problems, two strategies:

* **Arithmetic** (`BigInt`, `BigRat`, `Polynomial`): normal heap
  allocation, reusable buffers, scratch allocation.
* **Symbolic** (AST, e-graph nodes, rewrite candidates, proof traces):
  arena allocation (e.g. `bumpalo`, license-checked) for *short-lived*
  state — expression construction, rewrite candidates, proof traces,
  branch-local state — enabling O(1) bulk disposal of failed branches.

The **long-lived e-graph** is NOT arena-allocated; it uses packed indexed
storage (`Vec<EClass>` / `Vec<ENode>` with `EClassId` / `ENodeId`), which
also removes per-node allocation churn — potentially a larger win than the
arena itself.

## A5 — WASM threading (supersedes absolute §43 wording)

The precise statement: *the baseline WASM target must function without
threads; a threaded WASM target may optionally use WebAssembly shared
memory and atomics where the host environment supports them.*

* Baseline: single-threaded, works everywhere, no filesystem or timers
  assumed.
* Optional later: `RUSTFLAGS="-C
  target-feature=+atomics,+bulk-memory,+mutable-globals"` +
  `wasm-bindgen-rayon`. Cross-origin isolation (SharedArrayBuffer
  requirements) is an application/deployment concern, not something
  tpt-mathr bakes into the mathematical core.
* Parallelism is an **execution backend** of `tpt-mathr-search` (native
  threads / distributed / GPU / WASM workers), never part of core
  arithmetic design.

## A6 — FFI out-parameter ABI (supersedes the §42 example signature)

The canonical ABI for every allocation-returning function is:

```c
TptStatus tpt_bigint_mul(const TptBigInt* lhs, const TptBigInt* rhs, TptBigInt** out);
```

Return value = status; result = out-parameter. This removes ambiguity
around allocation failure, invalid arguments, overflow/resource limits, and
internal failures. Struct-by-value returns across the C ABI are prohibited
(sysv64 vs Microsoft x64 conventions differ). Status codes: `TPT_OK`,
`TPT_NULL_ARGUMENT`, `TPT_INVALID_ARGUMENT`, `TPT_OUT_OF_MEMORY`,
resource-limit, cancelled, `TPT_INTERNAL_ERROR`, plus dedicated
inconclusive and verification-failure codes so that inconclusive evidence
can never be presented as success or as a verdict.

## A7 — Cooperative cancellation (extends §13/§14)

Distributed search requires coordinator-driven cooperative cancellation:
when worker B finds the counterexample, the coordinator cancels A, C, D
immediately. The search and long-operation APIs carry:

```rust
pub struct CancellationToken { /* cheap clone, atomic flag, no_std-friendly */ }
```

Cancellation is cooperative (polled at safety points) and shares machinery
with user cancellation, timeouts, memory budgets, distributed cancellation,
and browser worker termination.

## A8 — Parallelism: measure, never assume (extends §27/§28)

"Huge integers + concurrency = advantage over GMP" is rejected as a design
assumption. Multiplication may have serial dependencies even when the
workload parallelises. The architecture distinguishes:

* **Algorithm-level parallelism** (hard: parallel stages inside one huge
  multiply; scaling depends on operand size and algorithm), from
* **Workload-level parallelism** (easy: f(1), f(2), f(3), … distributes
  almost perfectly).

Benchmarks measure 1/2/4/8+ threads; no linear-scaling assumption is built
into any API, and no performance claim exists without benchmark evidence.

## A9 — Public API non-exposure (extends §23)

The public API must not expose any of the above implementation decisions.
Users see `BigInt`, `BigRat`, `BigFloat`, `Polynomial` — not
`BigInt<u64>`, `BigInt<Wasm32Limb>`, `ArenaBigInt`, or `ParallelBigInt`.
The library stays pleasant to use while the implementation becomes
considerably more sophisticated underneath.
