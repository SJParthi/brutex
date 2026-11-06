//! Wilder's RSI (PARTS 3.3): the first candle records its close; each later candle's
//! change gives a scaled gain and loss; the first `n` changes are summed and seeded as
//! their floored mean, then `G += (gain - G).div_euclid(n)`, the same for `L`.
//! `above(level)`: `G == L == 0` reads as RSI 100, else `10^6 G > level (G + L)`;
//! `below(level)`: `10^6 G < level (G + L)`; `faithful`: `G + L >= 1000 x period`.

use super::SCALE;
use crate::market::Bar;

/// RSI 100 in parts per million of the 0..100 scale.
pub const MAX_LEVEL_PPM: u32 = 1_000_000;
/// The faithful guard's factor.
pub const FAITHFUL_FACTOR: i128 = 1_000;

/// The RSI state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rsi {
    period: u16,
    previous: Option<i64>,
    changes: u32,
    gain_sum: i128,
    loss_sum: i128,
    gain: i128,
    loss: i128,
}

/// One warm reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    /// The period.
    pub period: u16,
    /// The scaled average gain.
    pub gain: i128,
    /// The scaled average loss.
    pub loss: i128,
}

impl Reading {
    /// Both averages are 0.
    #[must_use]
    pub const fn flat(&self) -> bool {
        self.gain == 0 && self.loss == 0
    }

    /// `G + L >= 1000 x period`: within 0.1 RSI point of exact Wilder.
    #[must_use]
    pub fn faithful(&self) -> bool {
        self.gain + self.loss >= FAITHFUL_FACTOR * i128::from(self.period)
    }

    /// `floor(10^6 G / (G + L))`; 1,000,000 when `L == 0`.
    #[must_use]
    pub fn ppm(&self) -> i128 {
        if self.loss == 0 {
            return i128::from(MAX_LEVEL_PPM);
        }
        (i128::from(MAX_LEVEL_PPM) * self.gain).div_euclid(self.gain + self.loss)
    }

    /// RSI strictly above `level` ppm.
    #[must_use]
    pub fn above(&self, level: u32) -> bool {
        if self.flat() {
            return level < MAX_LEVEL_PPM;
        }
        i128::from(MAX_LEVEL_PPM) * self.gain > i128::from(level) * (self.gain + self.loss)
    }

    /// RSI strictly below `level` ppm (false when flat).
    #[must_use]
    pub fn below(&self, level: u32) -> bool {
        i128::from(MAX_LEVEL_PPM) * self.gain < i128::from(level) * (self.gain + self.loss)
    }
}

impl Rsi {
    /// An RSI of `period` (a zero period reads as 1).
    #[must_use]
    pub const fn new(period: u16) -> Self {
        let period = if period == 0 { 1 } else { period };
        Self {
            period,
            previous: None,
            changes: 0,
            gain_sum: 0,
            loss_sum: 0,
            gain: 0,
            loss: 0,
        }
    }

    /// Folds one evaluable candle.
    pub fn fold(&mut self, bar: &Bar) {
        let Some(previous) = self.previous.replace(bar.close) else {
            return;
        };
        let change = i128::from(bar.close) - i128::from(previous);
        let gain = change.max(0) * SCALE;
        let loss = (-change).max(0) * SCALE;
        self.changes = self.changes.saturating_add(1);
        let n = i128::from(self.period);
        if self.changes <= u32::from(self.period) {
            self.gain_sum += gain;
            self.loss_sum += loss;
        } else {
            self.gain += (gain - self.gain).div_euclid(n);
            self.loss += (loss - self.loss).div_euclid(n);
        }
        if self.changes == u32::from(self.period) {
            self.gain = self.gain_sum.div_euclid(n);
            self.loss = self.loss_sum.div_euclid(n);
        }
    }

    /// At least `period` changes folded.
    #[must_use]
    pub fn warm(&self) -> bool {
        self.changes >= u32::from(self.period)
    }

    /// The reading, once warm.
    #[must_use]
    pub fn reading(&self) -> Option<Reading> {
        self.warm().then_some(Reading {
            period: self.period,
            gain: self.gain,
            loss: self.loss,
        })
    }
}
