# One-shot generator for the Phase 0 documentation skeleton set.
import os

# spec section 47: every doc follows a fixed structure so algorithms can be
# audited uniformly: mathematical definition, algorithm, complexity,
# constraints, implementation, testing, verification.

FILES = {
    "arithmetic.md": ("Arithmetic", "The shared arithmetic layers: limb model, strategies, limits."),
    "bigint.md": ("Big integers (BigUint / BigInt)", "Arbitrary-precision integers."),
    "bigfloat.md": ("BigFloat", "Arbitrary-precision floating-point arithmetic."),
    "rational.md": ("BigRat", "Exact rational arithmetic."),
    "polynomials.md": ("Polynomials", "Polynomial arithmetic over exact coefficients."),
    "simd.md": ("SIMD", "Architecture-specific acceleration with runtime dispatch."),
    "verification.md": ("Verification", "Certificates, checkers, assurance profiles, proof export."),
    "search.md": ("Search", "Reproducible, cancellable, certificate-producing search."),
    "ffi.md": ("FFI", "C ABI, Python, and language interoperability."),
    "wasm.md": ("WebAssembly", "Browser and WASM-runtime deployment."),
    "benchmarks.md": ("Benchmarks", "How performance is measured, recorded, and claimed."),
    "reproducibility.md": ("Reproducibility", "What must be recorded for a result to be reproduced."),
}

TEMPLATE = '''# {title}

> Status: skeleton (Phase 0). Content lands with the corresponding roadmap
> phase; the section structure below is fixed by spec section 47 so every
> algorithm is documented uniformly.

{blurb}

## Scope

{scope}

## Mathematical definition

TODO(phase-{phase}).

## Algorithm

TODO(phase-{phase}).

## Complexity

TODO(phase-{phase}). No complexity claims are made before measurement.

## Numerical constraints

TODO(phase-{phase}); resource limits that apply: `ResourceLimits` fields
documented in `tpt-mathr-base`.

## Implementation

TODO(phase-{phase}); unsafe policy: [unsafe-policy.md](unsafe-policy.md);
invariants: [invariants.md](invariants.md).

## Testing strategy

TODO(phase-{phase}); stack per spec section 30: unit, property,
differential, metamorphic, fuzz, Miri.

## Verification strategy

TODO(phase-{phase}); what evidence would make a result from this layer
independently checkable.
'''

SCOPES = {
    "arithmetic.md": """The limb representation (private, platform-conditional: u64 on 64-bit,
u32 on 32-bit/wasm32), the `MulStrategy` abstraction with benchmark-derived
thresholds, fallible allocation discipline, scratch-buffer reuse, and the
determinism contract that binds every layer.""",
    "bigint.md": """`BigUint` and `BigInt`: canonical representations, addition, subtraction,
comparison, multiplication strategies (schoolbook, Karatsuba, Toom-Cook,
NTT/FFT), division/remainder (Knuth D), gcd/extended gcd/lcm, pow, isqrt,
parsing and formatting for radix 2-36, primitive conversions.""",
    "bigfloat.md": """Sign / significand / exponent / precision representation; rounding modes
(Nearest, TowardZero, TowardPositive, TowardNegative); core arithmetic,
sqrt, powers, exp, log, trigonometric functions as maturity permits;
deterministic results independent of thread count and SIMD path.""",
    "rational.md": """`BigRat` with BigInt numerator and BigUint denominator, always reduced,
denominator positive; arithmetic, comparison, floor/ceil/round, numerator
and denominator accessors; the `tpt-math-exact` backend integration.""",
    "polynomials.md": """`Polynomial<T>`, `SparsePolynomial<T>`, `FormalPowerSeries<T>`;
add/sub/mul/div/gcd/evaluate/differentiate/integrate/compose/interpolate;
adaptive multiplication with benchmark-justified thresholds; factorisation
with scoped, documented limits; degree limits via `ResourceLimits`.""",
    "simd.md": """Dispatch topology (scalar / AVX2 / AVX-512 / NEON / WASM SIMD), capability
detection at runtime, the isolated unsafe kernel module with scalar
reference paths, and the bit-identical-output requirement across paths.""",
    "verification.md": """The certificate model and serialisation; local checkers as the trusted
kernel; independent verification of FFT/NTT product candidates;
verification-obligation generation; assurance profiles (Fast, Checked,
Certified, Formal, KernelVerified); Lean export; the trusted computing base
definition.""",
    "search.md": """`SearchProblem`/`SearchEngine`; the result taxonomy (Found,
NotFoundWithinBounds, Exhausted, Inconclusive, Verified, Counterexample);
deterministic work-unit identity; resumable jobs and persisted metadata;
backends (native threads / distributed / GPU / WASM workers); cancellation
and limits; worker failure isolation.""",
    "ffi.md": """The C ABI contract: opaque handles, `TptStatus f(..., T** out)`, explicit
free functions, no panics across the boundary, ABI versioning, error-detail
API; PyO3 bindings; header generation (cbindgen as a build-time tool).""",
    "wasm.md": """Single-threaded baseline; wasm-bindgen bindings for BigInt, BigRat,
selected BigFloat, polynomials; capability detection (SIMD, no filesystem
or timers assumed); the optional threaded target (atomics + shared memory +
wasm-bindgen-rayon) and its cross-origin-isolation deployment requirements.""",
    "benchmarks.md": """The benchmark contract: recorded hardware, compiler, flags, operand size,
operation, threads, backend, and version for every published number;
operand matrix 32 B to 100 MiB; parallel-scaling runs at 1/2/4/8+ threads;
comparisons vs num-bigint, GMP, Python int only with full metadata; no
claims without data.""",
    "reproducibility.md": """The record required to reproduce any engine result: inputs, version,
precision, rounding mode, algorithm/backend configuration, CPU feature
selection, seed, and work-unit partition; how the engine pins each of
these and how search jobs persist them.""",
}

for name, (title, blurb) in FILES.items():
    phase = {
        "bigint.md": 2, "polynomials.md": 5, "bigfloat.md": 6, "rational.md": 4,
        "verification.md": 8, "search.md": 9, "ffi.md": 11, "wasm.md": 11,
        "benchmarks.md": 3, "reproducibility.md": 3, "simd.md": 3,
        "arithmetic.md": 2,
    }[name]
    body = TEMPLATE.format(title=title, blurb=blurb, scope=SCOPES[name], phase=phase)
    with open(os.path.join("docs", name), "w", encoding="utf-8", newline="\n") as f:
        f.write(body)
    print("wrote", name)
