//! The average true range: the first candle's own high-low span, then Wilder's true
//! range; seeded by the simple mean of the first `period` ranges (D-1542), then
//! `atr += (tr x SCALE - atr).div_euclid(n)`; held scaled, read in whole paisa.

use super::SCALE;
use crate::market::Bar;

/// The ATR state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Atr {
    period: u16,
    count: u32,
    sum: i128,
    value: i128,
    previous_close: Option<i64>,
}

impl Atr {
    /// An ATR of `period` candles (a zero period reads as 1).
    #[must_use]
    pub const fn new(period: u16) -> Self {
        let period = if period == 0 { 1 } else { period };
        Self {
            period,
            count: 0,
            sum: 0,
            value: 0,
            previous_close: None,
        }
    }

    /// The true range of `bar` after the previous close.
    #[must_use]
    pub fn true_range(bar: &Bar, previous_close: Option<i64>) -> i128 {
        let span = i128::from(bar.high) - i128::from(bar.low);
        previous_close.map_or(span, |pc| {
            let up = (i128::from(bar.high) - i128::from(pc)).abs();
            let down = (i128::from(bar.low) - i128::from(pc)).abs();
            span.max(up).max(down)
        })
    }

    /// Folds one evaluable candle.
    pub fn fold(&mut self, bar: &Bar) {
        let tr = Self::true_range(bar, self.previous_close) * SCALE;
        self.count = self.count.saturating_add(1);
        let n = i128::from(self.period);
        if self.count <= u32::from(self.period) {
            self.sum += tr;
            self.value = self.sum.div_euclid(i128::from(self.count));
        } else {
            self.value += (tr - self.value).div_euclid(n);
        }
        self.previous_close = Some(bar.close);
    }

    /// At least `period` candles folded.
    #[must_use]
    pub fn warm(&self) -> bool {
        self.count >= u32::from(self.period)
    }

    /// The ATR in whole paisa (floored), once warm.
    #[must_use]
    pub fn value(&self) -> Option<i64> {
        if !self.warm() {
            return None;
        }
        i64::try_from(self.value.div_euclid(SCALE)).ok()
    }

    /// The scaled state.
    #[must_use]
    pub const fn scaled(&self) -> i128 {
        self.value
    }
}
