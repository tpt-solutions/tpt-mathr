# Interface review: `tpt-math-exact` backend abstraction (Phase 4)

**Reviewed:** `tpt-math` workspace, crate `tpt-math-exact` v0.1.0 (local
checkout `C:\Programming\2 WIP\tpt-math`, 248-line `lib.rs`, reviewed
2026-10-06). This document is the Phase 4 "interface review" deliverable;
the backend feature-flag implementation itself lives in the `tpt-math`
repository and is **deferred** to a coordinated change there.

## Current shape

`tpt-math-exact` is `no_std + alloc` and thin-wraps the `num` crates:

```rust
pub use num_bigint::{BigInt, BigUint};
pub use num_rational::BigRational;
pub type Rational = BigRational;   // concrete alias — there is no backend trait
pub struct Interval<T> { lo: T, hi: T }  // requires T: Clone + PartialOrd
```

* `Rational` is an **alias, not an abstraction**: every consumer of
  `tpt-math-exact` gets `num_rational::Ratio<num_bigint::BigInt>`
  concretely, with `num_traits` trait bounds providing arithmetic.
* Constructors used downstream: `Rational::new(numerator, denominator)`
  (auto-reducing, **panics on zero denominator**), `One`, `FromPrimitive`
  (via `tpt-math-numeric`), std `ops` traits, `PartialOrd`.
* Features: `std` (default), `alloc`.

## Findings

1. **No seam exists today.** A `tpt-mathr` backend cannot be added
   "behind a feature flag" without first introducing a seam — either a
   trait (e.g. `ExactBackend { type Rational; ... }`) or a type-level
   switch (feature-gated alias). The feature-alias route
   (`#[cfg(feature = "tpt-mathr")] pub type Rational = tpt_mathr_arith::BigRat;`)
   is the smaller change and preserves the alias-based API.

2. **Fallibility mismatch (the substantive finding).** `tpt-mathr::BigRat`
   operations return `Result<_, Error>` (fallible allocation, v1.2 delta
   A1); `num_rational::Ratio` operations are infallible and **abort** on
   allocation failure. A drop-in alias therefore needs either:
   * an infallible adapter in `tpt-math-exact` that `expect()`s engine
     errors — acceptable for parity with `num`'s abort semantics, and
     honest (the failure mode does not get *worse* than the current
     backend), or
   * `tpt-math-exact` growing fallible APIs — an API break for its
     consumers.
   Recommendation: adapter with `expect`, documented, plus a
   `tpt-mathr-checked` escape hatch later if callers need graceful
   degradation. The engine's budget/cancellation-aware entry points remain
   available to callers who opt in directly.

3. **Constructor semantics.** `Ratio::new` panics on zero denominator and
   auto-reduces; `BigRat::new` returns `Err(DivisionByZero)` and
   auto-reduces. The adapter maps the error to a panic with a message
   matching `num`'s behaviour.

4. **Trait surface.** `Interval<T>` and downstream code need: `Clone`,
   `Debug`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`/`Ord`, `Neg`,
   `Add/Sub/Mul/Div` (by ref and by value), `Zero`, `One`, `Inv`,
   `FromPrimitive`, `ToPrimitive`-style narrowing. `BigRat` currently
   provides all except the `num_traits` traits and std `ops` impls; the
   backend flag needs a small impl block in `tpt-math-exact` (or
   `num-traits`-compatible impls behind the feature). Cross-ordering note:
   `BigRat` ordering is fallible (`cmp_rat`, cross-multiplication
   allocates); an `Ord` impl for the adapter must `expect` — consistent
   with `num_rational`, which allocates inside `cmp` invisibly.

5. **What is *not* affected.** `Interval` logic, `tpt-math-numeric`
   integration, and the public `Rational` alias spelling all stay intact —
   consumers compile unchanged under either backend.

## Decision recorded

Adopt the **feature-gated alias + infallible adapter** route in
`tpt-math-exact` (`feature = "backend-tpt-mathr"`, default off), to be
implemented in the `tpt-math` repository against a tagged
`tpt-mathr-arith` release. Prerequisite tracked in this repo: expose the
`num`-compat surface (ops traits + `Zero`/`One`/`Inv` equivalents or
documented adapters) — tracked as a Phase 11-adjacent task so the core
stays dependency-free (the impls would live in `tpt-math-exact`, which may
depend on `num-traits` already).

## Deferred items (updated in todo.md)

* "Implement `tpt-mathr` backend behind a feature flag in `tpt-math-exact`"
  — belongs to the `tpt-math` repo; blocked on this review's decision,
  which is now recorded.
* "Milestone: `tpt-math-exact` can optionally use the tpt-mathr backend" —
  completes with the above.
