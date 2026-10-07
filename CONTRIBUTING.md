# Contributing to tpt-mathr

Thank you for contributing to **The Proof Telescope**. This document covers
the legal terms and the engineering conventions that keep the engine
trustworthy.

## Licensing and contributions (dual-license terms)

`tpt-mathr` is dual-licensed under the **MIT licence** ([LICENSE-MIT](LICENSE-MIT))
and the **Apache License 2.0** ([LICENSE-APACHE](LICENSE-APACHE)), at the
recipient's option.

By contributing, you agree that your contribution is provided under **both**
licences, and you confirm you have the right to license it that way. Unless
you state otherwise, any contribution intentionally submitted for inclusion
is dual-licensed `MIT OR Apache-2.0` **without additional terms or
conditions**. Copyright in contributions is retained by their authors, with
a non-exclusive, irrevocable, worldwide, royalty-free licence to TPT
Solutions and all downstream users under the two licences above.

Every source file carries an SPDX header:

```rust
// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0
```

New files must include it. Generated files are exempt (document why).

## Ground rules

These are hard requirements, enforced by review and CI:

1. **Licensing policy.** Dependencies must be MIT-compatible: MIT,
   MIT/Apache-2.0 dual, BSD, ISC, Zlib, Unlicense/CC0. **No Apache-2.0-only
   dependencies in shipped crates.** No GPL/LGPL/MPL anywhere shipped.
   GMP/MPFR/FLINT/Python may appear in dev-dependencies and benchmarks only.
   Every new dependency needs an entry in [docs/dependency-audit.md](docs/dependency-audit.md)
   *before* it lands; `cargo deny check` must stay green.
2. **Safe Rust baseline.** `#![forbid(unsafe_code)]` everywhere except the
   isolated limb/SIMD kernel module and the FFI/WASM boundary. Any `unsafe`
   outside those requires a design-note in [docs/unsafe-policy.md](docs/unsafe-policy.md)
   first. Every `unsafe` block carries a `// SAFETY:` comment and Miri +
   differential coverage against a scalar reference path.
3. **No hidden global state.** No process-global mutable configuration.
   Limits, cancellation, and time sources are explicit parameters.
4. **Result semantics.** `ResourceExhausted` and `Inconclusive` are errors,
   never negative results and never verified results. If you add an
   error variant, update the status mapping table in `tpt-mathr-base`
   (the exhaustive match makes CI force this conversation).
5. **Fallible allocation.** Growth paths in large-data code go through
   `tpt-mathr-base::alloc_helpers`; a raw `Vec::push`-driven growth loop in
   big-operand code is a review blocker.
6. **No performance claims without data.** Do not merge an "optimisation"
   without a benchmark run recorded per [docs/benchmarks.md](docs/benchmarks.md).
   Do not publish comparative numbers without the full metadata (hardware,
   compiler, flags, operand size, op, threads, backend, version).
7. **Determinism.** Mathematical results must not depend on thread count or
   SIMD path. Concurrency may change *when*, never *what*.
8. **Public API hygiene.** The public API never exposes limb types, arenas,
   or parallelism. Users see `BigInt`, not `BigInt<u64>`.

## Development workflow

```bash
cargo build --workspace
cargo test  --workspace
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

- The pinned toolchain comes from `rust-toolchain.toml`; MSRV is 1.85 and
  CI verifies it. Do not use std/core features newer than 1.85.
- Tests live with the code (`#[cfg(test)]`), plus integration tests under
  `tests/`. Property-style tests use the dependency-free RNG helpers in
  `tests/support/` — adding `proptest` requires the dependency-audit process.
- Differential tests against Python `int` are behind the
  `differential-tests` feature and skip silently when the interpreter is
  unavailable.
- Commits: small, imperative subject lines, reference checklist items from
  `todo.md` (e.g. `arith: Karatsuba multiplication (Phase 2)`).

## Documentation expectations

Algorithms document: mathematical definition, algorithm, complexity,
numerical constraints, implementation, testing strategy, verification
strategy (spec §47). New public items need rustdoc with examples where the
usage is non-obvious.

## Questions

Open an issue describing the mathematical claim you want the engine to
carry — including what evidence would make you *trust* the answer.
