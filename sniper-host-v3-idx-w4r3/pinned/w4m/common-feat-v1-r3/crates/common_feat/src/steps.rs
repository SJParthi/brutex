//! Deterministic step counters (SNIPER-ADANIENT-PLAN 8.4, SC-FOLD / SC-FEAT; REQ72-PLAN A.6).
//!
//! Every fold counts the elementary steps it took (one ring update, one deque pop, one comparison of a latch ...).
//! The counts depend only on the bounded state, never on how much data came before, so a host that runs the same
//! candles at 1x, 10x and 100x must see the SAME per-event distribution (p50, p99, max). That is the HARD gate; wall
//! times print beside it and are never the gate.

/// The step count of one event, by part.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Steps {
    /// Folding one bar into one timeframe's candle (SC-FOLD, bound 3).
    pub fold: u32,
    /// Momentum readings: RSI, CCI, %R.
    pub momentum: u32,
    /// Trend readings: EMA, ATR, Supertrend.
    pub trend: u32,
    /// Volatility: Bollinger.
    pub vol: u32,
    /// Levels, anatomy, relative volume, calendar, the higher-timeframe read.
    pub other: u32,
}

impl Steps {
    /// The total of the parts.
    #[must_use]
    pub const fn total(&self) -> u32 {
        self.fold + self.momentum + self.trend + self.vol + self.other
    }
}

/// A histogram of per-event step counts, exact (one bucket per count up to [`StepHist::CAP`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepHist {
    buckets: Vec<u64>,
    over: u64,
    n: u64,
    max: u32,
}

impl Default for StepHist {
    fn default() -> Self {
        Self::new()
    }
}

impl StepHist {
    /// Counts above this land in one overflow bucket (and set `max`).
    pub const CAP: usize = 4096;

    /// An empty histogram.
    #[must_use]
    pub fn new() -> Self {
        Self { buckets: vec![0; Self::CAP + 1], over: 0, n: 0, max: 0 }
    }

    /// Adds one event's count.
    pub fn add(&mut self, steps: u32) {
        self.n += 1;
        self.max = self.max.max(steps);
        match usize::try_from(steps).ok().and_then(|i| self.buckets.get_mut(i)) {
            Some(b) => *b += 1,
            None => self.over += 1,
        }
    }

    /// Events seen.
    #[must_use]
    pub const fn count(&self) -> u64 {
        self.n
    }

    /// The largest count seen.
    #[must_use]
    pub const fn max(&self) -> u32 {
        self.max
    }

    /// The nearest-rank percentile (`permille` in 1..=1000) of the counts.
    #[must_use]
    pub fn permille(&self, permille: u64) -> u32 {
        if self.n == 0 {
            return 0;
        }
        let rank = (self.n * permille).div_ceil(1000).max(1);
        let mut seen = 0u64;
        for (i, b) in self.buckets.iter().enumerate() {
            seen += *b;
            if seen >= rank {
                return u32::try_from(i).unwrap_or(u32::MAX);
            }
        }
        self.max
    }

    /// (p50, p99, max).
    #[must_use]
    pub fn summary(&self) -> (u32, u32, u32) {
        (self.permille(500), self.permille(990), self.max)
    }
}
