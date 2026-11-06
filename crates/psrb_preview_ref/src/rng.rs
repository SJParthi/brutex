//! A seeded, platform-independent generator for the synthetic market.

/// `SplitMix64` (Steele, Lea and Flood 2014): one `u64` of state, every output a
/// pure function of the seed and the call count, identical on every platform.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// A generator whose whole future is fixed by `seed`.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The next 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..bound`, without modulo bias; `0` when `bound` is `0`.
    pub fn below(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            return 0;
        }
        // 2^64 mod bound: the draws under it are the uneven tail and are redrawn.
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let r = self.next_u64();
            if r >= threshold {
                return r % bound;
            }
        }
    }

    /// Uniform in `lo..=hi` (the two swapped when `lo > hi`).
    pub fn between(&mut self, lo: i64, hi: i64) -> i64 {
        let (a, b) = if lo <= hi { (lo, hi) } else { (hi, lo) };
        let width = i128::from(b) - i128::from(a) + 1;
        let span = u64::try_from(width).unwrap_or(u64::MAX);
        let off = i128::from(self.below(span));
        i64::try_from(i128::from(a) + off).unwrap_or(a)
    }

    /// `true` with probability `num / den`.
    pub fn chance(&mut self, num: u64, den: u64) -> bool {
        self.below(den) < num
    }
}

#[cfg(test)]
#[path = "rng_tests.rs"]
mod tests;
