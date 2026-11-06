//! F20..F27: the eight conditions, each read on the CLOSED signal candle, long and
//! short mirrored (PREVIEW-SPEC section 3.1, table C1..C8). Every test is exact in
//! `i128`; a tie is never beyond.

use crate::dayref::AcceptedBar;
use crate::indicators::SCALE;
use crate::indicators::bollinger::Window;
use crate::indicators::ema::Ema;
use crate::indicators::supertrend::Reading as StReading;
use crate::indicators::{Trend, cci, rsi, willr};

/// The side of a trade.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Side {
    /// Buy, then sell.
    Long,
    /// Sell, then buy.
    Short,
}

impl Side {
    /// Both sides, long first.
    pub const BOTH: [Self; 2] = [Self::Long, Self::Short];

    /// `LONG` or `SHORT`.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Long => "LONG",
            Self::Short => "SHORT",
        }
    }
}

/// `x = 2C - H - L` and `T = H + L + C` of the previous session.
fn x_and_t(prev: &AcceptedBar) -> (i128, i128) {
    let (h, l, c) = (
        i128::from(prev.high()),
        i128::from(prev.low()),
        i128::from(prev.close()),
    );
    (2 * c - h - l, h + l + c)
}

/// F20 (C1): the previous session's CPR is narrow at `bp` basis points of the pivot:
/// `10_000 x |2C - H - L| <= bp x (H + L + C)`; a tie is narrow. The same on both sides.
#[must_use]
pub fn c1_narrow_cpr(prev: &AcceptedBar, bp: u16) -> bool {
    let (x, t) = x_and_t(prev);
    10_000 * x.abs() <= i128::from(bp) * t
}

/// F21 (C2): the close beyond R1 (long, `3 c > 2T - 3L`) or S1 (short, `3 c < 2T - 3H`).
#[must_use]
pub fn c2_beyond_r1_s1(prev: &AcceptedBar, close: i64, side: Side) -> bool {
    let (_, t) = x_and_t(prev);
    let c3 = 3 * i128::from(close);
    match side {
        Side::Long => c3 > 2 * t - 3 * i128::from(prev.low()),
        Side::Short => c3 < 2 * t - 3 * i128::from(prev.high()),
    }
}

/// F22 (C3): the Supertrend reading is settled and up (long) or down (short).
#[must_use]
pub fn c3_supertrend_side(reading: Option<&StReading>, side: Side) -> bool {
    let want = if side == Side::Long {
        Trend::Up
    } else {
        Trend::Down
    };
    reading.is_some_and(|r| r.settled && r.trend == want)
}

/// F23 (C4): the close beyond the band at width `z_milli` (above the upper, below the lower).
#[must_use]
pub fn c4_bollinger_beyond(window: Option<&Window>, z_milli: u32, side: Side) -> bool {
    window.is_some_and(|w| match side {
        Side::Long => w.above_upper(z_milli),
        Side::Short => w.below_lower(z_milli),
    })
}

/// F24 (C5): a faithful RSI above `long_ppm` (long) or below `short_ppm` (short).
#[must_use]
pub fn c5_rsi_beyond(
    reading: Option<&rsi::Reading>,
    long_ppm: u32,
    short_ppm: u32,
    side: Side,
) -> bool {
    reading.is_some_and(|r| {
        r.faithful()
            && match side {
                Side::Long => r.above(long_ppm),
                Side::Short => r.below(short_ppm),
            }
    })
}

/// F25 (C6): a %R that is not flat, above `long_ppm` (long) or below `short_ppm` (short).
#[must_use]
pub fn c6_willr_level(
    reading: Option<&willr::Reading>,
    long_ppm: i32,
    short_ppm: i32,
    side: Side,
) -> bool {
    reading.is_some_and(|r| {
        !r.flat()
            && match side {
                Side::Long => r.above(long_ppm),
                Side::Short => r.below(short_ppm),
            }
    })
}

/// F26 (C7): a CCI that is not flat, above `long_milli` (long) or below `short_milli` (short).
#[must_use]
pub fn c7_cci_beyond(
    reading: Option<&cci::Reading>,
    long_milli: i32,
    short_milli: i32,
    side: Side,
) -> bool {
    reading.is_some_and(|r| {
        !r.flat()
            && match side {
                Side::Long => r.above(long_milli),
                Side::Short => r.below(short_milli),
            }
    })
}

/// F27 (C8): a warm EMA, the close strictly above (long) or below (short) it, compared
/// on the scaled state: `close x SCALE` against `ema.scaled()`.
#[must_use]
pub fn c8_ema_side(ema: &Ema, close: i64, side: Side) -> bool {
    let c = i128::from(close) * SCALE;
    ema.warm()
        && match side {
            Side::Long => c > ema.scaled(),
            Side::Short => c < ema.scaled(),
        }
}

#[cfg(test)]
#[path = "cond_tests.rs"]
mod tests;
