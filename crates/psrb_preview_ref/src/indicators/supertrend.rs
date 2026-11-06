//! The standard two-band Supertrend (PARTS 5.3, the published per-candle rules):
//! both bands ratchet on every candle; the trend turns when the close crosses the
//! CURRENT candle's band; the previous TREND picks the band judged; the bands start at
//! 0 and the first ATR candle reads `Down`, not settled.

use super::Trend;
use super::atr::Atr;
use crate::market::Bar;

/// One reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    /// The direction.
    pub trend: Trend,
    /// The line: the lower band when up, the upper band when down.
    pub line: i128,
    /// The upper band.
    pub upper: i128,
    /// The lower band.
    pub lower: i128,
    /// False on the first ATR candle, true after.
    pub settled: bool,
}

/// The state of one Supertrend (one ATR period and one factor in thousandths).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SuperTrend {
    atr: Atr,
    factor_milli: i128,
    previous_close: Option<i64>,
    upper: i128,
    lower: i128,
    trend: Trend,
    seen_atr: bool,
    reading: Option<Reading>,
}

impl SuperTrend {
    /// ATR `period` and factor in thousandths (clamped to 1..=100,000).
    #[must_use]
    pub fn new(period: u16, factor_milli: u32) -> Self {
        Self {
            atr: Atr::new(period),
            factor_milli: i128::from(factor_milli.clamp(1, 100_000)),
            previous_close: None,
            upper: 0,
            lower: 0,
            trend: Trend::Down,
            seen_atr: false,
            reading: None,
        }
    }

    /// Folds one evaluable candle.
    pub fn fold(&mut self, bar: &Bar) {
        self.atr.fold(bar);
        let Some(atr) = self.atr.value() else {
            self.previous_close = Some(bar.close);
            return;
        };
        let mid = (i128::from(bar.high) + i128::from(bar.low)).div_euclid(2);
        let band = (i128::from(atr) * self.factor_milli).div_euclid(1_000);
        let basic_upper = mid + band;
        let basic_lower = mid - band;
        let pc = self.previous_close.map(i128::from);
        let upper = if basic_upper < self.upper || pc.is_some_and(|c| c > self.upper) {
            basic_upper
        } else {
            self.upper
        };
        let lower = if basic_lower > self.lower || pc.is_some_and(|c| c < self.lower) {
            basic_lower
        } else {
            self.lower
        };
        let close = i128::from(bar.close);
        let trend = if !self.seen_atr {
            Trend::Down
        } else if self.trend == Trend::Down {
            if close > upper { Trend::Up } else { Trend::Down }
        } else if close < lower {
            Trend::Down
        } else {
            Trend::Up
        };
        let line = if trend == Trend::Up { lower } else { upper };
        self.reading = Some(Reading { trend, line, upper, lower, settled: self.seen_atr });
        self.upper = upper;
        self.lower = lower;
        self.trend = trend;
        self.seen_atr = true;
        self.previous_close = Some(bar.close);
    }

    /// The reading, once the ATR has folded its period.
    #[must_use]
    pub const fn reading(&self) -> Option<Reading> {
        self.reading
    }

    /// The ATR in whole paisa, once warm.
    #[must_use]
    pub fn atr(&self) -> Option<i64> {
        self.atr.value()
    }
}
