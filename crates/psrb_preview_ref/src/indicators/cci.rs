//! The Commodity Channel Index on the typical price (PWC 6.3), in thirds and
//! n-multiples: `T = H + L + C`, `S` the sum of the last `n` values of `T`,
//! `dev = n T_t - S`, `D = sum |n T_i - S|`, `CCI = 200 n dev / (3 D)`.
//! `above(m)` iff `D > 0 && 200_000 n dev > 3 m D`; `below(m)` iff
//! `D > 0 && 200_000 n dev < 3 m D` (m in thousandths of a point).

use std::collections::VecDeque;

use crate::market::Bar;

/// The last `period` typical-price sums.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cci {
    period: usize,
    ring: VecDeque<i128>,
    sum: i128,
}

/// One warm reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    /// `n`.
    pub n: i128,
    /// `n T_t - S`.
    pub deviation: i128,
    /// `D`.
    pub total_deviation: i128,
}

impl Reading {
    /// `D == 0`: every typical price in the window equal.
    #[must_use]
    pub const fn flat(&self) -> bool {
        self.total_deviation == 0
    }

    /// CCI strictly above `m` thousandths of a point.
    #[must_use]
    pub fn above(&self, milli: i32) -> bool {
        let d = self.total_deviation;
        d > 0 && 200_000 * self.n * self.deviation > 3 * i128::from(milli) * d
    }

    /// CCI strictly below `m` thousandths of a point.
    #[must_use]
    pub fn below(&self, milli: i32) -> bool {
        let d = self.total_deviation;
        d > 0 && 200_000 * self.n * self.deviation < 3 * i128::from(milli) * d
    }
}

impl Cci {
    /// A CCI of `period` candles (a zero period reads as 1).
    #[must_use]
    pub fn new(period: u16) -> Self {
        let period = usize::from(period.max(1));
        Self {
            period,
            ring: VecDeque::with_capacity(period + 1),
            sum: 0,
        }
    }

    /// Folds one evaluable candle.
    pub fn fold(&mut self, bar: &Bar) {
        let t = i128::from(bar.high) + i128::from(bar.low) + i128::from(bar.close);
        self.ring.push_back(t);
        let old = if self.ring.len() > self.period {
            self.ring.pop_front().unwrap_or(0)
        } else {
            0
        };
        self.sum += t - old;
    }

    /// `period` candles held.
    #[must_use]
    pub fn warm(&self) -> bool {
        self.ring.len() == self.period
    }

    /// The reading, once warm: one pass of `period` terms for `D`.
    #[must_use]
    pub fn reading(&self) -> Option<Reading> {
        if !self.warm() {
            return None;
        }
        let n = i128::try_from(self.period).ok()?;
        let last = self.ring.back().copied()?;
        let total_deviation = self.ring.iter().map(|t| (n * t - self.sum).abs()).sum();
        Some(Reading {
            n,
            deviation: n * last - self.sum,
            total_deviation,
        })
    }
}
