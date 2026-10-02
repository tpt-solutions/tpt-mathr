# tpt-mathr — Project Checklist

**Organisation:** TPT Solutions · **Language:** Rust (Edition 2024) · **Spec:** v1.2.0 (v1.1 + v1.2 review changes)
**License:** dual `MIT OR Apache-2.0`

## Standing decisions

- [x] Dual licensed MIT **or** Apache-2.0 (replaces v1.1 "MIT only")
- [x] Dependencies must be MIT-compatible: MIT, MIT/Apache-2.0 dual, BSD, ISC, Zlib, Unlicense/CC0. **No Apache-2.0-only deps.** No GPL/LGPL/MPL in shipped crates
- [x] GMP / MPFR / FLINT / Python: dev-dependencies and benchmarks only, never runtime
- [x] Cargo workspace, multi-crate, published to crates.io
- [x] Crates: `tpt-mathr-base`, `-arith`, `-poly`, `-symbolic`, `-search`, `-verify`, `-ffi`, `-cli` + thin `tpt-mathr` facade (core/compute/sandbox stay removed)
- [x] `no_std + alloc` for base/arith/poly/symbolic; `std` behind a feature
- [x] Safe Rust baseline; audited `unsafe` confined to one limb/SIMD kernel module; `#![forbid(unsafe_code)]` elsewhere except ffi/wasm
- [x] Stable Rust, pinned and documented MSRV (≥ 1.85 for Edition 2024); nightly only for optional experiments
- [x] Public API never exposes limb type, arenas, or parallelism (`BigInt`, not `BigInt<u64>`)
- [x] Existing TPT repos (tpt-math, tpt-math-exact, tpt-math-symbolic, tpt-formal, tpt-telos, tpt-solver, tpt-compute, tpt-gpu, tpt-syntaxis) exist on GitHub; each integration starts with an interface review

---

## Phase 0 — Foundations & governance
- [ ] Create Cargo workspace and crate skeletons (base, arith, poly, symbolic, search, verify, ffi, cli, facade)
- [ ] Add `LICENSE-MIT` and `LICENSE-APACHE`; set `license = "MIT OR Apache-2.0"` in every manifest
- [ ] Add SPDX headers / copyright "TPT Solutions" convention
- [ ] README, CHANGELOG, CONTRIBUTING (state dual-license contribution terms)
- [ ] Add `cargo-deny` config: allow-list of permissive licenses, deny Apache-only / copyleft
- [ ] Audit any proposed dependency (bumpalo, rayon, pyo3, wasm-bindgen, cbindgen, proptest, etc.) against the license policy
- [ ] Pin toolchain (`rust-toolchain.toml`) and document MSRV policy
- [ ] CI: fmt, clippy, tests, `cargo-deny`, docs build
- [ ] CI targets: x86_64, aarch64, a 32-bit target, `wasm32-unknown-unknown`
- [ ] CI: Miri job for the unsafe kernel module
- [ ] Write unsafe policy; enforce `forbid(unsafe_code)` per crate
- [ ] Docs skeleton: architecture, arithmetic, bigint, bigfloat, rational, polynomials, simd, verification, search, ffi, wasm, benchmarks, reproducibility
- [ ] Record architecture decisions (v1.2 deltas, dual license, base crate, facade) in `docs/architecture.md`
- [ ] Document arithmetic invariants (normalised limbs, canonical zero, BigRat reduced, etc.)
- [ ] Benchmark harness (hardware, compiler, flags, operand size, op, threads, backend, version recorded)
- [ ] Differential-test harness vs GMP and Python `int` (dev-only)
- [ ] Reconcile `spec.txt` / `spec1.2.txt` into a single canonical spec in `docs/`
- [ ] **Milestone:** workspace builds and CI is green on all targets; no perf claims

## Phase 1 — `tpt-mathr-base` (shared infrastructure)
- [ ] Error enum (ParseError, Overflow, Underflow, DivisionByZero, InvalidPrecision, InvalidRadix, AllocationFailure, ResourceExhausted, VerificationFailure, UnsupportedOperation, InvalidCertificate, Inconclusive)
- [ ] Guarantee `ResourceExhausted`/`Inconclusive` can never map to false/verified
- [ ] `ResourceLimits` (memory bytes, integer bits, polynomial degree, search depth, certificate size, execution time)
- [ ] Cooperative limit-checking API (usable from `no_std`; time source behind `std`)
- [ ] `CancellationToken` (cheap clone, atomic, no_std-friendly)
- [ ] Fallible allocation helpers (`reserve_limbs` etc. via `try_reserve` / `try_reserve_exact`)
- [ ] Decide when to adopt `allocator-api2` (deferred; start with `try_reserve`)
- [ ] FFI status-code enum (`TPT_OK`, `TPT_NULL_ARGUMENT`, `TPT_INVALID_ARGUMENT`, `TPT_OUT_OF_MEMORY`, resource-limit, cancelled, `TPT_INTERNAL_ERROR`) and error→status mapping
- [ ] Unit tests incl. simulated allocation failure
- [ ] **Milestone:** all other crates can depend on base with `no_std + alloc`

## Phase 2 — `tpt-mathr-arith`: BigUint / BigInt
- [ ] Conditional `Limb`: u64 on 64-bit, u32 on 32-bit/wasm32; internal `DoubleLimb`; private
- [ ] `BigUint` with canonical normalised representation
- [ ] `BigInt` (sign + magnitude), canonical zero
- [ ] add, sub, comparison
- [ ] Parsing/formatting for radix 2–36; conversions to/from primitives
- [ ] `MulStrategy` abstraction; schoolbook multiplication
- [ ] Karatsuba multiplication
- [ ] Division and remainder
- [ ] gcd, extended gcd, lcm, pow, isqrt
- [ ] All growth paths use fallible allocation and honour `ResourceLimits` (`max_integer_bits`, memory)
- [ ] Cancellation checks in long operations
- [ ] Invariant checker (debug/test) for all invariants
- [ ] Unit + property tests (commutativity, associativity, distributivity, gcd divisibility)
- [ ] Differential tests vs GMP / Python on large generated corpus
- [ ] Metamorphic tests
- [ ] Fuzz targets: parsing, arithmetic, serialisation
- [ ] Run test suite on 32-bit and wasm32 limb configurations
- [ ] **Milestone:** correctness vs differential corpus; no hidden global state

## Phase 3 — Performance engine
- [ ] Toom-Cook multiplication
- [ ] NTT / FFT multiplication
- [ ] Benchmark-derived threshold selection (no hard-coded universal winner)
- [ ] Division optimisation
- [ ] Montgomery and Barrett reduction; `ModInt`; modular exponentiation (`pow_mod`), `mod_inverse`
- [ ] Number theory: primality testing, factorisation helpers (no speed promises)
- [ ] Isolated unsafe limb/SIMD kernel module; scalar reference implementations kept
- [ ] SIMD: AVX2, AVX-512 (where beneficial), NEON, WASM SIMD with runtime dispatch / capability detection
- [ ] Differential + Miri + fuzz coverage of every unsafe/SIMD path vs scalar
- [ ] Scratch-buffer reuse and capacity reuse in arithmetic hot paths
- [ ] Benchmark matrix: 32 b → 100 MiB; add/sub/mul/div/mod/gcd/pow/pow_mod/cmp/convert
- [ ] Parallel-scaling benchmarks at 1/2/4/8+ threads (algorithm-level vs workload-level); no linear-scaling assumption
- [ ] Compare vs num-bigint, GMP, Python int; publish results with full metadata
- [ ] CI benchmark regression threshold where infrastructure permits
- [ ] **Milestone:** measured competitive performance on defined benchmark classes

## Phase 4 — BigRat & `tpt-math-exact` backend
- [ ] `BigRat` (BigInt numerator, BigUint denominator, always reduced, denominator > 0)
- [ ] +, −, ×, ÷, %, comparison, floor, ceil, round, numerator/denominator accessors
- [ ] Property, differential and fuzz tests
- [ ] Interface review of `tpt-math-exact` backend abstraction
- [ ] Implement `tpt-mathr` backend behind a feature flag in `tpt-math-exact`
- [ ] Rational benchmarks (add, mul, div, reduce, compare)
- [ ] **Milestone:** `tpt-math-exact` can optionally use the tpt-mathr backend

## Phase 5 — `tpt-mathr-poly`
- [ ] Interface review of `tpt-math` polynomial types
- [ ] `Polynomial<T>`, `SparsePolynomial<T>`, `FormalPowerSeries<T>`
- [ ] add, sub, mul, div, gcd, evaluate, differentiate, integrate, compose, interpolate
- [ ] Adaptive multiplication: schoolbook / Karatsuba / Toom-Cook / NTT / FFT, justified by benchmarks
- [ ] Factorisation (scoped, documented limits)
- [ ] `max_polynomial_degree` limit enforcement; fallible allocation; cancellation
- [ ] Integration with `tpt-math-symbolic` (no independent CAS)
- [ ] Parallel polynomial multiplication with measured scaling
- [ ] Tests, differential tests, benchmarks
- [ ] **Milestone:** polynomial engine usable from tpt-math-symbolic with benchmarks

## Phase 6 — BigFloat
- [ ] Write BigFloat spec: representation, overflow/underflow, exponent limits, NaN, infinity, signed zero, precision propagation, conversion semantics
- [ ] Sign / significand / exponent / precision representation
- [ ] Rounding modes: Nearest, TowardZero, TowardPositive, TowardNegative
- [ ] Core arithmetic, comparison, integer and binary-float conversion, decimal conversion
- [ ] sqrt, powers
- [ ] exp, log
- [ ] Trigonometric functions (as maturity permits)
- [ ] Deterministic results independent of thread count / SIMD path
- [ ] Conformance tests vs reference implementations (MPFR dev-only)
- [ ] Benchmarks (add, mul, div, sqrt, exp, log, sin, cos, conversion)
- [ ] **Milestone:** conformance suite passing; no MPFR-compatibility claim until demonstrated

## Phase 7 — `tpt-mathr-symbolic`
- [ ] Decide what cannot live in `tpt-math-symbolic` (interface review first)
- [ ] Proof-aware expression representation; canonical forms; algebraic normalisation
- [ ] Temporary allocation: arena (bumpalo or custom; license-checked) for expression construction, rewrite candidates, proof traces, branch-local state
- [ ] Interned nodes with stable IDs
- [ ] Long-lived e-graph in packed indexed storage (`Vec<EClass>` / `Vec<ENode>`, `EClassId`/`ENodeId`), not arena-allocated
- [ ] Bulk disposal of failed search branches
- [ ] Rewrite provenance and rewrite certificates
- [ ] Equivalence checking with a path to independent validation
- [ ] Benchmarks: allocation churn, rewrite throughput
- [ ] **Milestone:** rewrites emit certificates that an independent checker accepts

## Phase 8 — Certificates & `tpt-mathr-verify`
- [ ] Certificate model and serialisation (prime, factorisation, polynomial identity, linear algebra, SAT/SMT, counterexample witness, interval enclosure, exact rational witness)
- [ ] Local certificate checkers (small trusted kernel); checker never converts resource exhaustion into acceptance
- [ ] Independent verification of FFT/NTT product candidates
- [ ] Verification-obligation generation
- [ ] Assurance profiles: Fast, Checked, Certified, Formal, KernelVerified
- [ ] Interface review + integration: `tpt-solver` (core/check split reused conceptually)
- [ ] Interface review + integration: `tpt-formal`
- [ ] Interface review + integration: `tpt-telos`
- [ ] Lean export: expressions, theorem statements, proofs/certificates; replay
- [ ] Coq export (only if justified)
- [ ] Certificate parser hardening (bounds, size limits) + fuzzing
- [ ] Define and document trusted computing base
- [ ] **Milestone:** one major computation independently checked from a compact certificate; Lean export for a defined subset

## Phase 9 — `tpt-mathr-search`
- [ ] `SearchProblem` trait and `SearchEngine`
- [ ] Result taxonomy: Found, NotFoundWithinBounds, Exhausted, Inconclusive, Verified, Counterexample (not-found ≠ proved-absent)
- [ ] Deterministic work-unit identity; partitioning independent of execution order
- [ ] Resumable jobs and persisted metadata (algorithm, version, seed, partition, precision, status)
- [ ] Bounded / exhaustive search, parameter sweeps, Monte Carlo
- [ ] Rayon backend (license-checked); execution-backend abstraction (native threads / distributed / GPU / WASM workers)
- [ ] `CancellationToken` and `ResourceLimits` checked cooperatively (memory, time, depth, certificate size)
- [ ] Worker failure isolation: one worker's OOM must not take down the node
- [ ] Interface review + integration with `tpt-compute`
- [ ] Certificate output for results; never trust a worker's success report
- [ ] Parallel-scaling benchmarks (single-thread baseline vs 2/4/8+)
- [ ] **Milestone:** resumable, cancellable, reproducible counterexample search with certificates

## Phase 10 — `tpt-syntaxis` integration
- [ ] Interface review of tpt-syntaxis discovery pipeline
- [ ] Expose arith / poly / search / verify entry points for Syntaxis
- [ ] Delegate numerical exploration, counterexample discovery, large-integer and polynomial computation
- [ ] End-to-end flow: conjecture → search → candidate → verify → Lean
- [ ] **Milestone:** Syntaxis delegates computationally intensive exploration to tpt-mathr

## Phase 11 — FFI, CLI & distribution (`tpt-mathr-ffi`, `-cli`, facade)
- [ ] C ABI: opaque handles; canonical `TptStatus fn(..., T** out)`; explicit free functions
- [ ] No Rust layout exposed; ABI versioning; null handling; documented lifetimes; thread-safety rules
- [ ] No panics across ABI boundary (catch_unwind / `panic=abort` strategy)
- [ ] Explicit error-detail API
- [ ] Generate C header (cbindgen or equivalent; license-checked)
- [ ] PyO3 Python API (BigInt, BigRat, batch ops, minimal copying); reproducible wheels
- [ ] WASM bindings (wasm-bindgen): BigInt, BigRat, selected BigFloat, polynomials
- [ ] WASM baseline is single-threaded; feature/capability detection (SIMD, no FS/timers assumed)
- [ ] `tpt-mathr-cli`: calc, bigint, rational, float, poly, prime, search, verify, benchmark
- [ ] Thin `tpt-mathr` facade crate with feature-flagged re-exports
- [ ] FFI/WASM/Python fuzzing and security tests (malformed input, DoS limits)
- [ ] Packaging: crates.io, PyPI wheels, native libraries, WASM package
- [ ] **Milestone:** external languages consume the same deterministic engine; WASM builds

## Phase 12 — Advanced acceleration (evidence-gated)
- [ ] Threaded WASM target: atomics + shared memory + `wasm-bindgen-rayon`; documented build flags; cross-origin-isolation requirements documented (deployment concern)
- [ ] GPU arithmetic kernels via `tpt-gpu`/TPTIR (no second GPU runtime); independent validation of GPU results
- [ ] Distributed search: coordinator/workers, work-unit protocol, verifier, cancellation propagation across workers
- [ ] Advanced multiplication and specialised number theory
- [ ] Large-scale certificate generation
- [ ] Each item requires benchmark evidence before merge
- [ ] **Milestone:** acceleration backends validated and documented

## Phase 13 — Release hardening
- [ ] All docs complete (algorithm definition, complexity, constraints, testing + verification strategy)
- [ ] Publish benchmark data and methodology
- [ ] Security review of FFI, Python, WASM, certificate parsing, distributed workers
- [ ] Final dependency license audit (`cargo-deny` clean; no Apache-only deps)
- [ ] Reproducibility check (inputs, version, precision, rounding, backend, CPU features, seed, partition)
- [ ] Verify spec §52 success criteria 1–18
- [ ] Tag release, publish to crates.io / PyPI, update CHANGELOG
- [ ] **Milestone:** `tpt-mathr v1.2.x` released

---

## Open questions / TBD
- [ ] Final MSRV number
- [ ] Arena choice: `bumpalo` vs custom (confirm license)
- [ ] Which `tpt-math-symbolic` capabilities, if any, justify keeping `tpt-mathr-symbolic` as a separate crate
- [ ] Benchmark CI hardware and regression threshold
- [ ] Exact certificate serialisation format
