# unsafe policy

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
