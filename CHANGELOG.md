# Changelog

All notable changes to `tpt-mathr` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versioning follows [Semantic Versioning](https://semver.org/). The project is
dual-licensed MIT OR Apache-2.0; this file and every source file carry the
SPDX header `SPDX-License-Identifier: MIT OR Apache-2.0`.

## [Unreleased]

### Added — Phase 0: Foundations & governance
- Cargo workspace with the nine-crate layout from spec §4:
  `tpt-mathr-base`, `-arith`, `-poly`, `-symbolic`, `-search`, `-verify`,
  `-ffi`, `-cli`, and the thin `tpt-mathr` facade
  (`tpt-mathr-core`/`-compute`/`-sandbox` stay removed per spec).
- Dual `MIT OR Apache-2.0` licensing: `LICENSE-MIT`, `LICENSE-APACHE`,
  `license` in every manifest, SPDX headers in every source file.
- `deny.toml`: license allow-list (MIT-compatible set, no copyleft),
  advisory and sources policy; dependency audit process in
  `docs/dependency-audit.md`.
- Pinned toolchain (`rust-toolchain.toml`) and MSRV 1.85 policy.
- CI: fmt, clippy (`-D warnings`), tests, `cargo-deny`, docs build;
  targets x86_64, aarch64, i686, wasm32-unknown-unknown; Miri job for the
  (future) unsafe kernel module.
- Documentation skeleton (architecture, arithmetic, bigint, bigfloat,
  rational, polynomials, simd, verification, search, ffi, wasm, benchmarks,
  reproducibility) plus `docs/invariants.md`, `docs/unsafe-policy.md`,
  `docs/dependency-audit.md`, and the reconciled canonical spec
  `docs/spec.md` (supersedes `spec.txt` / `spec1.1.txt` / `spec1.2.txt`,
  which are preserved under `docs/history/`).
- Benchmark harness skeleton (`benchmarks/`) recording hardware, compiler,
  flags, operand size, operation, threads, backend, and version.
- Differential-test harness skeleton (`tests/differential/`, dev-only)
  against Python `int`, feature-gated.

### Added — Phase 1: `tpt-mathr-base`
- `Error` enum implementing the spec §33 error model (`ParseError`,
  `Overflow`, `Underflow`, `DivisionByZero`, `InvalidPrecision`,
  `InvalidRadix`, `AllocationFailure`, `VerificationFailure`,
  `UnsupportedOperation`, `InvalidCertificate`, `ResourceExhausted`,
  `Inconclusive`, plus `Cancelled` for cooperative cancellation), with the
  guarantee that `ResourceExhausted`/`Inconclusive` can never be coerced
  into a negative or verified result (`Error::definite_refutation`,
  `is_inconclusive`, `is_resource_exhausted`, pinned tests).
- `ResourceLimits` (memory bytes, integer bits, polynomial degree, search
  depth, certificate size, execution time) with cooperative check API
  usable from `no_std`.
- `CancellationToken`: cheap `Arc`-backed clone, atomic flag, no_std-safe,
  checked cooperatively.
- Fallible allocation helpers (`reserve`, `reserve_exact`,
  `try_with_capacity`, `resize_with_reserve`) over `try_reserve` with a
  portable capacity-overflow pre-check; unit tests include simulated
  allocation failure.
- FFI status-code enum `TptStatus` (`TPT_OK`, `TPT_NULL_ARGUMENT`,
  `TPT_INVALID_ARGUMENT`, `TPT_OUT_OF_MEMORY`, `TPT_RESOURCE_EXHAUSTED`,
  `TPT_CANCELLED`, `TPT_INCONCLUSIVE`, `TPT_VERIFICATION_FAILURE`,
  `TPT_INTERNAL_ERROR`) with the total, audited error→status mapping.
- `std`-feature cooperative `Deadline` time source.

### Added — Phase 2: `tpt-mathr-arith` BigUint / BigInt
- Private platform-conditional limb layer: `Limb` = u64 on 64-bit targets,
  u32 on 32-bit/wasm32, with `DoubleLimb`/`SignedDouble` intermediates and
  no public limb exposure (v1.2 delta A3).
- `BigUint`: canonical normalised little-endian representation (I1/I2),
  fallible `add`/`checked_sub`, comparison, shifts, bit tests,
  `bit_len`/`trailing_zeros`, multiplication via the strategy table
  (integer-bit budgets checked before allocation, I9), truncated
  `div_rem` (I5), `from_bytes_le`/`to_bytes_le`.
- `BigInt`: sign + magnitude with canonical zero (I3), full signed
  arithmetic, truncated division with sign-correct remainders (I5),
  sign-correct `pow`, `isqrt` refusing negatives (I7).
- Multiplication architecture (spec §7): `MulStrategy` trait, schoolbook
  and Karatsuba implementations, threshold dispatch with provisional
  constants explicitly scheduled for benchmark-derived replacement.
- Division: single-limb short division plus Knuth Algorithm D (TAOCP
  4.3.1 / Hacker's Delight formulation) with normalisation, quotient
  correction, and add-back.
- Number theory: binary gcd, extended gcd with non-negative gcd and exact
  Bezout identity (I6), lcm identity, `pow` by squaring, `isqrt` by
  Newton iteration (I7).
- Radix layer: chunked parsing/formatting for radix 2–36 with byte-position
  parse errors, canonical lowercase output (I4), signed forms, and
  `max_integer_bits` enforcement on parse (I9).
- `Progress` context: limits + cancellation threaded through every fallible
  operation (I9, deltas A2/A7); long operations poll the token.
- Invariant checker `assert_invariants` on both types; debug-pinned in
  every operation.
- Tests: 52 unit tests, 10 property tests (commutativity, associativity,
  distributivity, gcd divisibility, I5 division identity, I7 sqrt bounds,
  dispatch-vs-schoolbook equivalence via the feature-gated reference
  path), and a Python-differential corpus of ~1,100 signed cases up to
  ~1,280 bits (add/sub/mul/divrem/gcd/pow/isqrt, decimal and hex round
  trips) — all matching CPython exactly.
- Fuzz targets (`fuzz/`, separate cargo-fuzz workspace): radix parsing
  round-trip, arithmetic identities, byte serialisation round-trip.
- Verified green on x86_64 (unit + property + differential), i686
  (unit + property + harness), wasm32-unknown-unknown (compile), and
  aarch64-pc-windows-msvc (compile).

### Added — Phase 4: `BigRat` exact rationals
- `BigRat`: reduced signed fraction (`BigInt` numerator, positive
  `BigUint` denominator, canonical `0/1`), with fallible `add`/`sub`/
  `mul`/`div`/`rem`, cross-multiplying comparison, `floor`/`ceil`/
  `round` (ties **away from zero** — normative, pinned by tests),
  `inv`/`neg`/`abs`, `numerator`/`denominator` accessors, and radix
  `num/den` parsing and formatting (I4-style round trip).
- Differential corpus vs Python `fractions.Fraction`: ~480 cases covering
  arithmetic (including truncated remainder via an explicit
  `a - trunc(a/b)·b` reference), floor/ceil, away-from-zero rounding
  (Python's `round` is half-even and deliberately not used as reference),
  and external confirmation of the always-reduced invariant.
- `rational_invariants` fuzz target (field identities, division-identity
  analogue, rounding bounds).
- Rational benchmark suite (reduce/add/mul/div/compare at 64-bit and
  4096-bit operand classes) recorded through the harness's metadata
  contract.
- Interface review of `tpt-math-exact` (Phase 4): findings and recorded
  decision in `docs/tpt-math-exact-interface-review.md` — the backend
  feature flag lands in the `tpt-math` repository (feature-gated alias +
  infallible adapter); deferred there by agreement.

### Fixed
- Python batch harness no longer deadlocks on Windows: stdin is fed from a
  writer thread so the child's stdout pipe drains concurrently.
- `BigRat::div` dropped the divisor's sign in its first draft (caught by
  the randomised invariant suite); division now routes through the exact
  inverse. `BigRat::round` mis-rounded values ≥ 1 (it classified the whole
  value instead of the fractional remainder); rewritten as
  truncate-then-classify.

### Notes
- No performance claims: benchmarks exist to *establish* baselines first
  (milestone policy, Phase 0).
