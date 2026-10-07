# One-shot reconciliation: spec.txt (v1.2.0 base text) + spec1.2.txt deltas -> docs/spec.md

HEADER = """# Software Design Specification: tpt-mathr

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

"""

AMENDMENTS = """

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
"""


def main():
    with open("spec.txt", encoding="utf-8") as f:
        base = f.read().rstrip() + "\n"
    # Drop the base text's own H1 title block duplication: keep body from its
    # first line (the title is re-stated in the header above).
    body = base.split("---", 1)[1] if base.startswith("# Software Design Specification") else base
    doc = HEADER + body.rstrip() + "\n" + AMENDMENTS
    with open("docs/spec.md", "w", encoding="utf-8", newline="\n") as f:
        f.write(doc)
    print("wrote docs/spec.md:", len(doc), "chars")


if __name__ == "__main__":
    main()
