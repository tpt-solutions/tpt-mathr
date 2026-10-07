// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic, dependency-free random generation for property and
//! metamorphic tests (behind the `testing` feature).
//!
//! The engine's property tests must be **reproducible**: a failure with seed
//! S must re-fail with seed S on any machine. External property-testing
//! frameworks are deferred (docs/dependency-audit.md), so this module
//! provides the small fixed core: a SplitMix64 generator with helpers for
//! the shapes arithmetic tests need.
//!
//! Not for cryptographic or benchmark use — test scaffolding only.

/// Deterministic `SplitMix64` generator (fixed, portable sequence).
#[derive(Debug, Clone)]
pub struct TestRng {
    state: u64,
}

impl TestRng {
    /// Create a generator from a seed. The same seed yields the same
    /// sequence on every platform.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// `SplitMix64` step.
    #[must_use]
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform value in `0..n`. `n <= 1` yields 0 (no panic).
    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the truncating half of a u64*u64 product is exactly the low 64 bits by definition"
    )]
    pub fn below(&mut self, n: u64) -> u64 {
        if n <= 1 {
            return 0;
        }
        // Lemire's bounded generation with rejection sampling:
        // the low-wrapping region `lo < 2^64 mod n` is biased, so redraw.
        loop {
            let x = self.next_u64();
            let m = (u128::from(x)) * (u128::from(n));
            let lo = m as u64;
            if lo < n {
                let threshold = n.wrapping_neg() % n; // 2^64 mod n
                if lo < threshold {
                    continue;
                }
            }
            #[allow(
                clippy::cast_possible_truncation,
                reason = "high part of a u64*u64 product, fits u64 by construction"
            )]
            return (m >> 64) as u64;
        }
    }

    /// A random usize in `0..n` (bounded by u64 range).
    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        reason = "test helper; result is below n which is a usize by construction"
    )]
    pub fn below_usize(&mut self, n: usize) -> usize {
        self.below(n as u64) as usize
    }

    /// `true` with probability `numerator/denominator`.
    #[must_use]
    pub fn chance(&mut self, numerator: u64, denominator: u64) -> bool {
        self.below(denominator) < numerator
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_is_deterministic_across_platforms() {
        let mut a = TestRng::new(0xDEAD_BEEF);
        let mut b = TestRng::new(0xDEAD_BEEF);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        // And a pinned value so an accidental algorithm change is caught:
        let mut r = TestRng::new(1);
        assert_eq!(r.next_u64(), 0x910A_2DEC_8902_5CC1);
    }

    #[test]
    fn below_respects_bounds() {
        let mut r = TestRng::new(42);
        for _ in 0..1000 {
            assert!(r.below(7) < 7);
        }
        assert_eq!(r.below(0), 0);
        assert_eq!(r.below(1), 0);
    }

    #[test]
    fn chance_covers_the_range() {
        let mut r = TestRng::new(7);
        let mut trues = 0;
        for _ in 0..1000 {
            if r.chance(1, 2) {
                trues += 1;
            }
        }
        assert!(
            (400..600).contains(&trues),
            "coin should be near fair, got {trues}/1000"
        );
    }
}
