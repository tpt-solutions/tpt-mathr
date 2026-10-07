# tpt-mathr

**The Proof Telescope** — the high-performance, arbitrary-precision, proof-aware numerical engine for the TPT mathematical computing ecosystem.

`tpt-mathr` is *not* a replacement for `tpt-math`; it is the computational substrate underneath it, for when ordinary mathematical computing is insufficient:

> compute enormous mathematical objects, search enormous mathematical spaces, generate evidence, and independently verify the resulting claims.

**Status: early development (Phase 0–2 of the roadmap).** The workspace builds and is fully tested, but the public API is not yet stable and **no performance claims are made**. See [todo.md](todo.md) for the live checklist and [docs/spec.md](docs/spec.md) for the canonical specification.

## Design principles

- **Search is not proof.** A computed candidate is not a verified result; evidence flows through independent checkers.
- **Determinism.** Identical inputs, precision, rounding mode, and configuration give identical results — independent of thread count or SIMD path.
- **Fallible allocation.** Large-data paths use `try_reserve`; allocation failure is an error, never a process abort.
- **Explicit budgets.** Memory, integer bits, polynomial degree, search depth, certificate size, and execution time are cooperative limits (`ResourceLimits`), not hidden global state.
- **Cooperative cancellation.** Long operations poll a cheap, cloneable `CancellationToken`.
- **Minimal audited unsafe.** Pure safe Rust except one isolated limb/SIMD kernel module and the FFI boundary.
- **Dependency-light core.** No runtime dependencies; no GMP/MPFR/FLINT/Python at runtime (reference implementations for testing only).

## Workspace

| Crate | Purpose | no_std |
|---|---|---|
| [`tpt-mathr-base`](crates/tpt-mathr-base) | Errors, resource limits, cancellation, fallible allocation, FFI status codes | yes (`std` feature for deadlines) |
| [`tpt-mathr-arith`](crates/tpt-mathr-arith) | `BigUint`, `BigInt`, modular arithmetic, number theory | yes |
| [`tpt-mathr-poly`](crates/tpt-mathr-poly) | Polynomial arithmetic over exact coefficients | yes |
| [`tpt-mathr-symbolic`](crates/tpt-mathr-symbolic) | Proof-aware rewriting, packed e-graphs | yes |
| [`tpt-mathr-search`](crates/tpt-mathr-search) | Reproducible, cancellable, certificate-producing search | yes |
| [`tpt-mathr-verify`](crates/tpt-mathr-verify) | Independent certificate checkers, assurance profiles | yes |
| [`tpt-mathr-ffi`](crates/tpt-mathr-ffi) | C ABI, Python, WASM bindings | hosted |
| [`tpt-mathr-cli`](crates/tpt-mathr-cli) | Command-line interface | hosted |
| [`tpt-mathr`](crates/tpt-mathr) | Thin facade with feature-flagged re-exports | depends on features |

## Development

```bash
cargo build --workspace          # build
cargo test  --workspace          # unit + doc tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo deny check                 # license / advisory / bans policy
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

- **Toolchain:** pinned in [`rust-toolchain.toml`](rust-toolchain.toml); MSRV is **1.85** (Edition 2024).
- **License:** dual [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE), copyright TPT Solutions. Contributions are accepted under the same dual license — see [CONTRIBUTING.md](CONTRIBUTING.md).
- **Dependency policy:** MIT-compatible permissive licenses only; no Apache-2.0-only or copyleft deps in shipped crates. Every new dependency requires a review in [docs/dependency-audit.md](docs/dependency-audit.md).
- **Documentation:** start at [docs/architecture.md](docs/architecture.md); the reconciled specification is [docs/spec.md](docs/spec.md).

## Ecosystem context

`tpt-math` defines mathematical functionality; `tpt-mathr` provides high-performance and high-assurance machinery underneath it. Integrations (tpt-math-exact, tpt-math-symbolic, tpt-formal, tpt-telos, tpt-solver, tpt-compute, tpt-gpu, tpt-syntaxis) each begin with an interface review — see spec §3 and the roadmap phases.
