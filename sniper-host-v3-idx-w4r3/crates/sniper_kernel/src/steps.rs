//! Deterministic per-event step counters with their EXACT p50 / p99 / max (SNIPER-ADANIENT-PLAN 8.4: "the per-event step counts
//! are identical at 1x, 10x and 100x data, and asserted"; wall times print beside, never as the gate).

/// An exact histogram of small step counts (every value below `CAP` counted exactly; larger ones in the last cell, the max kept).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hist {
    counts: Vec<u64>,
    n: u64,
    max: u64,
}

impl Default for Hist {
    fn default() -> Self {
        Self::new()
    }
}

impl Hist {
    /// Exact below this many steps.
    pub const CAP: usize = 4096;

    /// Empty.
    #[must_use]
    pub fn new() -> Self {
        Self { counts: vec![0; Self::CAP + 1], n: 0, max: 0 }
    }

    /// Counts one event of `steps` steps.
    pub fn add(&mut self, steps: u64) {
        let i = usize::try_from(steps).unwrap_or(Self::CAP).min(Self::CAP);
        if let Some(c) = self.counts.get_mut(i) {
            *c += 1;
        }
        self.n += 1;
        self.max = self.max.max(steps);
    }

    /// Counts `n` events of `steps` steps each: exactly `n` calls of [`Hist::add`] (speed build: the key-read tally).
    pub fn add_n(&mut self, steps: u64, n: u64) {
        if n == 0 {
            return;
        }
        let i = usize::try_from(steps).unwrap_or(Self::CAP).min(Self::CAP);
        if let Some(c) = self.counts.get_mut(i) {
            *c += n;
        }
        self.n += n;
        self.max = self.max.max(steps);
    }

    /// Joins another histogram (associative: shards merge in any order).
    pub fn merge(&mut self, o: &Self) {
        for (a, b) in self.counts.iter_mut().zip(&o.counts) {
            *a += b;
        }
        self.n += o.n;
        self.max = self.max.max(o.max);
    }

    /// Events counted.
    #[must_use]
    pub const fn count(&self) -> u64 {
        self.n
    }

    /// The largest event.
    #[must_use]
    pub const fn max(&self) -> u64 {
        self.max
    }

    /// The smallest step count s with at least `permille` / 1000 of the events at or below it (0 when empty).
    #[must_use]
    pub fn permille(&self, permille: u64) -> u64 {
        if self.n == 0 {
            return 0;
        }
        let need = (self.n * permille).div_ceil(1000).max(1);
        let mut run = 0_u64;
        for (i, c) in self.counts.iter().enumerate() {
            run += c;
            if run >= need {
                return if i == Self::CAP { self.max } else { i as u64 };
            }
        }
        self.max
    }

    /// (p50, p99, max).
    #[must_use]
    pub fn summary(&self) -> (u64, u64, u64) {
        (self.permille(500), self.permille(990), self.max)
    }
}
