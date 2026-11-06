//! Williams %R (PWC 5.3): `%R = -100 (HH - C) / (HH - LL)` over the last `n` candles,
//! the current one included. With `U = HH - C` and `R = HH - LL`:
//! `above(l)` iff `R > 0 && 10^6 U < (-l) R`; `below(l)` iff `R > 0 && 10^6 U > (-l) R`;
//! a flat window (`R == 0`) decides nothing.

use std::collections::VecDeque;

use crate::market::Bar;

/// The last `period` highs and lows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WilliamsR {
    period: usize,
    ring: VecDeque<(i64, i64)>,
    close: i64,
}

/// One warm reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    /// The highest high of the window.
    pub highest: i64,
    /// The lowest low of the window.
    pub lowest: i64,
    /// The current close.
    pub close: i64,
}

impl Reading {
    /// `HH == LL`: no %R value.
    #[must_use]
    pub const fn flat(&self) -> bool {
        self.highest == self.lowest
    }

    fn parts(&self) -> (i128, i128) {
        let u = i128::from(self.highest) - i128::from(self.close);
        let r = i128::from(self.highest) - i128::from(self.lowest);
        (u, r)
    }

    /// %R strictly above `level` ppm (`-1_000_000..=0`).
    #[must_use]
    pub fn above(&self, level_ppm: i32) -> bool {
        let (u, r) = self.parts();
        r > 0 && 1_000_000 * u < -i128::from(level_ppm) * r
    }

    /// %R strictly below `level` ppm.
    #[must_use]
    pub fn below(&self, level_ppm: i32) -> bool {
        let (u, r) = self.parts();
        r > 0 && 1_000_000 * u > -i128::from(level_ppm) * r
    }
}

impl WilliamsR {
    /// A %R of `period` candles (a zero period reads as 1).
    #[must_use]
    pub fn new(period: u16) -> Self {
        let period = usize::from(period.max(1));
        Self {
            period,
            ring: VecDeque::with_capacity(period + 1),
            close: 0,
        }
    }

    /// Folds one evaluable candle.
    pub fn fold(&mut self, bar: &Bar) {
        self.ring.push_back((bar.high, bar.low));
        if self.ring.len() > self.period {
            self.ring.pop_front();
        }
        self.close = bar.close;
    }

    /// `period` candles folded.
    #[must_use]
    pub fn warm(&self) -> bool {
        self.ring.len() == self.period
    }

    /// The reading, once warm: one pass of `period` entries.
    #[must_use]
    pub fn reading(&self) -> Option<Reading> {
        if !self.warm() {
            return None;
        }
        let highest = self.ring.iter().map(|(h, _)| *h).max()?;
        let lowest = self.ring.iter().map(|(_, l)| *l).min()?;
        Some(Reading {
            highest,
            lowest,
            close: self.close,
        })
    }
}
