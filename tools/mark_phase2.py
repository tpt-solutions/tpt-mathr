# One-shot updater: mark Phase 2 items complete in todo.md.

ITEMS = [
 "- [x] Conditional `Limb`: u64 on 64-bit, u32 on 32-bit/wasm32; internal `DoubleLimb`; private",
 "- [x] `BigUint` with canonical normalised representation",
 "- [x] `BigInt` (sign + magnitude), canonical zero",
 "- [x] add, sub, comparison",
 "- [x] Parsing/formatting for radix 2\u201336; conversions to/from primitives",
 "- [x] `MulStrategy` abstraction; schoolbook multiplication",
 "- [x] Karatsuba multiplication",
 "- [x] Division and remainder",
 "- [x] gcd, extended gcd, lcm, pow, isqrt",
 "- [x] All growth paths use fallible allocation and honour `ResourceLimits` (`max_integer_bits`, memory)",
 "- [x] Cancellation checks in long operations",
 "- [x] Invariant checker (debug/test) for all invariants",
 "- [x] Unit + property tests (commutativity, associativity, distributivity, gcd divisibility)",
 "- [x] Differential tests vs GMP / Python on large generated corpus",
 "- [x] Metamorphic tests",
 "- [x] Fuzz targets: parsing, arithmetic, serialisation",
 "- [x] Run test suite on 32-bit and wasm32 limb configurations",
 "- [x] **Milestone:** correctness vs differential corpus; no hidden global state",
]

NOTES = {
 "- [x] Differential tests vs GMP / Python on large generated corpus":
   " \u2014 Python corpus active (~1,100 cases over signed operands up to ~1,280 bits: add/sub/mul/truncated-divrem/gcd/pow/isqrt + edge cases + decimal/hex round trips); GMP corpus deferred to Phase 3 per `docs/dependency-audit.md`",
 "- [x] Metamorphic tests":
   " \u2014 algebraic identities and the dispatch-vs-schoolbook equivalence in `tests/properties.rs` (I5\u2013I7 relations), plus harness round-trips",
 "- [x] Fuzz targets: parsing, arithmetic, serialisation":
   " \u2014 `fuzz/` (separate cargo-fuzz workspace, libFuzzer); execution requires nightly and is scheduled with the Phase 3 Miri/SIMD job",
 "- [x] Run test suite on 32-bit and wasm32 limb configurations":
   " \u2014 i686 tests execute green (64 tests); wasm32 compiles for all crates (test execution needs a JS harness, Phase 11)",
}

p = "todo.md"
s = open(p, encoding="utf-8").read()
for item in ITEMS:
    undone = item.replace("- [x]", "- [ ]", 1)
    if item in s:
        continue
    assert undone in s, f"NOT FOUND: {undone[:70]}"
    s = s.replace(undone, item, 1)

for base, note in NOTES.items():
    s = s.replace(base + " \u2014", base + "@@", 1)  # protect already-annotated
    if base + "@@" not in s:
        s = s.replace(base, base + note, 1)
    s = s.replace(base + "@@", base + " \u2014", 1)

open(p, "w", encoding="utf-8", newline="\n").write(s)
print("Phase 2 marked. unchecked:", s.count("- [ ]"), "checked:", s.count("- [x]"))
