//! The exponential moving average of the close (PWC 4.1, D-1542): one running sum while
//! the first `period` closes fold, the value their running mean (exactly the simple mean
//! of the first `period` at warm), then `ema += ((close x SCALE - ema) x 2).div_euclid(n + 1)`;
//! held scaled by [`SCALE`].

use super::SCALE;
use crate::market::Bar;

/// The EMA state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ema {
    period: u16,
    count: u32,
    sum: i128,
    value: i128,
}

impl Ema {
    /// An EMA of `period` (a zero period reads as 1).
    #[must_use]
    pub const fn new(period: u16) -> Self {
        let period = if period == 0 { 1 } else { period };
        Self { period, count: 0, sum: 0, value: 0 }
    }

    /// Folds one evaluable candle's close.
    pub fn fold(&mut self, bar: &Bar) {
        let price = i128::from(bar.close) * SCALE;
        self.count = self.count.saturating_add(1);
        if self.count <= u32::from(self.period) {
            self.sum += price;
            self.value = self.sum.div_euclid(i128::from(self.count));
        } else {
            self.value += ((price - self.value) * 2).div_euclid(i128::from(self.period) + 1);
        }
    }

    /// `period` closes folded.
    #[must_use]
    pub fn warm(&self) -> bool {
        self.count >= u32::from(self.period)
    }

    /// The scaled average (read on the scaled state by C8).
    #[must_use]
    pub const fn scaled(&self) -> i128 {
        self.value
    }
}
