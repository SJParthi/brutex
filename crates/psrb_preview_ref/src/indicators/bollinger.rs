//! Bollinger bands on the close, population sigma, the window including the current
//! candle (PARTS 4.3). With `S` the sum, `Q` the sum of squares, `V = nQ - S^2` and
//! `D = n x close - S`: above the upper band iff `D > 0 && (1000 D)^2 > z^2 V`, below
//! the lower band iff `D < 0 && (1000 D)^2 > z^2 V`, exact, no square root.

use std::collections::VecDeque;

use crate::market::Bar;

/// The largest close a window accepts: 2^32 paisa.
pub const MAX_CLOSE: i64 = 1 << 32;

/// The last `period` closes and their two running sums.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bollinger {
    period: usize,
    ring: VecDeque<i64>,
    sum: i128,
    sum_sq: i128,
}

/// One warm window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    /// Closes in the window.
    pub n: i128,
    /// Their sum.
    pub sum: i128,
    /// Their sum of squares.
    pub sum_sq: i128,
    /// The current candle's close.
    pub last: i64,
}

impl Window {
    /// `V = n Q - S^2`, which is `n^2 sigma^2` (population).
    #[must_use]
    pub const fn variance_numerator(&self) -> i128 {
        self.n * self.sum_sq - self.sum * self.sum
    }

    /// `D = n x close - S`.
    #[must_use]
    pub fn deviation(&self) -> i128 {
        self.n * i128::from(self.last) - self.sum
    }

    /// The close strictly above `basis + z sigma`, z in thousandths.
    #[must_use]
    pub fn above_upper(&self, z_milli: u32) -> bool {
        let d = self.deviation();
        let z = i128::from(z_milli);
        d > 0 && (d * 1_000) * (d * 1_000) > z * z * self.variance_numerator()
    }

    /// The close strictly below `basis - z sigma`, z in thousandths.
    #[must_use]
    pub fn below_lower(&self, z_milli: u32) -> bool {
        let d = self.deviation();
        let z = i128::from(z_milli);
        d < 0 && (d * 1_000) * (d * 1_000) > z * z * self.variance_numerator()
    }
}

impl Bollinger {
    /// A window of `period` closes (a zero period reads as 1).
    #[must_use]
    pub fn new(period: u16) -> Self {
        let period = usize::from(period.max(1));
        Self {
            period,
            ring: VecDeque::with_capacity(period + 1),
            sum: 0,
            sum_sq: 0,
        }
    }

    /// Folds one close (`1..=MAX_CLOSE`, checked by the caller).
    pub fn fold(&mut self, bar: &Bar) {
        let close = i128::from(bar.close);
        self.ring.push_back(bar.close);
        let old = if self.ring.len() > self.period {
            i128::from(self.ring.pop_front().unwrap_or(0))
        } else {
            0
        };
        self.sum += close - old;
        self.sum_sq += close * close - old * old;
    }

    /// `period` closes held.
    #[must_use]
    pub fn warm(&self) -> bool {
        self.ring.len() == self.period
    }

    /// The window, once warm.
    #[must_use]
    pub fn window(&self) -> Option<Window> {
        if !self.warm() {
            return None;
        }
        let n = i128::try_from(self.period).ok()?;
        let last = self.ring.back().copied()?;
        Some(Window {
            n,
            sum: self.sum,
            sum_sq: self.sum_sq,
            last,
        })
    }
}
