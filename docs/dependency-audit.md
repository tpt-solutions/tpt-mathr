# Dependency audit

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
